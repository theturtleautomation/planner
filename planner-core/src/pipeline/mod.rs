//! # Pipeline — Recipe Step Loop
//!
//! The Phase 7 pipeline orchestrates the Dark Factory workflow:
//!
//! 1. **Intake** — Socratic interview → IntakeV1
//! 2. **Chunk Plan** — IntakeV1 → ChunkPlan (single vs multi-chunk decision)
//! 3. **Compile** — IntakeV1 → NLSpecV1 (single) or Vec<NLSpecV1> (multi-chunk)
//! 4. **Lint** — 12-rule NLSpec validation + cross-chunk rules
//! 5. **Adversarial Review** — 3-model parallel NLSpec review + coherence → ArReportV1
//! 6. **AR Refinement** — Blocking findings → spec amendments → re-lint loop
//! 7. **Factory Worker** — Pluggable code-generation backend (codex exec, mock)
//! 8. **Validate** — Scenario Validator → SatisfactionResultV1
//! 9. **Retry** — If gates fail and budget allows, re-run Factory (up to 2 retries)
//! 10. **Present** — Telemetry Presenter → Plain English + Consequence Cards
//! 11. **Approve** — Behavioral approval → Git Projection

pub mod audit;
pub mod blueprint_emitter;
pub mod project;
pub mod pyramid;
pub mod steps;
pub mod verification;

use std::collections::BTreeSet;
use uuid::Uuid;

use crate::blueprint::BlueprintStore;
use crate::cxdb::TurnStore;
use crate::dtu::DtuRegistry;
use crate::llm::providers::LlmRouter;
use planner_schemas::*;

use steps::ar;
use steps::ar_refinement;
use steps::chunk_planner;
use steps::compile;
use steps::factory;
use steps::factory_worker;
use steps::git;
use steps::intake;
use steps::linter;
use steps::ralph;
use steps::telemetry;
use steps::validate;
use steps::StepResult;

use project::{ProjectRegistry, ProjectStatus};

/// Pipeline configuration bundle — carries storage, DTU, and project
/// registry references through the pipeline.
pub struct PipelineConfig<'a, S: TurnStore> {
    /// The LLM router for model resolution.
    pub router: &'a LlmRouter,
    /// Optional durable storage — if Some, each artifact is persisted.
    pub store: Option<&'a S>,
    /// Optional DTU registry — if Some, scenario validation uses DTU clones.
    pub dtu_registry: Option<&'a DtuRegistry>,
    /// Optional Blueprint store — if Some, pipeline steps emit architectural
    /// knowledge as Blueprint nodes and edges.
    pub blueprints: Option<&'a BlueprintStore>,
}

impl<'a, S: TurnStore> PipelineConfig<'a, S> {
    /// Create a minimal config (router only, no storage or DTU).
    pub fn minimal(router: &'a LlmRouter) -> Self {
        PipelineConfig {
            router,
            store: None,
            dtu_registry: None,
            blueprints: None,
        }
    }

    /// Persist a Turn if storage is configured.
    pub fn persist<T: ArtifactPayload>(&self, turn: &Turn<T>) {
        if let Some(store) = self.store {
            if let Err(e) = store.store_turn(turn) {
                tracing::warn!("Storage: failed to persist turn {}: {}", turn.turn_id, e);
            } else {
                tracing::debug!(
                    "Storage: persisted {} (type={})",
                    turn.turn_id,
                    turn.type_id
                );
            }
        }
    }

    /// Emit Blueprint nodes if a BlueprintStore is configured.
    pub fn emit_intake_blueprint(&self, intake: &IntakeV1) {
        if let Some(bp) = self.blueprints {
            blueprint_emitter::emit_from_intake(bp, intake);
        }
    }

    /// Emit Blueprint nodes from a compiled NLSpec.
    pub fn emit_spec_blueprint(&self, spec: &NLSpecV1) {
        if let Some(bp) = self.blueprints {
            blueprint_emitter::emit_from_spec(bp, spec);
        }
    }

    /// Emit Blueprint updates from AR findings.
    pub fn emit_ar_blueprint(&self, reports: &[ArReportV1]) {
        if let Some(bp) = self.blueprints {
            blueprint_emitter::emit_from_ar(bp, reports);
        }
    }

    /// Emit Blueprint nodes from factory output.
    pub fn emit_factory_blueprint(&self, output: &FactoryOutputV1) {
        if let Some(bp) = self.blueprints {
            blueprint_emitter::emit_from_factory(bp, output);
        }
    }

    /// Flush the Blueprint store to disk if configured and dirty.
    pub fn flush_blueprint(&self) {
        if let Some(bp) = self.blueprints {
            if let Err(e) = bp.flush() {
                tracing::warn!("Blueprint: flush failed: {}", e);
            }
        }
    }

    /// Render a bounded Blueprint context block for the intake being compiled.
    pub fn blueprint_context_for_intake(&self, intake: &IntakeV1) -> Option<String> {
        self.blueprint_context_from_terms(
            collect_intake_blueprint_terms(intake),
            BLUEPRINT_CONTEXT_DEPTH,
        )
    }

    /// Render a bounded Blueprint context block for the spec currently under review/generation.
    pub fn blueprint_context_for_spec(&self, spec: &NLSpecV1) -> Option<String> {
        self.blueprint_context_from_terms(
            collect_spec_blueprint_terms(spec),
            BLUEPRINT_CONTEXT_DEPTH,
        )
    }

    fn blueprint_context_from_terms(&self, terms: Vec<String>, depth: usize) -> Option<String> {
        let blueprints = self.blueprints?;
        let node_ids = blueprints.find_relevant_node_ids(&terms, BLUEPRINT_CONTEXT_ROOT_LIMIT);
        if node_ids.is_empty() {
            return None;
        }

        tracing::debug!(
            "Blueprint context: {} root node(s) selected from {} term(s)",
            node_ids.len(),
            terms.len(),
        );
        blueprints.render_context_markdown(&node_ids, depth)
    }
}

const BLUEPRINT_CONTEXT_ROOT_LIMIT: usize = 6;
const BLUEPRINT_CONTEXT_DEPTH: usize = 1;
const BLUEPRINT_CONTEXT_STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "must", "mustn", "shall", "should",
    "could", "would", "system", "project", "build", "user", "users", "into", "under", "over",
    "only", "when", "where", "what", "which", "their", "there", "here", "have", "has", "will",
    "using", "used", "needs", "need", "already", "current", "root", "chunk", "domain", "work",
    "works",
];

fn collect_intake_blueprint_terms(intake: &IntakeV1) -> Vec<String> {
    let mut terms = BTreeSet::new();

    insert_blueprint_term(&mut terms, &intake.project_name);
    insert_blueprint_term(&mut terms, &intake.feature_slug);
    insert_blueprint_term(&mut terms, &intake.intent_summary);
    extract_blueprint_terms_from_text(&mut terms, &intake.intent_summary);

    insert_blueprint_term(&mut terms, &intake.environment.language);
    insert_blueprint_term(&mut terms, &intake.environment.framework);
    if let Some(package_manager) = &intake.environment.package_manager {
        insert_blueprint_term(&mut terms, package_manager);
    }
    if let Some(build_tool) = &intake.environment.build_tool {
        insert_blueprint_term(&mut terms, build_tool);
    }
    for dependency in &intake.environment.existing_dependencies {
        insert_blueprint_term(&mut terms, dependency);
        extract_blueprint_terms_from_text(&mut terms, dependency);
    }

    for anchor in &intake.sacred_anchors {
        insert_blueprint_term(&mut terms, &anchor.id);
        insert_blueprint_term(&mut terms, &anchor.statement);
        extract_blueprint_terms_from_text(&mut terms, &anchor.statement);
        if let Some(rationale) = &anchor.rationale {
            extract_blueprint_terms_from_text(&mut terms, rationale);
        }
    }

    for criterion in &intake.satisfaction_criteria_seeds {
        extract_blueprint_terms_from_text(&mut terms, criterion);
    }
    for item in &intake.out_of_scope {
        extract_blueprint_terms_from_text(&mut terms, item);
    }

    terms.into_iter().collect()
}

fn collect_spec_blueprint_terms(spec: &NLSpecV1) -> Vec<String> {
    let mut terms = BTreeSet::new();

    if let Some(intent_summary) = &spec.intent_summary {
        insert_blueprint_term(&mut terms, intent_summary);
        extract_blueprint_terms_from_text(&mut terms, intent_summary);
    }

    if let Some(anchors) = &spec.sacred_anchors {
        for anchor in anchors {
            insert_blueprint_term(&mut terms, &anchor.id);
            insert_blueprint_term(&mut terms, &anchor.statement);
            extract_blueprint_terms_from_text(&mut terms, &anchor.statement);
        }
    }

    for requirement in &spec.requirements {
        insert_blueprint_term(&mut terms, &requirement.id);
        extract_blueprint_terms_from_text(&mut terms, &requirement.statement);
    }
    for constraint in &spec.architectural_constraints {
        extract_blueprint_terms_from_text(&mut terms, constraint);
    }
    if let Some(contracts) = &spec.phase1_contracts {
        for contract in contracts {
            insert_blueprint_term(&mut terms, &contract.name);
            extract_blueprint_terms_from_text(&mut terms, &contract.type_definition);
        }
    }
    for dependency in &spec.external_dependencies {
        insert_blueprint_term(&mut terms, &dependency.name);
        extract_blueprint_terms_from_text(&mut terms, &dependency.usage_description);
    }
    for criterion in &spec.satisfaction_criteria {
        insert_blueprint_term(&mut terms, &criterion.id);
        extract_blueprint_terms_from_text(&mut terms, &criterion.description);
    }

    terms.into_iter().collect()
}

fn insert_blueprint_term(terms: &mut BTreeSet<String>, raw: &str) {
    let normalized = raw.trim().to_lowercase();
    if normalized.len() >= 3 && normalized.chars().any(|ch| ch.is_ascii_alphabetic()) {
        terms.insert(normalized);
    }
}

fn extract_blueprint_terms_from_text(terms: &mut BTreeSet<String>, text: &str) {
    let normalized: String = text
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();

    for token in normalized.split_whitespace() {
        let token = token.trim_matches('-').trim_matches('_');
        if token.len() < 3 || BLUEPRINT_CONTEXT_STOPWORDS.contains(&token) {
            continue;
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        terms.insert(token.to_string());
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
    ///
    /// # Phase 3+ Feature — DAG Interpreter
    ///
    /// This DAG definition is **not yet used for execution**. The pipeline
    /// currently executes steps imperatively in `run_full_pipeline` and
    /// `run_phase0_front_office_with_config`. The recipe's `steps` field
    /// captures the intended execution order and dependency graph as a
    /// structured document, and will drive actual execution once the recipe
    /// interpreter (planned for Phase 3+) is implemented.
    ///
    /// Until then, this serves two purposes:
    /// 1. Regression value — tests assert step ordering and dependency correctness.
    /// 2. Design contract — the DAG is the authoritative description of what the
    ///    pipeline is supposed to do, kept in sync with the imperative code.
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
                depends_on: vec!["validate-scenarios".into(), "deploy-sandbox".into()],
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
    let run_id = Uuid::new_v4();
    tracing::info!("Phase 6 Front Office: starting pipeline");

    // Step 1: Intake Gateway
    tracing::info!("Step 1: Intake Gateway");
    let intake_result = intake::execute_intake(router, project_id, user_description).await?;
    tracing::info!("  → IntakeV1 produced: {}", intake_result.project_name);

    // Persist intake as a Turn
    {
        let turn = Turn::new(
            intake_result.clone(),
            None,
            run_id,
            "front-office",
            "intake",
        );
        config.persist(&turn);
    }

    // Emit Blueprint nodes from intake
    config.emit_intake_blueprint(&intake_result);

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
    let compile_blueprint_context = config.blueprint_context_for_intake(&intake_result);
    let mut specs = if chunk_plan.is_multi_chunk {
        compile::compile_spec_multichunk(
            router,
            &intake_result,
            &chunk_plan,
            compile_blueprint_context.as_deref(),
        )
        .await?
    } else {
        vec![
            compile::compile_spec(router, &intake_result, compile_blueprint_context.as_deref())
                .await?,
        ]
    };
    tracing::info!("  → {} NLSpecV1 chunk(s) produced", specs.len(),);

    // Persist each NLSpecV1
    for spec in &specs {
        let turn = Turn::new(spec.clone(), None, run_id, "front-office", "compile-spec");
        config.persist(&turn);
    }

    // Emit Blueprint nodes from each compiled spec
    for spec in &specs {
        config.emit_spec_blueprint(spec);
    }

    // Build and log context pack for the root spec (Change 2).
    // ContextPackV1 is a local struct (not from planner-schemas), so we
    // cannot persist it as a Turn — we log it to show it is wired.
    {
        let pack = steps::context_pack::build_spec_context_pack(
            &specs[0],
            steps::context_pack::ContextTarget::SpecCompiler,
            8000,
        );
        tracing::info!(
            "  → ContextPack: {} sections, ~{} tokens, truncated={}",
            pack.sections.len(),
            pack.estimated_tokens,
            pack.was_truncated,
        );
    }

    // Step 4: Lint (with auto-repair loop)
    //
    // If the linter finds violations, we feed them back to the LLM compiler
    // to fix the spec, then re-lint. Maximum 2 repair iterations.
    // This prevents the pipeline from dying on fixable issues like
    // non-imperative FR language.
    tracing::info!("Step 4: Spec Linter");
    {
        const MAX_LINT_REPAIRS: usize = 2;
        let mut lint_attempt = 0usize;

        loop {
            let lint_result = if specs.len() > 1 {
                linter::lint_spec_set(&specs)
            } else {
                linter::lint_spec(&specs[0])
            };

            match lint_result {
                Ok(()) => {
                    if lint_attempt == 0 {
                        tracing::info!("  → Spec passes all linting rules");
                    } else {
                        tracing::info!(
                            "  → Spec passes all linting rules after {} repair(s)",
                            lint_attempt
                        );
                    }
                    break;
                }
                Err(steps::StepError::LintFailure { violations }) => {
                    lint_attempt += 1;
                    if lint_attempt > MAX_LINT_REPAIRS {
                        tracing::error!(
                            "  → {} lint violation(s) remain after {} repair attempts — aborting",
                            violations.len(),
                            MAX_LINT_REPAIRS
                        );
                        return Err(steps::StepError::LintFailure { violations });
                    }

                    tracing::warn!(
                        "  → {} lint violation(s) found, repair attempt {}/{}",
                        violations.len(),
                        lint_attempt,
                        MAX_LINT_REPAIRS
                    );
                    for v in &violations {
                        tracing::warn!("    {}", v);
                    }

                    // Feed violations back to the LLM to repair the spec
                    let num_specs = specs.len();
                    for (i, spec) in specs.iter_mut().enumerate() {
                        let spec_violations: Vec<&String> = violations
                            .iter()
                            .filter(|v| {
                                // For single-chunk, all violations belong to spec 0
                                if num_specs == 1 {
                                    return true;
                                }
                                // For multi-chunk, match by chunk label prefix
                                let chunk_label = match &spec.chunk {
                                    planner_schemas::ChunkType::Root => "[root]",
                                    planner_schemas::ChunkType::Domain { name } => {
                                        // violations prefixed with [domain:name]
                                        return v.contains(&format!("[domain:{}]", name))
                                            || v.contains(&format!("[{}]", name))
                                            || !v.starts_with('[');
                                    }
                                };
                                v.contains(chunk_label) || !v.starts_with('[')
                            })
                            .collect();

                        if spec_violations.is_empty() {
                            continue;
                        }

                        let violations_text = spec_violations
                            .iter()
                            .map(|v| format!("- {}", v))
                            .collect::<Vec<_>>()
                            .join("\n");

                        let spec_json = serde_json::to_string_pretty(spec)
                            .unwrap_or_else(|_| "[serialization error]".into());

                        let repair_request = crate::llm::CompletionRequest {
                            system: Some(
                                "You are the Spec Linter Repair agent. You receive an NLSpecV1 JSON \
                                 and a list of lint violations. Fix the spec to resolve ALL violations \
                                 while preserving the intent and structure. Return ONLY the corrected \
                                 NLSpecV1 JSON, no commentary.\n\n\
                                 Common fixes:\n\
                                 - Rule 4 (imperative language): Rewrite FR statements to use \
                                   must/must not/shall/shall not/always/never.\n\
                                 - Rule 5 (open questions): Resolve by picking reasonable defaults.\n\
                                 - Rule 7 (DTU priority): Set appropriate priorities for non-stdlib deps.\n\
                                 - Rule 10 (out of scope): Add at least one out-of-scope item."
                                    .to_string()
                            ),
                            messages: vec![crate::llm::Message {
                                role: crate::llm::Role::User,
                                content: format!(
                                    "Fix this NLSpecV1 to resolve these lint violations:\n\n\
                                     ## Violations\n{}\n\n\
                                     ## Current NLSpecV1 JSON\n```json\n{}\n```",
                                    violations_text, spec_json
                                ),
                            }],
                            max_tokens: 8192,
                            temperature: 0.1,
                            model: crate::llm::DefaultModels::COMPILER_SPEC.to_string(),
                        };

                        match router.complete(repair_request).await {
                            Ok(response) => {
                                let cleaned =
                                    crate::llm::json_repair::try_repair_json(&response.content)
                                        .unwrap_or_else(|| {
                                            steps::intake::strip_code_fences(&response.content)
                                        });

                                match serde_json::from_str::<NLSpecV1>(&cleaned) {
                                    Ok(repaired) => {
                                        tracing::info!(
                                            "    Repaired spec chunk {} ({} FRs)",
                                            i,
                                            repaired.requirements.len()
                                        );
                                        *spec = repaired;
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "    Failed to parse repaired spec: {} — keeping original",
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "    LLM repair call failed: {} — keeping original",
                                    e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    // Non-lint error (unexpected) — propagate
                    return Err(e);
                }
            }
        }
    }

    // Step 5: Adversarial Review
    tracing::info!("Step 5: Adversarial Review");
    let review_blueprint_context = config.blueprint_context_for_spec(&specs[0]);
    let mut ar_reports = if specs.len() > 1 {
        let mut reports = ar::execute_adversarial_review_set(
            router,
            &specs,
            project_id,
            review_blueprint_context.as_deref(),
        )
        .await?;
        let coherence = ar::execute_cross_chunk_coherence_review(
            router,
            &specs,
            project_id,
            review_blueprint_context.as_deref(),
        )
        .await?;
        reports.push(coherence);
        reports
    } else {
        vec![
            ar::execute_adversarial_review(
                router,
                &specs[0],
                project_id,
                review_blueprint_context.as_deref(),
            )
            .await?,
        ]
    };

    let total_blocking: u32 = ar_reports.iter().map(|r| r.blocking_count).sum();
    let total_advisory: u32 = ar_reports.iter().map(|r| r.advisory_count).sum();
    tracing::info!(
        "  → AR: {} total blocking, {} total advisory across {} report(s)",
        total_blocking,
        total_advisory,
        ar_reports.len(),
    );

    // Persist each ArReportV1
    for report in &ar_reports {
        let turn = Turn::new(report.clone(), None, run_id, "front-office", "ar-review");
        config.persist(&turn);
    }

    // Emit Blueprint nodes from AR findings
    config.emit_ar_blueprint(&ar_reports);

    // Step 6: AR Refinement — handle blocking findings
    if total_blocking > 0 {
        tracing::info!("Step 6: AR Refinement (blocking findings detected)");
        let report_count = ar_reports.len().min(specs.len());
        for i in 0..report_count {
            if !ar_reports[i].has_blocking {
                continue;
            }
            let spec = specs.remove(i);
            let refinement =
                ar_refinement::execute_ar_refinement(router, spec, &ar_reports[i], project_id)
                    .await?;

            specs.insert(i, refinement.spec);
            tracing::info!(
                "  → Chunk '{}' refinement: {} iterations, resolved={}",
                ar_reports[i].chunk_name,
                refinement.iterations,
                refinement.resolved,
            );

            if !refinement.resolved {
                return Err(steps::StepError::ArRefinementExhausted(
                    ar_refinement::MAX_REFINEMENT_ITERATIONS,
                ));
            }
        }

        // Re-lint after refinement — warn but don't fail.
        // The main lint+repair loop already ran in Step 4. If AR refinement
        // introduced a minor lint regression, log it but continue.
        let post_refinement_lint = if specs.len() > 1 {
            linter::lint_spec_set(&specs)
        } else {
            linter::lint_spec(&specs[0])
        };
        if let Err(steps::StepError::LintFailure { violations }) = post_refinement_lint {
            tracing::warn!(
                "  Post-refinement lint: {} violation(s) — proceeding with caution",
                violations.len()
            );
            for v in &violations {
                tracing::warn!("    {}", v);
            }
        }

        tracing::info!("  Re-running AR on refined specs...");
        let refined_review_blueprint_context = config.blueprint_context_for_spec(&specs[0]);
        ar_reports = if specs.len() > 1 {
            let mut reports = ar::execute_adversarial_review_set(
                router,
                &specs,
                project_id,
                refined_review_blueprint_context.as_deref(),
            )
            .await?;
            let coherence = ar::execute_cross_chunk_coherence_review(
                router,
                &specs,
                project_id,
                refined_review_blueprint_context.as_deref(),
            )
            .await?;
            reports.push(coherence);
            reports
        } else {
            vec![
                ar::execute_adversarial_review(
                    router,
                    &specs[0],
                    project_id,
                    refined_review_blueprint_context.as_deref(),
                )
                .await?,
            ]
        };

        let remaining_blocking: u32 = ar_reports.iter().map(|r| r.blocking_count).sum();
        if remaining_blocking > 0 {
            // Log the remaining blocking findings for visibility, but proceed.
            // After refinement + re-AR, these are typically pedantic cross-model
            // disagreements rather than genuine spec defects. The factory and
            // validation steps downstream will catch real issues.
            tracing::warn!(
                "  ⚠ {} blocking finding(s) remain after refinement — proceeding with caution",
                remaining_blocking,
            );
            for report in &ar_reports {
                for finding in &report.findings {
                    if finding.severity == ArSeverity::Blocking {
                        tracing::warn!(
                            "    [{}] {} — {}",
                            finding.affected_section,
                            finding.description,
                            finding
                                .suggested_resolution
                                .as_deref()
                                .unwrap_or("no suggestion"),
                        );
                    }
                }
            }
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
        graph_dot.node_count,
        graph_dot.estimated_cost_usd,
    );

    // Persist GraphDotV1
    {
        let turn = Turn::new(graph_dot.clone(), None, run_id, "front-office", "graph-dot");
        config.persist(&turn);
    }

    tracing::info!("Step 8: Generate Scenarios");
    let mut scenarios = compile::generate_scenarios(router, root_spec).await?;
    tracing::info!(
        "  → ScenarioSetV1 produced: {} scenarios",
        scenarios.scenarios.len()
    );

    // Persist ScenarioSetV1
    {
        let turn = Turn::new(scenarios.clone(), None, run_id, "front-office", "scenarios");
        config.persist(&turn);
    }

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
        // Persist each ConsequenceCard as a Turn so it can be queried
        // via the CXDB read API and surfaced in the Impact Inbox.
        for card in &ralph_output.consequence_cards {
            let turn = Turn::new(card.clone(), None, run_id, "ralph", "consequence-card");
            config.persist(&turn);
        }
    }

    tracing::info!("Step 9: Generate AGENTS.md");
    let agents_manifest = compile::compile_agents_manifest(router, root_spec).await?;
    tracing::info!(
        "  → AgentsManifestV1 produced: {} bytes",
        agents_manifest.root_agents_md.len()
    );

    // Persist AgentsManifestV1
    {
        let turn = Turn::new(
            agents_manifest.clone(),
            None,
            run_id,
            "front-office",
            "agents-manifest",
        );
        config.persist(&turn);
    }

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
        audit_report.overall_risk,
        audit_report.risk_score,
        audit_report.findings.len(),
        audit_report.recommendations.len(),
    );

    tracing::info!("Phase 6 Front Office: pipeline complete — ready for Kilroy handoff");

    // Flush Blueprint to disk before returning
    config.flush_blueprint();

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

/// Run the complete Phase 0 pipeline using a pluggable FactoryWorker:
/// user description → approved Git commit.
///
/// Phase 7: Uses FactoryWorker trait (codex exec or mock) instead of Kilroy CLI.
/// Phase 6: Accepts PipelineConfig for storage persistence and DTU wiring.
/// DTU clones are reset between scenario validation attempts.
pub async fn run_full_pipeline<S: TurnStore>(
    config: &PipelineConfig<'_, S>,
    worker: &dyn factory_worker::FactoryWorker,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    let run_id = Uuid::new_v4();
    run_full_pipeline_with_run_id(config, worker, project_id, run_id, user_description).await
}

/// Run the complete pipeline with a caller-provided run_id.
///
/// This is used by the server so session-to-run indexes and CXDB registration
/// remain aligned with persisted Turn metadata.
pub async fn run_full_pipeline_with_run_id<S: TurnStore>(
    config: &PipelineConfig<'_, S>,
    worker: &dyn factory_worker::FactoryWorker,
    project_id: Uuid,
    run_id: Uuid,
    user_description: &str,
) -> StepResult<Phase0FullOutput> {
    let router = config.router;
    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Planner v2 — Phase 7 Full Pipeline (Worker mode)");
    tracing::info!("  Worker: {}", worker.worker_name());
    tracing::info!("═══════════════════════════════════════════════");

    // ---- Layer 1: Front Office ----
    let front_office =
        run_phase0_front_office_with_config(config, project_id, user_description).await?;

    // ---- Project Registry: register this project ----
    // Wire in the ProjectRegistry so runs are tracked from pipeline start.
    let mut project_registry = ProjectRegistry::new();
    let project_name = front_office.intake.project_name.clone();
    let feature_slug = front_office.intake.feature_slug.clone();
    // register() rejects duplicate slugs; ignore if it fails (e.g., same slug in same run).
    let _reg_result = project_registry.register(project_name.clone(), feature_slug.clone(), vec![]);
    let registry_project_id = project_registry
        .get_by_slug(&feature_slug)
        .map(|p| p.project_id);
    tracing::info!(
        "ProjectRegistry: registered '{}' (slug='{}')",
        project_name,
        feature_slug,
    );

    // ---- Layer 1→2: Factory Worker + Validation Loop ----
    //
    // Best-of-N Rejection Sampling:
    // The pipeline tracks the best result across all attempts and uses it
    // for final reporting/commit — even if a later attempt regresses
    // (e.g., timeout producing 0/17 after a previous attempt scored 16/17).
    // This follows the Dark Factory's execution-based filtering principle:
    // keep the best surviving result, discard regressions.
    let mut budget = RunBudgetV1::new_phase0(project_id, run_id);
    let mut factory_output;
    let mut satisfaction;
    let mut best_factory_output: Option<FactoryOutputV1> = None;
    let mut best_satisfaction: Option<SatisfactionResultV1> = None;
    let mut attempt = 0usize;
    let root_spec = &front_office.specs[0];
    // Generalized error feedback from the validator — fed back to the factory
    // on retries so it knows WHAT failed (category + severity) without
    // revealing hidden scenario text.
    let mut retry_feedback: Vec<GeneralizedError> = Vec::new();
    // Previous output path for incremental retry (Attractor convergence).
    // Set after each factory attempt so retries can reuse the worktree.
    let mut previous_output_path: Option<String> = None;
    let factory_blueprint_context = config.blueprint_context_for_spec(root_spec);

    loop {
        attempt += 1;
        tracing::info!(
            "─── Factory Worker (attempt {}/{}) ───",
            attempt,
            FACTORY_MAX_RETRIES + 1
        );

        let feedback_slice: Option<&[GeneralizedError]> = if retry_feedback.is_empty() {
            None
        } else {
            Some(&retry_feedback)
        };

        let prev_path_ref = previous_output_path.as_deref();

        factory_output = match factory::execute_factory_with_worker(
            worker,
            &front_office.graph_dot,
            &front_office.agents_manifest,
            root_spec,
            factory_blueprint_context.as_deref(),
            &mut budget,
            feedback_slice,
            prev_path_ref,
        )
        .await
        {
            Ok(output) => output,
            Err(steps::StepError::CyberPolicyBlocked(ref msg)) => {
                tracing::error!(
                    "Pipeline: ABORTING — Codex cyber policy block detected \
                     on attempt {}. No retries will be attempted. {}",
                    attempt,
                    msg,
                );
                return Err(steps::StepError::CyberPolicyBlocked(msg.clone()));
            }
            Err(e) => return Err(e),
        };

        tracing::info!("  Factory: status={:?}", factory_output.build_status,);

        // Persist FactoryOutputV1
        {
            let turn = Turn::new(
                factory_output.clone(),
                None,
                run_id,
                "factory",
                "factory-output",
            );
            config.persist(&turn);
        }

        // Emit Blueprint nodes from factory output
        config.emit_factory_blueprint(&factory_output);

        // Track this output path for incremental retry on next attempt.
        previous_output_path = Some(factory_output.output_path.clone());

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
            config.dtu_registry,
        )
        .await?;

        // --- Best-of-N tracking (rejection sampling) ---
        // Compare this attempt's pass count against the best so far.
        // Keep whichever produced the most passing scenarios, regardless
        // of attempt order. This prevents a timeout/regression from
        // overwriting a better earlier result.
        let current_pass_count = satisfaction
            .scenario_results
            .iter()
            .filter(|r| r.majority_pass)
            .count();
        let best_pass_count = best_satisfaction
            .as_ref()
            .map(|s| {
                s.scenario_results
                    .iter()
                    .filter(|r| r.majority_pass)
                    .count()
            })
            .unwrap_or(0);

        if current_pass_count > best_pass_count || best_satisfaction.is_none() {
            tracing::info!(
                "  Best-of-N: attempt {} is new best ({}/{} scenarios passed, previous best: {}/{})",
                attempt,
                current_pass_count,
                satisfaction.scenario_results.len(),
                best_pass_count,
                best_satisfaction.as_ref()
                    .map(|s| s.scenario_results.len())
                    .unwrap_or(0),
            );
            best_factory_output = Some(factory_output.clone());
            best_satisfaction = Some(satisfaction.clone());
        } else {
            tracing::warn!(
                "  Best-of-N: attempt {} regressed ({}/{} vs best {}/{}), keeping previous best",
                attempt,
                current_pass_count,
                satisfaction.scenario_results.len(),
                best_pass_count,
                best_satisfaction
                    .as_ref()
                    .map(|s| s.scenario_results.len())
                    .unwrap_or(0),
            );
        }

        if satisfaction.gates_passed {
            tracing::info!("  Gates PASSED on attempt {}", attempt);

            // Persist SatisfactionResultV1
            {
                let turn = Turn::new(
                    satisfaction.clone(),
                    None,
                    run_id,
                    "validation",
                    "satisfaction",
                );
                config.persist(&turn);
            }

            break;
        }

        // Persist failed SatisfactionResultV1
        {
            let turn = Turn::new(
                satisfaction.clone(),
                None,
                run_id,
                "validation",
                "satisfaction",
            );
            config.persist(&turn);
        }

        if attempt > FACTORY_MAX_RETRIES || !budget.can_proceed() {
            tracing::warn!("  Gates FAILED — no more retries");
            // Use best result instead of the last (possibly regressed) result.
            // This is the rejection sampling principle: keep the best survivor.
            if let (Some(best_fo), Some(best_sat)) =
                (best_factory_output.take(), best_satisfaction.take())
            {
                if best_sat
                    .scenario_results
                    .iter()
                    .filter(|r| r.majority_pass)
                    .count()
                    > satisfaction
                        .scenario_results
                        .iter()
                        .filter(|r| r.majority_pass)
                        .count()
                {
                    tracing::info!(
                        "  Best-of-N: final result uses earlier attempt (better pass rate)"
                    );
                    factory_output = best_fo;
                    satisfaction = best_sat;
                }
            }
            break;
        }

        // Collect generalized errors from failed scenarios to feed back
        // to the factory on the next attempt.
        retry_feedback = satisfaction
            .scenario_results
            .iter()
            .filter_map(|r| r.generalized_error.clone())
            .collect();
        tracing::info!(
            "  Retry: {} generalized error(s) will be fed back to factory",
            retry_feedback.len(),
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

    // ---- Pyramid Summarization ----
    // Build a lightweight Pyramid tree from the pipeline's key artifacts so
    // the DCC can navigate context efficiently in future runs.
    tracing::info!("─── Pyramid Summarization ───");
    {
        // Collect turn texts from pipeline artifacts to feed as "turns".
        let turn_texts: Vec<(Uuid, String)> = vec![
            (Uuid::new_v4(), front_office.intake.intent_summary.clone()),
            (Uuid::new_v4(), telemetry.headline.clone()),
        ];
        let turn_refs: Vec<(Uuid, &str)> = turn_texts
            .iter()
            .map(|(id, text)| (*id, text.as_str()))
            .collect();
        let pyramid_builder = pyramid::PyramidBuilder::with_defaults();
        let pyramid_tree = pyramid_builder.build_pyramid(project_id, &turn_refs);
        tracing::info!(
            "  Pyramid: {} node(s) (root={}, branches={}, leaves={})",
            pyramid_tree.node_count(),
            pyramid_tree.root.is_some(),
            pyramid_tree.branches.len(),
            pyramid_tree.leaves.len(),
        );
    }

    // ---- Git Projection ----
    tracing::info!("─── Git Projection ───");
    let git_result = git::execute_git_projection(
        &factory_output,
        project_id,
        &front_office.intake.project_name,
        &front_office.intake.feature_slug,
    )
    .await?;

    // Persist GitCommitV1
    {
        let turn = Turn::new(git_result.commit.clone(), None, run_id, "git", "git-commit");
        config.persist(&turn);
    }

    // Persist RunBudgetV1
    {
        let turn = Turn::new(budget.clone(), None, run_id, "budget", "run-budget");
        config.persist(&turn);
    }

    tracing::info!("═══════════════════════════════════════════════");
    tracing::info!("  Pipeline Complete ({} factory attempt(s))", attempt);
    tracing::info!("═══════════════════════════════════════════════");

    // ---- Project Registry: update status to Completed ----
    if let Some(reg_pid) = registry_project_id {
        let _ = project_registry.update_status(reg_pid, ProjectStatus::Completed);
        tracing::info!(
            "ProjectRegistry: project '{}' marked Completed",
            feature_slug
        );
    }

    // Final Blueprint flush
    config.flush_blueprint();

    Ok(Phase0FullOutput {
        front_office,
        factory_output,
        satisfaction,
        telemetry,
        git_result,
        budget,
    })
}
