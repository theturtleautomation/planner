//! Prompt-batch planning for Phase 2 prompt-envelope Socratic flow.

use std::collections::HashSet;

use chrono::Utc;
use planner_schemas::{
    Contradiction, Dimension, InterviewerConstitution, PromptDirectEffect, PromptEnvelope,
    PromptItem, PromptItemKind, PromptKind, PromptOption, PromptPreferredLayout,
    PromptResponseMode, PromptUiHints, RequirementsBeliefState, SocraticCategoryPathEntry,
    SocraticTurn, SpeculativeDraft,
};

use super::super::StepResult;
use super::question_planner;
use crate::llm::providers::LlmRouter;

#[derive(Debug, Clone)]
pub struct PromptCandidate {
    pub kind: PromptItemKind,
    pub target_dimension: Option<Dimension>,
    pub contradiction: Option<Contradiction>,
    pub rationale: String,
    pub score: f32,
    source: PromptCandidateSource,
}

#[derive(Debug, Clone)]
enum PromptCandidateSource {
    CoreQuestion,
    DraftVerification { value: String, confidence: f32 },
    DraftSection { heading: String },
    DraftGap,
}

/// Select prompt candidates with strict priority and dependency guards.
///
/// Priority ordering:
/// 1. contradictions
/// 2. verification
/// 3. draft review candidates
/// 4. discovery
pub fn select_prompt_candidates(
    state: &RequirementsBeliefState,
    draft: Option<&SpeculativeDraft>,
    max_visible_items: u32,
) -> Vec<PromptCandidate> {
    let limit = max_visible_items.max(1) as usize;
    let mut candidates = Vec::new();

    let mut contradictions: Vec<PromptCandidate> = state
        .contradictions
        .iter()
        .filter(|c| !c.resolved)
        .map(|c| PromptCandidate {
            kind: PromptItemKind::Contradiction,
            target_dimension: Some(c.dimension_a.clone()),
            contradiction: Some(c.clone()),
            rationale: format!(
                "Resolve contradiction between '{}' and '{}'",
                c.dimension_a.label(),
                c.dimension_b.label()
            ),
            score: 10_000.0,
            source: PromptCandidateSource::CoreQuestion,
        })
        .collect();
    contradictions.sort_by(|a, b| candidate_sort_key(a).cmp(&candidate_sort_key(b)));
    candidates.extend(contradictions);

    let mut verification: Vec<PromptCandidate> = state
        .uncertain
        .iter()
        .map(|(dimension, (slot, confidence))| {
            let score = dimension.priority_weight() * (1.0 - confidence);
            let source = if draft.is_some() {
                PromptCandidateSource::DraftVerification {
                    value: slot.value.clone(),
                    confidence: *confidence,
                }
            } else {
                PromptCandidateSource::CoreQuestion
            };
            PromptCandidate {
                kind: PromptItemKind::Verification,
                target_dimension: Some(dimension.clone()),
                contradiction: None,
                rationale: format!(
                    "Verify uncertain dimension '{}' ({:.0}% confidence)",
                    dimension.label(),
                    confidence * 100.0
                ),
                score,
                source,
            }
        })
        .collect();
    verification.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| candidate_sort_key(a).cmp(&candidate_sort_key(b)))
    });
    candidates.extend(verification);

    if let Some(draft) = draft {
        for (index, section) in draft.sections.iter().enumerate() {
            candidates.push(PromptCandidate {
                kind: PromptItemKind::DraftSection,
                target_dimension: section.dimensions.first().cloned(),
                contradiction: None,
                rationale: format!("Review draft section '{}'", section.heading),
                score: 500.0 - index as f32,
                source: PromptCandidateSource::DraftSection {
                    heading: section.heading.clone(),
                },
            });
        }

        for (index, dimension) in draft.not_discussed.iter().enumerate() {
            candidates.push(PromptCandidate {
                kind: PromptItemKind::Discovery,
                target_dimension: Some(dimension.clone()),
                contradiction: None,
                rationale: format!("Close draft gap for '{}'", dimension.label()),
                score: 300.0 - index as f32,
                source: PromptCandidateSource::DraftGap,
            });
        }
    }

    let mut discovery: Vec<PromptCandidate> = state
        .missing
        .iter()
        .cloned()
        .map(|dimension| PromptCandidate {
            kind: PromptItemKind::Discovery,
            target_dimension: Some(dimension.clone()),
            contradiction: None,
            rationale: format!("Discover missing dimension '{}'", dimension.label()),
            score: dimension.priority_weight(),
            source: PromptCandidateSource::CoreQuestion,
        })
        .collect();
    discovery.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| candidate_sort_key(a).cmp(&candidate_sort_key(b)))
    });
    candidates.extend(discovery);

    let mut selected = Vec::new();
    let mut seen_dimensions: HashSet<Dimension> = HashSet::new();

    for candidate in candidates {
        if selected.len() >= limit {
            break;
        }

        let Some(target_dimension) = candidate.target_dimension.as_ref() else {
            selected.push(candidate);
            continue;
        };

        if seen_dimensions.contains(target_dimension) {
            continue;
        }

        if matches!(
            candidate.kind,
            PromptItemKind::Verification | PromptItemKind::Discovery
        ) && has_unresolved_dependencies(target_dimension, state)
        {
            continue;
        }

        seen_dimensions.insert(target_dimension.clone());
        selected.push(candidate);
    }

    selected
}

/// Plan the next prompt envelope for contradictions, verification, draft review, and discovery.
pub async fn plan_prompt_batch(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    max_visible_items: u32,
    draft: Option<&SpeculativeDraft>,
) -> StepResult<Option<PromptEnvelope>> {
    let candidates = select_prompt_candidates(state, draft, max_visible_items);
    plan_prompt_batch_from_candidates(
        router,
        state,
        constitution,
        conversation_history,
        candidates,
        draft,
        None,
        Vec::new(),
    )
    .await
}

pub async fn plan_prompt_batch_from_candidates(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    candidates: Vec<PromptCandidate>,
    draft: Option<&SpeculativeDraft>,
    origin_category_id: Option<String>,
    category_path: Vec<SocraticCategoryPathEntry>,
) -> StepResult<Option<PromptEnvelope>> {
    if candidates.is_empty() {
        return Ok(None);
    }

    let has_draft_candidates = candidates.iter().any(|candidate| {
        matches!(
            candidate.source,
            PromptCandidateSource::DraftVerification { .. }
                | PromptCandidateSource::DraftSection { .. }
                | PromptCandidateSource::DraftGap
        )
    });

    let mut items = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        let priority = (candidates.len().saturating_sub(index) as u32) * 10;
        items.push(
            build_prompt_item(
                router,
                state,
                constitution,
                conversation_history,
                candidate,
                priority,
            )
            .await?,
        );
    }

    let kind = match candidates.first() {
        Some(first)
            if matches!(
                first.source,
                PromptCandidateSource::DraftVerification { .. }
                    | PromptCandidateSource::DraftSection { .. }
                    | PromptCandidateSource::DraftGap
            ) =>
        {
            PromptKind::DraftReview
        }
        Some(first) => match first.kind {
            PromptItemKind::Contradiction => PromptKind::ContradictionBatch,
            PromptItemKind::Verification => PromptKind::VerificationBatch,
            _ => PromptKind::QuestionBatch,
        },
        None => PromptKind::QuestionBatch,
    };
    let (title, instructions) = prompt_header(kind.clone());

    let required_item_ids = items
        .iter()
        .filter(|item| item.required)
        .map(|item| item.item_id.clone())
        .collect();

    let prompt_kind_slug = match kind {
        PromptKind::QuestionBatch => "question",
        PromptKind::VerificationBatch => "verification",
        PromptKind::ContradictionBatch => "contradiction",
        PromptKind::DraftReview => "draft-review",
    };
    let category_slug = origin_category_id
        .as_deref()
        .map(|value| {
            value
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| String::from("global"));

    Ok(Some(PromptEnvelope {
        prompt_id: format!(
            "prompt-{}-{}-{}-{}",
            state.turn_count.saturating_add(1),
            prompt_kind_slug,
            category_slug,
            Utc::now().timestamp_millis()
        ),
        kind,
        title,
        instructions,
        origin_category_id,
        category_path,
        items,
        draft_snapshot: has_draft_candidates.then(|| draft.cloned()).flatten(),
        required_item_ids,
        allow_partial_submit: true,
        ui_hints: PromptUiHints {
            preferred_layout: if has_draft_candidates {
                PromptPreferredLayout::Review
            } else {
                PromptPreferredLayout::Cards
            },
            show_draft_sidebar: has_draft_candidates,
        },
        based_on_turn: state.turn_count,
        created_at: Utc::now().to_rfc3339(),
    }))
}

fn draft_section_options() -> Vec<PromptOption> {
    vec![
        PromptOption {
            option_id: String::from("confirm"),
            label: String::from("Looks correct"),
            semantic_value: String::from("confirm"),
            direct_effect: None,
        },
        PromptOption {
            option_id: String::from("correct"),
            label: String::from("Needs correction"),
            semantic_value: String::from("correct"),
            direct_effect: None,
        },
        PromptOption {
            option_id: String::from("surprise"),
            label: String::from("Unexpected but useful"),
            semantic_value: String::from("surprise"),
            direct_effect: None,
        },
        PromptOption {
            option_id: String::from("reject"),
            label: String::from("Fundamentally wrong"),
            semantic_value: String::from("reject"),
            direct_effect: None,
        },
    ]
}

async fn build_prompt_item(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    conversation_history: &[SocraticTurn],
    candidate: &PromptCandidate,
    priority: u32,
) -> StepResult<PromptItem> {
    let item_id = deterministic_item_id(candidate);

    if candidate.kind == PromptItemKind::Contradiction {
        let contradiction = candidate
            .contradiction
            .as_ref()
            .expect("contradiction candidates carry contradiction details");
        return Ok(PromptItem {
            item_id,
            kind: PromptItemKind::Contradiction,
            target_dimension: Some(contradiction.dimension_a.clone()),
            section_ref: None,
            text: format!(
                "You gave conflicting answers for '{}' and '{}'. Which should we keep?",
                contradiction.dimension_a.label(),
                contradiction.dimension_b.label()
            ),
            options: vec![
                PromptOption {
                    option_id: String::from("keep-a"),
                    label: contradiction.value_a.clone(),
                    semantic_value: contradiction.value_a.clone(),
                    direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                        dimension: contradiction.dimension_a.clone(),
                        value: contradiction.value_a.clone(),
                    }),
                },
                PromptOption {
                    option_id: String::from("keep-b"),
                    label: contradiction.value_b.clone(),
                    semantic_value: contradiction.value_b.clone(),
                    direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                        dimension: contradiction.dimension_b.clone(),
                        value: contradiction.value_b.clone(),
                    }),
                },
                PromptOption {
                    option_id: String::from("neither"),
                    label: String::from("Neither; I'll clarify"),
                    semantic_value: String::from("neither"),
                    direct_effect: None,
                },
            ],
            response_mode: PromptResponseMode::SingleSelectWithCustomText,
            required: false,
            priority,
            dependency_item_ids: Vec::new(),
        });
    }

    match &candidate.source {
        PromptCandidateSource::DraftSection { heading } => {
            return Ok(PromptItem {
                item_id,
                kind: PromptItemKind::DraftSection,
                target_dimension: candidate.target_dimension.clone(),
                section_ref: Some(heading.clone()),
                text: format!(
                    "Review section '{}'. Confirm what is accurate and correct what is wrong.",
                    heading
                ),
                options: draft_section_options(),
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority,
                dependency_item_ids: Vec::new(),
            });
        }
        PromptCandidateSource::DraftGap => {
            let target_dimension = candidate
                .target_dimension
                .clone()
                .expect("draft gap candidates carry a target dimension");
            return Ok(PromptItem {
                item_id,
                kind: PromptItemKind::Discovery,
                target_dimension: Some(target_dimension.clone()),
                section_ref: Some(String::from("Not yet discussed")),
                text: format!(
                    "The draft does not cover '{}'. Should we include it now?",
                    target_dimension.label()
                ),
                options: vec![
                    PromptOption {
                        option_id: String::from("add-details"),
                        label: String::from("Add details now"),
                        semantic_value: String::from("add_details"),
                        direct_effect: None,
                    },
                    PromptOption {
                        option_id: String::from("out-of-scope"),
                        label: String::from("Out of scope for now"),
                        semantic_value: String::from("out_of_scope"),
                        direct_effect: Some(PromptDirectEffect::MarkDimensionOutOfScope {
                            dimension: target_dimension.clone(),
                        }),
                    },
                    PromptOption {
                        option_id: String::from("defer"),
                        label: String::from("Defer / still uncertain"),
                        semantic_value: String::from("defer"),
                        direct_effect: Some(PromptDirectEffect::MarkDimensionUncertain {
                            dimension: target_dimension,
                            value: String::from("Needs clarification"),
                        }),
                    },
                ],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority,
                dependency_item_ids: Vec::new(),
            });
        }
        PromptCandidateSource::DraftVerification { value, confidence } => {
            let dimension = candidate
                .target_dimension
                .clone()
                .expect("draft verification candidates carry a target dimension");
            return Ok(PromptItem {
                item_id,
                kind: PromptItemKind::Verification,
                target_dimension: Some(dimension.clone()),
                section_ref: Some(String::from("Draft verification")),
                text: format!(
                    "This draft is still uncertain about '{}': \"{}\" ({:.0}% confidence). Confirm or correct it.",
                    dimension.label(),
                    value,
                    confidence * 100.0
                ),
                options: vec![
                    PromptOption {
                        option_id: String::from("confirm"),
                        label: String::from("Confirm as written"),
                        semantic_value: String::from("confirm"),
                        direct_effect: Some(PromptDirectEffect::SetDimensionValue {
                            dimension: dimension.clone(),
                            value: value.clone(),
                        }),
                    },
                    PromptOption {
                        option_id: String::from("still-uncertain"),
                        label: String::from("Still uncertain"),
                        semantic_value: String::from("still_uncertain"),
                        direct_effect: Some(PromptDirectEffect::MarkDimensionUncertain {
                            dimension: dimension.clone(),
                            value: value.clone(),
                        }),
                    },
                    PromptOption {
                        option_id: String::from("out-of-scope"),
                        label: String::from("Out of scope"),
                        semantic_value: String::from("out_of_scope"),
                        direct_effect: Some(PromptDirectEffect::MarkDimensionOutOfScope {
                            dimension,
                        }),
                    },
                ],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority,
                dependency_item_ids: Vec::new(),
            });
        }
        PromptCandidateSource::CoreQuestion => {}
    }

    let target_dimension = candidate
        .target_dimension
        .clone()
        .expect("core question candidates carry a target dimension");
    let generated = question_planner::plan_question_for_dimension(
        router,
        state,
        constitution,
        conversation_history,
        target_dimension.clone(),
        candidate.rationale.as_str(),
    )
    .await?;

    Ok(PromptItem {
        item_id,
        kind: candidate.kind.clone(),
        target_dimension: Some(target_dimension),
        section_ref: None,
        text: generated.question,
        options: generated
            .quick_options
            .into_iter()
            .enumerate()
            .map(|(idx, option)| PromptOption {
                option_id: format!("option-{}", idx + 1),
                label: option.label,
                semantic_value: option.value,
                direct_effect: None,
            })
            .collect(),
        response_mode: PromptResponseMode::SingleSelectWithCustomText,
        required: !generated.allow_skip,
        priority,
        dependency_item_ids: Vec::new(),
    })
}

fn prompt_header(kind: PromptKind) -> (String, Option<String>) {
    match kind {
        PromptKind::ContradictionBatch => (
            String::from("Resolve conflicting requirements"),
            Some(String::from(
                "Please resolve contradictions first so later answers stay consistent.",
            )),
        ),
        PromptKind::VerificationBatch => (
            String::from("Verify key assumptions"),
            Some(String::from(
                "Confirm or correct uncertain requirements before adding new scope.",
            )),
        ),
        PromptKind::DraftReview => (
            String::from("Review and refine draft"),
            Some(String::from(
                "Review draft sections and close uncertain or missing areas in the same flow.",
            )),
        ),
        PromptKind::QuestionBatch => (
            String::from("Continue requirements interview"),
            Some(String::from(
                "Answer as many items as you can; unanswered items will return later.",
            )),
        ),
    }
}

fn has_unresolved_dependencies(dimension: &Dimension, state: &RequirementsBeliefState) -> bool {
    dependencies_for(dimension)
        .iter()
        .any(|dependency| !is_dimension_resolved(dependency, state))
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

fn is_dimension_resolved(dimension: &Dimension, state: &RequirementsBeliefState) -> bool {
    state.filled.contains_key(dimension) || state.out_of_scope.contains(dimension)
}

fn slug_dimension(dimension: &Dimension) -> String {
    slug_text(&dimension.label())
}

fn slug_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn item_kind_prefix(kind: &PromptItemKind) -> &'static str {
    match kind {
        PromptItemKind::Contradiction => "contradiction",
        PromptItemKind::Verification => "verification",
        PromptItemKind::DraftSection => "draft",
        PromptItemKind::Discovery => "discovery",
    }
}

fn short_hash(input: &str) -> String {
    let hex = blake3::hash(input.as_bytes()).to_hex().to_string();
    hex.chars().take(10).collect()
}

pub(crate) fn candidate_identity_key(candidate: &PromptCandidate) -> String {
    match (&candidate.kind, &candidate.source) {
        (PromptItemKind::Contradiction, _) => {
            let contradiction = candidate
                .contradiction
                .as_ref()
                .expect("contradiction candidate carries contradiction details");
            let mut sides = vec![
                (
                    slug_dimension(&contradiction.dimension_a),
                    slug_text(&contradiction.value_a),
                ),
                (
                    slug_dimension(&contradiction.dimension_b),
                    slug_text(&contradiction.value_b),
                ),
            ];
            sides.sort();
            format!(
                "contradiction:{}:{}|{}:{}",
                sides[0].0, sides[0].1, sides[1].0, sides[1].1
            )
        }
        (_, PromptCandidateSource::DraftSection { heading }) => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("none"));
            format!("draft_section:{}:{}", slug_text(heading), dimension_slug)
        }
        (_, PromptCandidateSource::DraftGap) => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("none"));
            format!("draft_gap:{}", dimension_slug)
        }
        (_, PromptCandidateSource::DraftVerification { value, .. }) => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("none"));
            format!("draft_verify:{}:{}", dimension_slug, slug_text(value))
        }
        _ => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("none"));
            format!("{}:{}", item_kind_prefix(&candidate.kind), dimension_slug)
        }
    }
}

pub(crate) fn deterministic_item_id(candidate: &PromptCandidate) -> String {
    let key = candidate_identity_key(candidate);
    match (&candidate.kind, &candidate.source) {
        (PromptItemKind::Contradiction, _) => format!("contradiction-{}", short_hash(&key)),
        (PromptItemKind::DraftSection, PromptCandidateSource::DraftSection { heading }) => {
            format!("draft-section-{}-{}", slug_text(heading), short_hash(&key))
        }
        (PromptItemKind::Discovery, PromptCandidateSource::DraftGap) => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("unknown"));
            format!("draft-gap-{}", dimension_slug)
        }
        (PromptItemKind::Verification, PromptCandidateSource::DraftVerification { .. }) => {
            let dimension_slug = candidate
                .target_dimension
                .as_ref()
                .map(slug_dimension)
                .unwrap_or_else(|| String::from("unknown"));
            format!("draft-verify-{}-{}", dimension_slug, short_hash(&key))
        }
        _ => match candidate.target_dimension.as_ref() {
            Some(dimension) => format!(
                "{}-{}",
                item_kind_prefix(&candidate.kind),
                slug_dimension(dimension)
            ),
            None => format!("{}-{}", item_kind_prefix(&candidate.kind), short_hash(&key)),
        },
    }
}

fn candidate_sort_key(candidate: &PromptCandidate) -> String {
    candidate_identity_key(candidate)
}

#[cfg(test)]
mod tests {
    use planner_schemas::{
        ComplexityTier, DomainClassification, DraftSection, ProjectType, SlotValue,
    };

    use super::*;

    fn make_state() -> RequirementsBeliefState {
        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec![],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        RequirementsBeliefState::from_classification(&classification)
    }

    #[test]
    fn contradiction_candidates_are_prioritized_before_verification_and_discovery() {
        let mut state = make_state();
        state.add_contradiction(Contradiction {
            dimension_a: Dimension::Auth,
            value_a: "SSO required".into(),
            dimension_b: Dimension::Stakeholders,
            value_b: "single-user tool".into(),
            explanation: "Conflict".into(),
            resolved: false,
        });
        state.mark_uncertain(
            Dimension::Performance,
            SlotValue {
                value: "<1s".into(),
                source_turn: 1,
                source_quote: None,
            },
            0.4,
        );

        let candidates = select_prompt_candidates(&state, None, 3);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].kind, PromptItemKind::Contradiction);
    }

    #[test]
    fn dependent_dimensions_are_excluded_until_dependencies_resolve() {
        let state = make_state();
        let candidates = select_prompt_candidates(&state, None, 10);

        assert!(candidates.iter().all(|candidate| {
            candidate.target_dimension.as_ref() != Some(&Dimension::Performance)
                && candidate.target_dimension.as_ref() != Some(&Dimension::Scalability)
                && candidate.target_dimension.as_ref() != Some(&Dimension::DataModel)
        }));
    }

    #[test]
    fn candidate_selection_respects_max_visible_items_limit() {
        let state = make_state();
        let candidates = select_prompt_candidates(&state, None, 2);
        assert!(candidates.len() <= 2);
    }

    #[test]
    fn mixed_priority_selection_orders_verification_then_draft_then_discovery() {
        let mut state = make_state();
        state.filled.insert(
            Dimension::CoreFeatures,
            SlotValue {
                value: "Timer presets and session history".into(),
                source_turn: 1,
                source_quote: None,
            },
        );
        state
            .missing
            .retain(|dimension| dimension != &Dimension::CoreFeatures);
        state.mark_uncertain(
            Dimension::Performance,
            SlotValue {
                value: "Sub-200ms p95".into(),
                source_turn: 2,
                source_quote: None,
            },
            0.55,
        );

        let draft = SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a planning assistant".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: vec![Dimension::Integrations],
        };

        let candidates = select_prompt_candidates(&state, Some(&draft), 6);
        assert_eq!(candidates[0].kind, PromptItemKind::Verification);
        assert!(matches!(
            candidates[0].source,
            PromptCandidateSource::DraftVerification { .. }
        ));
        assert!(candidates
            .iter()
            .skip(1)
            .any(|candidate| candidate.kind == PromptItemKind::DraftSection));
        assert!(candidates.iter().skip(1).any(|candidate| {
            candidate.kind == PromptItemKind::Discovery
                && matches!(candidate.source, PromptCandidateSource::DraftGap)
        }));
    }

    #[test]
    fn reissued_item_keeps_stable_identity_when_semantic_target_is_unchanged() {
        let candidate = PromptCandidate {
            kind: PromptItemKind::Discovery,
            target_dimension: Some(Dimension::Goal),
            contradiction: None,
            rationale: "Initial wording".into(),
            score: 1.0,
            source: PromptCandidateSource::CoreQuestion,
        };
        let mut reworded = candidate.clone();
        reworded.rationale = "Reworded with newer context".into();

        assert_eq!(
            deterministic_item_id(&candidate),
            deterministic_item_id(&reworded)
        );
    }

    #[test]
    fn contradiction_item_identity_changes_when_semantic_target_changes() {
        let old_candidate = PromptCandidate {
            kind: PromptItemKind::Contradiction,
            target_dimension: Some(Dimension::Auth),
            contradiction: Some(Contradiction {
                dimension_a: Dimension::Auth,
                value_a: "SSO required".into(),
                dimension_b: Dimension::Stakeholders,
                value_b: "single-user tool".into(),
                explanation: "Conflict".into(),
                resolved: false,
            }),
            rationale: "Resolve contradiction".into(),
            score: 10_000.0,
            source: PromptCandidateSource::CoreQuestion,
        };

        let new_candidate = PromptCandidate {
            kind: PromptItemKind::Contradiction,
            target_dimension: Some(Dimension::Auth),
            contradiction: Some(Contradiction {
                dimension_a: Dimension::Auth,
                value_a: "No auth needed".into(),
                dimension_b: Dimension::Stakeholders,
                value_b: "single-user tool".into(),
                explanation: "Conflict".into(),
                resolved: false,
            }),
            rationale: "Resolve contradiction".into(),
            score: 10_000.0,
            source: PromptCandidateSource::CoreQuestion,
        };

        assert_ne!(
            deterministic_item_id(&old_candidate),
            deterministic_item_id(&new_candidate)
        );
    }
}
