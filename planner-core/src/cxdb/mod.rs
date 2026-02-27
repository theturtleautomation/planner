//! # CXDB — Conversation Experience Database (Rust Server)
//!
//! Phase 5 replaces the SQLite sidecar with a proper CXDB implementation.
//! The CXDB is a purpose-built store for conversation turns with:
//!
//! - **Binary protocol**: MessagePack-framed writes over TCP for low-latency ingestion
//! - **HTTP reads**: JSON-over-HTTP for query flexibility and tooling compatibility
//! - **Content-addressed blobs**: Deduplication via BLAKE3 hashing
//! - **Turn DAG**: Parent-child relationships between turns form a directed acyclic graph
//! - **Multi-project**: Isolated namespaces per project with cross-project query support
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐
//! │  Pipeline Steps  │────▶│  CxdbClient       │
//! │  (write path)    │ TCP │  (binary proto)   │
//! └─────────────────┘     └────────┬───────────┘
//!                                  │
//!                           ┌──────▼───────┐
//!                           │   CxdbEngine  │
//!                           │   (storage)   │
//!                           └──────┬───────┘
//!                                  │
//! ┌─────────────────┐     ┌────────▼───────────┐
//! │  DCC / Tooling   │────▶│  HTTP read API     │
//! │  (read path)     │ GET │  /turns, /blobs    │
//! └─────────────────┘     └────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use planner_schemas::{ArtifactPayload, Turn, TurnMetadata};
use crate::storage::{TurnStore, StorageError};

pub mod protocol;
pub mod query;

// ---------------------------------------------------------------------------
// CXDB Engine — the core storage engine
// ---------------------------------------------------------------------------

/// CXDB storage engine — thread-safe, in-process store with content-addressed
/// blobs and a turn index. This is the core that both the TCP writer and
/// HTTP reader talk to.
#[derive(Debug)]
pub struct CxdbEngine {
    /// Turn metadata index: turn_id → StoredTurn
    turns: RwLock<HashMap<Uuid, StoredTurn>>,

    /// Content-addressed blob store: blob_hash → serialized bytes
    blobs: RwLock<HashMap<String, Vec<u8>>>,

    /// Index: (run_id, type_id) → Vec<turn_id> (ordered by created_at)
    run_type_index: RwLock<HashMap<(Uuid, String), Vec<Uuid>>>,

    /// Index: project_id → Vec<run_id>
    project_runs: RwLock<HashMap<Uuid, Vec<Uuid>>>,

    /// Server configuration
    config: CxdbConfig,
}

/// A stored turn record (metadata only — payload is in the blob store).
#[derive(Debug, Clone)]
struct StoredTurn {
    turn_id: Uuid,
    type_id: String,
    parent_id: Option<Uuid>,
    blob_hash: String,
    run_id: Uuid,
    execution_id: String,
    produced_by: String,
    created_at: DateTime<Utc>,
    note: Option<String>,
    /// Project ID extracted from the payload for multi-project indexing.
    project_id: Option<Uuid>,
}

/// CXDB server configuration.
#[derive(Debug, Clone)]
pub struct CxdbConfig {
    /// Maximum blob size in bytes (default: 16 MiB).
    pub max_blob_size: usize,

    /// Maximum number of turns before compaction triggers (default: 100_000).
    pub compaction_threshold: usize,

    /// Enable content-addressed deduplication (default: true).
    pub dedup_enabled: bool,
}

impl Default for CxdbConfig {
    fn default() -> Self {
        CxdbConfig {
            max_blob_size: 16 * 1024 * 1024,
            compaction_threshold: 100_000,
            dedup_enabled: true,
        }
    }
}

impl CxdbEngine {
    /// Create a new CXDB engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(CxdbConfig::default())
    }

    /// Create a new CXDB engine with custom configuration.
    pub fn with_config(config: CxdbConfig) -> Self {
        CxdbEngine {
            turns: RwLock::new(HashMap::new()),
            blobs: RwLock::new(HashMap::new()),
            run_type_index: RwLock::new(HashMap::new()),
            project_runs: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Wrap in an Arc for shared ownership across threads.
    pub fn into_shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Get engine statistics.
    pub fn stats(&self) -> CxdbStats {
        let turns = self.turns.read().unwrap();
        let blobs = self.blobs.read().unwrap();
        let projects = self.project_runs.read().unwrap();

        let total_blob_bytes: usize = blobs.values().map(|b| b.len()).sum();

        CxdbStats {
            total_turns: turns.len(),
            total_blobs: blobs.len(),
            total_blob_bytes,
            total_projects: projects.len(),
            dedup_savings: if turns.len() > blobs.len() {
                turns.len() - blobs.len()
            } else {
                0
            },
        }
    }

    /// List all run IDs for a given project.
    pub fn list_runs(&self, project_id: Uuid) -> Vec<Uuid> {
        let index = self.project_runs.read().unwrap();
        index.get(&project_id).cloned().unwrap_or_default()
    }

    /// List all turn IDs for a given run, optionally filtered by type.
    pub fn list_turn_ids(&self, run_id: Uuid, type_filter: Option<&str>) -> Vec<Uuid> {
        let index = self.run_type_index.read().unwrap();

        if let Some(type_id) = type_filter {
            index.get(&(run_id, type_id.to_string())).cloned().unwrap_or_default()
        } else {
            // Return all turn IDs for this run across all types
            index.iter()
                .filter(|((rid, _), _)| *rid == run_id)
                .flat_map(|(_, ids)| ids.iter().copied())
                .collect()
        }
    }

    /// Get raw blob bytes by hash.
    pub fn get_blob(&self, blob_hash: &str) -> Option<Vec<u8>> {
        let blobs = self.blobs.read().unwrap();
        blobs.get(blob_hash).cloned()
    }

    /// Register a project-run association.
    pub fn register_run(&self, project_id: Uuid, run_id: Uuid) {
        let mut index = self.project_runs.write().unwrap();
        let runs = index.entry(project_id).or_default();
        if !runs.contains(&run_id) {
            runs.push(run_id);
        }
    }

    /// Internal: store a turn record and its blob.
    fn store_turn_internal(
        &self,
        turn_id: Uuid,
        type_id: &str,
        parent_id: Option<Uuid>,
        blob_hash: &str,
        blob_data: Vec<u8>,
        run_id: Uuid,
        execution_id: &str,
        produced_by: &str,
        created_at: DateTime<Utc>,
        note: Option<String>,
        project_id: Option<Uuid>,
    ) -> Result<(), StorageError> {
        // Validate blob size
        if blob_data.len() > self.config.max_blob_size {
            return Err(StorageError::Serialization(format!(
                "Blob size {} exceeds max {}",
                blob_data.len(),
                self.config.max_blob_size,
            )));
        }

        // Store blob (content-addressed — dedup via hash)
        {
            let mut blobs = self.blobs.write().unwrap();
            if self.config.dedup_enabled {
                blobs.entry(blob_hash.to_string()).or_insert(blob_data);
            } else {
                blobs.insert(blob_hash.to_string(), blob_data);
            }
        }

        // Store turn record
        let stored = StoredTurn {
            turn_id,
            type_id: type_id.to_string(),
            parent_id,
            blob_hash: blob_hash.to_string(),
            run_id,
            execution_id: execution_id.to_string(),
            produced_by: produced_by.to_string(),
            created_at,
            note,
            project_id,
        };

        {
            let mut turns = self.turns.write().unwrap();
            turns.insert(turn_id, stored);
        }

        // Update run-type index
        {
            let mut index = self.run_type_index.write().unwrap();
            let key = (run_id, type_id.to_string());
            let ids = index.entry(key).or_default();
            ids.push(turn_id);
        }

        // Update project index if project_id is known
        if let Some(pid) = project_id {
            self.register_run(pid, run_id);
        }

        Ok(())
    }

    /// Reconstruct a Turn<T> from stored data.
    fn reconstruct_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        stored: &StoredTurn,
    ) -> Result<Turn<T>, StorageError> {
        let blobs = self.blobs.read().unwrap();
        let blob_data = blobs.get(&stored.blob_hash)
            .ok_or_else(|| StorageError::NotFound(stored.turn_id))?;

        let payload: T = rmp_serde::from_slice(blob_data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        Ok(Turn {
            turn_id: stored.turn_id,
            type_id: stored.type_id.clone(),
            parent_id: stored.parent_id,
            blob_hash: stored.blob_hash.clone(),
            payload,
            metadata: TurnMetadata {
                created_at: stored.created_at,
                produced_by: stored.produced_by.clone(),
                run_id: stored.run_id,
                execution_id: stored.execution_id.clone(),
                note: stored.note.clone(),
            },
        })
    }
}

/// CXDB engine statistics.
#[derive(Debug, Clone)]
pub struct CxdbStats {
    pub total_turns: usize,
    pub total_blobs: usize,
    pub total_blob_bytes: usize,
    pub total_projects: usize,
    pub dedup_savings: usize,
}

// ---------------------------------------------------------------------------
// TurnStore implementation — same trait as SQLite, drop-in replacement
// ---------------------------------------------------------------------------

impl TurnStore for CxdbEngine {
    fn store_turn<T: ArtifactPayload>(&self, turn: &Turn<T>) -> Result<(), StorageError> {
        let blob_data = rmp_serde::to_vec(&turn.payload)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.store_turn_internal(
            turn.turn_id,
            &turn.type_id,
            turn.parent_id,
            &turn.blob_hash,
            blob_data,
            turn.metadata.run_id,
            &turn.metadata.execution_id,
            &turn.metadata.produced_by,
            turn.metadata.created_at,
            turn.metadata.note.clone(),
            None, // project_id extracted at a higher level
        )
    }

    fn get_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        turn_id: Uuid,
    ) -> Result<Turn<T>, StorageError> {
        let turns = self.turns.read().unwrap();
        let stored = turns.get(&turn_id)
            .ok_or(StorageError::NotFound(turn_id))?;
        self.reconstruct_turn(stored)
    }

    fn get_turns_by_type<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Vec<Turn<T>>, StorageError> {
        let index = self.run_type_index.read().unwrap();
        let turn_ids = index.get(&(run_id, type_id.to_string()))
            .cloned()
            .unwrap_or_default();
        drop(index);

        // Collect stored turns first (clone to release the lock), then reconstruct
        let turns_lock = self.turns.read().unwrap();
        let stored_turns: Vec<StoredTurn> = turn_ids.iter()
            .filter_map(|tid| turns_lock.get(tid).cloned())
            .collect();
        drop(turns_lock);

        stored_turns.iter()
            .map(|st| self.reconstruct_turn(st))
            .collect()
    }

    fn get_latest_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Option<Turn<T>>, StorageError> {
        let index = self.run_type_index.read().unwrap();
        let turn_ids = index.get(&(run_id, type_id.to_string()))
            .cloned()
            .unwrap_or_default();
        drop(index);

        if turn_ids.is_empty() {
            return Ok(None);
        }

        // Find the one with the latest created_at
        let turns_lock = self.turns.read().unwrap();
        let latest = turn_ids.iter()
            .filter_map(|tid| turns_lock.get(tid))
            .max_by_key(|st| st.created_at)
            .cloned();
        drop(turns_lock);

        match latest {
            Some(stored) => Ok(Some(self.reconstruct_turn(&stored)?)),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use planner_schemas::*;

    fn make_test_intake(name: &str) -> IntakeV1 {
        IntakeV1 {
            project_id: Uuid::new_v4(),
            project_name: name.to_string(),
            feature_slug: "test-feature".into(),
            intent_summary: "A test intake".into(),
            output_domain: OutputDomain::MicroTool {
                variant: MicroToolVariant::ReactWidget,
            },
            environment: EnvironmentInfo {
                language: "TypeScript".into(),
                framework: "React".into(),
                package_manager: None,
                existing_dependencies: vec![],
                build_tool: None,
            },
            sacred_anchors: vec![],
            satisfaction_criteria_seeds: vec![],
            out_of_scope: vec![],
            conversation_log: vec![],
        }
    }

    #[test]
    fn cxdb_roundtrip_store_and_retrieve() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        let intake = make_test_intake("CXDB Test");
        let turn = Turn::new(intake, None, run_id, "cxdb-test", "exec-1");
        let turn_id = turn.turn_id;

        engine.store_turn(&turn).unwrap();

        let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
        assert_eq!(retrieved.turn_id, turn_id);
        assert_eq!(retrieved.payload.project_name, "CXDB Test");
        assert!(retrieved.verify_integrity());
    }

    #[test]
    fn cxdb_get_turns_by_type() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        for i in 0..3 {
            let intake = make_test_intake(&format!("Project {}", i));
            let turn = Turn::new(intake, None, run_id, "test", format!("exec-{}", i));
            engine.store_turn(&turn).unwrap();
        }

        let turns: Vec<Turn<IntakeV1>> = engine
            .get_turns_by_type(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert_eq!(turns.len(), 3);
    }

    #[test]
    fn cxdb_get_latest_turn() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        for i in 0..3 {
            let intake = make_test_intake(&format!("Project {}", i));
            let turn = Turn::new(intake, None, run_id, "test", format!("exec-{}", i));
            engine.store_turn(&turn).unwrap();
        }

        let latest: Option<Turn<IntakeV1>> = engine
            .get_latest_turn(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().payload.project_name, "Project 2");
    }

    #[test]
    fn cxdb_not_found_returns_error() {
        let engine = CxdbEngine::new();
        let result: Result<Turn<IntakeV1>, _> = engine.get_turn(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn cxdb_content_addressed_dedup() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        // Store the same payload twice — should dedup blobs
        let intake = make_test_intake("Dedup Test");
        let turn1 = Turn::new(intake.clone(), None, run_id, "test", "exec-1");
        let turn2 = Turn::new(intake, None, run_id, "test", "exec-2");

        // Both turns compute the same blob_hash for the same payload
        assert_eq!(turn1.blob_hash, turn2.blob_hash);

        engine.store_turn(&turn1).unwrap();
        engine.store_turn(&turn2).unwrap();

        let stats = engine.stats();
        assert_eq!(stats.total_turns, 2);
        assert_eq!(stats.total_blobs, 1); // Deduped!
        assert_eq!(stats.dedup_savings, 1);
    }

    #[test]
    fn cxdb_stats_tracking() {
        let engine = CxdbEngine::new();
        let stats = engine.stats();
        assert_eq!(stats.total_turns, 0);
        assert_eq!(stats.total_blobs, 0);

        let run_id = Uuid::new_v4();
        let intake = make_test_intake("Stats Test");
        let turn = Turn::new(intake, None, run_id, "test", "exec-1");
        engine.store_turn(&turn).unwrap();

        let stats = engine.stats();
        assert_eq!(stats.total_turns, 1);
        assert_eq!(stats.total_blobs, 1);
        assert!(stats.total_blob_bytes > 0);
    }

    #[test]
    fn cxdb_multi_project_runs() {
        let engine = CxdbEngine::new();
        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();
        let run_a1 = Uuid::new_v4();
        let run_a2 = Uuid::new_v4();
        let run_b1 = Uuid::new_v4();

        engine.register_run(project_a, run_a1);
        engine.register_run(project_a, run_a2);
        engine.register_run(project_b, run_b1);

        assert_eq!(engine.list_runs(project_a).len(), 2);
        assert_eq!(engine.list_runs(project_b).len(), 1);
        assert_eq!(engine.list_runs(Uuid::new_v4()).len(), 0);
    }

    #[test]
    fn cxdb_list_turn_ids() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        let intake = make_test_intake("List Test");
        let turn = Turn::new(intake, None, run_id, "test", "exec-1");
        engine.store_turn(&turn).unwrap();

        let ids = engine.list_turn_ids(run_id, Some(IntakeV1::TYPE_ID));
        assert_eq!(ids.len(), 1);

        let ids_all = engine.list_turn_ids(run_id, None);
        assert_eq!(ids_all.len(), 1);

        let ids_empty = engine.list_turn_ids(run_id, Some("nonexistent.type"));
        assert!(ids_empty.is_empty());
    }

    #[test]
    fn cxdb_blob_size_limit() {
        let engine = CxdbEngine::with_config(CxdbConfig {
            max_blob_size: 100, // Very small limit
            ..Default::default()
        });

        let run_id = Uuid::new_v4();
        let intake = make_test_intake("Big payload that will exceed the tiny limit");
        let turn = Turn::new(intake, None, run_id, "test", "exec-1");

        // This should fail because the serialized blob exceeds 100 bytes
        let result = engine.store_turn(&turn);
        assert!(result.is_err());
    }

    #[test]
    fn cxdb_empty_run_returns_empty() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        let turns: Vec<Turn<IntakeV1>> = engine
            .get_turns_by_type(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(turns.is_empty());

        let latest: Option<Turn<IntakeV1>> = engine
            .get_latest_turn(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn cxdb_parent_child_relationship() {
        let engine = CxdbEngine::new();
        let run_id = Uuid::new_v4();

        let parent_intake = make_test_intake("Parent");
        let parent_turn = Turn::new(parent_intake, None, run_id, "test", "exec-1");
        let parent_id = parent_turn.turn_id;
        engine.store_turn(&parent_turn).unwrap();

        let child_intake = make_test_intake("Child");
        let child_turn = Turn::new(child_intake, Some(parent_id), run_id, "test", "exec-2");
        let child_id = child_turn.turn_id;
        engine.store_turn(&child_turn).unwrap();

        let retrieved: Turn<IntakeV1> = engine.get_turn(child_id).unwrap();
        assert_eq!(retrieved.parent_id, Some(parent_id));
    }
}
