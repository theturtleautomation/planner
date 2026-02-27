//! # CXDB Query Engine
//!
//! HTTP-oriented query layer for CXDB reads. Provides structured queries
//! over the turn DAG, including:
//!
//! - Turn-by-ID lookup
//! - Turns-by-type within a run
//! - DAG traversal (ancestors, descendants)
//! - Cross-run queries (find all specs across runs for a project)
//! - Timeline views (ordered by created_at)

use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// Query types
// ---------------------------------------------------------------------------

/// A query against the CXDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CxdbQuery {
    /// Get a single turn by ID.
    GetTurn { turn_id: Uuid },

    /// List turns for a run, optionally filtered by type.
    ListTurns {
        run_id: Uuid,
        type_filter: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    },

    /// Get all ancestors of a turn (walk parent_id chain).
    Ancestors {
        turn_id: Uuid,
        max_depth: Option<usize>,
    },

    /// Get all descendants of a turn (turns with this as ancestor).
    Descendants {
        turn_id: Uuid,
        max_depth: Option<usize>,
    },

    /// Timeline view: all turns in a run ordered by created_at.
    Timeline {
        run_id: Uuid,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    },

    /// List all runs for a project.
    ProjectRuns {
        project_id: Uuid,
    },

    /// Cross-run query: find turns of a given type across all runs in a project.
    CrossRunQuery {
        project_id: Uuid,
        type_id: String,
        limit: Option<usize>,
    },
}

/// A lightweight turn summary for query results (no payload blob).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: Uuid,
    pub type_id: String,
    pub parent_id: Option<Uuid>,
    pub blob_hash: String,
    pub run_id: Uuid,
    pub execution_id: String,
    pub produced_by: String,
    pub created_at: DateTime<Utc>,
    pub note: Option<String>,
}

/// Query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The turns matching the query.
    pub turns: Vec<TurnSummary>,

    /// Total count (may differ from turns.len() if limit/offset used).
    pub total_count: usize,

    /// Whether more results are available beyond the limit.
    pub has_more: bool,
}

impl QueryResult {
    /// Empty result.
    pub fn empty() -> Self {
        QueryResult {
            turns: vec![],
            total_count: 0,
            has_more: false,
        }
    }

    /// Single-turn result.
    pub fn single(summary: TurnSummary) -> Self {
        QueryResult {
            turns: vec![summary],
            total_count: 1,
            has_more: false,
        }
    }

    /// Multi-turn result with pagination info.
    pub fn paginated(turns: Vec<TurnSummary>, total_count: usize, has_more: bool) -> Self {
        QueryResult {
            turns,
            total_count,
            has_more,
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP API route definitions (for documentation and routing)
// ---------------------------------------------------------------------------

/// HTTP API routes for the CXDB read layer.
///
/// These are constants defining the URL patterns — the actual HTTP server
/// will be wired up when we add a web framework (Phase 5+).
pub mod routes {
    /// GET /api/v1/turns/:turn_id — Get a single turn
    pub const GET_TURN: &str = "/api/v1/turns/:turn_id";

    /// GET /api/v1/runs/:run_id/turns?type=...&limit=...&offset=...
    pub const LIST_TURNS: &str = "/api/v1/runs/:run_id/turns";

    /// GET /api/v1/turns/:turn_id/ancestors?max_depth=...
    pub const ANCESTORS: &str = "/api/v1/turns/:turn_id/ancestors";

    /// GET /api/v1/turns/:turn_id/descendants?max_depth=...
    pub const DESCENDANTS: &str = "/api/v1/turns/:turn_id/descendants";

    /// GET /api/v1/runs/:run_id/timeline?since=...&until=...
    pub const TIMELINE: &str = "/api/v1/runs/:run_id/timeline";

    /// GET /api/v1/projects/:project_id/runs
    pub const PROJECT_RUNS: &str = "/api/v1/projects/:project_id/runs";

    /// GET /api/v1/projects/:project_id/turns?type=...&limit=...
    pub const CROSS_RUN: &str = "/api/v1/projects/:project_id/turns";

    /// GET /api/v1/blobs/:blob_hash — Raw blob retrieval
    pub const GET_BLOB: &str = "/api/v1/blobs/:blob_hash";

    /// GET /api/v1/stats — Engine statistics
    pub const STATS: &str = "/api/v1/stats";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_result_empty() {
        let result = QueryResult::empty();
        assert!(result.turns.is_empty());
        assert_eq!(result.total_count, 0);
        assert!(!result.has_more);
    }

    #[test]
    fn query_result_single() {
        let summary = TurnSummary {
            turn_id: Uuid::new_v4(),
            type_id: "planner.intake.v1".into(),
            parent_id: None,
            blob_hash: "abc123".into(),
            run_id: Uuid::new_v4(),
            execution_id: "exec-1".into(),
            produced_by: "test".into(),
            created_at: Utc::now(),
            note: None,
        };

        let result = QueryResult::single(summary.clone());
        assert_eq!(result.turns.len(), 1);
        assert_eq!(result.turns[0].turn_id, summary.turn_id);
    }

    #[test]
    fn query_result_paginated() {
        let summaries: Vec<TurnSummary> = (0..5).map(|i| TurnSummary {
            turn_id: Uuid::new_v4(),
            type_id: "test".into(),
            parent_id: None,
            blob_hash: format!("hash_{}", i),
            run_id: Uuid::new_v4(),
            execution_id: format!("exec-{}", i),
            produced_by: "test".into(),
            created_at: Utc::now(),
            note: None,
        }).collect();

        let result = QueryResult::paginated(summaries, 10, true);
        assert_eq!(result.turns.len(), 5);
        assert_eq!(result.total_count, 10);
        assert!(result.has_more);
    }

    #[test]
    fn cxdb_query_serialization() {
        let query = CxdbQuery::ListTurns {
            run_id: Uuid::new_v4(),
            type_filter: Some("planner.intake.v1".into()),
            limit: Some(10),
            offset: Some(0),
        };

        let json = serde_json::to_string(&query).unwrap();
        let decoded: CxdbQuery = serde_json::from_str(&json).unwrap();

        if let CxdbQuery::ListTurns { type_filter, limit, .. } = decoded {
            assert_eq!(type_filter.unwrap(), "planner.intake.v1");
            assert_eq!(limit.unwrap(), 10);
        } else {
            panic!("Wrong query type");
        }
    }

    #[test]
    fn turn_summary_serialization() {
        let summary = TurnSummary {
            turn_id: Uuid::new_v4(),
            type_id: "test".into(),
            parent_id: Some(Uuid::new_v4()),
            blob_hash: "abc".into(),
            run_id: Uuid::new_v4(),
            execution_id: "exec".into(),
            produced_by: "test".into(),
            created_at: Utc::now(),
            note: Some("a note".into()),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let decoded: TurnSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.turn_id, summary.turn_id);
        assert_eq!(decoded.parent_id, summary.parent_id);
    }

    #[test]
    fn routes_are_defined() {
        // Ensure all routes are non-empty strings
        assert!(!routes::GET_TURN.is_empty());
        assert!(!routes::LIST_TURNS.is_empty());
        assert!(!routes::ANCESTORS.is_empty());
        assert!(!routes::TIMELINE.is_empty());
        assert!(!routes::PROJECT_RUNS.is_empty());
        assert!(!routes::CROSS_RUN.is_empty());
        assert!(!routes::GET_BLOB.is_empty());
        assert!(!routes::STATS.is_empty());
    }
}
