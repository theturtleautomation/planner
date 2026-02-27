//! # Pyramid Summaries — Hierarchical Context Compression
//!
//! For large projects with many CXDB turns, Pyramid Summaries provide
//! a tiered compression scheme that the DCC (Dynamic Context Compiler)
//! uses to route Consequence Cards and build Context Packs efficiently.
//!
//! ## Three Tiers
//!
//! 1. **Leaf** — Individual turn summaries (1-2 sentences each)
//! 2. **Branch** — Group summaries aggregating 10-20 leaves (1 paragraph)
//! 3. **Root** — Project-level summary aggregating all branches (1 page)
//!
//! The DCC traverses top-down: Root → relevant Branch → relevant Leaves.
//! This prevents context window exhaustion on large projects.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// PyramidSummaryV1
// ---------------------------------------------------------------------------

/// A Pyramid Summary — hierarchical context compression for large projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidSummaryV1 {
    /// Which project this summary covers.
    pub project_id: Uuid,

    /// Summary tier.
    pub tier: PyramidTier,

    /// Unique ID for this summary node.
    pub node_id: Uuid,

    /// Parent node ID (None for Root tier).
    pub parent_id: Option<Uuid>,

    /// Child node IDs (empty for Leaf tier).
    pub children: Vec<Uuid>,

    /// The compressed summary text.
    pub summary: String,

    /// Token count of the summary text.
    pub token_count: u32,

    /// CXDB turn IDs covered by this summary node.
    pub covered_turn_ids: Vec<Uuid>,

    /// Keywords/topics extracted for routing.
    pub topics: Vec<String>,

    /// When this summary was last refreshed.
    pub refreshed_at: String,

    /// Whether this summary is stale (underlying turns changed since refresh).
    pub stale: bool,
}

impl ArtifactPayload for PyramidSummaryV1 {
    const TYPE_ID: &'static str = "planner.pyramid_summary.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Pyramid Summary tier — determines compression level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PyramidTier {
    /// Individual turn summary (1-2 sentences).
    Leaf,
    /// Group summary covering 10-20 leaves (1 paragraph).
    Branch,
    /// Project-level summary covering all branches (1 page).
    Root,
}

impl PyramidTier {
    /// Target token count for this tier's summary.
    pub fn target_tokens(&self) -> u32 {
        match self {
            PyramidTier::Leaf => 50,
            PyramidTier::Branch => 200,
            PyramidTier::Root => 800,
        }
    }

    /// Maximum number of children per node at this tier.
    pub fn max_children(&self) -> usize {
        match self {
            PyramidTier::Leaf => 0,    // Leaves have no children
            PyramidTier::Branch => 20, // Each branch covers up to 20 leaves
            PyramidTier::Root => 50,   // Root covers up to 50 branches
        }
    }
}

/// Configuration for Pyramid Summary generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidConfig {
    /// Minimum number of turns before pyramid is triggered.
    pub min_turns_for_pyramid: usize,

    /// How many leaves per branch (default: 15).
    pub leaves_per_branch: usize,

    /// Maximum staleness before forced refresh (seconds).
    pub max_staleness_secs: u64,

    /// Whether to auto-refresh on Context Pack generation.
    pub auto_refresh: bool,
}

impl Default for PyramidConfig {
    fn default() -> Self {
        PyramidConfig {
            min_turns_for_pyramid: 50,
            leaves_per_branch: 15,
            max_staleness_secs: 3600, // 1 hour
            auto_refresh: true,
        }
    }
}
