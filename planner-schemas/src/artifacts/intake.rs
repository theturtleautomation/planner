//! # planner.intake.v1
//!
//! Produced by the Intake Gateway after the Opus-driven Socratic interview.
//! Contains project identity, environment choices, and output domain
//! classification. The Intake Gateway enforces the output domain constraint
//! (micro-tools only in Phase 0).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// IntakeV1
// ---------------------------------------------------------------------------

/// The output of the Socratic Intake interview.
///
/// This artifact captures everything the Smart Tinkerer described,
/// plus what the system discovered about the existing codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeV1 {
    /// Unique project identifier.
    pub project_id: Uuid,

    /// Human-readable project name (e.g. "Task Tracker").
    pub project_name: String,

    /// Feature slug for this specific intake (e.g. "add-stripe-checkout").
    pub feature_slug: String,

    /// Plain-English description of what the user wants to build.
    pub intent_summary: String,

    /// Output domain classification — determines what the Compiler can generate.
    pub output_domain: OutputDomain,

    /// Detected or chosen programming environment.
    pub environment: EnvironmentInfo,

    /// Sacred Anchors — immutable intent constraints from the user.
    /// Amendments require a `planner.decision.v1` turn.
    pub sacred_anchors: Vec<SacredAnchor>,

    /// Satisfaction criteria seeds — plain-English descriptions of what
    /// "working" means to the user. The Compiler expands these into
    /// full BDD scenarios.
    pub satisfaction_criteria_seeds: Vec<String>,

    /// Explicit out-of-scope items identified during the interview.
    pub out_of_scope: Vec<String>,

    /// Raw conversation turns from the Socratic interview (for audit trail).
    pub conversation_log: Vec<ConversationTurn>,
}

impl ArtifactPayload for IntakeV1 {
    const TYPE_ID: &'static str = "planner.intake.v1";
}

// ---------------------------------------------------------------------------
// Output Domain
// ---------------------------------------------------------------------------

/// What class of software this intake is targeting.
///
/// Phase 0 enforces `MicroTool` only. The Intake Gateway redirects
/// complex requests gracefully: "Let's start with one piece of this —
/// which part is most important to you?"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputDomain {
    /// Phase 0: Single-view React/Tailwind widget OR single-file Python
    /// FastAPI backend. ~200 lines of generated code.
    MicroTool {
        /// Which flavor of micro-tool.
        variant: MicroToolVariant,
    },

    /// Phase 3+: Multi-domain applications with multiple NLSpec chunks.
    FullApp {
        /// Estimated domain count (auth, api, ui, payments, etc.).
        estimated_domains: u32,
    },
}

/// The specific micro-tool variant for Phase 0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MicroToolVariant {
    /// Single-view React + Tailwind widget.
    ReactWidget,
    /// Single-file Python FastAPI backend.
    FastApiBackend,
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

/// Detected or specified development environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Primary language (e.g. "TypeScript", "Python").
    pub language: String,

    /// Framework (e.g. "React", "FastAPI").
    pub framework: String,

    /// Package manager (e.g. "npm", "pip", "cargo").
    pub package_manager: Option<String>,

    /// Detected existing dependencies.
    pub existing_dependencies: Vec<String>,

    /// Build system / tooling (e.g. "vite", "uvicorn").
    pub build_tool: Option<String>,
}

// ---------------------------------------------------------------------------
// Sacred Anchor
// ---------------------------------------------------------------------------

/// An immutable intent constraint. Once set, cannot be modified without
/// a formal `planner.decision.v1` amendment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SacredAnchor {
    /// Stable ID for cross-referencing (e.g. "SA-1", "SA-2").
    pub id: String,

    /// Plain-English statement of the constraint.
    pub statement: String,

    /// Why this anchor matters to the user.
    pub rationale: Option<String>,
}

// ---------------------------------------------------------------------------
// Conversation Turn (audit trail)
// ---------------------------------------------------------------------------

/// A single turn in the Socratic interview conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Who spoke: "user" or "system".
    pub role: String,

    /// What was said.
    pub content: String,

    /// ISO 8601 timestamp.
    pub timestamp: String,
}
