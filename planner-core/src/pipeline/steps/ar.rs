//! # Adversarial Review — Three-Model NLSpec Review
//!
//! AR runs after spec linting, before graph.dot generation. Three LLMs
//! review the NLSpec independently, each with a different lens:
//!
//! | Reviewer | Lens                     | Specific Checks                                           |
//! |----------|--------------------------|-----------------------------------------------------------|
//! | Opus     | Intent completeness      | Sacred Anchors covered? SC behavioral? OQs resolved?      |
//! | GPT      | Implementability         | FRs unambiguous? Contracts precise? No contradictions?     |
//! | Gemini   | Scope integrity          | Out-of-scope complete? FRs within intent? DoD checkable?  |
//!
//! Findings are categorized as blocking / advisory / informational.
//! Blocking findings prevent graph.dot generation — the spec must be
//! amended, re-linted, and re-reviewed before proceeding.

use uuid::Uuid;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// Reviewer system prompts
// ---------------------------------------------------------------------------

const OPUS_REVIEW_PROMPT: &str = r#"You are an Adversarial Reviewer (Intent Completeness) for Planner v2 NLSpec documents.

## Your Lens: Intent Completeness
You verify that the NLSpec faithfully captures and protects user intent.

## Checks
1. Every Sacred Anchor has ≥1 corresponding Functional Requirement (FR) that enforces it
2. Every Satisfaction Criterion is behavioral (testable via BDD scenario), not vague
3. All Open Questions are resolved (empty list)
4. The Intent Summary accurately reflects the Sacred Anchors
5. Definition of Done items are specific enough to mechanically verify
6. No requirement contradicts a Sacred Anchor

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "findings": [
    {
      "severity": "blocking"|"advisory"|"informational",
      "affected_section": "section name (e.g. Sacred Anchors, Requirements)",
      "affected_requirements": ["FR-1", "SA-2"],
      "description": "What the issue is",
      "suggested_resolution": "How to fix it"
    }
  ],
  "summary": "One paragraph overall assessment"
}

## Rules
- "blocking" = must be fixed before proceeding to code generation
- "advisory" = should be addressed but doesn't block
- "informational" = for awareness only
- Be rigorous but fair — don't flag style preferences as blocking
- If the spec is solid, return an empty findings array"#;

const GPT_REVIEW_PROMPT: &str = r#"You are an Adversarial Reviewer (Implementability) for Planner v2 NLSpec documents.

## Your Lens: Implementability
You verify that the NLSpec can be unambiguously implemented by a coding agent.

## Checks
1. Each FR is unambiguous — a developer reading it would implement the same thing
2. Phase 1 Contracts are precise enough to code against (specific types, not vague interfaces)
3. Architectural constraints don't contradict each other
4. No FR is impossible or unreasonably expensive to implement
5. External dependencies are identified with enough detail
6. Requirements don't have circular dependencies

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "findings": [
    {
      "severity": "blocking"|"advisory"|"informational",
      "affected_section": "section name",
      "affected_requirements": ["FR-1"],
      "description": "What the issue is",
      "suggested_resolution": "How to fix it"
    }
  ],
  "summary": "One paragraph overall assessment"
}

## Rules
- "blocking" = ambiguity or contradiction that would cause implementation failure
- "advisory" = could cause confusion but likely resolvable
- "informational" = style or best-practice suggestion
- Focus on things that would cause a coding agent to fail or produce wrong output"#;

const GEMINI_REVIEW_PROMPT: &str = r#"You are an Adversarial Reviewer (Scope Integrity) for Planner v2 NLSpec documents.

## Your Lens: Scope Integrity
You verify that the NLSpec doesn't suffer from scope creep and stays within its declared boundaries.

## Checks
1. Out of Scope list is complete — no obvious exclusions missing
2. Every FR is within the Intent Summary's bounds (no scope creep)
3. DoD items are mechanically checkable by a factory (not subjective)
4. Satisfaction Criteria have ≥1 critical-tier seed
5. The spec isn't trying to do too much for a single build cycle
6. Requirements trace to actual user needs, not gold-plating

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "findings": [
    {
      "severity": "blocking"|"advisory"|"informational",
      "affected_section": "section name",
      "affected_requirements": ["FR-3"],
      "description": "What the issue is",
      "suggested_resolution": "How to fix it"
    }
  ],
  "summary": "One paragraph overall assessment"
}

## Rules
- "blocking" = scope issue that would cause the build to fail or produce the wrong thing
- "advisory" = potential scope concern worth noting
- "informational" = observation about scope decisions
- Don't flag things explicitly listed in Out of Scope"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Maximum retries per individual AR reviewer LLM call.
const AR_REVIEW_MAX_RETRIES: usize = 1;

/// Run the full Adversarial Review pipeline on an NLSpec.
///
/// Three reviewers run in sequence (Phase 0/1/2 — sequential for simplicity;
/// Phase 3 can parallelize with tokio::join!). Each produces findings that
/// are merged into a single ArReportV1.
pub async fn execute_adversarial_review(
    router: &LlmRouter,
    spec: &NLSpecV1,
    project_id: Uuid,
) -> StepResult<ArReportV1> {
    tracing::info!("Adversarial Review: reviewing NLSpec (chunk={:?})", spec.chunk);

    let spec_text = render_spec_for_review(spec);

    // Run all three reviewers
    let opus_result = run_single_reviewer(
        router,
        &spec_text,
        ArReviewer::Opus,
        OPUS_REVIEW_PROMPT,
        DefaultModels::AR_REVIEWER_OPUS,
    ).await?;

    let gpt_result = run_single_reviewer(
        router,
        &spec_text,
        ArReviewer::Gpt,
        GPT_REVIEW_PROMPT,
        DefaultModels::AR_REVIEWER_GPT,
    ).await?;

    let gemini_result = run_single_reviewer(
        router,
        &spec_text,
        ArReviewer::Gemini,
        GEMINI_REVIEW_PROMPT,
        DefaultModels::AR_REVIEWER_GEMINI,
    ).await?;

    // Merge findings and build report
    let mut all_findings = Vec::new();
    let mut reviewer_summaries = Vec::new();

    for (reviewer, result) in [
        (ArReviewer::Opus, opus_result),
        (ArReviewer::Gpt, gpt_result),
        (ArReviewer::Gemini, gemini_result),
    ] {
        let blocking_count = result.findings.iter()
            .filter(|f| f.severity == ArSeverity::Blocking).count() as u32;

        reviewer_summaries.push(ReviewerSummary {
            reviewer: reviewer.clone(),
            summary: result.summary,
            finding_count: result.findings.len() as u32,
            blocking_count,
        });

        all_findings.extend(result.findings);
    }

    // Assign IDs to findings
    let mut blocking_idx = 0u32;
    let mut advisory_idx = 0u32;
    let mut info_idx = 0u32;

    for finding in &mut all_findings {
        match finding.severity {
            ArSeverity::Blocking => {
                blocking_idx += 1;
                finding.id = format!("AR-B-{}", blocking_idx);
            }
            ArSeverity::Advisory => {
                advisory_idx += 1;
                finding.id = format!("AR-A-{}", advisory_idx);
            }
            ArSeverity::Informational => {
                info_idx += 1;
                finding.id = format!("AR-I-{}", info_idx);
            }
        }
    }

    let chunk_name = format!("{:?}", spec.chunk).to_lowercase();

    let mut report = ArReportV1 {
        project_id,
        chunk_name,
        nlspec_version: spec.version.clone(),
        findings: all_findings,
        reviewer_summaries,
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };
    report.recalculate();

    tracing::info!(
        "Adversarial Review complete: {} blocking, {} advisory, {} informational",
        report.blocking_count, report.advisory_count, report.informational_count,
    );

    Ok(report)
}

// ---------------------------------------------------------------------------
// Single reviewer execution
// ---------------------------------------------------------------------------

/// Result from a single AR reviewer (before merging into the report).
struct SingleReviewResult {
    findings: Vec<ArFinding>,
    summary: String,
}

/// Run a single reviewer against the NLSpec text.
async fn run_single_reviewer(
    router: &LlmRouter,
    spec_text: &str,
    reviewer: ArReviewer,
    system_prompt: &str,
    model: &str,
) -> StepResult<SingleReviewResult> {
    tracing::info!("  Running {:?} reviewer...", reviewer);

    let mut last_error = None;

    for attempt in 0..=AR_REVIEW_MAX_RETRIES {
        if attempt > 0 {
            tracing::warn!("  Retrying {:?} reviewer (attempt {}/{})",
                reviewer, attempt + 1, AR_REVIEW_MAX_RETRIES + 1);
        }

        let request = CompletionRequest {
            system: Some(system_prompt.to_string()),
            messages: vec![Message {
                role: Role::User,
                content: format!(
                    "Review this NLSpec and produce your findings:\n\n{}",
                    spec_text,
                ),
            }],
            max_tokens: 2048,
            temperature: 0.2, // Low temperature for consistent reviews
            model: model.to_string(),
        };

        match router.complete(request).await {
            Ok(response) => {
                match parse_review_response(&response.content, &reviewer) {
                    Ok(result) => {
                        tracing::info!("    {:?}: {} finding(s)", reviewer, result.findings.len());
                        return Ok(result);
                    }
                    Err(e) => {
                        tracing::warn!("    Parse error from {:?}: {}", reviewer, e);
                        last_error = Some(e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("    LLM error from {:?}: {}", reviewer, e);
                last_error = Some(StepError::LlmError(e.to_string()));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| StepError::Other(
        format!("{:?} reviewer failed after retries", reviewer),
    )))
}

// ---------------------------------------------------------------------------
// Spec rendering for review
// ---------------------------------------------------------------------------

/// Render an NLSpecV1 into a text format suitable for LLM review.
pub fn render_spec_for_review(spec: &NLSpecV1) -> String {
    let mut sections = Vec::new();

    sections.push(format!("# NLSpec — Chunk: {:?}", spec.chunk));
    sections.push(format!("Version: {}", spec.version));
    sections.push(format!("Status: {:?}", spec.status));

    if let Some(ref intent) = spec.intent_summary {
        sections.push(format!("\n## Intent Summary\n{}", intent));
    }

    if let Some(ref anchors) = spec.sacred_anchors {
        sections.push("\n## Sacred Anchors".to_string());
        for a in anchors {
            sections.push(format!("- {}: {}", a.id, a.statement));
        }
    }

    sections.push("\n## Functional Requirements".to_string());
    for r in &spec.requirements {
        sections.push(format!("- {} [{}]: {} (traces: {:?})",
            r.id, format!("{:?}", r.priority), r.statement, r.traces_to));
    }

    sections.push("\n## Architectural Constraints".to_string());
    for c in &spec.architectural_constraints {
        sections.push(format!("- {}", c));
    }

    if let Some(ref contracts) = spec.phase1_contracts {
        sections.push("\n## Phase 1 Contracts".to_string());
        for c in contracts {
            sections.push(format!("- {} = {} (consumed by: {:?})",
                c.name, c.type_definition, c.consumed_by));
        }
    }

    sections.push("\n## Definition of Done".to_string());
    for d in &spec.definition_of_done {
        sections.push(format!("- [{}] {}",
            if d.mechanically_checkable { "mechanical" } else { "manual" },
            d.criterion));
    }

    sections.push("\n## Satisfaction Criteria".to_string());
    for sc in &spec.satisfaction_criteria {
        sections.push(format!("- {} [{:?}]: {}", sc.id, sc.tier_hint, sc.description));
    }

    sections.push("\n## Open Questions".to_string());
    if spec.open_questions.is_empty() {
        sections.push("(none)".to_string());
    } else {
        for oq in &spec.open_questions {
            sections.push(format!("- {} (raised by: {})", oq.question, oq.raised_by));
        }
    }

    sections.push("\n## Out of Scope".to_string());
    for oos in &spec.out_of_scope {
        sections.push(format!("- {}", oos));
    }

    if !spec.amendment_log.is_empty() {
        sections.push("\n## Amendment Log".to_string());
        for a in &spec.amendment_log {
            sections.push(format!("- [{}] {}: {} (section: {})",
                a.timestamp, a.reason, a.description, a.affected_section));
        }
    }

    sections.join("\n")
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ReviewJson {
    #[serde(default)]
    findings: Vec<ReviewFindingJson>,
    #[serde(default)]
    summary: String,
}

#[derive(Debug, serde::Deserialize)]
struct ReviewFindingJson {
    severity: String,
    #[serde(default)]
    affected_section: String,
    #[serde(default)]
    affected_requirements: Vec<String>,
    description: String,
    #[serde(default)]
    suggested_resolution: Option<String>,
}

fn parse_review_response(content: &str, reviewer: &ArReviewer) -> StepResult<SingleReviewResult> {
    let cleaned = super::intake::strip_code_fences(content);

    let json: ReviewJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse {:?} AR response: {}. Raw: {}",
            reviewer, e, &content[..content.len().min(300)]
        ))
    })?;

    let findings: Vec<ArFinding> = json.findings.into_iter().map(|f| {
        let severity = match f.severity.to_lowercase().as_str() {
            "blocking" => ArSeverity::Blocking,
            "advisory" => ArSeverity::Advisory,
            _ => ArSeverity::Informational,
        };

        ArFinding {
            id: String::new(), // Assigned later during merge
            reviewer: reviewer.clone(),
            severity,
            affected_section: f.affected_section,
            affected_requirements: f.affected_requirements,
            description: f.description,
            suggested_resolution: f.suggested_resolution,
        }
    }).collect();

    Ok(SingleReviewResult {
        findings,
        summary: json.summary,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_review_response_empty() {
        let content = r#"{"findings": [], "summary": "The spec looks solid."}"#;
        let result = parse_review_response(content, &ArReviewer::Opus);
        assert!(result.is_ok());
        let review = result.unwrap();
        assert!(review.findings.is_empty());
        assert_eq!(review.summary, "The spec looks solid.");
    }

    #[test]
    fn parse_valid_review_with_findings() {
        let content = r#"{
            "findings": [
                {
                    "severity": "blocking",
                    "affected_section": "Requirements",
                    "affected_requirements": ["FR-1"],
                    "description": "FR-1 is ambiguous about error handling",
                    "suggested_resolution": "Specify what happens when input is invalid"
                },
                {
                    "severity": "advisory",
                    "affected_section": "Definition of Done",
                    "affected_requirements": [],
                    "description": "DoD item 2 is hard to mechanically check",
                    "suggested_resolution": null
                }
            ],
            "summary": "Generally solid but one blocking issue."
        }"#;

        let result = parse_review_response(content, &ArReviewer::Gpt);
        assert!(result.is_ok());
        let review = result.unwrap();
        assert_eq!(review.findings.len(), 2);
        assert_eq!(review.findings[0].severity, ArSeverity::Blocking);
        assert_eq!(review.findings[0].affected_requirements, vec!["FR-1"]);
        assert_eq!(review.findings[1].severity, ArSeverity::Advisory);
    }

    #[test]
    fn parse_review_with_code_fences() {
        let content = "```json\n{\"findings\": [], \"summary\": \"All good.\"}\n```";
        let result = parse_review_response(content, &ArReviewer::Gemini);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_review_unknown_severity_defaults_informational() {
        let content = r#"{
            "findings": [{
                "severity": "note",
                "affected_section": "Out of Scope",
                "affected_requirements": [],
                "description": "Consider adding more exclusions"
            }],
            "summary": "Minor observation."
        }"#;

        let result = parse_review_response(content, &ArReviewer::Opus);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().findings[0].severity, ArSeverity::Informational);
    }

    #[test]
    fn render_spec_for_review_includes_all_sections() {
        let spec = NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: Some("Build a timer".into()),
            sacred_anchors: Some(vec![
                NLSpecAnchor { id: "SA-1".into(), statement: "Never negative".into() },
            ]),
            requirements: vec![
                Requirement {
                    id: "FR-1".into(),
                    statement: "Must count down".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-1".into()],
                },
            ],
            architectural_constraints: vec!["React only".into()],
            phase1_contracts: Some(vec![Phase1Contract {
                name: "TimerState".into(),
                type_definition: "{ remaining: number }".into(),
                consumed_by: vec!["ui".into()],
            }]),
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem { criterion: "Timer works".into(), mechanically_checkable: true },
            ],
            satisfaction_criteria: vec![
                SatisfactionCriterion {
                    id: "SC-1".into(),
                    description: "Counts down to zero".into(),
                    tier_hint: ScenarioTierHint::Critical,
                },
            ],
            open_questions: vec![],
            out_of_scope: vec!["Sound alerts".into()],
            amendment_log: vec![],
        };

        let text = render_spec_for_review(&spec);
        assert!(text.contains("Intent Summary"));
        assert!(text.contains("Sacred Anchors"));
        assert!(text.contains("SA-1: Never negative"));
        assert!(text.contains("FR-1"));
        assert!(text.contains("Phase 1 Contracts"));
        assert!(text.contains("TimerState"));
        assert!(text.contains("Definition of Done"));
        assert!(text.contains("Satisfaction Criteria"));
        assert!(text.contains("Open Questions"));
        assert!(text.contains("(none)"));
        assert!(text.contains("Out of Scope"));
        assert!(text.contains("Sound alerts"));
    }

    #[test]
    fn report_recalculate_counts() {
        let mut report = ArReportV1 {
            project_id: Uuid::new_v4(),
            chunk_name: "root".into(),
            nlspec_version: "1.0".into(),
            findings: vec![
                ArFinding {
                    id: "AR-B-1".into(),
                    reviewer: ArReviewer::Opus,
                    severity: ArSeverity::Blocking,
                    affected_section: "Requirements".into(),
                    affected_requirements: vec!["FR-1".into()],
                    description: "Ambiguous".into(),
                    suggested_resolution: Some("Clarify".into()),
                },
                ArFinding {
                    id: "AR-A-1".into(),
                    reviewer: ArReviewer::Gpt,
                    severity: ArSeverity::Advisory,
                    affected_section: "DoD".into(),
                    affected_requirements: vec![],
                    description: "Consider adding detail".into(),
                    suggested_resolution: None,
                },
                ArFinding {
                    id: "AR-I-1".into(),
                    reviewer: ArReviewer::Gemini,
                    severity: ArSeverity::Informational,
                    affected_section: "Out of Scope".into(),
                    affected_requirements: vec![],
                    description: "Looks fine".into(),
                    suggested_resolution: None,
                },
            ],
            reviewer_summaries: vec![],
            has_blocking: false,
            blocking_count: 0,
            advisory_count: 0,
            informational_count: 0,
        };

        report.recalculate();
        assert!(report.has_blocking);
        assert_eq!(report.blocking_count, 1);
        assert_eq!(report.advisory_count, 1);
        assert_eq!(report.informational_count, 1);
    }

    #[test]
    fn report_no_blocking_findings() {
        let mut report = ArReportV1 {
            project_id: Uuid::new_v4(),
            chunk_name: "root".into(),
            nlspec_version: "1.0".into(),
            findings: vec![
                ArFinding {
                    id: "AR-A-1".into(),
                    reviewer: ArReviewer::Gpt,
                    severity: ArSeverity::Advisory,
                    affected_section: "DoD".into(),
                    affected_requirements: vec![],
                    description: "Minor".into(),
                    suggested_resolution: None,
                },
            ],
            reviewer_summaries: vec![],
            has_blocking: false,
            blocking_count: 0,
            advisory_count: 0,
            informational_count: 0,
        };

        report.recalculate();
        assert!(!report.has_blocking);
        assert_eq!(report.blocking_count, 0);
        assert_eq!(report.advisory_count, 1);
    }
}
