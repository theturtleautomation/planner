use std::time::Instant;

use planner_core::llm::providers::LlmRouter;
use planner_core::pipeline::steps::socratic::prompt_response_adjudicator::adjudicate_prompt_response;
use planner_schemas::{
    ComplexityTier, Dimension, DomainClassification, ProjectType, PromptAnswer, PromptDirectEffect,
    PromptEnvelope, PromptItem, PromptItemKind, PromptKind, PromptOption, PromptPreferredLayout,
    PromptResponse, PromptResponseMode, PromptUiHints, RequirementsBeliefState,
};

fn benchmark_state() -> RequirementsBeliefState {
    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec![],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    RequirementsBeliefState::from_classification(&classification)
}

fn benchmark_prompt(item_count: usize) -> PromptEnvelope {
    let dimensions = Dimension::required_for(&ProjectType::WebApp);
    let items = (0..item_count)
        .map(|index| {
            let dimension = dimensions[index % dimensions.len()].clone();
            PromptItem {
                item_id: format!("benchmark-item-{index}"),
                kind: PromptItemKind::Verification,
                target_dimension: Some(dimension.clone()),
                section_ref: None,
                text: format!("Benchmark verification prompt for {}", dimension.label()),
                options: vec![PromptOption {
                    option_id: format!("opt-set-{index}"),
                    label: "Set value".into(),
                    semantic_value: format!("value-{index}"),
                    direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                        dimension,
                        value: format!("value-{index}"),
                    }),
                }],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority: (item_count.saturating_sub(index)) as u32,
                dependency_item_ids: Vec::new(),
            }
        })
        .collect();

    PromptEnvelope {
        prompt_id: "benchmark-prompt".into(),
        kind: PromptKind::VerificationBatch,
        title: "Benchmark".into(),
        instructions: None,
        origin_category_id: None,
        category_path: Vec::new(),
        items,
        draft_snapshot: None,
        required_item_ids: Vec::new(),
        allow_partial_submit: true,
        ui_hints: PromptUiHints {
            preferred_layout: PromptPreferredLayout::Cards,
            show_draft_sidebar: false,
        },
        based_on_turn: 10,
        created_at: "2026-03-08T00:00:00Z".into(),
    }
}

fn benchmark_response(prompt: &PromptEnvelope) -> PromptResponse {
    PromptResponse {
        prompt_id: prompt.prompt_id.clone(),
        answers: prompt
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| PromptAnswer {
                item_id: item.item_id.clone(),
                selected_option_id: Some(format!("opt-set-{index}")),
                custom_text: None,
                structured_payload: None,
                skipped: false,
            })
            .collect(),
        submitted_at: "2026-03-08T00:00:00Z".into(),
        client_context: None,
    }
}

#[tokio::test]
#[ignore = "benchmark-style latency measurement"]
async fn benchmark_high_answer_count_prompt_batch_adjudication() {
    let router = LlmRouter::from_env();
    let prompt = benchmark_prompt(24);
    let response = benchmark_response(&prompt);

    let iterations = 200usize;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut state = benchmark_state();
        let adjudication = adjudicate_prompt_response(&router, &mut state, &prompt, &response)
            .await
            .expect("benchmark adjudication should succeed");
        assert_eq!(adjudication.applied_answers.len(), prompt.items.len());
    }
    let elapsed = start.elapsed();
    let per_iteration_ms = (elapsed.as_secs_f64() * 1000.0) / iterations as f64;

    eprintln!(
        "benchmark.prompt_batch_adjudication iterations={} total_ms={} per_iteration_ms={:.3}",
        iterations,
        elapsed.as_millis(),
        per_iteration_ms
    );
    assert!(per_iteration_ms.is_finite());
}
