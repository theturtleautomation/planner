//! # WebSocket Socratic Handler
//!
//! Implements `SocraticIO` for WebSocket connections and provides the
//! `handle_socratic_ws()` entry point that drives a Socratic interview
//! session over a WebSocket connection.
//!
//! ## Message flow
//!
//! ```text
//! Client                           Server
//!   │  SocraticResponse / SkipQuestion / Done  │
//!   │ ────────────────────────────────────────► │  input_tx
//!   │                                           │      │
//!   │                                           │  WsSocraticIO::receive_input()
//!   │                                           │      │
//!   │                                           │  run_interview() (socratic_engine)
//!   │                                           │      │
//!   │  classified / question / belief_state_update / … │
//!   │ ◄──────────────────────────────────────── │  event_tx
//! ```
//!
//! After `Converged` is received the handler transitions to pipeline mode,
//! delegating to `api::run_pipeline_for_session`.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use uuid::Uuid;

use planner_schemas::{
    DomainClassification, QuestionOutput, RequirementsBeliefState, ConvergenceResult,
    SpeculativeDraft, SocraticEvent,
};

// Import SocraticIO trait so we can call .send_message() on Arc<WsSocraticIO>
// inside the spawned engine task's error handler.
use planner_core::pipeline::steps::socratic::SocraticIO;

use crate::AppState;
use crate::ws::{ClientMessage, ServerMessage};

// ---------------------------------------------------------------------------
// WsSocraticIO — SocraticIO impl for WebSocket
// ---------------------------------------------------------------------------

/// WebSocket-based `SocraticIO` implementation.
///
/// Forwards engine events to the WebSocket client via `event_tx` and receives
/// user input from the client via `input_rx`.
pub struct WsSocraticIO {
    /// Send events to the WebSocket client.
    event_tx: mpsc::UnboundedSender<ServerMessage>,
    /// Receive user input forwarded from the WebSocket client.
    input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
}

impl WsSocraticIO {
    pub fn new(
        event_tx: mpsc::UnboundedSender<ServerMessage>,
        input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    ) -> Self {
        Self { event_tx, input_rx }
    }

    /// Helper: send a `ServerMessage`, logging errors silently.
    fn send(&self, msg: ServerMessage) {
        let _ = self.event_tx.send(msg);
    }
}

#[async_trait::async_trait]
impl planner_core::pipeline::steps::socratic::SocraticIO for WsSocraticIO {
    async fn send_message(&self, content: &str) {
        self.send(ServerMessage::ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "planner".into(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    async fn send_question(&self, output: &QuestionOutput) {
        let quick_options = output
            .quick_options
            .iter()
            .filter_map(|opt| serde_json::to_value(opt).ok())
            .collect();

        self.send(ServerMessage::Question {
            text: output.question.clone(),
            target_dimension: output.target_dimension.label(),
            quick_options,
            allow_skip: output.allow_skip,
        });
    }

    async fn send_belief_state(&self, state: &RequirementsBeliefState) {
        // Serialize the HashMap keys as strings for JSON compatibility.
        let filled: serde_json::Map<String, serde_json::Value> = state
            .filled
            .iter()
            .filter_map(|(dim, slot)| {
                serde_json::to_value(slot)
                    .ok()
                    .map(|v| (dim.label(), v))
            })
            .collect();

        let uncertain: serde_json::Map<String, serde_json::Value> = state
            .uncertain
            .iter()
            .filter_map(|(dim, (slot, conf))| {
                let entry = serde_json::json!({ "value": slot, "confidence": conf });
                Some((dim.label(), entry))
            })
            .collect();

        self.send(ServerMessage::BeliefStateUpdate {
            filled: serde_json::Value::Object(filled),
            uncertain: serde_json::Value::Object(uncertain),
            missing: state.missing.iter().map(|d| d.label()).collect(),
            out_of_scope: state.out_of_scope.iter().map(|d| d.label()).collect(),
            convergence_pct: state.convergence_pct(),
        });
    }

    async fn send_draft(&self, draft: &SpeculativeDraft) {
        let sections = draft
            .sections
            .iter()
            .filter_map(|s| serde_json::to_value(s).ok())
            .collect();

        let assumptions = draft
            .assumptions
            .iter()
            .filter_map(|a| serde_json::to_value(a).ok())
            .collect();

        let not_discussed = draft
            .not_discussed
            .iter()
            .map(|d| d.label())
            .collect();

        self.send(ServerMessage::SpeculativeDraft {
            sections,
            assumptions,
            not_discussed,
        });
    }

    async fn send_convergence(&self, result: &ConvergenceResult) {
        let reason = serde_json::to_string(&result.reason)
            .unwrap_or_else(|_| "converged".into());

        self.send(ServerMessage::Converged {
            reason,
            convergence_pct: result.convergence_pct,
        });
    }

    async fn send_classification(&self, classification: &DomainClassification) {
        self.send(ServerMessage::Classified {
            project_type: classification.project_type.to_string(),
            complexity: match classification.complexity {
                planner_schemas::ComplexityTier::Light => "light".into(),
                planner_schemas::ComplexityTier::Standard => "standard".into(),
                planner_schemas::ComplexityTier::Deep => "deep".into(),
            },
            question_budget: classification.question_budget,
        });
    }

    async fn receive_input(&self) -> Option<String> {
        self.input_rx.lock().await.recv().await
    }

    async fn send_event(&self, event: &SocraticEvent) {
        // If this is a ContradictionDetected event, send it as a typed
        // ServerMessage so the frontend can render it in the belief state panel
        // without parsing a generic JSON blob.
        if let SocraticEvent::ContradictionDetected { contradiction } = event {
            self.send(ServerMessage::ContradictionDetected {
                dimension_a: contradiction.dimension_a.label(),
                value_a: contradiction.value_a.clone(),
                dimension_b: contradiction.dimension_b.label(),
                value_b: contradiction.value_b.clone(),
                explanation: contradiction.explanation.clone(),
            });
        }

        // Also forward the raw event as a ChatMessage so the chat log shows it.
        match serde_json::to_string(event) {
            Ok(json) => {
                let _ = self.event_tx.send(ServerMessage::ChatMessage {
                    id: Uuid::new_v4().to_string(),
                    role: "event".into(),
                    content: json,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
            Err(e) => {
                tracing::warn!("Failed to serialize SocraticEvent: {}", e);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// handle_socratic_ws — main WebSocket handler
// ---------------------------------------------------------------------------

/// Drive a Socratic interview session over a WebSocket connection.
///
/// ## Protocol
///
/// 1. The first `SocraticResponse` (or `StartPipeline`) message carries the
///    initial project description and starts `run_interview`.
/// 2. Subsequent `SocraticResponse` / `SkipQuestion` / `Done` messages are
///    forwarded to the engine via `input_tx`.
/// 3. After convergence the session transitions to pipeline mode and the
///    existing `api::run_pipeline_for_session` task is spawned.
///
/// The caller is responsible for verifying session ownership before invoking
/// this function.
pub async fn handle_socratic_ws(
    mut socket: WebSocket,
    state: Arc<AppState>,
    session_id: Uuid,
) {
    // Verify the session exists.
    if state.sessions.get(session_id).is_none() {
        let err = ServerMessage::Error {
            message: format!("Session {} not found", session_id),
        };
        if let Ok(json) = serde_json::to_string(&err) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        return;
    }

    // Channel: engine → WebSocket (outbound events)
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Channel: WebSocket → engine (inbound user text)
    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    let input_rx = Arc::new(Mutex::new(input_rx));

    // Wait for the first SocraticResponse / StartPipeline to get the initial
    // project description before launching the engine.
    let initial_description = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::SocraticResponse { content }) => break content,
                    Ok(ClientMessage::StartPipeline { description }) => break description,
                    Ok(ClientMessage::Done) => {
                        // User quit immediately — nothing to do.
                        return;
                    }
                    Ok(_) => continue, // ignore other messages while waiting
                    Err(e) => {
                        tracing::warn!(
                            "Session {}: failed to parse initial client message: {}",
                            session_id,
                            e
                        );
                        continue;
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => return,
            _ => continue,
        }
    };

    // Mark session as interviewing.
    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.add_message("user", &initial_description);
    });

    // Build the IO bridge and spawn the engine.
    // Note: we pass a *clone* of event_tx to the IO bridge and drop the
    // original immediately so the channel closes when the engine task
    // finishes (its Arc<WsSocraticIO> drops), allowing event_rx.recv()
    // to return None and unblock the I/O loop.
    let io = Arc::new(WsSocraticIO::new(event_tx.clone(), input_rx));
    drop(event_tx); // keep channel alive only through io's clone

    let router = planner_core::llm::providers::LlmRouter::from_env();

    // Pre-flight check: warn if no LLM providers are available.
    let available = router.available_providers();
    if available.is_empty() {
        tracing::error!(
            "Session {}: No LLM CLI providers found. Install and authenticate at least one of: claude, gemini, codex",
            session_id
        );
        let err_msg = "No LLM providers available. The planner service user needs at least one of the following CLI tools installed and authenticated: `claude` (Anthropic), `gemini` (Google), or `codex` (OpenAI). Check that these are on the PATH for the user running the planner service.";

        // Send error directly on the socket (the I/O loop hasn't started yet).
        let error_msg = ServerMessage::Error {
            message: err_msg.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&error_msg) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        state.sessions.update(session_id, |s| {
            s.intake_phase = "error".into();
            s.add_message("system", err_msg);
        });
        return;
    }
    tracing::info!(
        "Session {}: LLM providers available: {:?}",
        session_id, available
    );

    let state_for_engine = state.clone();

    // Spawn the interview engine as a background task.
    let engine_handle = tokio::spawn(async move {
        let result = planner_core::pipeline::steps::socratic::run_interview::<WsSocraticIO, planner_core::cxdb::CxdbEngine>(
            &router,
            &*io,
            None::<&planner_core::cxdb::CxdbEngine>,
            &initial_description,
        )
        .await;

        match result {
            Ok(session) => {
                // Persist belief state to the server session.
                state_for_engine.sessions.update(session_id, |s| {
                    s.belief_state = Some(session.belief_state.clone());
                    s.classification = session.belief_state.classification.clone();
                    s.intake_phase = "pipeline_running".into();
                });

                // Build the description from the completed belief state.
                let intake = planner_core::pipeline::steps::socratic::session_to_intake(
                    &session,
                    Uuid::new_v4(),
                );
                Some(intake.intent_summary)
            }
            Err(e) => {
                let err_msg = format!("Socratic interview failed: {}", e);
                tracing::warn!("Session {}: {}", session_id, err_msg);

                // Send the error to the client so the UI doesn't hang.
                io.send(ServerMessage::Error {
                    message: err_msg.clone(),
                });
                io.send_message(&format!("Error: {}", err_msg)).await;

                // Mark session as errored.
                state_for_engine.sessions.update(session_id, |s| {
                    s.intake_phase = "error".into();
                    s.add_message("system", &err_msg);
                });

                None
            }
        }
    });

    // Drive the WebSocket I/O loop while the engine runs.
    let mut converged = false;

    loop {
        tokio::select! {
            // Forward engine events to the client.
            msg = event_rx.recv() => {
                match msg {
                    Some(server_msg) => {
                        // Note convergence so we can start the pipeline afterwards.
                        if matches!(&server_msg, ServerMessage::Converged { .. }) {
                            converged = true;
                        }
                        if let Ok(json) = serde_json::to_string(&server_msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                return; // client disconnected
                            }
                        }
                    }
                    None => {
                        // event_tx dropped — engine finished.
                        break;
                    }
                }
            }

            // Forward client messages to the engine.
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::SocraticResponse { content }) => {
                                // Record in session and forward to engine.
                                state.sessions.update(session_id, |s| {
                                    s.add_message("user", &content);
                                });
                                let _ = input_tx.send(content);
                            }
                            Ok(ClientMessage::SkipQuestion) => {
                                let _ = input_tx.send("skip".into());
                            }
                            Ok(ClientMessage::Done) => {
                                // Signal the engine that the user wants to stop.
                                let _ = input_tx.send("done".into());
                            }
                            Ok(ClientMessage::DraftReaction { target, action, correction }) => {
                                // Forward draft reactions to the engine as structured input.
                                // The engine's receive_input() will parse these prefixed commands.
                                let corr_str = correction.as_deref().unwrap_or("(no correction)");
                                let msg = if correction.is_some() {
                                    format!("[draft_reaction] target={} action={} correction={}", target, action, corr_str)
                                } else {
                                    format!("[draft_reaction] target={} action={}", target, action)
                                };
                                state.sessions.update(session_id, |s| {
                                    s.add_message("user", &format!("Draft feedback: {} section {} — {}",
                                        action, target, corr_str));
                                });
                                let _ = input_tx.send(msg);
                            }
                            Ok(ClientMessage::DimensionEdit { dimension, new_value }) => {
                                // Forward dimension edits to the engine.
                                let msg = format!("[dimension_edit] {}={}", dimension, new_value);
                                state.sessions.update(session_id, |s| {
                                    s.add_message("user", &format!("Edited dimension '{}' → '{}'", dimension, new_value));
                                });
                                let _ = input_tx.send(msg);
                            }
                            Ok(_) => {} // ignore other message types
                            Err(e) => {
                                tracing::warn!(
                                    "Session {}: failed to parse client message: {}",
                                    session_id, e
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        // Client disconnected — drop input_tx to unblock engine.
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    // Wait for the engine task to finish, then start the pipeline if converged.
    let description = engine_handle.await.ok().flatten();

    if converged {
        let description = description.unwrap_or_else(|| {
            // Fall back to raw belief state summary if engine produced nothing.
            state.sessions.get(session_id)
                .and_then(|s| s.project_description.clone())
                .unwrap_or_else(|| "Project requirements gathered via Socratic interview".into())
        });

        // Mark pipeline as running.
        let was_running = state.sessions.get(session_id)
            .map(|s| s.pipeline_running)
            .unwrap_or(false);

        if !was_running {
            state.sessions.update(session_id, |s| {
                s.pipeline_running = true;
                s.project_description = Some(description.clone());
                s.intake_phase = "pipeline_running".into();
                if let Some(stage) = s.stages.first_mut() {
                    stage.status = "running".into();
                }
            });

            let state_clone = state.clone();
            let desc = description.clone();
            tokio::spawn(async move {
                crate::api::run_pipeline_for_session(state_clone, session_id, desc).await;
            });
        }

        // Continue in pipeline-poll mode (same behaviour as handle_ws).
        let mut last_msg_count = state.sessions.get(session_id)
            .map(|s| s.messages.len())
            .unwrap_or(0);
        let mut last_sent_stages: Vec<(String, String)> = Vec::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let session = match state.sessions.get(session_id) {
                        Some(s) => s,
                        None => return,
                    };

                    // Forward new chat messages.
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
                                return;
                            }
                        }
                    }
                    last_msg_count = current_count;

                    // Forward changed stage statuses.
                    let current_stages: Vec<(String, String)> = session.stages
                        .iter()
                        .map(|s| (s.name.clone(), s.status.clone()))
                        .collect();

                    for stage in &session.stages {
                        let last_status = last_sent_stages
                            .iter()
                            .find(|(name, _)| name == &stage.name)
                            .map(|(_, status)| status.as_str());

                        if last_status != Some(stage.status.as_str()) {
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

                    // Close when pipeline finishes.
                    if !session.pipeline_running && session.project_description.is_some() {
                        let success = session.stages.iter().all(|s| s.status == "complete");
                        let server_msg = ServerMessage::PipelineComplete {
                            success,
                            summary: "Pipeline finished".into(),
                        };
                        if let Ok(json) = serde_json::to_string(&server_msg) {
                            let _ = socket.send(Message::Text(json.into())).await;
                        }

                        // Update intake_phase.
                        state.sessions.update(session_id, |s| {
                            s.intake_phase = "complete".into();
                        });

                        return;
                    }
                }

                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Close(_))) | None => return,
                        _ => {}
                    }
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

    #[tokio::test]
    async fn ws_socratic_io_send_classification() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(event_tx, Arc::new(Mutex::new(input_rx)));

        use planner_schemas::{DomainClassification, ProjectType, ComplexityTier, Dimension};

        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            question_budget: 12,
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        use planner_core::pipeline::steps::socratic::SocraticIO;
        io.send_classification(&classification).await;

        let msg = event_rx.try_recv().unwrap();
        match msg {
            ServerMessage::Classified { project_type, complexity, question_budget } => {
                assert_eq!(project_type, "Web App");
                assert_eq!(complexity, "standard");
                assert_eq!(question_budget, 12);
            }
            _ => panic!("expected Classified, got {:?}", msg),
        }
    }

    #[tokio::test]
    async fn ws_socratic_io_send_message() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(event_tx, Arc::new(Mutex::new(input_rx)));

        use planner_core::pipeline::steps::socratic::SocraticIO;
        io.send_message("Hello from the engine").await;

        let msg = event_rx.try_recv().unwrap();
        match msg {
            ServerMessage::ChatMessage { role, content, .. } => {
                assert_eq!(role, "planner");
                assert_eq!(content, "Hello from the engine");
            }
            _ => panic!("expected ChatMessage, got {:?}", msg),
        }
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_input() {
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(event_tx, Arc::new(Mutex::new(input_rx)));

        input_tx.send("hello world".into()).unwrap();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let received = io.receive_input().await;
        assert_eq!(received, Some("hello world".into()));
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_input_returns_none_when_closed() {
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(event_tx, Arc::new(Mutex::new(input_rx)));

        // Drop the sender — channel is closed.
        drop(input_tx);

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let received = io.receive_input().await;
        assert!(received.is_none());
    }

    #[tokio::test]
    async fn ws_socratic_io_send_event_contradiction() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(event_tx, Arc::new(Mutex::new(input_rx)));

        use planner_schemas::{Contradiction, Dimension};
        let contradiction = Contradiction {
            dimension_a: Dimension::Database,
            value_a: "PostgreSQL".into(),
            dimension_b: Dimension::Deployment,
            value_b: "serverless".into(),
            explanation: "PostgreSQL requires a persistent server".into(),
            resolved: false,
        };
        let event = SocraticEvent::ContradictionDetected { contradiction };

        use planner_core::pipeline::steps::socratic::SocraticIO;
        io.send_event(&event).await;

        // First message should be the typed ContradictionDetected
        let msg1 = event_rx.try_recv().unwrap();
        match msg1 {
            ServerMessage::ContradictionDetected { dimension_a, dimension_b, explanation, .. } => {
                assert_eq!(dimension_a, "Database");
                assert_eq!(dimension_b, "Deployment");
                assert!(explanation.contains("persistent server"));
            }
            other => panic!("expected ContradictionDetected, got {:?}", other),
        }

        // Second message should be the generic ChatMessage with role "event"
        let msg2 = event_rx.try_recv().unwrap();
        match msg2 {
            ServerMessage::ChatMessage { role, .. } => {
                assert_eq!(role, "event");
            }
            other => panic!("expected ChatMessage with event role, got {:?}", other),
        }
    }
}
