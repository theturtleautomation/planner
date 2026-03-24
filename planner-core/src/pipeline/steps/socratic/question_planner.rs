//! # Question Planner — Dimension Scoring + Question Generation
//!
//! Selects the next question to maximize information gain while
//! respecting the question budget and UX cost.
//!
//! Two-level selection:
//! 1. Strategy: which dimension to target (verify uncertainty before expanding scope)
//! 2. Generation: how to ask about the chosen dimension (LLM call)

use planner_schemas::*;
use std::time::Instant;

use super::super::{StepError, StepResult};
use super::belief_state::format_belief_state_for_llm;
use super::constitution::{evaluate_question, ConstitutionViolation};
use crate::llm::providers::LlmRouter;
use crate::llm::{CompletionRequest, DefaultModels, Message, Role};

// ---------------------------------------------------------------------------
// System Prompt
// ---------------------------------------------------------------------------

const QUESTION_GEN_SYSTEM_PROMPT: &str = r#"You are an expert requirements interviewer conducting a Socratic elicitation session.

Given:
1. The current belief state (what's known, uncertain, missing)
2. The target dimension you should ask about
3. The interviewer constitution (rules you must follow)
4. The conversation history so far

Generate ONE focused question about the target dimension.

Respond with ONLY a JSON object (no markdown fences):
{
  "question": "The natural-language question to ask the user",
  "quick_options": [
    {"label": "Option A", "value": "Detailed meaning of option A"},
    {"label": "Option B", "value": "Detailed meaning of option B"}
  ],
  "allow_skip": true
}

## Rules:
- Ask exactly ONE question. Never compound questions.
- Use clear, jargon-free language unless the user has demonstrated expertise.
- Provide 4-7 quick-select options whenever the answer can be scaffolded.
- Treat quick-select options as a baseline shortlist the user can choose from inline before sending a fuller answer.
- Include a "Not sure yet" option via allow_skip: true when the question is optional.
- Reference what's already known to show you've been listening.
- If the target dimension already has an uncertain candidate, ask the user to verify or correct it directly before moving on.
- Use Paul & Elder's taxonomy: clarifying → probing uncertainty → exploring implications.
- Calibrate difficulty to the user's expertise level.
- Never assume technologies the user hasn't mentioned."#;

const COMPLEX_HISTORY_TURN_THRESHOLD: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestionGenerationLane {
    Scaffold,
    FastModel,
    DeepModel,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Plan the next question: select a target dimension and generate the question.
///
/// Returns a QuestionOutput ready for display, or None if no questions remain.
pub async fn plan_next_question(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
) -> StepResult<Option<QuestionOutput>> {
    // Step 1: Select the best dimension to target
    let strategy = select_target_dimension(state);
    let strategy = match strategy {
        Some(s) => s,
        None => return Ok(None), // No more dimensions to ask about
    };

    plan_question_for_dimension(
        router,
        state,
        constitution,
        conversation_history,
        strategy.target_dimension,
        strategy.rationale.as_str(),
    )
    .await
    .map(Some)
}

/// Generate a question for a specific target dimension.
pub async fn plan_question_for_dimension(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    target_dimension: Dimension,
    rationale: &str,
) -> StepResult<QuestionOutput> {
    let strategy = QuestionStrategy {
        target_dimension: target_dimension.clone(),
        rationale: rationale.to_string(),
        score: target_dimension.priority_weight(),
    };

    let output =
        generate_question(router, state, &strategy, constitution, conversation_history).await?;

    let violations = evaluate_question(
        &output.question,
        &strategy.target_dimension,
        state,
        constitution,
    );

    if violations.is_empty() {
        return Ok(output);
    }

    regenerate_with_critique(
        router,
        state,
        &strategy,
        constitution,
        conversation_history,
        &violations,
    )
    .await
}

/// Select the best dimension to target next.
///
/// Scores each missing/uncertain dimension by:
/// - Priority weight (from Dimension::priority_weight)
/// - Information gain estimate (missing > uncertain)
/// - Dependency check (don't ask about auth before scope is set)
pub fn select_target_dimension(state: &RequirementsBeliefState) -> Option<QuestionStrategy> {
    let contradiction_candidate = state
        .contradictions
        .iter()
        .find(|contradiction| !contradiction.resolved)
        .map(|contradiction| QuestionStrategy {
            target_dimension: contradiction.dimension_a.clone(),
            rationale: format!(
                "Unresolved contradiction: {} vs {}",
                contradiction.dimension_a.label(),
                contradiction.dimension_b.label()
            ),
            score: 2.0,
        });

    if contradiction_candidate.is_some() {
        return contradiction_candidate;
    }

    let mut uncertain_candidates: Vec<(Dimension, f32, String)> = Vec::new();
    for (dim, (_val, confidence)) in &state.uncertain {
        let weight = dim.priority_weight();
        let info_gain = 1.0 - confidence; // Lower confidence → higher info gain
        let dep_penalty = dependency_penalty(dim, state);
        let score = weight * info_gain * dep_penalty;

        uncertain_candidates.push((
            dim.clone(),
            score,
            format!(
                "Needs verification first (confidence={:.0}%, weight={:.2}, info_gain={:.2})",
                confidence * 100.0,
                weight,
                info_gain
            ),
        ));
    }

    uncertain_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((dim, score, rationale)) = uncertain_candidates.first() {
        return Some(QuestionStrategy {
            target_dimension: dim.clone(),
            rationale: rationale.clone(),
            score: *score,
        });
    }

    let mut missing_candidates: Vec<(Dimension, f32, String)> = Vec::new();
    for dim in &state.missing {
        let weight = dim.priority_weight();
        let info_gain = 1.0; // Full info gain for missing dims
        let dep_penalty = dependency_penalty(dim, state);
        let score = weight * info_gain * dep_penalty;

        missing_candidates.push((
            dim.clone(),
            score,
            format!(
                "Missing dimension (weight={:.2}, info_gain={:.1}, dep={:.1})",
                weight, info_gain, dep_penalty
            ),
        ));
    }

    missing_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    missing_candidates
        .first()
        .map(|(dim, score, rationale)| QuestionStrategy {
            target_dimension: dim.clone(),
            rationale: rationale.clone(),
            score: *score,
        })
}

/// Dependency penalty: reduce score for dimensions that depend on others
/// that haven't been filled yet.
fn dependency_penalty(dim: &Dimension, state: &RequirementsBeliefState) -> f32 {
    match dim {
        // Don't ask about auth mechanism before knowing if auth is needed
        Dimension::Auth => {
            if state.filled.contains_key(&Dimension::Stakeholders)
                || state.filled.contains_key(&Dimension::CoreFeatures)
            {
                1.0
            } else {
                0.3
            }
        }
        // Don't ask about performance before knowing the features
        Dimension::Performance | Dimension::Scalability => {
            if state.filled.contains_key(&Dimension::CoreFeatures) {
                1.0
            } else {
                0.3
            }
        }
        // Don't ask about error handling before knowing the features
        Dimension::ErrorHandling => {
            if state.filled.contains_key(&Dimension::CoreFeatures) {
                1.0
            } else {
                0.4
            }
        }
        // Don't ask about data model before knowing features
        Dimension::DataModel => {
            if state.filled.contains_key(&Dimension::CoreFeatures) {
                1.0
            } else {
                0.5
            }
        }
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// LLM Question Generation
// ---------------------------------------------------------------------------

async fn generate_question(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
) -> StepResult<QuestionOutput> {
    let lane = choose_generation_lane(state, strategy, conversation_history);

    if lane == QuestionGenerationLane::Scaffold {
        if let Some(scaffolded) = scaffold_question(state, strategy, conversation_history) {
            tracing::info!(
                target: "planner.socratic.question_planner",
                lane = "scaffold",
                dimension = %strategy.target_dimension.label(),
                history_turns = conversation_history.len(),
                "Generated deterministic Socratic question scaffold"
            );
            return Ok(scaffolded);
        }

        tracing::warn!(
            target: "planner.socratic.question_planner",
            dimension = %strategy.target_dimension.label(),
            "Question lane chooser selected scaffold but no scaffold implementation was available; falling back to fast model lane"
        );
    }

    let user_prompt = build_generation_prompt(
        state,
        strategy,
        constitution,
        conversation_history,
        None,
    );

    match lane {
        QuestionGenerationLane::Scaffold | QuestionGenerationLane::FastModel => {
            let fast_attempt = generate_question_with_model(
                router,
                &user_prompt,
                &strategy.target_dimension,
                DefaultModels::INTAKE_QUESTION_FAST,
                "fast_model",
            )
            .await;

            match fast_attempt {
                Ok(output) => Ok(output),
                Err(error) => {
                    tracing::warn!(
                        target: "planner.socratic.question_planner",
                        lane = "fast_model",
                        fallback_lane = "deep_model",
                        fallback_model = DefaultModels::INTAKE_QUESTION_DEEP,
                        dimension = %strategy.target_dimension.label(),
                        error = %error,
                        "Fast question generation failed; retrying with deep lane"
                    );
                    generate_question_with_model(
                        router,
                        &user_prompt,
                        &strategy.target_dimension,
                        DefaultModels::INTAKE_QUESTION_DEEP,
                        "deep_model_fallback",
                    )
                    .await
                }
            }
        }
        QuestionGenerationLane::DeepModel => {
            generate_question_with_model(
                router,
                &user_prompt,
                &strategy.target_dimension,
                DefaultModels::INTAKE_QUESTION_DEEP,
                "deep_model",
            )
            .await
        }
    }
}

async fn regenerate_with_critique(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    violations: &[ConstitutionViolation],
) -> StepResult<QuestionOutput> {
    let critique_text: String = violations
        .iter()
        .map(|v| format!("- Rule {}: {}", v.rule_id, v.explanation))
        .collect::<Vec<_>>()
        .join("\n");

    let user_prompt = build_generation_prompt(
        state,
        strategy,
        constitution,
        conversation_history,
        Some(&critique_text),
    );

    generate_question_with_model(
        router,
        &user_prompt,
        &strategy.target_dimension,
        DefaultModels::INTAKE_QUESTION_DEEP,
        "deep_model_regeneration",
    )
    .await
}

fn format_conversation_history(history: &[SocraticTurn]) -> String {
    if history.is_empty() {
        return "(no conversation yet — this is the first turn)".to_string();
    }

    history
        .iter()
        .map(|turn| {
            let role = match turn.role {
                SocraticRole::User => "User",
                SocraticRole::Interviewer => "Interviewer",
            };
            format!("{}: {}", role, turn.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_generation_prompt(
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    critique_text: Option<&str>,
) -> String {
    let state_text = format_belief_state_for_llm(state);
    let history_text = format_conversation_history(conversation_history);
    let verification_context = state
        .uncertain
        .get(&strategy.target_dimension)
        .map(|(slot, confidence)| {
            format!(
                "## Existing Uncertain Candidate For This Dimension:\nCurrent candidate: {}\nConfidence: {:.0}%\nAsk the user to confirm, correct, or refine this candidate directly.\n\n",
                slot.value,
                confidence * 100.0
            )
        })
        .unwrap_or_default();

    match critique_text {
        Some(critique_text) => format!(
            "## Belief State:\n{}\n\n{}## Target Dimension: {} ({})\nRationale: {}\n\n## Constitution:\n{}\n\n## Conversation So Far:\n{}\n\n## SELF-CRITIQUE — Previous question violated these rules:\n{}\n\nGenerate a REVISED question that does NOT violate the above rules.",
            state_text,
            verification_context,
            strategy.target_dimension.label(),
            serde_json::to_string(&strategy.target_dimension).unwrap_or_default(),
            strategy.rationale,
            constitution.as_prompt_text(),
            history_text,
            critique_text,
        ),
        None => format!(
            "## Belief State:\n{}\n\n{}## Target Dimension: {} ({})\nRationale: {}\n\n## Constitution:\n{}\n\n## Conversation So Far:\n{}\n\nGenerate the next question.",
            state_text,
            verification_context,
            strategy.target_dimension.label(),
            serde_json::to_string(&strategy.target_dimension).unwrap_or_default(),
            strategy.rationale,
            constitution.as_prompt_text(),
            history_text,
        ),
    }
}

async fn generate_question_with_model(
    router: &LlmRouter,
    user_prompt: &str,
    target_dimension: &Dimension,
    model: &str,
    lane: &str,
) -> StepResult<QuestionOutput> {
    let request = CompletionRequest {
        system: Some(QUESTION_GEN_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt.to_string(),
        }],
        max_tokens: 1024,
        temperature: 0.4,
        model: model.to_string(),
    };

    tracing::info!(
        target: "planner.socratic.question_planner",
        lane,
        model,
        dimension = %target_dimension.label(),
        "Starting Socratic question generation"
    );

    let started_at = Instant::now();
    let response = router.complete(request).await?;
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::info!(
        target: "planner.socratic.question_planner",
        lane,
        model = %response.model,
        dimension = %target_dimension.label(),
        elapsed_ms,
        "Completed Socratic question generation"
    );

    parse_question_response(&response.content, target_dimension)
}

fn choose_generation_lane(
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    conversation_history: &[SocraticTurn],
) -> QuestionGenerationLane {
    if scaffold_question(state, strategy, conversation_history).is_some() {
        return QuestionGenerationLane::Scaffold;
    }

    if matches!(strategy.target_dimension, Dimension::Custom(_))
        || has_unresolved_contradictions(state)
        || conversation_history.len() >= COMPLEX_HISTORY_TURN_THRESHOLD
        || unresolved_dependency_count(&strategy.target_dimension, state) > 1
    {
        return QuestionGenerationLane::DeepModel;
    }

    QuestionGenerationLane::FastModel
}

fn has_unresolved_contradictions(state: &RequirementsBeliefState) -> bool {
    state.contradictions.iter().any(|contradiction| !contradiction.resolved)
}

fn unresolved_dependency_count(dimension: &Dimension, state: &RequirementsBeliefState) -> usize {
    dependencies_for(dimension)
        .iter()
        .filter(|dependency| {
            !state.filled.contains_key(*dependency) && !state.out_of_scope.contains(*dependency)
        })
        .count()
}

fn dependencies_for(dimension: &Dimension) -> Vec<Dimension> {
    match dimension {
        Dimension::Auth => vec![Dimension::Stakeholders, Dimension::CoreFeatures],
        Dimension::Performance
        | Dimension::Scalability
        | Dimension::ErrorHandling
        | Dimension::DataModel
        | Dimension::Integrations => vec![Dimension::CoreFeatures],
        _ => Vec::new(),
    }
}

fn scaffold_question(
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    _conversation_history: &[SocraticTurn],
) -> Option<QuestionOutput> {
    if state.uncertain.contains_key(&strategy.target_dimension)
        || has_unresolved_contradictions(state)
        || matches!(strategy.target_dimension, Dimension::Custom(_))
    {
        return None;
    }

    let (question, quick_options, allow_skip) = match &strategy.target_dimension {
        Dimension::Goal => (
            "What is the main outcome this project needs to deliver first?".to_string(),
            vec![
                quick_option("Customer product", "A customer-facing product or feature"),
                quick_option("Internal tool", "An internal workflow or operations tool"),
                quick_option("Automation", "Automation for a manual process"),
                quick_option("Reporting", "A reporting or insight workspace"),
            ],
            true,
        ),
        Dimension::Platform => (
            "Which platform should the first version prioritize?".to_string(),
            vec![
                quick_option("Web app", "Web application"),
                quick_option("Mobile app", "Mobile application"),
                quick_option("API/service", "API or backend service"),
                quick_option("Desktop app", "Desktop application"),
                quick_option("CLI/tool", "CLI or developer tool"),
            ],
            true,
        ),
        Dimension::CoreFeatures => (
            "Which capabilities must be present in the first usable version?".to_string(),
            vec![
                quick_option("Create and edit", "Create and edit the core records"),
                quick_option("Browse and search", "Browse, filter, and search existing records"),
                quick_option("Share or collaborate", "Collaboration or shared workflows"),
                quick_option("Notifications", "Notifications, reminders, or alerts"),
                quick_option("Reporting", "Reporting or dashboard views"),
            ],
            true,
        ),
        Dimension::SuccessCriteria => (
            "How will you judge the first release as successful?".to_string(),
            vec![
                quick_option("Main flow works", "Users can complete the main workflow reliably"),
                quick_option("Time saved", "The product saves time or reduces manual work"),
                quick_option("Adoption target", "Success is measured by usage or adoption"),
                quick_option("Business impact", "Success is measured by revenue or conversion impact"),
                quick_option("Fewer errors", "Success is measured by fewer mistakes or support issues"),
            ],
            true,
        ),
        Dimension::UserFlows => (
            "Which end-to-end user flow needs to work first?".to_string(),
            vec![
                quick_option("New user starts", "A new user can start and finish the primary task"),
                quick_option("Returning user", "A returning user can review and continue work"),
                quick_option("Admin setup", "An admin can set up people, settings, or permissions"),
                quick_option("Edit existing work", "A user can update or correct existing data"),
                quick_option("Repeat workflow", "A user can complete a scheduled or repeat action"),
            ],
            true,
        ),
        Dimension::OutOfScope => (
            "What should stay out of the first version on purpose?".to_string(),
            vec![
                quick_option("Advanced automation", "Advanced automation can wait"),
                quick_option("Third-party integrations", "External integrations can wait"),
                quick_option("Complex permissions", "Advanced roles or permission systems can wait"),
                quick_option("Analytics", "Advanced analytics or reporting can wait"),
                quick_option("Mobile/offline", "Mobile or offline support can wait"),
            ],
            true,
        ),
        Dimension::Stakeholders => (
            "Who needs this system most in the first release?".to_string(),
            vec![
                quick_option("Individual users", "Individual end users"),
                quick_option("Internal team", "An internal operations or delivery team"),
                quick_option("Managers/admins", "Managers, admins, or workspace owners"),
                quick_option("Customers/clients", "Customers, clients, or external users"),
                quick_option("Partners/vendors", "External partners or vendors"),
            ],
            true,
        ),
        _ => return None,
    };

    Some(QuestionOutput {
        question,
        target_dimension: strategy.target_dimension.clone(),
        quick_options,
        allow_skip,
    })
}

fn quick_option(label: &str, value: &str) -> QuickOption {
    QuickOption {
        label: label.to_string(),
        value: value.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Response Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct QuestionJson {
    question: String,
    #[serde(default)]
    quick_options: Vec<QuickOptionJson>,
    #[serde(default = "default_true")]
    allow_skip: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
struct QuickOptionJson {
    label: String,
    value: String,
}

fn parse_question_response(
    content: &str,
    target_dimension: &Dimension,
) -> StepResult<QuestionOutput> {
    let cleaned = crate::pipeline::steps::intake::strip_code_fences(content);
    let json: QuestionJson = serde_json::from_str(&cleaned)
        .or_else(|_| {
            let repaired = crate::llm::json_repair::try_repair_json(content)
                .unwrap_or_else(|| cleaned.clone());
            serde_json::from_str(&repaired)
        })
        .map_err(|e| {
            StepError::JsonError(format!(
                "Failed to parse question response: {}. Raw: {}",
                e,
                &content[..content.len().min(300)]
            ))
        })?;

    Ok(QuestionOutput {
        question: json.question,
        target_dimension: target_dimension.clone(),
        quick_options: json
            .quick_options
            .into_iter()
            .map(|o| QuickOption {
                label: o.label,
                value: o.value,
            })
            .collect(),
        allow_skip: json.allow_skip,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CompletionResponse, LlmClient, LlmError};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    fn make_empty_state() -> RequirementsBeliefState {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec![],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        RequirementsBeliefState::from_classification(&classification)
    }

    #[test]
    fn select_targets_highest_priority() {
        let state = make_empty_state();
        let strategy = select_target_dimension(&state);

        assert!(strategy.is_some());
        let s = strategy.unwrap();
        // Goal should be highest priority (weight=1.0, info_gain=1.0)
        assert_eq!(s.target_dimension, Dimension::Goal);
    }

    #[test]
    fn select_skips_filled_dimensions() {
        let mut state = make_empty_state();
        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "Task tracker".into(),
                source_turn: 1,
                source_quote: None,
            },
        );

        let strategy = select_target_dimension(&state);
        assert!(strategy.is_some());
        assert_ne!(strategy.unwrap().target_dimension, Dimension::Goal);
    }

    #[test]
    fn contradictions_get_top_priority() {
        let mut state = make_empty_state();
        state.add_contradiction(Contradiction {
            dimension_a: Dimension::Auth,
            value_a: "SSO required".into(),
            dimension_b: Dimension::Stakeholders,
            value_b: "Single user tool".into(),
            explanation: "Conflict".into(),
            resolved: false,
        });

        let strategy = select_target_dimension(&state);
        assert!(strategy.is_some());
        // Contradictions have score 2.0, which should be highest
        assert_eq!(strategy.unwrap().target_dimension, Dimension::Auth);
    }

    #[test]
    fn uncertain_dimensions_are_verified_before_new_missing_dimensions() {
        let mut state = make_empty_state();
        state.mark_uncertain(
            Dimension::Performance,
            SlotValue {
                value: "Under 1 second".into(),
                source_turn: 1,
                source_quote: None,
            },
            0.5,
        );

        let strategy = select_target_dimension(&state);
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().target_dimension, Dimension::Performance);
    }

    #[test]
    fn dependency_penalty_reduces_score() {
        let state = make_empty_state();

        // Performance should have a low score when CoreFeatures isn't filled
        let perf_penalty = dependency_penalty(&Dimension::Performance, &state);
        assert!(perf_penalty < 1.0);

        // Goal should have no penalty
        let goal_penalty = dependency_penalty(&Dimension::Goal, &state);
        assert_eq!(goal_penalty, 1.0);
    }

    #[test]
    fn parse_question_json() {
        let json = r#"{"question":"What's the primary goal?","quick_options":[{"label":"Tracking","value":"Task tracking"}],"allow_skip":false}"#;
        let result = parse_question_response(json, &Dimension::Goal).unwrap();
        assert_eq!(result.question, "What's the primary goal?");
        assert_eq!(result.quick_options.len(), 1);
        assert!(!result.allow_skip);
    }

    #[test]
    fn no_questions_when_all_filled() {
        let mut state = make_empty_state();
        // Fill all missing dimensions
        let missing_clone: Vec<_> = state.missing.clone();
        for dim in missing_clone {
            state.fill(
                dim,
                SlotValue {
                    value: "filled".into(),
                    source_turn: 1,
                    source_quote: None,
                },
            );
        }

        let strategy = select_target_dimension(&state);
        assert!(strategy.is_none());
    }

    struct RecordingMockClient {
        models: Arc<Mutex<Vec<String>>>,
        fail_first: bool,
    }

    #[async_trait]
    impl LlmClient for RecordingMockClient {
        async fn complete(
            &self,
            request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            let mut models = self
                .models
                .lock()
                .expect("model log mutex should not be poisoned");
            let call_index = models.len();
            models.push(request.model.clone());

            if self.fail_first && call_index == 0 {
                return Err(LlmError::Other("fast lane unavailable".into()));
            }

            Ok(CompletionResponse {
                content: r#"{"question":"What should Planner ask next?","quick_options":[{"label":"Option A","value":"option_a"}],"allow_skip":true}"#.into(),
                model: request.model,
                input_tokens: 10,
                output_tokens: 12,
                estimated_cost_usd: 0.0,
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    fn make_constitution() -> InterviewerConstitution {
        InterviewerConstitution::default_constitution()
    }

    fn make_strategy(target_dimension: Dimension) -> QuestionStrategy {
        QuestionStrategy {
            target_dimension,
            rationale: "Test rationale".into(),
            score: 1.0,
        }
    }

    #[tokio::test]
    async fn standard_dimensions_use_scaffolds_without_llm_calls() {
        let state = make_empty_state();
        let strategy = make_strategy(Dimension::Platform);
        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: false,
        }));

        let output = generate_question(
            &router,
            &state,
            &strategy,
            &make_constitution(),
            &[],
        )
        .await
        .expect("platform scaffold should succeed");

        assert_eq!(output.target_dimension, Dimension::Platform);
        assert_eq!(output.question, "Which platform should the first version prioritize?");
        assert!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn standard_dimensions_keep_scaffolds_even_after_long_history() {
        let state = make_empty_state();
        let strategy = make_strategy(Dimension::SuccessCriteria);
        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: false,
        }));
        let long_history: Vec<SocraticTurn> = (0..12)
            .map(|index| SocraticTurn {
                turn_number: index as u32 + 1,
                role: if index % 2 == 0 {
                    SocraticRole::Interviewer
                } else {
                    SocraticRole::User
                },
                content: format!("Turn {}", index + 1),
                target_dimension: None,
                slots_updated: Vec::new(),
                timestamp: "2026-03-24T00:00:00Z".into(),
            })
            .collect();

        let output = generate_question(
            &router,
            &state,
            &strategy,
            &make_constitution(),
            &long_history,
        )
        .await
        .expect("success criteria scaffold should still succeed");

        assert_eq!(output.target_dimension, Dimension::SuccessCriteria);
        assert_eq!(output.question, "How will you judge the first release as successful?");
        assert!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn verification_questions_use_fast_model_lane() {
        let mut state = make_empty_state();
        state.mark_uncertain(
            Dimension::Platform,
            SlotValue {
                value: "Web application".into(),
                source_turn: 1,
                source_quote: None,
            },
            0.5,
        );

        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: false,
        }));

        let output = generate_question(
            &router,
            &state,
            &make_strategy(Dimension::Platform),
            &make_constitution(),
            &[],
        )
        .await
        .expect("fast lane generation should succeed");

        assert_eq!(output.question, "What should Planner ask next?");
        assert_eq!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .as_slice(),
            [DefaultModels::INTAKE_QUESTION_FAST]
        );
    }

    #[tokio::test]
    async fn verification_questions_do_not_escalate_to_deep_for_moderate_history() {
        let mut state = make_empty_state();
        state.mark_uncertain(
            Dimension::Platform,
            SlotValue {
                value: "Web application".into(),
                source_turn: 1,
                source_quote: None,
            },
            0.5,
        );

        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: false,
        }));
        let moderate_history: Vec<SocraticTurn> = (0..10)
            .map(|index| SocraticTurn {
                turn_number: index as u32 + 1,
                role: if index % 2 == 0 {
                    SocraticRole::Interviewer
                } else {
                    SocraticRole::User
                },
                content: format!("Turn {}", index + 1),
                target_dimension: None,
                slots_updated: Vec::new(),
                timestamp: "2026-03-24T00:00:00Z".into(),
            })
            .collect();

        let output = generate_question(
            &router,
            &state,
            &make_strategy(Dimension::Platform),
            &make_constitution(),
            &moderate_history,
        )
        .await
        .expect("verification lane should remain fast for moderate history");

        assert_eq!(output.question, "What should Planner ask next?");
        assert_eq!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .as_slice(),
            [DefaultModels::INTAKE_QUESTION_FAST]
        );
    }

    #[tokio::test]
    async fn custom_dimensions_escalate_to_deep_model_lane() {
        let state = make_empty_state();
        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: false,
        }));

        let output = generate_question(
            &router,
            &state,
            &make_strategy(Dimension::Custom("Browser Support".into())),
            &make_constitution(),
            &[],
        )
        .await
        .expect("deep lane generation should succeed");

        assert_eq!(output.question, "What should Planner ask next?");
        assert_eq!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .as_slice(),
            [DefaultModels::INTAKE_QUESTION_DEEP]
        );
    }

    #[tokio::test]
    async fn fast_lane_failures_fallback_to_deep_model() {
        let mut state = make_empty_state();
        state.mark_uncertain(
            Dimension::Platform,
            SlotValue {
                value: "Web application".into(),
                source_turn: 1,
                source_quote: None,
            },
            0.5,
        );

        let models = Arc::new(Mutex::new(Vec::new()));
        let router = LlmRouter::with_mock(Box::new(RecordingMockClient {
            models: models.clone(),
            fail_first: true,
        }));

        let output = generate_question(
            &router,
            &state,
            &make_strategy(Dimension::Platform),
            &make_constitution(),
            &[],
        )
        .await
        .expect("deep fallback should recover from fast lane failure");

        assert_eq!(output.question, "What should Planner ask next?");
        assert_eq!(
            models
                .lock()
                .expect("model log mutex should not be poisoned")
                .as_slice(),
            [
                DefaultModels::INTAKE_QUESTION_FAST,
                DefaultModels::INTAKE_QUESTION_DEEP,
            ]
        );
    }
}
