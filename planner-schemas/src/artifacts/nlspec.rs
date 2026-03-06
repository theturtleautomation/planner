//! # planner.nlspec.v1
//!
//! Chunked Progressive Specification — the Compiler's primary output.
//! Each chunk is independently reviewable by AR, independently loadable
//! by the factory agent, and independently lintable. Sub-500 lines enforced.
//!
//! Phase 0 produces a single root chunk only.
//! Phase 3 introduces multi-chunk (auth, api, ui, payments, etc.).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// NLSpecV1
// ---------------------------------------------------------------------------

/// A single chunk of the Progressive Specification.
///
/// The root chunk contains Intent Summary, Sacred Anchors, and Phase 1
/// Contracts. Domain chunks contain domain-specific FRs, constraints,
/// and DoD items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLSpecV1 {
    /// Which project this spec belongs to.
    pub project_id: Uuid,

    /// Spec version (monotonically increasing per project).
    pub version: String,

    /// Which chunk this is.
    pub chunk: ChunkType,

    /// Current lifecycle status.
    pub status: NLSpecStatus,

    /// Line count — enforced ≤500 by the Spec Linter.
    pub line_count: u32,

    /// Pointer to the IntakeV1 turn that created this spec.
    pub created_from: String,

    // -- Sections (root chunk has all; domain chunks have a subset) --
    /// Intent Summary — root chunk only. Plain-English project description.
    pub intent_summary: Option<String>,

    /// Sacred Anchors — root chunk only. Copied from IntakeV1.
    pub sacred_anchors: Option<Vec<NLSpecAnchor>>,

    /// Functional Requirements for this chunk's domain.
    pub requirements: Vec<Requirement>,

    /// Architectural Constraints for this chunk's domain.
    pub architectural_constraints: Vec<String>,

    /// Phase 1 Contracts — root chunk only. Shared type definitions
    /// that must be locked before parallel domain work begins.
    pub phase1_contracts: Option<Vec<Phase1Contract>>,

    /// External Dependencies with DTU priority.
    pub external_dependencies: Vec<ExternalDependency>,

    /// Definition of Done checklist.
    pub definition_of_done: Vec<DoDItem>,

    /// Satisfaction Criteria — scenario seeds for the Scenario Generator.
    pub satisfaction_criteria: Vec<SatisfactionCriterion>,

    /// Open Questions — must be empty before graph.dot generation.
    pub open_questions: Vec<OpenQuestion>,

    /// Explicit out-of-scope items.
    pub out_of_scope: Vec<String>,

    /// Amendment Log — append-only. No retroactive edits.
    pub amendment_log: Vec<Amendment>,
}

impl ArtifactPayload for NLSpecV1 {
    const TYPE_ID: &'static str = "planner.nlspec.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Which chunk of the Progressive Specification this is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    /// The root chunk — contains Intent Summary, Sacred Anchors, Phase 1 Contracts.
    Root,
    /// A domain-specific chunk (e.g. auth, api, ui, payments).
    Domain { name: String },
}

/// NLSpec lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NLSpecStatus {
    /// Initial generation, not yet linted.
    Draft,
    /// Passed spec linting, ready for AR.
    Linted,
    /// Passed Adversarial Review.
    ArReviewed,
    /// Ready for graph.dot generation and Kilroy handoff.
    FactoryReady,
    /// Modified after initial approval (amendment applied).
    Amended,
}

/// A Sacred Anchor reference within the NLSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLSpecAnchor {
    /// Stable ID matching IntakeV1 (e.g. "SA-1").
    pub id: String,
    /// The anchor statement.
    pub statement: String,
}

/// A functional requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    /// Stable ID for cross-chunk referencing (e.g. "FR-1", "FR-2").
    pub id: String,

    /// The requirement statement — must use imperative language
    /// (must/must not/always/never). Enforced by Spec Linter rule 4.
    pub statement: String,

    /// Priority level.
    pub priority: Priority,

    /// Which Sacred Anchors this FR traces to.
    pub traces_to: Vec<String>,
}

/// Requirement priority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Must,
    Should,
    Could,
}

/// A Phase 1 Contract — shared type locked before parallel work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase1Contract {
    /// Contract name (e.g. "UserSession", "PaymentIntent").
    pub name: String,

    /// TypeScript/Python-style type definition.
    pub type_definition: String,

    /// Which domains consume this contract.
    pub consumed_by: Vec<String>,
}

/// An external dependency with DTU priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    /// Dependency name (e.g. "Stripe", "Auth0", "SendGrid").
    pub name: String,

    /// DTU priority for behavioral clone generation.
    pub dtu_priority: DtuPriority,

    /// How this dependency is used.
    pub usage_description: String,
}

/// DTU priority for external dependency mocking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DtuPriority {
    /// Phase 4: Full stateful in-memory clone.
    High,
    /// Phase 5: Stateful clone.
    Medium,
    /// Static mock responses sufficient.
    Low,
    /// No mock needed (e.g. standard library).
    None,
}

/// A Definition of Done checklist item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoDItem {
    /// What must be true for this to be "done".
    pub criterion: String,
    /// Whether this can be mechanically verified by the factory.
    pub mechanically_checkable: bool,
}

/// A satisfaction criterion seed — expanded into BDD scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionCriterion {
    /// Stable ID (e.g. "SC-1").
    pub id: String,
    /// Plain-English description of the expected behavior.
    pub description: String,
    /// Scenario tier hint.
    pub tier_hint: ScenarioTierHint,
}

/// Hint for which scenario tier this criterion maps to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScenarioTierHint {
    Critical,
    High,
    Medium,
}

/// An unresolved question that blocks graph.dot generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    /// The question.
    pub question: String,
    /// Who/what raised it (e.g. "spec-linter", "ar-reviewer-opus").
    pub raised_by: String,
    /// Resolution, once answered. Must be populated before proceeding.
    pub resolution: Option<String>,
}

/// An entry in the append-only Amendment Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amendment {
    /// ISO 8601 timestamp of the amendment.
    pub timestamp: String,
    /// What changed.
    pub description: String,
    /// Why it changed (e.g. "user feedback", "AR finding AR-B-3").
    pub reason: String,
    /// Which section was modified.
    pub affected_section: String,
}
