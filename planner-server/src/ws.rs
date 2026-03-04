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

    /// A structured observability event.
    #[serde(rename = "planner_event")]
    PlannerEvent {
        id: String,
        timestamp: String,
        level: String,
        source: String,
        step: Option<String>,
        message: String,
        duration_ms: Option<u64>,
        metadata: serde_json::Value,
    },

    // -----------------------------------------------------------------------
    // Socratic interview messages
    // -----------------------------------------------------------------------

    /// Domain classification complete.
    #[serde(rename = "classified")]
    Classified {
        project_type: String,
        complexity: String,
        question_budget: u8,
    },

    /// Belief state update.
    #[serde(rename = "belief_state_update")]
    BeliefStateUpdate {
        filled: serde_json::Value,
        uncertain: serde_json::Value,
        missing: Vec<String>,
        out_of_scope: Vec<String>,
        convergence_pct: f32,
    },

    /// Question for the user.
    #[serde(rename = "question")]
    Question {
        text: String,
        target_dimension: String,
        quick_options: Vec<serde_json::Value>,
        allow_skip: bool,
    },

    /// Speculative draft for review.
    #[serde(rename = "speculative_draft")]
    SpeculativeDraft {
        sections: Vec<serde_json::Value>,
        assumptions: Vec<serde_json::Value>,
        not_discussed: Vec<String>,
    },

    /// Interview converged — ready to build.
    #[serde(rename = "converged")]
    Converged {
        reason: String,
        convergence_pct: f32,
    },

    /// A contradiction was detected between two dimensions.
    #[serde(rename = "contradiction_detected")]
    ContradictionDetected {
        dimension_a: String,
        value_a: String,
        dimension_b: String,
        value_b: String,
        explanation: String,
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

    // -----------------------------------------------------------------------
    // Socratic interview messages
    // -----------------------------------------------------------------------

    /// User responds during Socratic interview.
    #[serde(rename = "socratic_response")]
    SocraticResponse {
        content: String,
    },

    /// User skips current question.
    #[serde(rename = "skip_question")]
    SkipQuestion,

    /// User says "done, start building".
    #[serde(rename = "done")]
    Done,

    /// User reacts to a speculative draft section (correct, fix, or confirm/fix assumptions).
    #[serde(rename = "draft_reaction")]
    DraftReaction {
        /// Which section index or "assumptions" this applies to.
        target: String,
        /// "correct" | "fix" | "confirm_all" | "fix_these"
        action: String,
        /// Optional free-text correction when action is "fix" or "fix_these".
        correction: Option<String>,
    },

    /// User edits a dimension value directly from the belief state panel.
    #[serde(rename = "dimension_edit")]
    DimensionEdit {
        dimension: String,
        new_value: String,
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
                                // Socratic messages are handled by ws_socratic::handle_socratic_ws;
                                // ignore them in the pipeline-phase handler.
                                ClientMessage::SocraticResponse { .. }
                                | ClientMessage::SkipQuestion
                                | ClientMessage::Done
                                | ClientMessage::DraftReaction { .. }
                                | ClientMessage::DimensionEdit { .. } => {}
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
    fn server_message_planner_event_serde() {
        let msg = ServerMessage::PlannerEvent {
            id: "test-id".into(),
            timestamp: "2026-03-04T00:00:00Z".into(),
            level: "info".into(),
            source: "llm_router".into(),
            step: Some("llm.call.complete".into()),
            message: "LLM call completed".into(),
            duration_ms: Some(1234),
            metadata: serde_json::json!({"model": "claude-opus-4-6"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"planner_event\""));
        assert!(json.contains("llm.call.complete"));
        assert!(json.contains("1234"));

        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::PlannerEvent { level, source, duration_ms, .. } => {
                assert_eq!(level, "info");
                assert_eq!(source, "llm_router");
                assert_eq!(duration_ms, Some(1234));
            }
            _ => panic!("wrong variant"),
        }
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

    #[test]
    fn server_message_contradiction_detected_serde() {
        let msg = ServerMessage::ContradictionDetected {
            dimension_a: "Deployment".into(),
            value_a: "serverless".into(),
            dimension_b: "Database".into(),
            value_b: "bare metal PostgreSQL".into(),
            explanation: "Serverless hosting conflicts with bare metal DB requirement".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"contradiction_detected\""));
        assert!(json.contains("Deployment"));
        assert!(json.contains("bare metal"));

        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::ContradictionDetected { dimension_a, explanation, .. } => {
                assert_eq!(dimension_a, "Deployment");
                assert!(explanation.contains("conflicts"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_draft_reaction_serde() {
        let json = r#"{"type":"draft_reaction","target":"0","action":"fix","correction":"Should use REST not GraphQL"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::DraftReaction { target, action, correction } => {
                assert_eq!(target, "0");
                assert_eq!(action, "fix");
                assert_eq!(correction.unwrap(), "Should use REST not GraphQL");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_draft_reaction_no_correction() {
        let json = r#"{"type":"draft_reaction","target":"assumptions","action":"confirm_all"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::DraftReaction { target, action, correction } => {
                assert_eq!(target, "assumptions");
                assert_eq!(action, "confirm_all");
                assert!(correction.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_dimension_edit_serde() {
        let json = r#"{"type":"dimension_edit","dimension":"Database","new_value":"SQLite"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::DimensionEdit { dimension, new_value } => {
                assert_eq!(dimension, "Database");
                assert_eq!(new_value, "SQLite");
            }
            _ => panic!("wrong variant"),
        }
    }
}
