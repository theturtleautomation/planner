//! # Speculative Draft Generator — Draft + Reaction Parsing
//!
//! After N turns (typically 3–5), generates a partial spec from the
//! current belief state and presents it for user reaction.
//!
//! User reactions to a draft are 2–5× more information-dense than
//! answers to open questions (recognition vs recall).

use planner_schemas::*;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use super::super::{StepResult, StepError};
use super::belief_state::format_belief_state_for_llm;

// ---------------------------------------------------------------------------
// Trigger Logic
// ---------------------------------------------------------------------------

/// Check if a speculative draft should be triggered.
///
/// Trigger conditions (any one is sufficient):
/// 1. filled.len() >= 5 AND uncertain.len() >= 2
/// 2. User's last response was unusually long (>200 chars)
/// 3. Turn count hits half the question budget
pub fn should_trigger_draft(
    state: &RequirementsBeliefState,
    last_user_message_len: usize,
    draft_already_shown: bool,
) -> bool {
    // Don't show draft twice in quick succession — at least 3 turns between drafts
    // (we don't track last-draft-turn here; caller should manage that)
    if draft_already_shown {
        return false;
    }

    // Condition 1: enough filled + uncertain to show something useful
    if state.filled.len() >= 5 && state.uncertain.len() >= 2 {
        return true;
    }

    // Condition 2: user gave a very detailed response
    if last_user_message_len > 200 && state.filled.len() >= 3 {
        return true;
    }

    // Condition 3: enough turns have passed
    if state.turn_count >= 6 && state.filled.len() >= 3 {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// Draft Generation
// ---------------------------------------------------------------------------

const DRAFT_SYSTEM_PROMPT: &str = r#"You are generating a speculative draft specification from the current requirements belief state.

Given the filled and uncertain dimensions, produce a structured draft that the user can review section by section.

Respond with ONLY a JSON object (no markdown fences):
{
  "sections": [
    {
      "heading": "Goal",
      "content": "Build a task tracker for team visibility...",
      "dimensions": ["goal", "business_context"]
    }
  ],
  "assumptions": [
    {
      "dimension": "performance",
      "assumption": "Sub-200ms response times for all pages",
      "confidence": 0.6
    }
  ],
  "not_discussed": ["regulatory", "future_phases"]
}

## Rules:
- Group related filled dimensions into logical sections
- Mark uncertain dimensions as explicit assumptions with confidence percentages
- List dimensions not yet discussed so the user sees what's coming
- Write in plain English, not technical jargon
- Be specific — use actual values from the belief state, not placeholders
- If something seems wrong or contradictory, flag it explicitly"#;

/// Generate a speculative draft from the current belief state.
pub async fn generate_draft(
    router: &LlmRouter,
    state: &RequirementsBeliefState,
) -> StepResult<SpeculativeDraft> {
    let state_text = format_belief_state_for_llm(state);

    let user_prompt = format!(
        "## Current Belief State:\n{}\n\nGenerate a speculative draft specification for the user to review.",
        state_text
    );

    let request = CompletionRequest {
        system: Some(DRAFT_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt,
        }],
        max_tokens: 2048,
        temperature: 0.3,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    parse_draft_response(&response.content)
}

// ---------------------------------------------------------------------------
// Response Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct DraftJson {
    sections: Vec<DraftSectionJson>,
    #[serde(default)]
    assumptions: Vec<DraftAssumptionJson>,
    #[serde(default)]
    not_discussed: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct DraftSectionJson {
    heading: String,
    content: String,
    #[serde(default)]
    dimensions: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct DraftAssumptionJson {
    dimension: String,
    assumption: String,
    confidence: f32,
}

fn parse_draft_response(content: &str) -> StepResult<SpeculativeDraft> {
    let cleaned = crate::pipeline::steps::intake::strip_code_fences(content);
    let json: DraftJson = serde_json::from_str(&cleaned)
        .or_else(|_| {
            let repaired = crate::llm::json_repair::try_repair_json(content)
                .unwrap_or_else(|| cleaned.clone());
            serde_json::from_str(&repaired)
        })
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse draft response: {}. Raw: {}",
            e, &content[..content.len().min(300)]
        )))?;

    let sections = json.sections.into_iter().map(|s| DraftSection {
        heading: s.heading,
        content: s.content,
        dimensions: s.dimensions.into_iter()
            .filter_map(|d| super::belief_state::parse_dimension(&d))
            .collect(),
    }).collect();

    let assumptions = json.assumptions.into_iter().map(|a| DraftAssumption {
        dimension: super::belief_state::parse_dimension(&a.dimension)
            .unwrap_or(Dimension::Custom(a.dimension)),
        assumption: a.assumption,
        confidence: a.confidence,
    }).collect();

    let not_discussed = json.not_discussed.into_iter()
        .filter_map(|d| super::belief_state::parse_dimension(&d))
        .collect();

    Ok(SpeculativeDraft {
        sections,
        assumptions,
        not_discussed,
    })
}

/// Format a speculative draft as plain text for display in TUI.
pub fn format_draft_for_display(draft: &SpeculativeDraft) -> String {
    let mut text = String::from("═══ DRAFT SPECIFICATION ═══\n\n");

    for section in &draft.sections {
        text.push_str(&format!("## {}\n{}\n\n", section.heading, section.content));
    }

    if !draft.assumptions.is_empty() {
        text.push_str("## Assumptions (unconfirmed)\n");
        for assumption in &draft.assumptions {
            text.push_str(&format!(
                "  ? {} ({}% confidence): {}\n",
                assumption.dimension.label(),
                (assumption.confidence * 100.0) as u32,
                assumption.assumption
            ));
        }
        text.push('\n');
    }

    if !draft.not_discussed.is_empty() {
        text.push_str("## Not Yet Discussed\n");
        for dim in &draft.not_discussed {
            text.push_str(&format!("  ○ {}\n", dim.label()));
        }
    }

    text.push_str("\n═══ Review above and correct anything that's wrong. ═══");
    text
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_state(filled_count: usize, uncertain_count: usize, turn_count: u32) -> RequirementsBeliefState {
        let mut filled = HashMap::new();
        let dims = vec![
            Dimension::Goal, Dimension::CoreFeatures, Dimension::Stakeholders,
            Dimension::Auth, Dimension::DataModel, Dimension::UserFlows,
        ];
        for (i, dim) in dims.into_iter().enumerate() {
            if i < filled_count {
                filled.insert(dim, SlotValue {
                    value: "test".into(), source_turn: i as u32 + 1, source_quote: None,
                });
            }
        }

        let mut uncertain = HashMap::new();
        let uncertain_dims = vec![Dimension::Performance, Dimension::Scalability, Dimension::Security];
        for (i, dim) in uncertain_dims.into_iter().enumerate() {
            if i < uncertain_count {
                uncertain.insert(dim, (SlotValue {
                    value: "guess".into(), source_turn: 1, source_quote: None,
                }, 0.5));
            }
        }

        RequirementsBeliefState {
            filled,
            uncertain,
            missing: vec![],
            out_of_scope: vec![],
            contradictions: vec![],
            required_dimensions: vec![],
            turn_count,
            classification: None,
        }
    }

    #[test]
    fn trigger_when_enough_filled_and_uncertain() {
        let state = make_state(5, 2, 5);
        assert!(should_trigger_draft(&state, 50, false));
    }

    #[test]
    fn no_trigger_when_too_few_filled() {
        let state = make_state(2, 2, 2);
        assert!(!should_trigger_draft(&state, 50, false));
    }

    #[test]
    fn trigger_on_long_user_message() {
        let state = make_state(3, 0, 3);
        assert!(should_trigger_draft(&state, 250, false));
    }

    #[test]
    fn trigger_at_half_budget() {
        let state = make_state(3, 0, 6); // turn 6 of 12 = half budget
        assert!(should_trigger_draft(&state, 50, false));
    }

    #[test]
    fn no_trigger_if_already_shown() {
        let state = make_state(5, 2, 5);
        assert!(!should_trigger_draft(&state, 50, true));
    }

    #[test]
    fn parse_draft_json() {
        let json = r#"{
            "sections": [{"heading": "Goal", "content": "Build a tracker", "dimensions": ["goal"]}],
            "assumptions": [{"dimension": "performance", "assumption": "Fast", "confidence": 0.6}],
            "not_discussed": ["regulatory"]
        }"#;

        let result = parse_draft_response(json).unwrap();
        assert_eq!(result.sections.len(), 1);
        assert_eq!(result.assumptions.len(), 1);
        assert_eq!(result.not_discussed.len(), 1);
    }

    #[test]
    fn format_draft_display() {
        let draft = SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a task tracker".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: vec![DraftAssumption {
                dimension: Dimension::Performance,
                assumption: "Sub-200ms responses".into(),
                confidence: 0.6,
            }],
            not_discussed: vec![Dimension::Regulatory],
        };

        let text = format_draft_for_display(&draft);
        assert!(text.contains("DRAFT SPECIFICATION"));
        assert!(text.contains("Goal"));
        assert!(text.contains("Assumptions"));
        assert!(text.contains("60%"));
        assert!(text.contains("Not Yet Discussed"));
    }
}
