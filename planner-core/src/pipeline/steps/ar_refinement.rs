//! # AR Refinement Loop — Blocking Findings → Spec Amendments
//!
//! When the Adversarial Review produces blocking findings, this module
//! drives the refinement loop:
//!
//! 1. Collect all blocking findings from the ArReportV1
//! 2. Ask the AR Refiner (Opus) to produce spec amendments
//! 3. Apply amendments to the NLSpecV1
//! 4. Re-lint the amended spec
//! 5. If lint passes, return the amended spec
//! 6. If lint fails or new blocking findings emerge, loop (up to MAX_REFINEMENT_ITERATIONS)
//!
//! Open Question Resolution:
//! - If an AR finding identifies an unresolvable ambiguity (Open Question),
//!   the system generates a Consequence Card for user input.
//! - The OQ is added to the spec's open_questions list.
//! - The pipeline halts until the user resolves it.

use uuid::Uuid;

use super::linter;
use super::{StepError, StepResult};
use crate::llm::providers::LlmRouter;
use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use planner_schemas::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum iterations of the refine → re-lint loop.
pub const MAX_REFINEMENT_ITERATIONS: u32 = 3;

// ---------------------------------------------------------------------------
// Refinement prompt
// ---------------------------------------------------------------------------

const REFINEMENT_SYSTEM_PROMPT: &str = r#"You are the NLSpec Refiner for Planner v2. You receive blocking findings from the Adversarial Review and must produce precise amendments to fix them.

## Your Job
Take each blocking finding and produce a specific amendment to the NLSpec that resolves it.

## Input
You receive:
1. The current NLSpec text
2. A list of blocking findings with descriptions and suggested resolutions

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "amendments": [
    {
      "finding_id": "AR-B-1",
      "section": "requirements",
      "action": "modify"|"add"|"remove",
      "target_id": "FR-1" or null,
      "new_content": "The amended text for this item",
      "rationale": "Why this change resolves the finding"
    }
  ],
  "open_questions": [
    "Question that needs user input to resolve (only if finding cannot be resolved without user intent)"
  ],
  "amendment_log_entry": "Summary of all changes made in this refinement pass"
}

## Rules
1. Only amend what's necessary to resolve blocking findings
2. Never change Sacred Anchors (they're immutable)
3. Preserve existing requirement IDs — modify in place, don't renumber
4. If a finding truly requires user input to resolve, add it as an open_question instead of guessing
5. Keep amendments minimal and precise
6. The amendment_log_entry should be a single human-readable sentence"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Result of the AR refinement process.
#[derive(Debug)]
pub struct RefinementResult {
    /// The amended NLSpec (or original if no amendments needed).
    pub spec: NLSpecV1,
    /// How many refinement iterations were needed (0 = no blocking findings).
    pub iterations: u32,
    /// Open questions that need user resolution (generates Consequence Cards).
    pub open_questions: Vec<String>,
    /// Whether refinement succeeded (all blocking findings resolved).
    pub resolved: bool,
    /// Amendment log entries from all iterations.
    pub amendment_entries: Vec<String>,
}

/// Run the AR refinement loop: blocking findings → amendments → re-lint.
///
/// Returns the refined spec if all blocking findings are resolved,
/// or the best-effort spec with remaining open questions.
pub async fn execute_ar_refinement(
    router: &LlmRouter,
    mut spec: NLSpecV1,
    ar_report: &ArReportV1,
    _project_id: Uuid,
) -> StepResult<RefinementResult> {
    // If no blocking findings, nothing to do
    if !ar_report.has_blocking {
        tracing::info!("AR Refinement: no blocking findings — spec passes AR");
        return Ok(RefinementResult {
            spec,
            iterations: 0,
            open_questions: vec![],
            resolved: true,
            amendment_entries: vec![],
        });
    }

    tracing::info!(
        "AR Refinement: {} blocking finding(s) — starting refinement loop",
        ar_report.blocking_count,
    );

    let mut all_open_questions = Vec::new();
    let mut all_amendment_entries = Vec::new();
    let mut blocking_findings: Vec<ArFinding> = ar_report
        .findings
        .iter()
        .filter(|f| f.severity == ArSeverity::Blocking)
        .cloned()
        .collect();

    for iteration in 1..=MAX_REFINEMENT_ITERATIONS {
        tracing::info!(
            "  Refinement iteration {}/{}",
            iteration,
            MAX_REFINEMENT_ITERATIONS
        );

        // Build the refinement request
        let spec_text = super::ar::render_spec_for_review(&spec);
        let findings_text = render_blocking_findings(&blocking_findings);

        let request = CompletionRequest {
            system: Some(REFINEMENT_SYSTEM_PROMPT.to_string()),
            messages: vec![Message {
                role: Role::User,
                content: format!(
                    "Here is the current NLSpec:\n\n{}\n\n---\n\nBlocking findings to resolve:\n\n{}",
                    spec_text, findings_text,
                ),
            }],
            max_tokens: 2048,
            temperature: 0.1, // Very low — precision matters
            model: DefaultModels::AR_REFINER.to_string(),
        };

        let response = router.complete(request).await?;
        let refinement = parse_refinement_response(&response.content)?;

        // Apply amendments to the spec
        for amendment in &refinement.amendments {
            apply_amendment(&mut spec, amendment);
        }

        // Collect open questions
        let new_oqs: Vec<OpenQuestion> = refinement
            .open_questions
            .iter()
            .map(|q| OpenQuestion {
                question: q.clone(),
                raised_by: format!("ar-refiner-iter-{}", iteration),
                resolution: None,
            })
            .collect();
        all_open_questions.extend(refinement.open_questions.clone());
        spec.open_questions.extend(new_oqs);

        // Add to amendment log
        if !refinement.amendment_log_entry.is_empty() {
            spec.amendment_log.push(Amendment {
                timestamp: chrono::Utc::now().to_rfc3339(),
                description: refinement.amendment_log_entry.clone(),
                reason: format!("AR Refinement iteration {}", iteration),
                affected_section: "multiple".into(),
            });
            all_amendment_entries.push(refinement.amendment_log_entry);
        }

        // Re-lint the amended spec
        match linter::lint_spec(&spec) {
            Ok(()) => {
                tracing::info!("    Re-lint passed after iteration {}", iteration);
                return Ok(RefinementResult {
                    spec,
                    iterations: iteration,
                    open_questions: all_open_questions,
                    resolved: true,
                    amendment_entries: all_amendment_entries,
                });
            }
            Err(StepError::LintFailure { violations }) => {
                tracing::warn!("    Re-lint failed with {} violations — converting to blocking findings for next iteration",
                    violations.len());
                // Convert lint violations into pseudo-blocking ArFinding objects
                // so the next refinement iteration has concrete findings to address.
                blocking_findings = violations
                    .iter()
                    .enumerate()
                    .map(|(i, msg)| ArFinding {
                        id: format!("LINT-{:02}", i + 1),
                        severity: ArSeverity::Blocking,
                        reviewer: ArReviewer::Opus,
                        description: msg.clone(),
                        affected_section: "spec".into(),
                        affected_requirements: vec![],
                        suggested_resolution: Some(format!("Fix lint violation: {}", msg)),
                    })
                    .collect();
            }
            Err(e) => return Err(e),
        }
    }

    // Exhausted iterations
    tracing::warn!(
        "AR Refinement: exhausted {} iterations — {} open questions remain",
        MAX_REFINEMENT_ITERATIONS,
        all_open_questions.len(),
    );

    Ok(RefinementResult {
        spec,
        iterations: MAX_REFINEMENT_ITERATIONS,
        open_questions: all_open_questions,
        resolved: false,
        amendment_entries: all_amendment_entries,
    })
}

/// Generate Consequence Cards for any Open Questions produced by AR refinement.
///
/// Each OQ becomes a ConsequenceCardV1 that the user must resolve before
/// the pipeline can proceed.
pub fn generate_oq_consequence_cards(
    open_questions: &[String],
    project_id: Uuid,
) -> Vec<ConsequenceCardV1> {
    open_questions
        .iter()
        .map(|oq| ConsequenceCardV1 {
            card_id: Uuid::new_v4(),
            project_id,
            trigger: CardTrigger::OpenQuestion,
            problem: format!("I need your input: {}", oq),
            proposed_solution: "Please answer this question so I can refine the spec.".into(),
            impact: "The build can't proceed until this is clarified.".into(),
            actions: vec![
                CardAction {
                    label: "Answer".into(),
                    description: "Provide your answer to this question.".into(),
                },
                CardAction {
                    label: "Out of Scope".into(),
                    description: "Mark this as out of scope for now.".into(),
                },
            ],
            status: CardStatus::Pending,
            resolution: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn render_blocking_findings(findings: &[ArFinding]) -> String {
    findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            format!(
                "{}. [{}] {}: {}\n   Section: {}\n   Affected: {:?}\n   Suggested fix: {}",
                i + 1,
                f.id,
                format!("{:?}", f.reviewer),
                f.description,
                f.affected_section,
                f.affected_requirements,
                f.suggested_resolution
                    .as_deref()
                    .unwrap_or("(none provided)"),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Debug)]
struct ParsedRefinement {
    amendments: Vec<ParsedAmendment>,
    open_questions: Vec<String>,
    amendment_log_entry: String,
}

#[derive(Debug)]
struct ParsedAmendment {
    #[allow(dead_code)] // Part of AR finding traceability — read by downstream consumers
    finding_id: String,
    section: String,
    action: String,
    target_id: Option<String>,
    new_content: String,
    #[allow(dead_code)]
    rationale: String,
}

#[derive(Debug, serde::Deserialize)]
struct RefinementJson {
    #[serde(default)]
    amendments: Vec<AmendmentJson>,
    #[serde(default)]
    open_questions: Vec<String>,
    #[serde(default)]
    amendment_log_entry: String,
}

#[derive(Debug, serde::Deserialize)]
struct AmendmentJson {
    #[serde(default)]
    finding_id: String,
    #[serde(default)]
    section: String,
    #[serde(default)]
    action: String,
    target_id: Option<String>,
    #[serde(default)]
    new_content: String,
    #[serde(default)]
    rationale: String,
}

fn parse_refinement_response(content: &str) -> StepResult<ParsedRefinement> {
    let cleaned = crate::llm::json_repair::try_repair_json(content)
        .unwrap_or_else(|| super::intake::strip_code_fences(content));

    let json: RefinementJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse AR refinement response: {}. Raw: {}",
            e,
            &content[..content.len().min(300)]
        ))
    })?;

    let amendments = json
        .amendments
        .into_iter()
        .map(|a| ParsedAmendment {
            finding_id: a.finding_id,
            section: a.section,
            action: a.action,
            target_id: a.target_id,
            new_content: a.new_content,
            rationale: a.rationale,
        })
        .collect();

    Ok(ParsedRefinement {
        amendments,
        open_questions: json.open_questions,
        amendment_log_entry: json.amendment_log_entry,
    })
}

/// Apply a single amendment to the NLSpec.
///
/// Handles: requirements (modify/add), definition_of_done (modify/add),
/// architectural_constraints (add), out_of_scope (add).
fn apply_amendment(spec: &mut NLSpecV1, amendment: &ParsedAmendment) {
    let section = amendment.section.to_lowercase();
    let action = amendment.action.to_lowercase();

    match section.as_str() {
        "requirements" | "functional requirements" => match action.as_str() {
            "modify" => {
                if let Some(target_id) = &amendment.target_id {
                    if let Some(req) = spec.requirements.iter_mut().find(|r| r.id == *target_id) {
                        req.statement = amendment.new_content.clone();
                        tracing::info!("      Amended {} statement", target_id);
                    }
                }
            }
            "add" => {
                let new_id = format!("FR-{}", spec.requirements.len() + 1);
                spec.requirements.push(Requirement {
                    id: new_id.clone(),
                    statement: amendment.new_content.clone(),
                    priority: Priority::Must,
                    traces_to: vec![],
                });
                tracing::info!("      Added requirement {}", new_id);
            }
            _ => {}
        },
        "definition of done" | "dod" => {
            match action.as_str() {
                "modify" => {
                    // Modify by index in target_id (e.g., "DOD-1")
                    if let Some(target_id) = &amendment.target_id {
                        if let Some(idx_str) = target_id.strip_prefix("DOD-") {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                if idx > 0 && idx <= spec.definition_of_done.len() {
                                    spec.definition_of_done[idx - 1].criterion =
                                        amendment.new_content.clone();
                                    tracing::info!("      Amended DoD item {}", target_id);
                                }
                            }
                        }
                    }
                }
                "add" => {
                    spec.definition_of_done.push(DoDItem {
                        criterion: amendment.new_content.clone(),
                        mechanically_checkable: true,
                    });
                    tracing::info!("      Added DoD item");
                }
                _ => {}
            }
        }
        "architectural constraints" | "constraints" => {
            if action == "add" {
                spec.architectural_constraints
                    .push(amendment.new_content.clone());
                tracing::info!("      Added architectural constraint");
            }
        }
        "out of scope" => {
            if action == "add" {
                spec.out_of_scope.push(amendment.new_content.clone());
                tracing::info!("      Added out-of-scope item");
            }
        }
        "satisfaction criteria" => {
            if action == "add" {
                let new_id = format!("SC-{}", spec.satisfaction_criteria.len() + 1);
                spec.satisfaction_criteria.push(SatisfactionCriterion {
                    id: new_id.clone(),
                    description: amendment.new_content.clone(),
                    tier_hint: ScenarioTierHint::High,
                });
                tracing::info!("      Added satisfaction criterion {}", new_id);
            }
        }
        _ => {
            tracing::warn!("      Unknown section '{}' in amendment — skipped", section);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_refinement_response() {
        let content = r#"{
            "amendments": [
                {
                    "finding_id": "AR-B-1",
                    "section": "requirements",
                    "action": "modify",
                    "target_id": "FR-1",
                    "new_content": "The system must display remaining time, updating every second, and show 00:00 when complete",
                    "rationale": "Original was ambiguous about completion behavior"
                }
            ],
            "open_questions": [],
            "amendment_log_entry": "Clarified FR-1 completion behavior per AR-B-1"
        }"#;

        let result = parse_refinement_response(content);
        assert!(result.is_ok());
        let refinement = result.unwrap();
        assert_eq!(refinement.amendments.len(), 1);
        assert_eq!(refinement.amendments[0].finding_id, "AR-B-1");
        assert!(refinement.open_questions.is_empty());
    }

    #[test]
    fn parse_refinement_with_open_questions() {
        let content = r#"{
            "amendments": [],
            "open_questions": [
                "Should the timer support custom time formats (mm:ss vs just seconds)?"
            ],
            "amendment_log_entry": "Cannot resolve AR-B-2 without user input on time format"
        }"#;

        let result = parse_refinement_response(content);
        assert!(result.is_ok());
        let refinement = result.unwrap();
        assert!(refinement.amendments.is_empty());
        assert_eq!(refinement.open_questions.len(), 1);
    }

    #[test]
    fn apply_amendment_modifies_requirement() {
        let mut spec = build_test_spec();
        let amendment = ParsedAmendment {
            finding_id: "AR-B-1".into(),
            section: "requirements".into(),
            action: "modify".into(),
            target_id: Some("FR-1".into()),
            new_content: "The system must accept a positive integer duration in seconds and reject non-positive values".into(),
            rationale: "Clarified error handling".into(),
        };

        apply_amendment(&mut spec, &amendment);
        assert!(spec.requirements[0]
            .statement
            .contains("reject non-positive"));
    }

    #[test]
    fn apply_amendment_adds_requirement() {
        let mut spec = build_test_spec();
        let original_count = spec.requirements.len();

        let amendment = ParsedAmendment {
            finding_id: "AR-B-2".into(),
            section: "requirements".into(),
            action: "add".into(),
            target_id: None,
            new_content: "The system must handle zero-length durations gracefully".into(),
            rationale: "Missing edge case".into(),
        };

        apply_amendment(&mut spec, &amendment);
        assert_eq!(spec.requirements.len(), original_count + 1);
        assert!(spec
            .requirements
            .last()
            .unwrap()
            .statement
            .contains("zero-length"));
    }

    #[test]
    fn apply_amendment_adds_out_of_scope() {
        let mut spec = build_test_spec();
        let original_count = spec.out_of_scope.len();

        let amendment = ParsedAmendment {
            finding_id: "AR-B-3".into(),
            section: "out of scope".into(),
            action: "add".into(),
            target_id: None,
            new_content: "Timer persistence across browser sessions".into(),
            rationale: "Scope creep prevention".into(),
        };

        apply_amendment(&mut spec, &amendment);
        assert_eq!(spec.out_of_scope.len(), original_count + 1);
    }

    #[test]
    fn generate_oq_cards_produces_correct_cards() {
        let oqs = vec![
            "Should the timer support custom formats?".to_string(),
            "What happens when the user enters 0?".to_string(),
        ];

        let cards = generate_oq_consequence_cards(&oqs, Uuid::new_v4());
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].trigger, CardTrigger::OpenQuestion);
        assert!(cards[0].problem.contains("custom formats"));
        assert_eq!(cards[0].actions.len(), 2);
        assert_eq!(cards[0].status, CardStatus::Pending);
    }

    // Helper to build a minimal test spec
    fn build_test_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: Some("Build a timer".into()),
            sacred_anchors: Some(vec![NLSpecAnchor {
                id: "SA-1".into(),
                statement: "Never negative".into(),
            }]),
            requirements: vec![Requirement {
                id: "FR-1".into(),
                statement: "The system must accept a positive integer duration in seconds".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            }],
            architectural_constraints: vec!["React only".into()],
            phase1_contracts: Some(vec![Phase1Contract {
                name: "TimerState".into(),
                type_definition: "{ remaining: number }".into(),
                consumed_by: vec!["ui".into()],
            }]),
            external_dependencies: vec![],
            definition_of_done: vec![DoDItem {
                criterion: "Timer works".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Counts down to zero".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec!["Sound alerts".into()],
            amendment_log: vec![],
        }
    }
}
