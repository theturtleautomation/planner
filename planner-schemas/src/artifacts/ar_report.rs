//! # planner.ar_report.v1
//!
//! Output from the Adversarial Review pipeline. Three LLMs review
//! NLSpec chunks in parallel (Opus, GPT, Gemini), each with a
//! different lens. Findings are categorized as blocking / advisory /
//! informational.
//!
//! AR runs pre-execution — it reviews specs, not code.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// ArReportV1
// ---------------------------------------------------------------------------

/// Combined Adversarial Review report for an NLSpec chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArReportV1 {
    /// Which project was reviewed.
    pub project_id: Uuid,

    /// Which NLSpec chunk was reviewed (e.g. "root", "auth").
    pub chunk_name: String,

    /// Which NLSpec version was reviewed.
    pub nlspec_version: String,

    /// Findings from all three reviewers.
    pub findings: Vec<ArFinding>,

    /// Per-reviewer summaries.
    pub reviewer_summaries: Vec<ReviewerSummary>,

    /// Whether any blocking findings exist (blocks graph.dot generation).
    pub has_blocking: bool,

    /// Count by severity.
    pub blocking_count: u32,
    pub advisory_count: u32,
    pub informational_count: u32,
}

impl ArtifactPayload for ArReportV1 {
    const TYPE_ID: &'static str = "planner.ar_report.v1";
}

impl ArReportV1 {
    /// Recalculate counts and `has_blocking` from findings.
    pub fn recalculate(&mut self) {
        self.blocking_count = self.findings.iter()
            .filter(|f| f.severity == ArSeverity::Blocking).count() as u32;
        self.advisory_count = self.findings.iter()
            .filter(|f| f.severity == ArSeverity::Advisory).count() as u32;
        self.informational_count = self.findings.iter()
            .filter(|f| f.severity == ArSeverity::Informational).count() as u32;
        self.has_blocking = self.blocking_count > 0;
    }
}

// ---------------------------------------------------------------------------
// ArFinding
// ---------------------------------------------------------------------------

/// A single finding from an AR reviewer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArFinding {
    /// Finding ID (e.g. "AR-B-1" for blocking, "AR-A-3" for advisory).
    pub id: String,

    /// Which reviewer produced this finding.
    pub reviewer: ArReviewer,

    /// Severity — blocking findings prevent graph.dot generation.
    pub severity: ArSeverity,

    /// Which section of the NLSpec this finding applies to.
    pub affected_section: String,

    /// Which requirement ID(s) are affected, if applicable.
    pub affected_requirements: Vec<String>,

    /// The finding description.
    pub description: String,

    /// Suggested resolution.
    pub suggested_resolution: Option<String>,
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Which AR reviewer produced a finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArReviewer {
    /// Opus: Intent completeness, Sacred Anchor coverage, SC testability.
    Opus,
    /// GPT: Implementability, contradiction detection, contract precision.
    Gpt,
    /// Gemini: Scope integrity, out-of-scope completeness, DoD checkability.
    Gemini,
}

/// Per-reviewer summary in the AR report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewerSummary {
    /// Which reviewer this summary is for.
    pub reviewer: ArReviewer,

    /// One-paragraph summary of the reviewer's assessment.
    pub summary: String,

    /// Number of findings from this reviewer.
    pub finding_count: u32,

    /// Number of blocking findings from this reviewer.
    pub blocking_count: u32,
}

/// AR finding severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArSeverity {
    /// Blocks graph.dot generation. Must be resolved.
    Blocking,
    /// Should be addressed but doesn't block.
    Advisory,
    /// For awareness only.
    Informational,
}
