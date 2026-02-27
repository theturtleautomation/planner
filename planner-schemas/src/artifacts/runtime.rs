//! # Runtime artifacts: gate_result, decision, context_pack
//!
//! These are internal runtime artifacts used by the Dark Factory
//! engine during pipeline execution.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::artifacts::scenario_set::ScenarioTier;
use crate::turn::ArtifactPayload;

// ===========================================================================
// planner.gate_result.v1
// ===========================================================================

/// Result of a tiered validation gate.
///
/// Gates evaluate scenario results in strict order:
/// 100% Critical → 95% High → 90% Medium aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResultV1 {
    /// Which project this gate belongs to.
    pub project_id: Uuid,

    /// Which run this gate evaluated.
    pub run_id: Uuid,

    /// Which gate in the pipeline (e.g. "post-kilroy", "post-retry-1").
    pub gate_name: String,

    /// Per-tier pass rates.
    pub critical_pass_rate: f32,
    pub high_pass_rate: f32,
    pub medium_pass_rate: f32,

    /// Overall gate decision.
    pub passed: bool,

    /// Which tier caused the failure, if any.
    pub failed_tier: Option<ScenarioTier>,

    /// What action was taken.
    pub action: GateAction,
}

impl ArtifactPayload for GateResultV1 {
    const TYPE_ID: &'static str = "planner.gate_result.v1";
}

/// Action taken by the gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GateAction {
    /// All tiers passed — proceed to next stage.
    Proceed,
    /// Failed tier — retry within budget.
    Retry,
    /// Budget exhausted — surface Consequence Card.
    Escalate,
    /// Manual intervention required.
    Halt,
}

// ===========================================================================
// planner.decision.v1
// ===========================================================================

/// Audit trail for human overrides or Sacred Anchor amendments.
///
/// Any modification to a Sacred Anchor requires a Decision turn.
/// This is the only way immutable constraints can change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionV1 {
    /// Which project this decision belongs to.
    pub project_id: Uuid,

    /// Which run this decision was made during.
    pub run_id: Uuid,

    /// What type of decision.
    pub decision_type: DecisionType,

    /// Plain-English description of the decision.
    pub description: String,

    /// What was the state before this decision.
    pub before: String,

    /// What is the state after this decision.
    pub after: String,

    /// Why the decision was made.
    pub rationale: String,

    /// Who made the decision ("user" or "system").
    pub decided_by: String,
}

impl ArtifactPayload for DecisionV1 {
    const TYPE_ID: &'static str = "planner.decision.v1";
}

/// Types of decisions that require an audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionType {
    /// Modifying a Sacred Anchor.
    SacredAnchorAmendment,
    /// Overriding a gate failure.
    GateOverride,
    /// Changing run budget.
    BudgetChange,
    /// Changing output domain constraint.
    OutputDomainChange,
    /// Other decision requiring audit trail.
    Other,
}

// ===========================================================================
// planner.context_pack.v1
// ===========================================================================

/// A dynamically compiled subset of state fed to an agent.
///
/// Context Packs prevent "lost in the middle" amnesia by giving each
/// agent exactly the context it needs — no more, no less.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackV1 {
    /// Which project this context pack belongs to.
    pub project_id: Uuid,

    /// Which pipeline step this context was compiled for.
    pub target_step: String,

    /// Which agent will consume this context.
    pub target_agent: String,

    /// The compiled context sections.
    pub sections: Vec<ContextSection>,

    /// Total token estimate for this context pack.
    pub estimated_tokens: u64,

    /// Which CXDB turns were selected for this pack.
    pub source_turn_ids: Vec<Uuid>,
}

impl ArtifactPayload for ContextPackV1 {
    const TYPE_ID: &'static str = "planner.context_pack.v1";
}

/// A section within a Context Pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    /// Section header (e.g. "Sacred Anchors", "Phase 1 Contracts").
    pub header: String,

    /// The context content.
    pub content: String,

    /// Priority — higher priority sections are included first when
    /// the context window is tight.
    pub priority: u32,

    /// Estimated tokens for this section.
    pub estimated_tokens: u64,
}
