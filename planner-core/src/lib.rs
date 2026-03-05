//! # planner-core — Library
//!
//! Re-exports pipeline components for integration tests and downstream consumers.
//! The binary entrypoint is in main.rs.
//!
//! Phase 0: Many types and functions are built for the full pipeline but not yet
//! wired into main.rs. They're used by tests and will be used in Phase 1+.

pub mod blueprint;
pub mod cxdb;
pub mod dtu;
pub mod llm;
pub mod observability;
pub mod pipeline;
