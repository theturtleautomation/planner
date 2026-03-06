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
use super::question_planner;
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

    /// Send a question with optional quick-select options.
    async fn send_question(&self, output: &QuestionOutput);

    /// Send a belief state update (for the right-pane display).
    async fn send_belief_state(&self, state: &RequirementsBeliefState);

    /// Send a speculative draft for review.
    async fn send_draft(&self, draft: &SpeculativeDraft);

    /// Send a convergence notification.
    async fn send_convergence(&self, result: &ConvergenceResult);

    /// Send the domain classification.
    async fn send_classification(&self, classification: &DomainClassification);

    /// Receive user input. Returns the user's text response.
    /// Returns None if the user disconnected or quit.
    async fn receive_input(&self) -> Option<String>;

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
    initial_message: &str,
) -> StepResult<SocraticSession> {
    let run_id = Uuid::new_v4();

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

    // --- Phase 4: Interview Loop ---
    #[allow(unused_assignments)]
    let mut last_question: Option<String> = None;

    loop {
        // Check convergence
        let conv_result = convergence::check_convergence(
            &belief_state,
            &constitution,
            verifier_output.user_wants_to_stop,
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
            engine_state.session.belief_state = belief_state;
            return Ok(engine_state.session);
        }

        // Check if we should show a speculative draft
        let draft_already_shown = engine_state
            .draft_shown_at_turn
            .map(|t| belief_state.turn_count - t < 3)
            .unwrap_or(false);

        let last_msg_len = engine_state
            .session
            .conversation
            .last()
            .map(|t| t.content.len())
            .unwrap_or(0);

        if speculative_draft::should_trigger_draft(&belief_state, last_msg_len, draft_already_shown)
        {
            match speculative_draft::generate_draft(router, &belief_state).await {
                Ok(draft) => {
                    io.send_draft(&draft).await;
                    io.send_event(&SocraticEvent::SpeculativeDraftReady {
                        draft: draft.clone(),
                    })
                    .await;
                    engine_state.draft_shown_at_turn = Some(belief_state.turn_count);

                    // Wait for user reaction to the draft
                    if let Some(reaction) = io.receive_input().await {
                        // Process draft reaction through verifier
                        let pre_filled = belief_state.filled.len();
                        let pre_confs: Vec<f32> =
                            belief_state.uncertain.values().map(|(_, c)| *c).collect();

                        let verifier_output = belief_state::verify_and_update(
                            router,
                            &mut belief_state,
                            &reaction,
                            Some("Review the speculative draft above and correct anything that's wrong."),
                        ).await?;

                        let post_confs: Vec<f32> =
                            belief_state.uncertain.values().map(|(_, c)| *c).collect();

                        if convergence::is_stale_turn(
                            pre_filled,
                            belief_state.filled.len(),
                            &pre_confs,
                            &post_confs,
                        ) {
                            engine_state.stale_turns += 1;
                        } else {
                            engine_state.stale_turns = 0;
                        }

                        engine_state.session.conversation.push(SocraticTurn {
                            turn_number: belief_state.turn_count,
                            role: SocraticRole::User,
                            content: reaction,
                            target_dimension: None,
                            slots_updated: verifier_output
                                .filled_updates
                                .iter()
                                .filter_map(|u| belief_state::parse_dimension(&u.dimension))
                                .collect(),
                            timestamp: Utc::now().to_rfc3339(),
                        });

                        io.send_belief_state(&belief_state).await;
                        io.send_event(&SocraticEvent::BeliefStateUpdate {
                            state: belief_state.clone(),
                        })
                        .await;

                        if let Some(store) = store {
                            let _ = belief_state::persist_to_cxdb(store, run_id, &belief_state);
                        }

                        // Check convergence again after draft reaction
                        if verifier_output.user_wants_to_stop {
                            let conv_result = convergence::check_convergence(
                                &belief_state,
                                &constitution,
                                true,
                                engine_state.stale_turns,
                            );
                            io.send_convergence(&conv_result).await;
                            engine_state.session.is_complete = true;
                            engine_state.session.convergence_result = Some(conv_result);
                            engine_state.session.belief_state = belief_state;
                            return Ok(engine_state.session);
                        }

                        continue; // Back to top of loop
                    } else {
                        // User disconnected
                        engine_state.session.belief_state = belief_state;
                        return Ok(engine_state.session);
                    }
                }
                Err(e) => {
                    // Draft generation failed — not fatal, continue with questions
                    io.send_message(&format!("(Draft generation skipped: {})", e))
                        .await;
                }
            }
        }

        // Generate the next question
        let question_output = question_planner::plan_next_question(
            router,
            &belief_state,
            &constitution,
            &engine_state.session.conversation,
        )
        .await?;

        let question_output = match question_output {
            Some(q) => q,
            None => {
                // No more questions — converge
                let conv_result = ConvergenceResult {
                    is_done: true,
                    reason: StoppingReason::CompletenessGate,
                    convergence_pct: belief_state.convergence_pct(),
                };
                io.send_convergence(&conv_result).await;
                engine_state.session.is_complete = true;
                engine_state.session.convergence_result = Some(conv_result);
                engine_state.session.belief_state = belief_state;
                return Ok(engine_state.session);
            }
        };

        // Send the question
        io.send_question(&question_output).await;
        io.send_event(&SocraticEvent::Question {
            output: question_output.clone(),
        })
        .await;

        last_question = Some(question_output.question.clone());

        // Record interviewer turn
        engine_state.session.conversation.push(SocraticTurn {
            turn_number: belief_state.turn_count + 1,
            role: SocraticRole::Interviewer,
            content: question_output.question.clone(),
            target_dimension: Some(question_output.target_dimension.clone()),
            slots_updated: vec![],
            timestamp: Utc::now().to_rfc3339(),
        });

        // Wait for user response
        let user_response = match io.receive_input().await {
            Some(r) => r,
            None => {
                // User disconnected
                engine_state.session.belief_state = belief_state;
                return Ok(engine_state.session);
            }
        };

        // Check for skip signal
        let trimmed = user_response.trim().to_lowercase();
        if trimmed == "skip" || trimmed == "next" || trimmed == "pass" {
            engine_state.session.conversation.push(SocraticTurn {
                turn_number: belief_state.turn_count + 1,
                role: SocraticRole::User,
                content: user_response.clone(),
                target_dimension: None,
                slots_updated: vec![],
                timestamp: Utc::now().to_rfc3339(),
            });
            belief_state.turn_count += 1;
            engine_state.stale_turns += 1;
            continue;
        }

        // Process user response through verifier
        let pre_filled = belief_state.filled.len();
        let pre_confs: Vec<f32> = belief_state.uncertain.values().map(|(_, c)| *c).collect();

        let verifier_output = belief_state::verify_and_update(
            router,
            &mut belief_state,
            &user_response,
            last_question.as_deref(),
        )
        .await?;

        let post_confs: Vec<f32> = belief_state.uncertain.values().map(|(_, c)| *c).collect();

        // Track staleness
        if convergence::is_stale_turn(
            pre_filled,
            belief_state.filled.len(),
            &pre_confs,
            &post_confs,
        ) {
            engine_state.stale_turns += 1;
        } else {
            engine_state.stale_turns = 0;
        }

        // Record user turn
        engine_state.session.conversation.push(SocraticTurn {
            turn_number: belief_state.turn_count,
            role: SocraticRole::User,
            content: user_response,
            target_dimension: Some(question_output.target_dimension),
            slots_updated: verifier_output
                .filled_updates
                .iter()
                .filter_map(|u| belief_state::parse_dimension(&u.dimension))
                .collect(),
            timestamp: Utc::now().to_rfc3339(),
        });

        // Send updated belief state
        io.send_belief_state(&belief_state).await;
        io.send_event(&SocraticEvent::BeliefStateUpdate {
            state: belief_state.clone(),
        })
        .await;

        // Send contradiction alerts
        for contradiction in &belief_state.contradictions {
            if !contradiction.resolved {
                io.send_event(&SocraticEvent::ContradictionDetected {
                    contradiction: contradiction.clone(),
                })
                .await;
            }
        }

        // Persist
        if let Some(store) = store {
            let _ = belief_state::persist_to_cxdb(store, run_id, &belief_state);
        }

        // Check if user wants to stop (detected by verifier)
        if verifier_output.user_wants_to_stop {
            let conv_result = convergence::check_convergence(
                &belief_state,
                &constitution,
                true,
                engine_state.stale_turns,
            );
            io.send_convergence(&conv_result).await;
            engine_state.session.is_complete = true;
            engine_state.session.convergence_result = Some(conv_result);
            engine_state.session.belief_state = belief_state;
            return Ok(engine_state.session);
        }
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
    use super::*;
    use std::collections::HashMap;

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
}
