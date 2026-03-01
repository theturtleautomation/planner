//! # Session Store — In-Memory Planning Session Management
//!
//! Tracks active Socratic planning sessions with their chat history
//! and pipeline state.

use std::collections::HashMap;
use parking_lot::RwLock;
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
    /// Auth0 sub claim of the owning user (or "dev|local" in dev mode).
    pub user_id: String,
    pub created_at: String,
    /// RFC3339 timestamp of the last get() or update() access.
    pub last_accessed: String,
    pub messages: Vec<SessionMessage>,
    pub stages: Vec<PipelineStageInfo>,
    pub pipeline_running: bool,
    pub project_description: Option<String>,
}

impl Session {
    pub fn new(user_id: &str) -> Self {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            created_at: now.to_rfc3339(),
            last_accessed: now.to_rfc3339(),
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
    pub(crate) sessions: RwLock<HashMap<Uuid, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session owned by `user_id` and return it.
    pub fn create(&self, user_id: &str) -> Session {
        let session = Session::new(user_id);
        let id = session.id;
        self.sessions.write().insert(id, session.clone());
        session
    }

    /// Get a session by ID. Updates `last_accessed`.
    pub fn get(&self, id: Uuid) -> Option<Session> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&id) {
            session.last_accessed = Utc::now().to_rfc3339();
            Some(session.clone())
        } else {
            None
        }
    }

    /// Update a session. Updates `last_accessed`.
    pub fn update<F>(&self, id: Uuid, f: F) -> Option<Session>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&id) {
            f(session);
            session.last_accessed = Utc::now().to_rfc3339();
            Some(session.clone())
        } else {
            None
        }
    }

    /// List all sessions owned by `user_id`.
    pub fn list_for_user(&self, user_id: &str) -> Vec<Session> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    /// List all session IDs.
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.sessions.read().keys().copied().collect()
    }

    /// Count active sessions.
    pub fn count(&self) -> usize {
        self.sessions.read().len()
    }

    /// Remove sessions that have not been accessed within `max_age_secs` seconds.
    pub fn cleanup_expired(&self, max_age_secs: u64) {
        let now = Utc::now();
        let mut sessions = self.sessions.write();
        let before = sessions.len();
        sessions.retain(|_id, session| {
            // Parse last_accessed; if unparseable, keep the session.
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&session.last_accessed) {
                let age = now.signed_duration_since(last).num_seconds();
                age < max_age_secs as i64
            } else {
                true
            }
        });
        let removed = before - sessions.len();
        if removed > 0 {
            tracing::info!("Session cleanup: removed {} expired session(s)", removed);
        }
    }

} // impl SessionStore

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_creation() {
        let session = Session::new("dev|local");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "system");
        assert_eq!(session.stages.len(), 12);
        assert!(!session.pipeline_running);
        assert_eq!(session.user_id, "dev|local");
    }

    #[test]
    fn session_add_message() {
        let mut session = Session::new("dev|local");
        let msg = session.add_message("user", "Build me a widget");

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Build me a widget");
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn session_store_crud() {
        let store = SessionStore::new();

        // Create
        let session = store.create("user1");
        let id = session.id;
        assert_eq!(store.count(), 1);
        assert_eq!(session.user_id, "user1");

        // Get
        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.user_id, "user1");

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
    fn session_store_list_for_user() {
        let store = SessionStore::new();

        store.create("user_a");
        store.create("user_a");
        store.create("user_b");

        let user_a_sessions = store.list_for_user("user_a");
        assert_eq!(user_a_sessions.len(), 2);

        let user_b_sessions = store.list_for_user("user_b");
        assert_eq!(user_b_sessions.len(), 1);

        let user_c_sessions = store.list_for_user("user_c");
        assert_eq!(user_c_sessions.len(), 0);
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
        let session = Session::new("auth0|abc123");
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.stages.len(), 12);
        assert_eq!(deserialized.user_id, "auth0|abc123");
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

    #[test]
    fn cleanup_expired_removes_old_sessions() {
        let store = SessionStore::new();

        // Create two sessions
        let s1 = store.create("user_cleanup_1");
        let s2 = store.create("user_cleanup_2");

        // Manually back-date s1's last_accessed to over 1 hour ago
        {
            let old_time = (chrono::Utc::now() - chrono::Duration::seconds(7200)).to_rfc3339();
            let mut sessions = store.sessions.write();
            sessions.get_mut(&s1.id).unwrap().last_accessed = old_time;
        }

        assert_eq!(store.count(), 2);

        // Cleanup sessions older than 3600 seconds (1 hour)
        store.cleanup_expired(3600);

        // s1 should be removed, s2 should remain
        // (Note: count() acquires a write lock via get(), so we use read count)
        assert_eq!(store.sessions.read().len(), 1);
        assert!(store.sessions.read().get(&s1.id).is_none());
        assert!(store.sessions.read().get(&s2.id).is_some());
    }
}
