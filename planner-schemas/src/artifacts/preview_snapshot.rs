//! # planner.preview_snapshot.v1
//!
//! Simulator sandbox state — captures the Live Preview URL, build status,
//! and scenario results for audit trail. The snapshot persists in CXDB
//! even after the ephemeral sandbox is destroyed.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// PreviewSnapshotV1
// ---------------------------------------------------------------------------

/// Snapshot of a Simulator sandbox state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewSnapshotV1 {
    /// Unique sandbox identifier.
    pub sandbox_id: Uuid,

    /// Which project this preview belongs to.
    pub project_id: Uuid,

    /// Which Kilroy run produced the code in this sandbox.
    pub kilroy_run_id: Uuid,

    /// The Live Preview URL (e.g. "sandbox_{id}.preview.local").
    pub preview_url: String,

    /// Build status of the sandbox.
    pub build_status: PreviewBuildStatus,

    /// Scenario validation summary (from SatisfactionResultV1).
    pub test_summary: TestResultSummary,

    /// Sandbox resource usage.
    pub resource_usage: ResourceUsage,

    /// Whether the user approved this preview.
    pub approved: Option<bool>,
}

impl ArtifactPayload for PreviewSnapshotV1 {
    const TYPE_ID: &'static str = "planner.preview_snapshot.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Build status of the sandbox application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreviewBuildStatus {
    /// Build in progress.
    Building,
    /// Build succeeded, app is running.
    Running,
    /// Build failed.
    BuildFailed { error: String },
    /// App crashed after build.
    RuntimeError { error: String },
    /// Sandbox destroyed (cleanup completed).
    Destroyed,
}

/// Summary of scenario validation results for the preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultSummary {
    /// Total scenarios evaluated.
    pub total: u32,
    /// Scenarios that passed (majority pass).
    pub passed: u32,
    /// Scenarios that failed.
    pub failed: u32,
    /// Whether all tiered gates passed.
    pub gates_passed: bool,
}

/// Sandbox resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU cores allocated.
    pub cpu_cores: f32,
    /// Memory in MB.
    pub memory_mb: u32,
    /// Disk in MB.
    pub disk_mb: u32,
    /// Sandbox lifetime in seconds.
    pub lifetime_secs: u64,
}
