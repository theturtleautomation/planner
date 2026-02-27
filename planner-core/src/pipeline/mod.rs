//! # Pipeline — Recipe Step Loop
//!
//! The Phase 1 pipeline orchestrates the Dark Factory workflow:
//!
//! 1. **Intake** — Socratic interview → IntakeV1
//! 2. **Compile** — IntakeV1 → NLSpecV1 + GraphDotV1 + ScenarioSetV1 + AgentsManifestV1
//! 3. **Handoff** — Factory Diplomat → Kilroy CLI invocation
//! 4. **Validate** — Scenario Validator → SatisfactionResultV1
//! 5. **Retry** — If gates fail and budget allows, re-run Factory (up to 2 retries)
//! 6. **Present** — Telemetry Presenter → Plain English + Consequence Cards
//! 7. **Approve** — Behavioral approval → Git Projection

pub mod steps;

use uuid::Uuid;

use crate::llm::providers::LlmRouter;
use planner_schemas::*;

use steps::StepResult;
use steps::intake;
use steps::compile;
use steps::linter;
use steps::factory;
use steps::validate;
use steps::telemetry;
use steps::git;

/// The recipe — a versioned DAG of steps defining the complete workflow.
/// Phase 0 uses a linear sequence; Phase 3+ introduces parallel branches.
#[derive(Debug)]
pub struct Recipe {
    pub recipe_id: Uuid,
    pub version: String,
    pub steps: Vec<PipelineStep>,
}

/// A single step in the pipeline recipe.
#[derive(Debug, Clone)]
pub struct PipelineStep {
    pub step_id: String,
    pub name: String,
    pub step_type: StepType,
    pub depends_on: Vec<String>,
}

/// The type of pipeline step.
#[derive(Debug, Clone)]
pub enum StepType {
    /// Socratic interview → IntakeV1.
    Intake,
    /// IntakeV1 → NLSpecV1 (single root chunk in Phase 0).
    CompileSpec,
    /// NLSpecV1 → 12-rule linting.
    LintSpec,
    /// NLSpecV1 → GraphDotV1.
    CompileGraphDot,
    /// NLSpecV1 + Sacred Anchors → ScenarioSetV1 (critical tier only in Phase 0).
    GenerateScenarios,
    /// NLSpecV1 → AgentsManifestV1.
    CompileAgentsManifest,
    /// Drop artifacts + invoke Kilroy CLI.
    FactoryHandoff,
    /// Poll checkpoint.json for Kilroy completion.
    FactoryPoll,
    /// Cross-model scenario evaluation → SatisfactionResultV1.
    ValidateScenarios,
    /// Kilroy output → Docker sandbox → Live Preview URL.
    DeploySandbox,
    /// Satisfaction scores + Kilroy logs → plain English.
    PresentTelemetry,
    /// User behavioral approval.
    AwaitApproval,
    /// Approved code → Git commit.
    GitProjection,
}

impl Recipe {
    /// Create the Phase 0 linear recipe.
    pub fn phase0() -> Self {
        let steps = vec![
            PipelineStep {
                step_id: "intake".into(),
                name: "Socratic Interview".into(),
                step_type: StepType::Intake,
                depends_on: vec![],
            },
            PipelineStep {
                step_id: "compile-spec".into(),
                name: "Compile NLSpec".into(),
                step_type: StepType::CompileSpec,
                depends_on: vec!["intake".into()],
            },
            PipelineStep {
                step_id: "lint-spec".into(),
                name: "Spec Linter".into(),
                step_type: StepType::LintSpec,
                depends_on: vec!["compile-spec".into()],
            },
            PipelineStep {
                step_id: "compile-graph-dot".into(),
                name: "Generate graph.dot".into(),
                step_type: StepType::CompileGraphDot,
                depends_on: vec!["lint-spec".into()],
            },
            PipelineStep {
                step_id: "generate-scenarios".into(),
                name: "Generate Scenarios".into(),
                step_type: StepType::GenerateScenarios,
                depends_on: vec!["lint-spec".into()],
            },
            PipelineStep {
                step_id: "compile-agents-manifest".into(),
                name: "Generate AGENTS.md".into(),
                step_type: StepType::CompileAgentsManifest,
                depends_on: vec!["lint-spec".into()],
            },
            PipelineStep {
                step_id: "factory-handoff".into(),
                name: "Factory Diplomat Handoff".into(),
                step_type: StepType::FactoryHandoff,
                depends_on: vec![
                    "compile-graph-dot".into(),
                    "generate-scenarios".into(),
                    "compile-agents-manifest".into(),
                ],
            },
            PipelineStep {
                step_id: "factory-poll".into(),
                name: "Poll Kilroy Checkpoint".into(),
                step_type: StepType::FactoryPoll,
                depends_on: vec!["factory-handoff".into()],
            },
            PipelineStep {
                step_id: "validate-scenarios".into(),
                name: "Scenario Validator".into(),
                step_type: StepType::ValidateScenarios,
                depends_on: vec!["factory-poll".into()],
            },
            PipelineStep {
                step_id: "deploy-sandbox".into(),
                name: "Deploy Sandbox".into(),
                step_type: StepType::DeploySandbox,
                depends_on: vec!["factory-poll".into()],
            },
            PipelineStep {
                step_id: "present-telemetry".into(),
                name: "Telemetry Presenter".into(),
                step_type: StepType::PresentTelemetry,
                depends_on: vec![
                    "validate-scenarios".into(),
                    "deploy-sandbox".into(),
                ],
            },
            PipelineStep {
                step_id: "await-approval".into(),
                name: "Behavioral Approval".into(),
                step_type: StepType::AwaitApproval,
                depends_on: vec!["present-telemetry".into()],
            },
            PipelineStep {
                step_id: "git-projection".into(),
                name: "Git Projection".into(),
                step_type: StepType::GitProjection,
                depends_on: vec!["await-approval".into()],
            },
        ];

        Recipe {
            recipe_id: Uuid::new_v4(),
            version: "phase0-v1".into(),
            steps,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 0 Pipeline Runner — Front Office
// ---------------------------------------------------------------------------

/// Phase 0 pipeline execution output — all the artifacts produced by the
/// Front Office before Kilroy handoff.
#[derive(Debug)]
pub struct Phase0FrontOfficeOutput {
    pub intake: IntakeV1,
    pub spec: NLSpecV1,
    pub graph_dot: GraphDotV1,
    pub scenarios: ScenarioSetV1,
    pub agents_manifest: AgentsManifestV1,
}

/// Run the Phase 0 Front Office pipeline: user description → all compilation
/// artifacts ready for Kilroy handoff.
///
/// Steps: Intake → Compile Spec → Lint → (GraphDot + Scenarios + AGENTS.md)
pub async fn run_phase0_front_office(
    router: &LlmRouter,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FrontOfficeOutput> {
    tracing::info!("Phase 0 Front Office: starting pipeline");

    // Step 1: Intake Gateway
    tracing::info!("Step 1/6: Intake Gateway");
    let intake_result = intake::execute_intake(router, project_id, user_description).await?;
    tracing::info!("  → IntakeV1 produced: {}", intake_result.project_name);

    // Step 2: Compile Spec
    tracing::info!("Step 2/6: Compile NLSpec");
    let spec = compile::compile_spec(router, &intake_result).await?;
    tracing::info!("  → NLSpecV1 produced: {} requirements, {} satisfaction criteria",
        spec.requirements.len(), spec.satisfaction_criteria.len());

    // Step 3: Lint Spec
    tracing::info!("Step 3/6: Spec Linter");
    linter::lint_spec(&spec)?;
    tracing::info!("  → Spec passes all 12 linting rules");

    // Steps 4-6 can run in parallel (all depend on linted spec only)
    // Phase 0: run sequentially for simplicity
    tracing::info!("Step 4/6: Compile graph.dot");
    let graph_dot = compile::compile_graph_dot(router, &spec).await?;
    tracing::info!("  → GraphDotV1 produced: {} nodes, ${:.2} estimated cost",
        graph_dot.node_count, graph_dot.estimated_cost_usd);

    tracing::info!("Step 5/6: Generate Scenarios");
    let scenarios = compile::generate_scenarios(router, &spec).await?;
    tracing::info!("  → ScenarioSetV1 produced: {} scenarios",
        scenarios.scenarios.len());

    tracing::info!("Step 6/6: Generate AGENTS.md");
    let agents_manifest = compile::compile_agents_manifest(router, &spec).await?;
    tracing::info!("  → AgentsManifestV1 produced: {} bytes",
        agents_manifest.root_agents_md.len());

    tracing::info!("Phase 0 Front Office: pipeline complete — ready for Kilroy handoff");

    Ok(Phase0FrontOfficeOutput {
        intake: intake_result,
        spec,
        graph_dot,
        scenarios,
        agents_manifest,
    })
}

// ---------------------------------------------------------------------------
// Phase 0 Pipeline Runner — Full Loop
// ---------------------------------------------------------------------------

/// Complete Phase 0 pipeline output — end-to-end.
#[derive(Debug)]
pub struct Phase0FullOutput {
    /// Front Office artifacts.
    pub front_office: Phase0FrontOfficeOutput,
    /// Factory output (Kilroy run results).
    pub factory_output: FactoryOutputV1,
    /// Scenario validation results.
    pub satisfaction: SatisfactionResultV1,
    /// Telemetry report (plain English for the user).
    pub telemetry: telemetry::TelemetryReport,
    /// Git projection result.
    pub git_result: git::GitProjectionResult,
    /// Budget state at end of run.
    pub budget: RunBudgetV1,
}

/// Maximum factory retry attempts when validation gates fail.
const FACTORY_MAX_RETRIES: usize = 2;

/// Run the complete Phase 0 pipeline: user description → approved Git commit.
///
/// This is the full loop:
/// Front Office → Factory Diplomat → Scenario Validator →
///   (retry Factory if gates fail and budget allows) →
///   Telemetry Presenter → Git Projection
///
/// Phase 1: Factory retry loop re-invokes Kilroy with generalized errors
/// when validation gates fail, up to FACTORY_MAX_RETRIES additional attempts
/// within the budget cap.
pub async fn run_phase0_full(
    router: &LlmRouter,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Planner v2 — Phase 1 Full Pipeline");
    tracing::info!("═══════════════════════════════════════════════");

    // ---- Layer 1: Front Office ----
    let front_office = run_phase0_front_office(router, project_id, user_description).await?;

    // ---- Layer 1→2: Factory Diplomat + Validation Loop ----
    let run_id = Uuid::new_v4();
    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);
    let mut factory_output;
    let mut satisfaction;
    let mut attempt = 0usize;

    loop {
        attempt += 1;
        tracing::info!("─── Factory Diplomat (attempt {}/{}) ───", attempt, FACTORY_MAX_RETRIES + 1);

        factory_output = factory::execute_factory_handoff(
            &front_office.graph_dot,
            &front_office.agents_manifest,
            &front_office.spec,
            &mut budget,
        )
        .await?;

        tracing::info!(
            "  Factory: status={:?}, spend=${:.2}",
            factory_output.build_status,
            factory_output.spend_usd,
        );

        // ---- Layer 3: Return Trip — Validate ----
        tracing::info!("─── Scenario Validator (attempt {}) ───", attempt);
        satisfaction = validate::execute_scenario_validation(
            router,
            &front_office.scenarios,
            &factory_output,
        )
        .await?;

        // Check if we passed or should retry
        if satisfaction.gates_passed {
            tracing::info!("  Gates PASSED on attempt {}", attempt);
            break;
        }

        // Gates failed — decide whether to retry
        if attempt > FACTORY_MAX_RETRIES {
            tracing::warn!(
                "  Gates FAILED after {} attempts — no more retries",
                attempt,
            );
            break;
        }

        if !budget.can_proceed() {
            tracing::warn!(
                "  Gates FAILED on attempt {} — budget exhausted, cannot retry",
                attempt,
            );
            break;
        }

        // Log the generalized errors being fed back to the factory
        let error_categories: Vec<&str> = satisfaction
            .scenario_results
            .iter()
            .filter_map(|r| r.generalized_error.as_ref())
            .map(|e| e.category.as_str())
            .collect();

        tracing::info!(
            "  Gates FAILED on attempt {} — retrying with error feedback: {:?}",
            attempt,
            error_categories,
        );
    }

    // ---- Telemetry Presenter ----
    tracing::info!("─── Telemetry Presenter ───");
    let telemetry = telemetry::execute_telemetry_presentation(
        router,
        &factory_output,
        &satisfaction,
        &budget,
        project_id,
    )
    .await?;

    // ---- Git Projection ----
    // Phase 0: Auto-approve (no behavioral approval gate)
    tracing::info!("─── Git Projection ───");
    let git_result = git::execute_git_projection(
        &factory_output,
        project_id,
        &front_office.intake.project_name,
        &front_office.intake.feature_slug,
    )
    .await?;

    // ---- Done ----
    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Pipeline Complete ({} factory attempt(s))", attempt);
    tracing::info!("  {}", telemetry.headline);
    tracing::info!("  Commit: {}", &git_result.commit.commit_hash[..12.min(git_result.commit.commit_hash.len())]);
    tracing::info!("═══════════════════════════════════════════════");

    Ok(Phase0FullOutput {
        front_office,
        factory_output,
        satisfaction,
        telemetry,
        git_result,
        budget,
    })
}
