//! # Durable CXDB — Filesystem-Backed Content-Addressed Store
//!
//! Phase 6 replaces the in-memory `CxdbEngine` with a durable filesystem
//! store that persists Turn records as MessagePack blobs on disk.
//!
//! ## Layout
//!
//! ```text
//! <root>/
//!   blobs/
//!     ab/cd/abcdef0123456789...  ← BLAKE3 hash → MessagePack bytes
//!   turns/
//!     <run_id>/
//!       <type_id>/
//!         <turn_id>.msgpack      ← StoredTurnRecord metadata
//!   projects/
//!     <project_id>.msgpack       ← Vec<run_id>
//!   index.msgpack                ← Global turn→blob mapping (rebuild on startup)
//! ```
//!
//! Reads are lock-free (each file is immutable once written). Writes use
//! a directory-level advisory lock via `RwLock` to serialize concurrent
//! turn inserts within the same process.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use planner_schemas::{ArtifactPayload, Turn, TurnMetadata};
use super::{TurnStore, StorageError};

// ---------------------------------------------------------------------------
// On-disk metadata record (stored as MessagePack)
// ---------------------------------------------------------------------------

/// Metadata record persisted alongside the blob pointer.
/// This is what goes into `turns/<run_id>/<type_id>/<turn_id>.msgpack`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTurnRecord {
    turn_id: String,
    type_id: String,
    parent_id: Option<String>,
    blob_hash: String,
    run_id: String,
    execution_id: String,
    produced_by: String,
    created_at: String,       // RFC 3339
    note: Option<String>,
    project_id: Option<String>,
}

// ---------------------------------------------------------------------------
// DurableCxdbEngine
// ---------------------------------------------------------------------------

/// Filesystem-backed CXDB engine. Every Turn is persisted as:
/// - A content-addressed blob under `blobs/<2>/<2>/<hash>`
/// - A metadata record under `turns/<run_id>/<type_id>/<turn_id>.msgpack`
#[derive(Debug)]
pub struct DurableCxdbEngine {
    /// Root directory on the filesystem.
    root: PathBuf,

    /// In-memory index: (run_id, type_id) → Vec<turn_id>  (rebuilt from disk).
    run_type_index: RwLock<HashMap<(Uuid, String), Vec<Uuid>>>,

    /// In-memory turn metadata cache: turn_id → StoredTurnRecord.
    turn_cache: RwLock<HashMap<Uuid, StoredTurnRecord>>,

    /// Write serialization lock.
    write_lock: RwLock<()>,
}

impl DurableCxdbEngine {
    /// Open or create a durable CXDB at the given root directory.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();

        // Ensure directory structure exists
        fs::create_dir_all(root.join("blobs"))
            .map_err(|e| StorageError::Serialization(format!("Cannot create blobs dir: {}", e)))?;
        fs::create_dir_all(root.join("turns"))
            .map_err(|e| StorageError::Serialization(format!("Cannot create turns dir: {}", e)))?;
        fs::create_dir_all(root.join("projects"))
            .map_err(|e| StorageError::Serialization(format!("Cannot create projects dir: {}", e)))?;

        let engine = DurableCxdbEngine {
            root,
            run_type_index: RwLock::new(HashMap::new()),
            turn_cache: RwLock::new(HashMap::new()),
            write_lock: RwLock::new(()),
        };

        // Rebuild in-memory indices from disk
        engine.rebuild_indices()?;

        Ok(engine)
    }

    /// Open a CXDB in a temporary directory (for testing).
    #[cfg(test)]
    pub fn open_temp() -> Result<Self, StorageError> {
        let dir = std::env::temp_dir().join(format!("cxdb-test-{}", Uuid::new_v4()));
        Self::open(dir)
    }

    /// Get the root directory.
    pub fn root_path(&self) -> &Path {
        &self.root
    }

    // -----------------------------------------------------------------------
    // Blob I/O
    // -----------------------------------------------------------------------

    /// Content-addressed blob path: `blobs/<first 2 chars>/<next 2 chars>/<full hash>`
    fn blob_path(&self, hash: &str) -> PathBuf {
        let (prefix_a, rest) = hash.split_at(2.min(hash.len()));
        let (prefix_b, _) = rest.split_at(2.min(rest.len()));
        self.root.join("blobs").join(prefix_a).join(prefix_b).join(hash)
    }

    /// Write blob bytes to disk (no-op if already exists — content-addressed).
    fn write_blob(&self, hash: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.blob_path(hash);
        if path.exists() {
            return Ok(()); // Already stored — dedup
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| StorageError::Serialization(format!("Blob dir: {}", e)))?;
        }
        fs::write(&path, data)
            .map_err(|e| StorageError::Serialization(format!("Write blob: {}", e)))?;
        Ok(())
    }

    /// Read blob bytes from disk.
    fn read_blob(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.blob_path(hash);
        fs::read(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => StorageError::Serialization(
                format!("Blob not found: {}", hash),
            ),
            _ => StorageError::Serialization(format!("Read blob: {}", e)),
        })
    }

    // -----------------------------------------------------------------------
    // Turn metadata I/O
    // -----------------------------------------------------------------------

    /// Turn metadata path: `turns/<run_id>/<type_id>/<turn_id>.msgpack`
    fn turn_meta_path(&self, run_id: Uuid, type_id: &str, turn_id: Uuid) -> PathBuf {
        self.root
            .join("turns")
            .join(run_id.to_string())
            .join(type_id)
            .join(format!("{}.msgpack", turn_id))
    }

    /// Write a turn metadata record to disk.
    fn write_turn_record(&self, record: &StoredTurnRecord) -> Result<(), StorageError> {
        let run_id = Uuid::parse_str(&record.run_id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let turn_id = Uuid::parse_str(&record.turn_id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let path = self.turn_meta_path(run_id, &record.type_id, turn_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| StorageError::Serialization(format!("Turn dir: {}", e)))?;
        }

        let data = rmp_serde::to_vec(record)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        fs::write(&path, data)
            .map_err(|e| StorageError::Serialization(format!("Write turn: {}", e)))?;
        Ok(())
    }

    /// Read a turn metadata record from a path.
    fn read_turn_record(path: &Path) -> Result<StoredTurnRecord, StorageError> {
        let data = fs::read(path)
            .map_err(|e| StorageError::Serialization(format!("Read turn: {}", e)))?;
        rmp_serde::from_slice(&data)
            .map_err(|e| StorageError::Serialization(e.to_string()))
    }

    // -----------------------------------------------------------------------
    // Project index I/O
    // -----------------------------------------------------------------------

    /// Register a project→run association.
    pub fn register_run(&self, project_id: Uuid, run_id: Uuid) -> Result<(), StorageError> {
        let path = self.root.join("projects").join(format!("{}.msgpack", project_id));

        let mut runs: Vec<String> = if path.exists() {
            let data = fs::read(&path)
                .map_err(|e| StorageError::Serialization(format!("Read project: {}", e)))?;
            rmp_serde::from_slice(&data)
                .map_err(|e| StorageError::Serialization(e.to_string()))?
        } else {
            Vec::new()
        };

        let run_str = run_id.to_string();
        if !runs.contains(&run_str) {
            runs.push(run_str);
        }

        let data = rmp_serde::to_vec(&runs)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        fs::write(&path, data)
            .map_err(|e| StorageError::Serialization(format!("Write project: {}", e)))?;
        Ok(())
    }

    /// List all run IDs for a given project.
    pub fn list_runs(&self, project_id: Uuid) -> Vec<Uuid> {
        let path = self.root.join("projects").join(format!("{}.msgpack", project_id));
        if !path.exists() {
            return Vec::new();
        }

        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let runs: Vec<String> = match rmp_serde::from_slice(&data) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        runs.iter()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect()
    }

    // -----------------------------------------------------------------------
    // Index rebuild (startup)
    // -----------------------------------------------------------------------

    /// Walk the `turns/` directory tree and rebuild in-memory indices.
    fn rebuild_indices(&self) -> Result<(), StorageError> {
        let turns_dir = self.root.join("turns");
        if !turns_dir.exists() {
            return Ok(());
        }

        let mut index = self.run_type_index.write().unwrap();
        let mut cache = self.turn_cache.write().unwrap();

        // Walk turns/<run_id>/<type_id>/<turn_id>.msgpack
        for run_entry in Self::read_dir_sorted(&turns_dir)? {
            let run_path = run_entry;
            let run_id_str = run_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let run_id = match Uuid::parse_str(run_id_str) {
                Ok(id) => id,
                Err(_) => continue,
            };

            for type_entry in Self::read_dir_sorted(&run_path)? {
                let type_path = type_entry;
                let type_id = type_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                for turn_file in Self::read_dir_sorted(&type_path)? {
                    if turn_file.extension().and_then(|e| e.to_str()) != Some("msgpack") {
                        continue;
                    }
                    if let Ok(record) = Self::read_turn_record(&turn_file) {
                        if let Ok(turn_id) = Uuid::parse_str(&record.turn_id) {
                            index
                                .entry((run_id, type_id.clone()))
                                .or_default()
                                .push(turn_id);
                            cache.insert(turn_id, record);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Helper: read directory entries sorted by name.
    fn read_dir_sorted(path: &Path) -> Result<Vec<PathBuf>, StorageError> {
        if !path.is_dir() {
            return Ok(Vec::new());
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|e| StorageError::Serialization(format!("Read dir: {}", e)))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        entries.sort();
        Ok(entries)
    }

    // -----------------------------------------------------------------------
    // Reconstruct Turn<T> from record + blob
    // -----------------------------------------------------------------------

    fn reconstruct_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        record: &StoredTurnRecord,
    ) -> Result<Turn<T>, StorageError> {
        let blob_data = self.read_blob(&record.blob_hash)?;

        let payload: T = rmp_serde::from_slice(&blob_data)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let turn_id = Uuid::parse_str(&record.turn_id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let parent_id = record.parent_id.as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let run_id = Uuid::parse_str(&record.run_id)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let created_at: DateTime<Utc> = record.created_at.parse()
            .map_err(|e: chrono::ParseError| StorageError::Serialization(e.to_string()))?;

        Ok(Turn {
            turn_id,
            type_id: record.type_id.clone(),
            parent_id,
            blob_hash: record.blob_hash.clone(),
            payload,
            metadata: TurnMetadata {
                created_at,
                produced_by: record.produced_by.clone(),
                run_id,
                execution_id: record.execution_id.clone(),
                note: record.note.clone(),
                project_id: None,
            },
        })
    }

    /// Engine statistics.
    pub fn stats(&self) -> DurableCxdbStats {
        let cache = self.turn_cache.read().unwrap();

        // Count blobs on disk
        let blob_count = Self::count_files_recursive(&self.root.join("blobs"));

        DurableCxdbStats {
            total_turns: cache.len(),
            total_blobs: blob_count,
            root_path: self.root.display().to_string(),
        }
    }

    /// List all turn metadata for a given project ID.
    ///
    /// Iterates all runs belonging to the project and collects turn metadata
    /// from the in-memory cache. Returns lightweight metadata without
    /// deserializing blob payloads.
    pub fn list_turn_metadata_for_project(&self, project_id: Uuid) -> Vec<TurnMetadataView> {
        let runs = self.list_runs(project_id);
        if runs.is_empty() {
            return Vec::new();
        }

        let cache = self.turn_cache.read().unwrap();
        let run_set: std::collections::HashSet<String> =
            runs.iter().map(|r| r.to_string()).collect();

        cache
            .values()
            .filter(|record| run_set.contains(&record.run_id))
            .map(|record| TurnMetadataView {
                turn_id: record.turn_id.clone(),
                type_id: record.type_id.clone(),
                timestamp: record.created_at.clone(),
                produced_by: record.produced_by.clone(),
            })
            .collect()
    }

    fn count_files_recursive(dir: &Path) -> usize {
        if !dir.is_dir() {
            return 0;
        }
        let mut count = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    count += Self::count_files_recursive(&path);
                } else {
                    count += 1;
                }
            }
        }
        count
    }
}

/// Statistics for the durable CXDB.
#[derive(Debug, Clone)]
pub struct DurableCxdbStats {
    pub total_turns: usize,
    pub total_blobs: usize,
    pub root_path: String,
}

/// Lightweight turn metadata view for list endpoints.
/// Does not include the deserialized payload — just enough to populate
/// the REST API response.
#[derive(Debug, Clone)]
pub struct TurnMetadataView {
    pub turn_id: String,
    pub type_id: String,
    pub timestamp: String,
    pub produced_by: String,
}

// ---------------------------------------------------------------------------
// TurnStore implementation — drop-in replacement for CxdbEngine
// ---------------------------------------------------------------------------

impl TurnStore for DurableCxdbEngine {
    fn store_turn<T: ArtifactPayload>(&self, turn: &Turn<T>) -> Result<(), StorageError> {
        let _write = self.write_lock.write().unwrap();

        // Serialize payload to MessagePack
        let blob_data = rmp_serde::to_vec(&turn.payload)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Write content-addressed blob
        self.write_blob(&turn.blob_hash, &blob_data)?;

        // Build metadata record
        let record = StoredTurnRecord {
            turn_id: turn.turn_id.to_string(),
            type_id: turn.type_id.clone(),
            parent_id: turn.parent_id.map(|id| id.to_string()),
            blob_hash: turn.blob_hash.clone(),
            run_id: turn.metadata.run_id.to_string(),
            execution_id: turn.metadata.execution_id.clone(),
            produced_by: turn.metadata.produced_by.clone(),
            created_at: turn.metadata.created_at.to_rfc3339(),
            note: turn.metadata.note.clone(),
            project_id: None,
        };

        // Write metadata to disk
        self.write_turn_record(&record)?;

        // Update in-memory index
        {
            let mut idx = self.run_type_index.write().unwrap();
            idx.entry((turn.metadata.run_id, turn.type_id.clone()))
                .or_default()
                .push(turn.turn_id);
        }

        // Update in-memory cache
        {
            let mut cache = self.turn_cache.write().unwrap();
            cache.insert(turn.turn_id, record);
        }

        Ok(())
    }

    fn get_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        turn_id: Uuid,
    ) -> Result<Turn<T>, StorageError> {
        let cache = self.turn_cache.read().unwrap();
        let record = cache.get(&turn_id)
            .ok_or(StorageError::NotFound(turn_id))?
            .clone();
        drop(cache);

        self.reconstruct_turn(&record)
    }

    fn get_turns_by_type<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Vec<Turn<T>>, StorageError> {
        let idx = self.run_type_index.read().unwrap();
        let turn_ids = idx.get(&(run_id, type_id.to_string()))
            .cloned()
            .unwrap_or_default();
        drop(idx);

        let cache = self.turn_cache.read().unwrap();
        let records: Vec<StoredTurnRecord> = turn_ids.iter()
            .filter_map(|tid| cache.get(tid).cloned())
            .collect();
        drop(cache);

        records.iter()
            .map(|r| self.reconstruct_turn(r))
            .collect()
    }

    fn get_latest_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Option<Turn<T>>, StorageError> {
        let idx = self.run_type_index.read().unwrap();
        let turn_ids = idx.get(&(run_id, type_id.to_string()))
            .cloned()
            .unwrap_or_default();
        drop(idx);

        if turn_ids.is_empty() {
            return Ok(None);
        }

        let cache = self.turn_cache.read().unwrap();
        let latest = turn_ids.iter()
            .filter_map(|tid| cache.get(tid))
            .max_by_key(|r| r.created_at.clone())
            .cloned();
        drop(cache);

        match latest {
            Some(record) => Ok(Some(self.reconstruct_turn(&record)?)),
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
    fn durable_cxdb_roundtrip() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let run_id = Uuid::new_v4();

        let intake = make_test_intake("Durable CXDB Test");
        let turn = Turn::new(intake, None, run_id, "durable-test", "exec-1");
        let turn_id = turn.turn_id;

        engine.store_turn(&turn).unwrap();

        let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
        assert_eq!(retrieved.turn_id, turn_id);
        assert_eq!(retrieved.payload.project_name, "Durable CXDB Test");
        assert!(retrieved.verify_integrity());

        // Clean up
        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_persistence_across_opens() {
        let dir = std::env::temp_dir().join(format!("cxdb-persist-{}", Uuid::new_v4()));
        let run_id = Uuid::new_v4();
        let turn_id;

        // Write with first engine instance
        {
            let engine = DurableCxdbEngine::open(&dir).unwrap();
            let intake = make_test_intake("Persistence Test");
            let turn = Turn::new(intake, None, run_id, "persist-test", "exec-1");
            turn_id = turn.turn_id;
            engine.store_turn(&turn).unwrap();
        }

        // Read with second engine instance (new process simulation)
        {
            let engine = DurableCxdbEngine::open(&dir).unwrap();
            let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
            assert_eq!(retrieved.payload.project_name, "Persistence Test");
            assert!(retrieved.verify_integrity());
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn durable_cxdb_get_turns_by_type() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let run_id = Uuid::new_v4();

        for i in 0..3 {
            let intake = make_test_intake(&format!("Project {}", i));
            let turn = Turn::new(intake, None, run_id, "type-test", format!("exec-{}", i));
            engine.store_turn(&turn).unwrap();
        }

        let turns: Vec<Turn<IntakeV1>> = engine
            .get_turns_by_type(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert_eq!(turns.len(), 3);

        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_get_latest_turn() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let run_id = Uuid::new_v4();

        for i in 0..3 {
            let intake = make_test_intake(&format!("Project {}", i));
            let turn = Turn::new(intake, None, run_id, "latest-test", format!("exec-{}", i));
            engine.store_turn(&turn).unwrap();
        }

        let latest: Option<Turn<IntakeV1>> = engine
            .get_latest_turn(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().payload.project_name, "Project 2");

        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_content_addressed_dedup() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let run_id = Uuid::new_v4();

        let intake = make_test_intake("Dedup Test");
        let turn1 = Turn::new(intake.clone(), None, run_id, "dedup-test", "exec-1");
        let turn2 = Turn::new(intake, None, run_id, "dedup-test", "exec-2");

        // Same payload → same blob hash
        assert_eq!(turn1.blob_hash, turn2.blob_hash);

        engine.store_turn(&turn1).unwrap();
        engine.store_turn(&turn2).unwrap();

        let stats = engine.stats();
        assert_eq!(stats.total_turns, 2);
        assert_eq!(stats.total_blobs, 1); // Deduped on disk!

        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_not_found() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let result: Result<Turn<IntakeV1>, _> = engine.get_turn(Uuid::new_v4());
        assert!(result.is_err());

        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_project_runs() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();
        let run_a1 = Uuid::new_v4();
        let run_a2 = Uuid::new_v4();
        let run_b1 = Uuid::new_v4();

        engine.register_run(project_a, run_a1).unwrap();
        engine.register_run(project_a, run_a2).unwrap();
        engine.register_run(project_b, run_b1).unwrap();

        assert_eq!(engine.list_runs(project_a).len(), 2);
        assert_eq!(engine.list_runs(project_b).len(), 1);
        assert_eq!(engine.list_runs(Uuid::new_v4()).len(), 0);

        let _ = fs::remove_dir_all(engine.root_path());
    }

    #[test]
    fn durable_cxdb_empty_run() {
        let engine = DurableCxdbEngine::open_temp().unwrap();
        let run_id = Uuid::new_v4();

        let turns: Vec<Turn<IntakeV1>> = engine
            .get_turns_by_type(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(turns.is_empty());

        let latest: Option<Turn<IntakeV1>> = engine
            .get_latest_turn(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(latest.is_none());

        let _ = fs::remove_dir_all(engine.root_path());
    }
}
