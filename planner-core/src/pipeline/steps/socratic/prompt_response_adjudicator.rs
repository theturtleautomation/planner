//! Prompt-response adjudication for prompt-envelope Socratic submissions.
//!
//! Phase 3 introduces a single-pass adjudication model:
//! - Apply deterministic option `direct_effect` updates without LLM calls.
//! - Batch-interpret ambiguous/custom answers in one targeted LLM call.
//! - Apply all resulting updates in stable prompt-item order.

use std::collections::HashMap;

use planner_schemas::{
    Contradiction, Dimension, PromptAnswer, PromptDirectEffect, PromptEnvelope, PromptItem,
    PromptItemKind, PromptOption, PromptResponse, RequirementsBeliefState, SlotValue,
};
use serde::{Deserialize, Serialize};

use super::super::{StepError, StepResult};
use super::belief_state::{self, ContradictionUpdate, FilledUpdate, UncertainUpdate};
use super::prompt_protocol;
use crate::llm::providers::LlmRouter;
use crate::llm::{CompletionRequest, DefaultModels, Message, Role};

const BATCH_ADJUDICATOR_SYSTEM_PROMPT: &str = r#"You are a Belief State Adjudicator for a Socratic requirements system.

You will receive:
1. The current belief state.
2. A batch of answered prompt items that still need interpretation.

For each item, extract structured updates only from what the user actually said.

Return ONLY JSON with this shape:
{
  "items": [
    {
      "item_id": "string",
      "filled_updates": [
        {"dimension": "goal", "value": "value", "source_quote": "optional quote"}
      ],
      "uncertain_updates": [
        {"dimension": "performance", "value": "value", "confidence": 0.6, "source_quote": "optional quote"}
      ],
      "out_of_scope": ["scalability"],
      "contradictions": [
        {
          "dimension_a": "auth",
          "value_a": "value",
          "dimension_b": "stakeholders",
          "value_b": "value",
          "explanation": "why they conflict"
        }
      ],
      "user_wants_to_stop": false
    }
  ]
}

Rules:
- Only use these dimension keys: goal, success_criteria, stakeholders, business_context, core_features, user_flows, data_model, integrations, auth, error_handling, performance, availability, security, scalability, usability, tech_stack, timeline, budget, regulatory, platform, in_scope, out_of_scope, future_phases.
- Interpret each item independently but keep extraction faithful to the provided answer text.
- If no update is implied for an item, return empty arrays for that item.
- user_wants_to_stop is true only for explicit stop signals ("that's enough", "just build it", etc.)."#;

#[derive(Debug, Clone)]
pub struct AppliedPromptAnswer {
    pub item_id: String,
    pub turn_number: u32,
    pub content: String,
    pub target_dimension: Option<Dimension>,
    pub slots_updated: Vec<Dimension>,
    pub skipped: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PromptAdjudicationResult {
    pub applied_answers: Vec<AppliedPromptAnswer>,
    pub user_wants_to_stop: bool,
}

#[derive(Debug, Clone)]
struct OrderedAnswerContext<'a> {
    item: &'a PromptItem,
    answer: &'a PromptAnswer,
    display_input: String,
    selected_semantic_value: Option<String>,
    direct_effect: Option<PromptDirectEffect>,
    custom_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BatchAdjudicationInput {
    items: Vec<BatchAdjudicationInputItem>,
}

#[derive(Debug, Clone, Serialize)]
struct BatchAdjudicationInputItem {
    item_id: String,
    question: String,
    target_dimension: Option<String>,
    selected_option: Option<String>,
    custom_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchAdjudicationOutput {
    #[serde(default)]
    items: Vec<BatchAdjudicationOutputItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchAdjudicationOutputItem {
    item_id: String,
    #[serde(default)]
    filled_updates: Vec<FilledUpdate>,
    #[serde(default)]
    uncertain_updates: Vec<UncertainUpdate>,
    #[serde(default)]
    out_of_scope: Vec<String>,
    #[serde(default)]
    contradictions: Vec<ContradictionUpdate>,
    #[serde(default)]
    user_wants_to_stop: bool,
}

/// Apply a structured prompt submission to the belief state in stable item order.
pub async fn adjudicate_prompt_response(
    router: &LlmRouter,
    state: &mut RequirementsBeliefState,
    prompt: &PromptEnvelope,
    response: &PromptResponse,
) -> StepResult<PromptAdjudicationResult> {
    let ordered_answers = prompt_protocol::ordered_answered_items(prompt, response);

    let mut contexts = Vec::with_capacity(ordered_answers.len());
    let mut interpreted_inputs = Vec::new();

    for (item, answer) in ordered_answers {
        let display_input = prompt_protocol::answer_to_input_text(answer, item).unwrap_or_default();
        let selected_option = selected_option(item, answer);
        let selected_semantic_value = selected_option
            .map(|option| option.semantic_value.clone())
            .or_else(|| normalized_option_id(answer));
        let direct_effect = selected_option.and_then(|option| option.direct_effect.clone());
        let custom_text = normalize_custom_text(answer);

        let needs_interpretation =
            !answer.skipped && (custom_text.is_some() || direct_effect.is_none());
        if needs_interpretation {
            interpreted_inputs.push(BatchAdjudicationInputItem {
                item_id: item.item_id.clone(),
                question: item.text.clone(),
                target_dimension: item.target_dimension.as_ref().map(|dimension| {
                    serde_json::to_string(dimension).unwrap_or_else(|_| dimension.label())
                }),
                selected_option: selected_semantic_value.clone(),
                custom_text: custom_text.clone(),
            });
        }

        contexts.push(OrderedAnswerContext {
            item,
            answer,
            display_input,
            selected_semantic_value,
            direct_effect,
            custom_text,
        });
    }

    let mut interpreted_outputs =
        adjudicate_interpreted_answers(router, state, interpreted_inputs).await?;

    let mut result = PromptAdjudicationResult::default();
    for context in contexts {
        let turn_number = state.turn_count.saturating_add(1);
        let mut slots_updated = Vec::new();

        if let Some(effect) = context.direct_effect.as_ref() {
            apply_direct_effect(
                state,
                effect,
                turn_number,
                context.display_input.as_str(),
                &mut slots_updated,
            );
        }

        if let Some(output) = interpreted_outputs.remove(context.item.item_id.as_str()) {
            let user_wants_to_stop = output.user_wants_to_stop;
            apply_interpreted_output(
                state,
                output,
                turn_number,
                context
                    .custom_text
                    .as_deref()
                    .or(Some(context.display_input.as_str())),
                &mut slots_updated,
            );
            result.user_wants_to_stop |= user_wants_to_stop;
        }

        maybe_resolve_contradiction(
            state,
            context.item,
            context.answer,
            context.selected_semantic_value.as_deref(),
        );

        state.turn_count = turn_number;
        result.applied_answers.push(AppliedPromptAnswer {
            item_id: context.item.item_id.clone(),
            turn_number,
            content: context.display_input,
            target_dimension: context.item.target_dimension.clone(),
            slots_updated,
            skipped: context.answer.skipped,
        });
    }

    Ok(result)
}

fn selected_option<'a>(item: &'a PromptItem, answer: &PromptAnswer) -> Option<&'a PromptOption> {
    let selected_option_id = answer
        .selected_option_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    item.options
        .iter()
        .find(|option| option.option_id == selected_option_id)
}

fn normalize_custom_text(answer: &PromptAnswer) -> Option<String> {
    answer
        .custom_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn normalized_option_id(answer: &PromptAnswer) -> Option<String> {
    answer
        .selected_option_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

fn apply_direct_effect(
    state: &mut RequirementsBeliefState,
    effect: &PromptDirectEffect,
    turn_number: u32,
    source_text: &str,
    slots_updated: &mut Vec<Dimension>,
) {
    let source_quote = if source_text.trim().is_empty() {
        None
    } else {
        Some(source_text.trim().to_string())
    };

    match effect {
        PromptDirectEffect::SetDimensionValue { dimension, value } => {
            state.fill(
                dimension.clone(),
                SlotValue {
                    value: value.clone(),
                    source_turn: turn_number,
                    source_quote,
                },
            );
            push_unique_dimension(slots_updated, dimension.clone());
        }
        PromptDirectEffect::MarkDimensionUncertain { dimension, value } => {
            state.mark_uncertain(
                dimension.clone(),
                SlotValue {
                    value: value.clone(),
                    source_turn: turn_number,
                    source_quote,
                },
                0.5,
            );
            push_unique_dimension(slots_updated, dimension.clone());
        }
        PromptDirectEffect::MarkDimensionOutOfScope { dimension } => {
            state.mark_out_of_scope(dimension.clone());
            push_unique_dimension(slots_updated, dimension.clone());
        }
    }
}

fn apply_interpreted_output(
    state: &mut RequirementsBeliefState,
    output: BatchAdjudicationOutputItem,
    turn_number: u32,
    source_text: Option<&str>,
    slots_updated: &mut Vec<Dimension>,
) {
    let source_quote = source_text
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);

    for update in output.filled_updates {
        if let Some(dimension) = belief_state::parse_dimension(update.dimension.as_str()) {
            state.fill(
                dimension.clone(),
                SlotValue {
                    value: update.value,
                    source_turn: turn_number,
                    source_quote: update.source_quote.or_else(|| source_quote.clone()),
                },
            );
            push_unique_dimension(slots_updated, dimension);
        }
    }

    for update in output.uncertain_updates {
        if let Some(dimension) = belief_state::parse_dimension(update.dimension.as_str()) {
            state.mark_uncertain(
                dimension.clone(),
                SlotValue {
                    value: update.value,
                    source_turn: turn_number,
                    source_quote: update.source_quote.or_else(|| source_quote.clone()),
                },
                update.confidence.clamp(0.0, 1.0),
            );
            push_unique_dimension(slots_updated, dimension);
        }
    }

    for out_of_scope_dimension in output.out_of_scope {
        if let Some(dimension) = belief_state::parse_dimension(out_of_scope_dimension.as_str()) {
            state.mark_out_of_scope(dimension.clone());
            push_unique_dimension(slots_updated, dimension);
        }
    }

    for contradiction in output.contradictions {
        if let (Some(dimension_a), Some(dimension_b)) = (
            belief_state::parse_dimension(contradiction.dimension_a.as_str()),
            belief_state::parse_dimension(contradiction.dimension_b.as_str()),
        ) {
            state.add_contradiction(Contradiction {
                dimension_a,
                value_a: contradiction.value_a,
                dimension_b,
                value_b: contradiction.value_b,
                explanation: contradiction.explanation,
                resolved: false,
            });
        }
    }
}

fn maybe_resolve_contradiction(
    state: &mut RequirementsBeliefState,
    item: &PromptItem,
    answer: &PromptAnswer,
    selected_semantic_value: Option<&str>,
) {
    if answer.skipped || item.kind != PromptItemKind::Contradiction {
        return;
    }

    let Some(selected_value) = selected_semantic_value else {
        return;
    };

    for contradiction in &mut state.contradictions {
        if contradiction.resolved {
            continue;
        }

        let target_dimension_matches = match item.target_dimension.as_ref() {
            Some(target_dimension) => {
                contradiction.dimension_a == *target_dimension
                    || contradiction.dimension_b == *target_dimension
            }
            None => true,
        };
        if !target_dimension_matches {
            continue;
        }

        if contradiction.value_a == selected_value || contradiction.value_b == selected_value {
            contradiction.resolved = true;
            break;
        }
    }
}

fn push_unique_dimension(slots_updated: &mut Vec<Dimension>, dimension: Dimension) {
    if !slots_updated.contains(&dimension) {
        slots_updated.push(dimension);
    }
}

async fn adjudicate_interpreted_answers(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    items: Vec<BatchAdjudicationInputItem>,
) -> StepResult<HashMap<String, BatchAdjudicationOutputItem>> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let state_summary = belief_state::format_belief_state_for_llm(state);
    let payload =
        serde_json::to_string_pretty(&BatchAdjudicationInput { items }).map_err(|error| {
            StepError::JsonError(format!("Failed to serialize adjudication input: {}", error))
        })?;

    let user_prompt = format!(
        "## Current Belief State:\n{}\n\n## Prompt Answers To Adjudicate:\n{}\n\nReturn JSON now.",
        state_summary, payload
    );

    let request = CompletionRequest {
        system: Some(BATCH_ADJUDICATOR_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt,
        }],
        max_tokens: 2048,
        temperature: 0.1,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    let output = parse_batch_adjudication_response(response.content.as_str())?;

    let mut by_item_id = HashMap::new();
    for item in output.items {
        by_item_id.entry(item.item_id.clone()).or_insert(item);
    }
    Ok(by_item_id)
}

fn parse_batch_adjudication_response(content: &str) -> StepResult<BatchAdjudicationOutput> {
    let cleaned = crate::pipeline::steps::intake::strip_code_fences(content);
    let output: BatchAdjudicationOutput = serde_json::from_str(&cleaned)
        .or_else(|_| {
            let repaired = crate::llm::json_repair::try_repair_json(content)
                .unwrap_or_else(|| cleaned.clone());
            serde_json::from_str(&repaired)
        })
        .map_err(|error| {
            StepError::JsonError(format!(
                "Failed to parse batch adjudication response: {}. Raw: {}",
                error,
                &content[..content.len().min(300)]
            ))
        })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use async_trait::async_trait;
    use planner_schemas::{
        ComplexityTier, DomainClassification, ProjectType, PromptItemKind, PromptKind,
        PromptOption, PromptPreferredLayout, PromptResponseMode, PromptUiHints,
    };

    use crate::llm::{CompletionResponse, LlmClient, LlmError};

    use super::*;

    #[derive(Clone)]
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

    fn make_state() -> RequirementsBeliefState {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: Vec::new(),
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        RequirementsBeliefState::from_classification(&classification)
    }

    fn make_prompt(items: Vec<PromptItem>) -> PromptEnvelope {
        PromptEnvelope {
            prompt_id: "prompt-1".into(),
            kind: PromptKind::QuestionBatch,
            title: "Prompt".into(),
            instructions: None,
            items,
            draft_snapshot: None,
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Cards,
                show_draft_sidebar: false,
            },
            based_on_turn: 0,
            created_at: "2026-03-08T00:00:00Z".into(),
        }
    }

    fn make_item(
        item_id: &str,
        target_dimension: Dimension,
        options: Vec<PromptOption>,
        priority: u32,
    ) -> PromptItem {
        PromptItem {
            item_id: item_id.into(),
            kind: PromptItemKind::Verification,
            target_dimension: Some(target_dimension),
            section_ref: None,
            text: format!("Question for {}", item_id),
            options,
            response_mode: PromptResponseMode::SingleSelectWithCustomText,
            required: false,
            priority,
            dependency_item_ids: Vec::new(),
        }
    }

    #[tokio::test]
    async fn direct_effect_updates_state_without_llm_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: calls.clone(),
            response_content: r#"{"items":[]}"#.into(),
        }));

        let prompt = make_prompt(vec![make_item(
            "item-a",
            Dimension::Goal,
            vec![PromptOption {
                option_id: "set-goal".into(),
                label: "Set goal".into(),
                semantic_value: "Set a concrete goal".into(),
                direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                    dimension: Dimension::Goal,
                    value: "Build a weekly planning app".into(),
                }),
            }],
            100,
        )]);

        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![PromptAnswer {
                item_id: "item-a".into(),
                selected_option_id: Some("set-goal".into()),
                custom_text: None,
                skipped: false,
            }],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let mut state = make_state();
        let adjudication = adjudicate_prompt_response(&router, &mut state, &prompt, &response)
            .await
            .expect("adjudication should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(adjudication.applied_answers.len(), 1);
        assert_eq!(
            state
                .filled
                .get(&Dimension::Goal)
                .map(|slot| slot.value.as_str()),
            Some("Build a weekly planning app")
        );
    }

    #[tokio::test]
    async fn mixed_answers_apply_in_stable_item_order() {
        let calls = Arc::new(AtomicUsize::new(0));
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: calls.clone(),
            response_content: r#"{
              "items": [
                {
                  "item_id": "item-b",
                  "filled_updates": [{"dimension": "goal", "value": "Goal from custom text", "source_quote": "custom goal"}],
                  "uncertain_updates": [],
                  "out_of_scope": [],
                  "contradictions": [],
                  "user_wants_to_stop": false
                }
              ]
            }"#
            .into(),
        }));

        let prompt = make_prompt(vec![
            make_item(
                "item-a",
                Dimension::Goal,
                vec![PromptOption {
                    option_id: "set-goal".into(),
                    label: "Goal A".into(),
                    semantic_value: "Goal A".into(),
                    direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                        dimension: Dimension::Goal,
                        value: "Goal from direct effect".into(),
                    }),
                }],
                100,
            ),
            make_item(
                "item-b",
                Dimension::Goal,
                vec![PromptOption {
                    option_id: "free-text".into(),
                    label: "Needs details".into(),
                    semantic_value: "Needs details".into(),
                    direct_effect: None,
                }],
                90,
            ),
        ]);

        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![
                PromptAnswer {
                    item_id: "item-b".into(),
                    selected_option_id: Some("free-text".into()),
                    custom_text: Some("custom goal".into()),
                    skipped: false,
                },
                PromptAnswer {
                    item_id: "item-a".into(),
                    selected_option_id: Some("set-goal".into()),
                    custom_text: None,
                    skipped: false,
                },
            ],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let mut state = make_state();
        let adjudication = adjudicate_prompt_response(&router, &mut state, &prompt, &response)
            .await
            .expect("adjudication should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let applied_ids: Vec<&str> = adjudication
            .applied_answers
            .iter()
            .map(|answer| answer.item_id.as_str())
            .collect();
        assert_eq!(applied_ids, vec!["item-a", "item-b"]);
        assert_eq!(
            state
                .filled
                .get(&Dimension::Goal)
                .map(|slot| slot.value.as_str()),
            Some("Goal from custom text")
        );
    }

    #[tokio::test]
    async fn ambiguous_answers_use_one_batch_llm_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let router = LlmRouter::with_mock(Box::new(CountingMockClient {
            calls: calls.clone(),
            response_content: r#"{
              "items": [
                {
                  "item_id": "item-a",
                  "filled_updates": [{"dimension": "goal", "value": "Goal A", "source_quote": null}],
                  "uncertain_updates": [],
                  "out_of_scope": [],
                  "contradictions": [],
                  "user_wants_to_stop": false
                },
                {
                  "item_id": "item-b",
                  "filled_updates": [{"dimension": "core_features", "value": "Feature B", "source_quote": null}],
                  "uncertain_updates": [],
                  "out_of_scope": [],
                  "contradictions": [],
                  "user_wants_to_stop": false
                }
              ]
            }"#
            .into(),
        }));

        let prompt = make_prompt(vec![
            make_item(
                "item-a",
                Dimension::Goal,
                vec![PromptOption {
                    option_id: "opt-a".into(),
                    label: "A".into(),
                    semantic_value: "A".into(),
                    direct_effect: None,
                }],
                100,
            ),
            make_item(
                "item-b",
                Dimension::CoreFeatures,
                vec![PromptOption {
                    option_id: "opt-b".into(),
                    label: "B".into(),
                    semantic_value: "B".into(),
                    direct_effect: None,
                }],
                90,
            ),
        ]);

        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![
                PromptAnswer {
                    item_id: "item-a".into(),
                    selected_option_id: Some("opt-a".into()),
                    custom_text: Some("goal detail".into()),
                    skipped: false,
                },
                PromptAnswer {
                    item_id: "item-b".into(),
                    selected_option_id: Some("opt-b".into()),
                    custom_text: Some("feature detail".into()),
                    skipped: false,
                },
            ],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let mut state = make_state();
        let _ = adjudicate_prompt_response(&router, &mut state, &prompt, &response)
            .await
            .expect("adjudication should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            state
                .filled
                .get(&Dimension::Goal)
                .map(|slot| slot.value.as_str()),
            Some("Goal A")
        );
        assert_eq!(
            state
                .filled
                .get(&Dimension::CoreFeatures)
                .map(|slot| slot.value.as_str()),
            Some("Feature B")
        );
    }

    #[test]
    fn parse_batch_adjudication_response_accepts_json() {
        let parsed = parse_batch_adjudication_response(
            r#"{
                "items": [{
                    "item_id": "item-a",
                    "filled_updates": [],
                    "uncertain_updates": [],
                    "out_of_scope": [],
                    "contradictions": [],
                    "user_wants_to_stop": false
                }]
            }"#,
        )
        .expect("parse should succeed");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].item_id, "item-a");
    }

    #[test]
    fn parse_batch_adjudication_response_rejects_invalid_json() {
        let err = parse_batch_adjudication_response("{not-json}")
            .expect_err("invalid payload should fail to parse");
        assert!(err
            .to_string()
            .contains("Failed to parse batch adjudication response"));
    }

    #[test]
    fn contradiction_selection_marks_matching_contradiction_resolved() {
        let mut state = RequirementsBeliefState {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: vec![Dimension::Auth],
            out_of_scope: Vec::new(),
            contradictions: vec![Contradiction {
                dimension_a: Dimension::Auth,
                value_a: "SSO required".into(),
                dimension_b: Dimension::Stakeholders,
                value_b: "single user".into(),
                explanation: "Conflict".into(),
                resolved: false,
            }],
            required_dimensions: vec![Dimension::Auth],
            turn_count: 0,
            classification: None,
        };
        let item = PromptItem {
            item_id: "contradiction-auth".into(),
            kind: PromptItemKind::Contradiction,
            target_dimension: Some(Dimension::Auth),
            section_ref: None,
            text: "Resolve contradiction".into(),
            options: Vec::new(),
            response_mode: PromptResponseMode::SingleSelectWithCustomText,
            required: false,
            priority: 100,
            dependency_item_ids: Vec::new(),
        };
        let answer = PromptAnswer {
            item_id: "contradiction-auth".into(),
            selected_option_id: Some("keep-a".into()),
            custom_text: None,
            skipped: false,
        };

        maybe_resolve_contradiction(&mut state, &item, &answer, Some("SSO required"));
        assert!(state.contradictions[0].resolved);
    }
}
