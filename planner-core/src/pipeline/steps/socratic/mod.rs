//! # Socratic Intake Engine
//!
//! A 6-component elicitation engine that replaces the fire-and-forget
//! intake with a principled multi-turn Socratic interview.
//!
//! ## Components
//!
//! 1. **Domain Classifier** (`domain_classifier`) — Classifies project type and complexity
//! 2. **Belief State** (`belief_state`) — CRUD + CXDB persistence for RequirementsBeliefState
//! 3. **Constitution** (`constitution`) — Rule loading + self-critique evaluation
//! 4. **Question Planner** (`question_planner`) — Dimension scoring + question generation
//! 5. **Convergence** (`convergence`) — Multi-criteria stopping logic
//! 6. **Speculative Draft** (`speculative_draft`) — Draft generation + reaction parsing
//!
//! The **Socratic Engine** (`socratic_engine`) orchestrates the turn loop.
//!
//! ## IO Abstraction
//!
//! The engine communicates via `SocraticIO` trait, implemented by both
//! the TUI and WebSocket server. This keeps the engine IO-agnostic.

pub mod belief_state;
pub mod constitution;
pub mod convergence;
pub mod domain_classifier;
pub mod question_planner;
pub mod socratic_engine;
pub mod speculative_draft;

// Re-export the key public APIs
pub use belief_state::{
    format_belief_state_for_llm, persist_to_cxdb, restore_from_cxdb, verify_and_update,
};
pub use constitution::{check_coverage, evaluate_question, load_constitution};
pub use convergence::check_convergence;
pub use domain_classifier::classify_domain;
pub use question_planner::{plan_next_question, select_target_dimension};
pub use socratic_engine::{run_interview, session_to_intake, SocraticIO};
pub use speculative_draft::{format_draft_for_display, generate_draft, should_trigger_draft};
