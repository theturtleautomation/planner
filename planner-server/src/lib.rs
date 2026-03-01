//! # Planner Server — Library Re-exports
//!
//! Thin library target that re-exports server modules for integration tests.
//! The binary entrypoint remains `main.rs`.

pub mod api;
pub mod auth;
pub mod rate_limit;
pub mod rbac;
pub mod session;
pub mod ws;

use auth::AuthConfig;
use session::SessionStore;

/// Shared application state.
pub struct AppState {
    /// Active planning sessions.
    pub sessions: SessionStore,
    /// Auth0 JWT config. None = dev mode (auth bypassed).
    pub auth_config: Option<AuthConfig>,
}
