//! # planner.satisfaction_result.v1
//!
//! Produced by the Scenario Validator after cross-model evaluation.
//! Contains tiered pass rates and generalized errors (category + severity).
//! The factory only receives generalized errors — never scenario text.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::artifacts::scenario_set::ScenarioTier;
use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// SatisfactionResultV1
// ---------------------------------------------------------------------------

/// Results from cross-model scenario evaluation.
///
/// Gemini evaluates Claude's code — never the same model family
/// for builder and judge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionResultV1 {
    /// Which Kilroy run was evaluated.
    pub kilroy_run_id: Uuid,

    /// Critical tier pass rate — must be 1.0 to proceed.
    pub critical_pass_rate: f32,

    /// High tier pass rate — must be ≥0.95 to proceed.
    pub high_pass_rate: f32,

    /// Medium tier pass rate — must be ≥0.90 to proceed.
    pub medium_pass_rate: f32,

    /// Whether all tiered gates passed.
    pub gates_passed: bool,

    /// Per-scenario results.
    pub scenario_results: Vec<ScenarioResult>,
}

impl ArtifactPayload for SatisfactionResultV1 {
    const TYPE_ID: &'static str = "planner.satisfaction_result.v1";
}

impl SatisfactionResultV1 {
    /// Evaluate whether all tiered gates pass.
    pub fn evaluate_gates(&self) -> bool {
        self.critical_pass_rate >= 1.0
            && self.high_pass_rate >= 0.95
            && self.medium_pass_rate >= 0.90
    }

    /// Get the user-facing satisfaction message per the Telemetry Presenter spec.
    pub fn user_message(&self) -> &'static str {
        if self.critical_pass_rate >= 1.0
            && self.high_pass_rate >= 0.95
            && self.medium_pass_rate >= 0.90
        {
            "Everything works as described."
        } else if self.critical_pass_rate >= 1.0 && self.high_pass_rate >= 0.95 {
            "Your app works. A few minor behaviors didn't match expectations."
        } else if self.critical_pass_rate >= 1.0 {
            "Your app is mostly right but some important behaviors need attention."
        } else {
            "Something critical didn't work. I need to ask you something before I try again."
        }
    }
}

// ---------------------------------------------------------------------------
// ScenarioResult
// ---------------------------------------------------------------------------

/// Result of evaluating a single scenario (3 runs, majority pass).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// The scenario ID (e.g. "SC-CRIT-1").
    pub scenario_id: String,

    /// Which tier this scenario belongs to.
    pub tier: ScenarioTier,

    /// Scores from each of the 3 runs (0.0–1.0).
    pub runs: [f32; 3],

    /// Whether majority (2/3) of runs passed (score ≥ 0.5).
    pub majority_pass: bool,

    /// Aggregate score across runs.
    pub score: f32,

    /// Generalized error sent to the factory (never contains scenario text).
    pub generalized_error: Option<GeneralizedError>,
}

impl ScenarioResult {
    /// Compute majority pass from the 3 run scores.
    pub fn compute_majority_pass(runs: &[f32; 3]) -> bool {
        let pass_count = runs.iter().filter(|&&s| s >= 0.5).count();
        pass_count >= 2
    }
}

// ---------------------------------------------------------------------------
// GeneralizedError
// ---------------------------------------------------------------------------

/// Error feedback to the factory — category + severity only.
/// The factory never receives scenario text or specific BDD details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralizedError {
    /// Error category (e.g. "checkout-flow", "auth", "data-persistence").
    pub category: String,

    /// Severity level.
    pub severity: Severity,
}

/// Error severity for factory feedback.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}
