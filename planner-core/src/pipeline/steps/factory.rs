//! # Factory Diplomat — Artifact Handoff + Factory Worker Invocation
//!
//! The Factory Diplomat is middleware between the Compiler and the
//! pluggable FactoryWorker backend (Phase 7+).
//!
//! It:
//! 1. Prepares a worktree with spec + graph + agents context
//! 2. Invokes the factory worker (e.g., CodexFactoryWorker via `codex exec`)
//! 3. Builds FactoryOutputV1 from the worker result
//!
//! Phase 7: Kilroy CLI is replaced by the pluggable FactoryWorker trait,
//! which enables codex exec, mock workers for testing, and future backends.

use planner_schemas::*;
use super::StepResult;
use super::StepError;
use super::factory_worker::{FactoryWorker, WorkerConfig, WorktreeManager, WorkerResult};

// ---------------------------------------------------------------------------
// NLSpec Markdown Renderer — shared by worker path
// ---------------------------------------------------------------------------

/// Render an NLSpec into markdown format for factory agents.
pub fn render_nlspec_markdown(spec: &NLSpecV1) -> String {
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

// ---------------------------------------------------------------------------
// Factory Worker-Powered Handoff — Phase 7 (only entry point)
// ---------------------------------------------------------------------------

/// Execute factory handoff using a pluggable FactoryWorker.
///
/// This is the Phase 7 factory execution path. Instead of invoking a Kilroy
/// CLI, it:
/// 1. Prepares a worktree with spec + graph + agents context
/// 2. Invokes the factory worker (e.g., CodexFactoryWorker via `codex exec`)
/// 3. Builds FactoryOutputV1 from the worker result
pub async fn execute_factory_with_worker(
    worker: &dyn FactoryWorker,
    graph: &GraphDotV1,
    agents: &AgentsManifestV1,
    spec: &NLSpecV1,
    budget: &mut RunBudgetV1,
    error_feedback: Option<&[GeneralizedError]>,
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
    let mut task_prompt = String::from(
        "Implement all requirements from the NLSpec. Create a working project \
         in the current directory. The project should compile, pass tests, and \
         satisfy all Definition of Done criteria.",
    );

    // Append generalized error feedback from a previous validation pass.
    // This tells the factory WHAT failed (category + severity) without
    // revealing scenario text or BDD details.
    if let Some(errors) = error_feedback {
        if !errors.is_empty() {
            task_prompt.push_str("\n\n## Previous Validation Feedback\n\n");
            task_prompt.push_str(
                "The previous implementation failed quality gates. \
                 Fix these issues in your new implementation:\n\n",
            );
            for err in errors {
                task_prompt.push_str(&format!(
                    "- [{}] {}: ensure proper handling is in place\n",
                    format!("{:?}", err.severity),
                    err.category,
                ));
            }
            tracing::info!(
                "Factory retry: {} generalized error(s) included in prompt",
                errors.len(),
            );
        }
    }

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
            // CyberPolicyBlocked is unrecoverable — propagate immediately
            // so the pipeline loop can abort without wasting retries.
            if matches!(&e, StepError::CyberPolicyBlocked(_)) {
                return Err(e);
            }
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
    use std::path::PathBuf;
    use uuid::Uuid;
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
            &worker, &graph, &agents, &spec, &mut budget, None,
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
            &worker, &graph, &agents, &spec, &mut budget, None,
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
