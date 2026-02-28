//! # Pipeline — Recipe Step Loop
//!
//! The Phase 1 pipeline orchestrates the Dark Factory workflow:
//!
//! 1. **Intake** — Socratic interview → IntakeV1
//! 2. **Chunk Plan** — IntakeV1 → ChunkPlan (single vs multi-chunk decision)
//! 3. **Compile** — IntakeV1 → NLSpecV1 (single) or Vec<NLSpecV1> (multi-chunk)
//! 4. **Lint** — 12-rule NLSpec validation + cross-chunk rules
//! 5. **Adversarial Review** — 3-model parallel NLSpec review + coherence → ArReportV1
//! 6. **AR Refinement** — Blocking findings → spec amendments → re-lint loop
//! 7. **Handoff** — Factory Diplomat → Kilroy CLI invocation
//! 8. **Validate** — Scenario Validator → SatisfactionResultV1
//! 9. **Retry** — If gates fail and budget allows, re-run Factory (up to 2 retries)
//! 10. **Present** — Telemetry Presenter → Plain English + Consequence Cards
//! 11. **Approve** — Behavioral approval → Git Projection

pub mod steps;
pub mod pyramid;
pub mod project;
pub mod verification;
pub mod audit;

use uuid::Uuid;

use crate::llm::providers::LlmRouter;
use crate::storage::TurnStore;
use crate::dtu::DtuRegistry;
use planner_schemas::*;

use steps::StepResult;
use steps::intake;
use steps::compile;
use steps::linter;
use steps::chunk_planner;
use steps::ar;
use steps::ar_refinement;
use steps::ralph;
use steps::factory;
use steps::factory_worker;
use steps::validate;
use steps::telemetry;
use steps::git;

/// Pipeline configuration bundle — carries storage, DTU, and project
/// registry references through the pipeline.
pub struct PipelineConfig<'a, S: TurnStore> {
    /// The LLM router for model resolution.
    pub router: &'a LlmRouter,
    /// Optional durable storage — if Some, each artifact is persisted.
    pub store: Option<&'a S>,
    /// Optional DTU registry — if Some, scenario validation uses DTU clones.
    pub dtu_registry: Option<&'a DtuRegistry>,
}

impl<'a, S: TurnStore> PipelineConfig<'a, S> {
    /// Create a minimal config (router only, no storage or DTU).
    pub fn minimal(router: &'a LlmRouter) -> Self {
        PipelineConfig {
            router,
            store: None,
            dtu_registry: None,
        }
    }

    /// Persist a Turn if storage is configured.
    pub fn persist<T: ArtifactPayload>(&self, turn: &Turn<T>) {
        if let Some(store) = self.store {
            if let Err(e) = store.store_turn(turn) {
                tracing::warn!("Storage: failed to persist turn {}: {}", turn.turn_id, e);
            } else {
                tracing::debug!("Storage: persisted {} (type={})", turn.turn_id, turn.type_id);
            }
        }
    }
}

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
    /// IntakeV1 → ChunkPlan (single vs multi-chunk decision).
    ChunkPlan,
    /// IntakeV1 → NLSpecV1 (single root chunk in Phase 0).
    CompileSpec,
    /// NLSpecV1 → 12-rule linting.
    LintSpec,
    /// NLSpecV1 → 3-model Adversarial Review.
    AdversarialReview,
    /// Blocking AR findings → spec amendments → re-lint.
    ArRefinement,
    /// Scenario Augmentation + Gene Transfusion.
    RalphLoop,
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
                step_id: "chunk-plan".into(),
                name: "Chunk Planner".into(),
                step_type: StepType::ChunkPlan,
                depends_on: vec!["intake".into()],
            },
            PipelineStep {
                step_id: "compile-spec".into(),
                name: "Compile NLSpec".into(),
                step_type: StepType::CompileSpec,
                depends_on: vec!["chunk-plan".into()],
            },
            PipelineStep {
                step_id: "lint-spec".into(),
                name: "Spec Linter".into(),
                step_type: StepType::LintSpec,
                depends_on: vec!["compile-spec".into()],
            },
            PipelineStep {
                step_id: "adversarial-review".into(),
                name: "Adversarial Review".into(),
                step_type: StepType::AdversarialReview,
                depends_on: vec!["lint-spec".into()],
            },
            PipelineStep {
                step_id: "ar-refinement".into(),
                name: "AR Refinement".into(),
                step_type: StepType::ArRefinement,
                depends_on: vec!["adversarial-review".into()],
            },
            PipelineStep {
                step_id: "generate-scenarios".into(),
                name: "Generate Scenarios".into(),
                step_type: StepType::GenerateScenarios,
                depends_on: vec!["ar-refinement".into()],
            },
            PipelineStep {
                step_id: "ralph-loop".into(),
                name: "Ralph Advisory Loop".into(),
                step_type: StepType::RalphLoop,
                depends_on: vec!["generate-scenarios".into()],
            },
            PipelineStep {
                step_id: "compile-graph-dot".into(),
                name: "Generate graph.dot".into(),
                step_type: StepType::CompileGraphDot,
                depends_on: vec!["ralph-loop".into()],
            },
            PipelineStep {
                step_id: "compile-agents-manifest".into(),
                name: "Generate AGENTS.md".into(),
                step_type: StepType::CompileAgentsManifest,
                depends_on: vec!["ralph-loop".into()],
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
    pub specs: Vec<NLSpecV1>,
    pub ar_reports: Vec<ArReportV1>,
    pub graph_dot: GraphDotV1,
    pub scenarios: ScenarioSetV1,
    pub agents_manifest: AgentsManifestV1,
    /// Lean4 formal verification propositions (Phase 6 wiring).
    pub propositions: Vec<verification::Lean4Proposition>,
    /// Anti-lock-in audit report (Phase 6 wiring).
    pub audit_report: audit::LockInAuditReport,
}

/// Run the Phase 0 Front Office pipeline: user description → all compilation
/// artifacts ready for Kilroy handoff.
///
/// Phase 3: Now supports multi-chunk compilation via ChunkPlan.
/// Phase 6: Accepts PipelineConfig for storage persistence, DTU, and
///          project registry wiring. Also runs verification + audit.
///
/// Steps: Intake → ChunkPlan → Compile Spec(s) → Lint → AR Review →
///        AR Refinement → (GraphDot + Scenarios + AGENTS.md) →
///        Verification → Audit
pub async fn run_phase0_front_office_with_config<S: TurnStore>(
    config: &PipelineConfig<'_, S>,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FrontOfficeOutput> {
    let router = config.router;
    tracing::info!("Phase 6 Front Office: starting pipeline");

    // Step 1: Intake Gateway
    tracing::info!("Step 1: Intake Gateway");
    let intake_result = intake::execute_intake(router, project_id, user_description).await?;
    tracing::info!("  → IntakeV1 produced: {}", intake_result.project_name);

    // Persist intake as a Turn
    {
        let run_id = Uuid::new_v4();
        let turn = Turn::new(intake_result.clone(), None, run_id, "front-office", "intake");
        config.persist(&turn);
    }

    // Step 2: Chunk Planning
    tracing::info!("Step 2: Chunk Planner");
    let chunk_plan = chunk_planner::plan_chunks(router, &intake_result, project_id).await?;
    tracing::info!(
        "  → ChunkPlan: {} chunk(s), multi_chunk={}",
        chunk_plan.chunks.len(),
        chunk_plan.is_multi_chunk,
    );

    // Step 3: Compile Spec(s)
    tracing::info!("Step 3: Compile NLSpec(s)");
    let mut specs = if chunk_plan.is_multi_chunk {
        compile::compile_spec_multichunk(router, &intake_result, &chunk_plan).await?
    } else {
        vec![compile::compile_spec(router, &intake_result).await?]
    };
    tracing::info!(
        "  → {} NLSpecV1 chunk(s) produced",
        specs.len(),
    );

    // Step 4: Lint
    tracing::info!("Step 4: Spec Linter");
    if specs.len() > 1 {
        linter::lint_spec_set(&specs)?;
        tracing::info!("  → Multi-chunk spec set passes all lint rules");
    } else {
        linter::lint_spec(&specs[0])?;
        tracing::info!("  → Spec passes all 12 linting rules");
    }

    // Step 5: Adversarial Review
    tracing::info!("Step 5: Adversarial Review");
    let mut ar_reports = if specs.len() > 1 {
        let mut reports = ar::execute_adversarial_review_set(router, &specs, project_id).await?;
        let coherence = ar::execute_cross_chunk_coherence_review(router, &specs, project_id).await?;
        reports.push(coherence);
        reports
    } else {
        vec![ar::execute_adversarial_review(router, &specs[0], project_id).await?]
    };

    let total_blocking: u32 = ar_reports.iter().map(|r| r.blocking_count).sum();
    let total_advisory: u32 = ar_reports.iter().map(|r| r.advisory_count).sum();
    tracing::info!(
        "  → AR: {} total blocking, {} total advisory across {} report(s)",
        total_blocking, total_advisory, ar_reports.len(),
    );

    // Step 6: AR Refinement — handle blocking findings
    if total_blocking > 0 {
        tracing::info!("Step 6: AR Refinement (blocking findings detected)");
        let report_count = ar_reports.len().min(specs.len());
        for i in 0..report_count {
            if !ar_reports[i].has_blocking {
                continue;
            }
            let spec = specs.remove(i);
            let refinement = ar_refinement::execute_ar_refinement(
                router, spec, &ar_reports[i], project_id,
            ).await?;

            specs.insert(i, refinement.spec);
            tracing::info!(
                "  → Chunk '{}' refinement: {} iterations, resolved={}",
                ar_reports[i].chunk_name, refinement.iterations, refinement.resolved,
            );

            if !refinement.resolved {
                return Err(steps::StepError::ArRefinementExhausted(
                    ar_refinement::MAX_REFINEMENT_ITERATIONS,
                ));
            }
        }

        if specs.len() > 1 {
            linter::lint_spec_set(&specs)?;
        } else {
            linter::lint_spec(&specs[0])?;
        }

        tracing::info!("  Re-running AR on refined specs...");
        ar_reports = if specs.len() > 1 {
            let mut reports = ar::execute_adversarial_review_set(router, &specs, project_id).await?;
            let coherence = ar::execute_cross_chunk_coherence_review(router, &specs, project_id).await?;
            reports.push(coherence);
            reports
        } else {
            vec![ar::execute_adversarial_review(router, &specs[0], project_id).await?]
        };

        let remaining_blocking: u32 = ar_reports.iter().map(|r| r.blocking_count).sum();
        if remaining_blocking > 0 {
            return Err(steps::StepError::ArBlockingFindings(remaining_blocking));
        }
    } else {
        tracing::info!("Step 6: AR Refinement (skipped — no blocking findings)");
    }

    // Steps 7-9: GraphDot + Scenarios + AGENTS.md
    let root_spec = &specs[0];

    tracing::info!("Step 7: Compile graph.dot");
    let graph_dot = if specs.len() > 1 {
        compile::compile_graph_dot_multichunk(router, &specs).await?
    } else {
        compile::compile_graph_dot(router, root_spec).await?
    };
    tracing::info!(
        "  → GraphDotV1 produced: {} nodes, ${:.2} estimated cost",
        graph_dot.node_count, graph_dot.estimated_cost_usd,
    );

    tracing::info!("Step 8: Generate Scenarios");
    let mut scenarios = compile::generate_scenarios(router, root_spec).await?;
    tracing::info!("  → ScenarioSetV1 produced: {} scenarios", scenarios.scenarios.len());

    // Step 8b: Ralph Loop
    tracing::info!("Step 8b: Ralph Loop");
    let ralph_output = ralph::execute_ralph(router, root_spec, &scenarios, project_id).await?;

    if !ralph_output.augmented_scenarios.is_empty() {
        tracing::info!(
            "  → Ralph added {} edge-case scenarios",
            ralph_output.augmented_scenarios.len(),
        );
        scenarios.scenarios.extend(ralph_output.augmented_scenarios);
        scenarios.ralph_augmented = true;
    }

    if !ralph_output.consequence_cards.is_empty() {
        tracing::warn!(
            "  → Ralph surfaced {} ConsequenceCard(s) to Impact Inbox",
            ralph_output.consequence_cards.len(),
        );
    }

    tracing::info!("Step 9: Generate AGENTS.md");
    let agents_manifest = compile::compile_agents_manifest(router, root_spec).await?;
    tracing::info!("  → AgentsManifestV1 produced: {} bytes", agents_manifest.root_agents_md.len());

    // Step 10 (Phase 6): Formal Verification — generate Lean4 proposition templates
    tracing::info!("Step 10: Formal Verification propositions");
    let propositions = verification::generate_propositions(root_spec);
    tracing::info!(
        "  → {} Lean4 propositions generated across {} categories",
        propositions.len(),
        {
            let mut cats: Vec<&verification::PropositionCategory> =
                propositions.iter().map(|p| &p.category).collect();
            cats.sort_by_key(|c| format!("{:?}", c));
            cats.dedup();
            cats.len()
        },
    );

    // Step 11 (Phase 6): Anti-Lock-In Audit
    tracing::info!("Step 11: Anti-Lock-In Audit");
    let audit_report = audit::audit_lock_in(root_spec);
    tracing::info!(
        "  → Audit: {:?} risk (score={:.2}), {} findings, {} recommendations",
        audit_report.overall_risk, audit_report.risk_score,
        audit_report.findings.len(), audit_report.recommendations.len(),
    );

    tracing::info!("Phase 6 Front Office: pipeline complete — ready for Kilroy handoff");

    Ok(Phase0FrontOfficeOutput {
        intake: intake_result,
        specs,
        ar_reports,
        graph_dot,
        scenarios,
        agents_manifest,
        propositions,
        audit_report,
    })
}

/// Backward-compatible entry point (no storage / DTU wiring).
pub async fn run_phase0_front_office(
    router: &LlmRouter,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FrontOfficeOutput> {
    let config = PipelineConfig::<crate::cxdb::CxdbEngine>::minimal(router);
    run_phase0_front_office_with_config(&config, project_id, user_description).await
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
/// Phase 6: Accepts PipelineConfig for storage persistence and DTU wiring.
/// DTU clones are reset between scenario validation attempts.
pub async fn run_phase0_full_with_config<S: TurnStore>(
    config: &PipelineConfig<'_, S>,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    let router = config.router;
    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Planner v2 — Phase 6 Full Pipeline");
    tracing::info!("═══════════════════════════════════════════════");

    // ---- Layer 1: Front Office ----
    let front_office = run_phase0_front_office_with_config(config, project_id, user_description).await?;

    // ---- Layer 1→2: Factory Diplomat + Validation Loop ----
    let run_id = Uuid::new_v4();
    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);
    let mut factory_output;
    let mut satisfaction;
    let mut attempt = 0usize;
    let root_spec = &front_office.specs[0];

    loop {
        attempt += 1;
        tracing::info!("─── Factory Diplomat (attempt {}/{}) ───", attempt, FACTORY_MAX_RETRIES + 1);

        factory_output = factory::execute_factory_handoff(
            &front_office.graph_dot,
            &front_office.agents_manifest,
            root_spec,
            &mut budget,
        )
        .await?;

        tracing::info!(
            "  Factory: status={:?}, spend=${:.2}",
            factory_output.build_status,
            factory_output.spend_usd,
        );

        // Reset DTU clones between attempts
        if let Some(dtu_reg) = config.dtu_registry {
            dtu_reg.reset_all();
            tracing::debug!("  DTU clones reset for validation attempt {}", attempt);
        }

        // ---- Layer 3: Return Trip — Validate ----
        tracing::info!("─── Scenario Validator (attempt {}) ───", attempt);
        satisfaction = validate::execute_scenario_validation(
            router,
            &front_office.scenarios,
            &factory_output,
        )
        .await?;

        if satisfaction.gates_passed {
            tracing::info!("  Gates PASSED on attempt {}", attempt);
            break;
        }

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

/// Backward-compatible entry point (no storage / DTU wiring).
pub async fn run_phase0_full(
    router: &LlmRouter,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    let config = PipelineConfig::<crate::cxdb::CxdbEngine>::minimal(router);
    run_phase0_full_with_config(&config, project_id, user_description).await
}

/// Phase 7: Run the complete pipeline using a pluggable FactoryWorker
/// instead of the Kilroy CLI.
///
/// This replaces simulation mode with real code generation via
/// codex exec (or a mock worker for testing).
pub async fn run_phase0_full_with_worker<S: TurnStore>(
    config: &PipelineConfig<'_, S>,
    worker: &dyn factory_worker::FactoryWorker,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    let router = config.router;
    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Planner v2 — Phase 7 Full Pipeline (Worker mode)");
    tracing::info!("  Worker: {}", worker.worker_name());
    tracing::info!("═══════════════════════════════════════════════");

    // ---- Layer 1: Front Office ----
    let front_office = run_phase0_front_office_with_config(config, project_id, user_description).await?;

    // ---- Layer 1→2: Factory Worker + Validation Loop ----
    let run_id = Uuid::new_v4();
    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);
    let mut factory_output;
    let mut satisfaction;
    let mut attempt = 0usize;
    let root_spec = &front_office.specs[0];

    loop {
        attempt += 1;
        tracing::info!("─── Factory Worker (attempt {}/{}) ───", attempt, FACTORY_MAX_RETRIES + 1);

        factory_output = factory::execute_factory_with_worker(
            worker,
            &front_office.graph_dot,
            &front_office.agents_manifest,
            root_spec,
            &mut budget,
        )
        .await?;

        tracing::info!(
            "  Factory: status={:?}",
            factory_output.build_status,
        );

        // Reset DTU clones between attempts
        if let Some(dtu_reg) = config.dtu_registry {
            dtu_reg.reset_all();
        }

        // ---- Layer 3: Return Trip — Validate ----
        tracing::info!("─── Scenario Validator (attempt {}) ───", attempt);
        satisfaction = validate::execute_scenario_validation(
            router,
            &front_office.scenarios,
            &factory_output,
        )
        .await?;

        if satisfaction.gates_passed {
            tracing::info!("  Gates PASSED on attempt {}", attempt);
            break;
        }

        if attempt > FACTORY_MAX_RETRIES || !budget.can_proceed() {
            tracing::warn!("  Gates FAILED — no more retries");
            break;
        }
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
    tracing::info!("─── Git Projection ───");
    let git_result = git::execute_git_projection(
        &factory_output,
        project_id,
        &front_office.intake.project_name,
        &front_office.intake.feature_slug,
    )
    .await?;

    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Pipeline Complete ({} factory attempt(s))", attempt);
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
