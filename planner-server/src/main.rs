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

mod api;
mod auth;
mod rate_limit;
mod rbac;
mod session;
mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use auth::AuthConfig;
use session::SessionStore;

/// Shared application state.
pub struct AppState {
    /// Active planning sessions.
    pub sessions: SessionStore,
    /// Auth0 JWT config. None = dev mode (auth bypassed).
    pub auth_config: Option<AuthConfig>,
}

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

    // Create shared state
    let state = Arc::new(AppState {
        sessions: SessionStore::new(),
        auth_config,
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

    // Start background session cleanup task (runs every 5 minutes)
    {
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
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
    tracing::info!("Planner server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
