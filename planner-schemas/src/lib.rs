//! # planner-schemas
//!
//! Type definitions for all CXDB artifacts in Planner v2.
//!
//! This crate defines:
//! - `Turn<T>` — the immutable, content-addressed wrapper for all state
//! - `ArtifactPayload` — the trait every typed artifact implements
//! - All artifact types in the CXDB type registry
//!
//! ## CXDB Type Registry
//!
//! | Type ID | Struct |
//! |---|---|
//! | `planner.intake.v1` | `IntakeV1` |
//! | `planner.nlspec.v1` | `NLSpecV1` |
//! | `planner.graph_dot.v1` | `GraphDotV1` |
//! | `planner.scenario_set.v1` | `ScenarioSetV1` |
//! | `planner.factory_output.v1` | `FactoryOutputV1` |
//! | `planner.satisfaction_result.v1` | `SatisfactionResultV1` |
//! | `planner.run_budget.v1` | `RunBudgetV1` |
//! | `planner.agents_manifest.v1` | `AgentsManifestV1` |
//! | `planner.ar_report.v1` | `ArReportV1` |
//! | `planner.consequence_card.v1` | `ConsequenceCardV1` |
//! | `planner.preview_snapshot.v1` | `PreviewSnapshotV1` |
//! | `planner.ralph_finding.v1` | `RalphFindingV1` |
//! | `planner.git_commit.v1` | `GitCommitV1` |
//! | `planner.gate_result.v1` | `GateResultV1` |
//! | `planner.decision.v1` | `DecisionV1` |
//! | `planner.context_pack.v1` | `ContextPackV1` |
//! | `planner.dtu_config.v1` | `DtuConfigV1` |
//! | `planner.pyramid_summary.v1` | `PyramidSummaryV1` |

pub mod artifacts;
pub mod turn;

// Re-export the core trait and Turn for convenience.
pub use turn::{ArtifactPayload, Turn, TurnMetadata};

// Re-export all artifact types at the crate root for ergonomic access.
pub use artifacts::agents_manifest::*;
pub use artifacts::ar_report::*;
pub use artifacts::consequence_card::*;
pub use artifacts::dtu::*;
pub use artifacts::factory_output::*;
pub use artifacts::git_commit::*;
pub use artifacts::graph_dot::*;
pub use artifacts::intake::*;
pub use artifacts::nlspec::*;
pub use artifacts::preview_snapshot::*;
pub use artifacts::pyramid_summary::*;
pub use artifacts::ralph_finding::*;
pub use artifacts::run_budget::*;
pub use artifacts::runtime::*;
pub use artifacts::satisfaction_result::*;
pub use artifacts::scenario_set::*;
