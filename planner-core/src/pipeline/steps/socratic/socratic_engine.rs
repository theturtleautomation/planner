//! # Socratic Engine — Main Turn Loop Orchestrator
//!
//! Implements the data flow diagram from the architecture synthesis:
//!
//! ```text
//! User message arrives
//!     │
//!     ├──► [Verifier] Update RequirementsBeliefState
//!     ├──► [Convergence] Check stopping criteria
//!     ├──► [Speculative Draft] Check trigger conditions
//!     ├──► [Question Planner] Score → Generate → Self-Critique
//!     └──► Send question (or final spec) to user
//! ```
//!
//! The engine is IO-agnostic: it communicates via an `SocraticIO` trait
//! that both the TUI and WebSocket server implement.

use chrono::Utc;
use uuid::Uuid;

use planner_schemas::*;

use crate::cxdb::TurnStore;
use crate::llm::providers::LlmRouter;

use super::super::StepResult;
use super::belief_state;
use super::constitution;
use super::convergence;
use super::domain_classifier;
use super::prompt_batch_planner;
use super::prompt_protocol;
use super::prompt_response_adjudicator;
use super::speculative_draft;

// ---------------------------------------------------------------------------
// IO Trait — the boundary between engine and presentation
// ---------------------------------------------------------------------------

/// IO interface for the Socratic engine.
///
/// Both TUI and WebSocket implement this trait. The engine calls these
/// methods to communicate with the user; the presentation layer decides
/// how to render them.
#[async_trait::async_trait]
pub trait SocraticIO: Send + Sync {
    /// Send a system message (informational, not a question).
    async fn send_message(&self, content: &str);

    /// Send a prompt envelope.
    async fn send_prompt(&self, prompt: &PromptEnvelope);

    /// Send a belief state update (for the right-pane display).
    async fn send_belief_state(&self, state: &RequirementsBeliefState);

    /// Send a convergence notification.
    async fn send_convergence(&self, result: &ConvergenceResult);

    /// Send the domain classification.
    async fn send_classification(&self, classification: &DomainClassification);

    /// Receive a structured prompt response.
    /// Returns None if the user disconnected or quit.
    async fn receive_prompt_response(&self, prompt: &PromptEnvelope) -> Option<PromptResponse>;

    /// Current UI capabilities used for prompt batch planning.
    fn current_ui_capabilities(&self) -> UiCapabilities {
        UiCapabilities {
            viewport_class: ViewportClass::Desktop,
            max_visible_items: 3,
            supports_split_draft_view: true,
        }
    }

    /// Send an event (for structured consumers like WebSocket).
    async fn send_event(&self, event: &SocraticEvent);
}

// ---------------------------------------------------------------------------
// Engine State
// ---------------------------------------------------------------------------

/// Internal state of the Socratic engine during an interview.
pub struct SocraticEngineState {
    pub session: SocraticSession,
    pub stale_turns: u32,
    pub draft_shown_at_turn: Option<u32>,
}

/// Pending prompt restored from a durable checkpoint.
#[derive(Debug, Clone)]
pub struct ResumePendingPrompt {
    pub prompt: PromptEnvelope,
}

/// Input state required to resume an interview from a saved checkpoint.
#[derive(Debug, Clone)]
pub struct CheckpointResumeState {
    pub belief_state: RequirementsBeliefState,
    pub classification: Option<DomainClassification>,
    pub stale_turns: u32,
    pub draft_shown_at_turn: Option<u32>,
    pub pending_prompt: Option<ResumePendingPrompt>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the full Socratic interview loop.
///
/// This is the main entry point. It:
/// 1. Takes the first user message
/// 2. Classifies the domain
/// 3. Runs the interview loop until convergence
/// 4. Returns the final belief state (ready for IntakeV1 synthesis)
///
/// The `io` parameter abstracts the presentation layer — TUI or WebSocket.
pub async fn run_interview<IO: SocraticIO, S: TurnStore>(
    router: &LlmRouter,
    io: &IO,
    store: Option<&S>,
    run_id: Uuid,
    initial_message: &str,
) -> StepResult<SocraticSession> {
    // --- Phase 1: Domain Classification ---
    io.send_message("Analyzing your project description...")
        .await;

    let classification = domain_classifier::classify_domain(router, initial_message).await?;

    io.send_classification(&classification).await;
    io.send_event(&SocraticEvent::Classified {
        classification: classification.clone(),
    })
    .await;

    io.send_message(&format!(
        "Classified as: {} ({}). Let's dig into your requirements.",
        classification.project_type,
        match classification.complexity {
            ComplexityTier::Light => "simple",
            ComplexityTier::Standard => "standard",
            ComplexityTier::Deep => "complex",
        },
    ))
    .await;

    // --- Phase 2: Initialize State ---
    let mut belief_state = RequirementsBeliefState::from_classification(&classification);
    let constitution = constitution::load_constitution();

    let mut engine_state = SocraticEngineState {
        session: SocraticSession {
            belief_state: belief_state.clone(),
            conversation: Vec::new(),
            constitution: constitution.clone(),
            is_complete: false,
            convergence_result: None,
        },
        stale_turns: 0,
        draft_shown_at_turn: None,
    };

    // --- Phase 3: Process initial message through verifier ---
    let verifier_output =
        belief_state::verify_and_update(router, &mut belief_state, initial_message, None).await?;

    // Record the initial exchange
    engine_state.session.conversation.push(SocraticTurn {
        turn_number: 1,
        role: SocraticRole::User,
        content: initial_message.to_string(),
        target_dimension: None,
        slots_updated: verifier_output
            .filled_updates
            .iter()
            .filter_map(|u| belief_state::parse_dimension(&u.dimension))
            .collect(),
        timestamp: Utc::now().to_rfc3339(),
    });

    // Send initial belief state
    io.send_belief_state(&belief_state).await;
    io.send_event(&SocraticEvent::BeliefStateUpdate {
        state: belief_state.clone(),
    })
    .await;

    // Persist
    if let Some(store) = store {
        let _ = belief_state::persist_to_cxdb(store, run_id, &belief_state);
    }

    if verifier_output.user_wants_to_stop {
        let conv_result = convergence::check_convergence(
            &belief_state,
            &constitution,
            true,
            engine_state.stale_turns,
        );
        io.send_convergence(&conv_result).await;
        io.send_event(&SocraticEvent::Converged {
            result: conv_result.clone(),
        })
        .await;
        engine_state.session.is_complete = true;
        engine_state.session.convergence_result = Some(conv_result);
        engine_state.session.belief_state = belief_state;
        return Ok(engine_state.session);
    }

    run_prompt_loop(
        router,
        io,
        store,
        run_id,
        &mut belief_state,
        &constitution,
        &mut engine_state,
        None,
    )
    .await
}

/// Resume an in-progress Socratic interview from a persisted checkpoint.
///
/// Unlike `run_interview`, this path does not re-classify or re-process an
/// initial description. Instead, it restores a prior belief-state snapshot,
/// re-emits any pending prompt, and continues the normal interview loop.
pub async fn run_interview_from_checkpoint<IO: SocraticIO, S: TurnStore>(
    router: &LlmRouter,
    io: &IO,
    store: Option<&S>,
    run_id: Uuid,
    resume_state: CheckpointResumeState,
) -> StepResult<SocraticSession> {
    let mut belief_state = resume_state.belief_state;
    if belief_state.classification.is_none() {
        belief_state.classification = resume_state.classification.clone();
    }

    let constitution = constitution::load_constitution();

    let mut engine_state = SocraticEngineState {
        session: SocraticSession {
            belief_state: belief_state.clone(),
            conversation: Vec::new(),
            constitution: constitution.clone(),
            is_complete: false,
            convergence_result: None,
        },
        stale_turns: resume_state.stale_turns,
        draft_shown_at_turn: resume_state.draft_shown_at_turn,
    };

    io.send_message("Resuming interview from saved checkpoint...")
        .await;

    if let Some(classification) = belief_state.classification.as_ref() {
        io.send_classification(classification).await;
        io.send_event(&SocraticEvent::Classified {
            classification: classification.clone(),
        })
        .await;
    }

    io.send_belief_state(&belief_state).await;
    io.send_event(&SocraticEvent::BeliefStateUpdate {
        state: belief_state.clone(),
    })
    .await;

    if let Some(store) = store {
        let _ = belief_state::persist_to_cxdb(store, run_id, &belief_state);
    }

    let pending_prompt = resume_state.pending_prompt.map(|pending| pending.prompt);
    if pending_prompt.is_none() {
        io.send_message("Checkpoint restored. Regenerating the next prompt...")
            .await;
    }

    run_prompt_loop(
        router,
        io,
        store,
        run_id,
        &mut belief_state,
        &constitution,
        &mut engine_state,
        pending_prompt,
    )
    .await
}

async fn run_prompt_loop<IO: SocraticIO, S: TurnStore>(
    router: &LlmRouter,
    io: &IO,
    store: Option<&S>,
    run_id: Uuid,
    belief_state: &mut RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    engine_state: &mut SocraticEngineState,
    mut pending_prompt: Option<PromptEnvelope>,
) -> StepResult<SocraticSession> {
    if let Some(prompt) = pending_prompt.as_ref() {
        emit_prompt(io, engine_state, belief_state, prompt).await;
    }

    loop {
        if pending_prompt.is_none() {
            let conv_result = convergence::check_convergence(
                belief_state,
                constitution,
                false,
                engine_state.stale_turns,
            );
            if conv_result.is_done {
                io.send_convergence(&conv_result).await;
                io.send_event(&SocraticEvent::Converged {
                    result: conv_result.clone(),
                })
                .await;
                engine_state.session.is_complete = true;
                engine_state.session.convergence_result = Some(conv_result);
                engine_state.session.belief_state = belief_state.clone();
                return Ok(engine_state.session.clone());
            }

            let draft_already_shown = engine_state
                .draft_shown_at_turn
                .map(|turn| belief_state.turn_count.saturating_sub(turn) < 3)
                .unwrap_or(false);
            let last_msg_len = engine_state
                .session
                .conversation
                .last()
                .map(|turn| turn.content.len())
                .unwrap_or(0);

            let mut draft_for_planner: Option<SpeculativeDraft> = None;
            if speculative_draft::should_trigger_draft(
                belief_state,
                last_msg_len,
                draft_already_shown,
            ) {
                match speculative_draft::generate_draft(router, belief_state).await {
                    Ok(draft) => {
                        draft_for_planner = Some(draft);
                    }
                    Err(error) => {
                        io.send_message(&format!("(Draft generation skipped: {})", error))
                            .await;
                    }
                }
            }

            if pending_prompt.is_none() {
                let ui_capabilities = io.current_ui_capabilities();
                pending_prompt = prompt_batch_planner::plan_prompt_batch(
                    router,
                    belief_state,
                    constitution,
                    &engine_state.session.conversation,
                    ui_capabilities.max_visible_items,
                    draft_for_planner.as_ref(),
                )
                .await?;
                if pending_prompt
                    .as_ref()
                    .and_then(|prompt| prompt.draft_snapshot.as_ref())
                    .is_some()
                {
                    engine_state.draft_shown_at_turn = Some(belief_state.turn_count);
                }
            }

            let Some(prompt) = pending_prompt.as_ref() else {
                let conv_result = ConvergenceResult {
                    is_done: true,
                    reason: StoppingReason::CompletenessGate,
                    convergence_pct: belief_state.convergence_pct(),
                };
                io.send_convergence(&conv_result).await;
                io.send_event(&SocraticEvent::Converged {
                    result: conv_result.clone(),
                })
                .await;
                engine_state.session.is_complete = true;
                engine_state.session.convergence_result = Some(conv_result);
                engine_state.session.belief_state = belief_state.clone();
                return Ok(engine_state.session.clone());
            };

            emit_prompt(io, engine_state, belief_state, prompt).await;
        }

        let active_prompt = pending_prompt
            .clone()
            .expect("pending prompt should be present before waiting for response");
        let response = match io.receive_prompt_response(&active_prompt).await {
            Some(response) => response,
            None => {
                engine_state.session.belief_state = belief_state.clone();
                return Ok(engine_state.session.clone());
            }
        };

        let answered_items = prompt_protocol::ordered_answered_items(&active_prompt, &response);
        if answered_items.is_empty() {
            engine_state.stale_turns = engine_state.stale_turns.saturating_add(1);
            pending_prompt = None;
            continue;
        }

        let pre_filled = belief_state.filled.len();
        let pre_confs: Vec<f32> = belief_state.uncertain.values().map(|(_, c)| *c).collect();
        let adjudication = prompt_response_adjudicator::adjudicate_prompt_response(
            router,
            belief_state,
            &active_prompt,
            &response,
        )
        .await?;
        let user_wants_to_stop = adjudication.user_wants_to_stop;

        for applied_answer in adjudication.applied_answers {
            engine_state.session.conversation.push(SocraticTurn {
                turn_number: applied_answer.turn_number,
                role: SocraticRole::User,
                content: applied_answer.content,
                target_dimension: applied_answer.target_dimension,
                slots_updated: applied_answer.slots_updated,
                timestamp: Utc::now().to_rfc3339(),
            });
        }

        let post_confs: Vec<f32> = belief_state.uncertain.values().map(|(_, c)| *c).collect();
        if convergence::is_stale_turn(
            pre_filled,
            belief_state.filled.len(),
            &pre_confs,
            &post_confs,
        ) {
            engine_state.stale_turns = engine_state.stale_turns.saturating_add(1);
        } else {
            engine_state.stale_turns = 0;
        }

        io.send_belief_state(belief_state).await;
        io.send_event(&SocraticEvent::BeliefStateUpdate {
            state: belief_state.clone(),
        })
        .await;
        for contradiction in &belief_state.contradictions {
            if !contradiction.resolved {
                io.send_event(&SocraticEvent::ContradictionDetected {
                    contradiction: contradiction.clone(),
                })
                .await;
            }
        }

        if let Some(store) = store {
            let _ = belief_state::persist_to_cxdb(store, run_id, belief_state);
        }

        if user_wants_to_stop {
            let conv_result = convergence::check_convergence(
                belief_state,
                constitution,
                true,
                engine_state.stale_turns,
            );
            io.send_convergence(&conv_result).await;
            io.send_event(&SocraticEvent::Converged {
                result: conv_result.clone(),
            })
            .await;
            engine_state.session.is_complete = true;
            engine_state.session.convergence_result = Some(conv_result);
            engine_state.session.belief_state = belief_state.clone();
            return Ok(engine_state.session.clone());
        }

        pending_prompt = None;
    }
}

async fn emit_prompt<IO: SocraticIO>(
    io: &IO,
    engine_state: &mut SocraticEngineState,
    belief_state: &RequirementsBeliefState,
    prompt: &PromptEnvelope,
) {
    io.send_prompt(prompt).await;
    io.send_event(&SocraticEvent::PromptGenerated {
        prompt: prompt.clone(),
    })
    .await;

    for item in &prompt.items {
        engine_state.session.conversation.push(SocraticTurn {
            turn_number: belief_state.turn_count.saturating_add(1),
            role: SocraticRole::Interviewer,
            content: item.text.clone(),
            target_dimension: item.target_dimension.clone(),
            slots_updated: Vec::new(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }
}

/// Convert a completed SocraticSession into an IntakeV1.
///
/// This bridges the Socratic engine output into the existing pipeline's
/// expected input format.
pub fn session_to_intake(session: &SocraticSession, project_id: Uuid) -> IntakeV1 {
    let bs = &session.belief_state;

    // Extract project name from goal or core features
    let project_name = bs
        .filled
        .get(&Dimension::Goal)
        .map(|v| {
            // Try to extract a short name from the goal
            let goal = &v.value;
            if goal.len() <= 40 {
                goal.clone()
            } else {
                goal.chars().take(40).collect::<String>() + "..."
            }
        })
        .unwrap_or_else(|| "Unnamed Project".to_string());

    // Generate feature slug from project name
    let feature_slug = project_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    // Build intent summary from all filled dimensions
    let mut intent_parts: Vec<String> = Vec::new();
    for (dim, val) in &bs.filled {
        intent_parts.push(format!("{}: {}", dim.label(), val.value));
    }
    let intent_summary = intent_parts.join(". ");

    // Determine output domain from classification
    let output_domain = match bs.classification.as_ref().map(|c| &c.project_type) {
        Some(ProjectType::WebApp) => OutputDomain::MicroTool {
            variant: MicroToolVariant::ReactWidget,
        },
        Some(ProjectType::ApiBackend) | Some(ProjectType::DataPipeline) => {
            OutputDomain::MicroTool {
                variant: MicroToolVariant::FastApiBackend,
            }
        }
        _ => OutputDomain::MicroTool {
            variant: MicroToolVariant::ReactWidget,
        },
    };

    // Extract environment info
    let (language, framework) = match bs.classification.as_ref().map(|c| &c.project_type) {
        Some(ProjectType::WebApp) => ("TypeScript".to_string(), "React".to_string()),
        Some(ProjectType::ApiBackend) | Some(ProjectType::DataPipeline) => {
            ("Python".to_string(), "FastAPI".to_string())
        }
        _ => {
            let tech = bs
                .filled
                .get(&Dimension::TechStack)
                .map(|v| v.value.clone());
            if let Some(ref t) = tech {
                let lower = t.to_lowercase();
                if lower.contains("python") || lower.contains("fastapi") {
                    ("Python".to_string(), "FastAPI".to_string())
                } else {
                    ("TypeScript".to_string(), "React".to_string())
                }
            } else {
                ("TypeScript".to_string(), "React".to_string())
            }
        }
    };

    let environment = EnvironmentInfo {
        language,
        framework,
        package_manager: None,
        existing_dependencies: vec![],
        build_tool: None,
    };

    // Sacred anchors from success criteria + high-priority filled dims
    let mut sacred_anchors: Vec<SacredAnchor> = Vec::new();
    if let Some(criteria) = bs.filled.get(&Dimension::SuccessCriteria) {
        sacred_anchors.push(SacredAnchor {
            id: "SA-1".into(),
            statement: criteria.value.clone(),
            rationale: Some("User-defined success criteria".into()),
        });
    }
    if let Some(security) = bs.filled.get(&Dimension::Security) {
        sacred_anchors.push(SacredAnchor {
            id: format!("SA-{}", sacred_anchors.len() + 1),
            statement: format!("Security: {}", security.value),
            rationale: Some("Security is non-negotiable".into()),
        });
    }

    // Satisfaction criteria from success criteria + goal
    let mut satisfaction_criteria_seeds: Vec<String> = Vec::new();
    if let Some(criteria) = bs.filled.get(&Dimension::SuccessCriteria) {
        satisfaction_criteria_seeds.push(criteria.value.clone());
    }
    if let Some(goal) = bs.filled.get(&Dimension::Goal) {
        satisfaction_criteria_seeds.push(format!("System achieves: {}", goal.value));
    }

    // Out of scope
    let mut out_of_scope: Vec<String> = Vec::new();
    if let Some(oos) = bs.filled.get(&Dimension::OutOfScope) {
        out_of_scope.push(oos.value.clone());
    }
    for dim in &bs.out_of_scope {
        out_of_scope.push(dim.label());
    }

    // Conversation log
    let conversation_log: Vec<ConversationTurn> = session
        .conversation
        .iter()
        .map(|t| ConversationTurn {
            role: match t.role {
                SocraticRole::User => "user".into(),
                SocraticRole::Interviewer => "system".into(),
            },
            content: t.content.clone(),
            timestamp: t.timestamp.clone(),
        })
        .collect();

    IntakeV1 {
        project_id,
        project_name,
        feature_slug,
        intent_summary,
        output_domain,
        environment,
        sacred_anchors,
        satisfaction_criteria_seeds,
        out_of_scope,
        conversation_log,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use async_trait::async_trait;

    use crate::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};

    use super::*;

    fn make_complete_session() -> SocraticSession {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        let mut belief_state = RequirementsBeliefState::from_classification(&classification);
        belief_state.fill(
            Dimension::Goal,
            SlotValue {
                value: "Task tracker for team visibility".into(),
                source_turn: 1,
                source_quote: Some("I want a task tracker".into()),
            },
        );
        belief_state.fill(
            Dimension::CoreFeatures,
            SlotValue {
                value: "Create, assign, complete tasks with Kanban board".into(),
                source_turn: 2,
                source_quote: None,
            },
        );
        belief_state.fill(
            Dimension::SuccessCriteria,
            SlotValue {
                value: "Tasks never fall through the cracks".into(),
                source_turn: 3,
                source_quote: None,
            },
        );

        SocraticSession {
            belief_state,
            conversation: vec![SocraticTurn {
                turn_number: 1,
                role: SocraticRole::User,
                content: "I want a task tracker for my team".into(),
                target_dimension: None,
                slots_updated: vec![Dimension::Goal],
                timestamp: Utc::now().to_rfc3339(),
            }],
            constitution: InterviewerConstitution::default_constitution(),
            is_complete: true,
            convergence_result: Some(ConvergenceResult {
                is_done: true,
                reason: StoppingReason::UserSignal,
                convergence_pct: 0.8,
            }),
        }
    }

    #[test]
    fn session_to_intake_basic() {
        let session = make_complete_session();
        let intake = session_to_intake(&session, Uuid::new_v4());

        assert!(intake.project_name.contains("Task tracker"));
        assert!(!intake.intent_summary.is_empty());
        assert!(!intake.sacred_anchors.is_empty());
        assert!(!intake.satisfaction_criteria_seeds.is_empty());
        assert_eq!(intake.conversation_log.len(), 1);
    }

    #[test]
    fn session_to_intake_preserves_conversation() {
        let session = make_complete_session();
        let intake = session_to_intake(&session, Uuid::new_v4());

        assert_eq!(intake.conversation_log[0].role, "user");
        assert!(intake.conversation_log[0].content.contains("task tracker"));
    }

    struct RecordingIo {
        next_response: Mutex<Option<PromptResponse>>,
        convergence_calls: AtomicUsize,
    }

    impl RecordingIo {
        fn new(response: PromptResponse) -> Self {
            Self {
                next_response: Mutex::new(Some(response)),
                convergence_calls: AtomicUsize::new(0),
            }
        }

        fn convergence_calls(&self) -> usize {
            self.convergence_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SocraticIO for RecordingIo {
        async fn send_message(&self, _content: &str) {}

        async fn send_prompt(&self, _prompt: &PromptEnvelope) {}

        async fn send_belief_state(&self, _state: &RequirementsBeliefState) {}

        async fn send_convergence(&self, _result: &ConvergenceResult) {
            self.convergence_calls.fetch_add(1, Ordering::SeqCst);
        }

        async fn send_classification(&self, _classification: &DomainClassification) {}

        async fn receive_prompt_response(
            &self,
            _prompt: &PromptEnvelope,
        ) -> Option<PromptResponse> {
            self.next_response
                .lock()
                .expect("response mutex should not be poisoned")
                .take()
        }

        async fn send_event(&self, _event: &SocraticEvent) {}
    }

    struct CountingMockClient {
        calls: Arc<AtomicUsize>,
        response_content: String,
    }

    #[async_trait]
    impl LlmClient for CountingMockClient {
        async fn complete(
            &self,
            request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionResponse {
                content: self.response_content.clone(),
                model: request.model,
                input_tokens: 10,
                output_tokens: 10,
                estimated_cost_usd: 0.0,
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn prompt_submission_runs_batch_adjudication_and_convergence_once() {
        let llm_calls = Arc::new(AtomicUsize::new(0));
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: llm_calls.clone(),
            response_content: r#"{
              "items": [
                {
                  "item_id": "item-goal",
                  "filled_updates": [{"dimension": "goal", "value": "Team planning workspace", "source_quote": null}],
                  "uncertain_updates": [],
                  "out_of_scope": [],
                  "contradictions": [],
                  "user_wants_to_stop": false
                },
                {
                  "item_id": "item-features",
                  "filled_updates": [{"dimension": "core_features", "value": "Boards, assignments, and status tracking", "source_quote": null}],
                  "uncertain_updates": [],
                  "out_of_scope": [],
                  "contradictions": [],
                  "user_wants_to_stop": false
                }
              ]
            }"#
            .into(),
        }));

        let prompt = PromptEnvelope {
            prompt_id: "prompt-test".into(),
            kind: PromptKind::QuestionBatch,
            title: "Prompt".into(),
            instructions: None,
            items: vec![
                PromptItem {
                    item_id: "item-goal".into(),
                    kind: PromptItemKind::Discovery,
                    target_dimension: Some(Dimension::Goal),
                    section_ref: None,
                    text: "What's the core goal?".into(),
                    options: vec![PromptOption {
                        option_id: "opt-goal".into(),
                        label: "Goal option".into(),
                        semantic_value: "Goal option".into(),
                        direct_effect: None,
                    }],
                    response_mode: PromptResponseMode::SingleSelectWithCustomText,
                    required: false,
                    priority: 100,
                    dependency_item_ids: Vec::new(),
                },
                PromptItem {
                    item_id: "item-features".into(),
                    kind: PromptItemKind::Discovery,
                    target_dimension: Some(Dimension::CoreFeatures),
                    section_ref: None,
                    text: "What features matter most?".into(),
                    options: vec![PromptOption {
                        option_id: "opt-features".into(),
                        label: "Feature option".into(),
                        semantic_value: "Feature option".into(),
                        direct_effect: None,
                    }],
                    response_mode: PromptResponseMode::SingleSelectWithCustomText,
                    required: false,
                    priority: 90,
                    dependency_item_ids: Vec::new(),
                },
            ],
            draft_snapshot: None,
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Cards,
                show_draft_sidebar: false,
            },
            based_on_turn: 0,
            created_at: "2026-03-08T00:00:00Z".into(),
        };

        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![
                PromptAnswer {
                    item_id: "item-goal".into(),
                    selected_option_id: Some("opt-goal".into()),
                    custom_text: Some("Need a shared planning workspace".into()),
                    skipped: false,
                },
                PromptAnswer {
                    item_id: "item-features".into(),
                    selected_option_id: Some("opt-features".into()),
                    custom_text: Some("Need boards, assignments, and status".into()),
                    skipped: false,
                },
            ],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let io = RecordingIo::new(response);

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::new(),
                uncertain: HashMap::new(),
                missing: vec![Dimension::Goal, Dimension::CoreFeatures],
                out_of_scope: Vec::new(),
                contradictions: Vec::new(),
                required_dimensions: vec![Dimension::Goal, Dimension::CoreFeatures],
                turn_count: 0,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: None,
            pending_prompt: Some(ResumePendingPrompt { prompt }),
        };

        let session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("checkpoint resume interview should succeed");

        assert!(session.is_complete);
        assert_eq!(llm_calls.load(Ordering::SeqCst), 1);
        assert_eq!(io.convergence_calls(), 1);
    }
}
