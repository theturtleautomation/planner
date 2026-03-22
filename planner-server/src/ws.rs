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

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use planner_schemas::{
    PromptEnvelope, PromptResponse as StructuredPromptResponse, SocraticCategorySnapshot,
    UiCapabilities as ClientUiCapabilities,
};

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
    StageUpdate { stage: String, status: String },
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
    PipelineComplete { success: bool, summary: String },
    /// Error occurred.
    #[serde(rename = "error")]
    Error { message: String },

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

    /// Structured prompt envelope for user response.
    #[serde(rename = "prompt")]
    Prompt { prompt: PromptEnvelope },

    /// Current Socratic category-navigation state.
    #[serde(rename = "category_state")]
    CategoryState { snapshot: SocraticCategorySnapshot },

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
    UserMessage { content: String },
    /// User requests pipeline start.
    #[serde(rename = "start_pipeline")]
    StartPipeline { description: String },

    // -----------------------------------------------------------------------
    // Socratic interview messages
    // -----------------------------------------------------------------------
    /// User submits structured answers for a Socratic prompt envelope.
    #[serde(rename = "prompt_response")]
    PromptResponse {
        #[serde(flatten)]
        response: StructuredPromptResponse,
    },

    /// Client-advertised UI capabilities for prompt batch sizing and layout.
    #[serde(rename = "ui_capabilities")]
    UiCapabilities {
        #[serde(flatten)]
        capabilities: ClientUiCapabilities,
    },

    /// Enter a category from the current snapshot revision.
    #[serde(rename = "enter_category")]
    EnterCategory {
        category_id: String,
        revision: String,
    },

    /// Return to the main category screen.
    #[serde(rename = "back_to_categories")]
    BackToCategories,

    /// User says "done, start building".
    #[serde(rename = "done")]
    Done,

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
                                        let _ = crate::api::spawn_pipeline_runtime(
                                            state.clone(),
                                            session_id,
                                            description.clone(),
                                        );
                                    }
                                }
                                // Socratic messages are handled by ws_socratic::handle_socratic_ws;
                                // ignore them in the pipeline-phase handler.
                                ClientMessage::PromptResponse { .. }
                                | ClientMessage::UiCapabilities { .. }
                                | ClientMessage::EnterCategory { .. }
                                | ClientMessage::BackToCategories
                                | ClientMessage::Done
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
    use crate::session::SessionStore;
    use crate::AppState;
    use std::sync::Arc;
    use uuid::Uuid;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            auth_config: None,
            event_store: None,
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: crate::import::default_import_acquirer(),
            import_analyzer: crate::import::default_import_analyzer(),
        })
    }

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
            ServerMessage::PlannerEvent {
                level,
                source,
                duration_ms,
                ..
            } => {
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
    fn server_message_prompt_serde() {
        let msg = ServerMessage::Prompt {
            prompt: PromptEnvelope {
                prompt_id: "prompt-1".into(),
                kind: planner_schemas::PromptKind::QuestionBatch,
                title: "Continue interview".into(),
                instructions: Some("Answer what you can.".into()),
                origin_category_id: None,
                category_path: Vec::new(),
                items: vec![planner_schemas::PromptItem {
                    item_id: "item-1".into(),
                    kind: planner_schemas::PromptItemKind::Discovery,
                    target_dimension: Some(planner_schemas::Dimension::Goal),
                    section_ref: None,
                    text: "What should this app optimize for first?".into(),
                    options: vec![planner_schemas::PromptOption {
                        option_id: "opt-1".into(),
                        label: "Speed".into(),
                        semantic_value: "speed".into(),
                        direct_effect: None,
                    }],
                    response_mode: planner_schemas::PromptResponseMode::SingleSelectWithCustomText,
                    required: true,
                    priority: 100,
                    dependency_item_ids: vec![],
                }],
                draft_snapshot: None,
                required_item_ids: vec!["item-1".into()],
                allow_partial_submit: true,
                ui_hints: planner_schemas::PromptUiHints {
                    preferred_layout: planner_schemas::PromptPreferredLayout::Cards,
                    show_draft_sidebar: false,
                },
                based_on_turn: 2,
                created_at: "2026-03-08T00:00:00Z".into(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"prompt\""));
        assert!(json.contains("\"prompt_id\":\"prompt-1\""));

        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            ServerMessage::Prompt { prompt } => {
                assert_eq!(prompt.prompt_id, "prompt-1");
                assert_eq!(prompt.items.len(), 1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_prompt_response_serde() {
        let json = r#"{
            "type":"prompt_response",
            "prompt_id":"prompt-1",
            "answers":[{"item_id":"item-1","selected_option_id":"opt-1","custom_text":"Primary path","skipped":false}],
            "submitted_at":"2026-03-08T00:00:00Z",
            "client_context":{"viewport_class":"desktop"}
        }"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::PromptResponse { response } => {
                assert_eq!(response.prompt_id, "prompt-1");
                assert_eq!(response.answers.len(), 1);
                assert_eq!(response.answers[0].item_id, "item-1");
                assert_eq!(
                    response.answers[0].selected_option_id.as_deref(),
                    Some("opt-1")
                );
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_ui_capabilities_serde() {
        let json = r#"{
            "type":"ui_capabilities",
            "viewport_class":"tablet",
            "max_visible_items":3,
            "supports_split_draft_view":false
        }"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::UiCapabilities { capabilities } => {
                assert_eq!(
                    capabilities.viewport_class,
                    planner_schemas::ViewportClass::Tablet
                );
                assert_eq!(capabilities.max_visible_items, 3);
                assert!(!capabilities.supports_split_draft_view);
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
            ServerMessage::ContradictionDetected {
                dimension_a,
                explanation,
                ..
            } => {
                assert_eq!(dimension_a, "Deployment");
                assert!(explanation.contains("conflicts"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn client_message_dimension_edit_serde() {
        let json = r#"{"type":"dimension_edit","dimension":"Database","new_value":"SQLite"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::DimensionEdit {
                dimension,
                new_value,
            } => {
                assert_eq!(dimension, "Database");
                assert_eq!(new_value, "SQLite");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn ws_pipeline_start_registers_pipeline_runtime() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;
        let description = "Run websocket pipeline".to_string();

        let was_running = state
            .sessions
            .get(session_id)
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
            let _ = crate::api::spawn_pipeline_runtime(state.clone(), session_id, description);
        }

        assert!(state.pipeline_runtimes.get(session_id).is_some());
        crate::api::stop_active_session_work(&state, session_id);
    }
}
