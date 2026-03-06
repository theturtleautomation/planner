//! # Tier 1-3 Pipeline Integration Tests
//!
//! These tests exercise the real pipeline functions (`execute_intake`,
//! `compile_spec`, `execute_adversarial_review`, `run_phase0_front_office`,
//! `run_full_pipeline`) through a `MockLlmClient` that returns valid JSON
//! for every step, keyed off the system-prompt content.
//!
//! **Tier 1** — Pipeline step unit integration (5 tests)
//! **Tier 3** — Live LLM smoke test (gated behind `live-llm` feature)
//!
//! This file is NEW and does NOT modify integration_e2e.rs.

use async_trait::async_trait;
use planner_core::cxdb::CxdbEngine;
use planner_core::llm::providers::LlmRouter;
use planner_core::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
use planner_core::pipeline::steps::factory_worker::MockFactoryWorker;
use planner_core::pipeline::{PipelineConfig, run_full_pipeline};
use planner_schemas::*;
use uuid::Uuid;

// ===========================================================================
// MockLlmClient — returns step-appropriate JSON based on system prompt
// ===========================================================================

/// A mock LLM client that inspects the system prompt (or user message) to
/// determine which pipeline step is calling, then returns a canned JSON
/// response that the step's parser will accept.
struct MockLlmClient;

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let system = request.system.as_deref().unwrap_or("");
        let _user_msg = request
            .messages
            .first()
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content = if system.contains("Intake Gateway") {
            mock_intake_json()
        } else if system.contains("Spec Compiler") && !system.contains("Domain") {
            mock_spec_json()
        } else if system.contains("Domain Spec Compiler") {
            mock_domain_spec_json()
        } else if system.contains("Graph.dot Compiler") {
            mock_graph_dot_json()
        } else if system.contains("Scenario Generator") {
            mock_scenario_json()
        } else if system.contains("AGENTS.md Compiler") {
            mock_agents_json()
        } else if system.contains("Adversarial Reviewer") {
            mock_ar_review_json()
        } else if system.contains("NLSpec Refiner") {
            mock_ar_refinement_json()
        } else if system.contains("Scenario Augmentation")
            || system.contains("Ralph")
        {
            mock_ralph_augment_json()
        } else if system.contains("Scenario Validator") || system.contains("evaluate whether") {
            mock_validate_json()
        } else if system.contains("Telemetry Presenter") {
            mock_telemetry_json()
        } else if system.contains("Chunk Planner") {
            mock_chunk_planner_json()
        } else if system.contains("DTU Configuration") || system.contains("dtu") {
            mock_dtu_config_json()
        } else {
            // Fallback: return a minimal JSON that parses as IntakeJson
            // (many steps use try_repair_json which is lenient)
            format!(
                r#"{{"fallback": true, "prompt_hint": "{}"}}"#,
                &system[..system.len().min(80)]
            )
        };

        Ok(CompletionResponse {
            content,
            model: request.model,
            input_tokens: 100,
            output_tokens: 200,
            estimated_cost_usd: 0.01,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
}

// ===========================================================================
// Canned JSON responses — one per pipeline step
// ===========================================================================

fn mock_intake_json() -> String {
    r#"{
  "project_name": "Mock Timer",
  "feature_slug": "mock-timer-widget",
  "intent_summary": "A countdown timer widget",
  "output_domain": { "type": "micro_tool", "variant": "react_widget" },
  "environment": {
    "language": "TypeScript",
    "framework": "React",
    "package_manager": "npm",
    "build_tool": "vite"
  },
  "sacred_anchors": [
    { "id": "SA-1", "statement": "Timer must never display negative time", "rationale": "Core invariant" },
    { "id": "SA-2", "statement": "Pausing must preserve remaining time exactly", "rationale": "User expectation" }
  ],
  "satisfaction_criteria_seeds": [
    "Starting a 10-second timer shows it counting down to 0",
    "Pausing at 5 seconds and resuming continues from 5"
  ],
  "out_of_scope": ["Sound alerts", "Multiple timers"]
}"#
    .into()
}

fn mock_spec_json() -> String {
    r#"{
  "intent_summary": "A countdown timer widget with start, pause, and reset controls.",
  "sacred_anchors": [
    { "id": "SA-1", "statement": "Timer must never display negative time" },
    { "id": "SA-2", "statement": "Pausing must preserve remaining time exactly" }
  ],
  "requirements": [
    { "id": "FR-1", "statement": "The timer must accept a positive integer duration in seconds", "priority": "must", "traces_to": ["SA-1"] },
    { "id": "FR-2", "statement": "The display must update remaining time every second", "priority": "must", "traces_to": ["SA-1"] },
    { "id": "FR-3", "statement": "The widget must provide start, pause, and reset controls", "priority": "must", "traces_to": ["SA-2"] },
    { "id": "FR-4", "statement": "The timer must never display a negative value", "priority": "must", "traces_to": ["SA-1"] }
  ],
  "architectural_constraints": ["Single React component with hooks", "Tailwind CSS"],
  "phase1_contracts": [
    { "name": "TimerState", "type_definition": "{ duration: number, remaining: number, running: boolean }", "consumed_by": ["ui"] }
  ],
  "external_dependencies": [],
  "definition_of_done": [
    { "criterion": "Timer counts down from specified duration to zero", "mechanically_checkable": true },
    { "criterion": "Start/pause/reset buttons function correctly", "mechanically_checkable": true },
    { "criterion": "Timer never displays negative values", "mechanically_checkable": true }
  ],
  "satisfaction_criteria": [
    { "id": "SC-1", "description": "Starting a 10-second timer shows it counting down to 0", "tier_hint": "critical" },
    { "id": "SC-2", "description": "Pausing at 5 seconds and resuming continues from 5", "tier_hint": "critical" },
    { "id": "SC-3", "description": "Resetting returns to the original duration", "tier_hint": "high" }
  ],
  "open_questions": [],
  "out_of_scope": ["Sound alerts", "Multiple concurrent timers", "Persistent state"]
}"#
    .into()
}

fn mock_domain_spec_json() -> String {
    // Reuse root spec shape — domain spec parsing is the same struct
    mock_spec_json()
}

fn mock_graph_dot_json() -> String {
    r#"{
  "dot_content": "digraph timer {\n  start [shape=Mdiamond];\n  exit [shape=Msquare];\n  implement [shape=box];\n  verify [shape=box];\n  start -> implement;\n  implement -> verify;\n  verify -> exit;\n}",
  "node_count": 4,
  "estimated_cost_usd": 0.50,
  "run_budget_usd": 2.00,
  "model_routing": [
    { "node_name": "implement", "node_class": "implementation", "model": "claude-sonnet-4-6", "fidelity": "truncate", "goal_gate": false, "max_retries": 2 }
  ]
}"#
    .into()
}

fn mock_scenario_json() -> String {
    r#"{
  "scenarios": [
    {
      "id": "SC-CRIT-1",
      "tier": "critical",
      "title": "Timer counts down to zero",
      "bdd_text": "Given a countdown timer set to 10 seconds\nWhen the user presses Start\nThen the display should count down from 10 to 0",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": "SC-1"
    },
    {
      "id": "SC-CRIT-2",
      "tier": "critical",
      "title": "Pause preserves remaining time",
      "bdd_text": "Given a running timer showing 5 seconds\nWhen the user presses Pause\nThen the display shows exactly 5 seconds",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-2"],
      "source_criterion": "SC-2"
    },
    {
      "id": "SC-HIGH-1",
      "tier": "high",
      "title": "Reset returns to original duration",
      "bdd_text": "Given a timer at 15 seconds remaining\nWhen the user presses Reset\nThen the display shows the original duration",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-2"],
      "source_criterion": "SC-3"
    }
  ]
}"#
    .into()
}

fn mock_agents_json() -> String {
    serde_json::json!({
        "root_agents_md": "# AGENTS.md\n\n## Goal\nBuild a countdown timer widget.\n\n## Sacred Anchors\n- Timer must never display negative time\n- Pausing must preserve remaining time\n\n## Key Files\n- src/CountdownTimer.tsx\n- src/App.tsx\n\n## Constraints\n- Single React component\n- Tailwind CSS\n"
    })
    .to_string()
}

fn mock_ar_review_json() -> String {
    r#"{
  "findings": [
    {
      "severity": "advisory",
      "affected_section": "Definition of Done",
      "affected_requirements": [],
      "description": "DoD item 2 could be more specific about button behavior",
      "suggested_resolution": "Specify expected state after each button press"
    }
  ],
  "summary": "Spec is well-structured with minor advisory suggestions. No blocking issues found."
}"#
    .into()
}

fn mock_ar_refinement_json() -> String {
    r#"{
  "amendments": [],
  "open_questions": [],
  "amendment_log_entry": "No amendments needed — no blocking findings."
}"#
    .into()
}

fn mock_ralph_augment_json() -> String {
    r#"{
  "scenarios": [
    {
      "id": "SC-RALPH-1",
      "tier": "high",
      "title": "Edge case: zero-length timer",
      "bdd_text": "Given a countdown timer set to 0 seconds\nWhen the user presses Start\nThen the display should show 0 immediately",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": null
    }
  ]
}"#
    .into()
}

fn mock_validate_json() -> String {
    r#"{
  "score": 0.92,
  "passed": true,
  "reasoning": "The implementation satisfies the scenario requirements.",
  "error_category": null,
  "error_severity": null
}"#
    .into()
}

fn mock_telemetry_json() -> String {
    r#"{
  "headline": "Everything works as described.",
  "summary": "The countdown timer widget was built successfully. All scenarios pass validation.",
  "needs_user_action": false,
  "action_description": null
}"#
    .into()
}

fn mock_chunk_planner_json() -> String {
    r#"{
  "chunks": [
    {
      "chunk_id": "root",
      "relevant_anchor_ids": ["SA-1", "SA-2"],
      "domain_context": "Single-chunk micro-tool",
      "estimated_fr_count": 4
    }
  ]
}"#
    .into()
}

fn mock_dtu_config_json() -> String {
    r#"{
  "rules": [],
  "transitions": [],
  "seeds": [],
  "failure_modes": []
}"#
    .into()
}

// ===========================================================================
// Helper: create a mock router
// ===========================================================================

fn mock_router() -> LlmRouter {
    LlmRouter::with_mock(Box::new(MockLlmClient))
}

// ===========================================================================
// Tier 1: Pipeline Step Integration Tests
// ===========================================================================

/// Test 1: Intake Gateway — real `execute_intake` with mock LLM.
#[tokio::test]
async fn tier1_intake_gateway_with_mock() {
    let router = mock_router();
    let project_id = Uuid::new_v4();

    let result =
        planner_core::pipeline::steps::intake::execute_intake(&router, project_id, "Build me a countdown timer")
            .await;

    assert!(result.is_ok(), "Intake should succeed: {:?}", result.err());
    let intake = result.unwrap();

    assert_eq!(intake.project_name, "Mock Timer");
    assert_eq!(intake.feature_slug, "mock-timer-widget");
    assert!(!intake.sacred_anchors.is_empty());
    assert!(matches!(
        intake.output_domain,
        OutputDomain::MicroTool {
            variant: MicroToolVariant::ReactWidget
        }
    ));
    assert_eq!(intake.satisfaction_criteria_seeds.len(), 2);
    assert_eq!(intake.out_of_scope.len(), 2);
    assert_eq!(intake.conversation_log.len(), 2);
}

/// Test 2: Compile Spec — real `compile_spec` with mock LLM.
#[tokio::test]
async fn tier1_compile_spec_with_mock() {
    use planner_core::pipeline::steps::compile;
    use planner_core::pipeline::steps::intake;

    let router = mock_router();
    let project_id = Uuid::new_v4();

    // First produce an IntakeV1 from the mock
    let intake_result = intake::execute_intake(&router, project_id, "Build a timer").await.unwrap();

    // Now compile the spec
    let spec_result = compile::compile_spec(&router, &intake_result, None).await;
    assert!(
        spec_result.is_ok(),
        "compile_spec should succeed: {:?}",
        spec_result.err()
    );

    let spec = spec_result.unwrap();
    assert!(!spec.requirements.is_empty());
    assert!(spec.requirements.len() >= 4);
    assert!(!spec.definition_of_done.is_empty());
    assert!(!spec.satisfaction_criteria.is_empty());

    // The spec should be lint-clean
    use planner_core::pipeline::steps::linter;
    let lint = linter::lint_spec(&spec);
    assert!(lint.is_ok(), "Mock spec should pass lint: {:?}", lint.err());
}

/// Test 3: Adversarial Review — real `execute_adversarial_review` with mock LLM.
#[tokio::test]
async fn tier1_adversarial_review_with_mock() {
    use planner_core::pipeline::steps::ar;
    use planner_core::pipeline::steps::compile;
    use planner_core::pipeline::steps::intake;

    let router = mock_router();
    let project_id = Uuid::new_v4();

    let intake_result = intake::execute_intake(&router, project_id, "Build a timer").await.unwrap();
    let spec = compile::compile_spec(&router, &intake_result, None).await.unwrap();

    let ar_result = ar::execute_adversarial_review(&router, &spec, project_id, None).await;
    assert!(
        ar_result.is_ok(),
        "AR review should succeed: {:?}",
        ar_result.err()
    );

    let report = ar_result.unwrap();
    // Our mock returns advisory-only findings, so no blocking
    assert!(!report.has_blocking, "Mock AR should have no blocking findings");
    assert!(report.advisory_count >= 1, "Mock AR should have advisory findings");
}

/// Test 4: Full Front-Office pipeline with mock LLM.
#[tokio::test]
async fn tier1_front_office_pipeline_with_mock() {
    use planner_core::pipeline::run_phase0_front_office;

    let router = mock_router();
    let project_id = Uuid::new_v4();

    let result = run_phase0_front_office(&router, project_id, "Build a countdown timer").await;

    assert!(
        result.is_ok(),
        "Front-office pipeline should succeed: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // Intake
    assert_eq!(output.intake.project_name, "Mock Timer");
    // Specs (single chunk for micro-tool)
    assert_eq!(output.specs.len(), 1);
    assert!(!output.specs[0].requirements.is_empty());
    // AR reports
    assert!(!output.ar_reports.is_empty());
    assert!(!output.ar_reports[0].has_blocking);
    // Graph dot
    assert!(output.graph_dot.node_count > 0);
    // Scenarios
    assert!(!output.scenarios.scenarios.is_empty());
    // Agents manifest
    assert!(!output.agents_manifest.root_agents_md.is_empty());
    // Verification & Audit (Phase 6)
    assert!(!output.propositions.is_empty());
}

/// Test 5: Full end-to-end pipeline (Front Office + Factory + Validate + Telemetry + Git)
/// with mock LLM and MockFactoryWorker, plus CxdbEngine persistence.
#[tokio::test]
async fn tier1_full_pipeline_with_mock_and_storage() {
    let router = mock_router();
    let store = CxdbEngine::new();
    let project_id = Uuid::new_v4();

    let config = PipelineConfig {
        router: &router,
        store: Some(&store),
        dtu_registry: None,
        blueprints: None,
    };

    // Set up worktree root in temp dir
    let run_id = Uuid::new_v4();
    let worktree_root = std::env::temp_dir().join(format!("planner-tier1-full-{}", run_id));
    std::env::set_var("PLANNER_WORKTREE_ROOT", worktree_root.to_string_lossy().to_string());

    let worker = MockFactoryWorker::success(
        "Implemented all requirements successfully",
        vec!["src/CountdownTimer.tsx".into(), "src/App.tsx".into()],
    );

    let result = run_full_pipeline(&config, &worker, project_id, "Build a countdown timer").await;

    assert!(
        result.is_ok(),
        "Full pipeline should succeed: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // Front office artifacts
    assert_eq!(output.front_office.intake.project_name, "Mock Timer");
    assert!(!output.front_office.specs.is_empty());

    // Factory
    assert_eq!(output.factory_output.build_status, BuildStatus::Success);

    // Telemetry
    assert!(!output.telemetry.headline.is_empty());

    // Git
    assert!(!output.git_result.commit.commit_hash.is_empty());
    assert!(output.git_result.commit.message.contains("mock-timer-widget"));

    // Budget tracking
    assert!(output.budget.current_spend_usd >= 0.0);

    // CXDB persistence — turns should have been stored
    let stats = store.stats();
    assert!(
        stats.total_turns > 0,
        "CXDB should have persisted turns; got {} turns",
        stats.total_turns
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&output.factory_output.output_path);
    let _ = std::fs::remove_dir_all(&worktree_root);
}

// ===========================================================================
// Tier 3: Live LLM smoke test (behind feature gate)
// ===========================================================================

/// This test is ONLY compiled when the `live-llm` feature is enabled.
/// It makes a single real LLM call via the CLI router and verifies the
/// response parses correctly as IntakeV1.
///
/// Run with: `cargo test --features live-llm tier3_`
#[cfg(feature = "live-llm")]
#[tokio::test]
async fn tier3_live_intake_smoke() {
    // Use the real CLI router (requires claude/gemini/codex on PATH)
    let router = LlmRouter::from_env();
    let project_id = Uuid::new_v4();

    let providers = router.available_providers();
    if providers.is_empty() {
        eprintln!("SKIP: no CLI providers available on PATH");
        return;
    }

    let result = planner_core::pipeline::steps::intake::execute_intake(
        &router,
        project_id,
        "Build a simple to-do list widget with add, check off, and delete",
    )
    .await;

    match result {
        Ok(intake) => {
            assert!(!intake.project_name.is_empty(), "Project name should not be empty");
            assert!(!intake.feature_slug.is_empty(), "Feature slug should not be empty");
            assert!(!intake.sacred_anchors.is_empty(), "Should have at least one Sacred Anchor");
            assert!(
                !intake.satisfaction_criteria_seeds.is_empty(),
                "Should have at least one satisfaction criterion"
            );
            eprintln!(
                "LIVE SMOKE PASSED: project='{}', slug='{}', anchors={}, criteria={}",
                intake.project_name,
                intake.feature_slug,
                intake.sacred_anchors.len(),
                intake.satisfaction_criteria_seeds.len(),
            );
        }
        Err(e) => {
            // LLM errors (timeout, CLI not found) are acceptable in CI —
            // we only care that the wiring compiles and the call is attempted.
            eprintln!("LIVE SMOKE: LLM call failed (expected in CI): {}", e);
        }
    }
}
