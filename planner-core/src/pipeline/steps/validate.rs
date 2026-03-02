//! # Scenario Validator — Cross-Model Code Review
//!
//! Evaluates the factory worker's generated code against the hidden scenario
//! set using a different model family than the coding agent (Gemini evaluates
//! Codex's code — never the same model family).
//!
//! This is STATIC CODE ANALYSIS, not runtime testing. The evaluator reads
//! source files and judges whether the implementation correctly addresses
//! each BDD scenario based on code structure, logic, and patterns.
//!
//! Phase 1: All tiers evaluated. Each scenario runs 3x, majority pass (2/3).
//! The factory receives only generalized errors (category + severity),
//! never the scenario text.
//!
//! Flow:
//! 1. For each scenario in the ScenarioSetV1
//! 2. Gemini reads the source code files + BDD text
//! 3. Gemini scores 0.0–1.0 per run based on code evidence
//! 4. Majority pass required (2/3 runs with score ≥ 0.5)
//! 5. Tiered gates applied: 100% Critical → 95% High → 90% Medium

use uuid::Uuid;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use crate::dtu::DtuRegistry;
use planner_schemas::*;
use super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// Factory output file reader
// ---------------------------------------------------------------------------

/// Read up to 30 files from the factory output directory, concatenating
/// their contents with file path headers. Caps total size at 100KB.
/// Skips hidden directories (.git, .planner-context, etc.).
///
/// Returns "[Could not read output files]" if the path does not exist.
fn read_factory_files(output_path: &str) -> String {
    const MAX_FILES: usize = 30;
    const MAX_TOTAL_BYTES: usize = 100 * 1024; // 100 KB

    let base = std::path::Path::new(output_path);
    if !base.exists() {
        return "[Could not read output files: path does not exist]".into();
    }

    let mut result = String::new();
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;

    collect_files(base, base, &mut result, &mut file_count, &mut total_bytes, MAX_FILES, MAX_TOTAL_BYTES);

    if result.is_empty() {
        return "[No source files found in output directory]".into();
    }
    result
}

fn collect_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    result: &mut String,
    file_count: &mut usize,
    total_bytes: &mut usize,
    max_files: usize,
    max_bytes: usize,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if *file_count >= max_files || *total_bytes >= max_bytes {
            break;
        }
        let path = entry.path();

        // Skip hidden directories (.git, .planner-context, etc.)
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            collect_files(root, &path, result, file_count, total_bytes, max_files, max_bytes);
        } else {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let rel = path.strip_prefix(root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.to_string_lossy().to_string());
                    let header = format!("\n=== {} ===\n", rel);
                    let available = max_bytes.saturating_sub(*total_bytes);
                    let truncated = if content.len() > available {
                        &content[..available]
                    } else {
                        &content
                    };
                    result.push_str(&header);
                    result.push_str(truncated);
                    *total_bytes += header.len() + truncated.len();
                    *file_count += 1;
                }
                Err(_) => {} // Skip unreadable files
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation prompt
// ---------------------------------------------------------------------------

const EVALUATOR_SYSTEM_PROMPT: &str = r#"You are the Scenario Validator for Planner v2. You evaluate whether source code correctly implements BDD scenarios through static code analysis.

## Your Role
You are a DIFFERENT model family from the coding agent. You judge the code objectively through code review, preventing shared blind spots.

## Important: Static Analysis, Not Runtime Testing
You are reviewing SOURCE CODE, not running the application. You CANNOT test runtime behavior.
Instead, you evaluate:
1. Does the code contain the logic/components needed for this scenario?
2. Are event handlers, state management, and rendering paths present?
3. Does the implementation approach match what the scenario requires?
4. Are there obvious bugs that would prevent the scenario from working?

## Scoring Guidelines
- 1.0 = Code clearly implements everything the scenario needs. Logic is correct.
- 0.7-0.9 = Code has the right structure and most logic, minor gaps or edge cases uncertain.
- 0.5-0.6 = Partial implementation exists but key pieces are missing or questionable.
- 0.2-0.4 = Some relevant code exists but the scenario's core behavior is not properly implemented.
- 0.0-0.1 = No relevant code found, or build failed entirely, or the code is fundamentally broken.

## Common Mistake to Avoid
Do NOT score 0.0 just because you "cannot run the code." You CAN read it. If a scenario says
"clicking Add creates a task" and you see an onClick handler that pushes to a task array and
re-renders, that's a 0.8-1.0, not a 0.0.

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "score": 0.0 to 1.0,
  "passed": true|false,
  "reasoning": "Brief explanation citing specific code evidence",
  "error_category": "category-name" or null,
  "error_severity": "Critical"|"High"|"Medium"|"Low" or null
}

## Rules
1. If the build failed entirely, score 0.0 for all scenarios.
2. Score >= 0.5 counts as a "pass" for majority voting.
3. Cite specific files, functions, or code patterns in your reasoning.
4. error_category should be kebab-case (e.g., "missing-handler", "state-management").
5. Only set error fields if the scenario did NOT pass."#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate all scenarios against the factory output.
///
/// All tiers (Critical, High, Medium). Each scenario runs 3 times.
/// Returns a SatisfactionResultV1 with tiered pass rates.
///
/// `dtu_registry` — if `Some`, DTU clone context is included in the
/// evaluation prompt so the validator knows which providers are
/// available for request routing during sandbox evaluation.
pub async fn execute_scenario_validation(
    router: &LlmRouter,
    scenarios: &ScenarioSetV1,
    factory_output: &FactoryOutputV1,
    dtu_registry: Option<&DtuRegistry>,
) -> StepResult<SatisfactionResultV1> {
    tracing::info!(
        "Scenario Validator: evaluating {} scenarios against {}",
        scenarios.scenarios.len(),
        factory_output.output_path,
    );
    tracing::info!(
        "Scenario Validator: build_status={:?}, factory_attempt={}",
        factory_output.build_status,
        factory_output.attempt,
    );

    // If DTU clones are available, log them so evaluators can reference
    // provider state during scenario evaluation.
    if let Some(dtu_reg) = dtu_registry {
        let providers = dtu_reg.list_providers();
        if !providers.is_empty() {
            let dtu_context: Vec<String> = providers
                .iter()
                .map(|p| {
                    format!(
                        "  - {} ({}): endpoints={:?}",
                        p.name, p.id, p.supported_endpoints
                    )
                })
                .collect();
            tracing::info!(
                "Scenario Validator: {} DTU clone(s) available:\n{}",
                providers.len(),
                dtu_context.join("\n"),
            );
        }
    }

    // If the build failed entirely, short-circuit with all failures
    if factory_output.build_status == BuildStatus::Failed {
        tracing::warn!("Build failed — all scenarios automatically fail");
        return Ok(build_all_failed_result(
            factory_output.kilroy_run_id,
            scenarios,
        ));
    }

    let mut scenario_results = Vec::new();

    for scenario in &scenarios.scenarios {
        tracing::info!(
            "  Evaluating scenario {} [{}] — {}",
            scenario.id,
            format!("{:?}", scenario.tier),
            scenario.title,
        );

        let result = evaluate_single_scenario(
            router,
            scenario,
            factory_output,
        )
        .await?;

        tracing::info!(
            "    → score={:.2}, majority_pass={}, runs=[{:.2}, {:.2}, {:.2}]",
            result.score,
            result.majority_pass,
            result.runs[0],
            result.runs[1],
            result.runs[2],
        );

        scenario_results.push(result);
    }

    // Calculate tiered pass rates
    let critical_results: Vec<&ScenarioResult> = scenario_results
        .iter()
        .filter(|r| r.tier == ScenarioTier::Critical)
        .collect();
    let high_results: Vec<&ScenarioResult> = scenario_results
        .iter()
        .filter(|r| r.tier == ScenarioTier::High)
        .collect();
    let medium_results: Vec<&ScenarioResult> = scenario_results
        .iter()
        .filter(|r| r.tier == ScenarioTier::Medium)
        .collect();

    let critical_pass_rate = if critical_results.is_empty() {
        1.0 // No critical scenarios = pass
    } else {
        let passed = critical_results.iter().filter(|r| r.majority_pass).count();
        passed as f32 / critical_results.len() as f32
    };

    let high_pass_rate = if high_results.is_empty() {
        1.0
    } else {
        let passed = high_results.iter().filter(|r| r.majority_pass).count();
        passed as f32 / high_results.len() as f32
    };

    let medium_pass_rate = if medium_results.is_empty() {
        1.0
    } else {
        let passed = medium_results.iter().filter(|r| r.majority_pass).count();
        passed as f32 / medium_results.len() as f32
    };

    let result = SatisfactionResultV1 {
        kilroy_run_id: factory_output.kilroy_run_id,
        critical_pass_rate,
        high_pass_rate,
        medium_pass_rate,
        gates_passed: critical_pass_rate >= 1.0
            && high_pass_rate >= 0.95
            && medium_pass_rate >= 0.90,
        scenario_results,
    };

    tracing::info!(
        "Scenario Validator complete: critical={:.0}%, high={:.0}%, medium={:.0}% — gates={}",
        result.critical_pass_rate * 100.0,
        result.high_pass_rate * 100.0,
        result.medium_pass_rate * 100.0,
        if result.gates_passed { "PASSED" } else { "FAILED" },
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Single scenario evaluation (3x runs)
// ---------------------------------------------------------------------------

/// Evaluate a single scenario 3 times and compute majority pass.
async fn evaluate_single_scenario(
    router: &LlmRouter,
    scenario: &Scenario,
    factory_output: &FactoryOutputV1,
) -> StepResult<ScenarioResult> {
    let mut runs = [0.0f32; 3];
    let mut last_error_category: Option<String> = None;
    let mut last_error_severity: Option<Severity> = None;

    for run_idx in 0..3 {
        let eval = evaluate_scenario_once(
            router,
            scenario,
            factory_output,
            run_idx + 1,
        )
        .await?;

        runs[run_idx] = eval.score;

        if let Some(cat) = eval.error_category {
            last_error_category = Some(cat);
        }
        if let Some(sev) = eval.error_severity {
            last_error_severity = Some(sev);
        }
    }

    let majority_pass = ScenarioResult::compute_majority_pass(&runs);
    let score = runs.iter().sum::<f32>() / 3.0;

    let generalized_error = if !majority_pass {
        Some(GeneralizedError {
            category: last_error_category.unwrap_or_else(|| "unknown".into()),
            severity: last_error_severity.unwrap_or(Severity::Medium),
        })
    } else {
        None
    };

    Ok(ScenarioResult {
        scenario_id: scenario.id.clone(),
        tier: scenario.tier.clone(),
        runs,
        majority_pass,
        score,
        generalized_error,
    })
}

/// Single evaluation result from the LLM.
struct SingleEvalResult {
    score: f32,
    #[allow(dead_code)]
    passed: bool,
    error_category: Option<String>,
    error_severity: Option<Severity>,
}

/// Maximum retries for a single evaluation LLM call.
const EVAL_MAX_RETRIES: usize = 2;

/// Run one evaluation of a scenario against the factory output.
/// Retries up to EVAL_MAX_RETRIES times on LLM or parse failures.
async fn evaluate_scenario_once(
    router: &LlmRouter,
    scenario: &Scenario,
    factory_output: &FactoryOutputV1,
    _run_number: usize,
) -> StepResult<SingleEvalResult> {
    let source_files = read_factory_files(&factory_output.output_path);

    // Log source file stats for every scenario
    let file_count = source_files.matches("=== ").count();
    tracing::info!(
        "    Source files for evaluation: {} files, {} bytes",
        file_count, source_files.len()
    );

    let mut last_error = None;

    for attempt in 0..=EVAL_MAX_RETRIES {
        if attempt > 0 {
            tracing::warn!(
                "    Retrying evaluation for {} (attempt {}/{})",
                scenario.id, attempt + 1, EVAL_MAX_RETRIES + 1,
            );
        }

        let request = CompletionRequest {
            system: Some(EVALUATOR_SYSTEM_PROMPT.to_string()),
            messages: vec![Message {
                role: Role::User,
                content: format!(
                    "Review the source code below and evaluate whether it correctly implements this BDD scenario.\n\nScenario: {} [{}]\n{}\n\nBuild status: {}\n\n## Source Code\n\n{}",
                    scenario.title,
                    format!("{:?}", scenario.tier),
                    scenario.bdd_text,
                    format!("{:?}", factory_output.build_status),
                    source_files,
                ),
            }],
            max_tokens: 1024,
            temperature: 0.3, // Some variation across runs is desired
            model: DefaultModels::SCENARIO_VALIDATOR.to_string(),
        };

        match router.complete(request).await {
            Ok(response) => {
                tracing::debug!(
                    "    Gemini response for {} (attempt {}): {}",
                    scenario.id,
                    attempt + 1,
                    &response.content[..response.content.len().min(500)]
                );
                match parse_eval_response(&response.content) {
                    Ok(result) => {
                        tracing::info!(
                            "    {} run {}: score={:.2}, passed={}",
                            scenario.id, attempt + 1, result.score, result.passed
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "    Parse error on attempt {} for {}: {}. Raw response (first 300 chars): {}",
                            attempt + 1, scenario.id, e,
                            &response.content[..response.content.len().min(300)]
                        );
                        last_error = Some(e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("    LLM error on attempt {}: {}", attempt + 1, e);
                last_error = Some(StepError::LlmError(e.to_string()));
            }
        }
    }

    // All retries exhausted — return the last error
    Err(last_error.unwrap_or_else(|| StepError::Other("Evaluation failed after retries".into())))
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct EvalJson {
    score: f32,
    #[serde(default)]
    passed: bool,
    #[allow(dead_code)]
    #[serde(default)]
    reasoning: String,
    error_category: Option<String>,
    error_severity: Option<String>,
}

fn parse_eval_response(content: &str) -> StepResult<SingleEvalResult> {
    let cleaned = crate::llm::json_repair::try_repair_json(content)
        .unwrap_or_else(|| super::intake::strip_code_fences(content));

    let json: EvalJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse evaluator response: {}. Raw: {}",
            e,
            &content[..content.len().min(300)]
        ))
    })?;

    let error_severity = json.error_severity.and_then(|s| {
        match s.to_lowercase().as_str() {
            "critical" => Some(Severity::Critical),
            "high" => Some(Severity::High),
            "medium" => Some(Severity::Medium),
            "low" => Some(Severity::Low),
            _ => None,
        }
    });

    Ok(SingleEvalResult {
        score: json.score.clamp(0.0, 1.0),
        passed: json.passed || json.score >= 0.5,
        error_category: json.error_category,
        error_severity,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a result where all scenarios fail (used when build itself failed).
fn build_all_failed_result(
    kilroy_run_id: Uuid,
    scenarios: &ScenarioSetV1,
) -> SatisfactionResultV1 {
    let scenario_results: Vec<ScenarioResult> = scenarios
        .scenarios
        .iter()
        .map(|s| ScenarioResult {
            scenario_id: s.id.clone(),
            tier: s.tier.clone(),
            runs: [0.0, 0.0, 0.0],
            majority_pass: false,
            score: 0.0,
            generalized_error: Some(GeneralizedError {
                category: "build-failure".into(),
                severity: Severity::Critical,
            }),
        })
        .collect();

    SatisfactionResultV1 {
        kilroy_run_id,
        critical_pass_rate: 0.0,
        high_pass_rate: 0.0,
        medium_pass_rate: 0.0,
        gates_passed: false,
        scenario_results,
    }
}

// ---------------------------------------------------------------------------
// DoD Mechanical Checker
// ---------------------------------------------------------------------------

/// Result of checking a single Definition of Done item.
#[derive(Debug, Clone)]
pub struct DoDCheckResult {
    /// The DoD criterion text.
    pub criterion: String,
    /// Whether this item passed verification.
    pub passed: bool,
    /// How it was checked: "mechanical" (code-verified) or "manual" (assumed pass).
    pub check_method: String,
    /// Details about the check.
    pub detail: String,
}

/// Mechanically check DoD items against the factory output.
///
/// Items marked `mechanically_checkable: true` are checked against the
/// build results. Items marked `mechanically_checkable: false` are
/// marked as needing manual review (assumed pass for now).
pub fn check_definition_of_done(
    spec: &NLSpecV1,
    factory_output: &FactoryOutputV1,
    satisfaction: &SatisfactionResultV1,
) -> Vec<DoDCheckResult> {
    spec.definition_of_done.iter().map(|dod| {
        if !dod.mechanically_checkable {
            return DoDCheckResult {
                criterion: dod.criterion.clone(),
                passed: true, // Assume pass for non-mechanical items
                check_method: "manual".into(),
                detail: "Requires manual review — assumed pass.".into(),
            };
        }

        // Mechanical checks based on factory output state
        let criterion_lower = dod.criterion.to_lowercase();

        // Check: Build succeeds
        if criterion_lower.contains("build") || criterion_lower.contains("compile") {
            let passed = factory_output.build_status == BuildStatus::Success
                || factory_output.build_status == BuildStatus::PartialSuccess;
            return DoDCheckResult {
                criterion: dod.criterion.clone(),
                passed,
                check_method: "mechanical".into(),
                detail: format!("Build status: {:?}", factory_output.build_status),
            };
        }

        // Check: Tests/scenarios pass
        if criterion_lower.contains("test") || criterion_lower.contains("scenario")
            || criterion_lower.contains("pass")
        {
            return DoDCheckResult {
                criterion: dod.criterion.clone(),
                passed: satisfaction.gates_passed,
                check_method: "mechanical".into(),
                detail: format!(
                    "Gates: critical={:.0}%, high={:.0}%, medium={:.0}%",
                    satisfaction.critical_pass_rate * 100.0,
                    satisfaction.high_pass_rate * 100.0,
                    satisfaction.medium_pass_rate * 100.0,
                ),
            };
        }

        // Check: Persist/save/store keywords → look at scenario results
        if criterion_lower.contains("persist") || criterion_lower.contains("save")
            || criterion_lower.contains("store") || criterion_lower.contains("data")
        {
            // Check if any scenario about data persistence passed
            let data_scenarios_pass = satisfaction.scenario_results.iter()
                .filter(|r| {
                    let id_lower = r.scenario_id.to_lowercase();
                    id_lower.contains("persist") || id_lower.contains("data")
                        || id_lower.contains("save")
                })
                .all(|r| r.majority_pass);

            // If no specific scenarios found, fall back to critical pass rate
            let passed = if satisfaction.scenario_results.iter()
                .any(|r| r.scenario_id.to_lowercase().contains("persist")
                    || r.scenario_id.to_lowercase().contains("data"))
            {
                data_scenarios_pass
            } else {
                satisfaction.critical_pass_rate >= 1.0
            };

            return DoDCheckResult {
                criterion: dod.criterion.clone(),
                passed,
                check_method: "mechanical".into(),
                detail: "Checked via data-related scenario results.".into(),
            };
        }

        // Default: use overall gate result for mechanically-checkable items
        DoDCheckResult {
            criterion: dod.criterion.clone(),
            passed: satisfaction.gates_passed,
            check_method: "mechanical".into(),
            detail: "Checked via overall gate result.".into(),
        }
    }).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_eval_response() {
        let content = r#"{"score": 0.85, "passed": true, "reasoning": "Looks good", "error_category": null, "error_severity": null}"#;
        let result = parse_eval_response(content);
        assert!(result.is_ok());
        let eval = result.unwrap();
        assert!((eval.score - 0.85).abs() < 0.01);
        assert!(eval.error_category.is_none());
    }

    #[test]
    fn parse_failed_eval_response() {
        let content = r#"{"score": 0.2, "passed": false, "reasoning": "Data not persisted", "error_category": "data-persistence", "error_severity": "Critical"}"#;
        let result = parse_eval_response(content);
        assert!(result.is_ok());
        let eval = result.unwrap();
        assert!(eval.score < 0.5);
        assert_eq!(eval.error_category.unwrap(), "data-persistence");
        assert_eq!(eval.error_severity.unwrap(), Severity::Critical);
    }

    #[test]
    fn parse_eval_with_code_fences() {
        let content = "```json\n{\"score\": 1.0, \"passed\": true, \"reasoning\": \"Perfect\"}\n```";
        let result = parse_eval_response(content);
        assert!(result.is_ok());
        assert!((result.unwrap().score - 1.0).abs() < 0.01);
    }

    #[test]
    fn score_clamped_to_range() {
        let content = r#"{"score": 1.5, "passed": true, "reasoning": "Over-scored"}"#;
        let result = parse_eval_response(content);
        assert!(result.is_ok());
        assert!((result.unwrap().score - 1.0).abs() < 0.01);
    }

    #[test]
    fn build_all_failed_result_zeros_pass_rates() {
        let scenarios = ScenarioSetV1 {
            project_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            scenarios: vec![
                Scenario {
                    id: "SC-CRIT-1".into(),
                    tier: ScenarioTier::Critical,
                    title: "Test".into(),
                    bdd_text: "Given/When/Then".into(),
                    dtu_deps: vec![],
                    traces_to_anchors: vec![],
                    source_criterion: None,
                },
            ],
            isolation_context_id: Uuid::new_v4(),
            ralph_augmented: false,
        };

        let result = build_all_failed_result(Uuid::new_v4(), &scenarios);
        assert_eq!(result.critical_pass_rate, 0.0);
        assert!(!result.gates_passed);
        assert_eq!(result.scenario_results.len(), 1);
        assert!(!result.scenario_results[0].majority_pass);
    }

    #[test]
    fn tiered_gates_logic() {
        // All passing
        let result = SatisfactionResultV1 {
            kilroy_run_id: Uuid::new_v4(),
            critical_pass_rate: 1.0,
            high_pass_rate: 0.96,
            medium_pass_rate: 0.92,
            gates_passed: true,
            scenario_results: vec![],
        };
        assert!(result.evaluate_gates());

        // Critical failing
        let result2 = SatisfactionResultV1 {
            critical_pass_rate: 0.5,
            ..result.clone()
        };
        assert!(!result2.evaluate_gates());

        // High below threshold
        let result3 = SatisfactionResultV1 {
            high_pass_rate: 0.90,
            ..result.clone()
        };
        assert!(!result3.evaluate_gates());
    }

    #[test]
    fn user_messages_by_tier() {
        let all_pass = SatisfactionResultV1 {
            kilroy_run_id: Uuid::new_v4(),
            critical_pass_rate: 1.0,
            high_pass_rate: 1.0,
            medium_pass_rate: 1.0,
            gates_passed: true,
            scenario_results: vec![],
        };
        assert_eq!(all_pass.user_message(), "Everything works as described.");

        let medium_low = SatisfactionResultV1 {
            medium_pass_rate: 0.8,
            ..all_pass.clone()
        };
        assert!(medium_low.user_message().contains("minor behaviors"));

        let critical_fail = SatisfactionResultV1 {
            critical_pass_rate: 0.5,
            ..all_pass.clone()
        };
        assert!(critical_fail.user_message().contains("critical"));
    }

    #[test]
    fn dod_checker_mechanical_build_pass() {
        let spec = NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: None,
            sacred_anchors: None,
            requirements: vec![],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem {
                    criterion: "Build compiles without errors".into(),
                    mechanically_checkable: true,
                },
            ],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        let factory_output = FactoryOutputV1 {
            kilroy_run_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            attempt: 1,
            build_status: BuildStatus::Success,
            spend_usd: 0.5,
            checkpoint_path: "/tmp/cp.json".into(),
            dod_results: vec![],
            node_results: vec![],
            output_path: "/tmp/out".into(),
        };

        let satisfaction = SatisfactionResultV1 {
            kilroy_run_id: factory_output.kilroy_run_id,
            critical_pass_rate: 1.0,
            high_pass_rate: 1.0,
            medium_pass_rate: 1.0,
            gates_passed: true,
            scenario_results: vec![],
        };

        let results = check_definition_of_done(&spec, &factory_output, &satisfaction);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert_eq!(results[0].check_method, "mechanical");
    }

    #[test]
    fn dod_checker_mechanical_build_fail() {
        let spec = NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: None,
            sacred_anchors: None,
            requirements: vec![],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem {
                    criterion: "Build compiles without errors".into(),
                    mechanically_checkable: true,
                },
            ],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        let factory_output = FactoryOutputV1 {
            kilroy_run_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            attempt: 1,
            build_status: BuildStatus::Failed,
            spend_usd: 0.5,
            checkpoint_path: "/tmp/cp.json".into(),
            dod_results: vec![],
            node_results: vec![],
            output_path: "/tmp/out".into(),
        };

        let satisfaction = SatisfactionResultV1 {
            kilroy_run_id: factory_output.kilroy_run_id,
            critical_pass_rate: 0.0,
            high_pass_rate: 0.0,
            medium_pass_rate: 0.0,
            gates_passed: false,
            scenario_results: vec![],
        };

        let results = check_definition_of_done(&spec, &factory_output, &satisfaction);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert_eq!(results[0].check_method, "mechanical");
    }

    #[test]
    fn dod_checker_manual_item_assumed_pass() {
        let spec = NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: None,
            sacred_anchors: None,
            requirements: vec![],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem {
                    criterion: "Code is clean and readable".into(),
                    mechanically_checkable: false,
                },
            ],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        let factory_output = FactoryOutputV1 {
            kilroy_run_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            attempt: 1,
            build_status: BuildStatus::Success,
            spend_usd: 0.5,
            checkpoint_path: "/tmp/cp.json".into(),
            dod_results: vec![],
            node_results: vec![],
            output_path: "/tmp/out".into(),
        };

        let satisfaction = SatisfactionResultV1 {
            kilroy_run_id: factory_output.kilroy_run_id,
            critical_pass_rate: 1.0,
            high_pass_rate: 1.0,
            medium_pass_rate: 1.0,
            gates_passed: true,
            scenario_results: vec![],
        };

        let results = check_definition_of_done(&spec, &factory_output, &satisfaction);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert_eq!(results[0].check_method, "manual");
    }

    #[test]
    fn dod_checker_test_criterion_checks_gates() {
        let spec = NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: None,
            sacred_anchors: None,
            requirements: vec![],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem {
                    criterion: "All scenarios pass their tests".into(),
                    mechanically_checkable: true,
                },
            ],
            satisfaction_criteria: vec![],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        };

        let factory_output = FactoryOutputV1 {
            kilroy_run_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            attempt: 1,
            build_status: BuildStatus::Success,
            spend_usd: 0.5,
            checkpoint_path: "/tmp/cp.json".into(),
            dod_results: vec![],
            node_results: vec![],
            output_path: "/tmp/out".into(),
        };

        // Gates pass
        let satisfaction_pass = SatisfactionResultV1 {
            kilroy_run_id: factory_output.kilroy_run_id,
            critical_pass_rate: 1.0,
            high_pass_rate: 0.96,
            medium_pass_rate: 0.92,
            gates_passed: true,
            scenario_results: vec![],
        };
        let results = check_definition_of_done(&spec, &factory_output, &satisfaction_pass);
        assert!(results[0].passed);

        // Gates fail
        let satisfaction_fail = SatisfactionResultV1 {
            kilroy_run_id: factory_output.kilroy_run_id,
            critical_pass_rate: 0.5,
            high_pass_rate: 0.5,
            medium_pass_rate: 0.5,
            gates_passed: false,
            scenario_results: vec![],
        };
        let results = check_definition_of_done(&spec, &factory_output, &satisfaction_fail);
        assert!(!results[0].passed);
    }
}
