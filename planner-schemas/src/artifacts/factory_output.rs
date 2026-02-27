//! # planner.factory_output.v1
//!
//! Produced by the Factory Diplomat after Kilroy completes a run.
//! Captures build status, spend, checkpoint path, and DoD results.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// FactoryOutputV1
// ---------------------------------------------------------------------------

/// Results from a Kilroy execution run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryOutputV1 {
    /// Kilroy's internal run identifier.
    pub kilroy_run_id: Uuid,

    /// Which NLSpec version this run executed.
    pub nlspec_version: String,

    /// Attempt number (1-based; increments on retry within budget).
    pub attempt: u32,

    /// Overall build status.
    pub build_status: BuildStatus,

    /// Total USD spent on this run (LLM API calls).
    pub spend_usd: f32,

    /// Path to Kilroy's checkpoint.json for session rehydration.
    pub checkpoint_path: String,

    /// Definition of Done results — which items passed/failed.
    pub dod_results: Vec<DoDResult>,

    /// Per-node execution summaries.
    pub node_results: Vec<NodeResult>,

    /// Path to the generated code output directory.
    pub output_path: String,
}

impl ArtifactPayload for FactoryOutputV1 {
    const TYPE_ID: &'static str = "planner.factory_output.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Overall build status from Kilroy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildStatus {
    /// All nodes completed successfully.
    Success,
    /// Some nodes failed but the build is partially usable.
    PartialSuccess,
    /// Build failed — goal_gate node exhausted retries.
    Failed,
    /// Run was terminated by the financial circuit breaker.
    BudgetExhausted,
    /// Kilroy crashed or encountered an unrecoverable error.
    Error { message: String },
}

/// Result of a single DoD checklist item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoDResult {
    /// The DoD criterion text.
    pub criterion: String,

    /// Whether this criterion passed.
    pub passed: bool,

    /// Evidence or reason for the result.
    pub evidence: Option<String>,
}

/// Execution summary for a single graph.dot node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// DOT node name (e.g. "contracts", "auth").
    pub node_name: String,

    /// Whether this node succeeded.
    pub success: bool,

    /// Number of attempts (including retries).
    pub attempts: u32,

    /// USD spent on this node.
    pub spend_usd: f32,

    /// Duration in seconds.
    pub duration_secs: f64,

    /// Error message if failed.
    pub error: Option<String>,
}
