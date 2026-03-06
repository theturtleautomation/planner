//! # Question Planner — Dimension Scoring + Question Generation
//!
//! Selects the next question to maximize information gain while
//! respecting the question budget and UX cost.
//!
//! Two-level selection:
//! 1. Strategy: which dimension to target (priority × information gain)
//! 2. Generation: how to ask about the chosen dimension (LLM call)

use planner_schemas::*;

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
- Provide 2-4 quick-select options when the answer is likely categorical.
- Include a "Not sure yet" option via allow_skip: true when the question is optional.
- Reference what's already known to show you've been listening.
- Use Paul & Elder's taxonomy: clarifying → probing assumptions → exploring implications.
- Calibrate difficulty to the user's expertise level.
- Never assume technologies the user hasn't mentioned."#;

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

    // Step 2: Generate the question via LLM
    let output =
        generate_question(router, state, &strategy, constitution, conversation_history).await?;

    // Step 3: Self-critique against constitution
    let violations = evaluate_question(
        &output.question,
        &strategy.target_dimension,
        state,
        constitution,
    );

    if !violations.is_empty() {
        // Regenerate with critique feedback
        let output = regenerate_with_critique(
            router,
            state,
            &strategy,
            constitution,
            conversation_history,
            &violations,
        )
        .await?;
        return Ok(Some(output));
    }

    Ok(Some(output))
}

/// Select the best dimension to target next.
///
/// Scores each missing/uncertain dimension by:
/// - Priority weight (from Dimension::priority_weight)
/// - Information gain estimate (missing > uncertain)
/// - Dependency check (don't ask about auth before scope is set)
pub fn select_target_dimension(state: &RequirementsBeliefState) -> Option<QuestionStrategy> {
    let mut candidates: Vec<(Dimension, f32, String)> = Vec::new();

    // Score missing dimensions (higher info gain — we know nothing)
    for dim in &state.missing {
        let weight = dim.priority_weight();
        let info_gain = 1.0; // Full info gain for missing dims
        let dep_penalty = dependency_penalty(dim, state);
        let score = weight * info_gain * dep_penalty;

        candidates.push((
            dim.clone(),
            score,
            format!(
                "Missing dimension (weight={:.2}, info_gain={:.1}, dep={:.1})",
                weight, info_gain, dep_penalty
            ),
        ));
    }

    // Score uncertain dimensions (lower info gain — we have a guess)
    for (dim, (_val, confidence)) in &state.uncertain {
        let weight = dim.priority_weight();
        let info_gain = 1.0 - confidence; // Lower confidence → higher info gain
        let dep_penalty = dependency_penalty(dim, state);
        let score = weight * info_gain * dep_penalty;

        candidates.push((
            dim.clone(),
            score,
            format!(
                "Uncertain (confidence={:.0}%, weight={:.2}, info_gain={:.2})",
                confidence * 100.0,
                weight,
                info_gain
            ),
        ));
    }

    // Handle unresolved contradictions — these get top priority
    for contradiction in &state.contradictions {
        if !contradiction.resolved {
            candidates.push((
                contradiction.dimension_a.clone(),
                2.0,
                format!(
                    "Unresolved contradiction: {} vs {}",
                    contradiction.dimension_a.label(),
                    contradiction.dimension_b.label()
                ),
            ));
        }
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    candidates
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
    let state_text = format_belief_state_for_llm(state);
    let history_text = format_conversation_history(conversation_history);

    let user_prompt = format!(
        "## Belief State:\n{}\n\n## Target Dimension: {} ({})\nRationale: {}\n\n## Constitution:\n{}\n\n## Conversation So Far:\n{}\n\nGenerate the next question.",
        state_text,
        strategy.target_dimension.label(),
        serde_json::to_string(&strategy.target_dimension).unwrap_or_default(),
        strategy.rationale,
        constitution.as_prompt_text(),
        history_text,
    );

    let request = CompletionRequest {
        system: Some(QUESTION_GEN_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt,
        }],
        max_tokens: 1024,
        temperature: 0.4,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    parse_question_response(&response.content, &strategy.target_dimension)
}

async fn regenerate_with_critique(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    strategy: &QuestionStrategy,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    violations: &[ConstitutionViolation],
) -> StepResult<QuestionOutput> {
    let state_text = format_belief_state_for_llm(state);
    let history_text = format_conversation_history(conversation_history);

    let critique_text: String = violations
        .iter()
        .map(|v| format!("- Rule {}: {}", v.rule_id, v.explanation))
        .collect::<Vec<_>>()
        .join("\n");

    let user_prompt = format!(
        "## Belief State:\n{}\n\n## Target Dimension: {} ({})\n\n## Constitution:\n{}\n\n## Conversation So Far:\n{}\n\n## SELF-CRITIQUE — Previous question violated these rules:\n{}\n\nGenerate a REVISED question that does NOT violate the above rules.",
        state_text,
        strategy.target_dimension.label(),
        serde_json::to_string(&strategy.target_dimension).unwrap_or_default(),
        constitution.as_prompt_text(),
        history_text,
        critique_text,
    );

    let request = CompletionRequest {
        system: Some(QUESTION_GEN_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt,
        }],
        max_tokens: 1024,
        temperature: 0.4,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    parse_question_response(&response.content, &strategy.target_dimension)
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
    use std::collections::HashMap;

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
}
