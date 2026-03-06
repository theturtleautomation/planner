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

use planner_core::observability::EventSink;
use planner_core::pipeline::steps::socratic::convergence;
use planner_core::pipeline::steps::socratic::{
    run_interview_from_checkpoint, CheckpointResumeState, ResumePendingPrompt,
};

use planner_schemas::{
    ConvergenceResult, DomainClassification, QuestionOutput, RequirementsBeliefState,
    SocraticEvent, SpeculativeDraft,
};

// Import SocraticIO trait so we can call .send_message() on Arc<WsSocraticIO>
// inside the spawned engine task's error handler.
use planner_core::pipeline::steps::socratic::SocraticIO;

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
    /// Send events to the WebSocket client.
    event_tx: mpsc::UnboundedSender<ServerMessage>,
    /// Receive user input forwarded from the WebSocket client.
    input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    /// Optional observability event sink.
    event_sink: Option<Arc<dyn EventSink>>,
    /// Session ID for tagging emitted events.
    session_id: Uuid,
}

impl WsSocraticIO {
    pub fn new(
        event_tx: mpsc::UnboundedSender<ServerMessage>,
        input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
        event_sink: Option<Arc<dyn EventSink>>,
        session_id: Uuid,
    ) -> Self {
        Self {
            event_tx,
            input_rx,
            event_sink,
            session_id,
        }
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

        if let Some(ref sink) = self.event_sink {
            sink.emit(
                planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.question.generated",
                    format!(
                        "Question generated for dimension '{}'",
                        output.target_dimension.label(),
                    ),
                )
                .with_session(self.session_id)
                .with_metadata(serde_json::json!({
                    "target_dimension": output.target_dimension.label(),
                    "allow_skip": output.allow_skip,
                })),
            );
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
        }
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

        let not_discussed = draft.not_discussed.iter().map(|d| d.label()).collect();

        self.send(ServerMessage::SpeculativeDraft {
            sections,
            assumptions,
            not_discussed,
        });
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

fn mark_interview_detached_if_active(state: &Arc<AppState>, session_id: Uuid) {
    let _ = state.sessions.update(session_id, |s| {
        if s.intake_phase == "interviewing" {
            s.interview_live_attached = false;
        }
    });
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
            checkpoint.current_question = None;
            checkpoint.pending_draft = None;
            checkpoint.stale_turns = if is_stale_turn {
                previous_stale_turns.saturating_add(1)
            } else {
                0
            };
            checkpoint.touch();
        }
        SocraticEvent::Question { output } => {
            let checkpoint = s.ensure_checkpoint();
            checkpoint.current_question = Some(output.clone());
            checkpoint.pending_draft = None;
            checkpoint.touch();
        }
        SocraticEvent::SpeculativeDraftReady { draft } => {
            let draft_turn = s
                .checkpoint
                .as_ref()
                .and_then(|cp| cp.belief_state.as_ref())
                .map(|bs| bs.turn_count);
            let checkpoint = s.ensure_checkpoint();
            checkpoint.current_question = None;
            checkpoint.pending_draft = Some(draft.clone());
            if let Some(turn) = draft_turn {
                checkpoint.draft_shown_at_turn = Some(turn);
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
            checkpoint.current_question = None;
            checkpoint.pending_draft = None;
            checkpoint.touch();
        }
        SocraticEvent::SystemMessage { .. } | SocraticEvent::Error { .. } => {}
    });
}

fn apply_checkpoint_from_server_message(
    state: &Arc<AppState>,
    session_id: Uuid,
    server_msg: &ServerMessage,
) {
    if let ServerMessage::ChatMessage { role, content, .. } = server_msg {
        if role == "event" {
            if let Ok(event) = serde_json::from_str::<SocraticEvent>(content) {
                apply_checkpoint_from_event(state, session_id, &event);
            }
        }
    }
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

    let pending_prompt = if let Some(output) = checkpoint.current_question.clone() {
        Some(ResumePendingPrompt::Question(output))
    } else {
        checkpoint
            .pending_draft
            .clone()
            .map(ResumePendingPrompt::Draft)
    };

    Some(CheckpointResumeState {
        belief_state,
        classification,
        stale_turns: checkpoint.stale_turns,
        draft_shown_at_turn: checkpoint.draft_shown_at_turn,
        pending_prompt,
    })
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
pub async fn handle_socratic_ws(mut socket: WebSocket, state: Arc<AppState>, session_id: Uuid) {
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

    // Attach-only resume for non-waiting terminal/build phases.
    let (intake_phase, checkpoint_resume_state) = state
        .sessions
        .get(session_id)
        .map(|s| {
            let resume_state = if s.intake_phase == "interviewing" && !s.interview_live_attached {
                build_checkpoint_resume_state(&s)
            } else {
                None
            };
            (s.intake_phase.clone(), resume_state)
        })
        .unwrap_or_else(|| ("waiting".to_string(), None));
    if intake_phase == "pipeline_running" || intake_phase == "complete" || intake_phase == "error" {
        handle_resume_ws(socket, state, session_id).await;
        return;
    }

    // Channel: engine → WebSocket (outbound events)
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Channel: WebSocket → engine (inbound user text)
    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    let input_rx = Arc::new(Mutex::new(input_rx));

    enum InterviewStartMode {
        Fresh { initial_description: String },
        CheckpointResume { resume_state: CheckpointResumeState },
    }

    let start_mode = if let Some(resume_state) = checkpoint_resume_state {
        let _ = state.sessions.update(session_id, |s| {
            s.intake_phase = "interviewing".into();
            s.interview_live_attached = true;
            s.ensure_socratic_run_id();
        });
        InterviewStartMode::CheckpointResume { resume_state }
    } else {
        // Wait for the first SocraticResponse / StartPipeline to get the
        // initial project description before launching the engine.
        let initial_description = loop {
            match socket.recv().await {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text)
                {
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
                },
                Some(Ok(Message::Close(_))) | None => {
                    mark_interview_detached_if_active(&state, session_id);
                    return;
                }
                _ => continue,
            }
        };

        // Mark session as interviewing and ensure a stable Socratic run ID.
        let _ = state.sessions.update(session_id, |s| {
            s.intake_phase = "interviewing".into();
            s.interview_live_attached = true;
            let run_id = s.ensure_socratic_run_id();
            s.checkpoint = Some(crate::session::InterviewCheckpoint::new(run_id));
            s.has_checkpoint = true;
            s.add_message("user", &initial_description);
        });

        InterviewStartMode::Fresh {
            initial_description,
        }
    };

    let run_id = state
        .sessions
        .get(session_id)
        .and_then(|s| s.socratic_run_id)
        .unwrap_or_else(Uuid::new_v4);

    // Build the IO bridge and spawn the engine.
    // Note: we pass a *clone* of event_tx to the IO bridge and drop the
    // original immediately so the channel closes when the engine task
    // finishes (its Arc<WsSocraticIO> drops), allowing event_rx.recv()
    // to return None and unblock the I/O loop.

    // Create the observability event sink and its receiver.
    let (event_sink, mut planner_event_rx) = planner_core::observability::ChannelEventSink::new();
    let event_sink: Arc<dyn planner_core::observability::EventSink> = Arc::new(event_sink);

    let io = Arc::new(WsSocraticIO::new(
        event_tx.clone(),
        input_rx,
        Some(event_sink.clone()),
        session_id,
    ));
    drop(event_tx); // keep channel alive only through io's clone

    let router = state.llm_router.clone();
    let requires_immediate_llm = match &start_mode {
        InterviewStartMode::Fresh { .. } => true,
        InterviewStartMode::CheckpointResume { resume_state } => {
            resume_state.pending_prompt.is_none()
        }
    };

    // Pre-flight check: warn if no LLM providers are available.
    let available = router.available_providers();
    if requires_immediate_llm && available.is_empty() {
        tracing::error!(
            "Session {}: No LLM CLI providers found. Install and authenticate at least one of: claude, gemini, codex",
            session_id
        );
        let err_msg = "No LLM providers available. The planner service user needs at least one of the following CLI tools installed in /opt/planner/bin/ and authenticated: `claude` (Anthropic), `gemini` (Google), or `codex` (OpenAI). Run 'sudo ./deploy/install.sh --update' to install them.";

        // Send error directly on the socket (the I/O loop hasn't started yet).
        let error_msg = ServerMessage::Error {
            message: err_msg.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&error_msg) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        state.sessions.update(session_id, |s| {
            s.intake_phase = "error".into();
            s.interview_live_attached = false;
            s.add_message("system", err_msg);
        });
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

    // Emit the session-start lifecycle event.
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
    let start_mode_for_engine = start_mode;

    // Spawn the interview engine as a background task.
    let sink = event_sink.clone();
    let engine_handle = tokio::spawn(async move {
        let result = match start_mode_for_engine {
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

                // Persist belief state to the server session.
                state_for_engine.sessions.update(session_id, |s| {
                    s.belief_state = Some(session.belief_state.clone());
                    s.classification = session.belief_state.classification.clone();
                    let checkpoint = s.ensure_checkpoint();
                    checkpoint.belief_state = Some(session.belief_state.clone());
                    checkpoint.classification = session.belief_state.classification.clone();
                    checkpoint.contradictions = session.belief_state.contradictions.clone();
                    if did_converge {
                        checkpoint.current_question = None;
                        checkpoint.pending_draft = None;
                    }
                    checkpoint.touch();
                    if did_converge {
                        s.intake_phase = "pipeline_running".into();
                        s.interview_live_attached = false;
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

                    // Build the description from the completed belief state.
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

                // Send the error to the client so the UI doesn't hang.
                io.send(ServerMessage::Error {
                    message: err_msg.clone(),
                });
                io.send_message(&format!("Error: {}", err_msg)).await;

                // Mark session as errored.
                state_for_engine.sessions.update(session_id, |s| {
                    s.intake_phase = "error".into();
                    s.interview_live_attached = false;
                    s.add_message("system", &err_msg);
                });

                None
            }
        }
    });

    // Drive the WebSocket I/O loop while the engine runs.

    loop {
        tokio::select! {
            // Forward engine events to the client.
            msg = event_rx.recv() => {
                match msg {
                    Some(server_msg) => {
                        apply_checkpoint_from_server_message(&state, session_id, &server_msg);
                        if let Ok(json) = serde_json::to_string(&server_msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                mark_interview_detached_if_active(&state, session_id);
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

            // Forward observability events to the client and record in session.
            planner_evt = planner_event_rx.recv() => {
                if let Some(evt) = planner_evt {
                    // Record in session.
                    state.sessions.update(session_id, |s| {
                        s.record_event(evt.clone());
                    });
                    // Persist to disk if event store is available.
                    if let Some(ref store) = state.event_store {
                        if let Some(session) = state.sessions.get(session_id) {
                            if let Err(e) = store.save_session_events(session_id, &session.events) {
                                tracing::warn!("Failed to persist events for session {}: {}", session_id, e);
                            }
                        }
                    }
                    // Forward to WebSocket client.
                    let ws_msg = ServerMessage::PlannerEvent {
                        id: evt.id.to_string(),
                        timestamp: evt.timestamp.to_rfc3339(),
                        level: format!("{}", evt.level),
                        source: format!("{}", evt.source),
                        step: evt.step.clone(),
                        message: evt.message.clone(),
                        duration_ms: evt.duration_ms,
                        metadata: evt.metadata.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&ws_msg) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            mark_interview_detached_if_active(&state, session_id);
                            return;
                        }
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

                                // Immediately acknowledge the reaction so the frontend can
                                // persist the confirmed state without waiting for a full
                                // round-trip through the engine.
                                let ack = ServerMessage::DraftReactionAck {
                                    target: target.clone(),
                                    action: action.clone(),
                                };
                                if let Ok(json) = serde_json::to_string(&ack) {
                                    if socket.send(Message::Text(json.into())).await.is_err() {
                                        mark_interview_detached_if_active(&state, session_id);
                                        return; // client disconnected
                                    }
                                }
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
                        mark_interview_detached_if_active(&state, session_id);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    // Wait for the engine task to finish, then start the pipeline if converged.
    let description = engine_handle.await.ok().flatten();

    if description.is_some() {
        let description = description.unwrap_or_else(|| {
            // Fall back to raw belief state summary if engine produced nothing.
            state
                .sessions
                .get(session_id)
                .and_then(|s| s.project_description.clone())
                .unwrap_or_else(|| "Project requirements gathered via Socratic interview".into())
        });

        // Mark pipeline as running.
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

            let state_clone = state.clone();
            let desc = description.clone();
            tokio::spawn(async move {
                crate::api::run_pipeline_for_session(state_clone, session_id, desc).await;
            });
        }

        // Continue in pipeline-poll mode (same behaviour as handle_ws).
        let mut last_msg_count = state
            .sessions
            .get(session_id)
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

                // Forward observability events to the client and record in session.
                planner_evt = planner_event_rx.recv() => {
                    if let Some(evt) = planner_evt {
                        // Record in session.
                        state.sessions.update(session_id, |s| {
                            s.record_event(evt.clone());
                        });
                        // Persist to disk if event store is available.
                        if let Some(ref store) = state.event_store {
                            if let Some(session) = state.sessions.get(session_id) {
                                if let Err(e) = store.save_session_events(session_id, &session.events) {
                                    tracing::warn!("Failed to persist events for session {}: {}", session_id, e);
                                }
                            }
                        }
                        // Forward to WebSocket client.
                        let ws_msg = ServerMessage::PlannerEvent {
                            id: evt.id.to_string(),
                            timestamp: evt.timestamp.to_rfc3339(),
                            level: format!("{}", evt.level),
                            source: format!("{}", evt.source),
                            step: evt.step.clone(),
                            message: evt.message.clone(),
                            duration_ms: evt.duration_ms,
                            metadata: evt.metadata.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                return;
                            }
                        }
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
    use planner_schemas::{ComplexityTier, Dimension, DraftSection, ProjectType, QuickOption};

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            auth_config: None,
            event_store: None,
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            started_at: std::time::Instant::now(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
        })
    }

    #[tokio::test]
    async fn ws_socratic_io_send_classification() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(
            event_tx,
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
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (_input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(
            event_tx,
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
    async fn ws_socratic_io_receive_input() {
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(
            event_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

        input_tx.send("hello world".into()).unwrap();

        use planner_core::pipeline::steps::socratic::SocraticIO;
        let received = io.receive_input().await;
        assert_eq!(received, Some("hello world".into()));
    }

    #[tokio::test]
    async fn ws_socratic_io_receive_input_returns_none_when_closed() {
        let (event_tx, _event_rx) = mpsc::unbounded_channel::<ServerMessage>();
        let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
        let io = WsSocraticIO::new(
            event_tx,
            Arc::new(Mutex::new(input_rx)),
            None,
            Uuid::new_v4(),
        );

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
        let io = WsSocraticIO::new(
            event_tx,
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

        // Second message should be the generic ChatMessage with role "event"
        let msg2 = event_rx.try_recv().unwrap();
        match msg2 {
            ServerMessage::ChatMessage { role, .. } => {
                assert_eq!(role, "event");
            }
            other => panic!("expected ChatMessage with event role, got {:?}", other),
        }
    }

    #[test]
    fn checkpoint_updates_on_question_event() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let session_id = session.id;

        let event = SocraticEvent::Question {
            output: QuestionOutput {
                question: "Who will use this tool most often?".into(),
                target_dimension: Dimension::Stakeholders,
                quick_options: vec![QuickOption {
                    label: "Internal team".into(),
                    value: "internal_team".into(),
                }],
                allow_skip: true,
            },
        };

        apply_checkpoint_from_event(&state, session_id, &event);

        let after = state
            .sessions
            .get(session_id)
            .expect("session should exist");
        let checkpoint = after.checkpoint.expect("checkpoint should be present");
        assert_eq!(
            checkpoint
                .current_question
                .as_ref()
                .map(|q| q.question.as_str()),
            Some("Who will use this tool most often?")
        );
        assert!(after.has_checkpoint);
    }

    #[test]
    fn checkpoint_updates_on_draft_event() {
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

        let draft = SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a resilient task tracker".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: vec![Dimension::Integrations],
        };
        apply_checkpoint_from_event(
            &state,
            session_id,
            &SocraticEvent::SpeculativeDraftReady {
                draft: draft.clone(),
            },
        );

        let after = state
            .sessions
            .get(session_id)
            .expect("session should exist");
        let checkpoint = after.checkpoint.expect("checkpoint should be present");
        assert_eq!(
            checkpoint
                .pending_draft
                .as_ref()
                .and_then(|d| d.sections.first())
                .map(|s| s.heading.as_str()),
            Some("Goal")
        );
        assert_eq!(checkpoint.draft_shown_at_turn, Some(3));
    }
}
