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
pub mod ws_socratic;

use auth::AuthConfig;
use session::SessionStore;

/// Shared application state.
pub struct AppState {
    /// Active planning sessions.
    pub sessions: SessionStore,
    /// Living System Blueprint graph store.
    pub blueprints: planner_core::blueprint::BlueprintStore,
    /// Auth0 JWT config. None = dev mode (auth bypassed).
    pub auth_config: Option<AuthConfig>,
    /// Filesystem-backed event store. None if persistence is unavailable.
    pub event_store: Option<planner_core::observability::EventStore>,
    /// Durable CXDB engine for persisting pipeline Turn records.
    /// None if CXDB initialization failed (pipeline runs without persistence).
    pub cxdb: Option<planner_core::cxdb::durable::DurableCxdbEngine>,
    /// Server start time for uptime calculation.
    pub started_at: std::time::Instant,
}
