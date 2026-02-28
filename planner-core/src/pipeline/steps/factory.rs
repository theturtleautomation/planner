//! # Factory Diplomat — Artifact Handoff + Kilroy CLI Invocation
//!
//! The Factory Diplomat is middleware between the Compiler and Kilroy.
//! It:
//! 1. Creates a run directory structure
//! 2. Writes graph.dot, AGENTS.md, nlspec files
//! 3. Writes run_config.yaml with model preferences and spend cap
//! 4. Invokes `kilroy attractor run graph.dot`
//! 5. Polls checkpoint.json for completion
//!
//! Phase 0: Basic handoff with spend tracking. No pre-tool-hook HTTP endpoint
//! (that comes in Phase 1 when we need real-time budget enforcement).

use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

use planner_schemas::*;
use super::{StepResult, StepError};
use super::factory_worker::{FactoryWorker, WorkerConfig, WorktreeManager, WorkerResult};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Default run directory root. Overridable via PLANNER_RUN_DIR env var.
const DEFAULT_RUN_ROOT: &str = "/tmp/planner-runs";

/// How long to wait between checkpoint polls (seconds).
const POLL_INTERVAL_SECS: u64 = 5;

/// Maximum time to wait for Kilroy completion (seconds).
/// Phase 0 micro-tools should complete well under this.
const MAX_POLL_DURATION_SECS: u64 = 600; // 10 minutes

// ---------------------------------------------------------------------------
// Run Directory Builder
// ---------------------------------------------------------------------------

/// A prepared run directory with all artifacts written.
#[derive(Debug)]
pub struct RunDirectory {
    /// Root path of the run directory (e.g. /tmp/planner-runs/<run-id>)
    pub path: PathBuf,
    /// Path to the graph.dot file
    pub graph_dot_path: PathBuf,
    /// Path to the checkpoint.json (will be created by Kilroy)
    pub checkpoint_path: PathBuf,
    /// Path to the output code directory (Kilroy writes here)
    pub output_path: PathBuf,
}

/// Create the run directory and write all artifacts Kilroy needs.
pub fn prepare_run_directory(
    run_id: Uuid,
    graph: &GraphDotV1,
    agents: &AgentsManifestV1,
    spec: &NLSpecV1,
    budget: &RunBudgetV1,
) -> StepResult<RunDirectory> {
    let run_root = std::env::var("PLANNER_RUN_DIR")
        .unwrap_or_else(|_| DEFAULT_RUN_ROOT.to_string());

    let run_dir = PathBuf::from(&run_root).join(run_id.to_string());

    // Create directory structure:
    //   <run>/
    //     graph.dot
    //     agents/AGENTS.md
    //     nlspecs/root.md
    //     config/run_config.yaml
    //     output/   (Kilroy writes generated code here)
    //     logs/     (Kilroy writes execution logs here)
    let agents_dir = run_dir.join("agents");
    let nlspecs_dir = run_dir.join("nlspecs");
    let config_dir = run_dir.join("config");
    let output_dir = run_dir.join("output");
    let logs_dir = run_dir.join("logs");

    for dir in [&run_dir, &agents_dir, &nlspecs_dir, &config_dir, &output_dir, &logs_dir] {
        std::fs::create_dir_all(dir).map_err(|e| {
            StepError::KilroyError(format!("Failed to create directory {}: {}", dir.display(), e))
        })?;
    }

    // 1. Write graph.dot
    let graph_dot_path = run_dir.join("graph.dot");
    std::fs::write(&graph_dot_path, &graph.dot_content).map_err(|e| {
        StepError::KilroyError(format!("Failed to write graph.dot: {}", e))
    })?;

    // 2. Write AGENTS.md
    let agents_md_path = agents_dir.join("AGENTS.md");
    std::fs::write(&agents_md_path, &agents.root_agents_md).map_err(|e| {
        StepError::KilroyError(format!("Failed to write AGENTS.md: {}", e))
    })?;

    // 3. Write NLSpec root chunk as root.md
    let nlspec_content = render_nlspec_markdown(spec);
    let nlspec_path = nlspecs_dir.join("root.md");
    std::fs::write(&nlspec_path, &nlspec_content).map_err(|e| {
        StepError::KilroyError(format!("Failed to write nlspec root.md: {}", e))
    })?;

    // 4. Write run_config.yaml
    let run_config = render_run_config(budget, &output_dir, &logs_dir);
    let config_path = config_dir.join("run_config.yaml");
    std::fs::write(&config_path, &run_config).map_err(|e| {
        StepError::KilroyError(format!("Failed to write run_config.yaml: {}", e))
    })?;

    let checkpoint_path = run_dir.join("checkpoint.json");
    let output_path = output_dir;

    tracing::info!("Run directory prepared at: {}", run_dir.display());
    tracing::info!("  graph.dot:   {}", graph_dot_path.display());
    tracing::info!("  AGENTS.md:   {}", agents_md_path.display());
    tracing::info!("  root.md:     {}", nlspec_path.display());
    tracing::info!("  config:      {}", config_path.display());

    Ok(RunDirectory {
        path: run_dir,
        graph_dot_path,
        checkpoint_path,
        output_path,
    })
}

/// Render an NLSpec into markdown format for Kilroy's factory agents.
fn render_nlspec_markdown(spec: &NLSpecV1) -> String {
    let mut md = String::new();

    // YAML frontmatter
    md.push_str("---\n");
    md.push_str(&format!("artifact_type: {}\n", NLSpecV1::TYPE_ID));
    md.push_str(&format!("chunk: {:?}\n", spec.chunk));
    md.push_str(&format!("version: \"{}\"\n", spec.version));
    md.push_str(&format!("project_id: \"{}\"\n", spec.project_id));
    md.push_str(&format!("created_from: \"{}\"\n", spec.created_from));
    md.push_str(&format!("status: {:?}\n", spec.status));
    md.push_str(&format!("line_count: {}\n", spec.line_count));
    md.push_str("---\n\n");

    // Intent Summary
    if let Some(ref intent) = spec.intent_summary {
        md.push_str("## Intent Summary\n\n");
        md.push_str(intent);
        md.push_str("\n\n");
    }

    // Sacred Anchors
    if let Some(ref anchors) = spec.sacred_anchors {
        if !anchors.is_empty() {
            md.push_str("## Sacred Anchors\n\n");
            for a in anchors {
                md.push_str(&format!("- **{}**: {}\n", a.id, a.statement));
            }
            md.push_str("\n");
        }
    }

    // Functional Requirements
    if !spec.requirements.is_empty() {
        md.push_str("## Functional Requirements\n\n");
        for r in &spec.requirements {
            md.push_str(&format!(
                "- **{}** [{:?}]: {} (traces to: {})\n",
                r.id, r.priority, r.statement, r.traces_to.join(", ")
            ));
        }
        md.push_str("\n");
    }

    // Architectural Constraints
    if !spec.architectural_constraints.is_empty() {
        md.push_str("## Architectural Constraints\n\n");
        for c in &spec.architectural_constraints {
            md.push_str(&format!("- {}\n", c));
        }
        md.push_str("\n");
    }

    // Phase 1 Contracts
    if let Some(ref contracts) = spec.phase1_contracts {
        if !contracts.is_empty() {
            md.push_str("## Phase 1 Contracts\n\n");
            for c in contracts {
                md.push_str(&format!(
                    "### {}\n```\n{}\n```\nConsumed by: {}\n\n",
                    c.name, c.type_definition, c.consumed_by.join(", ")
                ));
            }
        }
    }

    // External Dependencies
    if !spec.external_dependencies.is_empty() {
        md.push_str("## External Dependencies\n\n");
        for d in &spec.external_dependencies {
            md.push_str(&format!(
                "- **{}** [DTU: {:?}]: {}\n",
                d.name, d.dtu_priority, d.usage_description
            ));
        }
        md.push_str("\n");
    }

    // Definition of Done
    if !spec.definition_of_done.is_empty() {
        md.push_str("## Definition of Done\n\n");
        for d in &spec.definition_of_done {
            let check = if d.mechanically_checkable { "✓ auto" } else { "○ manual" };
            md.push_str(&format!("- [{}] {}\n", check, d.criterion));
        }
        md.push_str("\n");
    }

    // Satisfaction Criteria
    if !spec.satisfaction_criteria.is_empty() {
        md.push_str("## Satisfaction Criteria\n\n");
        for s in &spec.satisfaction_criteria {
            md.push_str(&format!(
                "- **{}** [{:?}]: {}\n",
                s.id, s.tier_hint, s.description
            ));
        }
        md.push_str("\n");
    }

    // Out of Scope
    if !spec.out_of_scope.is_empty() {
        md.push_str("## Out of Scope\n\n");
        for o in &spec.out_of_scope {
            md.push_str(&format!("- {}\n", o));
        }
        md.push_str("\n");
    }

    // Amendment Log
    if !spec.amendment_log.is_empty() {
        md.push_str("## Amendment Log\n\n");
        for a in &spec.amendment_log {
            md.push_str(&format!(
                "- [{}] {}: {} (section: {})\n",
                a.timestamp, a.reason, a.description, a.affected_section
            ));
        }
        md.push_str("\n");
    }

    md
}

/// Render the Kilroy run configuration YAML.
fn render_run_config(
    budget: &RunBudgetV1,
    output_dir: &Path,
    logs_dir: &Path,
) -> String {
    format!(
        r#"# Planner v2 — Kilroy Run Configuration
# Generated for run: {run_id}

output_dir: "{output_dir}"
logs_dir: "{logs_dir}"

# Financial circuit breaker
budget:
  hard_cap_usd: {hard_cap:.2}
  warn_threshold_usd: {warn:.2}

# Model preferences (Kilroy uses these via model_stylesheet in graph.dot)
# Planner generates the model_stylesheet directly in graph.dot
"#,
        run_id = budget.run_id,
        output_dir = output_dir.display(),
        logs_dir = logs_dir.display(),
        hard_cap = budget.hard_cap_usd,
        warn = budget.warn_threshold_usd,
    )
}

// ---------------------------------------------------------------------------
// Kilroy CLI Invocation
// ---------------------------------------------------------------------------

/// Invoke Kilroy CLI to execute the graph.dot pipeline.
///
/// Returns the Kilroy run ID on successful invocation.
/// This does NOT wait for completion — use `poll_checkpoint` for that.
pub async fn invoke_kilroy(run_dir: &RunDirectory) -> StepResult<String> {
    let graph_dot_path = run_dir.graph_dot_path.to_string_lossy().to_string();
    let config_path = run_dir.path.join("config").join("run_config.yaml");
    let logs_path = run_dir.path.join("logs");

    // Check if kilroy binary is available
    let kilroy_available = std::process::Command::new("which")
        .arg("kilroy")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !kilroy_available {
        tracing::warn!("Kilroy binary not found — running in simulation mode");
        return run_kilroy_simulation(run_dir).await;
    }

    let config_str = config_path.to_string_lossy().to_string();
    let logs_str = logs_path.to_string_lossy().to_string();

    let args = vec![
        "attractor",
        "run",
        &graph_dot_path,
        "--config",
        &config_str,
        "--logs-root",
        &logs_str,
    ];

    tracing::info!("Invoking: kilroy {}", args.join(" "));

    let output = tokio::process::Command::new("kilroy")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| StepError::KilroyError(format!("Failed to spawn kilroy: {}", e)))?
        .wait_with_output()
        .await
        .map_err(|e| StepError::KilroyError(format!("Kilroy process failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(StepError::KilroyError(format!(
            "Kilroy exited with {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::info!("Kilroy started: {}", stdout.trim());

    // Return a synthetic run ID (in practice Kilroy returns one)
    Ok(Uuid::new_v4().to_string())
}

/// Simulation mode when Kilroy is not installed.
/// Creates a fake checkpoint.json and output directory structure
/// so the rest of the pipeline can be tested end-to-end.
async fn run_kilroy_simulation(run_dir: &RunDirectory) -> StepResult<String> {
    tracing::info!("=== KILROY SIMULATION MODE ===");
    tracing::info!("Simulating factory execution for end-to-end testing");

    let run_id = Uuid::new_v4().to_string();

    // Create a simulated checkpoint.json
    let checkpoint = serde_json::json!({
        "run_id": run_id,
        "status": "complete",
        "nodes_completed": ["check_toolchain", "expand_spec", "implement", "verify_build", "verify_test", "review"],
        "nodes_failed": [],
        "total_spend_usd": 0.42,
        "elapsed_secs": 45.2,
    });

    std::fs::write(
        &run_dir.checkpoint_path,
        serde_json::to_string_pretty(&checkpoint).unwrap(),
    )
    .map_err(|e| StepError::KilroyError(format!("Failed to write simulated checkpoint: {}", e)))?;

    // Create a simulated output file
    let output_file = run_dir.output_path.join("index.html");
    std::fs::write(
        &output_file,
        "<!-- Simulated Kilroy output -->\n<html><body><h1>Simulated App</h1></body></html>\n",
    )
    .map_err(|e| StepError::KilroyError(format!("Failed to write simulated output: {}", e)))?;

    tracing::info!("Simulation complete — checkpoint.json written");
    Ok(run_id)
}

// ---------------------------------------------------------------------------
// Checkpoint Polling
// ---------------------------------------------------------------------------

/// Kilroy checkpoint.json structure (subset we need).
#[derive(Debug, serde::Deserialize)]
struct KilroyCheckpoint {
    #[allow(dead_code)]
    run_id: Option<String>,
    status: Option<String>,
    #[serde(default)]
    nodes_completed: Vec<String>,
    #[serde(default)]
    nodes_failed: Vec<String>,
    #[serde(default)]
    total_spend_usd: f32,
    #[serde(default)]
    elapsed_secs: f64,
}

/// Poll checkpoint.json until Kilroy reports completion or budget exhaustion.
pub async fn poll_checkpoint(
    run_dir: &RunDirectory,
    budget: &mut RunBudgetV1,
) -> StepResult<FactoryOutputV1> {
    let checkpoint_path = &run_dir.checkpoint_path;
    let mut elapsed = Duration::ZERO;
    let interval = Duration::from_secs(POLL_INTERVAL_SECS);
    let max_duration = Duration::from_secs(MAX_POLL_DURATION_SECS);

    loop {
        if elapsed > max_duration {
            return Err(StepError::KilroyError(format!(
                "Kilroy run timed out after {} seconds",
                max_duration.as_secs()
            )));
        }

        if checkpoint_path.exists() {
            let content = std::fs::read_to_string(checkpoint_path).map_err(|e| {
                StepError::KilroyError(format!("Failed to read checkpoint.json: {}", e))
            })?;

            let checkpoint: KilroyCheckpoint = serde_json::from_str(&content).map_err(|e| {
                StepError::KilroyError(format!("Failed to parse checkpoint.json: {}", e))
            })?;

            // Update budget tracking
            if checkpoint.total_spend_usd > 0.0 {
                let spent = checkpoint.total_spend_usd - budget.current_spend_usd;
                if spent > 0.0 {
                    budget.record_spend(SpendEvent {
                        timestamp: chrono::Utc::now(),
                        node_name: "kilroy-aggregate".into(),
                        model: "mixed".into(),
                        input_tokens: 0,
                        output_tokens: 0,
                        amount_usd: spent,
                    });
                }
            }

            // Check budget
            if !budget.can_proceed() {
                tracing::warn!("Budget exhausted — halting Kilroy run");
                return Ok(build_factory_output(
                    &checkpoint,
                    budget,
                    &run_dir,
                    BuildStatus::BudgetExhausted,
                ));
            }

            // Check completion status
            match checkpoint.status.as_deref() {
                Some("complete") | Some("success") => {
                    tracing::info!(
                        "Kilroy run complete: {} nodes succeeded, {} failed, ${:.2} spent",
                        checkpoint.nodes_completed.len(),
                        checkpoint.nodes_failed.len(),
                        checkpoint.total_spend_usd,
                    );

                    let status = if checkpoint.nodes_failed.is_empty() {
                        BuildStatus::Success
                    } else {
                        BuildStatus::PartialSuccess
                    };

                    return Ok(build_factory_output(&checkpoint, budget, run_dir, status));
                }
                Some("failed") | Some("error") => {
                    tracing::error!(
                        "Kilroy run failed: {} nodes failed",
                        checkpoint.nodes_failed.len()
                    );
                    return Ok(build_factory_output(
                        &checkpoint,
                        budget,
                        run_dir,
                        BuildStatus::Failed,
                    ));
                }
                Some("running") | Some("in_progress") => {
                    tracing::debug!(
                        "Kilroy still running: {} nodes done, ${:.2} spent",
                        checkpoint.nodes_completed.len(),
                        checkpoint.total_spend_usd,
                    );
                    // Continue polling
                }
                other => {
                    tracing::debug!("Kilroy checkpoint status: {:?}", other);
                    // Unknown status — if nodes_completed is non-empty and
                    // we have a status field, treat as complete
                    if !checkpoint.nodes_completed.is_empty()
                        && checkpoint.status.is_some()
                    {
                        let status = if checkpoint.nodes_failed.is_empty() {
                            BuildStatus::Success
                        } else {
                            BuildStatus::PartialSuccess
                        };
                        return Ok(build_factory_output(&checkpoint, budget, run_dir, status));
                    }
                }
            }
        }

        tokio::time::sleep(interval).await;
        elapsed += interval;
    }
}

/// Build the FactoryOutputV1 from a Kilroy checkpoint.
fn build_factory_output(
    checkpoint: &KilroyCheckpoint,
    budget: &RunBudgetV1,
    run_dir: &RunDirectory,
    status: BuildStatus,
) -> FactoryOutputV1 {
    let node_results: Vec<NodeResult> = checkpoint
        .nodes_completed
        .iter()
        .map(|n| NodeResult {
            node_name: n.clone(),
            success: true,
            attempts: 1,
            spend_usd: 0.0, // Per-node spend not available from checkpoint
            duration_secs: 0.0,
            error: None,
        })
        .chain(checkpoint.nodes_failed.iter().map(|n| NodeResult {
            node_name: n.clone(),
            success: false,
            attempts: 1,
            spend_usd: 0.0,
            duration_secs: 0.0,
            error: Some("Node failed — see Kilroy logs for details".into()),
        }))
        .collect();

    FactoryOutputV1 {
        kilroy_run_id: Uuid::new_v4(),
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: status,
        spend_usd: budget.current_spend_usd,
        checkpoint_path: run_dir.checkpoint_path.to_string_lossy().to_string(),
        dod_results: vec![], // Phase 0: DoD checking in Phase 1
        node_results,
        output_path: run_dir.output_path.to_string_lossy().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Full Factory Handoff — public API
// ---------------------------------------------------------------------------

/// Execute the complete Factory Diplomat workflow:
/// 1. Prepare run directory
/// 2. Invoke Kilroy CLI (or simulation)
/// 3. Poll checkpoint.json until completion
/// 4. Return FactoryOutputV1
pub async fn execute_factory_handoff(
    graph: &GraphDotV1,
    agents: &AgentsManifestV1,
    spec: &NLSpecV1,
    budget: &mut RunBudgetV1,
) -> StepResult<FactoryOutputV1> {
    tracing::info!("Factory Diplomat: starting handoff");

    // Step 1: Prepare run directory
    let run_dir = prepare_run_directory(budget.run_id, graph, agents, spec, budget)?;

    // Step 2: Invoke Kilroy
    let _kilroy_run_id = invoke_kilroy(&run_dir).await?;

    // Step 3: Poll checkpoint
    let output = poll_checkpoint(&run_dir, budget).await?;

    tracing::info!(
        "Factory Diplomat: handoff complete — status={:?}, spend=${:.2}",
        output.build_status,
        output.spend_usd,
    );

    Ok(output)
}

// ---------------------------------------------------------------------------
// Factory Worker-Powered Handoff — Phase 7
// ---------------------------------------------------------------------------

/// Execute factory handoff using a pluggable FactoryWorker instead of Kilroy.
///
/// This is the Phase 7 replacement for the Kilroy-based execution path.
/// Instead of invoking `kilroy attractor run`, it:
/// 1. Prepares a worktree with spec + graph + agents context
/// 2. Invokes the factory worker (e.g., CodexFactoryWorker via `codex exec`)
/// 3. Builds FactoryOutputV1 from the worker result
pub async fn execute_factory_with_worker(
    worker: &dyn FactoryWorker,
    graph: &GraphDotV1,
    agents: &AgentsManifestV1,
    spec: &NLSpecV1,
    budget: &mut RunBudgetV1,
) -> StepResult<FactoryOutputV1> {
    tracing::info!("Factory Diplomat (Worker mode): starting");

    // Step 1: Prepare worktree
    let worktree_mgr = WorktreeManager::default_root();
    let run_id = budget.run_id;

    let spec_md = render_nlspec_markdown(spec);
    let worktree_info = worktree_mgr.prepare(
        run_id,
        &spec_md,
        &graph.dot_content,
        &agents.root_agents_md,
    )?;

    // Step 2: Build prompt and config
    let task_prompt = format!(
        "Implement all requirements from the NLSpec. Create a working project \
         in the current directory. The project should compile, pass tests, and \
         satisfy all Definition of Done criteria."
    );

    let full_prompt = if worker.needs_worktree() {
        super::factory_worker::CodexFactoryWorker::build_codex_prompt(
            &task_prompt,
            &worktree_info,
        )
    } else {
        task_prompt.clone()
    };

    let config = WorkerConfig {
        worktree: worktree_info.path.clone(),
        model: crate::llm::DefaultModels::FACTORY_WORKER.to_string(),
        timeout_secs: 600,
        max_retries: 1,
    };

    // Step 3: Invoke worker
    let worker_result = worker.generate(&full_prompt, &config).await;

    // Step 4: Build FactoryOutputV1 from result
    let output = match worker_result {
        Ok(result) => {
            // Record spend (for subscription-based, this is notional)
            budget.record_spend(SpendEvent {
                timestamp: chrono::Utc::now(),
                node_name: worker.worker_name().to_string(),
                model: result.model.clone(),
                input_tokens: 0,
                output_tokens: 0,
                amount_usd: 0.0, // subscription-based
            });

            build_factory_output_from_worker(&result, budget, &worktree_info)
        }
        Err(e) => {
            tracing::error!("Factory worker failed: {}", e);
            FactoryOutputV1 {
                kilroy_run_id: run_id,
                nlspec_version: spec.version.clone(),
                attempt: 1,
                build_status: BuildStatus::Failed,
                spend_usd: budget.current_spend_usd,
                checkpoint_path: worktree_info.path.to_string_lossy().to_string(),
                dod_results: vec![],
                node_results: vec![NodeResult {
                    node_name: worker.worker_name().to_string(),
                    success: false,
                    attempts: 1,
                    spend_usd: 0.0,
                    duration_secs: 0.0,
                    error: Some(e.to_string()),
                }],
                output_path: worktree_info.path.to_string_lossy().to_string(),
            }
        }
    };

    // Note: we intentionally don't cleanup the worktree here so the output
    // can be inspected. The caller or a GC pass can clean up later.

    tracing::info!(
        "Factory Diplomat (Worker mode): complete — status={:?}",
        output.build_status,
    );

    Ok(output)
}

/// Build FactoryOutputV1 from a successful WorkerResult.
fn build_factory_output_from_worker(
    result: &WorkerResult,
    budget: &RunBudgetV1,
    worktree: &super::factory_worker::WorktreeInfo,
) -> FactoryOutputV1 {
    let status = if result.success {
        BuildStatus::Success
    } else {
        BuildStatus::Failed
    };

    FactoryOutputV1 {
        kilroy_run_id: result.invocation_id,
        nlspec_version: "1.0".into(),
        attempt: 1,
        build_status: status,
        spend_usd: budget.current_spend_usd,
        checkpoint_path: worktree.path.to_string_lossy().to_string(),
        dod_results: vec![],
        node_results: vec![NodeResult {
            node_name: "factory-worker".into(),
            success: result.success,
            attempts: 1,
            spend_usd: 0.0, // subscription-based
            duration_secs: result.duration_secs,
            error: result.error.clone(),
        }],
        output_path: worktree.path.to_string_lossy().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> GraphDotV1 {
        GraphDotV1 {
            project_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            dot_content: "digraph test { start [shape=Mdiamond]; exit [shape=Msquare]; start -> exit; }".into(),
            node_count: 2,
            estimated_cost_usd: 0.50,
            run_budget_usd: 2.00,
            model_routing: vec![],
        }
    }

    fn sample_agents() -> AgentsManifestV1 {
        AgentsManifestV1 {
            project_id: Uuid::new_v4(),
            nlspec_version: "1.0".into(),
            root_agents_md: "# AGENTS.md\n\nGoal: Build a test widget".into(),
            domain_docs: vec![],
            skill_refs: vec![],
        }
    }

    fn sample_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: Some("Test widget".into()),
            sacred_anchors: Some(vec![NLSpecAnchor {
                id: "SA-1".into(),
                statement: "Must not lose data".into(),
            }]),
            requirements: vec![Requirement {
                id: "FR-1".into(),
                statement: "The system must save data".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            }],
            architectural_constraints: vec!["Single file".into()],
            phase1_contracts: Some(vec![]),
            external_dependencies: vec![],
            definition_of_done: vec![DoDItem {
                criterion: "Data persists".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Save and reload".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec!["Cloud sync".into()],
            amendment_log: vec![],
        }
    }

    fn sample_budget() -> RunBudgetV1 {
        RunBudgetV1::new_phase0(Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn prepare_run_directory_creates_structure() {
        let run_id = Uuid::new_v4();
        let graph = sample_graph();
        let agents = sample_agents();
        let spec = sample_spec();
        let budget = sample_budget();

        // Use a temp dir
        std::env::set_var("PLANNER_RUN_DIR", std::env::temp_dir().join("planner-test-runs").to_string_lossy().to_string());

        let result = prepare_run_directory(run_id, &graph, &agents, &spec, &budget);
        assert!(result.is_ok());

        let run_dir = result.unwrap();
        assert!(run_dir.graph_dot_path.exists());
        assert!(run_dir.path.join("agents").join("AGENTS.md").exists());
        assert!(run_dir.path.join("nlspecs").join("root.md").exists());
        assert!(run_dir.path.join("config").join("run_config.yaml").exists());
        assert!(run_dir.path.join("output").exists());
        assert!(run_dir.path.join("logs").exists());

        // Verify graph.dot content
        let dot_content = std::fs::read_to_string(&run_dir.graph_dot_path).unwrap();
        assert!(dot_content.contains("digraph"));

        // Verify AGENTS.md content
        let agents_content = std::fs::read_to_string(run_dir.path.join("agents").join("AGENTS.md")).unwrap();
        assert!(agents_content.contains("AGENTS.md"));

        // Verify NLSpec markdown
        let nlspec_content = std::fs::read_to_string(run_dir.path.join("nlspecs").join("root.md")).unwrap();
        assert!(nlspec_content.contains("Sacred Anchors"));
        assert!(nlspec_content.contains("SA-1"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&run_dir.path);
    }

    #[test]
    fn render_nlspec_markdown_has_all_sections() {
        let spec = sample_spec();
        let md = render_nlspec_markdown(&spec);

        assert!(md.contains("---\n"));
        assert!(md.contains("artifact_type:"));
        assert!(md.contains("## Intent Summary"));
        assert!(md.contains("## Sacred Anchors"));
        assert!(md.contains("## Functional Requirements"));
        assert!(md.contains("## Architectural Constraints"));
        assert!(md.contains("## Definition of Done"));
        assert!(md.contains("## Satisfaction Criteria"));
        assert!(md.contains("## Out of Scope"));
    }

    #[test]
    fn render_run_config_contains_budget() {
        let budget = sample_budget();
        let output_dir = PathBuf::from("/tmp/output");
        let logs_dir = PathBuf::from("/tmp/logs");

        let config = render_run_config(&budget, &output_dir, &logs_dir);

        assert!(config.contains("hard_cap_usd: 5.00"));
        assert!(config.contains("warn_threshold_usd: 4.00"));
        assert!(config.contains("/tmp/output"));
    }

    #[tokio::test]
    async fn kilroy_simulation_creates_checkpoint() {
        let run_id = Uuid::new_v4();
        let graph = sample_graph();
        let agents = sample_agents();
        let spec = sample_spec();
        let budget = sample_budget();

        std::env::set_var("PLANNER_RUN_DIR", std::env::temp_dir().join("planner-test-sim").to_string_lossy().to_string());

        let run_dir = prepare_run_directory(run_id, &graph, &agents, &spec, &budget).unwrap();

        // Run simulation
        let result = run_kilroy_simulation(&run_dir).await;
        assert!(result.is_ok());

        // Verify checkpoint was created
        assert!(run_dir.checkpoint_path.exists());
        let checkpoint_content = std::fs::read_to_string(&run_dir.checkpoint_path).unwrap();
        assert!(checkpoint_content.contains("complete"));

        // Verify output was created
        assert!(run_dir.output_path.join("index.html").exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&run_dir.path);
    }

    #[tokio::test]
    async fn poll_checkpoint_reads_completed_run() {
        let run_id = Uuid::new_v4();
        let graph = sample_graph();
        let agents = sample_agents();
        let spec = sample_spec();
        let mut budget = sample_budget();

        std::env::set_var("PLANNER_RUN_DIR", std::env::temp_dir().join("planner-test-poll").to_string_lossy().to_string());

        let run_dir = prepare_run_directory(run_id, &graph, &agents, &spec, &budget).unwrap();

        // Write a completed checkpoint
        let checkpoint = serde_json::json!({
            "run_id": "test-run-123",
            "status": "complete",
            "nodes_completed": ["implement", "verify"],
            "nodes_failed": [],
            "total_spend_usd": 0.35,
            "elapsed_secs": 30.0,
        });
        std::fs::write(
            &run_dir.checkpoint_path,
            serde_json::to_string_pretty(&checkpoint).unwrap(),
        )
        .unwrap();

        let result = poll_checkpoint(&run_dir, &mut budget).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.build_status, BuildStatus::Success);
        assert_eq!(output.node_results.len(), 2);
        assert!(output.node_results.iter().all(|n| n.success));

        // Cleanup
        let _ = std::fs::remove_dir_all(&run_dir.path);
    }

    // ----- Phase 7: Factory Worker Integration Tests -----

    #[tokio::test]
    async fn execute_factory_with_mock_worker_succeeds() {
        use crate::pipeline::steps::factory_worker::MockFactoryWorker;

        let graph = sample_graph();
        let agents = sample_agents();
        let spec = sample_spec();
        let mut budget = sample_budget();

        std::env::set_var(
            "PLANNER_WORKTREE_ROOT",
            std::env::temp_dir()
                .join("planner-test-fw-success")
                .to_string_lossy()
                .to_string(),
        );

        let worker = MockFactoryWorker::success(
            "Implemented all requirements successfully",
            vec!["src/main.rs".into(), "Cargo.toml".into()],
        );

        let result = execute_factory_with_worker(
            &worker, &graph, &agents, &spec, &mut budget,
        )
        .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.build_status, BuildStatus::Success);
        assert_eq!(output.node_results.len(), 1);
        assert!(output.node_results[0].success);
        assert_eq!(output.node_results[0].node_name, "factory-worker");
    }

    #[tokio::test]
    async fn execute_factory_with_mock_worker_failure() {
        use crate::pipeline::steps::factory_worker::MockFactoryWorker;

        let graph = sample_graph();
        let agents = sample_agents();
        let spec = sample_spec();
        let mut budget = sample_budget();

        std::env::set_var(
            "PLANNER_WORKTREE_ROOT",
            std::env::temp_dir()
                .join("planner-test-fw-fail")
                .to_string_lossy()
                .to_string(),
        );

        let worker = MockFactoryWorker::failure("compile error");

        let result = execute_factory_with_worker(
            &worker, &graph, &agents, &spec, &mut budget,
        )
        .await;

        // execute_factory_with_worker catches failures gracefully
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.build_status, BuildStatus::Failed);
        assert_eq!(output.node_results.len(), 1);
        assert!(!output.node_results[0].success);
    }

    #[test]
    fn build_factory_output_from_worker_success() {
        use crate::pipeline::steps::factory_worker::WorktreeInfo;

        let result = WorkerResult {
            invocation_id: Uuid::new_v4(),
            success: true,
            model: "gpt-5.3-codex".into(),
            output: "Done".into(),
            files_changed: vec!["src/main.rs".into()],
            duration_secs: 30.0,
            error: None,
        };

        let budget = sample_budget();
        let info = WorktreeInfo {
            path: PathBuf::from("/tmp/test"),
            context_dir: PathBuf::from("/tmp/test/.planner-context"),
            run_id: Uuid::new_v4(),
        };

        let output = build_factory_output_from_worker(&result, &budget, &info);
        assert_eq!(output.build_status, BuildStatus::Success);
        assert!(output.node_results[0].success);
        assert_eq!(output.node_results[0].duration_secs, 30.0);
    }

    #[test]
    fn build_factory_output_from_worker_failure() {
        use crate::pipeline::steps::factory_worker::WorktreeInfo;

        let result = WorkerResult {
            invocation_id: Uuid::new_v4(),
            success: false,
            model: "gpt-5.3-codex".into(),
            output: String::new(),
            files_changed: vec![],
            duration_secs: 5.0,
            error: Some("build failed".into()),
        };

        let budget = sample_budget();
        let info = WorktreeInfo {
            path: PathBuf::from("/tmp/test"),
            context_dir: PathBuf::from("/tmp/test/.planner-context"),
            run_id: Uuid::new_v4(),
        };

        let output = build_factory_output_from_worker(&result, &budget, &info);
        assert_eq!(output.build_status, BuildStatus::Failed);
        assert!(!output.node_results[0].success);
        assert!(output.node_results[0].error.is_some());
    }
}
