//! # Session Store — In-Memory Planning Session Management
//!
//! Tracks active Socratic planning sessions with their chat history
//! and pipeline state.

use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

// ---------------------------------------------------------------------------
// Session Types
// ---------------------------------------------------------------------------

/// A single chat message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Uuid,
    pub role: String,      // "user", "planner", "system"
    pub content: String,
    pub timestamp: String,
}

/// Pipeline stage status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageInfo {
    pub name: String,
    pub status: String,    // "pending", "running", "complete", "failed"
}

/// A planning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub created_at: String,
    pub messages: Vec<SessionMessage>,
    pub stages: Vec<PipelineStageInfo>,
    pub pipeline_running: bool,
    pub project_description: Option<String>,
}

impl Session {
    pub fn new() -> Self {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4(),
            created_at: now.to_rfc3339(),
            messages: vec![SessionMessage {
                id: Uuid::new_v4(),
                role: "system".into(),
                content: "Welcome to Planner v2 — Socratic Planning Session. \
                         Describe what you want to build.".into(),
                timestamp: now.to_rfc3339(),
            }],
            stages: vec![
                PipelineStageInfo { name: "Intake".into(), status: "pending".into() },
                PipelineStageInfo { name: "Chunk".into(), status: "pending".into() },
                PipelineStageInfo { name: "Compile".into(), status: "pending".into() },
                PipelineStageInfo { name: "Lint".into(), status: "pending".into() },
                PipelineStageInfo { name: "AR Review".into(), status: "pending".into() },
                PipelineStageInfo { name: "Refine".into(), status: "pending".into() },
                PipelineStageInfo { name: "Scenarios".into(), status: "pending".into() },
                PipelineStageInfo { name: "Ralph".into(), status: "pending".into() },
                PipelineStageInfo { name: "Graph".into(), status: "pending".into() },
                PipelineStageInfo { name: "Factory".into(), status: "pending".into() },
                PipelineStageInfo { name: "Validate".into(), status: "pending".into() },
                PipelineStageInfo { name: "Git".into(), status: "pending".into() },
            ],
            pipeline_running: false,
            project_description: None,
        }
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, role: &str, content: &str) -> SessionMessage {
        let msg = SessionMessage {
            id: Uuid::new_v4(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.messages.push(msg.clone());
        msg
    }
}

// ---------------------------------------------------------------------------
// Session Store
// ---------------------------------------------------------------------------

/// Thread-safe in-memory store for planning sessions.
pub struct SessionStore {
    sessions: RwLock<HashMap<Uuid, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session and return it.
    pub fn create(&self) -> Session {
        let session = Session::new();
        let id = session.id;
        self.sessions.write().unwrap().insert(id, session.clone());
        session
    }

    /// Get a session by ID.
    pub fn get(&self, id: Uuid) -> Option<Session> {
        self.sessions.read().unwrap().get(&id).cloned()
    }

    /// Update a session.
    pub fn update<F>(&self, id: Uuid, f: F) -> Option<Session>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(&id) {
            f(session);
            Some(session.clone())
        } else {
            None
        }
    }

    /// List all session IDs.
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.sessions.read().unwrap().keys().copied().collect()
    }

    /// Count active sessions.
    pub fn count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_creation() {
        let session = Session::new();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "system");
        assert_eq!(session.stages.len(), 12);
        assert!(!session.pipeline_running);
    }

    #[test]
    fn session_add_message() {
        let mut session = Session::new();
        let msg = session.add_message("user", "Build me a widget");

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Build me a widget");
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn session_store_crud() {
        let store = SessionStore::new();

        // Create
        let session = store.create();
        let id = session.id;
        assert_eq!(store.count(), 1);

        // Get
        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved.id, id);

        // Update
        let updated = store.update(id, |s| {
            s.add_message("user", "Hello");
            s.pipeline_running = true;
        }).unwrap();
        assert_eq!(updated.messages.len(), 2);
        assert!(updated.pipeline_running);

        // List
        let ids = store.list_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&id));
    }

    #[test]
    fn session_store_get_missing() {
        let store = SessionStore::new();
        assert!(store.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn session_store_update_missing() {
        let store = SessionStore::new();
        let result = store.update(Uuid::new_v4(), |_| {});
        assert!(result.is_none());
    }

    #[test]
    fn session_serialization() {
        let session = Session::new();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.stages.len(), 12);
    }

    #[test]
    fn pipeline_stage_info_serde() {
        let stage = PipelineStageInfo {
            name: "Intake".into(),
            status: "running".into(),
        };
        let json = serde_json::to_string(&stage).unwrap();
        assert!(json.contains("Intake"));
        assert!(json.contains("running"));
    }
}
