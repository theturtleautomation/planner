//! # WebSocket — Real-Time Session Updates
//!
//! Provides WebSocket endpoint for real-time pipeline progress updates.
//! The web frontend connects here to receive live stage transitions,
//! planner messages, and pipeline completion events.
//!
//! Protocol (JSON messages):
//! - Server → Client: { "type": "stage_update", "stage": "Compile", "status": "running" }
//! - Server → Client: { "type": "message", "role": "planner", "content": "..." }
//! - Server → Client: { "type": "pipeline_complete", "success": true }
//! - Client → Server: { "type": "user_message", "content": "..." }

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// WebSocket Message Types
// ---------------------------------------------------------------------------

/// Server-to-client WebSocket message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// A pipeline stage changed status.
    #[serde(rename = "stage_update")]
    StageUpdate {
        stage: String,
        status: String,
    },
    /// A new chat message from the system or planner.
    #[serde(rename = "message")]
    ChatMessage {
        id: String,
        role: String,
        content: String,
        timestamp: String,
    },
    /// Pipeline completed.
    #[serde(rename = "pipeline_complete")]
    PipelineComplete {
        success: bool,
        summary: String,
    },
    /// Error occurred.
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// Client-to-server WebSocket message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// User sends a chat message.
    #[serde(rename = "user_message")]
    UserMessage {
        content: String,
    },
    /// User requests pipeline start.
    #[serde(rename = "start_pipeline")]
    StartPipeline {
        description: String,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn server_message_stage_update_serde() {
        let msg = ServerMessage::StageUpdate {
            stage: "Compile".into(),
            status: "running".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"stage_update\""));
        assert!(json.contains("\"stage\":\"Compile\""));

        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::StageUpdate { stage, status } => {
                assert_eq!(stage, "Compile");
                assert_eq!(status, "running");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn server_message_chat_serde() {
        let msg = ServerMessage::ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "planner".into(),
            content: "Let me ask some questions".into(),
            timestamp: "2026-02-28T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"message\""));
        assert!(json.contains("planner"));
    }

    #[test]
    fn server_message_pipeline_complete_serde() {
        let msg = ServerMessage::PipelineComplete {
            success: true,
            summary: "All 12 stages passed".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("pipeline_complete"));
    }

    #[test]
    fn client_message_user_message_serde() {
        let json = r#"{"type":"user_message","content":"Build me a widget"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::UserMessage { content } => {
                assert_eq!(content, "Build me a widget");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_start_pipeline_serde() {
        let json = r#"{"type":"start_pipeline","description":"Task tracker"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::StartPipeline { description } => {
                assert_eq!(description, "Task tracker");
            }
            _ => panic!("wrong variant"),
        }
    }
}
