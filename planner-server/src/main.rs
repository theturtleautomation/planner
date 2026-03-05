//! # Planner Server — HTTP + WebSocket Backend
//!
//! Serves the Socratic Lobby web frontend and provides:
//! - REST API for pipeline operations
//! - WebSocket endpoint for real-time session updates
//! - Static file serving for the React frontend
//!
//! Endpoints:
//! - GET  /api/health          — Health check (public)
//! - GET  /api/models          — List available LLM models (protected)
//! - GET  /api/sessions        — List sessions for current user (protected)
//! - POST /api/sessions        — Create a new planning session (protected)
//! - GET  /api/sessions/:id    — Get session state (protected)
//! - POST /api/sessions/:id/message — Send a message to the session (protected)
//! - GET  /api/sessions/:id/ws — WebSocket for real-time updates (protected)
//! - GET  /*                   — Static file serving (React frontend)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use planner_server::AppState;
use planner_server::api;
use planner_server::auth::AuthConfig;
use planner_server::rate_limit;
use planner_server::session::SessionStore;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("planner_server=info".parse().unwrap()),
        )
        .init();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(3100);

    let static_dir = args
        .iter()
        .position(|a| a == "--static-dir")
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| "./planner-web/dist".to_string());

    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: planner-server [--port <port>] [--static-dir <path>]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --port <port>        HTTP port (default: 3100)");
        eprintln!("  --static-dir <path>  Path to React build (default: ./planner-web/dist)");
        std::process::exit(0);
    }

    // Load auth config from environment
    let auth_config = AuthConfig::from_env();
    let auth_enabled = auth_config.is_some();
    if auth_enabled {
        tracing::info!("Auth0 JWT validation enabled");
    } else {
        tracing::warn!("Auth0 not configured — running in dev mode (no auth)");
    }

    // Initialize session store — disk-backed, loads existing sessions on startup.
    let data_dir = std::env::var("PLANNER_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let session_store = match SessionStore::open(std::path::Path::new(&data_dir)) {
        Ok(store) => {
            tracing::info!(
                "Session persistence enabled: {}/sessions/ ({} sessions loaded)",
                data_dir, store.count(),
            );
            store
        }
        Err(e) => {
            tracing::warn!("Session persistence unavailable ({}), falling back to in-memory only", e);
            SessionStore::new()
        }
    };

    // Initialize Blueprint store — disk-backed alongside sessions.
    let blueprint_store = match planner_core::blueprint::BlueprintStore::open(std::path::Path::new(&data_dir)) {
        Ok(store) => {
            let (nodes, edges) = store.counts();
            tracing::info!(
                "Blueprint persistence enabled: {}/blueprint/ ({} nodes, {} edges loaded)",
                data_dir, nodes, edges,
            );
            store
        }
        Err(e) => {
            tracing::warn!("Blueprint persistence unavailable ({}), falling back to in-memory only", e);
            planner_core::blueprint::BlueprintStore::new()
        }
    };

    // Initialize event persistence
    let event_store = match planner_core::observability::EventStore::new(std::path::Path::new(&data_dir)) {
        Ok(store) => {
            tracing::info!("Event persistence enabled: {}/events/", data_dir);
            Some(store)
        }
        Err(e) => {
            tracing::warn!("Event persistence disabled: {}", e);
            None
        }
    };

    // Initialize durable CXDB engine for pipeline Turn persistence.
    let cxdb_path = std::path::Path::new(&data_dir).join("cxdb");
    let cxdb = match planner_core::cxdb::durable::DurableCxdbEngine::open(&cxdb_path) {
        Ok(engine) => {
            let stats = engine.stats();
            tracing::info!(
                "CXDB persistence enabled: {} ({} turns, {} blobs)",
                cxdb_path.display(), stats.total_turns, stats.total_blobs,
            );
            Some(engine)
        }
        Err(e) => {
            tracing::warn!("CXDB persistence unavailable ({}), pipeline runs without durable storage", e);
            None
        }
    };

    // Create shared state
    let state = Arc::new(AppState {
        sessions: session_store,
        blueprints: blueprint_store,
        auth_config,
        event_store,
        cxdb,
        started_at: std::time::Instant::now(),
    });

    // Build in-memory rate limiter and start background eviction task.
    let rate_limiter = Arc::new(rate_limit::RateLimiter::new());
    rate_limit::spawn_eviction_task(rate_limiter.clone());
    tracing::info!("Rate limiter: 100 req/min per IP, eviction every 5 min");

    // Build CORS layer — restrict origins when auth is enabled
    let cors = if auth_enabled {
        CorsLayer::new()
            .allow_origin([
                "http://localhost:5173".parse().unwrap(),
                "http://localhost:3100".parse().unwrap(),
            ])
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = Router::new()
        .nest("/api/v1", api::routes(state.clone()))
        .nest("/api", api::routes(state.clone()))
        // Apply rate limiting across all API routes.
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit::rate_limit_middleware,
        ))
        .layer(cors);

    // Start background session + blueprint flush task (runs every 5 seconds, with initial delay)
    {
        let flush_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(
                tokio::time::Instant::now() + std::time::Duration::from_secs(5),
                std::time::Duration::from_secs(5),
            );
            loop {
                interval.tick().await;
                let (flushed, errors) = flush_state.sessions.flush_dirty();
                if errors > 0 {
                    tracing::warn!("Session flush: {} written, {} errors", flushed, errors);
                }
                match flush_state.blueprints.flush() {
                    Ok(true) => tracing::debug!("Blueprint flushed to disk"),
                    Ok(false) => {} // nothing dirty
                    Err(e) => tracing::warn!("Blueprint flush error: {}", e),
                }
            }
        });
    }

    // Start background session cleanup task (runs every 5 minutes, with initial delay)
    {
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(
                tokio::time::Instant::now() + std::time::Duration::from_secs(300),
                std::time::Duration::from_secs(300),
            );
            loop {
                interval.tick().await;
                cleanup_state.sessions.cleanup_expired(3600);
            }
        });
    }

    // Add static file serving if directory exists
    let app = if std::path::Path::new(&static_dir).exists() {
        tracing::info!("Serving static files from: {}", static_dir);
        app.fallback_service(
            tower_http::services::ServeDir::new(&static_dir)
                .fallback(tower_http::services::ServeFile::new(
                    format!("{}/index.html", static_dir),
                )),
        )
    } else {
        tracing::warn!("Static directory not found: {} — API only mode", static_dir);
        app
    };

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Report LLM provider status at startup.
    let router = planner_core::llm::providers::LlmRouter::from_env();
    let providers = router.available_providers();
    if providers.is_empty() {
        tracing::warn!("No LLM CLI providers detected! Install and authenticate at least one: claude, gemini, codex");
    } else {
        tracing::info!("LLM providers available: {:?}", providers);
    }

    tracing::info!("Planner server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Graceful shutdown: flush dirty sessions on SIGINT or SIGTERM.
    let shutdown_state = state.clone();
    let shutdown = async move {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ).expect("failed to register SIGTERM handler");
        #[cfg(unix)]
        let sigterm_recv = sigterm.recv();
        #[cfg(not(unix))]
        let sigterm_recv = std::future::pending::<Option<()>>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("SIGINT received — initiating graceful shutdown...");
            }
            _ = sigterm_recv => {
                tracing::info!("SIGTERM received — initiating graceful shutdown...");
            }
        }

        tracing::info!("Flushing dirty sessions and blueprint...");
        let (flushed, errors) = shutdown_state.sessions.flush_dirty();
        tracing::info!("Session flush: {} written, {} errors", flushed, errors);
        match shutdown_state.blueprints.flush() {
            Ok(true) => tracing::info!("Blueprint flushed to disk"),
            Ok(false) => tracing::info!("Blueprint clean — no flush needed"),
            Err(e) => tracing::warn!("Blueprint shutdown flush error: {}", e),
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .unwrap();
}
