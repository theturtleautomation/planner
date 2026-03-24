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
use super::category_planner;
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

    /// Send the current category-navigation state.
    async fn send_category_state(&self, snapshot: &SocraticCategorySnapshot);

    /// Send the current live question workspace state.
    async fn send_workspace_state(&self, _workspace: &SocraticWorkspaceSnapshot) {}

    /// Send a belief state update (for the right-pane display).
    async fn send_belief_state(&self, state: &RequirementsBeliefState);

    /// Send a convergence notification.
    async fn send_convergence(&self, result: &ConvergenceResult);

    /// Send the domain classification.
    async fn send_classification(&self, classification: &DomainClassification);

    /// Receive the next user action for the interview.
    /// Returns None if the user disconnected or quit.
    async fn receive_interview_input(
        &self,
        prompt: Option<&PromptEnvelope>,
        snapshot: Option<&SocraticCategorySnapshot>,
    ) -> Option<SocraticInteractiveInput>;

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
    pub active_category_ids: Vec<String>,
    pub last_category_snapshot: Option<SocraticCategorySnapshot>,
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
    pub category_snapshot: Option<SocraticCategorySnapshot>,
}

/// Navigation and submission actions available while the Socratic interview is active.
#[derive(Debug, Clone)]
pub enum SocraticInteractiveInput {
    PromptResponse(PromptResponse),
    EnterCategory {
        category_id: String,
        revision: String,
    },
    BackToCategories,
    Done,
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
        active_category_ids: Vec::new(),
        last_category_snapshot: None,
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
        active_category_ids: resume_state
            .category_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .active_category_path
                    .iter()
                    .map(|entry| entry.category_id.clone())
                    .collect()
            })
            .unwrap_or_default(),
        last_category_snapshot: resume_state.category_snapshot,
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
            let should_show_draft = !conv_result.is_done
                && speculative_draft::should_trigger_draft(
                    belief_state,
                    last_msg_len,
                    draft_already_shown,
                );
            if should_show_draft {
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

            if let Some(draft) = draft_for_planner.as_ref() {
                let ui_capabilities = io.current_ui_capabilities();
                pending_prompt = prompt_batch_planner::plan_prompt_batch(
                    router,
                    belief_state,
                    constitution,
                    &engine_state.session.conversation,
                    ui_capabilities.max_visible_items,
                    Some(draft),
                )
                .await?;
                if pending_prompt
                    .as_ref()
                    .and_then(|prompt| prompt.draft_snapshot.as_ref())
                    .is_some()
                {
                    engine_state.draft_shown_at_turn = Some(belief_state.turn_count);
                }
                if let Some(prompt) = pending_prompt.as_ref() {
                    emit_prompt(io, engine_state, belief_state, prompt).await;
                    continue;
                }
            }

            let mut category_snapshot = category_planner::build_category_snapshot(
                belief_state,
                &engine_state.active_category_ids,
                conv_result.is_done,
                engine_state.last_category_snapshot.as_ref(),
            );
            let mut branch_notice: Option<String> = None;
            engine_state.active_category_ids = category_snapshot
                .active_category_path
                .iter()
                .map(|entry| entry.category_id.clone())
                .collect();

            if engine_state.last_category_snapshot.is_none()
                && engine_state.active_category_ids.is_empty()
            {
                if let Some(initial_path) =
                    category_planner::first_prompt_ready_category_path(&category_snapshot)
                {
                    engine_state.active_category_ids = initial_path;
                    category_snapshot = category_planner::build_category_snapshot(
                        belief_state,
                        &engine_state.active_category_ids,
                        conv_result.is_done,
                        engine_state.last_category_snapshot.as_ref(),
                    );
                    engine_state.active_category_ids = category_snapshot
                        .active_category_path
                        .iter()
                        .map(|entry| entry.category_id.clone())
                        .collect();
                }
            }

            if let Some(active_category_id) =
                category_planner::active_leaf_category_id(&category_snapshot)
            {
                let ui_capabilities = io.current_ui_capabilities();
                let prompt_path = category_snapshot.active_category_path.clone();
                let category_id = active_category_id.to_string();
                let scoped_candidates = category_planner::filter_candidates_for_active_category(
                    belief_state,
                    category_id.as_str(),
                    ui_capabilities.max_visible_items,
                );
                pending_prompt = prompt_batch_planner::plan_prompt_batch_from_candidates(
                    router,
                    belief_state,
                    constitution,
                    &engine_state.session.conversation,
                    scoped_candidates,
                    None,
                    Some(category_id.clone()),
                    prompt_path,
                )
                .await?;

                if let Some(prompt) = pending_prompt.as_ref() {
                    let workspace_snapshot = category_planner::build_workspace_snapshot(
                        belief_state,
                        &category_snapshot,
                        Some(category_id.as_str()),
                        None,
                        ui_capabilities.max_visible_items,
                    );
                    io.send_category_state(&category_snapshot).await;
                    io.send_workspace_state(&workspace_snapshot).await;
                    io.send_event(&SocraticEvent::CategoryState {
                        snapshot: category_snapshot.clone(),
                    })
                    .await;
                    engine_state.last_category_snapshot = Some(category_snapshot.clone());
                    emit_prompt(io, engine_state, belief_state, prompt).await;
                    continue;
                }

                let collapsed_title = category_snapshot
                    .active_category_path
                    .last()
                    .map(|entry| entry.title.clone())
                    .unwrap_or_else(|| String::from("Selected category"));
                engine_state.active_category_ids.pop();
                category_snapshot = category_planner::build_category_snapshot(
                    belief_state,
                    &engine_state.active_category_ids,
                    conv_result.is_done,
                    engine_state.last_category_snapshot.as_ref(),
                );
                branch_notice = Some(format!(
                    "\"{collapsed_title}\" no longer has active questions. Review the updated workspace for the remaining work."
                ));
                io.send_message(
                    branch_notice
                        .as_deref()
                        .unwrap_or("The selected category changed. Review the updated workspace."),
                )
                .await;
            }

            let workspace_snapshot = category_planner::build_workspace_snapshot(
                belief_state,
                &category_snapshot,
                engine_state.active_category_ids.last().map(String::as_str),
                branch_notice,
                io.current_ui_capabilities().max_visible_items,
            );
            io.send_category_state(&category_snapshot).await;
            io.send_workspace_state(&workspace_snapshot).await;
            io.send_event(&SocraticEvent::CategoryState {
                snapshot: category_snapshot.clone(),
            })
            .await;
            engine_state.last_category_snapshot = Some(category_snapshot.clone());

            let Some(input) = io
                .receive_interview_input(None, Some(&category_snapshot))
                .await
            else {
                return finalize_convergence(
                    io,
                    belief_state,
                    engine_state,
                    convergence::check_convergence(
                        belief_state,
                        constitution,
                        true,
                        engine_state.stale_turns,
                    ),
                )
                .await;
            };

            match input {
                SocraticInteractiveInput::EnterCategory {
                    category_id,
                    revision,
                } => {
                    if revision == category_snapshot.revision {
                        if let Some(path) = category_planner::resolve_category_path(
                            &category_snapshot,
                            &category_id,
                        ) {
                            engine_state.active_category_ids = path;
                        } else {
                            io.send_message(
                                "That category is no longer visible. Showing the latest category list.",
                            )
                            .await;
                        }
                    } else {
                        io.send_message(
                            "That category view is stale. Showing the latest category list.",
                        )
                        .await;
                    }
                }
                SocraticInteractiveInput::BackToCategories => {
                    engine_state.active_category_ids.clear();
                }
                SocraticInteractiveInput::Done => {
                    if category_snapshot.build_ready && engine_state.active_category_ids.is_empty()
                    {
                        return finalize_convergence(
                            io,
                            belief_state,
                            engine_state,
                            ConvergenceResult {
                                is_done: true,
                                reason: StoppingReason::UserSignal,
                                convergence_pct: belief_state.convergence_pct(),
                            },
                        )
                        .await;
                    }
                    io.send_message(
                        "Return to the main category screen and resolve the remaining work before starting the build.",
                    )
                    .await;
                }
                SocraticInteractiveInput::PromptResponse(_) => {
                    io.send_message("Select a category before submitting answers.")
                        .await;
                }
            }
            continue;
        }

        let active_prompt = pending_prompt
            .clone()
            .expect("pending prompt should be present before waiting for response");
        let Some(input) = io.receive_interview_input(Some(&active_prompt), None).await else {
            return finalize_convergence(
                io,
                belief_state,
                engine_state,
                convergence::check_convergence(
                    belief_state,
                    constitution,
                    true,
                    engine_state.stale_turns,
                ),
            )
            .await;
        };

        let response = match input {
            SocraticInteractiveInput::PromptResponse(response) => response,
            SocraticInteractiveInput::BackToCategories => {
                pending_prompt = None;
                engine_state.active_category_ids.clear();
                continue;
            }
            SocraticInteractiveInput::EnterCategory { category_id, .. } => {
                pending_prompt = None;
                engine_state.active_category_ids = vec![category_id];
                continue;
            }
            SocraticInteractiveInput::Done => {
                io.send_message(
                    "Done is only available from the main category screen. Refreshing the latest category list.",
                )
                .await;
                pending_prompt = None;
                engine_state.active_category_ids.clear();
                continue;
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
            return finalize_convergence(
                io,
                belief_state,
                engine_state,
                convergence::check_convergence(
                    belief_state,
                    constitution,
                    true,
                    engine_state.stale_turns,
                ),
            )
            .await;
        }

        pending_prompt = None;
    }
}

async fn finalize_convergence<IO: SocraticIO>(
    io: &IO,
    belief_state: &RequirementsBeliefState,
    engine_state: &mut SocraticEngineState,
    conv_result: ConvergenceResult,
) -> StepResult<SocraticSession> {
    io.send_convergence(&conv_result).await;
    io.send_event(&SocraticEvent::Converged {
        result: conv_result.clone(),
    })
    .await;
    engine_state.session.is_complete = true;
    engine_state.session.convergence_result = Some(conv_result);
    engine_state.session.belief_state = belief_state.clone();
    Ok(engine_state.session.clone())
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
    use std::collections::{HashMap, VecDeque};
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

        async fn send_category_state(&self, _snapshot: &SocraticCategorySnapshot) {}

        async fn send_belief_state(&self, _state: &RequirementsBeliefState) {}

        async fn send_convergence(&self, _result: &ConvergenceResult) {
            self.convergence_calls.fetch_add(1, Ordering::SeqCst);
        }

        async fn send_classification(&self, _classification: &DomainClassification) {}

        async fn receive_interview_input(
            &self,
            prompt: Option<&PromptEnvelope>,
            _snapshot: Option<&SocraticCategorySnapshot>,
        ) -> Option<SocraticInteractiveInput> {
            let response = self
                .next_response
                .lock()
                .expect("response mutex should not be poisoned")
                .take();
            match (prompt, response) {
                (_, None) => None,
                (_, Some(response)) => Some(SocraticInteractiveInput::PromptResponse(response)),
            }
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

    enum SequenceStep {
        EnterVisible(usize),
        EnterVisibleWithRevision(usize, String),
        Done,
        Disconnect,
    }

    struct SequencedIo {
        steps: Mutex<VecDeque<SequenceStep>>,
        snapshots: Mutex<Vec<SocraticCategorySnapshot>>,
        prompts: Mutex<Vec<PromptEnvelope>>,
        messages: Mutex<Vec<String>>,
    }

    impl SequencedIo {
        fn new(steps: Vec<SequenceStep>) -> Self {
            Self {
                steps: Mutex::new(VecDeque::from(steps)),
                snapshots: Mutex::new(Vec::new()),
                prompts: Mutex::new(Vec::new()),
                messages: Mutex::new(Vec::new()),
            }
        }

        fn snapshots(&self) -> Vec<SocraticCategorySnapshot> {
            self.snapshots
                .lock()
                .expect("snapshot mutex should not be poisoned")
                .clone()
        }

        fn prompts(&self) -> Vec<PromptEnvelope> {
            self.prompts
                .lock()
                .expect("prompt mutex should not be poisoned")
                .clone()
        }

        fn messages(&self) -> Vec<String> {
            self.messages
                .lock()
                .expect("message mutex should not be poisoned")
                .clone()
        }
    }

    #[async_trait]
    impl SocraticIO for SequencedIo {
        async fn send_message(&self, content: &str) {
            self.messages
                .lock()
                .expect("message mutex should not be poisoned")
                .push(content.to_string());
        }

        async fn send_prompt(&self, prompt: &PromptEnvelope) {
            self.prompts
                .lock()
                .expect("prompt mutex should not be poisoned")
                .push(prompt.clone());
        }

        async fn send_category_state(&self, snapshot: &SocraticCategorySnapshot) {
            self.snapshots
                .lock()
                .expect("snapshot mutex should not be poisoned")
                .push(snapshot.clone());
        }

        async fn send_belief_state(&self, _state: &RequirementsBeliefState) {}

        async fn send_convergence(&self, _result: &ConvergenceResult) {}

        async fn send_classification(&self, _classification: &DomainClassification) {}

        async fn receive_interview_input(
            &self,
            prompt: Option<&PromptEnvelope>,
            snapshot: Option<&SocraticCategorySnapshot>,
        ) -> Option<SocraticInteractiveInput> {
            let step = self
                .steps
                .lock()
                .expect("steps mutex should not be poisoned")
                .pop_front()?;

            match step {
                SequenceStep::EnterVisible(index) => {
                    let snapshot = snapshot.expect("category step should receive a snapshot");
                    let visible_category_ids = category_planner::visible_category_ids(snapshot);
                    let category_id = visible_category_ids
                        .get(index.saturating_sub(1))
                        .expect("visible category should exist")
                        .clone();
                    Some(SocraticInteractiveInput::EnterCategory {
                        category_id,
                        revision: snapshot.revision.clone(),
                    })
                }
                SequenceStep::EnterVisibleWithRevision(index, revision) => {
                    let snapshot = snapshot.expect("category step should receive a snapshot");
                    let visible_category_ids = category_planner::visible_category_ids(snapshot);
                    let category_id = visible_category_ids
                        .get(index.saturating_sub(1))
                        .expect("visible category should exist")
                        .clone();
                    Some(SocraticInteractiveInput::EnterCategory {
                        category_id,
                        revision,
                    })
                }
                SequenceStep::Done => {
                    let _ = prompt;
                    Some(SocraticInteractiveInput::Done)
                }
                SequenceStep::Disconnect => None,
            }
        }

        async fn send_event(&self, _event: &SocraticEvent) {}
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
            origin_category_id: None,
            category_path: Vec::new(),
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
            category_snapshot: None,
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

    #[tokio::test]
    async fn done_during_prompt_loop_refreshes_main_category_screen() {
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: Arc::new(AtomicUsize::new(0)),
            response_content: "{}".into(),
        }));

        let prompt = PromptEnvelope {
            prompt_id: "prompt-test".into(),
            kind: PromptKind::DraftReview,
            title: "Review and refine draft".into(),
            instructions: Some(
                "Review draft sections and close uncertain or missing areas.".into(),
            ),
            origin_category_id: Some("root-discovery::dimension::security::missing".into()),
            category_path: vec![
                SocraticCategoryPathEntry {
                    category_id: "root-discovery".into(),
                    title: "Explore missing areas".into(),
                },
                SocraticCategoryPathEntry {
                    category_id: "root-discovery::dimension::security".into(),
                    title: "Security".into(),
                },
                SocraticCategoryPathEntry {
                    category_id: "root-discovery::dimension::security::missing".into(),
                    title: "Authentication model".into(),
                },
            ],
            items: vec![PromptItem {
                item_id: "item-goal".into(),
                kind: PromptItemKind::DraftSection,
                target_dimension: Some(Dimension::Goal),
                section_ref: Some("Goal".into()),
                text: "Review the goal section.".into(),
                options: vec![PromptOption {
                    option_id: "confirm".into(),
                    label: "Looks correct".into(),
                    semantic_value: "confirm".into(),
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
                preferred_layout: PromptPreferredLayout::Review,
                show_draft_sidebar: true,
            },
            based_on_turn: 0,
            created_at: "2026-03-08T00:00:00Z".into(),
        };

        let io = SequencedIo::new(vec![SequenceStep::Done, SequenceStep::Disconnect]);

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::from([
                    (
                        Dimension::Goal,
                        SlotValue {
                            value: "Personal task widget".into(),
                            source_turn: 1,
                            source_quote: None,
                        },
                    ),
                    (
                        Dimension::CoreFeatures,
                        SlotValue {
                            value: "Track tasks and mark complete".into(),
                            source_turn: 1,
                            source_quote: None,
                        },
                    ),
                ]),
                uncertain: HashMap::new(),
                missing: vec![Dimension::Security],
                out_of_scope: Vec::new(),
                contradictions: Vec::new(),
                required_dimensions: vec![
                    Dimension::Goal,
                    Dimension::CoreFeatures,
                    Dimension::Security,
                ],
                turn_count: 4,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: Some(4),
            pending_prompt: Some(ResumePendingPrompt { prompt }),
            category_snapshot: None,
        };

        let _session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("checkpoint resume interview should succeed");

        let messages = io.messages();
        assert!(messages.iter().any(|message| {
            message.contains("Done is only available from the main category screen")
        }));

        let snapshots = io.snapshots();
        assert!(!snapshots.is_empty());
        assert!(snapshots
            .last()
            .is_some_and(|snapshot| snapshot.active_category_path.is_empty()));
    }

    #[tokio::test]
    async fn recursive_category_entry_emits_nested_prompt_path() {
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: Arc::new(AtomicUsize::new(0)),
            response_content: "{}".into(),
        }));

        let io = SequencedIo::new(vec![
            SequenceStep::EnterVisible(1),
            SequenceStep::EnterVisible(1),
            SequenceStep::EnterVisible(1),
            SequenceStep::EnterVisible(1),
            SequenceStep::Disconnect,
        ]);

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::new(),
                uncertain: HashMap::new(),
                missing: vec![Dimension::Goal],
                out_of_scope: Vec::new(),
                contradictions: vec![Contradiction {
                    dimension_a: Dimension::Goal,
                    value_a: "Internal planning hub".into(),
                    dimension_b: Dimension::Platform,
                    value_b: "Mobile-only native app".into(),
                    explanation: "The requested planning hub needs desktop collaboration support."
                        .into(),
                    resolved: false,
                }],
                required_dimensions: vec![Dimension::Goal, Dimension::Platform],
                turn_count: 0,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: None,
            pending_prompt: None,
            category_snapshot: None,
        };

        let _session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("recursive category interview should succeed");

        let snapshots = io.snapshots();
        assert!(snapshots
            .iter()
            .any(|snapshot| snapshot.active_category_path.len() == 3));

        let prompts = io.prompts();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].category_path.len(), 4);
        assert_eq!(
            prompts[0]
                .category_path
                .last()
                .map(|entry| entry.category_id.as_str()),
            prompts[0].origin_category_id.as_deref()
        );
    }

    #[tokio::test]
    async fn initial_workspace_auto_enters_first_prompt_ready_thread() {
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: Arc::new(AtomicUsize::new(0)),
            response_content: r#"{"question":"What platform should this start on?","quick_options":[{"label":"Web app","value":"Web application"}],"allow_skip":false}"#.into(),
        }));

        let io = SequencedIo::new(vec![SequenceStep::Disconnect]);

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::new(),
                uncertain: HashMap::from([(
                    Dimension::Platform,
                    (
                        SlotValue {
                            value: "Web application".into(),
                            source_turn: 1,
                            source_quote: None,
                        },
                        0.5,
                    ),
                )]),
                missing: vec![Dimension::SuccessCriteria],
                out_of_scope: Vec::new(),
                contradictions: Vec::new(),
                required_dimensions: vec![Dimension::Platform, Dimension::SuccessCriteria],
                turn_count: 1,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: None,
            pending_prompt: None,
            category_snapshot: None,
        };

        let _session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("initial interview should auto-enter the first prompt-ready thread");

        let prompts = io.prompts();
        assert_eq!(prompts.len(), 1);
        assert_eq!(
            prompts[0].origin_category_id.as_deref(),
            Some("category-verification-platform")
        );

        let snapshots = io.snapshots();
        assert!(!snapshots.is_empty());
        assert_eq!(
            snapshots
                .last()
                .and_then(|snapshot| snapshot.active_category_path.last())
                .map(|entry| entry.category_id.as_str()),
            Some("category-verification-platform")
        );
    }

    #[tokio::test]
    async fn checkpoint_resume_reemits_pending_prompt_with_deep_category_path() {
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: Arc::new(AtomicUsize::new(0)),
            response_content: "{}".into(),
        }));

        let io = SequencedIo::new(vec![SequenceStep::Disconnect]);
        let prompt = PromptEnvelope {
            prompt_id: "prompt-resume-deep".into(),
            kind: PromptKind::QuestionBatch,
            title: "Clarify security".into(),
            instructions: Some("Answer the scoped security question.".into()),
            origin_category_id: Some("root-discovery::dimension::security::auth".into()),
            category_path: vec![
                SocraticCategoryPathEntry {
                    category_id: "root-discovery".into(),
                    title: "Explore missing areas".into(),
                },
                SocraticCategoryPathEntry {
                    category_id: "root-discovery::dimension::security".into(),
                    title: "Security".into(),
                },
                SocraticCategoryPathEntry {
                    category_id: "root-discovery::dimension::security::auth".into(),
                    title: "Authentication model".into(),
                },
            ],
            items: vec![PromptItem {
                item_id: "item-security".into(),
                kind: PromptItemKind::Discovery,
                target_dimension: Some(Dimension::Security),
                section_ref: None,
                text: "How should authentication work?".into(),
                options: Vec::new(),
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
            based_on_turn: 2,
            created_at: "2026-03-08T00:00:00Z".into(),
        };

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::new(),
                uncertain: HashMap::new(),
                missing: vec![Dimension::Security],
                out_of_scope: Vec::new(),
                contradictions: Vec::new(),
                required_dimensions: vec![Dimension::Security],
                turn_count: 2,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: None,
            pending_prompt: Some(ResumePendingPrompt {
                prompt: prompt.clone(),
            }),
            category_snapshot: None,
        };

        let _session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("checkpoint resume interview should succeed");

        let prompts = io.prompts();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].prompt_id, prompt.prompt_id);
        assert_eq!(prompts[0].category_path, prompt.category_path);
        assert_eq!(prompts[0].origin_category_id, prompt.origin_category_id);
    }

    #[tokio::test]
    async fn stale_category_revision_replays_latest_snapshot() {
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: Arc::new(AtomicUsize::new(0)),
            response_content: "{}".into(),
        }));

        let io = SequencedIo::new(vec![
            SequenceStep::EnterVisibleWithRevision(1, "stale-revision".into()),
            SequenceStep::Disconnect,
        ]);

        let resume_state = CheckpointResumeState {
            belief_state: RequirementsBeliefState {
                filled: HashMap::new(),
                uncertain: HashMap::new(),
                missing: vec![Dimension::Goal],
                out_of_scope: Vec::new(),
                contradictions: Vec::new(),
                required_dimensions: vec![Dimension::Goal],
                turn_count: 0,
                classification: None,
            },
            classification: None,
            stale_turns: 0,
            draft_shown_at_turn: None,
            pending_prompt: None,
            category_snapshot: None,
        };

        let _session = run_interview_from_checkpoint::<_, crate::cxdb::CxdbEngine>(
            &router,
            &io,
            None::<&crate::cxdb::CxdbEngine>,
            Uuid::new_v4(),
            resume_state,
        )
        .await
        .expect("checkpoint resume interview should succeed");

        let messages = io.messages();
        assert!(messages
            .iter()
            .any(|message| message.contains("That category view is stale")));

        let snapshots = io.snapshots();
        assert!(snapshots.len() >= 2);
        assert_eq!(snapshots[0].revision, snapshots[1].revision);
        assert_eq!(snapshots[0].active_category_path, snapshots[1].active_category_path);
    }
}
