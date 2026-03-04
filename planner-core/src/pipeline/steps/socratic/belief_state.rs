//! # Belief State Manager — CRUD + CXDB Persistence
//!
//! Manages the RequirementsBeliefState lifecycle:
//! - Update from user messages via LLM Verifier pass
//! - Persist to CXDB (MessagePack on disk) after every turn
//! - Restore from CXDB for session recovery
//!
//! The Verifier is a separate LLM call that parses user messages for:
//! new filled slots, corrections, contradictions, out-of-scope declarations,
//! and expertise signals.

use chrono::Utc;
use uuid::Uuid;

use planner_schemas::*;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use crate::cxdb::TurnStore;
use super::super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// Verifier System Prompt
// ---------------------------------------------------------------------------

const VERIFIER_SYSTEM_PROMPT: &str = r#"You are a Belief State Verifier for a Socratic requirements elicitation system.

Given:
1. The current belief state (what's known, uncertain, missing)
2. The latest user message
3. The question that was asked

Your job: determine what new information the user has provided and output structured updates.

Respond with ONLY a JSON object (no markdown fences):
{
  "filled_updates": [
    {
      "dimension": "goal",
      "value": "Build a task tracker for team visibility",
      "source_quote": "I want to track tasks so nothing falls through the cracks"
    }
  ],
  "uncertain_updates": [
    {
      "dimension": "performance",
      "value": "Under 1 second response time",
      "confidence": 0.6,
      "source_quote": "it should be pretty fast"
    }
  ],
  "out_of_scope": ["scalability"],
  "contradictions": [
    {
      "dimension_a": "auth",
      "value_a": "SSO required",
      "dimension_b": "stakeholders",
      "value_b": "Single user tool",
      "explanation": "SSO implies multi-user but stakeholders say single user"
    }
  ],
  "expertise_level": "intermediate",
  "user_wants_to_stop": false
}

## Dimension keys (use these exact strings):
goal, success_criteria, stakeholders, business_context,
core_features, user_flows, data_model, integrations, auth, error_handling,
performance, availability, security, scalability, usability,
tech_stack, timeline, budget, regulatory, platform,
in_scope, out_of_scope, future_phases

## Rules:
- Only extract what the user ACTUALLY said — don't infer unstated requirements
- If the user corrects a previous slot, include it in filled_updates (it overwrites)
- Contradictions should only be flagged when two filled slots genuinely conflict
- user_wants_to_stop: true if user says "that's enough", "just build it", "go ahead", etc.
- expertise_level: "beginner", "intermediate", or "expert" — inferred from vocabulary and specificity"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Verifier output — parsed updates to apply to the belief state.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VerifierOutput {
    pub filled_updates: Vec<FilledUpdate>,
    #[serde(default)]
    pub uncertain_updates: Vec<UncertainUpdate>,
    #[serde(default)]
    pub out_of_scope: Vec<String>,
    #[serde(default)]
    pub contradictions: Vec<ContradictionUpdate>,
    #[serde(default)]
    pub expertise_level: Option<String>,
    #[serde(default)]
    pub user_wants_to_stop: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FilledUpdate {
    pub dimension: String,
    pub value: String,
    pub source_quote: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UncertainUpdate {
    pub dimension: String,
    pub value: String,
    pub confidence: f32,
    pub source_quote: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContradictionUpdate {
    pub dimension_a: String,
    pub value_a: String,
    pub dimension_b: String,
    pub value_b: String,
    pub explanation: String,
}

/// Run the Verifier pass — update belief state from the user's message.
///
/// Returns the updated belief state and whether the user wants to stop.
pub async fn verify_and_update(
    router: &LlmRouter,
    state: &mut RequirementsBeliefState,
    user_message: &str,
    question_asked: Option<&str>,
) -> StepResult<VerifierOutput> {
    let state_summary = format_belief_state_for_llm(state);

    let user_prompt = format!(
        "## Current Belief State:\n{}\n\n## Question Asked:\n{}\n\n## User Response:\n{}",
        state_summary,
        question_asked.unwrap_or("(initial message — no question was asked)"),
        user_message
    );

    let request = CompletionRequest {
        system: Some(VERIFIER_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_prompt,
        }],
        max_tokens: 2048,
        temperature: 0.2,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    let output = parse_verifier_response(&response.content)?;

    // Apply updates to the belief state
    apply_updates(state, &output);
    state.turn_count += 1;

    Ok(output)
}

/// Apply verifier output to the belief state.
fn apply_updates(state: &mut RequirementsBeliefState, output: &VerifierOutput) {
    // Apply filled updates
    for update in &output.filled_updates {
        if let Some(dim) = parse_dimension(&update.dimension) {
            state.fill(dim, SlotValue {
                value: update.value.clone(),
                source_turn: state.turn_count + 1,
                source_quote: update.source_quote.clone(),
            });
        }
    }

    // Apply uncertain updates
    for update in &output.uncertain_updates {
        if let Some(dim) = parse_dimension(&update.dimension) {
            state.mark_uncertain(dim, SlotValue {
                value: update.value.clone(),
                source_turn: state.turn_count + 1,
                source_quote: update.source_quote.clone(),
            }, update.confidence);
        }
    }

    // Apply out-of-scope
    for dim_str in &output.out_of_scope {
        if let Some(dim) = parse_dimension(dim_str) {
            state.mark_out_of_scope(dim);
        }
    }

    // Apply contradictions
    for c in &output.contradictions {
        if let (Some(dim_a), Some(dim_b)) = (parse_dimension(&c.dimension_a), parse_dimension(&c.dimension_b)) {
            state.add_contradiction(Contradiction {
                dimension_a: dim_a,
                value_a: c.value_a.clone(),
                dimension_b: dim_b,
                value_b: c.value_b.clone(),
                explanation: c.explanation.clone(),
                resolved: false,
            });
        }
    }
}

/// Persist belief state to CXDB.
pub fn persist_to_cxdb<S: TurnStore>(
    store: &S,
    run_id: Uuid,
    state: &RequirementsBeliefState,
) -> StepResult<Uuid> {
    let turn_id = Uuid::new_v4();
    let payload_bytes = rmp_serde::to_vec(state)
        .map_err(|e| StepError::StorageError(format!("Failed to serialize belief state: {}", e)))?;
    let blob_hash = blake3::hash(&payload_bytes).to_hex().to_string();

    let turn = Turn {
        turn_id,
        type_id: RequirementsBeliefState::TYPE_ID.to_string(),
        parent_id: None,
        blob_hash,
        payload: state.clone(),
        metadata: TurnMetadata {
            created_at: Utc::now(),
            produced_by: "socratic_engine.belief_state".into(),
            run_id,
            execution_id: format!("belief_state_turn_{}", state.turn_count),
            note: Some(format!("Turn {} — {} filled, {} uncertain, {} missing",
                state.turn_count,
                state.filled.len(),
                state.uncertain.len(),
                state.missing.len(),
            )),
            project_id: None,
        },
    };

    store.store_turn(&turn)
        .map_err(|e| StepError::StorageError(format!("Failed to persist belief state: {}", e)))?;

    Ok(turn_id)
}

/// Restore the latest belief state from CXDB.
pub fn restore_from_cxdb<S: TurnStore>(
    store: &S,
    run_id: Uuid,
) -> StepResult<Option<RequirementsBeliefState>> {
    let result = store.get_latest_turn::<RequirementsBeliefState>(
        run_id,
        RequirementsBeliefState::TYPE_ID,
    ).map_err(|e| StepError::StorageError(format!("Failed to restore belief state: {}", e)))?;

    Ok(result.map(|turn| turn.payload))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a dimension string from the LLM into a Dimension enum.
pub fn parse_dimension(s: &str) -> Option<Dimension> {
    match s.trim().to_lowercase().as_str() {
        "goal" => Some(Dimension::Goal),
        "success_criteria" => Some(Dimension::SuccessCriteria),
        "stakeholders" => Some(Dimension::Stakeholders),
        "business_context" => Some(Dimension::BusinessContext),
        "core_features" => Some(Dimension::CoreFeatures),
        "user_flows" => Some(Dimension::UserFlows),
        "data_model" => Some(Dimension::DataModel),
        "integrations" => Some(Dimension::Integrations),
        "auth" => Some(Dimension::Auth),
        "error_handling" => Some(Dimension::ErrorHandling),
        "performance" => Some(Dimension::Performance),
        "availability" => Some(Dimension::Availability),
        "security" => Some(Dimension::Security),
        "scalability" => Some(Dimension::Scalability),
        "usability" => Some(Dimension::Usability),
        "tech_stack" => Some(Dimension::TechStack),
        "timeline" => Some(Dimension::Timeline),
        "budget" => Some(Dimension::Budget),
        "regulatory" => Some(Dimension::Regulatory),
        "platform" => Some(Dimension::Platform),
        "in_scope" => Some(Dimension::InScope),
        "out_of_scope" => Some(Dimension::OutOfScope),
        "future_phases" => Some(Dimension::FuturePhases),
        other => Some(Dimension::Custom(other.to_string())),
    }
}

/// Format the current belief state as text for inclusion in LLM prompts.
pub fn format_belief_state_for_llm(state: &RequirementsBeliefState) -> String {
    let mut text = String::new();

    text.push_str("### Filled (confirmed):\n");
    if state.filled.is_empty() {
        text.push_str("  (none yet)\n");
    } else {
        for (dim, val) in &state.filled {
            text.push_str(&format!("  - {}: {}\n", dim.label(), val.value));
        }
    }

    text.push_str("\n### Uncertain (guesses):\n");
    if state.uncertain.is_empty() {
        text.push_str("  (none)\n");
    } else {
        for (dim, (val, conf)) in &state.uncertain {
            text.push_str(&format!("  - {} ({}% confidence): {}\n",
                dim.label(), (conf * 100.0) as u32, val.value));
        }
    }

    text.push_str("\n### Missing (not yet discussed):\n");
    if state.missing.is_empty() {
        text.push_str("  (none — all covered)\n");
    } else {
        for dim in &state.missing {
            text.push_str(&format!("  - {}\n", dim.label()));
        }
    }

    text.push_str("\n### Out of Scope:\n");
    if state.out_of_scope.is_empty() {
        text.push_str("  (none declared)\n");
    } else {
        for dim in &state.out_of_scope {
            text.push_str(&format!("  - {}\n", dim.label()));
        }
    }

    if !state.contradictions.is_empty() {
        text.push_str("\n### Contradictions:\n");
        for c in &state.contradictions {
            if !c.resolved {
                text.push_str(&format!("  ⚠ {} ('{}') vs {} ('{}'): {}\n",
                    c.dimension_a.label(), c.value_a,
                    c.dimension_b.label(), c.value_b,
                    c.explanation));
            }
        }
    }

    text
}

fn parse_verifier_response(content: &str) -> StepResult<VerifierOutput> {
    let cleaned = crate::pipeline::steps::intake::strip_code_fences(content);
    let output: VerifierOutput = serde_json::from_str(&cleaned)
        .or_else(|_| {
            let repaired = crate::llm::json_repair::try_repair_json(content)
                .unwrap_or_else(|| cleaned.clone());
            serde_json::from_str(&repaired)
        })
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse verifier response: {}. Raw: {}",
            e, &content[..content.len().min(300)]
        )))?;
    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_dimension_known() {
        assert_eq!(parse_dimension("goal"), Some(Dimension::Goal));
        assert_eq!(parse_dimension("CORE_FEATURES"), Some(Dimension::CoreFeatures));
        assert_eq!(parse_dimension("auth"), Some(Dimension::Auth));
    }

    #[test]
    fn parse_dimension_custom() {
        assert_eq!(
            parse_dimension("browser_support"),
            Some(Dimension::Custom("browser_support".into()))
        );
    }

    #[test]
    fn format_empty_state() {
        let state = RequirementsBeliefState {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: vec![Dimension::Goal, Dimension::CoreFeatures],
            out_of_scope: vec![],
            contradictions: vec![],
            required_dimensions: vec![Dimension::Goal, Dimension::CoreFeatures],
            turn_count: 0,
            question_budget: 12,
            classification: None,
        };

        let text = format_belief_state_for_llm(&state);
        assert!(text.contains("(none yet)"));
        assert!(text.contains("Goal / Purpose"));
        assert!(text.contains("Core Features"));
    }

    #[test]
    fn apply_filled_update() {
        let mut state = RequirementsBeliefState {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: vec![Dimension::Goal],
            out_of_scope: vec![],
            contradictions: vec![],
            required_dimensions: vec![Dimension::Goal],
            turn_count: 0,
            question_budget: 5,
            classification: None,
        };

        let output = VerifierOutput {
            filled_updates: vec![FilledUpdate {
                dimension: "goal".into(),
                value: "Build a task tracker".into(),
                source_quote: Some("I want a task tracker".into()),
            }],
            uncertain_updates: vec![],
            out_of_scope: vec![],
            contradictions: vec![],
            expertise_level: Some("intermediate".into()),
            user_wants_to_stop: false,
        };

        apply_updates(&mut state, &output);
        assert!(state.filled.contains_key(&Dimension::Goal));
        assert!(!state.missing.contains(&Dimension::Goal));
    }

    #[test]
    fn parse_verifier_json() {
        let json = r#"{
            "filled_updates": [{"dimension": "goal", "value": "Task tracker", "source_quote": "I want a task tracker"}],
            "uncertain_updates": [],
            "out_of_scope": [],
            "contradictions": [],
            "expertise_level": "intermediate",
            "user_wants_to_stop": false
        }"#;

        let result = parse_verifier_response(json).unwrap();
        assert_eq!(result.filled_updates.len(), 1);
        assert_eq!(result.filled_updates[0].dimension, "goal");
        assert!(!result.user_wants_to_stop);
    }
}
