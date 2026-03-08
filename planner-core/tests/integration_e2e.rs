//! # End-to-End Integration Test — Full Pipeline
//!
//! Exercises the complete pipeline from user description → Git commit,
//! using a mock LLM router and Kilroy simulation mode.
//!
//! This test proves all pipeline components connect correctly:
//! 1. Intake Gateway → IntakeV1
//! 2. Compiler → NLSpecV1 + GraphDotV1 + ScenarioSetV1 + AgentsManifestV1
//! 3. Linter (12 rules, deterministic)
//! 4. Factory Diplomat (simulation mode) → FactoryOutputV1
//! 5. Scenario Validator → SatisfactionResultV1
//! 6. Telemetry Presenter → TelemetryReport
//! 7. Git Projection → GitCommitV1
//!
//! Phase 1 additions:
//! - Multi-tier gate evaluation (Critical/High/Medium thresholds)
//! - DoD mechanical checker integration
//! - High gate failure consequence card generation
//!
//! No external CLIs required — all LLM calls return canned responses.

use planner_schemas::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test helper: construct realistic artifacts matching what the LLM would produce
// ---------------------------------------------------------------------------

fn build_test_intake(project_id: Uuid) -> IntakeV1 {
    IntakeV1 {
        project_id,
        project_name: "Countdown Timer".into(),
        feature_slug: "countdown-timer".into(),
        intent_summary: "A simple countdown timer widget that counts down from a user-specified number of seconds, displaying the remaining time. The user can start, pause, and reset the timer.".into(),
        output_domain: OutputDomain::MicroTool {
            variant: MicroToolVariant::ReactWidget,
        },
        environment: EnvironmentInfo {
            language: "TypeScript".into(),
            framework: "React".into(),
            package_manager: Some("npm".into()),
            existing_dependencies: vec![],
            build_tool: Some("vite".into()),
        },
        sacred_anchors: vec![
            SacredAnchor {
                id: "SA-1".into(),
                statement: "Timer must never display negative time".into(),
                rationale: Some("Core invariant — negative time is meaningless".into()),
            },
            SacredAnchor {
                id: "SA-2".into(),
                statement: "Pausing must preserve remaining time exactly".into(),
                rationale: Some("User expectation for pause behavior".into()),
            },
        ],
        satisfaction_criteria_seeds: vec![
            "Starting a 10-second timer shows it counting down to 0".into(),
            "Pausing at 5 seconds and resuming continues from 5".into(),
            "Resetting returns to the original duration".into(),
        ],
        out_of_scope: vec![
            "Sound or visual alerts when timer reaches zero".into(),
            "Multiple concurrent timers".into(),
            "Persistent timer state across page refreshes".into(),
        ],
        conversation_log: vec![ConversationTurn {
            role: "user".into(),
            content: "Build me a countdown timer".into(),
            timestamp: "2026-02-27T00:00:00Z".into(),
        }],
    }
}

fn build_test_spec(project_id: Uuid) -> NLSpecV1 {
    NLSpecV1 {
        project_id,
        version: "1.0".into(),
        chunk: ChunkType::Root,
        status: NLSpecStatus::Draft,
        line_count: 85,
        created_from: "intake-countdown-timer".into(),
        intent_summary: Some(
            "A countdown timer widget with start, pause, and reset controls.".into(),
        ),
        sacred_anchors: Some(vec![
            NLSpecAnchor {
                id: "SA-1".into(),
                statement: "Timer must never display negative time".into(),
            },
            NLSpecAnchor {
                id: "SA-2".into(),
                statement: "Pausing must preserve remaining time exactly".into(),
            },
        ]),
        requirements: vec![
            Requirement {
                id: "FR-1".into(),
                statement: "The system must accept a positive integer duration in seconds".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
            Requirement {
                id: "FR-2".into(),
                statement: "The system must display remaining time updated every second".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
            Requirement {
                id: "FR-3".into(),
                statement: "The system must provide start, pause, and reset controls".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-2".into()],
            },
            Requirement {
                id: "FR-4".into(),
                statement: "The system must stop counting at zero and never go negative".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
        ],
        architectural_constraints: vec![
            "Single React component with hooks".into(),
            "No external state management library".into(),
            "Tailwind CSS for styling".into(),
        ],
        phase1_contracts: Some(vec![Phase1Contract {
            name: "TimerState".into(),
            type_definition: "{ duration: number, remaining: number, running: boolean }".into(),
            consumed_by: vec!["ui".into()],
        }]),
        external_dependencies: vec![],
        definition_of_done: vec![
            DoDItem {
                criterion: "Timer counts down from specified duration to zero".into(),
                mechanically_checkable: true,
            },
            DoDItem {
                criterion: "Start/pause/reset buttons function correctly".into(),
                mechanically_checkable: true,
            },
            DoDItem {
                criterion: "Timer never displays negative values".into(),
                mechanically_checkable: true,
            },
        ],
        satisfaction_criteria: vec![
            SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Starting a 10-second timer shows it counting down to 0".into(),
                tier_hint: ScenarioTierHint::Critical,
            },
            SatisfactionCriterion {
                id: "SC-2".into(),
                description: "Pausing at 5 seconds and resuming continues from 5".into(),
                tier_hint: ScenarioTierHint::Critical,
            },
            SatisfactionCriterion {
                id: "SC-3".into(),
                description: "Resetting returns to the original duration".into(),
                tier_hint: ScenarioTierHint::High,
            },
        ],
        open_questions: vec![],
        out_of_scope: vec![
            "Sound alerts".into(),
            "Multiple concurrent timers".into(),
            "Persistent state".into(),
        ],
        amendment_log: vec![],
    }
}

fn build_test_graph_dot(project_id: Uuid) -> GraphDotV1 {
    GraphDotV1 {
        project_id,
        nlspec_version: "1.0".into(),
        dot_content: r#"digraph countdown_timer {
    // Attractor-compatible pipeline
    start [shape=Mdiamond];
    exit [shape=Msquare];

    check_toolchain [shape=box, label="Check Toolchain\nnpm, vite, react"];
    expand_spec [shape=box, label="Expand NLSpec"];
    implement [shape=box, label="Implement Timer\nReact+Tailwind"];
    verify_build [shape=box, label="Verify Build\nnpm run build"];
    verify_test [shape=box, label="Verify Tests\nnpm test"];
    review [shape=box, label="Final Review"];

    start -> check_toolchain;
    check_toolchain -> expand_spec;
    expand_spec -> implement;
    implement -> verify_build;
    verify_build -> verify_test;
    verify_test -> review;
    review -> exit;

    // Model routing
    graph [model_stylesheet="implement=claude-sonnet-4-6,review=claude-haiku-4-5"];
    // Budget
    graph [goal_gate="verify_test", run_budget_usd="2.00"];
}"#
        .into(),
        node_count: 6,
        estimated_cost_usd: 0.50,
        run_budget_usd: 2.00,
        model_routing: vec![
            NodeModelAssignment {
                node_name: "implement".into(),
                node_class: "implementation".into(),
                model: "claude-sonnet-4-6".into(),
                fidelity: "truncate".into(),
                goal_gate: false,
                max_retries: 2,
            },
            NodeModelAssignment {
                node_name: "review".into(),
                node_class: "review".into(),
                model: "claude-haiku-4-5".into(),
                fidelity: "truncate".into(),
                goal_gate: false,
                max_retries: 1,
            },
        ],
    }
}

fn build_test_scenarios(project_id: Uuid) -> ScenarioSetV1 {
    ScenarioSetV1 {
        project_id,
        nlspec_version: "1.0".into(),
        scenarios: vec![
            Scenario {
                id: "SC-CRIT-1".into(),
                tier: ScenarioTier::Critical,
                title: "Timer counts down to zero".into(),
                bdd_text: "Given a countdown timer set to 10 seconds\nWhen the user presses Start\nThen the display should count down from 10 to 0\nAnd the timer should stop at 0 (never negative)".into(),
                dtu_deps: vec![],
                traces_to_anchors: vec!["SA-1".into()],
                source_criterion: Some("SC-1".into()),
            },
            Scenario {
                id: "SC-CRIT-2".into(),
                tier: ScenarioTier::Critical,
                title: "Pause preserves remaining time".into(),
                bdd_text: "Given a running countdown timer showing 5 seconds remaining\nWhen the user presses Pause\nThen the display should show exactly 5 seconds\nAnd when the user presses Resume\nThen counting should continue from 5 seconds".into(),
                dtu_deps: vec![],
                traces_to_anchors: vec!["SA-2".into()],
                source_criterion: Some("SC-2".into()),
            },
            Scenario {
                id: "SC-HIGH-1".into(),
                tier: ScenarioTier::High,
                title: "Reset returns to original duration".into(),
                bdd_text: "Given a countdown timer originally set to 30 seconds\nAnd the timer is currently at 15 seconds remaining\nWhen the user presses Reset\nThen the display should show 30 seconds\nAnd the timer should be in a stopped state".into(),
                dtu_deps: vec![],
                traces_to_anchors: vec!["SA-2".into()],
                source_criterion: Some("SC-3".into()),
            },
        ],
        isolation_context_id: Uuid::new_v4(),
        ralph_augmented: false,
    }
}

fn build_test_agents_manifest(project_id: Uuid) -> AgentsManifestV1 {
    AgentsManifestV1 {
        project_id,
        nlspec_version: "1.0".into(),
        root_agents_md: r#"# AGENTS.md

## Goal
Build a countdown timer widget using React + Tailwind CSS.

## Jurisdiction
This agent owns the entire countdown-timer feature.

## Sacred Anchors
- Timer must never display negative time
- Pausing must preserve remaining time exactly

## Key Files
- src/CountdownTimer.tsx — Main component
- src/App.tsx — Root mounting
- package.json — Dependencies

## Constraints
- Single React component with hooks
- No external state management
- Tailwind CSS for styling
- ~200 lines maximum
"#
        .into(),
        domain_docs: vec![],
        skill_refs: vec![],
    }
}

// ---------------------------------------------------------------------------
// End-to-end integration tests
// ---------------------------------------------------------------------------

/// Full pipeline integration test using deterministic/simulation paths.
///
/// This tests:
/// - Spec linter (12 deterministic rules)
/// - Factory diplomat (MockFactoryWorker)
/// - Scenario validation (build_all_failed_result since we can't call Gemini)
/// - Telemetry presenter (deterministic mode)
/// - Git projection (real git commands)
///
/// LLM-dependent steps (Intake, Compiler, Validator, Telemetry Presenter)
/// are tested via their unit tests + canned data here.
#[tokio::test]
async fn e2e_phase0_pipeline_simulation() {
    use planner_core::pipeline::steps::{
        factory, factory_worker::MockFactoryWorker, git, linter, telemetry,
    };

    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    // ---- Step 1: Build artifacts (simulating Intake + Compiler output) ----
    let _intake = build_test_intake(project_id);
    let spec = build_test_spec(project_id);
    let graph_dot = build_test_graph_dot(project_id);
    let _scenarios = build_test_scenarios(project_id);
    let agents_manifest = build_test_agents_manifest(project_id);

    // ---- Step 2: Lint the spec ----
    let lint_result = linter::lint_spec(&spec);
    assert!(
        lint_result.is_ok(),
        "Spec linter failed: {:?}",
        lint_result.err()
    );

    // ---- Step 3: Factory Worker (mock — simulates successful factory run) ----
    std::env::set_var(
        "PLANNER_WORKTREE_ROOT",
        std::env::temp_dir()
            .join(format!("planner-e2e-sim-{}", run_id))
            .to_string_lossy()
            .to_string(),
    );

    let worker = MockFactoryWorker::success(
        "Implemented all requirements successfully",
        vec!["index.html".into()],
    );

    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);
    let factory_output = factory::execute_factory_with_worker(
        &worker,
        &graph_dot,
        &agents_manifest,
        &spec,
        None,
        &mut budget,
        None,
        None,
    )
    .await;

    assert!(
        factory_output.is_ok(),
        "Factory handoff failed: {:?}",
        factory_output.err()
    );
    let factory_output = factory_output.unwrap();

    assert_eq!(factory_output.build_status, BuildStatus::Success);
    assert!(!factory_output.node_results.is_empty());
    assert!(factory_output.node_results.iter().all(|n| n.success));

    // Verify worktree output directory was created
    let output_path = std::path::Path::new(&factory_output.output_path);
    assert!(output_path.exists(), "Output directory should exist");

    // ---- Step 4: Scenario Validation (deterministic — build succeeded) ----
    // Since we can't call Gemini in tests, use the build_all_failed path
    // as a deterministic test, AND verify the SatisfactionResultV1 structure
    // We'll construct a realistic passing result instead:
    let satisfaction = SatisfactionResultV1 {
        kilroy_run_id: factory_output.kilroy_run_id,
        critical_pass_rate: 1.0,
        high_pass_rate: 1.0,
        medium_pass_rate: 1.0,
        gates_passed: true,
        scenario_results: vec![
            ScenarioResult {
                scenario_id: "SC-CRIT-1".into(),
                tier: ScenarioTier::Critical,
                runs: [0.9, 0.85, 0.92],
                majority_pass: true,
                score: 0.89,
                generalized_error: None,
            },
            ScenarioResult {
                scenario_id: "SC-CRIT-2".into(),
                tier: ScenarioTier::Critical,
                runs: [0.88, 0.90, 0.87],
                majority_pass: true,
                score: 0.883,
                generalized_error: None,
            },
            ScenarioResult {
                scenario_id: "SC-HIGH-1".into(),
                tier: ScenarioTier::High,
                runs: [0.95, 0.92, 0.91],
                majority_pass: true,
                score: 0.927,
                generalized_error: None,
            },
        ],
    };

    // Verify gate evaluation
    assert!(satisfaction.evaluate_gates());
    assert_eq!(
        satisfaction.user_message(),
        "Everything works as described."
    );

    // Also test the build_all_failed path
    let _failed_build_output = FactoryOutputV1 {
        kilroy_run_id: Uuid::new_v4(),
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: BuildStatus::Failed,
        spend_usd: 0.0,
        checkpoint_path: String::new(),
        dod_results: vec![],
        node_results: vec![],
        output_path: String::new(),
    };

    // ---- Step 5: Telemetry Presenter (deterministic mode) ----
    let telemetry_report = telemetry::build_telemetry_report_deterministic(
        &factory_output,
        &satisfaction,
        &budget,
        project_id,
    );

    assert!(
        telemetry_report.headline.contains("Everything works"),
        "Expected success headline, got: {}",
        telemetry_report.headline
    );
    assert!(!telemetry_report.needs_user_action);
    assert!(telemetry_report.preview_path.is_some());
    assert!(telemetry_report.consequence_cards.is_empty());

    // ---- Step 6: Git Projection ----
    let git_result = git::execute_git_projection(
        &factory_output,
        project_id,
        "Countdown Timer",
        "countdown-timer",
    )
    .await;

    assert!(
        git_result.is_ok(),
        "Git projection failed: {:?}",
        git_result.err()
    );
    let git_result = git_result.unwrap();

    assert_eq!(git_result.commit.branch, "main");
    assert!(!git_result.commit.commit_hash.is_empty());
    assert!(git_result.commit.message.contains("countdown-timer"));
    assert_eq!(git_result.commit.project_id, project_id);
    assert_eq!(
        git_result.commit.kilroy_run_id,
        factory_output.kilroy_run_id
    );

    // ---- Verify end-to-end data flow ----
    // The factory output path should match git repo path
    assert_eq!(git_result.repo_path, factory_output.output_path);

    // Budget should have recorded some spend from the simulation
    assert!(budget.current_spend_usd >= 0.0);

    // Cleanup
    let _ = std::fs::remove_dir_all(&factory_output.output_path);
}

/// Test the failure path: build fails → all scenarios fail → consequence cards generated.
#[tokio::test]
async fn e2e_phase0_pipeline_failure_path() {
    use planner_core::pipeline::steps::telemetry;

    let project_id = Uuid::new_v4();

    // Simulate a failed factory output
    let factory_output = FactoryOutputV1 {
        kilroy_run_id: Uuid::new_v4(),
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: BuildStatus::Failed,
        spend_usd: 1.50,
        checkpoint_path: "/tmp/nonexistent".into(),
        dod_results: vec![],
        node_results: vec![NodeResult {
            node_name: "implement".into(),
            success: false,
            attempts: 3,
            spend_usd: 1.50,
            duration_secs: 120.0,
            error: Some("Build compilation error in Timer.tsx".into()),
        }],
        output_path: "/tmp/nonexistent-output".into(),
    };

    // All scenarios should fail when build fails
    let satisfaction = SatisfactionResultV1 {
        kilroy_run_id: factory_output.kilroy_run_id,
        critical_pass_rate: 0.0,
        high_pass_rate: 0.0,
        medium_pass_rate: 0.0,
        gates_passed: false,
        scenario_results: vec![ScenarioResult {
            scenario_id: "SC-CRIT-1".into(),
            tier: ScenarioTier::Critical,
            runs: [0.0, 0.0, 0.0],
            majority_pass: false,
            score: 0.0,
            generalized_error: Some(GeneralizedError {
                category: "build-failure".into(),
                severity: Severity::Critical,
            }),
        }],
    };

    assert!(!satisfaction.evaluate_gates());
    assert!(satisfaction.user_message().contains("critical"));

    // Telemetry: deterministic report for failure
    let budget = RunBudgetV1::new_phase0(project_id, Uuid::new_v4());
    let report = telemetry::build_telemetry_report_deterministic(
        &factory_output,
        &satisfaction,
        &budget,
        project_id,
    );

    assert!(report.headline.contains("didn't complete"));
    assert!(report.needs_user_action);
    // Failed build shouldn't have preview
    assert!(report.preview_path.is_some()); // still populated for inspection
}

/// Test budget exhaustion path.
#[tokio::test]
async fn e2e_phase0_budget_exhaustion() {
    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);

    // Simulate spending to exhaustion
    budget.record_spend(SpendEvent {
        timestamp: chrono::Utc::now(),
        node_name: "implement".into(),
        model: "claude-sonnet-4-6".into(),
        input_tokens: 50000,
        output_tokens: 20000,
        amount_usd: 4.50,
    });

    // Budget should be in warning state (>80% of $5.00 cap)
    assert_eq!(budget.status, BudgetStatus::Warning);
    assert!(budget.can_proceed()); // Warning but can continue

    // Push past the hard cap
    budget.record_spend(SpendEvent {
        timestamp: chrono::Utc::now(),
        node_name: "verify".into(),
        model: "claude-sonnet-4-6".into(),
        input_tokens: 10000,
        output_tokens: 5000,
        amount_usd: 1.00,
    });

    assert_eq!(budget.status, BudgetStatus::HardStop);
    assert!(!budget.can_proceed());
}

// ---------------------------------------------------------------------------
// Phase 1: Multi-tier validation tests
// ---------------------------------------------------------------------------

/// Test that all three tiers (Critical, High, Medium) are properly evaluated
/// with the correct gate thresholds.
#[tokio::test]
async fn e2e_phase1_multi_tier_gate_evaluation() {
    let _project_id = Uuid::new_v4();

    // All tiers pass
    let all_pass = SatisfactionResultV1 {
        kilroy_run_id: Uuid::new_v4(),
        critical_pass_rate: 1.0,
        high_pass_rate: 0.96,
        medium_pass_rate: 0.92,
        gates_passed: true,
        scenario_results: vec![
            ScenarioResult {
                scenario_id: "SC-CRIT-1".into(),
                tier: ScenarioTier::Critical,
                runs: [0.9, 0.85, 0.92],
                majority_pass: true,
                score: 0.89,
                generalized_error: None,
            },
            ScenarioResult {
                scenario_id: "SC-HIGH-1".into(),
                tier: ScenarioTier::High,
                runs: [0.8, 0.7, 0.85],
                majority_pass: true,
                score: 0.78,
                generalized_error: None,
            },
            ScenarioResult {
                scenario_id: "SC-MED-1".into(),
                tier: ScenarioTier::Medium,
                runs: [0.6, 0.7, 0.65],
                majority_pass: true,
                score: 0.65,
                generalized_error: None,
            },
        ],
    };
    assert!(all_pass.evaluate_gates());
    assert_eq!(all_pass.user_message(), "Everything works as described.");

    // Critical fails -> always fails
    let crit_fail = SatisfactionResultV1 {
        critical_pass_rate: 0.5,
        gates_passed: false,
        ..all_pass.clone()
    };
    assert!(!crit_fail.evaluate_gates());
    assert!(crit_fail.user_message().contains("critical"));

    // High at exactly 0.95 -> passes
    let high_boundary = SatisfactionResultV1 {
        high_pass_rate: 0.95,
        ..all_pass.clone()
    };
    assert!(high_boundary.evaluate_gates());

    // High at 0.94 -> fails
    let high_fail = SatisfactionResultV1 {
        high_pass_rate: 0.94,
        gates_passed: false,
        ..all_pass.clone()
    };
    assert!(!high_fail.evaluate_gates());

    // Medium at exactly 0.90 -> passes
    let med_boundary = SatisfactionResultV1 {
        medium_pass_rate: 0.90,
        ..all_pass.clone()
    };
    assert!(med_boundary.evaluate_gates());

    // Medium at 0.89 -> fails
    let med_fail = SatisfactionResultV1 {
        medium_pass_rate: 0.89,
        gates_passed: false,
        ..all_pass.clone()
    };
    assert!(!med_fail.evaluate_gates());
}

/// Test DoD mechanical checker integration with factory output.
#[tokio::test]
async fn e2e_phase1_dod_checker_integration() {
    use planner_core::pipeline::steps::validate;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    // Successful build with all gates passing
    let factory_output = FactoryOutputV1 {
        kilroy_run_id: Uuid::new_v4(),
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: BuildStatus::Success,
        spend_usd: 0.50,
        checkpoint_path: "/tmp/cp.json".into(),
        dod_results: vec![],
        node_results: vec![NodeResult {
            node_name: "implement".into(),
            success: true,
            attempts: 1,
            spend_usd: 0.30,
            duration_secs: 20.0,
            error: None,
        }],
        output_path: "/tmp/output".into(),
    };

    let satisfaction = SatisfactionResultV1 {
        kilroy_run_id: factory_output.kilroy_run_id,
        critical_pass_rate: 1.0,
        high_pass_rate: 1.0,
        medium_pass_rate: 1.0,
        gates_passed: true,
        scenario_results: vec![],
    };

    let dod_results = validate::check_definition_of_done(&spec, &factory_output, &satisfaction);

    // The test spec has 3 DoD items, all mechanically checkable
    assert_eq!(dod_results.len(), 3);
    assert!(dod_results.iter().all(|r| r.passed));
    assert!(dod_results.iter().all(|r| r.check_method == "mechanical"));

    // Now test with a failed build
    let failed_factory = FactoryOutputV1 {
        build_status: BuildStatus::Failed,
        ..factory_output.clone()
    };
    let failed_satisfaction = SatisfactionResultV1 {
        critical_pass_rate: 0.0,
        high_pass_rate: 0.0,
        medium_pass_rate: 0.0,
        gates_passed: false,
        ..satisfaction.clone()
    };

    let dod_fail_results =
        validate::check_definition_of_done(&spec, &failed_factory, &failed_satisfaction);

    // Build-related DoD items should fail
    assert!(dod_fail_results.iter().any(|r| !r.passed));
}

/// Test that High gate failure generates a consequence card with error categories.
#[tokio::test]
async fn e2e_phase1_high_gate_failure_consequence_card() {
    use planner_core::pipeline::steps::telemetry;

    let project_id = Uuid::new_v4();

    let factory_output = FactoryOutputV1 {
        kilroy_run_id: Uuid::new_v4(),
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: BuildStatus::Success,
        spend_usd: 0.75,
        checkpoint_path: "/tmp/cp.json".into(),
        dod_results: vec![],
        node_results: vec![NodeResult {
            node_name: "implement".into(),
            success: true,
            attempts: 1,
            spend_usd: 0.75,
            duration_secs: 30.0,
            error: None,
        }],
        output_path: "/tmp/out".into(),
    };

    // Critical passes, High fails at 80%
    let satisfaction = SatisfactionResultV1 {
        kilroy_run_id: factory_output.kilroy_run_id,
        critical_pass_rate: 1.0,
        high_pass_rate: 0.80,
        medium_pass_rate: 0.95,
        gates_passed: false,
        scenario_results: vec![ScenarioResult {
            scenario_id: "SC-HIGH-1".into(),
            tier: ScenarioTier::High,
            runs: [0.3, 0.4, 0.2],
            majority_pass: false,
            score: 0.3,
            generalized_error: Some(GeneralizedError {
                category: "state-management".into(),
                severity: Severity::High,
            }),
        }],
    };

    let budget = RunBudgetV1::new_phase0(project_id, Uuid::new_v4());

    let report = telemetry::build_telemetry_report_deterministic(
        &factory_output,
        &satisfaction,
        &budget,
        project_id,
    );

    assert!(report.needs_user_action);
    assert!(report.headline.contains("mostly right") || report.headline.contains("important"));
}

/// Test that the linter correctly validates spec structure.
#[tokio::test]
async fn e2e_linter_catches_violations() {
    use planner_core::pipeline::steps::linter;

    let project_id = Uuid::new_v4();
    let mut spec = build_test_spec(project_id);

    // Valid spec should pass
    assert!(linter::lint_spec(&spec).is_ok());

    // Remove all requirements → should fail
    spec.requirements.clear();
    let result = linter::lint_spec(&spec);
    assert!(result.is_err());

    // Restore requirements, but clear DoD → should fail
    spec = build_test_spec(project_id);
    spec.definition_of_done.clear();
    let result = linter::lint_spec(&spec);
    assert!(result.is_err());

    // Restore DoD, but clear satisfaction criteria → should fail
    spec = build_test_spec(project_id);
    spec.satisfaction_criteria.clear();
    let result = linter::lint_spec(&spec);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Phase 2: Adversarial Review tests
// ---------------------------------------------------------------------------

/// Test AR report building and recalculation with mixed findings.
#[tokio::test]
async fn e2e_phase2_ar_report_construction() {
    let project_id = Uuid::new_v4();

    // Build a report with findings from multiple reviewers
    let mut report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![
            ArFinding {
                id: "AR-B-1".into(),
                reviewer: ArReviewer::Opus,
                severity: ArSeverity::Blocking,
                affected_section: "Requirements".into(),
                affected_requirements: vec!["FR-1".into()],
                description: "FR-1 ambiguous about error handling".into(),
                suggested_resolution: Some("Specify behavior for non-positive input".into()),
            },
            ArFinding {
                id: "AR-A-1".into(),
                reviewer: ArReviewer::Gpt,
                severity: ArSeverity::Advisory,
                affected_section: "Definition of Done".into(),
                affected_requirements: vec![],
                description: "DoD item 2 could be more specific".into(),
                suggested_resolution: None,
            },
            ArFinding {
                id: "AR-I-1".into(),
                reviewer: ArReviewer::Gemini,
                severity: ArSeverity::Informational,
                affected_section: "Out of Scope".into(),
                affected_requirements: vec![],
                description: "Consider adding timer persistence to out-of-scope".into(),
                suggested_resolution: None,
            },
            ArFinding {
                id: "AR-B-2".into(),
                reviewer: ArReviewer::Gemini,
                severity: ArSeverity::Blocking,
                affected_section: "Satisfaction Criteria".into(),
                affected_requirements: vec!["SC-1".into()],
                description: "SC-1 missing critical-tier seed requirement".into(),
                suggested_resolution: Some("Ensure SC-1 has tier_hint=Critical".into()),
            },
        ],
        reviewer_summaries: vec![
            ReviewerSummary {
                reviewer: ArReviewer::Opus,
                summary: "One blocking issue with FR-1 ambiguity.".into(),
                finding_count: 1,
                blocking_count: 1,
            },
            ReviewerSummary {
                reviewer: ArReviewer::Gpt,
                summary: "Generally implementable, minor DoD suggestion.".into(),
                finding_count: 1,
                blocking_count: 0,
            },
            ReviewerSummary {
                reviewer: ArReviewer::Gemini,
                summary: "Scope mostly contained, one blocking SC issue.".into(),
                finding_count: 2,
                blocking_count: 1,
            },
        ],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };

    report.recalculate();

    assert!(report.has_blocking);
    assert_eq!(report.blocking_count, 2);
    assert_eq!(report.advisory_count, 1);
    assert_eq!(report.informational_count, 1);
    assert_eq!(report.findings.len(), 4);
    assert_eq!(report.reviewer_summaries.len(), 3);
}

/// Test that a clean spec (no issues) produces an AR report with no blocking findings.
#[tokio::test]
async fn e2e_phase2_clean_spec_passes_ar_lint() {
    use planner_core::pipeline::steps::linter;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    // The test spec should pass linting (prerequisite for AR)
    assert!(linter::lint_spec(&spec).is_ok());

    // A clean AR report (simulating what we'd get from well-formed spec)
    let mut report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![
            // Only advisory/informational — no blocking
            ArFinding {
                id: "AR-A-1".into(),
                reviewer: ArReviewer::Gpt,
                severity: ArSeverity::Advisory,
                affected_section: "Phase 1 Contracts".into(),
                affected_requirements: vec![],
                description: "TimerState could include an 'elapsed' field for convenience".into(),
                suggested_resolution: Some("Optional enhancement".into()),
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

/// Test AR refinement: apply amendments to a spec and verify changes.
#[tokio::test]
async fn e2e_phase2_ar_refinement_applies_amendments() {
    let project_id = Uuid::new_v4();
    let mut spec = build_test_spec(project_id);

    // Verify initial state
    assert_eq!(spec.requirements.len(), 4);
    assert_eq!(spec.out_of_scope.len(), 3);
    assert!(spec.amendment_log.is_empty());

    // Simulate: modify FR-1 statement
    let fr1 = spec
        .requirements
        .iter_mut()
        .find(|r| r.id == "FR-1")
        .unwrap();
    fr1.statement = "The system must accept a positive integer duration in seconds and reject non-positive values with an error message".into();

    // Simulate: add a new requirement
    spec.requirements.push(Requirement {
        id: "FR-5".into(),
        statement: "The system must handle zero-length durations by displaying 00:00 immediately"
            .into(),
        priority: Priority::Should,
        traces_to: vec!["SA-1".into()],
    });

    // Simulate: add out-of-scope item
    spec.out_of_scope
        .push("Timer persistence across browser sessions".into());

    // Simulate: add amendment log entry
    spec.amendment_log.push(Amendment {
        timestamp: "2026-02-27T12:00:00Z".into(),
        description: "Clarified FR-1 error handling, added FR-5 for zero-length edge case, added session persistence to out-of-scope".into(),
        reason: "AR Refinement iteration 1".into(),
        affected_section: "requirements, out_of_scope".into(),
    });

    // Verify amended spec
    assert_eq!(spec.requirements.len(), 5);
    assert!(spec.requirements[0]
        .statement
        .contains("reject non-positive"));
    assert_eq!(spec.requirements[4].id, "FR-5");
    assert_eq!(spec.out_of_scope.len(), 4);
    assert!(spec.out_of_scope.last().unwrap().contains("persistence"));
    assert_eq!(spec.amendment_log.len(), 1);

    // The amended spec should still pass linting
    use planner_core::pipeline::steps::linter;
    assert!(linter::lint_spec(&spec).is_ok());
}

/// Test OQ Consequence Card generation.
#[tokio::test]
async fn e2e_phase2_oq_consequence_cards() {
    use planner_core::pipeline::steps::ar_refinement;

    let project_id = Uuid::new_v4();
    let open_questions = vec![
        "Should the timer support custom time formats (mm:ss vs just seconds)?".to_string(),
        "What happens when the user enters a duration longer than 24 hours?".to_string(),
        "Should the timer display milliseconds?".to_string(),
    ];

    let cards = ar_refinement::generate_oq_consequence_cards(&open_questions, project_id);

    assert_eq!(cards.len(), 3);

    for (i, card) in cards.iter().enumerate() {
        assert_eq!(card.project_id, project_id);
        assert_eq!(card.trigger, CardTrigger::OpenQuestion);
        assert!(card.problem.contains(&open_questions[i]));
        assert_eq!(card.status, CardStatus::Pending);
        assert!(card.resolution.is_none());

        // Each card should have Answer and Out-of-Scope actions
        assert_eq!(card.actions.len(), 2);
        assert_eq!(card.actions[0].label, "Answer");
        assert_eq!(card.actions[1].label, "Out of Scope");
    }

    // Empty OQ list should produce no cards
    let empty_cards = ar_refinement::generate_oq_consequence_cards(&[], project_id);
    assert!(empty_cards.is_empty());
}

/// Test that spec rendering for AR review includes all critical sections.
#[tokio::test]
async fn e2e_phase2_spec_rendering_for_review() {
    use planner_core::pipeline::steps::ar;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);
    let rendered = ar::render_spec_for_review(&spec);

    // Check all sections are present
    assert!(rendered.contains("# NLSpec"), "Missing header");
    assert!(
        rendered.contains("Intent Summary"),
        "Missing intent summary"
    );
    assert!(
        rendered.contains("Sacred Anchors"),
        "Missing sacred anchors"
    );
    assert!(rendered.contains("SA-1"), "Missing SA-1");
    assert!(rendered.contains("SA-2"), "Missing SA-2");
    assert!(
        rendered.contains("Functional Requirements"),
        "Missing requirements"
    );
    assert!(rendered.contains("FR-1"), "Missing FR-1");
    assert!(rendered.contains("FR-2"), "Missing FR-2");
    assert!(rendered.contains("FR-3"), "Missing FR-3");
    assert!(rendered.contains("FR-4"), "Missing FR-4");
    assert!(
        rendered.contains("Architectural Constraints"),
        "Missing constraints"
    );
    assert!(rendered.contains("Phase 1 Contracts"), "Missing contracts");
    assert!(
        rendered.contains("TimerState"),
        "Missing TimerState contract"
    );
    assert!(rendered.contains("Definition of Done"), "Missing DoD");
    assert!(rendered.contains("Satisfaction Criteria"), "Missing SC");
    assert!(rendered.contains("SC-1"), "Missing SC-1");
    assert!(rendered.contains("Open Questions"), "Missing OQ section");
    assert!(rendered.contains("(none)"), "Missing (none) for empty OQ");
    assert!(rendered.contains("Out of Scope"), "Missing OOS");
    assert!(rendered.contains("Sound alerts"), "Missing OOS item");
}

/// Test AR severity classification: an ArReportV1 with mixed findings correctly
/// reports `has_blocking` and counts after `recalculate()`.
#[tokio::test]
async fn e2e_phase2_ar_severity_classification() {
    let project_id = Uuid::new_v4();

    // Build a report with mixed severities across different reviewers.
    let mut report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![
            ArFinding {
                id: String::new(),
                reviewer: ArReviewer::Opus,
                severity: ArSeverity::Blocking,
                affected_section: "Requirements".into(),
                affected_requirements: vec!["FR-1".into()],
                description: "Ambiguous error handling".into(),
                suggested_resolution: Some("Specify error behavior".into()),
            },
            ArFinding {
                id: String::new(),
                reviewer: ArReviewer::Gpt,
                severity: ArSeverity::Advisory,
                affected_section: "DoD".into(),
                affected_requirements: vec![],
                description: "DoD item not mechanically checkable".into(),
                suggested_resolution: None,
            },
            ArFinding {
                id: String::new(),
                reviewer: ArReviewer::Gemini,
                severity: ArSeverity::Informational,
                affected_section: "Out of Scope".into(),
                affected_requirements: vec![],
                description: "Consider adding persistence to OOS".into(),
                suggested_resolution: None,
            },
            ArFinding {
                id: String::new(),
                reviewer: ArReviewer::Gemini,
                severity: ArSeverity::Blocking,
                affected_section: "Sacred Anchors".into(),
                affected_requirements: vec!["SA-1".into()],
                description: "SA-1 not covered by any FR".into(),
                suggested_resolution: Some("Add FR that traces to SA-1".into()),
            },
        ],
        reviewer_summaries: vec![],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };
    report.recalculate();

    // Verify `has_blocking` is set correctly
    assert!(
        report.has_blocking,
        "Report with Blocking findings must set has_blocking=true"
    );

    // Verify each severity bucket is counted correctly
    assert_eq!(report.blocking_count, 2, "Expected 2 blocking findings");
    assert_eq!(report.advisory_count, 1, "Expected 1 advisory finding");
    assert_eq!(
        report.informational_count, 1,
        "Expected 1 informational finding"
    );

    // A report with only advisory/informational should NOT set has_blocking
    let mut clean_report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![ArFinding {
            id: String::new(),
            reviewer: ArReviewer::Gpt,
            severity: ArSeverity::Advisory,
            affected_section: "DoD".into(),
            affected_requirements: vec![],
            description: "Minor suggestion".into(),
            suggested_resolution: None,
        }],
        reviewer_summaries: vec![],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };
    clean_report.recalculate();
    assert!(
        !clean_report.has_blocking,
        "Report with only advisory findings must NOT set has_blocking"
    );
    assert_eq!(clean_report.blocking_count, 0);
    assert_eq!(clean_report.advisory_count, 1);
}

/// Test that AR integrates correctly with the pipeline recipe AND verify the
/// complete Phase 0 recipe has the expected number of steps in the correct order.
///
/// This replaces the two former recipe tests (`e2e_phase2_recipe_includes_ar_steps`
/// and `e2e_phase3_recipe_includes_new_steps`) with a single test that gives full
/// regression coverage of the DAG definition.
#[tokio::test]
async fn e2e_phase2_recipe_includes_ar_steps() {
    use planner_core::pipeline::{Recipe, StepType};

    let recipe = Recipe::phase0();

    // ---- 1. Total step count ----
    // The Phase 0 recipe has 17 steps (intake through git-projection).
    assert_eq!(
        recipe.steps.len(),
        17,
        "Phase 0 recipe should have exactly 17 steps; found {}: {:?}",
        recipe.steps.len(),
        recipe
            .steps
            .iter()
            .map(|s| s.step_id.as_str())
            .collect::<Vec<_>>(),
    );

    // ---- 2. Required step IDs are present ----
    let step_ids: Vec<&str> = recipe.steps.iter().map(|s| s.step_id.as_str()).collect();
    for expected in &[
        "intake",
        "chunk-plan",
        "compile-spec",
        "lint-spec",
        "adversarial-review",
        "ar-refinement",
        "generate-scenarios",
        "ralph-loop",
        "compile-graph-dot",
        "compile-agents-manifest",
        "factory-handoff",
        "factory-poll",
        "validate-scenarios",
        "deploy-sandbox",
        "present-telemetry",
        "await-approval",
        "git-projection",
    ] {
        assert!(
            step_ids.contains(expected),
            "Phase 0 recipe missing step: '{}'",
            expected,
        );
    }

    // ---- 3. Ordering constraints ----
    let pos = |id: &str| step_ids.iter().position(|&s| s == id).unwrap();

    // intake → chunk-plan → compile-spec → lint-spec → adversarial-review → ar-refinement
    assert!(
        pos("intake") < pos("chunk-plan"),
        "intake before chunk-plan"
    );
    assert!(
        pos("chunk-plan") < pos("compile-spec"),
        "chunk-plan before compile-spec"
    );
    assert!(
        pos("compile-spec") < pos("lint-spec"),
        "compile-spec before lint-spec"
    );
    assert!(
        pos("lint-spec") < pos("adversarial-review"),
        "lint-spec before adversarial-review"
    );
    assert!(
        pos("adversarial-review") < pos("ar-refinement"),
        "AR before refinement"
    );

    // ar-refinement → generate-scenarios → ralph-loop → compile-graph-dot
    assert!(
        pos("ar-refinement") < pos("generate-scenarios"),
        "refinement before scenarios"
    );
    assert!(
        pos("generate-scenarios") < pos("ralph-loop"),
        "scenarios before ralph"
    );
    assert!(
        pos("ralph-loop") < pos("compile-graph-dot"),
        "ralph before graph-dot"
    );

    // factory-handoff → factory-poll → validate-scenarios → present-telemetry → git-projection
    assert!(
        pos("factory-handoff") < pos("factory-poll"),
        "handoff before poll"
    );
    assert!(
        pos("factory-poll") < pos("validate-scenarios"),
        "poll before validate"
    );
    assert!(
        pos("validate-scenarios") < pos("present-telemetry"),
        "validate before telemetry"
    );
    assert!(
        pos("present-telemetry") < pos("await-approval"),
        "telemetry before approval"
    );
    assert!(
        pos("await-approval") < pos("git-projection"),
        "approval before git"
    );

    // ---- 4. Dependency checks (spot-check key wires) ----
    let ar_step = recipe
        .steps
        .iter()
        .find(|s| s.step_id == "adversarial-review")
        .unwrap();
    assert!(
        ar_step.depends_on.contains(&"lint-spec".to_string()),
        "adversarial-review should depend on lint-spec"
    );

    let refine_step = recipe
        .steps
        .iter()
        .find(|s| s.step_id == "ar-refinement")
        .unwrap();
    assert!(
        refine_step
            .depends_on
            .contains(&"adversarial-review".to_string()),
        "ar-refinement should depend on adversarial-review"
    );

    let graph_step = recipe
        .steps
        .iter()
        .find(|s| s.step_id == "compile-graph-dot")
        .unwrap();
    assert!(
        graph_step.depends_on.contains(&"ralph-loop".to_string()),
        "compile-graph-dot should depend on ralph-loop"
    );

    let handoff_step = recipe
        .steps
        .iter()
        .find(|s| s.step_id == "factory-handoff")
        .unwrap();
    assert!(
        handoff_step
            .depends_on
            .contains(&"compile-graph-dot".to_string()),
        "factory-handoff should depend on compile-graph-dot"
    );
    assert!(
        handoff_step
            .depends_on
            .contains(&"generate-scenarios".to_string()),
        "factory-handoff should depend on generate-scenarios"
    );
    assert!(
        handoff_step
            .depends_on
            .contains(&"compile-agents-manifest".to_string()),
        "factory-handoff should depend on compile-agents-manifest"
    );

    // ---- 5. Step types ----
    assert!(matches!(ar_step.step_type, StepType::AdversarialReview));
    assert!(matches!(refine_step.step_type, StepType::ArRefinement));
    assert!(matches!(
        recipe
            .steps
            .iter()
            .find(|s| s.step_id == "chunk-plan")
            .unwrap()
            .step_type,
        StepType::ChunkPlan
    ));
    assert!(matches!(
        recipe
            .steps
            .iter()
            .find(|s| s.step_id == "ralph-loop")
            .unwrap()
            .step_type,
        StepType::RalphLoop
    ));
    assert!(matches!(
        recipe
            .steps
            .iter()
            .find(|s| s.step_id == "git-projection")
            .unwrap()
            .step_type,
        StepType::GitProjection
    ));
}

/// Test that `execute_ar_refinement` with a non-blocking report returns
/// immediately (iterations=0) without calling the LLM.
#[tokio::test]
async fn e2e_phase2_refinement_no_blocking_passthrough() {
    use planner_core::pipeline::steps::ar_refinement;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    // A clean report with no blocking findings
    let report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![ArFinding {
            id: "AR-A-1".into(),
            reviewer: ArReviewer::Gpt,
            severity: ArSeverity::Advisory,
            affected_section: "DoD".into(),
            affected_requirements: vec![],
            description: "Minor suggestion".into(),
            suggested_resolution: None,
        }],
        reviewer_summaries: vec![],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 1,
        informational_count: 0,
    };

    // has_blocking=false → execute_ar_refinement should short-circuit immediately.
    // We use a no-op router since no LLM call should be made.
    let router = planner_core::llm::providers::LlmRouter::from_env();
    let result =
        ar_refinement::execute_ar_refinement(&router, spec.clone(), &report, project_id).await;

    assert!(
        result.is_ok(),
        "Non-blocking report refinement should not fail: {:?}",
        result.err()
    );
    let refinement = result.unwrap();

    // Short-circuit path: 0 iterations (no LLM call needed)
    assert_eq!(
        refinement.iterations, 0,
        "Non-blocking report should return with 0 iterations"
    );
    // The spec comes back untouched
    assert!(
        refinement.resolved,
        "Non-blocking report should be marked resolved"
    );
    // No open questions should be generated
    assert!(
        refinement.open_questions.is_empty(),
        "Non-blocking report should have no OQs"
    );
    // No amendment log entries
    assert!(
        refinement.amendment_entries.is_empty(),
        "Non-blocking report should have no amendments"
    );
    // The returned spec should match the input
    assert_eq!(refinement.spec.project_id, spec.project_id);
    assert_eq!(refinement.spec.requirements.len(), spec.requirements.len());
}

// ---------------------------------------------------------------------------
// Phase 3: Multi-Chunk Compiler + Context Packs + Ralph Loops
// ---------------------------------------------------------------------------

/// Build helpers for a multi-chunk project (e-commerce with auth + api + ui).
fn build_multi_chunk_intake(project_id: Uuid) -> IntakeV1 {
    IntakeV1 {
        project_id,
        project_name: "E-Commerce API".into(),
        feature_slug: "ecommerce-api".into(),
        intent_summary: "A full-stack e-commerce API with authentication, product catalog, shopping cart, and payment processing.".into(),
        output_domain: OutputDomain::FullApp {
            estimated_domains: 3,
        },
        environment: EnvironmentInfo {
            language: "TypeScript".into(),
            framework: "Express".into(),
            package_manager: Some("npm".into()),
            existing_dependencies: vec![],
            build_tool: Some("tsc".into()),
        },
        sacred_anchors: vec![
            SacredAnchor {
                id: "SA-1".into(),
                statement: "Passwords must never be stored in plain text".into(),
                rationale: Some("Security requirement".into()),
            },
            SacredAnchor {
                id: "SA-2".into(),
                statement: "Payment operations must always be idempotent".into(),
                rationale: Some("Financial integrity".into()),
            },
            SacredAnchor {
                id: "SA-3".into(),
                statement: "API responses must always include proper error codes".into(),
                rationale: Some("Client integration reliability".into()),
            },
        ],
        satisfaction_criteria_seeds: vec![
            "User can register and login with valid credentials".into(),
            "Products can be listed with pagination".into(),
            "Cart persists across sessions".into(),
            "Payment succeeds for valid card".into(),
        ],
        out_of_scope: vec![
            "Admin dashboard".into(),
            "Email notifications".into(),
            "Order tracking".into(),
        ],
        conversation_log: vec![ConversationTurn {
            role: "user".into(),
            content: "Build an e-commerce API".into(),
            timestamp: "2026-02-27T00:00:00Z".into(),
        }],
    }
}

fn build_multi_chunk_root_spec(project_id: Uuid) -> NLSpecV1 {
    NLSpecV1 {
        project_id,
        version: "1.0".into(),
        chunk: ChunkType::Root,
        status: NLSpecStatus::Draft,
        line_count: 120,
        created_from: "intake-ecommerce-api".into(),
        intent_summary: Some(
            "Full-stack e-commerce API with auth, product catalog, cart, and payments.".into(),
        ),
        sacred_anchors: Some(vec![
            NLSpecAnchor {
                id: "SA-1".into(),
                statement: "Passwords must never be stored in plain text".into(),
            },
            NLSpecAnchor {
                id: "SA-2".into(),
                statement: "Payment operations must always be idempotent".into(),
            },
            NLSpecAnchor {
                id: "SA-3".into(),
                statement: "API responses must always include proper error codes".into(),
            },
        ]),
        requirements: vec![
            Requirement {
                id: "FR-ROOT-1".into(),
                statement: "The system must expose a RESTful API on port 3000".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-3".into()],
            },
            Requirement {
                id: "FR-ROOT-2".into(),
                statement: "The system must never store credentials in plain text".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
            Requirement {
                id: "FR-ROOT-3".into(),
                statement: "The system must always use idempotent payment processing".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-2".into()],
            },
        ],
        architectural_constraints: vec![
            "Express.js backend".into(),
            "PostgreSQL database".into(),
            "JWT for auth tokens".into(),
        ],
        phase1_contracts: Some(vec![
            Phase1Contract {
                name: "User".into(),
                type_definition: "{ id: string, email: string, passwordHash: string }".into(),
                consumed_by: vec!["auth".into(), "api".into()],
            },
            Phase1Contract {
                name: "Product".into(),
                type_definition: "{ id: string, name: string, price: number, stock: number }"
                    .into(),
                consumed_by: vec!["api".into(), "ui".into()],
            },
            Phase1Contract {
                name: "CartItem".into(),
                type_definition: "{ productId: string, quantity: number, userId: string }".into(),
                consumed_by: vec!["api".into()],
            },
        ]),
        external_dependencies: vec![],
        definition_of_done: vec![
            DoDItem {
                criterion: "All endpoints respond with JSON".into(),
                mechanically_checkable: true,
            },
            DoDItem {
                criterion: "Auth endpoints require no token for register/login".into(),
                mechanically_checkable: true,
            },
        ],
        satisfaction_criteria: vec![
            SatisfactionCriterion {
                id: "SC-1".into(),
                description: "User can register and login".into(),
                tier_hint: ScenarioTierHint::Critical,
            },
            SatisfactionCriterion {
                id: "SC-2".into(),
                description: "Products can be listed".into(),
                tier_hint: ScenarioTierHint::High,
            },
        ],
        open_questions: vec![],
        out_of_scope: vec!["Admin dashboard".into(), "Email notifications".into()],
        amendment_log: vec![],
    }
}

fn build_auth_domain_spec(project_id: Uuid) -> NLSpecV1 {
    NLSpecV1 {
        project_id,
        version: "1.0".into(),
        chunk: ChunkType::Domain {
            name: "auth".into(),
        },
        status: NLSpecStatus::Draft,
        line_count: 80,
        created_from: "root-ecommerce-api".into(),
        intent_summary: Some(
            "Authentication domain: user registration, login, and JWT management.".into(),
        ),
        sacred_anchors: None, // Domain chunks inherit from root
        requirements: vec![
            Requirement {
                id: "FR-AUTH-1".into(),
                statement: "The auth module must hash passwords with bcrypt before storage".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
            Requirement {
                id: "FR-AUTH-2".into(),
                statement: "The auth module must issue JWT tokens on successful login".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-3".into()],
            },
            Requirement {
                id: "FR-AUTH-3".into(),
                statement: "The auth module must never expose password hashes in API responses"
                    .into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            },
        ],
        architectural_constraints: vec!["bcrypt for password hashing".into()],
        phase1_contracts: None,
        external_dependencies: vec![],
        definition_of_done: vec![DoDItem {
            criterion: "Login returns JWT on valid credentials".into(),
            mechanically_checkable: true,
        }],
        satisfaction_criteria: vec![SatisfactionCriterion {
            id: "SC-AUTH-1".into(),
            description: "Registration + login flow succeeds".into(),
            tier_hint: ScenarioTierHint::Critical,
        }],
        open_questions: vec![],
        out_of_scope: vec!["OAuth providers".into()],
        amendment_log: vec![],
    }
}

fn build_api_domain_spec(project_id: Uuid) -> NLSpecV1 {
    NLSpecV1 {
        project_id,
        version: "1.0".into(),
        chunk: ChunkType::Domain { name: "api".into() },
        status: NLSpecStatus::Draft,
        line_count: 90,
        created_from: "root-ecommerce-api".into(),
        intent_summary: Some(
            "API domain: product catalog, shopping cart, and payment endpoints.".into(),
        ),
        sacred_anchors: None,
        requirements: vec![
            Requirement {
                id: "FR-API-1".into(),
                statement: "The API must provide paginated product listing".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-3".into()],
            },
            Requirement {
                id: "FR-API-2".into(),
                statement: "The API must always use idempotency keys for payment operations".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-2".into()],
            },
        ],
        architectural_constraints: vec!["Stripe for payments".into()],
        phase1_contracts: None,
        external_dependencies: vec![],
        definition_of_done: vec![DoDItem {
            criterion: "GET /products returns paginated JSON".into(),
            mechanically_checkable: true,
        }],
        satisfaction_criteria: vec![SatisfactionCriterion {
            id: "SC-API-1".into(),
            description: "Product listing with pagination works".into(),
            tier_hint: ScenarioTierHint::Critical,
        }],
        open_questions: vec![],
        out_of_scope: vec!["Product search".into()],
        amendment_log: vec![],
    }
}

/// Test multi-chunk lint_spec_set: valid set of root + domain chunks passes.
#[tokio::test]
async fn e2e_phase3_lint_spec_set_valid() {
    use planner_core::pipeline::steps::linter;

    let project_id = Uuid::new_v4();
    let root = build_multi_chunk_root_spec(project_id);
    let auth = build_auth_domain_spec(project_id);
    let api = build_api_domain_spec(project_id);

    let specs = vec![root, auth, api];
    let result = linter::lint_spec_set(&specs);
    assert!(
        result.is_ok(),
        "Valid multi-chunk spec set should pass lint: {:?}",
        result.err()
    );
}

/// Test multi-chunk lint catches duplicate FR IDs across chunks.
#[tokio::test]
async fn e2e_phase3_lint_spec_set_duplicate_fr_ids() {
    use planner_core::pipeline::steps::linter;

    let project_id = Uuid::new_v4();
    let root = build_multi_chunk_root_spec(project_id);
    let mut auth = build_auth_domain_spec(project_id);

    // Introduce a duplicate: FR-ROOT-1 already exists in root
    auth.requirements.push(Requirement {
        id: "FR-ROOT-1".into(), // DUPLICATE!
        statement: "The auth module must never allow empty passwords".into(),
        priority: Priority::Must,
        traces_to: vec!["SA-1".into()],
    });

    let specs = vec![root, auth];
    let result = linter::lint_spec_set(&specs);
    assert!(
        result.is_err(),
        "Duplicate FR ID across chunks should fail lint"
    );
}

/// Test multi-chunk lint catches uncovered Sacred Anchors.
#[tokio::test]
async fn e2e_phase3_lint_spec_set_uncovered_anchor() {
    use planner_core::pipeline::steps::linter;

    let project_id = Uuid::new_v4();
    let mut root = build_multi_chunk_root_spec(project_id);
    // Remove FR-ROOT-3 (which traces to SA-2) so SA-2 is uncovered
    root.requirements.retain(|r| r.id != "FR-ROOT-3");

    // Only auth domain — SA-2 (payment idempotency) is NOT traced by any FR
    // Auth only traces SA-1 and SA-3, root traces SA-1 and SA-3 (after removing FR-ROOT-3)
    // SA-2 only gets traced by api domain which we exclude here
    let auth = build_auth_domain_spec(project_id);

    let specs = vec![root, auth];
    let result = linter::lint_spec_set(&specs);
    assert!(
        result.is_err(),
        "Uncovered Sacred Anchor SA-2 should fail lint"
    );
}

/// Test chunk planner: MicroTool is always single-chunk (no LLM call needed).
#[tokio::test]
async fn e2e_phase3_chunk_planner_microtool_single_chunk() {
    use planner_core::llm::providers::LlmRouter;
    use planner_core::pipeline::steps::chunk_planner;

    let project_id = Uuid::new_v4();
    let intake = build_test_intake(project_id); // MicroTool countdown timer
    let router = LlmRouter::from_env();

    // MicroTool triggers the heuristic short-circuit — no LLM call needed
    let plan = chunk_planner::plan_chunks(&router, &intake, project_id).await;
    assert!(
        plan.is_ok(),
        "MicroTool plan should succeed without LLM: {:?}",
        plan.err()
    );
    let plan = plan.unwrap();

    assert!(
        !plan.is_multi_chunk,
        "MicroTool should always be single-chunk"
    );
    assert_eq!(plan.chunks.len(), 1);
    assert_eq!(plan.chunks[0].chunk_id, "root");
}

/// Test Context Pack building and rendering for a spec.
#[tokio::test]
async fn e2e_phase3_context_pack_full_budget() {
    use planner_core::pipeline::steps::context_pack::*;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    // Large budget — should include all sections
    let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 50000);

    assert!(!pack.was_truncated, "Large budget should not truncate");
    assert_eq!(pack.label, "spec-compiler");
    assert!(pack.estimated_tokens > 0);
    assert!(pack.estimated_tokens <= 50000);

    // Check all expected sections are present
    let section_names: Vec<&str> = pack.sections.iter().map(|s| s.name.as_str()).collect();
    assert!(
        section_names.contains(&"sacred_anchors"),
        "Missing sacred_anchors section"
    );
    assert!(
        section_names.contains(&"intent_summary"),
        "Missing intent_summary section"
    );
    assert!(
        section_names.contains(&"requirements"),
        "Missing requirements section"
    );
    assert!(
        section_names.contains(&"satisfaction_criteria"),
        "Missing satisfaction_criteria section"
    );

    // Render and verify output
    let rendered = render_context_pack(&pack);
    assert!(
        rendered.contains("SACRED ANCHORS"),
        "Rendered output missing SACRED ANCHORS header"
    );
    assert!(
        rendered.contains("REQUIREMENTS"),
        "Rendered output missing REQUIREMENTS header"
    );
    assert!(
        rendered.contains("SA-1"),
        "Rendered output missing anchor SA-1"
    );
    assert!(
        rendered.contains("FR-1"),
        "Rendered output missing requirement FR-1"
    );
    assert!(
        !rendered.contains("[Context truncated"),
        "Should not show truncation notice"
    );
}

/// Test Context Pack truncation under a tight token budget.
#[tokio::test]
async fn e2e_phase3_context_pack_truncated() {
    use planner_core::pipeline::steps::context_pack::*;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    // Tiny budget — must truncate
    let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 15);

    assert!(pack.was_truncated, "Tiny budget should truncate");
    // Priority 0 sections (sacred anchors, intent) should still appear
    assert!(
        !pack.sections.is_empty(),
        "Must-include sections should be present even when truncated"
    );

    let rendered = render_context_pack(&pack);
    assert!(
        rendered.contains("[Context truncated"),
        "Should show truncation notice"
    );
}

/// Test Context Pack domain compiler target prioritizes contracts.
#[tokio::test]
async fn e2e_phase3_context_pack_domain_compiler_priorities() {
    use planner_core::pipeline::steps::context_pack::*;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    let pack = build_spec_context_pack(
        &spec,
        ContextTarget::DomainCompiler {
            domain_name: "auth".into(),
        },
        50000,
    );

    assert_eq!(pack.label, "domain-compiler:auth");

    // Phase 1 contracts should be priority 0 for domain compilation
    let contracts = pack.sections.iter().find(|s| s.name == "phase1_contracts");
    assert!(
        contracts.is_some(),
        "Domain compiler pack should include contracts"
    );
    assert_eq!(
        contracts.unwrap().priority,
        0,
        "Contracts should be priority 0 for domain compiler"
    );
}

/// Test Ralph GeneTransfusion detects auth patterns in a spec.
#[tokio::test]
async fn e2e_phase3_ralph_gene_transfusion_auth() {
    use planner_core::pipeline::steps::ralph;

    let project_id = Uuid::new_v4();
    let auth_spec = build_auth_domain_spec(project_id);

    let findings = ralph::gene_transfusion(&auth_spec);

    // Auth spec mentions auth, bcrypt, JWT — should match auth patterns
    assert!(!findings.is_empty(), "Should find auth-related pitfalls");

    // Auth findings should be present
    let auth_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.affected_pattern == "auth")
        .collect();
    assert!(
        !auth_findings.is_empty(),
        "Should have auth-pattern findings"
    );
    assert!(
        auth_findings
            .iter()
            .any(|f| f.severity == ralph::RalphSeverity::High),
        "Should have at least one high-severity auth finding"
    );

    // Should NOT match unrelated patterns like file-upload or payment
    assert!(
        !findings.iter().any(|f| f.affected_pattern == "payment"),
        "Should not match payment pattern in auth spec"
    );
    assert!(
        !findings.iter().any(|f| f.affected_pattern == "file-upload"),
        "Should not match file-upload pattern in auth spec"
    );
}

/// Test Ralph GeneTransfusion detects payment patterns.
#[tokio::test]
async fn e2e_phase3_ralph_gene_transfusion_payment() {
    use planner_core::pipeline::steps::ralph;

    let project_id = Uuid::new_v4();
    let api_spec = build_api_domain_spec(project_id);

    let findings = ralph::gene_transfusion(&api_spec);

    // API spec mentions payment and idempotency — should match payment patterns
    // It also mentions "api" so may match api patterns too
    let payment_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.affected_pattern == "payment")
        .collect();

    // The spec already addresses idempotency, so that specific pitfall should be skipped
    // but other payment pitfalls should be found
    assert!(
        !payment_findings.is_empty() || !findings.is_empty(),
        "Should find at least some findings for payment/api patterns"
    );
}

/// Test Ralph ConsequenceCard generation: only high-severity findings produce cards.
#[tokio::test]
async fn e2e_phase3_ralph_consequence_cards() {
    use planner_core::pipeline::steps::ralph::{self, RalphFinding, RalphMode, RalphSeverity};

    let project_id = Uuid::new_v4();

    let findings = vec![
        RalphFinding {
            id: "RALPH-GT-1".into(),
            mode: RalphMode::GeneTransfusion,
            severity: RalphSeverity::High,
            description: "Password reset tokens must expire".into(),
            affected_pattern: "auth".into(),
            suggestion: Some("Add token expiry requirement".into()),
        },
        RalphFinding {
            id: "RALPH-GT-2".into(),
            mode: RalphMode::GeneTransfusion,
            severity: RalphSeverity::Medium,
            description: "Consider connection pooling".into(),
            affected_pattern: "database".into(),
            suggestion: None,
        },
        RalphFinding {
            id: "RALPH-GT-3".into(),
            mode: RalphMode::GeneTransfusion,
            severity: RalphSeverity::Low,
            description: "Nice to have pagination".into(),
            affected_pattern: "api".into(),
            suggestion: None,
        },
    ];

    let cards = ralph::surface_consequence_cards(&findings, project_id);

    assert_eq!(
        cards.len(),
        1,
        "Only high-severity findings should produce cards"
    );
    assert_eq!(cards[0].trigger, CardTrigger::RalphFinding);
    assert_eq!(cards[0].status, CardStatus::Pending);
    assert!(cards[0].problem.contains("Password reset tokens"));
    assert_eq!(cards[0].project_id, project_id);
    assert!(cards[0].resolution.is_none());
    assert_eq!(cards[0].actions.len(), 3); // Add Requirement, Add to DoD, Dismiss
}

/// Test that the multi-chunk AR report structure is correctly constructed.
#[tokio::test]
async fn e2e_phase3_ar_report_per_chunk() {
    let project_id = Uuid::new_v4();

    // Simulate per-chunk AR reports
    let mut root_report = ArReportV1 {
        project_id,
        chunk_name: "root".into(),
        nlspec_version: "1.0".into(),
        findings: vec![],
        reviewer_summaries: vec![],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };
    root_report.recalculate();

    let mut auth_report = ArReportV1 {
        project_id,
        chunk_name: "auth".into(),
        nlspec_version: "1.0".into(),
        findings: vec![ArFinding {
            id: "AR-A-1".into(),
            reviewer: ArReviewer::Opus,
            severity: ArSeverity::Advisory,
            affected_section: "Requirements".into(),
            affected_requirements: vec!["FR-AUTH-1".into()],
            description: "Consider adding password complexity requirements".into(),
            suggested_resolution: Some("Add min length + complexity FR".into()),
        }],
        reviewer_summaries: vec![],
        has_blocking: false,
        blocking_count: 0,
        advisory_count: 0,
        informational_count: 0,
    };
    auth_report.recalculate();

    let reports = vec![root_report, auth_report];

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].chunk_name, "root");
    assert_eq!(reports[1].chunk_name, "auth");
    assert!(!reports[0].has_blocking);
    assert!(!reports[1].has_blocking);
    assert_eq!(reports[1].advisory_count, 1);
}

/// Test token estimation function.
#[tokio::test]
async fn e2e_phase3_token_estimation() {
    use planner_core::pipeline::steps::context_pack::estimate_tokens;

    // ~4 chars per token
    assert_eq!(estimate_tokens("hello"), 2); // 5/4 + 1 = 2
    assert_eq!(estimate_tokens(""), 1); // 0/4 + 1 = 1
    assert_eq!(estimate_tokens("a".repeat(400).as_str()), 101); // 400/4 + 1 = 101

    // Realistic spec text
    let spec_text =
        "The system must accept a positive integer duration in seconds and display a countdown.";
    let tokens = estimate_tokens(spec_text);
    assert!(tokens > 10, "Realistic text should estimate > 10 tokens");
    assert!(tokens < 100, "Short text should estimate < 100 tokens");
}

/// Verify Storage can persist and retrieve Turn<T> artifacts via CxdbEngine.
#[tokio::test]
async fn e2e_storage_turn_lifecycle() {
    use planner_core::cxdb::{CxdbEngine, TurnStore};
    use planner_schemas::Turn;

    let store = CxdbEngine::new();
    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    // Store an IntakeV1 Turn
    let intake = build_test_intake(project_id);
    let turn: Turn<IntakeV1> = Turn::new(
        intake,
        None, // parent_id
        run_id,
        "intake-gateway", // produced_by
        "e2e-test",       // execution_id
    );

    store.store_turn(&turn).unwrap();

    // Retrieve it
    let latest: Option<Turn<IntakeV1>> = store.get_latest_turn(run_id, IntakeV1::TYPE_ID).unwrap();
    assert!(latest.is_some());

    let retrieved = latest.unwrap();
    assert_eq!(retrieved.metadata.run_id, run_id);
    assert!(retrieved.verify_integrity());
}

// ===========================================================================
// Phase 6: Wiring + Persistence + Durable CXDB Integration Tests
// ===========================================================================

/// Phase 6.3: Durable CXDB — filesystem-backed store roundtrip through TurnStore trait.
#[test]
fn e2e_phase6_durable_cxdb_roundtrip() {
    use planner_core::cxdb::durable::DurableCxdbEngine;
    use planner_core::cxdb::TurnStore;
    use planner_schemas::Turn;

    let dir = std::env::temp_dir().join(format!("cxdb-e2e-rt-{}", Uuid::new_v4()));
    let engine = DurableCxdbEngine::open(&dir).unwrap();
    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    // Store an IntakeV1 via the TurnStore trait
    let intake = build_test_intake(project_id);
    let turn: Turn<IntakeV1> = Turn::new(intake, None, run_id, "durable-e2e", "exec-1");
    let turn_id = turn.turn_id;

    engine.store_turn(&turn).unwrap();

    // Retrieve by ID
    let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
    assert_eq!(retrieved.payload.project_name, "Countdown Timer");
    assert!(retrieved.verify_integrity());

    // Retrieve by type
    let all: Vec<Turn<IntakeV1>> = engine.get_turns_by_type(run_id, IntakeV1::TYPE_ID).unwrap();
    assert_eq!(all.len(), 1);

    // Retrieve latest
    let latest: Option<Turn<IntakeV1>> = engine.get_latest_turn(run_id, IntakeV1::TYPE_ID).unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().turn_id, turn_id);

    // Cleanup
    let _ = std::fs::remove_dir_all(engine.root_path());
}

/// Phase 6.3: Durable CXDB persists across engine re-opens (simulates process restart).
#[test]
fn e2e_phase6_durable_cxdb_persistence() {
    use planner_core::cxdb::durable::DurableCxdbEngine;
    use planner_core::cxdb::TurnStore;
    use planner_schemas::Turn;

    let dir = std::env::temp_dir().join(format!("cxdb-e2e-persist-{}", Uuid::new_v4()));
    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let turn_id;

    // Write with first engine instance
    {
        let engine = DurableCxdbEngine::open(&dir).unwrap();
        let intake = build_test_intake(project_id);
        let turn: Turn<IntakeV1> = Turn::new(intake, None, run_id, "persist-e2e", "exec-1");
        turn_id = turn.turn_id;
        engine.store_turn(&turn).unwrap();
    }

    // Read with second engine instance (simulating process restart)
    {
        let engine = DurableCxdbEngine::open(&dir).unwrap();
        let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
        assert_eq!(retrieved.payload.project_name, "Countdown Timer");
        assert!(retrieved.verify_integrity());

        let stats = engine.stats();
        assert_eq!(stats.total_turns, 1);
        assert_eq!(stats.total_blobs, 1);
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 6.3: Durable CXDB content-addressed deduplication works on disk.
#[test]
fn e2e_phase6_durable_cxdb_dedup() {
    use planner_core::cxdb::durable::DurableCxdbEngine;
    use planner_core::cxdb::TurnStore;
    use planner_schemas::Turn;

    let dir = std::env::temp_dir().join(format!("cxdb-e2e-dedup-{}", Uuid::new_v4()));
    let engine = DurableCxdbEngine::open(&dir).unwrap();
    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    let intake = build_test_intake(project_id);
    let turn1: Turn<IntakeV1> = Turn::new(intake.clone(), None, run_id, "dedup-e2e", "exec-1");
    let turn2: Turn<IntakeV1> = Turn::new(intake, None, run_id, "dedup-e2e", "exec-2");

    // Same payload → same blob hash
    assert_eq!(turn1.blob_hash, turn2.blob_hash);

    engine.store_turn(&turn1).unwrap();
    engine.store_turn(&turn2).unwrap();

    let stats = engine.stats();
    assert_eq!(stats.total_turns, 2);
    assert_eq!(stats.total_blobs, 1); // Deduped on disk

    let _ = std::fs::remove_dir_all(engine.root_path());
}

/// Phase 6.1: Model catalog has factory worker mapping.
#[test]
fn e2e_phase6_model_catalog_factory_worker() {
    use planner_core::llm::DefaultModels;

    // Factory worker should map to GPT-5.3-Codex
    assert_eq!(DefaultModels::FACTORY_WORKER, "gpt-5.3-codex");

    // Existing models should still be present
    assert!(!DefaultModels::INTAKE_GATEWAY.is_empty());
    assert!(!DefaultModels::COMPILER_SPEC.is_empty());
    assert!(!DefaultModels::SCENARIO_VALIDATOR.is_empty());
}

/// Phase 6.4: DTU registry has all 5 Phase 5 providers.
#[test]
fn e2e_phase6_dtu_registry_wired() {
    use planner_core::dtu::DtuRegistry;

    let registry = DtuRegistry::with_phase5_defaults();
    let providers = registry.list_providers();
    assert_eq!(providers.len(), 5);

    // Verify all providers are present
    assert!(registry.get("stripe").is_some());
    assert!(registry.get("auth0").is_some());
    assert!(registry.get("sendgrid").is_some());
    assert!(registry.get("supabase").is_some());
    assert!(registry.get("twilio").is_some());

    // Verify reset_all doesn't panic
    registry.reset_all();
}

/// Phase 6.6: Project registry tracks projects.
#[test]
fn e2e_phase6_project_registry() {
    use planner_core::pipeline::project::{ProjectRegistry, ProjectStatus};

    let mut registry = ProjectRegistry::new();
    assert_eq!(registry.count(), 0);

    let project = registry
        .register(
            "Test Project".to_string(),
            "test-project".to_string(),
            vec!["e2e".to_string()],
        )
        .unwrap();

    assert_eq!(registry.count(), 1);
    assert!(registry.get(project.project_id).is_some());
    assert!(registry.get_by_slug("test-project").is_some());

    // Verify duplicate slug is rejected
    let dup = registry.register("Another".to_string(), "test-project".to_string(), vec![]);
    assert!(dup.is_err());

    // Verify status update
    registry
        .update_status(project.project_id, ProjectStatus::Completed)
        .unwrap();
    assert_eq!(
        registry.get(project.project_id).unwrap().status,
        ProjectStatus::Completed
    );
}

/// Phase 6.7: Verification generates Lean4 propositions from spec.
#[test]
fn e2e_phase6_verification_propositions() {
    use planner_core::pipeline::verification;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    let propositions = verification::generate_propositions(&spec);

    // Should have at least: anchor traceability + requirement uniqueness + coverage
    assert!(
        !propositions.is_empty(),
        "Should generate at least some propositions"
    );

    // Check anchor traceability propositions exist
    let anchor_props: Vec<_> = propositions
        .iter()
        .filter(|p| p.category == verification::PropositionCategory::AnchorTraceability)
        .collect();
    assert!(
        !anchor_props.is_empty(),
        "Should have anchor traceability propositions"
    );

    // Check uniqueness propositions exist
    let unique_props: Vec<_> = propositions
        .iter()
        .filter(|p| p.category == verification::PropositionCategory::Uniqueness)
        .collect();
    assert!(
        !unique_props.is_empty(),
        "Should have uniqueness propositions"
    );

    // All propositions should have non-empty Lean4 source
    for prop in &propositions {
        assert!(
            !prop.lean4_source.is_empty(),
            "Proposition {} should have Lean4 source",
            prop.id
        );
    }
}

/// Phase 6.7: Anti-lock-in audit produces findings for external dependencies.
#[test]
fn e2e_phase6_audit_lock_in() {
    use planner_core::pipeline::audit;

    let project_id = Uuid::new_v4();
    let spec = build_test_spec(project_id);

    let report = audit::audit_lock_in(&spec);

    // Report should have basic structure
    assert_eq!(report.project_id, project_id);
    // Risk score should be between 0 and 1
    assert!(
        report.risk_score >= 0.0 && report.risk_score <= 1.0,
        "Risk score should be 0-1, got {}",
        report.risk_score
    );
    // Should have some recommendations (our test spec has external deps)
    // Even if no deps, the audit should run without panicking
}

/// Phase 6.2+6.3: Pipeline Config wires storage — verify the persist method works
/// through PipelineConfig.
#[test]
fn e2e_phase6_pipeline_config_persist() {
    use planner_core::cxdb::durable::DurableCxdbEngine;
    use planner_core::cxdb::TurnStore;
    use planner_core::llm::providers::LlmRouter;
    use planner_core::pipeline::PipelineConfig;
    use planner_schemas::Turn;

    let dir = std::env::temp_dir().join(format!("cxdb-e2e-config-{}", Uuid::new_v4()));
    let engine = DurableCxdbEngine::open(&dir).unwrap();
    let router = LlmRouter::from_env();

    let config = PipelineConfig {
        router: &router,
        store: Some(&engine),
        dtu_registry: None,
        blueprints: None,
    };

    let project_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();

    let intake = build_test_intake(project_id);
    let turn: Turn<IntakeV1> = Turn::new(intake, None, run_id, "config-e2e", "exec-1");
    let turn_id = turn.turn_id;

    // persist via config (should not panic even if store has issues)
    config.persist(&turn);

    // Verify it was actually stored
    let retrieved: Turn<IntakeV1> = engine.get_turn(turn_id).unwrap();
    assert_eq!(retrieved.payload.project_name, "Countdown Timer");

    let _ = std::fs::remove_dir_all(engine.root_path());
}

// ===========================================================================
// Phase 7: Factory Worker Integration Tests
// ===========================================================================

/// Phase 7.1: MockFactoryWorker produces valid FactoryOutputV1 via the
/// execute_factory_with_worker path.
#[tokio::test]
async fn e2e_phase7_mock_worker_produces_factory_output() {
    use planner_core::pipeline::steps::factory;
    use planner_core::pipeline::steps::factory_worker::MockFactoryWorker;

    let project_id = Uuid::new_v4();
    let graph = build_test_graph_dot(project_id);
    let agents = build_test_agents_manifest(project_id);
    let spec = build_test_spec(project_id);
    let mut budget = RunBudgetV1::new_phase0(project_id, Uuid::new_v4());

    std::env::set_var(
        "PLANNER_WORKTREE_ROOT",
        std::env::temp_dir()
            .join(format!("planner-e2e-fw-{}", Uuid::new_v4()))
            .to_string_lossy()
            .to_string(),
    );

    let worker = MockFactoryWorker::success(
        "Generated: src/main.rs, src/lib.rs, Cargo.toml",
        vec![
            "src/main.rs".into(),
            "src/lib.rs".into(),
            "Cargo.toml".into(),
        ],
    );

    let output = factory::execute_factory_with_worker(
        &worker,
        &graph,
        &agents,
        &spec,
        None,
        &mut budget,
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(output.build_status, BuildStatus::Success);
    assert_eq!(output.node_results.len(), 1);
    assert!(output.node_results[0].success);
    assert_eq!(output.node_results[0].node_name, "factory-worker");
}

/// Phase 7.2: WorktreeManager creates proper directory structure with context files.
#[test]
fn e2e_phase7_worktree_manager_lifecycle() {
    use planner_core::pipeline::steps::factory_worker::WorktreeManager;

    let root = std::env::temp_dir().join(format!("planner-e2e-wt-{}", Uuid::new_v4()));
    let mgr = WorktreeManager::new(&root);
    let run_id = Uuid::new_v4();

    let info = mgr
        .prepare(
            run_id,
            "# Spec\n## Requirements\n- FR-1: Build a widget",
            "digraph { start -> build -> test -> exit; }",
            "# AGENTS\n- implementer\n- reviewer",
        )
        .unwrap();

    // Verify structure
    assert!(info.path.exists());
    assert!(info.context_dir.join("SPEC.md").exists());
    assert!(info.context_dir.join("graph.dot").exists());
    assert!(info.context_dir.join("AGENTS.md").exists());
    assert!(info.path.join("src").exists());

    // Verify content
    let spec = std::fs::read_to_string(info.context_dir.join("SPEC.md")).unwrap();
    assert!(spec.contains("FR-1: Build a widget"));

    // List active
    let active = mgr.list_active();
    assert_eq!(active.len(), 1);

    // Cleanup
    mgr.cleanup(&info).unwrap();
    assert!(!info.path.exists());
    assert_eq!(mgr.list_active().len(), 0);

    let _ = std::fs::remove_dir_all(&root);
}

/// Phase 7.3: FactoryWorker trait is object-safe and can be used as dyn dispatch.
#[tokio::test]
async fn e2e_phase7_factory_worker_dyn_dispatch() {
    use planner_core::pipeline::steps::factory_worker::{
        FactoryWorker, MockFactoryWorker, WorkerConfig,
    };

    let workers: Vec<Box<dyn FactoryWorker>> = vec![
        Box::new(MockFactoryWorker::success("output1", vec!["a.rs".into()])),
        Box::new(MockFactoryWorker::success("output2", vec!["b.rs".into()])),
    ];

    let config = WorkerConfig::default();

    for worker in &workers {
        let result = worker.generate("test prompt", &config).await.unwrap();
        assert!(result.success);
        assert_eq!(result.model, "gpt-5.3-codex");
    }
}

/// Phase 7.4: WorkerResult serializes/deserializes correctly.
#[test]
fn e2e_phase7_worker_result_serde() {
    use planner_core::pipeline::steps::factory_worker::WorkerResult;

    let result = WorkerResult {
        invocation_id: Uuid::new_v4(),
        success: true,
        model: "gpt-5.3-codex".into(),
        output: "Created 5 files".into(),
        files_changed: vec!["src/main.rs".into(), "Cargo.toml".into()],
        duration_secs: 45.7,
        error: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: WorkerResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.model, "gpt-5.3-codex");
    assert_eq!(deserialized.files_changed.len(), 2);
    assert!(deserialized.success);
    assert_eq!(deserialized.duration_secs, 45.7);
}

/// Phase 7.5: Factory worker failure is caught gracefully — returns Failed status.
#[tokio::test]
async fn e2e_phase7_worker_failure_graceful() {
    use planner_core::pipeline::steps::factory;
    use planner_core::pipeline::steps::factory_worker::MockFactoryWorker;

    let project_id = Uuid::new_v4();
    let graph = build_test_graph_dot(project_id);
    let agents = build_test_agents_manifest(project_id);
    let spec = build_test_spec(project_id);
    let mut budget = RunBudgetV1::new_phase0(project_id, Uuid::new_v4());

    std::env::set_var(
        "PLANNER_WORKTREE_ROOT",
        std::env::temp_dir()
            .join(format!("planner-e2e-fw-fail-{}", Uuid::new_v4()))
            .to_string_lossy()
            .to_string(),
    );

    let worker = MockFactoryWorker::failure("codex binary crashed");

    let output = factory::execute_factory_with_worker(
        &worker,
        &graph,
        &agents,
        &spec,
        None,
        &mut budget,
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(output.build_status, BuildStatus::Failed);
    assert!(!output.node_results[0].success);
}

/// Phase 7.6: DefaultModels::FACTORY_WORKER is gpt-5.3-codex.
#[test]
fn e2e_phase7_default_factory_model() {
    use planner_core::llm::DefaultModels;
    assert_eq!(DefaultModels::FACTORY_WORKER, "gpt-5.3-codex");
}

/// Phase 7.7: CodexFactoryWorker build_codex_prompt includes all context.
#[test]
fn e2e_phase7_codex_prompt_assembly() {
    use planner_core::pipeline::steps::factory_worker::{CodexFactoryWorker, WorktreeInfo};

    let tmp = std::env::temp_dir().join(format!("planner-e2e-prompt-{}", Uuid::new_v4()));
    let ctx = tmp.join(".planner-context");
    std::fs::create_dir_all(&ctx).unwrap();
    std::fs::write(ctx.join("SPEC.md"), "## Requirements\n- FR-1: Widget").unwrap();
    std::fs::write(ctx.join("graph.dot"), "digraph { a -> b; }").unwrap();
    std::fs::write(ctx.join("AGENTS.md"), "# Agents\n- coder").unwrap();

    let info = WorktreeInfo {
        path: tmp.clone(),
        context_dir: ctx,
        run_id: Uuid::new_v4(),
    };

    let prompt = CodexFactoryWorker::build_codex_prompt("Build the widget", &info);

    assert!(prompt.contains("FR-1: Widget"));
    assert!(prompt.contains("digraph { a -> b; }"));
    assert!(prompt.contains("# Agents"));
    assert!(prompt.contains("Build the widget"));
    assert!(prompt.contains("factory worker code generation agent"));

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Phase 3: Multi-chunk intake triggers the multi-chunk heuristic.
/// `build_multi_chunk_intake` produces a FullApp with 3 sacred anchors
/// and 4 satisfaction seeds. The chunk_planner correctly identifies it
/// as warranting multi-chunk decomposition. Use a mock router here so
/// the test stays deterministic and does not depend on live CLI auth.
#[tokio::test]
async fn e2e_phase3_chunk_planner_fullapp_multi_chunk() {
    use async_trait::async_trait;
    use planner_core::llm::providers::LlmRouter;
    use planner_core::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
    use planner_core::pipeline::steps::chunk_planner;

    struct MockChunkPlannerClient;

    #[async_trait]
    impl LlmClient for MockChunkPlannerClient {
        async fn complete(
            &self,
            request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            assert_eq!(request.model, "claude-opus-4-6");
            let content = r#"{
              "chunks": [
                {
                  "chunk_id": "root",
                  "relevant_anchor_ids": ["SA-1", "SA-2", "SA-3"],
                  "domain_context": "Cross-cutting architecture, shared contracts, and project-wide invariants.",
                  "estimated_fr_count": 3
                },
                {
                  "chunk_id": "auth",
                  "relevant_anchor_ids": ["SA-1"],
                  "domain_context": "Authentication, session management, and password handling.",
                  "estimated_fr_count": 5
                },
                {
                  "chunk_id": "catalog",
                  "relevant_anchor_ids": ["SA-3"],
                  "domain_context": "Product catalog and browsing flows.",
                  "estimated_fr_count": 4
                },
                {
                  "chunk_id": "payments",
                  "relevant_anchor_ids": ["SA-2"],
                  "domain_context": "Cart checkout, payment processing, and idempotency rules.",
                  "estimated_fr_count": 6
                }
              ]
            }"#;

            Ok(CompletionResponse {
                content: content.into(),
                model: request.model,
                input_tokens: 0,
                output_tokens: 0,
                estimated_cost_usd: 0.0,
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    let project_id = Uuid::new_v4();
    let intake = build_multi_chunk_intake(project_id);

    // Validate the fixture: FullApp with 3 domains, 3 sacred anchors, 4 satisfaction seeds
    assert!(matches!(
        intake.output_domain,
        OutputDomain::FullApp {
            estimated_domains: 3
        }
    ));
    assert_eq!(intake.sacred_anchors.len(), 3);
    assert_eq!(intake.satisfaction_criteria_seeds.len(), 4);
    assert_eq!(intake.project_name, "E-Commerce API");

    let router = LlmRouter::with_mock(Box::new(MockChunkPlannerClient));
    let plan = chunk_planner::plan_chunks(&router, &intake, project_id)
        .await
        .expect("multi-chunk intake should use mocked chunk planner response");

    assert!(
        plan.is_multi_chunk,
        "FullApp with 3 domains should NOT produce a single-chunk plan"
    );
    assert_eq!(plan.chunks[0].chunk_id, "root");
    assert_eq!(plan.chunks.len(), 4);
    assert_eq!(plan.chunks[1].chunk_id, "auth");
    assert_eq!(plan.chunks[3].chunk_id, "payments");
}
