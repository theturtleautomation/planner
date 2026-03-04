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

pub mod domain_classifier;
pub mod belief_state;
pub mod constitution;
pub mod question_planner;
pub mod convergence;
pub mod speculative_draft;
pub mod socratic_engine;

// Re-export the key public APIs
pub use socratic_engine::{SocraticIO, run_interview, session_to_intake};
pub use belief_state::{verify_and_update, persist_to_cxdb, restore_from_cxdb, format_belief_state_for_llm};
pub use domain_classifier::classify_domain;
pub use convergence::check_convergence;
pub use constitution::{load_constitution, evaluate_question, check_coverage};
pub use question_planner::{plan_next_question, select_target_dimension};
pub use speculative_draft::{should_trigger_draft, generate_draft, format_draft_for_display};
