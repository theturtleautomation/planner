//! # Planner Server — HTTP + WebSocket Backend
//!
//! Serves the Socratic Lobby web frontend and provides:
//! - REST API for pipeline operations
//! - WebSocket endpoint for real-time session updates
//! - Static file serving for the React frontend
//!
//! Endpoints:
//! - GET  /api/health          — Health check
//! - POST /api/sessions        — Create a new planning session
//! - GET  /api/sessions/:id    — Get session state
//! - POST /api/sessions/:id/message — Send a message to the session
//! - GET  /api/sessions/:id/ws — WebSocket for real-time updates
//! - GET  /api/models          — List available LLM models
//! - GET  /*                   — Static file serving (React frontend)

mod api;
mod session;
mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use session::SessionStore;

/// Shared application state.
pub struct AppState {
    /// Active planning sessions.
    pub sessions: SessionStore,
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

    // Create shared state
    let state = Arc::new(AppState {
        sessions: SessionStore::new(),
    });

    // Build router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", api::routes(state.clone()))
        .layer(cors);

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
