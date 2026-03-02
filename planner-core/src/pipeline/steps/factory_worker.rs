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
//! 3. Invokes `codex exec` with --full-auto (workspace-write sandbox) and the worktree as `-C`
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

        // Initialize a git repo so codex treats it as a trusted directory.
        // Without this, codex exec refuses with "Not inside a trusted directory".
        let git_init = std::process::Command::new("git")
            .arg("init")
            .current_dir(&worktree_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match git_init {
            Ok(s) if s.success() => {
                tracing::debug!("git init succeeded in worktree {}", worktree_dir.display());
            }
            Ok(s) => {
                tracing::warn!("git init exited with {} in {}", s, worktree_dir.display());
            }
            Err(e) => {
                tracing::warn!("git init failed in {}: {} — codex may refuse to run", worktree_dir.display(), e);
            }
        }

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
///   codex exec --json --full-auto \
///     -m gpt-5.3-codex -C <worktree> \
///     --output-last-message <path> -
///
/// ## Sandbox Strategy
///
/// Default: `--full-auto` (= `--sandbox workspace-write` + `--ask-for-approval
/// on-request`). Uses Landlock on Linux, Seatbelt on macOS.
///
/// If Landlock fails (Arch, NixOS, containers, WSL), the worker detects
/// `LandlockRestrict` in the output and sets `PLANNER_CODEX_SANDBOX=danger-full-access`
/// so the next retry disables OS-level sandboxing. This is safe because the
/// worktree is an isolated `/tmp` directory with its own `git init`.
///
/// Override via `PLANNER_CODEX_SANDBOX` env var:
///   - `full-auto` (default)      → Landlock/Seatbelt sandbox
///   - `full-auto-bwrap`          → bubblewrap sandbox (experimental)
///   - `danger-full-access`       → no OS sandbox (worktree isolation only)
///
/// NOTE: `-a` (--ask-for-approval) is a global flag that does NOT
/// propagate to `exec` — use `--full-auto` instead.
///
/// The `--skip-git-repo-check` flag is NOT used because
/// `WorktreeManager::prepare` already runs `git init` in the worktree,
/// making it a trusted git directory.
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

        // Sandbox strategy:
        //
        // 1. Try `--full-auto` first (= workspace-write sandbox + on-request
        //    approvals). This uses Landlock on Linux / Seatbelt on macOS.
        //
        // 2. If Landlock fails (common on Arch, NixOS, containers, WSL),
        //    the codex output will contain "LandlockRestrict" and 0 files.
        //    On retry, fall back to `--sandbox danger-full-access` which
        //    disables OS-level sandboxing. This is safe because:
        //    - The worktree is an isolated /tmp directory
        //    - Network access is not required
        //    - The worktree is cleaned up after the run
        //    - git init provides directory trust
        //
        // 3. Env var `PLANNER_CODEX_SANDBOX` overrides:
        //    - "full-auto"          → --full-auto (default)
        //    - "full-auto-bwrap"    → --full-auto + --enable use_linux_sandbox_bwrap
        //    - "danger-full-access" → --sandbox danger-full-access
        //
        // NOTE: `-a`/`--ask-for-approval` is a global flag that does NOT
        // work with `exec` subcommand. Use `--full-auto` instead.
        // git init is already done in WorktreeManager::prepare, so no need
        // for --skip-git-repo-check.
        let sandbox_mode = std::env::var("PLANNER_CODEX_SANDBOX")
            .unwrap_or_else(|_| "full-auto".to_string());

        let output_file = std::env::temp_dir().join(format!(
            "codex-factory-{}.txt",
            invocation_id
        ));
        let output_path = output_file.to_string_lossy().to_string();

        let mut args: Vec<&str> = vec!["exec", "--json"];

        match sandbox_mode.as_str() {
            "danger-full-access" => {
                args.push("--sandbox");
                args.push("danger-full-access");
                tracing::info!("CodexFactoryWorker: using danger-full-access sandbox (worktree isolation provides containment)");
            }
            "full-auto-bwrap" => {
                args.push("--full-auto");
                args.push("--enable");
                args.push("use_linux_sandbox_bwrap");
                tracing::info!("CodexFactoryWorker: using full-auto with bubblewrap sandbox");
            }
            _ => {
                args.push("--full-auto");
                tracing::info!("CodexFactoryWorker: using full-auto sandbox (Landlock/Seatbelt)");
            }
        }

        args.extend_from_slice(&[
            "-m", &model_str,
            "-C", &worktree_str,
            "--output-last-message", &output_path,
            "-",  // read prompt from stdin
        ]);

        tracing::info!(
            "CodexFactoryWorker: invoking codex exec (model={}, worktree={}, timeout={}s)",
            config.model,
            worktree_str,
            config.timeout_secs
        );

        tracing::debug!(
            "CodexFactoryWorker: full command: codex {}",
            args.join(" ")
        );

        let (stdout, stderr) = crate::llm::providers::run_cli(
            "codex",
            &args,
            Some(prompt),
            config.timeout_secs,
        )
        .await
        .map_err(|e| StepError::FactoryError(format!("codex exec failed: {}", e)))?;

        let duration_secs = start.elapsed().as_secs_f64();

        // --- Diagnostic logging: raw JSONL events ---
        {
            let event_count = stdout.lines().count();
            tracing::info!(
                "CodexFactoryWorker: codex produced {} JSONL event lines, {} bytes stdout",
                event_count,
                stdout.len()
            );
            // Log first 5 event types for diagnosis
            for (i, line) in stdout.lines().enumerate().take(10) {
                let trimmed = line.trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    let etype = val.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let item_type = val.get("item")
                        .and_then(|i| i.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    tracing::info!(
                        "  JSONL[{}]: type={}, item_type={}",
                        i, etype, item_type
                    );
                } else {
                    tracing::info!(
                        "  JSONL[{}]: (not JSON) {}",
                        i, &trimmed[..trimmed.len().min(200)]
                    );
                }
            }
            if event_count > 10 {
                tracing::info!("  ... ({} more JSONL events)", event_count - 10);
            }
        }

        if !stderr.is_empty() {
            tracing::warn!(
                "CodexFactoryWorker: stderr ({} bytes): {}",
                stderr.len(),
                &stderr[..stderr.len().min(2000)]
            );
        }

        // Strategy 1: Read from --output-last-message file (most reliable)
        let output = if output_file.exists() {
            let file_content = std::fs::read_to_string(&output_file).unwrap_or_default();
            let _ = std::fs::remove_file(&output_file);
            tracing::info!(
                "CodexFactoryWorker: --output-last-message file: {} bytes",
                file_content.len()
            );
            if !file_content.trim().is_empty() {
                file_content.trim().to_string()
            } else {
                crate::llm::providers::extract_codex_message_from_jsonl(&stdout)
            }
        } else {
            tracing::warn!("CodexFactoryWorker: --output-last-message file not found at {}", output_path);
            // Strategy 2: Parse JSONL events from stdout
            crate::llm::providers::extract_codex_message_from_jsonl(&stdout)
        };

        // --- Diagnostic: list worktree contents post-codex ---
        {
            let files_found = scan_worktree_files(&config.worktree);
            if files_found.is_empty() {
                tracing::warn!(
                    "CodexFactoryWorker: WORKTREE EMPTY after codex exec — no files in {}",
                    config.worktree.display()
                );
                // List everything including hidden dirs for diagnosis
                if let Ok(entries) = std::fs::read_dir(&config.worktree) {
                    let all: Vec<String> = entries
                        .flatten()
                        .map(|e| {
                            let p = e.path();
                            let name = p.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let is_dir = p.is_dir();
                            format!("{}{}", name, if is_dir { "/" } else { "" })
                        })
                        .collect();
                    tracing::warn!(
                        "CodexFactoryWorker: worktree top-level entries: [{}]",
                        all.join(", ")
                    );
                }
            } else {
                tracing::info!(
                    "CodexFactoryWorker: worktree has {} files: {:?}",
                    files_found.len(),
                    &files_found[..files_found.len().min(20)]
                );
            }
        }

        // Scan worktree for created/modified files
        let files_changed = scan_worktree_files(&config.worktree);

        // --- Sandbox failure detection ---
        // If codex produced 0 files and mentions LandlockRestrict in its
        // output or stderr, the OS-level sandbox failed. Set the env var
        // so the NEXT invocation automatically falls back to
        // danger-full-access (worktree isolation still provides containment).
        if files_changed.is_empty() {
            let sandbox_error = output.contains("LandlockRestrict")
                || output.contains("legacy Linux sandbox")
                || output.contains("sandbox panic")
                || stderr.contains("LandlockRestrict")
                || stderr.contains("sandbox restrictions");

            if sandbox_error && sandbox_mode != "danger-full-access" {
                tracing::warn!(
                    "CodexFactoryWorker: SANDBOX FAILURE DETECTED — \
                     Landlock sandbox blocked all file writes. \
                     Setting PLANNER_CODEX_SANDBOX=danger-full-access for next attempt. \
                     Worktree isolation at {} provides containment.",
                    config.worktree.display()
                );
                std::env::set_var("PLANNER_CODEX_SANDBOX", "danger-full-access");
            }
        }

        // Log extracted output summary
        tracing::info!(
            "CodexFactoryWorker: extracted output ({} bytes): {}",
            output.len(),
            &output[..output.len().min(500)]
        );

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

        // Skip hidden directories (.git, .planner-context, etc.)
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
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
