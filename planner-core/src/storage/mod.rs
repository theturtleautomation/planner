//! # Storage — SQLite Sidecar (CXDB Phase 0 Substitute)
//!
//! Phase 0 uses SQLite to store Turn records and blobs, mirroring the
//! real CXDB interface. Phase 5 swaps in the full Rust CXDB server —
//! only the driver changes; the trait boundary stays identical.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use planner_schemas::{ArtifactPayload, Turn, TurnMetadata};

// ---------------------------------------------------------------------------
// Storage trait — the boundary that survives the Phase 5 CXDB swap
// ---------------------------------------------------------------------------

/// The storage interface that both SQLite (Phase 0) and CXDB (Phase 5)
/// implement. All engine code talks to this trait, never to a concrete store.
pub trait TurnStore {
    /// Persist a turn and its blob.
    fn store_turn<T: ArtifactPayload>(&self, turn: &Turn<T>) -> Result<(), StorageError>;

    /// Retrieve a turn by ID.
    fn get_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        turn_id: Uuid,
    ) -> Result<Turn<T>, StorageError>;

    /// Retrieve all turns of a given type for a run.
    fn get_turns_by_type<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Vec<Turn<T>>, StorageError>;

    /// Get the latest turn of a given type for a run.
    fn get_latest_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Option<Turn<T>>, StorageError>;
}

/// Storage errors.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Turn not found: {0}")]
    NotFound(Uuid),

    #[error("Integrity check failed for turn {0}")]
    IntegrityFailure(Uuid),
}

// ---------------------------------------------------------------------------
// SQLite implementation
// ---------------------------------------------------------------------------

/// Phase 0 CXDB substitute using SQLite.
pub struct SqliteTurnStore {
    conn: Connection,
}

impl SqliteTurnStore {
    /// Create a new SQLite store, initializing the schema.
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS turns (
                turn_id     TEXT PRIMARY KEY,
                type_id     TEXT NOT NULL,
                parent_id   TEXT,
                blob_hash   TEXT NOT NULL,
                run_id      TEXT NOT NULL,
                execution_id TEXT NOT NULL,
                produced_by TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                note        TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_turns_run_type
                ON turns(run_id, type_id);

            CREATE INDEX IF NOT EXISTS idx_turns_type
                ON turns(type_id);

            CREATE TABLE IF NOT EXISTS blobs (
                blob_hash   TEXT PRIMARY KEY,
                data        BLOB NOT NULL
            );
            ",
        )?;
        Ok(SqliteTurnStore { conn })
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::new(":memory:")
    }
}

impl TurnStore for SqliteTurnStore {
    fn store_turn<T: ArtifactPayload>(&self, turn: &Turn<T>) -> Result<(), StorageError> {
        let blob_data = rmp_serde::to_vec(&turn.payload)
            .map_err(|e: rmp_serde::encode::Error| StorageError::Serialization(e.to_string()))?;

        // Store blob (content-addressed — idempotent)
        self.conn.execute(
            "INSERT OR IGNORE INTO blobs (blob_hash, data) VALUES (?1, ?2)",
            params![turn.blob_hash, blob_data],
        )?;

        // Store turn record
        self.conn.execute(
            "INSERT OR IGNORE INTO turns
                (turn_id, type_id, parent_id, blob_hash, run_id,
                 execution_id, produced_by, created_at, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                turn.turn_id.to_string(),
                turn.type_id,
                turn.parent_id.map(|id| id.to_string()),
                turn.blob_hash,
                turn.metadata.run_id.to_string(),
                turn.metadata.execution_id,
                turn.metadata.produced_by,
                turn.metadata.created_at.to_rfc3339(),
                turn.metadata.note,
            ],
        )?;

        Ok(())
    }

    fn get_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        turn_id: Uuid,
    ) -> Result<Turn<T>, StorageError> {
        let turn_id_str = turn_id.to_string();

        let row = self.conn.query_row(
            "SELECT t.turn_id, t.type_id, t.parent_id, t.blob_hash,
                    t.run_id, t.execution_id, t.produced_by, t.created_at, t.note,
                    b.data
             FROM turns t
             JOIN blobs b ON t.blob_hash = b.blob_hash
             WHERE t.turn_id = ?1",
            params![turn_id_str],
            |row| {
                let blob_data: Vec<u8> = row.get(9)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    blob_data,
                ))
            },
        ).map_err(|_| StorageError::NotFound(turn_id))?;

        let payload: T = rmp_serde::from_slice(&row.9)
            .map_err(|e: rmp_serde::decode::Error| StorageError::Serialization(e.to_string()))?;

        let parent_id = row.2.as_deref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let run_id = Uuid::parse_str(&row.4)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let created_at: DateTime<Utc> = row.7.parse()
            .map_err(|e: chrono::ParseError| StorageError::Serialization(e.to_string()))?;

        Ok(Turn {
            turn_id: Uuid::parse_str(&row.0)
                .map_err(|e| StorageError::Serialization(e.to_string()))?,
            type_id: row.1.clone(),
            parent_id,
            blob_hash: row.3.clone(),
            payload,
            metadata: TurnMetadata {
                created_at,
                produced_by: row.6.clone(),
                run_id,
                execution_id: row.5.clone(),
                note: row.8.clone(),
            },
        })
    }

    fn get_turns_by_type<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Vec<Turn<T>>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.turn_id
             FROM turns t
             WHERE t.run_id = ?1 AND t.type_id = ?2
             ORDER BY t.created_at ASC",
        )?;

        let turn_ids: Vec<Uuid> = stmt
            .query_map(params![run_id.to_string(), type_id], |row| {
                let id_str: String = row.get(0)?;
                Ok(id_str)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|s| Uuid::parse_str(&s).ok())
            .collect();

        let mut turns = Vec::new();
        for tid in turn_ids {
            turns.push(self.get_turn(tid)?);
        }
        Ok(turns)
    }

    fn get_latest_turn<T: ArtifactPayload + DeserializeOwned>(
        &self,
        run_id: Uuid,
        type_id: &str,
    ) -> Result<Option<Turn<T>>, StorageError> {
        let result = self.conn.query_row(
            "SELECT t.turn_id
             FROM turns t
             WHERE t.run_id = ?1 AND t.type_id = ?2
             ORDER BY t.created_at DESC
             LIMIT 1",
            params![run_id.to_string(), type_id],
            |row| {
                let id_str: String = row.get(0)?;
                Ok(id_str)
            },
        );

        match result {
            Ok(id_str) => {
                let tid = Uuid::parse_str(&id_str)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(self.get_turn(tid)?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Sqlite(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use planner_schemas::*;

    #[test]
    fn roundtrip_store_and_retrieve() {
        let store = SqliteTurnStore::in_memory().unwrap();
        let run_id = Uuid::new_v4();

        let intake = IntakeV1 {
            project_id: Uuid::new_v4(),
            project_name: "Test Project".into(),
            feature_slug: "test-feature".into(),
            intent_summary: "A simple test tool".into(),
            output_domain: OutputDomain::MicroTool {
                variant: MicroToolVariant::ReactWidget,
            },
            environment: EnvironmentInfo {
                language: "TypeScript".into(),
                framework: "React".into(),
                package_manager: Some("npm".into()),
                existing_dependencies: vec![],
                build_tool: Some("vite".into()),
            },
            sacred_anchors: vec![],
            satisfaction_criteria_seeds: vec![],
            out_of_scope: vec![],
            conversation_log: vec![],
        };

        let turn = Turn::new(intake, None, run_id, "test", "test-0");
        let turn_id = turn.turn_id;

        store.store_turn(&turn).unwrap();

        let retrieved: Turn<IntakeV1> = store.get_turn(turn_id).unwrap();
        assert_eq!(retrieved.turn_id, turn_id);
        assert_eq!(retrieved.payload.project_name, "Test Project");
        assert!(retrieved.verify_integrity());
    }

    #[test]
    fn get_latest_turn_works() {
        let store = SqliteTurnStore::in_memory().unwrap();
        let run_id = Uuid::new_v4();

        // Store two turns of the same type
        for i in 0..2 {
            let intake = IntakeV1 {
                project_id: Uuid::new_v4(),
                project_name: format!("Project {}", i),
                feature_slug: "test".into(),
                intent_summary: "test".into(),
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
            };

            let turn = Turn::new(intake, None, run_id, "test", format!("test-{}", i));
            store.store_turn(&turn).unwrap();
        }

        let latest: Option<Turn<IntakeV1>> = store
            .get_latest_turn(run_id, IntakeV1::TYPE_ID)
            .unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().payload.project_name, "Project 1");
    }
}
