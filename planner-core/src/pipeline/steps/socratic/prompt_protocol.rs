//! Prompt-protocol helpers shared across Socratic runtime adapters.

use std::collections::{HashMap, HashSet};

use planner_schemas::{PromptAnswer, PromptEnvelope, PromptItem, PromptResponse};

/// Returns answered prompt items in the same stable order as the prompt.
pub fn ordered_answered_items<'a>(
    prompt: &'a PromptEnvelope,
    response: &'a PromptResponse,
) -> Vec<(&'a PromptItem, &'a PromptAnswer)> {
    let mut answers_by_item: HashMap<&str, &PromptAnswer> = HashMap::new();
    for answer in &response.answers {
        answers_by_item
            .entry(answer.item_id.as_str())
            .or_insert(answer);
    }

    prompt
        .items
        .iter()
        .filter_map(|item| {
            answers_by_item
                .get(item.item_id.as_str())
                .copied()
                .filter(|answer| answer_has_payload(answer))
                .map(|answer| (item, answer))
        })
        .collect()
}

/// Returns item IDs from the prompt that were not answered in this submission.
pub fn unanswered_item_ids(prompt: &PromptEnvelope, response: &PromptResponse) -> Vec<String> {
    let answered: HashSet<&str> = response
        .answers
        .iter()
        .filter(|answer| answer_has_payload(answer))
        .map(|answer| answer.item_id.as_str())
        .collect();

    prompt
        .items
        .iter()
        .filter(|item| !answered.contains(item.item_id.as_str()))
        .map(|item| item.item_id.clone())
        .collect()
}

/// Convert a structured prompt answer into verifier-ready plain text.
pub fn answer_to_input_text(answer: &PromptAnswer, item: &PromptItem) -> Option<String> {
    if answer.skipped {
        return Some(String::from("skip"));
    }

    let selected = answer
        .selected_option_id
        .as_ref()
        .map(|selected_option_id| {
            item.options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .map(|option| option.semantic_value.clone())
                .unwrap_or_else(|| selected_option_id.clone())
        });

    let custom = answer
        .custom_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let structured = structured_payload_to_input_text(answer, item);

    let mut parts = Vec::new();
    if let Some(selected) = selected {
        parts.push(selected);
    }
    if let Some(custom) = custom {
        parts.push(custom);
    }
    if let Some(structured) = structured {
        parts.push(structured);
    }

    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn answer_has_payload(answer: &PromptAnswer) -> bool {
    if answer.skipped {
        return true;
    }

    if answer
        .selected_option_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }

    answer
        .custom_text
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || answer
            .structured_payload
            .as_ref()
            .is_some_and(structured_payload_has_content)
}

fn structured_payload_has_content(payload: &planner_schemas::PromptStructuredAnswer) -> bool {
    !payload.ordered_option_ids.is_empty()
        || !payload.field_values.is_empty()
        || payload.scalar_value.is_some()
        || payload
            .selected_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn structured_payload_to_input_text(
    answer: &PromptAnswer,
    item: &PromptItem,
) -> Option<String> {
    let payload = answer.structured_payload.as_ref()?;
    if !structured_payload_has_content(payload) {
        return None;
    }

    let mut parts = Vec::new();

    if !payload.ordered_option_ids.is_empty() {
        let values = payload
            .ordered_option_ids
            .iter()
            .map(|option_id| {
                item.options
                    .iter()
                    .find(|option| option.option_id == *option_id)
                    .map(|option| option.semantic_value.clone())
                    .unwrap_or_else(|| option_id.clone())
            })
            .collect::<Vec<_>>()
            .join(" > ");
        parts.push(values);
    }

    if let Some(path) = payload
        .selected_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let selected = item
            .options
            .iter()
            .find(|option| option.option_id == path)
            .map(|option| option.semantic_value.clone())
            .unwrap_or_else(|| path.to_string());
        parts.push(selected);
    }

    parts.extend(payload.field_values.iter().map(|(key, value)| {
        if key == "rationale" {
            value.clone()
        } else {
            format!("{key}: {value}")
        }
    }));

    if let Some(value) = payload.scalar_value {
        parts.push(value.to_string());
    }

    (!parts.is_empty()).then(|| parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use planner_schemas::{
        Dimension, PromptAnswer, PromptEnvelope, PromptItem, PromptItemKind, PromptKind,
        PromptOption, PromptPreferredLayout, PromptResponse, PromptResponseMode,
        PromptStructuredAnswer, PromptUiHints,
    };

    use super::*;

    fn test_prompt() -> PromptEnvelope {
        PromptEnvelope {
            prompt_id: "prompt-1".into(),
            kind: PromptKind::QuestionBatch,
            title: "Test".into(),
            instructions: None,
            origin_category_id: None,
            category_path: Vec::new(),
            items: vec![
                PromptItem {
                    item_id: "item-a".into(),
                    kind: PromptItemKind::Verification,
                    target_dimension: Some(Dimension::Goal),
                    section_ref: None,
                    text: "Question A".into(),
                    options: vec![PromptOption {
                        option_id: "opt-a".into(),
                        label: "A".into(),
                        semantic_value: "semantic-a".into(),
                        direct_effect: None,
                    }],
                    response_mode: PromptResponseMode::SingleSelectWithCustomText,
                    required: false,
                    priority: 100,
                    dependency_item_ids: Vec::new(),
                },
                PromptItem {
                    item_id: "item-b".into(),
                    kind: PromptItemKind::Discovery,
                    target_dimension: Some(Dimension::CoreFeatures),
                    section_ref: None,
                    text: "Question B".into(),
                    options: vec![PromptOption {
                        option_id: "opt-b".into(),
                        label: "B".into(),
                        semantic_value: "semantic-b".into(),
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
            based_on_turn: 2,
            created_at: "2026-03-08T00:00:00Z".into(),
        }
    }

    #[test]
    fn ordered_answers_follow_prompt_item_order() {
        let prompt = test_prompt();
        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![
                PromptAnswer {
                    item_id: "item-b".into(),
                    selected_option_id: Some("opt-b".into()),
                    custom_text: None,
                    structured_payload: None,
                    skipped: false,
                },
                PromptAnswer {
                    item_id: "item-a".into(),
                    selected_option_id: Some("opt-a".into()),
                    custom_text: None,
                    structured_payload: None,
                    skipped: false,
                },
            ],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let ordered = ordered_answered_items(&prompt, &response);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].0.item_id, "item-a");
        assert_eq!(ordered[1].0.item_id, "item-b");
    }

    #[test]
    fn unanswered_items_include_unsubmitted_prompt_items() {
        let prompt = test_prompt();
        let response = PromptResponse {
            prompt_id: prompt.prompt_id.clone(),
            answers: vec![PromptAnswer {
                item_id: "item-a".into(),
                selected_option_id: Some("opt-a".into()),
                custom_text: None,
                structured_payload: None,
                skipped: false,
            }],
            submitted_at: "2026-03-08T00:00:01Z".into(),
            client_context: None,
        };

        let unanswered = unanswered_item_ids(&prompt, &response);
        assert_eq!(unanswered, vec!["item-b".to_string()]);
    }

    #[test]
    fn answer_to_input_text_combines_structured_and_custom_text() {
        let prompt = test_prompt();
        let answer = PromptAnswer {
            item_id: "item-a".into(),
            selected_option_id: Some("opt-a".into()),
            custom_text: Some("extra detail".into()),
            structured_payload: None,
            skipped: false,
        };
        let text = answer_to_input_text(&answer, &prompt.items[0]);
        assert_eq!(text.as_deref(), Some("semantic-a\nextra detail"));
    }

    #[test]
    fn skipped_answer_maps_to_skip_token() {
        let prompt = test_prompt();
        let answer = PromptAnswer {
            item_id: "item-a".into(),
            selected_option_id: None,
            custom_text: None,
            structured_payload: None,
            skipped: true,
        };
        let text = answer_to_input_text(&answer, &prompt.items[0]);
        assert_eq!(text.as_deref(), Some("skip"));
    }

    #[test]
    fn answer_to_input_text_uses_structured_payload_when_present() {
        let prompt = test_prompt();
        let answer = PromptAnswer {
            item_id: "item-a".into(),
            selected_option_id: None,
            custom_text: None,
            structured_payload: Some(PromptStructuredAnswer {
                ordered_option_ids: vec!["opt-a".into()],
                field_values: [("detail".into(), "Need SSO".into())].into_iter().collect(),
                scalar_value: Some(0.8),
                selected_path: Some("path-a".into()),
            }),
            skipped: false,
        };

        let text = answer_to_input_text(&answer, &prompt.items[0]);
        assert_eq!(
            text.as_deref(),
            Some("semantic-a\npath-a\ndetail: Need SSO\n0.8")
        );
    }

    #[test]
    fn reissued_item_can_be_reworded_with_same_item_identity() {
        let original = test_prompt();
        let mut reissued = test_prompt();
        reissued.items[0].text = "Question A (reworded with newer context)".into();

        assert_eq!(original.items[0].item_id, reissued.items[0].item_id);
        assert_ne!(original.items[0].text, reissued.items[0].text);
    }
}
