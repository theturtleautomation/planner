//! # planner.ralph_finding.v1
//!
//! Output from Ralph Loop background agents. Ralph operates in three modes:
//! - Scenario Augmentation: additional critical/high scenarios from edge cases
//! - Gene Transfusion: advisory findings from known component patterns
//! - DTU Configuration: behavioral clone specs for external dependencies

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// RalphFindingV1
// ---------------------------------------------------------------------------

/// A finding from one of Ralph's three operational modes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphFindingV1 {
    /// Unique finding identifier.
    pub finding_id: Uuid,

    /// Which project this finding belongs to.
    pub project_id: Uuid,

    /// Which Ralph mode produced this finding.
    pub finding_type: RalphFindingType,

    /// Severity / priority of the finding.
    pub severity: RalphSeverity,

    /// Human-readable title.
    pub title: String,

    /// Detailed description.
    pub description: String,

    /// Suggested action (may become a Consequence Card).
    pub suggested_action: Option<String>,

    /// Whether this finding has been surfaced as a Consequence Card.
    pub surfaced: bool,
}

impl ArtifactPayload for RalphFindingV1 {
    const TYPE_ID: &'static str = "planner.ralph_finding.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Ralph's three operational modes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RalphFindingType {
    /// After initial scenario generation: additional edge case scenarios.
    ScenarioAugmentation {
        /// New scenarios to add to the ScenarioSet.
        additional_scenario_count: u32,
    },

    /// During spec generation for known component types.
    /// Advisory findings from established patterns.
    GeneTransfusion {
        /// Which component pattern was matched (e.g. "stripe-checkout").
        pattern_name: String,
    },

    /// When an external dependency has `dtu_priority: high`.
    /// Produces a behavioral clone specification.
    DtuConfiguration {
        /// Which dependency this DTU config targets.
        dependency_name: String,
    },
}

/// Ralph finding severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RalphSeverity {
    /// Should be surfaced as a Consequence Card.
    High,
    /// Logged, advisory only.
    Medium,
    /// Informational, background context.
    Low,
}
