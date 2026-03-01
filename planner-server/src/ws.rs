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

use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

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
// WebSocket connection handler
// ---------------------------------------------------------------------------

/// Drive a live WebSocket connection for `session_id`.
///
/// - Every 500 ms: sends any new chat messages and current stage statuses.
/// - Listens for incoming client messages (user text / pipeline start).
/// - Closes automatically once the pipeline finishes (or immediately if the
///   session does not exist).
pub async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>, session_id: Uuid) {
    // Verify the session exists before entering the loop
    if state.sessions.get(session_id).is_none() {
        let err = ServerMessage::Error {
            message: format!("Session {} not found", session_id),
        };
        if let Ok(json) = serde_json::to_string(&err) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        return;
    }

    let mut last_msg_count = 0usize;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    // Track the last-sent stage statuses to avoid sending duplicate stage_update messages.
    let mut last_sent_stages: Vec<(String, String)> = Vec::new();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let session = match state.sessions.get(session_id) {
                    Some(s) => s,
                    None => {
                        let err = ServerMessage::Error {
                            message: format!("Session {} not found", session_id),
                        };
                        if let Ok(json) = serde_json::to_string(&err) {
                            let _ = socket.send(Message::Text(json.into())).await;
                        }
                        return;
                    }
                };

                // Forward any new chat messages
                let current_count = session.messages.len();
                for msg in session.messages.iter().skip(last_msg_count) {
                    let server_msg = ServerMessage::ChatMessage {
                        id: msg.id.to_string(),
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        timestamp: msg.timestamp.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            return; // client disconnected
                        }
                    }
                }
                last_msg_count = current_count;

                // Send stage statuses — only when changed since the last send
                let current_stages: Vec<(String, String)> = session
                    .stages
                    .iter()
                    .map(|s| (s.name.clone(), s.status.clone()))
                    .collect();

                for stage in &session.stages {
                    let last_status = last_sent_stages
                        .iter()
                        .find(|(name, _)| name == &stage.name)
                        .map(|(_, status)| status.as_str());
                    let has_changed = last_status != Some(stage.status.as_str());

                    if has_changed {
                        let server_msg = ServerMessage::StageUpdate {
                            stage: stage.name.clone(),
                            status: stage.status.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&server_msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                }
                last_sent_stages = current_stages;

                // If pipeline finished, send completion event and close
                if !session.pipeline_running && session.project_description.is_some() {
                    let success = session.stages.iter().all(|s| s.status == "complete");
                    let server_msg = ServerMessage::PipelineComplete {
                        success,
                        summary: "Pipeline finished".into(),
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
                    return; // close the WebSocket after completion
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::UserMessage { content } => {
                                    state.sessions.update(session_id, |s| {
                                        s.add_message("user", &content);
                                    });
                                }
                                ClientMessage::StartPipeline { description } => {
                                    // Set pipeline state and spawn the pipeline task
                                    let was_running = state.sessions.get(session_id)
                                        .map(|s| s.pipeline_running)
                                        .unwrap_or(false);

                                    state.sessions.update(session_id, |s| {
                                        s.add_message("user", &description);
                                        if !s.pipeline_running {
                                            s.pipeline_running = true;
                                            s.project_description = Some(description.clone());
                                            if let Some(stage) = s.stages.first_mut() {
                                                stage.status = "running".into();
                                            }
                                        }
                                    });

                                    if !was_running {
                                        let state_clone = state.clone();
                                        let desc = description.clone();
                                        tokio::spawn(async move {
                                            crate::api::run_pipeline_for_session(
                                                state_clone,
                                                session_id,
                                                desc,
                                            )
                                            .await;
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
        }
    }
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
