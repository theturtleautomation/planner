//! # planner-schemas — Turn<T>
//!
//! The foundational type for all CXDB state. Every artifact, decision,
//! and event in Planner v2 is stored as an immutable, content-addressed Turn.
//!
//! Phase 0 uses a SQLite sidecar that mirrors the CXDB Turn + Blob interface.
//! The real CXDB Rust server replaces SQLite in Phase 5 — only the storage
//! driver changes; this trait boundary stays identical.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ArtifactPayload — the trait every typed artifact implements
// ---------------------------------------------------------------------------

/// Marker trait for typed CXDB artifacts.
///
/// Every struct stored inside a `Turn` implements this. The `TYPE_ID` constant
/// is the CXDB type registry key (e.g. `"planner.intake.v1"`).
pub trait ArtifactPayload: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// CXDB type registry identifier, e.g. `"planner.nlspec.v1"`.
    const TYPE_ID: &'static str;
}

// ---------------------------------------------------------------------------
// Turn<T> — immutable, content-addressed wrapper
// ---------------------------------------------------------------------------

/// An immutable, content-addressed turn in the CXDB DAG.
///
/// `T` is the typed artifact payload (e.g. `IntakeV1`, `NLSpecV1`).
/// The `blob_hash` is the BLAKE3 digest of the msgpack-encoded payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
pub struct Turn<T: ArtifactPayload> {
    /// Unique turn identifier.
    pub turn_id: Uuid,

    /// CXDB type registry key — matches `T::TYPE_ID`.
    pub type_id: String,

    /// Parent turn in the DAG (None for root turns).
    pub parent_id: Option<Uuid>,

    /// BLAKE3 hash of the msgpack-encoded payload.
    pub blob_hash: String,

    /// The typed artifact payload.
    pub payload: T,

    /// Metadata about this turn's creation.
    pub metadata: TurnMetadata,
}

/// Metadata attached to every turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMetadata {
    /// When this turn was created.
    pub created_at: DateTime<Utc>,

    /// Which pipeline step or component produced this turn.
    pub produced_by: String,

    /// Run ID this turn belongs to.
    pub run_id: Uuid,

    /// Execution ID for idempotency (step_id + attempt number).
    pub execution_id: String,

    /// Optional human-readable note.
    pub note: Option<String>,

    /// Project this turn belongs to, used by CXDB for multi-project indexing.
    /// `None` for turns created outside a project context (e.g. unit tests).
    pub project_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Turn<T> construction helper
// ---------------------------------------------------------------------------

impl<T: ArtifactPayload> Turn<T> {
    /// Create a new turn, computing the BLAKE3 blob hash from the msgpack-encoded payload.
    pub fn new(
        payload: T,
        parent_id: Option<Uuid>,
        run_id: Uuid,
        produced_by: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        let type_id = T::TYPE_ID.to_string();
        let blob_bytes =
            rmp_serde::to_vec(&payload).expect("payload must be serializable to msgpack");
        let blob_hash = blake3::hash(&blob_bytes).to_hex().to_string();

        Turn {
            turn_id: Uuid::new_v4(),
            type_id,
            parent_id,
            blob_hash,
            payload,
            metadata: TurnMetadata {
                created_at: Utc::now(),
                produced_by: produced_by.into(),
                run_id,
                execution_id: execution_id.into(),
                note: None,
                project_id: None,
            },
        }
    }

    /// Create a new turn with an explicit project ID.
    ///
    /// Identical to [`Turn::new`] but sets `metadata.project_id` so that
    /// `CxdbEngine::store_turn` can index this turn under the correct project
    /// without any caller-side post-processing.
    pub fn new_with_project(
        payload: T,
        parent_id: Option<Uuid>,
        run_id: Uuid,
        produced_by: impl Into<String>,
        execution_id: impl Into<String>,
        project_id: Uuid,
    ) -> Self {
        let mut turn = Self::new(payload, parent_id, run_id, produced_by, execution_id);
        turn.metadata.project_id = Some(project_id);
        turn
    }

    /// Recompute and verify the blob hash. Returns `true` if the payload
    /// matches the stored hash (integrity check).
    pub fn verify_integrity(&self) -> bool {
        let blob_bytes =
            rmp_serde::to_vec(&self.payload).expect("payload must be serializable to msgpack");
        let computed = blake3::hash(&blob_bytes).to_hex().to_string();
        computed == self.blob_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal test artifact for unit tests.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestPayload {
        value: String,
    }

    impl ArtifactPayload for TestPayload {
        const TYPE_ID: &'static str = "test.payload.v1";
    }

    #[test]
    fn turn_roundtrip_and_integrity() {
        let payload = TestPayload {
            value: "hello".into(),
        };
        let turn = Turn::new(payload, None, Uuid::new_v4(), "test", "test-step-0");

        assert_eq!(turn.type_id, "test.payload.v1");
        assert!(turn.verify_integrity());
    }

    #[test]
    fn turn_new_with_project_sets_project_id() {
        let pid = Uuid::new_v4();
        let payload = TestPayload {
            value: "proj".into(),
        };
        let turn = Turn::new_with_project(payload, None, Uuid::new_v4(), "test", "step-0", pid);
        assert_eq!(turn.metadata.project_id, Some(pid));
        assert!(turn.verify_integrity());
    }

    #[test]
    fn turn_new_without_project_has_none() {
        let payload = TestPayload {
            value: "no-proj".into(),
        };
        let turn = Turn::new(payload, None, Uuid::new_v4(), "test", "step-0");
        assert_eq!(turn.metadata.project_id, None);
    }

    #[test]
    fn tampered_payload_fails_integrity() {
        let payload = TestPayload {
            value: "original".into(),
        };
        let mut turn = Turn::new(payload, None, Uuid::new_v4(), "test", "test-step-0");

        // Tamper with the payload after construction
        turn.payload.value = "tampered".into();
        assert!(!turn.verify_integrity());
    }
}
