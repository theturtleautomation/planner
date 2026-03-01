//! # Factory Worker — Pluggable Code-Generation Backend
//!
//! The `FactoryWorker` trait abstracts code generation so the pipeline can use:
//! - `CodexFactoryWorker` (default): shells out to `codex exec` CLI with GPT-5.3-Codex
//! - `MockFactoryWorker` (testing): returns deterministic outputs without LLM calls
//!
//! The Factory Diplomat calls the worker instead of the old Kilroy simulation.
//! Each invocation:
//! 1. Prepares a worktree directory
//! 2. Writes spec + graph.dot + AGENTS.md context files
//! 3. Invokes `codex exec` with workspace-write sandbox and the worktree as `-C`
//! 4. Collects stdout as the code generation result

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::{StepError, StepResult};

// ---------------------------------------------------------------------------
// FactoryWorker Trait
// ---------------------------------------------------------------------------

/// Result of a factory worker invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// Unique ID for this invocation.
    pub invocation_id: Uuid,
    /// Whether the code generation succeeded.
    pub success: bool,
    /// The model used for generation.
    pub model: String,
    /// Raw output from the code generation agent.
    pub output: String,
    /// Files created or modified (relative to worktree root).
    pub files_changed: Vec<String>,
    /// Duration of the invocation in seconds.
    pub duration_secs: f64,
    /// Error message if generation failed.
    pub error: Option<String>,
}

/// Configuration for a factory worker invocation.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// The worktree directory to use for code generation.
    pub worktree: PathBuf,
    /// The model to use (e.g., "gpt-5.3-codex").
    pub model: String,
    /// Timeout in seconds for the invocation.
    pub timeout_secs: u64,
    /// Maximum retries on transient failures.
    pub max_retries: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        WorkerConfig {
            worktree: PathBuf::from("/tmp/planner-worktree"),
            model: crate::llm::DefaultModels::FACTORY_WORKER.to_string(),
            timeout_secs: 600, // 10 minutes
            max_retries: 1,
        }
    }
}

/// The pluggable code generation backend.
///
/// The factory pipeline step calls `generate` with the NLSpec context
/// and receives generated code back.
#[async_trait]
pub trait FactoryWorker: Send + Sync {
    /// Generate code from a specification prompt.
    ///
    /// The `prompt` contains the full context: NLSpec markdown, graph.dot,
    /// AGENTS.md, and specific task instructions.
    ///
    /// The `config` specifies worktree, model, and timeout.
    async fn generate(&self, prompt: &str, config: &WorkerConfig) -> StepResult<WorkerResult>;

    /// Name of this worker implementation.
    fn worker_name(&self) -> &str;

    /// Whether this worker needs a worktree on disk.
    fn needs_worktree(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Worktree Manager
// ---------------------------------------------------------------------------

/// Manages worktree directories for code generation.
///
/// Each factory run gets its own isolated worktree so codex can read/write
/// files without interfering with other runs.
#[derive(Debug)]
pub struct WorktreeManager {
    root: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager with the given root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        WorktreeManager { root: root.into() }
    }

    /// Create a new worktree manager using the default root.
    pub fn default_root() -> Self {
        let root = std::env::var("PLANNER_WORKTREE_ROOT")
            .unwrap_or_else(|_| "/tmp/planner-worktrees".to_string());
        WorktreeManager::new(root)
    }

    /// Prepare a worktree for a factory run.
    ///
    /// Creates the directory structure and writes context files that
    /// the codex CLI can reference.
    pub fn prepare(
        &self,
        run_id: Uuid,
        spec_markdown: &str,
        graph_dot: &str,
        agents_md: &str,
    ) -> StepResult<WorktreeInfo> {
        let worktree_dir = self.root.join(run_id.to_string());

        // Create directory structure
        let context_dir = worktree_dir.join(".planner-context");
        let src_dir = worktree_dir.join("src");

        for dir in [&worktree_dir, &context_dir, &src_dir] {
            std::fs::create_dir_all(dir).map_err(|e| {
                StepError::FactoryError(format!(
                    "Failed to create worktree dir {}: {}",
                    dir.display(),
                    e
                ))
            })?;
        }

        // Write context files
        std::fs::write(context_dir.join("SPEC.md"), spec_markdown).map_err(|e| {
            StepError::FactoryError(format!("Failed to write SPEC.md: {}", e))
        })?;

        std::fs::write(context_dir.join("graph.dot"), graph_dot).map_err(|e| {
            StepError::FactoryError(format!("Failed to write graph.dot: {}", e))
        })?;

        std::fs::write(context_dir.join("AGENTS.md"), agents_md).map_err(|e| {
            StepError::FactoryError(format!("Failed to write AGENTS.md: {}", e))
        })?;

        tracing::info!(
            "Worktree prepared at: {} (context files: SPEC.md, graph.dot, AGENTS.md)",
            worktree_dir.display()
        );

        Ok(WorktreeInfo {
            path: worktree_dir,
            context_dir,
            run_id,
        })
    }

    /// Clean up a worktree after a factory run.
    pub fn cleanup(&self, info: &WorktreeInfo) -> StepResult<()> {
        if info.path.exists() {
            std::fs::remove_dir_all(&info.path).map_err(|e| {
                StepError::FactoryError(format!(
                    "Failed to cleanup worktree {}: {}",
                    info.path.display(),
                    e
                ))
            })?;
            tracing::debug!("Worktree cleaned up: {}", info.path.display());
        }
        Ok(())
    }

    /// List all active worktrees.
    pub fn list_active(&self) -> Vec<PathBuf> {
        if !self.root.exists() {
            return vec![];
        }
        std::fs::read_dir(&self.root)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Information about a prepared worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Root path of the worktree.
    pub path: PathBuf,
    /// Path to the .planner-context directory (SPEC.md, graph.dot, AGENTS.md).
    pub context_dir: PathBuf,
    /// The run ID this worktree belongs to.
    pub run_id: Uuid,
}

// ---------------------------------------------------------------------------
// CodexFactoryWorker — Real Implementation
// ---------------------------------------------------------------------------

/// Factory worker that shells out to `codex exec` for code generation.
///
/// Uses GPT-5.3-Codex via the native Codex CLI tool.
/// The user must have the `codex` CLI installed and authenticated.
///
/// Invocation pattern:
///   codex exec --json --sandbox workspace-write -m gpt-5.3-codex \
///     -C <worktree> "<prompt>"
pub struct CodexFactoryWorker {
    /// Whether the codex CLI is available.
    cli_available: bool,
}

impl CodexFactoryWorker {
    /// Create a new CodexFactoryWorker.
    ///
    /// Checks if the `codex` CLI is available on PATH.
    pub fn new() -> StepResult<Self> {
        let available = crate::llm::providers::cli_available("codex");
        if !available {
            tracing::warn!("codex CLI not found — CodexFactoryWorker will fail on invocation");
        }
        Ok(CodexFactoryWorker {
            cli_available: available,
        })
    }

    /// Build the full prompt from spec, graph, agents context + task instruction.
    pub fn build_codex_prompt(task_prompt: &str, worktree: &WorktreeInfo) -> String {
        // Read context files from the worktree
        let spec = std::fs::read_to_string(worktree.context_dir.join("SPEC.md"))
            .unwrap_or_else(|_| "[SPEC.md not found]".into());
        let graph = std::fs::read_to_string(worktree.context_dir.join("graph.dot"))
            .unwrap_or_else(|_| "[graph.dot not found]".into());
        let agents = std::fs::read_to_string(worktree.context_dir.join("AGENTS.md"))
            .unwrap_or_else(|_| "[AGENTS.md not found]".into());

        format!(
            r#"You are a factory worker code generation agent. Your job is to implement
the specification below by creating files in the current working directory.

## NLSpec (Specification)

{spec}

## Execution Graph

```dot
{graph}
```

## Agent Manifest

{agents}

## Task

{task_prompt}

## Instructions

1. Read the spec carefully. Implement ALL requirements.
2. Create source files in the `src/` directory.
3. Follow the architectural constraints exactly.
4. Ensure the code compiles and all tests pass.
5. Output a summary of what you created.
"#
        )
    }
}

#[async_trait]
impl FactoryWorker for CodexFactoryWorker {
    async fn generate(&self, prompt: &str, config: &WorkerConfig) -> StepResult<WorkerResult> {
        let invocation_id = Uuid::new_v4();
        let start = std::time::Instant::now();

        if !self.cli_available {
            return Err(StepError::FactoryError(
                "codex CLI not found. Install it or check your PATH.".into(),
            ));
        }

        let worktree_str = config.worktree.to_string_lossy().to_string();
        let model_str = config.model.clone();

        // Build args: codex exec --json --sandbox workspace-write -m <model> -C <worktree> "<prompt>"
        let args = vec![
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "-m",
            &model_str,
            "-C",
            &worktree_str,
            prompt,
        ];

        tracing::info!(
            "CodexFactoryWorker: invoking codex exec (model={}, worktree={}, timeout={}s)",
            config.model,
            worktree_str,
            config.timeout_secs
        );

        let (stdout, stderr) = crate::llm::providers::run_cli(
            "codex",
            &args,
            None,
            config.timeout_secs,
        )
        .await
        .map_err(|e| StepError::FactoryError(format!("codex exec failed: {}", e)))?;

        let duration_secs = start.elapsed().as_secs_f64();

        // Parse the JSON response
        let output = if let Ok(resp) = serde_json::from_str::<CodexExecOutput>(&stdout) {
            resp.output
                .or(resp.result)
                .unwrap_or_else(|| stdout.trim().to_string())
        } else {
            stdout.trim().to_string()
        };

        // Scan worktree for created/modified files
        let files_changed = scan_worktree_files(&config.worktree);

        if !stderr.is_empty() {
            tracing::debug!("codex stderr: {}", stderr);
        }

        // Compilation check: try cargo check or tsc depending on what's in the worktree
        let (success, compile_error) = run_compilation_check(&config.worktree, config.timeout_secs).await;
        if !success {
            tracing::warn!(
                "CodexFactoryWorker: compilation check failed: {:?}",
                compile_error
            );
        }

        tracing::info!(
            "CodexFactoryWorker: complete in {:.1}s, {} files changed, compilation={}",
            duration_secs,
            files_changed.len(),
            if success { "ok" } else { "failed" }
        );

        Ok(WorkerResult {
            invocation_id,
            success,
            model: config.model.clone(),
            output,
            files_changed,
            duration_secs,
            error: compile_error,
        })
    }

    fn worker_name(&self) -> &str {
        "codex-factory-worker"
    }

    fn needs_worktree(&self) -> bool {
        true
    }
}

/// Codex exec JSON output structure.
#[derive(Debug, Deserialize)]
struct CodexExecOutput {
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    result: Option<String>,
}

/// Run a compilation check in the given worktree.
///
/// - If `Cargo.toml` exists, runs `cargo check --manifest-path <path>` (60s timeout).
/// - Else if `package.json` exists, tries `npx tsc --noEmit`.
/// - Otherwise, warns and returns success.
///
/// Returns `(success, error_message)`.
async fn run_compilation_check(
    worktree: &std::path::Path,
    max_timeout_secs: u64,
) -> (bool, Option<String>) {
    let timeout_secs = max_timeout_secs.min(60);

    // Check for Cargo.toml
    let cargo_toml = worktree.join("Cargo.toml");
    if cargo_toml.exists() {
        let manifest_path = cargo_toml.to_string_lossy().to_string();
        tracing::info!("Running cargo check on {}", manifest_path);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("cargo")
                .arg("check")
                .arg("--manifest-path")
                .arg(&manifest_path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output(),
        )
        .await;

        return match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    (true, None)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    (false, Some(format!("cargo check failed: {}", stderr)))
                }
            }
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::warn!("cargo binary not found — skipping compilation check");
                    (true, None)
                } else {
                    (false, Some(format!("cargo check error: {}", e)))
                }
            }
            Err(_) => {
                tracing::warn!("cargo check timed out after {}s", timeout_secs);
                (false, Some(format!("cargo check timed out after {}s", timeout_secs)))
            }
        };
    }

    // Check for package.json → try npx tsc --noEmit
    let package_json = worktree.join("package.json");
    if package_json.exists() {
        tracing::info!("Running npx tsc --noEmit in {}", worktree.display());

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("npx")
                .arg("tsc")
                .arg("--noEmit")
                .current_dir(worktree)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output(),
        )
        .await;

        return match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    (true, None)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    (false, Some(format!("tsc failed: {}", stderr)))
                }
            }
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::warn!("npx not found — skipping TypeScript compilation check");
                    (true, None)
                } else {
                    (false, Some(format!("tsc error: {}", e)))
                }
            }
            Err(_) => {
                tracing::warn!("npx tsc timed out after {}s", timeout_secs);
                (false, Some(format!("tsc timed out after {}s", timeout_secs)))
            }
        };
    }

    // Neither Cargo.toml nor package.json found
    tracing::warn!(
        "No Cargo.toml or package.json found in {} — skipping compilation check",
        worktree.display()
    );
    (true, None)
}

/// Scan a worktree directory and return relative paths of all files
/// (excluding the .planner-context directory).
fn scan_worktree_files(worktree: &Path) -> Vec<String> {
    let mut files = Vec::new();
    scan_dir_recursive(worktree, worktree, &mut files);
    files
}

fn scan_dir_recursive(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip the context directory
        if path
            .file_name()
            .map(|n| n == ".planner-context")
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            scan_dir_recursive(root, &path, files);
        } else if let Ok(rel) = path.strip_prefix(root) {
            files.push(rel.to_string_lossy().to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// MockFactoryWorker — Testing Implementation
// ---------------------------------------------------------------------------

/// A mock factory worker for testing that returns deterministic outputs.
pub struct MockFactoryWorker {
    /// The output to return.
    output: String,
    /// Files to report as changed.
    files: Vec<String>,
    /// Whether to simulate failure.
    should_fail: bool,
}

impl MockFactoryWorker {
    /// Create a mock worker that succeeds.
    pub fn success(output: &str, files: Vec<String>) -> Self {
        MockFactoryWorker {
            output: output.to_string(),
            files,
            should_fail: false,
        }
    }

    /// Create a mock worker that fails.
    pub fn failure(_error: &str) -> Self {
        MockFactoryWorker {
            output: String::new(),
            files: vec![],
            should_fail: true,
        }
    }
}

#[async_trait]
impl FactoryWorker for MockFactoryWorker {
    async fn generate(&self, _prompt: &str, config: &WorkerConfig) -> StepResult<WorkerResult> {
        if self.should_fail {
            return Err(StepError::FactoryError(
                "MockFactoryWorker: simulated failure".into(),
            ));
        }

        Ok(WorkerResult {
            invocation_id: Uuid::new_v4(),
            success: true,
            model: config.model.clone(),
            output: self.output.clone(),
            files_changed: self.files.clone(),
            duration_secs: 0.1,
            error: None,
        })
    }

    fn worker_name(&self) -> &str {
        "mock-factory-worker"
    }

    fn needs_worktree(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_manager_prepare_creates_structure() {
        let tmp = std::env::temp_dir().join("planner-test-worktree-mgr");
        let mgr = WorktreeManager::new(&tmp);
        let run_id = Uuid::new_v4();

        let info = mgr
            .prepare(run_id, "# Spec", "digraph { a -> b; }", "# AGENTS")
            .unwrap();

        assert!(info.path.exists());
        assert!(info.context_dir.exists());
        assert!(info.context_dir.join("SPEC.md").exists());
        assert!(info.context_dir.join("graph.dot").exists());
        assert!(info.context_dir.join("AGENTS.md").exists());
        assert!(info.path.join("src").exists());

        // Verify content
        let spec = std::fs::read_to_string(info.context_dir.join("SPEC.md")).unwrap();
        assert_eq!(spec, "# Spec");

        let graph = std::fs::read_to_string(info.context_dir.join("graph.dot")).unwrap();
        assert_eq!(graph, "digraph { a -> b; }");

        // Cleanup
        mgr.cleanup(&info).unwrap();
        assert!(!info.path.exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn worktree_manager_list_active() {
        let tmp = std::env::temp_dir().join("planner-test-worktree-list");
        let mgr = WorktreeManager::new(&tmp);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let info1 = mgr.prepare(id1, "s1", "g1", "a1").unwrap();
        let info2 = mgr.prepare(id2, "s2", "g2", "a2").unwrap();

        let active = mgr.list_active();
        assert_eq!(active.len(), 2);

        mgr.cleanup(&info1).unwrap();
        mgr.cleanup(&info2).unwrap();

        let active_after = mgr.list_active();
        assert_eq!(active_after.len(), 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_worktree_files_excludes_context() {
        let tmp = std::env::temp_dir().join("planner-test-scan");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".planner-context")).unwrap();
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        std::fs::write(tmp.join(".planner-context/SPEC.md"), "spec").unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.join("README.md"), "# Hello").unwrap();

        let files = scan_worktree_files(&tmp);

        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"README.md".to_string()));
        // .planner-context should be excluded
        assert!(!files.iter().any(|f| f.contains("SPEC.md")));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn mock_factory_worker_success() {
        let worker = MockFactoryWorker::success(
            "Generated 3 files",
            vec!["src/main.rs".into(), "src/lib.rs".into(), "Cargo.toml".into()],
        );

        let config = WorkerConfig::default();
        let result = worker.generate("build a widget", &config).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Generated 3 files");
        assert_eq!(result.files_changed.len(), 3);
        assert_eq!(result.model, "gpt-5.3-codex");
    }

    #[tokio::test]
    async fn mock_factory_worker_failure() {
        let worker = MockFactoryWorker::failure("compilation error");
        let config = WorkerConfig::default();
        let result = worker.generate("build a widget", &config).await;

        assert!(result.is_err());
    }

    #[test]
    fn worker_config_defaults() {
        let config = WorkerConfig::default();
        assert_eq!(config.model, "gpt-5.3-codex");
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.max_retries, 1);
    }

    #[test]
    fn codex_factory_worker_build_prompt_includes_context() {
        let tmp = std::env::temp_dir().join("planner-test-prompt-build");
        let _ = std::fs::remove_dir_all(&tmp);
        let context_dir = tmp.join(".planner-context");
        std::fs::create_dir_all(&context_dir).unwrap();

        std::fs::write(context_dir.join("SPEC.md"), "# My Spec\n## Requirements\n- FR-1").unwrap();
        std::fs::write(context_dir.join("graph.dot"), "digraph { a -> b; }").unwrap();
        std::fs::write(context_dir.join("AGENTS.md"), "# Agents\n- coder").unwrap();

        let info = WorktreeInfo {
            path: tmp.clone(),
            context_dir,
            run_id: Uuid::new_v4(),
        };

        let prompt = CodexFactoryWorker::build_codex_prompt("Implement the widget", &info);

        assert!(prompt.contains("# My Spec"));
        assert!(prompt.contains("digraph { a -> b; }"));
        assert!(prompt.contains("# Agents"));
        assert!(prompt.contains("Implement the widget"));
        assert!(prompt.contains("factory worker code generation agent"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn worker_result_serialization() {
        let result = WorkerResult {
            invocation_id: Uuid::new_v4(),
            success: true,
            model: "gpt-5.3-codex".into(),
            output: "Done".into(),
            files_changed: vec!["src/main.rs".into()],
            duration_secs: 42.5,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: WorkerResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "gpt-5.3-codex");
        assert_eq!(deserialized.duration_secs, 42.5);
        assert!(deserialized.success);
    }
}
