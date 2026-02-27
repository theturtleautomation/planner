//! # planner.scenario_set.v1
//!
//! Weighted BDD scenarios generated from Sacred Anchors and Satisfaction
//! Criteria. Stored in an isolated CXDB context — the factory (Kilroy)
//! never has read access to scenario text.
//!
//! Tiers: Critical (100% pass) → High (≥95%) → Medium (≥90% aggregate).
//! Each scenario runs 3x with majority pass (2/3) required.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// ScenarioSetV1
// ---------------------------------------------------------------------------

/// A set of weighted BDD scenarios for cross-model evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSetV1 {
    /// Which project these scenarios validate.
    pub project_id: Uuid,

    /// Which NLSpec version these scenarios were generated from.
    pub nlspec_version: String,

    /// The scenarios, ordered by tier (critical first).
    pub scenarios: Vec<Scenario>,

    /// Isolated CXDB context ID — factory has no read access.
    pub isolation_context_id: Uuid,

    /// Whether Ralph has augmented this set with additional scenarios.
    pub ralph_augmented: bool,
}

impl ArtifactPayload for ScenarioSetV1 {
    const TYPE_ID: &'static str = "planner.scenario_set.v1";
}

// ---------------------------------------------------------------------------
// Scenario
// ---------------------------------------------------------------------------

/// A single BDD scenario with tier weighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Stable ID (e.g. "SC-CRIT-1", "SC-HIGH-2", "SC-MED-3").
    pub id: String,

    /// Validation tier — determines pass threshold.
    pub tier: ScenarioTier,

    /// Human-readable title.
    pub title: String,

    /// BDD text (Given/When/Then format).
    pub bdd_text: String,

    /// External dependencies this scenario exercises (for DTU routing).
    pub dtu_deps: Vec<String>,

    /// Which Sacred Anchor(s) this scenario traces to.
    pub traces_to_anchors: Vec<String>,

    /// Which Satisfaction Criterion seed generated this scenario.
    pub source_criterion: Option<String>,
}

// ---------------------------------------------------------------------------
// ScenarioTier
// ---------------------------------------------------------------------------

/// Validation tier determining the pass threshold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScenarioTier {
    /// 100% pass required. Pipeline halts on failure.
    Critical,
    /// ≥95% pass required.
    High,
    /// ≥90% aggregate required.
    Medium,
}

impl ScenarioTier {
    /// Required pass rate for this tier.
    pub fn required_pass_rate(&self) -> f32 {
        match self {
            ScenarioTier::Critical => 1.0,
            ScenarioTier::High => 0.95,
            ScenarioTier::Medium => 0.90,
        }
    }
}
