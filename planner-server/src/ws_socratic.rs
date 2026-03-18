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
//!   │  prompt_response / ui_capabilities / done │
//!   │ ────────────────────────────────────────► │  input_tx
//!   │                                           │      │
//!   │                                           │  WsSocraticIO::receive_input()
//!   │                                           │      │
//!   │                                           │  run_interview() (socratic_engine)
//!   │                                           │      │
//!   │  classified / prompt / belief_state_update / … │
//!   │ ◄──────────────────────────────────────── │  event_tx
//! ```
//!
//! After `Converged` is received the handler transitions to pipeline mode,
//! delegating to `api::run_pipeline_for_session`.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::{broadcast, mpsc, Mutex};
use uuid::Uuid;

use planner_core::observability::EventSink;
use planner_core::pipeline::steps::socratic::convergence;
use planner_core::pipeline::steps::socratic::{
    run_interview_from_checkpoint, CheckpointResumeState, ResumePendingPrompt,
};

use planner_schemas::{
    ConvergenceResult, DomainClassification, PromptAnswer, PromptEnvelope, PromptResponse,
    RequirementsBeliefState, SocraticEvent, UiCapabilities, ViewportClass,
};

use crate::runtime::{AttachError, RuntimeAttachment, SessionRuntime, SocraticRuntimeInput};
use crate::ws::{ClientMessage, ServerMessage};
use crate::AppState;

// ---------------------------------------------------------------------------
// WsSocraticIO — SocraticIO impl for WebSocket
// ---------------------------------------------------------------------------

/// WebSocket-based `SocraticIO` implementation.
///
/// Forwards engine events to the WebSocket client via `event_tx` and receives
/// user input from the client via `input_rx`.
pub struct WsSocraticIO {
    /// Shared app state for reading live ui capabilities.
    state: Arc<AppState>,
    /// Send events to the WebSocket client.
    event_tx: mpsc::UnboundedSender<ServerMessage>,
    /// Forward raw Socratic events for checkpoint projection.
    checkpoint_tx: mpsc::UnboundedSender<SocraticEvent>,
    /// Receive user input forwarded from the WebSocket client.
    input_rx: Arc<Mutex<mpsc::UnboundedReceiver<SocraticRuntimeInput>>>,
    /// Optional observability event sink.
    event_sink: Option<Arc<dyn EventSink>>,
    /// Session ID for tagging emitted events.
    session_id: Uuid,
}

impl WsSocraticIO {
    pub fn new(
        state: Arc<AppState>,
        event_tx: mpsc::UnboundedSender<ServerMessage>,
        checkpoint_tx: mpsc::UnboundedSender<SocraticEvent>,
        input_rx: Arc<Mutex<mpsc::UnboundedReceiver<SocraticRuntimeInput>>>,
        event_sink: Option<Arc<dyn EventSink>>,
        session_id: Uuid,
    ) -> Self {
        Self {
            state,
            event_tx,
            checkpoint_tx,
            input_rx,
            event_sink,
            session_id,
        }
    }

    /// Helper: send a `ServerMessage`, logging errors silently.
    fn send(&self, msg: ServerMessage) {
        let _ = self.event_tx.send(msg);
    }

    fn emit_prompt_submission_event(&self, prompt: &PromptEnvelope, response: &PromptResponse) {
        let Some(ref sink) = self.event_sink else {
            return;
        };

        let answered_item_ids: HashSet<&str> = response
            .answers
            .iter()
            .map(|answer| answer.item_id.as_str())
            .collect();
        let answered_count = answered_item_ids.len();
        let total_count = prompt.items.len();
        let unanswered_count = total_count.saturating_sub(answered_count);
        let is_partial = unanswered_count > 0;
        let step = if is_partial {
            "socratic.prompt.partial_submitted"
        } else {
            "socratic.prompt.submitted"
        };

        sink.emit(
            planner_core::observability::PlannerEvent::info(
                planner_core::observability::EventSource::SocraticEngine,
                step,
                format!(
                    "Prompt '{}' submitted ({} answered, {} unanswered)",
                    prompt.prompt_id, answered_count, unanswered_count
                ),
            )
            .with_session(self.session_id)
            .with_metadata(serde_json::json!({
                "prompt_id": prompt.prompt_id.clone(),
                "kind": prompt.kind.clone(),
                "answered_count": answered_count,
                "unanswered_count": unanswered_count,
                "total_count": total_count,
                "submitted_at": response.submitted_at.clone(),
            })),
        );
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

    async fn send_prompt(&self, prompt: &PromptEnvelope) {
        self.send(ServerMessage::Prompt {
            prompt: prompt.clone(),
        });

        if let Some(ref sink) = self.event_sink {
            let previous_prompt = self
                .state
                .sessions
                .get(self.session_id)
                .and_then(|session| session.checkpoint)
                .and_then(|checkpoint| checkpoint.current_prompt);

            if let Some(previous_prompt) = previous_prompt {
                let previous_item_ids: HashSet<&str> = previous_prompt
                    .items
                    .iter()
                    .map(|item| item.item_id.as_str())
                    .collect();
                let next_item_ids: HashSet<&str> = prompt
                    .items
                    .iter()
                    .map(|item| item.item_id.as_str())
                    .collect();
                let invalidated_item_ids: Vec<String> = previous_prompt
                    .items
                    .iter()
                    .filter(|item| !next_item_ids.contains(item.item_id.as_str()))
                    .map(|item| item.item_id.clone())
                    .collect();
                let replacement_item_ids: Vec<String> = prompt
                    .items
                    .iter()
                    .filter(|item| !previous_item_ids.contains(item.item_id.as_str()))
                    .map(|item| item.item_id.clone())
                    .collect();
                if previous_prompt.prompt_id != prompt.prompt_id {
                    sink.emit(
                        planner_core::observability::PlannerEvent::info(
                            planner_core::observability::EventSource::SocraticEngine,
                            "socratic.prompt.invalidated",
                            format!("Prompt '{}' invalidated", previous_prompt.prompt_id),
                        )
                        .with_session(self.session_id)
                        .with_metadata(serde_json::json!({
                            "prompt_id": previous_prompt.prompt_id.clone(),
                            "next_prompt_id": prompt.prompt_id.clone(),
                            "invalidated_item_ids": invalidated_item_ids,
                            "replacement_item_ids": replacement_item_ids,
                        })),
                    );
                }

                let reissued_item_ids: Vec<String> = prompt
                    .items
                    .iter()
                    .filter(|item| previous_item_ids.contains(item.item_id.as_str()))
                    .map(|item| item.item_id.clone())
                    .collect();

                if !reissued_item_ids.is_empty() {
                    sink.emit(
                        planner_core::observability::PlannerEvent::info(
                            planner_core::observability::EventSource::SocraticEngine,
                            "socratic.prompt.reissued",
                            format!(
                                "Prompt '{}' reissued {} item(s)",
                                prompt.prompt_id,
                                reissued_item_ids.len()
                            ),
                        )
                        .with_session(self.session_id)
                        .with_metadata(serde_json::json!({
                            "prompt_id": prompt.prompt_id.clone(),
                            "reissued_item_ids": reissued_item_ids,
                            "kind": prompt.kind.clone(),
                        })),
                    );
                }
            }

            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.prompt.generated",
                    format!("Prompt generated with {} item(s)", prompt.items.len()),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "prompt_id": prompt.prompt_id.clone(),
                    "kind": prompt.kind.clone(),
                    "item_count": prompt.items.len(),
                })),
            );

            if prompt.draft_snapshot.is_some() {
                sink.emit(
                    planner_core::observability::PlannerEvent::info(
                        planner_core::observability::EventSource::SocraticEngine,
                        "socratic.draft.generated",
                        "Draft review prompt generated",
                    )
                    .with_session(self.session_id)
                    .with_metadata(serde_json::json!({
                        "prompt_id": prompt.prompt_id.clone(),
                        "item_count": prompt.items.len(),
                    })),
                );
            }
        }
    }

    async fn send_belief_state(&self, state: &RequirementsBeliefState) {
        // Serialize the HashMap keys as strings for JSON compatibility.
        let filled: serde_json::Map<String, serde_json::Value> = state
            .filled
            .iter()
            .filter_map(|(dim, slot)| serde_json::to_value(slot).ok().map(|v| (dim.label(), v)))
            .collect();

        let uncertain: serde_json::Map<String, serde_json::Value> = state
            .uncertain
            .iter()
            .filter_map(|(dim, (slot, conf))| {
                let entry = serde_json::json!({ "value": slot, "confidence": conf });
                Some((dim.label(), entry))
            })
            .collect();

        let convergence_pct = state.convergence_pct();

        self.send(ServerMessage::BeliefStateUpdate {
            filled: serde_json::Value::Object(filled),
            uncertain: serde_json::Value::Object(uncertain),
            missing: state.missing.iter().map(|d| d.label()).collect(),
            out_of_scope: state.out_of_scope.iter().map(|d| d.label()).collect(),
            convergence_pct,
        });

        if let Some(ref sink) = self.event_sink {
            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.verify.complete",
                    format!(
                        "Belief state updated: {:.0}% convergence ({} filled, {} uncertain, {} missing)",
                        convergence_pct * 100.0,
                        state.filled.len(),
                        state.uncertain.len(),
                        state.missing.len(),
                    ),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "convergence_pct": convergence_pct,
                    "filled_count": state.filled.len(),
                    "uncertain_count": state.uncertain.len(),
                    "missing_count": state.missing.len(),
                })),
            );

            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.response.adjudicated",
                    format!(
                        "Prompt response adjudicated at {:.0}% convergence",
                        convergence_pct * 100.0
                    ),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "convergence_pct": convergence_pct,
                    "filled_count": state.filled.len(),
                    "uncertain_count": state.uncertain.len(),
                    "missing_count": state.missing.len(),
                })),
            );
        }
    }

    async fn send_convergence(&self, result: &ConvergenceResult) {
        let reason = serde_json::to_string(&result.reason).unwrap_or_else(|_| "converged".into());

        self.send(ServerMessage::Converged {
            reason: reason.clone(),
            convergence_pct: result.convergence_pct,
        });

        if let Some(ref sink) = self.event_sink {
            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.converged",
                    format!(
                        "Socratic interview converged at {:.0}% (reason: {})",
                        result.convergence_pct * 100.0,
                        reason,
                    ),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "convergence_pct": result.convergence_pct,
                    "reason": reason,
                })),
            );
        }
    }

    async fn send_classification(&self, classification: &DomainClassification) {
        self.send(ServerMessage::Classified {
            project_type: classification.project_type.to_string(),
            complexity: match classification.complexity {
                planner_schemas::ComplexityTier::Light => "light".into(),
                planner_schemas::ComplexityTier::Standard => "standard".into(),
                planner_schemas::ComplexityTier::Deep => "deep".into(),
            },
        });

        if let Some(ref sink) = self.event_sink {
            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.classify.complete",
                    format!(
                        "Domain classified: {} ({})",
                        classification.project_type,
                        match classification.complexity {
                            planner_schemas::ComplexityTier::Light => "light",
                            planner_schemas::ComplexityTier::Standard => "standard",
                            planner_schemas::ComplexityTier::Deep => "deep",
                        },
                    ),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "project_type": classification.project_type.to_string(),
                    "complexity": match classification.complexity {
                        planner_schemas::ComplexityTier::Light => "light",
                        planner_schemas::ComplexityTier::Standard => "standard",
                        planner_schemas::ComplexityTier::Deep => "deep",
                    },
                })),
            );
        }
    }

    async fn receive_prompt_response(&self, prompt: &PromptEnvelope) -> Option<PromptResponse> {
        loop {
            let incoming = self.input_rx.lock().await.recv().await?;
            match incoming {
                SocraticRuntimeInput::PromptResponse(response) => {
                    if response.prompt_id != prompt.prompt_id {
                        tracing::warn!(
                            "Session {}: ignoring prompt_response for stale prompt '{}' (expected '{}')",
                            self.session_id,
                            response.prompt_id,
                            prompt.prompt_id
                        );
                        continue;
                    }
                    self.emit_prompt_submission_event(prompt, &response);
                    return Some(response);
                }
                SocraticRuntimeInput::Done => return None,
                SocraticRuntimeInput::DimensionEdit { .. } => {
                    // Dimension edits are applied by the websocket handler and
                    // are not prompt answers for the Socratic engine.
                    continue;
                }
            }
        }
    }

    fn current_ui_capabilities(&self) -> UiCapabilities {
        self.state
            .sessions
            .get(self.session_id)
            .and_then(|session| session.ui_capabilities)
            .unwrap_or(UiCapabilities {
                viewport_class: ViewportClass::Desktop,
                max_visible_items: 3,
                supports_split_draft_view: true,
            })
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
        if let Err(e) = self.checkpoint_tx.send(event.clone()) {
            tracing::warn!(
                "Failed to forward SocraticEvent for checkpoint projection: {}",
                e
            );
        }
    }
}

fn mark_interview_runtime_attached(state: &Arc<AppState>, session_id: Uuid) {
    let _ = state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.interview_runtime_active = true;
        s.interview_live_attached = true;
        s.ensure_socratic_run_id();
    });
}

fn mark_interview_detached_if_active(state: &Arc<AppState>, session_id: Uuid) {
    let _ = state.sessions.update(session_id, |s| {
        if s.intake_phase == "interviewing" {
            s.interview_runtime_active = true;
            s.interview_live_attached = false;
        }
    });
}

fn clear_interview_runtime_state(state: &Arc<AppState>, session_id: Uuid) {
    let _ = state.sessions.update(session_id, |s| {
        s.interview_runtime_active = false;
        s.interview_live_attached = false;
    });
}

fn update_session_ui_capabilities(
    state: &Arc<AppState>,
    session_id: Uuid,
    capabilities: UiCapabilities,
) {
    let _ = state.sessions.update(session_id, |s| {
        s.ui_capabilities = Some(capabilities);
    });
}

fn prompt_answer_to_input_text(
    answer: &PromptAnswer,
    prompt: Option<&PromptEnvelope>,
) -> Option<String> {
    if answer.skipped {
        return Some(String::from("skip"));
    }

    let matched_item =
        prompt.and_then(|p| p.items.iter().find(|item| item.item_id == answer.item_id));
    let selected = answer
        .selected_option_id
        .as_ref()
        .map(|selected_option_id| {
            matched_item
                .and_then(|item| {
                    item.options
                        .iter()
                        .find(|option| option.option_id == *selected_option_id)
                })
                .map(|option| option.semantic_value.clone())
                .unwrap_or_else(|| selected_option_id.clone())
        });
    let custom = answer
        .custom_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);

    match (selected, custom) {
        (Some(selected), Some(custom)) => Some(format!("{selected}\n{custom}")),
        (Some(selected), None) => Some(selected),
        (None, Some(custom)) => Some(custom),
        (None, None) => None,
    }
}

fn prompt_response_to_input(
    response: &PromptResponse,
    prompt: Option<&PromptEnvelope>,
) -> Option<String> {
    response
        .answers
        .iter()
        .find_map(|answer| prompt_answer_to_input_text(answer, prompt))
}

async fn replay_current_prompt_if_present(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    session_id: Uuid,
) -> Result<(), ()> {
    if let Some(msg) = current_prompt_replay_message(state, session_id) {
        send_ws_message(socket, &msg).await?;
    }

    Ok(())
}

fn current_prompt_replay_message(state: &Arc<AppState>, session_id: Uuid) -> Option<ServerMessage> {
    state
        .sessions
        .get(session_id)
        .and_then(|session| session.checkpoint)
        .and_then(|checkpoint| checkpoint.current_prompt)
        .map(|prompt| ServerMessage::Prompt { prompt })
}

pub fn expire_detached_runtimes(state: &Arc<AppState>) {
    for (session_id, runtime) in state.socratic_runtimes.expire_detached() {
        tracing::info!(
            "Session {}: live interview runtime lease expired; falling back to checkpoint resume",
            session_id
        );
        runtime.close_input();
        clear_interview_runtime_state(state, session_id);
    }
}

async fn send_ws_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|e| {
        tracing::warn!("failed to serialize websocket message: {}", e);
    })?;

    socket.send(Message::Text(json.into())).await.map_err(|e| {
        tracing::debug!("websocket send failed: {}", e);
    })
}

fn sorted_uncertain_confidences(state: &RequirementsBeliefState) -> Vec<f32> {
    let mut entries: Vec<(String, f32)> = state
        .uncertain
        .iter()
        .map(|(dim, (_slot, conf))| (dim.label(), *conf))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().map(|(_, conf)| conf).collect()
}

fn resume_pending_prompt_from_envelope(prompt: &PromptEnvelope) -> Option<ResumePendingPrompt> {
    Some(ResumePendingPrompt {
        prompt: prompt.clone(),
    })
}

fn apply_checkpoint_from_event(state: &Arc<AppState>, session_id: Uuid, event: &SocraticEvent) {
    let _ = state.sessions.update(session_id, |s| match event {
        SocraticEvent::Classified { classification } => {
            s.classification = Some(classification.clone());
            let checkpoint = s.ensure_checkpoint();
            checkpoint.classification = Some(classification.clone());
            checkpoint.touch();
        }
        SocraticEvent::BeliefStateUpdate { state: next_state } => {
            let previous_state = s
                .checkpoint
                .as_ref()
                .and_then(|cp| cp.belief_state.as_ref())
                .cloned();
            let previous_stale_turns = s.checkpoint.as_ref().map(|cp| cp.stale_turns).unwrap_or(0);

            let is_stale_turn = previous_state.as_ref().map_or(false, |prev| {
                let before_uncertain_confs = sorted_uncertain_confidences(prev);
                let after_uncertain_confs = sorted_uncertain_confidences(next_state);
                convergence::is_stale_turn(
                    prev.filled.len(),
                    next_state.filled.len(),
                    &before_uncertain_confs,
                    &after_uncertain_confs,
                )
            });

            s.belief_state = Some(next_state.clone());
            let checkpoint = s.ensure_checkpoint();
            checkpoint.belief_state = Some(next_state.clone());
            checkpoint.contradictions = next_state.contradictions.clone();
            checkpoint.current_prompt = None;
            checkpoint.stale_turns = if is_stale_turn {
                previous_stale_turns.saturating_add(1)
            } else {
                0
            };
            checkpoint.touch();
        }
        SocraticEvent::PromptGenerated { prompt } => {
            let checkpoint = s.ensure_checkpoint();
            checkpoint.current_prompt = Some(prompt.clone());
            if prompt.draft_snapshot.is_some() {
                checkpoint.draft_shown_at_turn = Some(prompt.based_on_turn);
            }
            checkpoint.touch();
        }
        SocraticEvent::ContradictionDetected { contradiction } => {
            let checkpoint = s.ensure_checkpoint();
            checkpoint.contradictions.push(contradiction.clone());
            checkpoint.touch();
        }
        SocraticEvent::Converged { .. } => {
            let checkpoint = s.ensure_checkpoint();
            checkpoint.current_prompt = None;
            checkpoint.touch();
        }
        SocraticEvent::SystemMessage { .. } | SocraticEvent::Error { .. } => {}
    });
}

fn build_checkpoint_resume_state(
    session: &crate::session::Session,
) -> Option<CheckpointResumeState> {
    let checkpoint = session.checkpoint.clone()?;
    let classification = checkpoint
        .classification
        .clone()
        .or_else(|| session.classification.clone());

    let belief_state = checkpoint
        .belief_state
        .clone()
        .or_else(|| session.belief_state.clone())
        .or_else(|| {
            classification
                .as_ref()
                .map(RequirementsBeliefState::from_classification)
        })?;

    let pending_prompt = checkpoint
        .current_prompt
        .as_ref()
        .and_then(resume_pending_prompt_from_envelope);

    Some(CheckpointResumeState {
        belief_state,
        classification,
        stale_turns: checkpoint.stale_turns,
        draft_shown_at_turn: checkpoint.draft_shown_at_turn,
        pending_prompt,
    })
}

enum InterviewStartMode {
    Fresh { initial_description: String },
    CheckpointResume { resume_state: CheckpointResumeState },
}

async fn run_interview_runtime(
    state: Arc<AppState>,
    session_id: Uuid,
    runtime: Arc<SessionRuntime>,
    input_rx: Arc<Mutex<mpsc::UnboundedReceiver<SocraticRuntimeInput>>>,
    start_mode: InterviewStartMode,
) {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
    let (checkpoint_tx, mut checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
    let (event_sink, mut planner_event_rx) = planner_core::observability::ChannelEventSink::new();
    let event_sink: Arc<dyn planner_core::observability::EventSink> = Arc::new(event_sink);

    let io = Arc::new(WsSocraticIO::new(
        state.clone(),
        event_tx,
        checkpoint_tx,
        input_rx,
        Some(event_sink.clone()),
        session_id,
    ));

    let run_id = state
        .sessions
        .get(session_id)
        .and_then(|s| s.socratic_run_id)
        .unwrap_or_else(Uuid::new_v4);

    let router = state.llm_router.clone();
    let requires_immediate_llm = match &start_mode {
        InterviewStartMode::Fresh { .. } => true,
        InterviewStartMode::CheckpointResume { resume_state } => {
            resume_state.pending_prompt.is_none()
        }
    };

    let available = router.available_providers();
    if requires_immediate_llm && available.is_empty() {
        tracing::error!(
            "Session {}: No LLM CLI providers found. Install and authenticate at least one of: claude, gemini, codex",
            session_id
        );
        let err_msg = "No LLM providers available. The planner service user needs at least one of the following CLI tools installed in /opt/planner/bin/ and authenticated: `claude` (Anthropic), `gemini` (Google), or `codex` (OpenAI). Run 'sudo ./deploy/install.sh --update' to install them.";

        runtime.publish(ServerMessage::Error {
            message: err_msg.to_string(),
        });
        state.sessions.update(session_id, |s| {
            s.intake_phase = "error".into();
            s.interview_live_attached = false;
            s.interview_runtime_active = false;
            s.add_message("system", err_msg);
        });
        runtime.close_input();
        runtime.signal_closed();
        let _ = state.socratic_runtimes.remove(session_id);
        return;
    }

    if available.is_empty() {
        tracing::warn!(
            "Session {}: resuming from checkpoint prompt without pre-flight providers; LLM will be required after the next user response",
            session_id
        );
    } else {
        tracing::info!(
            "Session {}: LLM providers available: {:?}",
            session_id,
            available
        );
    }

    event_sink.emit(
        planner_core::observability::PlannerEvent::info(
            planner_core::observability::EventSource::System,
            "system.session.start",
            format!("Session {} starting Socratic interview", session_id),
        )
        .with_session(session_id)
        .with_metadata(serde_json::json!({
            "providers": available.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        })),
    );

    let state_for_engine = state.clone();
    let runtime_for_engine = runtime.clone();
    let sink = event_sink.clone();
    let engine_handle = tokio::spawn(async move {
        let result = match start_mode {
            InterviewStartMode::Fresh {
                initial_description,
            } => match state_for_engine.cxdb.as_ref() {
                Some(engine) => {
                    planner_core::pipeline::steps::socratic::run_interview::<
                        WsSocraticIO,
                        planner_core::cxdb::durable::DurableCxdbEngine,
                    >(&router, &*io, Some(engine), run_id, &initial_description)
                    .await
                }
                None => {
                    planner_core::pipeline::steps::socratic::run_interview::<
                        WsSocraticIO,
                        planner_core::cxdb::CxdbEngine,
                    >(
                        &router,
                        &*io,
                        None::<&planner_core::cxdb::CxdbEngine>,
                        run_id,
                        &initial_description,
                    )
                    .await
                }
            },
            InterviewStartMode::CheckpointResume { resume_state } => {
                match state_for_engine.cxdb.as_ref() {
                    Some(engine) => {
                        run_interview_from_checkpoint::<
                            WsSocraticIO,
                            planner_core::cxdb::durable::DurableCxdbEngine,
                        >(
                            &router, &*io, Some(engine), run_id, resume_state.clone()
                        )
                        .await
                    }
                    None => run_interview_from_checkpoint::<
                        WsSocraticIO,
                        planner_core::cxdb::CxdbEngine,
                    >(
                        &router,
                        &*io,
                        None::<&planner_core::cxdb::CxdbEngine>,
                        run_id,
                        resume_state,
                    )
                    .await,
                }
            }
        };

        match result {
            Ok(session) => {
                let did_converge = session
                    .convergence_result
                    .as_ref()
                    .map(|r| r.is_done)
                    .unwrap_or(false);

                state_for_engine.sessions.update(session_id, |s| {
                    s.belief_state = Some(session.belief_state.clone());
                    s.classification = session.belief_state.classification.clone();
                    let checkpoint = s.ensure_checkpoint();
                    checkpoint.belief_state = Some(session.belief_state.clone());
                    checkpoint.classification = session.belief_state.classification.clone();
                    checkpoint.contradictions = session.belief_state.contradictions.clone();
                    if did_converge {
                        checkpoint.current_prompt = None;
                    }
                    checkpoint.touch();
                    if did_converge {
                        s.intake_phase = "pipeline_running".into();
                        s.interview_live_attached = false;
                        s.interview_runtime_active = false;
                    }
                });

                if did_converge {
                    sink.emit(
                        planner_core::observability::PlannerEvent::info(
                            planner_core::observability::EventSource::SocraticEngine,
                            "socratic.converged",
                            "Socratic interview converged, starting pipeline",
                        )
                        .with_session(session_id),
                    );

                    let intake = planner_core::pipeline::steps::socratic::session_to_intake(
                        &session,
                        Uuid::new_v4(),
                    );
                    Some(intake.intent_summary)
                } else {
                    sink.emit(
                        planner_core::observability::PlannerEvent::warn(
                            planner_core::observability::EventSource::SocraticEngine,
                            "socratic.detached",
                            "Socratic interview ended before explicit convergence",
                        )
                        .with_session(session_id),
                    );
                    None
                }
            }
            Err(e) => {
                let err_msg = format!("Socratic interview failed: {}", e);
                tracing::warn!("Session {}: {}", session_id, err_msg);

                sink.emit(
                    planner_core::observability::PlannerEvent::error(
                        planner_core::observability::EventSource::SocraticEngine,
                        "socratic.error",
                        format!("Socratic interview failed: {}", e),
                    )
                    .with_session(session_id),
                );

                runtime_for_engine.publish(ServerMessage::Error {
                    message: err_msg.clone(),
                });

                state_for_engine.sessions.update(session_id, |s| {
                    s.intake_phase = "error".into();
                    s.interview_live_attached = false;
                    s.interview_runtime_active = false;
                    s.add_message("system", &err_msg);
                });

                None
            }
        }
    });

    let mut engine_handle = Box::pin(engine_handle);
    let mut engine_finished = false;
    let mut description = None;

    loop {
        tokio::select! {
            msg = event_rx.recv() => {
                match msg {
                    Some(server_msg) => {
                        runtime.publish(server_msg);
                    }
                    None => {
                        if engine_finished {
                            break;
                        }
                    }
                }
            }
            planner_evt = planner_event_rx.recv() => {
                if let Some(evt) = planner_evt {
                    state.sessions.update(session_id, |s| {
                        s.record_event(evt.clone());
                    });
                    if let Some(ref store) = state.event_store {
                        if let Some(session) = state.sessions.get(session_id) {
                            if let Err(e) = store.save_session_events(session_id, &session.events) {
                                tracing::warn!(
                                    "Failed to persist events for session {}: {}",
                                    session_id,
                                    e
                                );
                            }
                        }
                    }
                    runtime.publish(ServerMessage::PlannerEvent {
                        id: evt.id.to_string(),
                        timestamp: evt.timestamp.to_rfc3339(),
                        level: format!("{}", evt.level),
                        source: format!("{}", evt.source),
                        step: evt.step.clone(),
                        message: evt.message.clone(),
                        duration_ms: evt.duration_ms,
                        metadata: evt.metadata.clone(),
                    });
                }
            }
            checkpoint_evt = checkpoint_rx.recv() => {
                if let Some(evt) = checkpoint_evt {
                    apply_checkpoint_from_event(&state, session_id, &evt);
                }
            }
            result = &mut engine_handle => {
                engine_finished = true;
                description = result.ok().flatten();
                if event_rx.is_closed() {
                    break;
                }
            }
        }
    }

    clear_interview_runtime_state(&state, session_id);
    let _ = state.socratic_runtimes.remove(session_id);
    runtime.close_input();
    runtime.signal_closed();

    if let Some(description) = description {
        let description = if description.trim().is_empty() {
            state
                .sessions
                .get(session_id)
                .and_then(|s| s.project_description.clone())
                .unwrap_or_else(|| "Project requirements gathered via Socratic interview".into())
        } else {
            description
        };

        let was_running = state
            .sessions
            .get(session_id)
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

            let _ = crate::api::spawn_pipeline_runtime(state.clone(), session_id, description);
        }
    }
}

fn start_interview_runtime(
    state: &Arc<AppState>,
    session_id: Uuid,
    start_mode: InterviewStartMode,
) -> Result<Arc<SessionRuntime>, Arc<SessionRuntime>> {
    let (runtime, input_rx) = SessionRuntime::new();
    if let Err(existing) = state
        .socratic_runtimes
        .try_insert(session_id, runtime.clone())
    {
        return Err(existing);
    }

    tokio::spawn(run_interview_runtime(
        state.clone(),
        session_id,
        runtime.clone(),
        input_rx,
        start_mode,
    ));

    Ok(runtime)
}

async fn wait_for_initial_description(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    session_id: Uuid,
) -> Option<String> {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::PromptResponse { response }) => {
                    if let Some(content) = prompt_response_to_input(&response, None) {
                        return Some(content);
                    }
                }
                Ok(ClientMessage::StartPipeline { description }) => return Some(description),
                Ok(ClientMessage::UiCapabilities { capabilities }) => {
                    update_session_ui_capabilities(state, session_id, capabilities);
                }
                Ok(ClientMessage::Done) => return None,
                Ok(_) => continue,
                Err(e) => {
                    tracing::warn!(
                        "Session {}: failed to parse initial client message: {}",
                        session_id,
                        e
                    );
                }
            },
            Some(Ok(Message::Close(_))) | None => return None,
            _ => {}
        }
    }
}

async fn handle_live_runtime_ws(
    mut socket: WebSocket,
    state: Arc<AppState>,
    session_id: Uuid,
    runtime: Arc<SessionRuntime>,
    attachment: RuntimeAttachment,
) {
    let RuntimeAttachment {
        input_tx,
        mut outbound_rx,
    } = attachment;
    let mut shutdown_rx = runtime.subscribe_shutdown();

    mark_interview_runtime_attached(&state, session_id);
    if replay_current_prompt_if_present(&mut socket, &state, session_id)
        .await
        .is_err()
    {
        runtime.mark_detached();
        mark_interview_detached_if_active(&state, session_id);
        return;
    }

    loop {
        tokio::select! {
            msg = outbound_rx.recv() => {
                match msg {
                    Ok(server_msg) => {
                        if send_ws_message(&mut socket, &server_msg).await.is_err() {
                            runtime.mark_detached();
                            mark_interview_detached_if_active(&state, session_id);
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "Session {}: websocket subscriber lagged behind live runtime by {} messages",
                            session_id,
                            skipped
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::PromptResponse { response }) => {
                                let prompt_for_response = state
                                    .sessions
                                    .get(session_id)
                                    .and_then(|session| session.checkpoint)
                                    .and_then(|checkpoint| checkpoint.current_prompt)
                                    .filter(|prompt| prompt.prompt_id == response.prompt_id);
                                if let Some(content) =
                                    prompt_response_to_input(&response, prompt_for_response.as_ref())
                                {
                                    state.sessions.update(session_id, |s| {
                                        s.add_message("user", &content);
                                    });
                                } else {
                                    tracing::debug!(
                                        "Session {}: prompt_response '{}' had no displayable inline content",
                                        session_id,
                                        response.prompt_id
                                    );
                                }
                                let _ = input_tx.send(SocraticRuntimeInput::PromptResponse(response));
                            }
                            Ok(ClientMessage::UiCapabilities { capabilities }) => {
                                update_session_ui_capabilities(&state, session_id, capabilities);
                            }
                            Ok(ClientMessage::Done) => {
                                let _ = input_tx.send(SocraticRuntimeInput::Done);
                            }
                            Ok(ClientMessage::DimensionEdit { dimension, new_value }) => {
                                state.sessions.update(session_id, |s| {
                                    s.add_message("user", &format!("Edited dimension '{}' → '{}'", dimension, new_value));
                                });
                                let _ = input_tx.send(SocraticRuntimeInput::DimensionEdit {
                                    dimension,
                                    new_value,
                                });
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(
                                    "Session {}: failed to parse client message: {}",
                                    session_id,
                                    e
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        runtime.mark_detached();
                        mark_interview_detached_if_active(&state, session_id);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    let phase = state
        .sessions
        .get(session_id)
        .map(|s| s.intake_phase)
        .unwrap_or_default();

    if phase == "pipeline_running" || phase == "complete" {
        handle_resume_ws(socket, state, session_id).await;
    }
}

// ---------------------------------------------------------------------------
// handle_socratic_ws — main WebSocket handler
// ---------------------------------------------------------------------------

/// Drive a Socratic interview session over a WebSocket connection.
///
/// ## Protocol
///
/// 1. The first `prompt_response` (or `StartPipeline`) message carries the
///    initial project description and starts `run_interview`.
/// 2. Subsequent `prompt_response` / `Done` messages are
///    forwarded to the engine via `input_tx`.
/// 3. After convergence the session transitions to pipeline mode and the
///    existing `api::run_pipeline_for_session` task is spawned.
///
/// The caller is responsible for verifying session ownership before invoking
/// this function.
pub async fn handle_socratic_ws(mut socket: WebSocket, state: Arc<AppState>, session_id: Uuid) {
    // Verify the session exists.
    if state.sessions.get(session_id).is_none() {
        let err = ServerMessage::Error {
            message: format!("Session {} not found", session_id),
        };
        let _ = send_ws_message(&mut socket, &err).await;
        return;
    }

    // Fast path: attach to an existing live interview runtime.
    if let Some(runtime) = state.socratic_runtimes.get(session_id) {
        match runtime.try_attach() {
            Ok(attachment) => {
                handle_live_runtime_ws(socket, state, session_id, runtime, attachment).await;
                return;
            }
            Err(AttachError::AlreadyAttached) => {
                let err = ServerMessage::Error {
                    message: "A live interview connection is already attached to this session"
                        .into(),
                };
                let _ = send_ws_message(&mut socket, &err).await;
                return;
            }
            Err(AttachError::Closed) => {
                let _ = state.socratic_runtimes.remove(session_id);
                clear_interview_runtime_state(&state, session_id);
            }
        }
    }

    let mut session = match state.sessions.get(session_id) {
        Some(session) => session,
        None => return,
    };
    if session.intake_phase == "interviewing" && session.interview_live_attached {
        clear_interview_runtime_state(&state, session_id);
        session = match state.sessions.get(session_id) {
            Some(session) => session,
            None => return,
        };
    }
    if session.intake_phase == "pipeline_running"
        || session.intake_phase == "complete"
        || session.intake_phase == "error"
    {
        handle_resume_ws(socket, state, session_id).await;
        return;
    }

    let checkpoint_resume_state = if session.intake_phase == "interviewing" {
        build_checkpoint_resume_state(&session)
    } else {
        None
    };

    let start_mode = if let Some(resume_state) = checkpoint_resume_state {
        let _ = state.sessions.update(session_id, |s| {
            s.intake_phase = "interviewing".into();
            s.ensure_socratic_run_id();
        });
        InterviewStartMode::CheckpointResume { resume_state }
    } else {
        let Some(initial_description) =
            wait_for_initial_description(&mut socket, &state, session_id).await
        else {
            return;
        };

        let _ = state.sessions.update(session_id, |s| {
            s.intake_phase = "interviewing".into();
            let run_id = s.ensure_socratic_run_id();
            s.checkpoint = Some(crate::session::InterviewCheckpoint::new(run_id));
            s.has_checkpoint = true;
            s.interview_runtime_active = false;
            s.interview_live_attached = false;
            s.add_message("user", &initial_description);
        });

        InterviewStartMode::Fresh {
            initial_description,
        }
    };

    let runtime = match start_interview_runtime(&state, session_id, start_mode) {
        Ok(runtime) => runtime,
        Err(existing) => existing,
    };

    let attachment = match runtime.try_attach() {
        Ok(attachment) => attachment,
        Err(AttachError::AlreadyAttached) => {
            let err = ServerMessage::Error {
                message: "A live interview connection is already attached to this session".into(),
            };
            let _ = send_ws_message(&mut socket, &err).await;
            return;
        }
        Err(AttachError::Closed) => {
            let _ = state.socratic_runtimes.remove(session_id);
            clear_interview_runtime_state(&state, session_id);
            let err = ServerMessage::Error {
                message: "The live interview runtime closed before this websocket could attach"
                    .into(),
            };
            let _ = send_ws_message(&mut socket, &err).await;
            return;
        }
    };

    handle_live_runtime_ws(socket, state, session_id, runtime, attachment).await;
}

/// Attach to an already-started session without restarting interview state.
///
/// Used for sessions in `pipeline_running`, `complete`, or `error`.
/// This path is strictly read-only against session state and only forwards
/// incremental updates/events to the client.
async fn handle_resume_ws(mut socket: WebSocket, state: Arc<AppState>, session_id: Uuid) {
    let Some(initial) = state.sessions.get(session_id) else {
        return;
    };
    let initial_phase = initial.intake_phase.clone();

    // Frontend hydrates snapshot via REST first; only stream updates from now on.
    let mut last_msg_count = initial.messages.len();
    let mut last_event_count = initial.events.len();
    let mut last_sent_stages: Vec<(String, String)> = initial
        .stages
        .iter()
        .map(|s| (s.name.clone(), s.status.clone()))
        .collect();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let session = match state.sessions.get(session_id) {
                    Some(s) => s,
                    None => return,
                };

                // Forward any new chat messages since attach.
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

                // Forward any new planner events since attach.
                let current_event_count = session.events.len();
                for evt in session.events.iter().skip(last_event_count) {
                    let server_msg = ServerMessage::PlannerEvent {
                        id: evt.id.to_string(),
                        timestamp: evt.timestamp.to_rfc3339(),
                        level: format!("{}", evt.level),
                        source: format!("{}", evt.source),
                        step: evt.step.clone(),
                        message: evt.message.clone(),
                        duration_ms: evt.duration_ms,
                        metadata: evt.metadata.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            return;
                        }
                    }
                }
                last_event_count = current_event_count;

                // Forward stage updates only when changed since attach.
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

                // Send terminal notifications for completed/errored sessions.
                if initial_phase == "complete" || session.intake_phase == "complete" {
                    let success = session.stages.iter().all(|s| s.status == "complete");
                    let server_msg = ServerMessage::PipelineComplete {
                        success,
                        summary: "Pipeline finished".into(),
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
                    return;
                }

                if initial_phase == "error" || session.intake_phase == "error" {
                    let err = session
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "Session is in an error state".into());
                    let server_msg = ServerMessage::Error { message: err };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
                    return;
                }

                // Pipeline-running attach: close when pipeline completes.
                if initial_phase == "pipeline_running"
                    && !session.pipeline_running
                    && session.project_description.is_some()
                {
                    let success = session.stages.iter().all(|s| s.status == "complete");
                    let server_msg = ServerMessage::PipelineComplete {
                        success,
                        summary: "Pipeline finished".into(),
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStore;
    use crate::AppState;
    use planner_schemas::{
        ComplexityTier, Dimension, DraftSection, ProjectType, PromptAnswer, PromptEnvelope,
        PromptItem, PromptItemKind, PromptKind, PromptOption, PromptPreferredLayout,
        PromptResponse, PromptResponseMode, PromptUiHints, SpeculativeDraft, UiCapabilities,
        ViewportClass,
    };

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
        })
    }

    fn test_prompt(text: &str) -> PromptEnvelope {
        PromptEnvelope {
            prompt_id: "prompt-test".into(),
            kind: PromptKind::QuestionBatch,
            title: "Continue interview".into(),
            instructions: None,
            items: vec![PromptItem {
                item_id: "item-1".into(),
                kind: PromptItemKind::Discovery,
                target_dimension: Some(Dimension::Stakeholders),
                section_ref: None,
                text: text.into(),
                options: vec![PromptOption {
                    option_id: "option-1".into(),
                    label: "Internal team".into(),
                    semantic_value: "internal_team".into(),
                    direct_effect: None,
                }],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority: 100,
                dependency_item_ids: Vec::new(),
            }],
            draft_snapshot: None,
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Cards,
                show_draft_sidebar: false,
            },
            based_on_turn: 1,
            created_at: "2026-03-08T00:00:00Z".into(),
        }
    }

    fn test_draft_prompt() -> PromptEnvelope {
        let draft = SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a resilient planner".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: vec![Dimension::Integrations],
        };

        PromptEnvelope {
            prompt_id: "prompt-draft".into(),
            kind: PromptKind::DraftReview,
            title: "Review draft".into(),
            instructions: Some(
                "Confirm accurate sections and provide corrections where needed.".into(),
            ),
            items: vec![PromptItem {
                item_id: "draft-item-1".into(),
                kind: PromptItemKind::DraftSection,
                target_dimension: Some(Dimension::Goal),
                section_ref: Some("Goal".into()),
                text: "Review section 'Goal'. Confirm or correct it.".into(),
                options: vec![
                    PromptOption {
                        option_id: "confirm".into(),
                        label: "Looks correct".into(),
                        semantic_value: "confirm".into(),
                        direct_effect: None,
                    },
                    PromptOption {
                        option_id: "correct".into(),
                        label: "Needs correction".into(),
                        semantic_value: "correct".into(),
                        direct_effect: None,
                    },
                ],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority: 100,
                dependency_item_ids: Vec::new(),
            }],
            draft_snapshot: Some(draft),
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Review,
                show_draft_sidebar: true,
            },
            based_on_turn: 2,
            created_at: "2026-03-08T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn ws_socratic_io_send_classification() {
        let state = test_state();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        use planner_schemas::{ComplexityTier, Dimension, DomainClassification, ProjectType};

        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        use planner_core::pipeline::steps::socratic::SocraticIO;
        io.send_classification(&classification).await;

        let msg = event_rx.try_recv().unwrap();
        match msg {
            ServerMessage::Classified {
                project_type,
                complexity,
            } => {
                assert_eq!(project_type, "Web App");
                assert_eq!(complexity, "standard");
            }
            _ => panic!("expected Classified, got {:?}", msg),
        }
    }

    #[tokio::test]
    async fn ws_socratic_io_send_message() {
        let state = test_state();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

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
    async fn ws_socratic_io_receive_prompt_response() {
        let state = test_state();
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        let prompt = test_prompt("Test question");
        let item_id = prompt
            .items
            .first()
            .expect("prompt should include item")
            .item_id
            .clone();
        input_tx
            .send(SocraticRuntimeInput::PromptResponse(PromptResponse {
                prompt_id: prompt.prompt_id.clone(),
                answers: vec![PromptAnswer {
                    item_id,
                    selected_option_id: None,
                    custom_text: Some("hello world".into()),
                    skipped: false,
                }],
                submitted_at: "2026-03-08T00:00:00Z".into(),
                client_context: None,
            }))
            .unwrap();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let received = io.receive_prompt_response(&prompt).await;
        assert_eq!(
            received
                .as_ref()
                .and_then(|response| response.answers.first())
                .and_then(|answer| answer.custom_text.as_deref()),
            Some("hello world")
        );
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_prompt_response_returns_none_when_closed() {
        let state = test_state();
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        // Drop the sender — channel is closed.
        drop(input_tx);

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let prompt = test_prompt("Test question");
        let received = io.receive_prompt_response(&prompt).await;
        assert!(received.is_none());
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_prompt_response_ignores_stale_prompt_ids() {
        let state = test_state();
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        input_tx
            .send(SocraticRuntimeInput::PromptResponse(PromptResponse {
                prompt_id: "stale-prompt".into(),
                answers: vec![PromptAnswer {
                    item_id: "item-1".into(),
                    selected_option_id: None,
                    custom_text: Some("stale".into()),
                    skipped: false,
                }],
                submitted_at: "2026-03-08T00:00:00Z".into(),
                client_context: None,
            }))
            .unwrap();
        input_tx.send(SocraticRuntimeInput::Done).unwrap();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let prompt = test_prompt("Test question");
        let received = io.receive_prompt_response(&prompt).await;
        assert!(received.is_none());
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_prompt_response_emits_partial_submit_event() {
        let state = test_state();
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let sink = Arc::new(planner_core::observability::CollectorEventSink::new());
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            Some(sink.clone()),
            Uuid::new_v4(),
        );

        let mut prompt = test_prompt("Which team owns this first?");
        prompt.items.push(PromptItem {
            item_id: "item-2".into(),
            kind: PromptItemKind::Verification,
            target_dimension: Some(Dimension::Performance),
            section_ref: None,
            text: "How strict should latency targets be?".into(),
            options: vec![PromptOption {
                option_id: "fast".into(),
                label: "Strict".into(),
                semantic_value: "strict".into(),
                direct_effect: None,
            }],
            response_mode: PromptResponseMode::SingleSelectWithCustomText,
            required: false,
            priority: 50,
            dependency_item_ids: Vec::new(),
        });

        input_tx
            .send(SocraticRuntimeInput::PromptResponse(PromptResponse {
                prompt_id: prompt.prompt_id.clone(),
                answers: vec![PromptAnswer {
                    item_id: "item-1".into(),
                    selected_option_id: Some("option-1".into()),
                    custom_text: None,
                    skipped: false,
                }],
                submitted_at: "2026-03-08T00:00:00Z".into(),
                client_context: None,
            }))
            .unwrap();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let received = io.receive_prompt_response(&prompt).await;
        assert!(received.is_some());

        let steps: Vec<Option<String>> =
            sink.events().into_iter().map(|event| event.step).collect();
        assert!(steps
            .iter()
            .any(|step| step.as_deref() == Some("socratic.prompt.partial_submitted")));
    }

    #[tokio::test]
    async fn ws_socratic_io_send_prompt_emits_draft_generated_event() {
        let state = test_state();
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, _checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let sink = Arc::new(planner_core::observability::CollectorEventSink::new());
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            Some(sink.clone()),
            Uuid::new_v4(),
        );

        let prompt = test_draft_prompt();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        io.send_prompt(&prompt).await;

        let steps: Vec<Option<String>> =
            sink.events().into_iter().map(|event| event.step).collect();
        assert!(steps
            .iter()
            .any(|step| step.as_deref() == Some("socratic.prompt.generated")));
        assert!(steps
            .iter()
            .any(|step| step.as_deref() == Some("socratic.draft.generated")));
    }

    #[tokio::test]
    async fn ws_socratic_io_send_event_contradiction() {
        let state = test_state();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (checkpoint_tx, mut checkpoint_rx) = mpsc::unbounded_channel::<SocraticEvent>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<SocraticRuntimeInput>();
        let io = WsSocraticIO::new(
            state,
            event_tx,
            checkpoint_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        use planner_schemas::{Contradiction, Dimension};
        let contradiction = Contradiction {
            dimension_a: Dimension::DataModel,
            value_a: "PostgreSQL".into(),
            dimension_b: Dimension::Integrations,
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
            ServerMessage::ContradictionDetected {
                dimension_a,
                dimension_b,
                explanation,
                ..
            } => {
                assert_eq!(dimension_a, "Data Model");
                assert_eq!(dimension_b, "Integrations");
                assert!(explanation.contains("persistent server"));
            }
            other => panic!("expected ContradictionDetected, got {:?}", other),
        }

        // Raw Socratic events are now forwarded through the checkpoint channel
        // rather than being serialized into chat messages.
        let projected = checkpoint_rx.try_recv().unwrap();
        match projected {
            SocraticEvent::ContradictionDetected { contradiction } => {
                assert_eq!(contradiction.dimension_a, Dimension::DataModel);
                assert_eq!(contradiction.dimension_b, Dimension::Integrations);
            }
            other => panic!(
                "expected ContradictionDetected checkpoint event, got {:?}",
                other
            ),
        }

        // No extra chat message should be emitted for operational events.
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn checkpoint_updates_on_prompt_generated_event() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;

        let event = SocraticEvent::PromptGenerated {
            prompt: test_prompt("Who will use this tool most often?"),
        };

        apply_checkpoint_from_event(&state, session_id, &event);

        let after = state
            .sessions
            .get(session_id)
            .expect("session should exist");
        let checkpoint = after.checkpoint.expect("checkpoint should be present");
        assert_eq!(
            checkpoint
                .current_prompt
                .as_ref()
                .and_then(|prompt| prompt.items.first())
                .map(|item| item.text.as_str()),
            Some("Who will use this tool most often?")
        );
        assert!(after.has_checkpoint);
    }

    #[test]
    fn checkpoint_updates_on_draft_prompt_generated_event() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;

        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        let mut belief_state = RequirementsBeliefState::from_classification(&classification);
        belief_state.turn_count = 3;

        apply_checkpoint_from_event(
            &state,
            session_id,
            &SocraticEvent::BeliefStateUpdate {
                state: belief_state.clone(),
            },
        );

        let mut draft_prompt = test_draft_prompt();
        draft_prompt.based_on_turn = 3;
        apply_checkpoint_from_event(
            &state,
            session_id,
            &SocraticEvent::PromptGenerated {
                prompt: draft_prompt,
            },
        );

        let after = state
            .sessions
            .get(session_id)
            .expect("session should exist");
        let checkpoint = after.checkpoint.expect("checkpoint should be present");
        assert_eq!(
            checkpoint
                .current_prompt
                .as_ref()
                .and_then(|prompt| prompt.draft_snapshot.as_ref())
                .and_then(|draft| draft.sections.first())
                .map(|section| section.heading.as_str()),
            Some("Goal")
        );
        assert_eq!(checkpoint.draft_shown_at_turn, Some(3));
    }

    #[tokio::test]
    async fn socratic_pipeline_transition_registers_pipeline_runtime() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;
        let description = "Socratic interview converged".to_string();

        let was_running = state
            .sessions
            .get(session_id)
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
            let _ = crate::api::spawn_pipeline_runtime(state.clone(), session_id, description);
        }

        assert!(state.pipeline_runtimes.get(session_id).is_some());
        crate::api::stop_active_session_work(&state, session_id);
    }

    #[test]
    fn prompt_response_to_input_uses_structured_answer_payload() {
        let prompt = test_prompt("Who is this for?");
        let item = prompt.items.first().expect("prompt item");

        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![PromptAnswer {
                item_id: item.item_id.clone(),
                selected_option_id: Some("option-1".into()),
                custom_text: Some("plus contractors".into()),
                skipped: false,
            }],
            submitted_at: "2026-03-08T00:00:00Z".into(),
            client_context: None,
        };

        let as_input = prompt_response_to_input(&response, Some(&prompt));
        assert_eq!(as_input.as_deref(), Some("internal_team\nplus contractors"));
    }

    #[test]
    fn current_prompt_replay_message_replays_checkpoint_prompt() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;

        apply_checkpoint_from_event(
            &state,
            session_id,
            &SocraticEvent::PromptGenerated {
                prompt: test_prompt("What should this optimize first?"),
            },
        );

        let replay = current_prompt_replay_message(&state, session_id)
            .expect("checkpoint prompt should be replayable");
        match replay {
            ServerMessage::Prompt { prompt } => {
                assert_eq!(prompt.kind, PromptKind::QuestionBatch);
                assert_eq!(
                    prompt.items.first().map(|item| item.text.as_str()),
                    Some("What should this optimize first?")
                );
            }
            other => panic!("expected prompt replay message, got {:?}", other),
        }
    }

    #[test]
    fn ui_capabilities_are_tracked_on_session_state() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;

        update_session_ui_capabilities(
            &state,
            session_id,
            UiCapabilities {
                viewport_class: ViewportClass::Desktop,
                max_visible_items: 4,
                supports_split_draft_view: true,
            },
        );

        let stored = state
            .sessions
            .get(session_id)
            .expect("session should exist")
            .ui_capabilities
            .expect("ui capabilities should be stored");
        assert_eq!(stored.viewport_class, ViewportClass::Desktop);
        assert_eq!(stored.max_visible_items, 4);
        assert!(stored.supports_split_draft_view);
    }
}
