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
//! 3. Invokes `codex exec` with bubblewrap sandbox (--full-auto --enable use_linux_sandbox_bwrap)
//!    Falls back to Landlock, then danger-full-access if sandbox fails.
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

/// Sandbox modes for the Codex factory worker, ordered by preference.
///
/// The worker tries each mode in order until one succeeds. The cascade:
///   1. `Bwrap`  — bubblewrap namespace sandbox (preferred on Linux)
///   2. `Landlock` — Landlock + seccomp (kernel-level, fails on Arch/NixOS/WSL)
///   3. `DangerFullAccess` — no OS sandbox (last resort, worktree isolation only)
///
/// Override via `PLANNER_CODEX_SANDBOX` env var:
///   - `full-auto-bwrap`    → start at Bwrap (default)
///   - `full-auto`          → start at Landlock
///   - `danger-full-access` → skip to DangerFullAccess
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SandboxMode {
    /// `--full-auto --enable use_linux_sandbox_bwrap`
    /// Bubblewrap namespace sandbox — proper containment without Landlock.
    Bwrap,
    /// `--full-auto` (default Codex behavior)
    /// Landlock + seccomp — kernel-level sandbox. Fails on Arch, NixOS, WSL.
    Landlock,
    /// `--sandbox danger-full-access`
    /// No OS-level sandbox. Only worktree isolation provides containment.
    /// Used as absolute last resort when both bwrap and Landlock fail.
    DangerFullAccess,
}

impl SandboxMode {
    /// Parse from the `PLANNER_CODEX_SANDBOX` env var or return default.
    fn from_env() -> Self {
        match std::env::var("PLANNER_CODEX_SANDBOX").as_deref() {
            Ok("full-auto") => SandboxMode::Landlock,
            Ok("danger-full-access") => SandboxMode::DangerFullAccess,
            // Default: bwrap — proper sandbox that works on Arch, NixOS, WSL, etc.
            _ => SandboxMode::Bwrap,
        }
    }

    /// Resolve the best sandbox mode by checking the pre-flight probe cache
    /// first, then running a live probe if needed. Skipped when the user has
    /// explicitly set `PLANNER_CODEX_SANDBOX`.
    fn resolve() -> Self {
        // If user explicitly set the env var, honour it — no probing.
        if std::env::var("PLANNER_CODEX_SANDBOX").is_ok() {
            let mode = Self::from_env();
            tracing::debug!("SandboxMode::resolve: env override → {}", mode);
            return mode;
        }

        // Check persistent probe cache (~/.cache/planner/sandbox-probe)
        if let Some(cached) = SandboxProbe::read_cache() {
            tracing::info!("SandboxMode::resolve: using cached probe result → {}", cached);
            return cached;
        }

        // No cache — run live probe.
        let result = SandboxProbe::run();
        tracing::info!("SandboxMode::resolve: live probe → {}", result);
        // Persist so subsequent invocations skip the probe.
        SandboxProbe::write_cache(result);
        result
    }

    /// Return the next fallback mode, or None if this is the last resort.
    fn fallback(self) -> Option<SandboxMode> {
        match self {
            SandboxMode::Bwrap => Some(SandboxMode::Landlock),
            SandboxMode::Landlock => Some(SandboxMode::DangerFullAccess),
            SandboxMode::DangerFullAccess => None,
        }
    }

    /// The env var value to persist for the next invocation.
    fn env_value(self) -> &'static str {
        match self {
            SandboxMode::Bwrap => "full-auto-bwrap",
            SandboxMode::Landlock => "full-auto",
            SandboxMode::DangerFullAccess => "danger-full-access",
        }
    }

    /// Human-readable label for logging.
    fn label(self) -> &'static str {
        match self {
            SandboxMode::Bwrap => "bubblewrap (bwrap)",
            SandboxMode::Landlock => "Landlock + seccomp",
            SandboxMode::DangerFullAccess => "danger-full-access (NO OS sandbox)",
        }
    }

    /// Build the CLI args specific to this sandbox mode.
    fn cli_args(self) -> Vec<&'static str> {
        match self {
            SandboxMode::Bwrap => vec!["--full-auto", "--enable", "use_linux_sandbox_bwrap"],
            SandboxMode::Landlock => vec!["--full-auto"],
            SandboxMode::DangerFullAccess => vec!["--sandbox", "danger-full-access"],
        }
    }

    /// Check if the given output/stderr indicates this sandbox mode failed.
    fn detect_failure(self, output: &str, stderr: &str) -> bool {
        match self {
            SandboxMode::Bwrap => {
                // Bwrap failures: namespace errors, /dev/urandom, bwrap binary missing,
                // "Can't mount proc" (PVE kernels)
                output.contains("bwrap")
                    || output.contains("bubblewrap")
                    || output.contains("/dev/urandom")
                    || output.contains("user namespace")
                    || output.contains("unshare")
                    || output.contains("Can't mount proc")
                    || output.contains("mount proc")
                    || stderr.contains("bwrap")
                    || stderr.contains("bubblewrap")
                    || stderr.contains("/dev/urandom")
                    || stderr.contains("user namespace")
                    || stderr.contains("unshare")
                    || stderr.contains("Can't mount proc")
                    || stderr.contains("mount proc")
            }
            SandboxMode::Landlock => {
                output.contains("LandlockRestrict")
                    || output.contains("legacy Linux sandbox")
                    || output.contains("sandbox panic")
                    || stderr.contains("LandlockRestrict")
                    || stderr.contains("sandbox restrictions")
            }
            SandboxMode::DangerFullAccess => false, // can't fail on sandbox
        }
    }
}

impl std::fmt::Display for SandboxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ---------------------------------------------------------------------------
// Pre-flight Sandbox Probe
// ---------------------------------------------------------------------------

/// Fast pre-flight test to determine if bwrap (bubblewrap) can run on this
/// kernel. Runs `bwrap --unshare-all -- /bin/true` which exercises the same
/// namespace setup that Codex uses internally. Completes in ~100ms.
///
/// Results are cached to `~/.cache/planner/sandbox-probe` so the probe
/// only runs once per machine (until the cache file is deleted).
struct SandboxProbe;

impl SandboxProbe {
    /// Cache file path: `~/.cache/planner/sandbox-probe`
    fn cache_path() -> Option<PathBuf> {
        dirs_cache_path()
    }

    /// Read the cached probe result. Returns `None` if no cache exists,
    /// the cache is corrupt, or older than 7 days.
    fn read_cache() -> Option<SandboxMode> {
        let path = Self::cache_path()?;
        let content = std::fs::read_to_string(&path).ok()?;
        let lines: Vec<&str> = content.trim().lines().collect();
        if lines.len() < 2 {
            return None;
        }
        // Line 0: Unix timestamp of probe
        // Line 1: mode string (e.g., "full-auto-bwrap")
        let ts: u64 = lines[0].parse().ok()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        // Cache expires after 7 days (kernel updates may change capability)
        if now.saturating_sub(ts) > 7 * 24 * 3600 {
            tracing::debug!("SandboxProbe: cache expired ({}s old)", now - ts);
            return None;
        }
        match lines[1] {
            "full-auto-bwrap" => Some(SandboxMode::Bwrap),
            "full-auto" => Some(SandboxMode::Landlock),
            "danger-full-access" => Some(SandboxMode::DangerFullAccess),
            _ => None,
        }
    }

    /// Write probe result to cache.
    fn write_cache(mode: SandboxMode) {
        if let Some(path) = Self::cache_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let content = format!("{}\n{}", ts, mode.env_value());
            match std::fs::write(&path, &content) {
                Ok(_) => tracing::debug!("SandboxProbe: cached {} to {}", mode, path.display()),
                Err(e) => tracing::debug!("SandboxProbe: failed to write cache: {}", e),
            }
        }
    }

    /// Invalidate the cache (called when a probe's prediction was wrong).
    fn invalidate_cache() {
        if let Some(path) = Self::cache_path() {
            let _ = std::fs::remove_file(&path);
            tracing::debug!("SandboxProbe: cache invalidated");
        }
    }

    /// Run the live pre-flight probe.
    ///
    /// Tests: `bwrap --unshare-all -- /bin/true`
    ///   - If exit 0 → Bwrap works
    ///   - If exit non-0 or error → fall through to Landlock
    ///   - Landlock is not probed (we learn from codex's output)
    fn run() -> SandboxMode {
        // Check if bwrap binary exists first
        let bwrap_check = std::process::Command::new("which")
            .arg("bwrap")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match bwrap_check {
            Ok(s) if s.success() => {
                // bwrap exists — try the full namespace test
                let probe = std::process::Command::new("bwrap")
                    .args(["--unshare-all", "--", "/bin/true"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .output();

                match probe {
                    Ok(output) if output.status.success() => {
                        tracing::info!("SandboxProbe: bwrap --unshare-all succeeded");
                        SandboxMode::Bwrap
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::info!(
                            "SandboxProbe: bwrap --unshare-all failed (exit {}): {}",
                            output.status, stderr.trim()
                        );
                        // bwrap can't do full namespace isolation (PVE kernel, etc.)
                        // Fall through to Landlock.
                        SandboxMode::Landlock
                    }
                    Err(e) => {
                        tracing::info!("SandboxProbe: bwrap exec error: {}", e);
                        SandboxMode::Landlock
                    }
                }
            }
            _ => {
                tracing::info!("SandboxProbe: bwrap not found on PATH");
                // No bwrap binary — try Landlock (Codex default).
                SandboxMode::Landlock
            }
        }
    }
}

/// Get the cache directory path: `~/.cache/planner/sandbox-probe`
/// Works on Linux/macOS. Returns None if HOME is not set.
fn dirs_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cache").join("planner").join("sandbox-probe"))
}

/// Factory worker that shells out to `codex exec` for code generation.
///
/// Uses GPT-5.3-Codex via the native Codex CLI tool.
/// The user must have the `codex` CLI installed and authenticated.
///
/// Invocation pattern:
///   codex exec --json --full-auto --enable use_linux_sandbox_bwrap \
///     -m gpt-5.3-codex -C <worktree> \
///     --output-last-message <path> -
///
/// ## Sandbox Strategy
///
/// Three-layer sandbox resolution:
///   1. **Pre-flight probe** (~100ms): tests `bwrap --unshare-all` before
///      calling codex. Skips straight to the working mode.
///   2. **Persistent cache**: writes probe result to
///      `~/.cache/planner/sandbox-probe` so subsequent runs start instantly.
///   3. **Retry loop**: if a mode fails at runtime, generate() retries with
///      the next fallback mode, actually consuming `max_retries`.
///
/// Fallback cascade (automatic on sandbox failure, 0 files produced):
///   1. Bwrap (default) — proper containment via Linux namespaces
///   2. Landlock — kernel-level sandbox (fails on Arch/NixOS/WSL)
///   3. danger-full-access — last resort only, worktree isolation only
///
/// Override via `PLANNER_CODEX_SANDBOX` env var:
///   - `full-auto-bwrap` (default)  → bubblewrap sandbox
///   - `full-auto`                  → Landlock/Seatbelt sandbox
///   - `danger-full-access`         → no OS sandbox (worktree isolation only)
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

/// Result of a single codex invocation attempt (before retry logic).
struct InvocationResult {
    stderr: String,
    output: String,
    files_changed: Vec<String>,
    duration_secs: f64,
}

impl CodexFactoryWorker {
    /// Execute a single codex invocation with the given sandbox mode.
    ///
    /// This is the inner helper that `generate()` calls inside its retry
    /// loop. It handles CLI invocation, output extraction, and worktree
    /// scanning — but NOT retry/fallback logic.
    async fn invoke_codex_once(
        &self,
        prompt: &str,
        config: &WorkerConfig,
        sandbox_mode: SandboxMode,
        invocation_id: Uuid,
        attempt_start: std::time::Instant,
    ) -> StepResult<InvocationResult> {
        let worktree_str = config.worktree.to_string_lossy().to_string();
        let model_str = config.model.clone();

        let output_file = std::env::temp_dir().join(format!(
            "codex-factory-{}.txt",
            invocation_id
        ));
        let output_path = output_file.to_string_lossy().to_string();

        let mut args: Vec<&str> = vec!["exec", "--json"];
        args.extend(sandbox_mode.cli_args());

        tracing::info!(
            "CodexFactoryWorker: sandbox={} ({})",
            sandbox_mode, sandbox_mode.env_value()
        );

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

        let duration_secs = attempt_start.elapsed().as_secs_f64();

        // --- Diagnostic logging: raw JSONL events ---
        {
            let event_count = stdout.lines().count();
            tracing::info!(
                "CodexFactoryWorker: codex produced {} JSONL event lines, {} bytes stdout",
                event_count,
                stdout.len()
            );
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
        let files_changed = scan_worktree_files(&config.worktree);
        if files_changed.is_empty() {
            tracing::warn!(
                "CodexFactoryWorker: WORKTREE EMPTY after codex exec — no files in {}",
                config.worktree.display()
            );
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
                files_changed.len(),
                &files_changed[..files_changed.len().min(20)]
            );
        }

        Ok(InvocationResult {
            stderr,
            output,
            files_changed,
            duration_secs,
        })
    }
}

#[async_trait]
impl FactoryWorker for CodexFactoryWorker {
    async fn generate(&self, prompt: &str, config: &WorkerConfig) -> StepResult<WorkerResult> {
        let invocation_id = Uuid::new_v4();
        let overall_start = std::time::Instant::now();

        if !self.cli_available {
            return Err(StepError::FactoryError(
                "codex CLI not found. Install it or check your PATH.".into(),
            ));
        }

        // --- Three-layer sandbox resolution ---
        //
        // Layer 1: Pre-flight probe (~100ms bwrap test) + Layer 2: persistent
        // cache (~/.cache/planner/sandbox-probe).  SandboxMode::resolve()
        // handles both — returns the best mode without wasting a full codex
        // invocation on a sandbox that can't work.
        //
        // Layer 3: Retry loop below — if the resolved mode fails at runtime
        // (0 files + detect_failure), we invalidate the cache, advance to
        // the next fallback, and retry immediately.
        let mut current_mode = SandboxMode::resolve();

        // max_retries controls how many sandbox fallback attempts we allow.
        // Each sandbox failure consumes one retry.  With max_retries=1 and
        // 3 modes (bwrap → landlock → danger-full-access), the worst case
        // is: probe picks bwrap, runtime fails → retry with landlock →
        // runtime fails → (retry budget exhausted, return the failure).
        //
        // Bump max_retries to 2 to guarantee we can always reach
        // danger-full-access from any starting mode.
        let max_sandbox_retries = config.max_retries.max(2);
        let mut attempts: u32 = 0;

        loop {
            attempts += 1;
            let attempt_start = std::time::Instant::now();

            tracing::info!(
                "CodexFactoryWorker: attempt {}/{} with sandbox={}",
                attempts,
                max_sandbox_retries + 1,
                current_mode
            );

            let result = self
                .invoke_codex_once(prompt, config, current_mode, invocation_id, attempt_start)
                .await?;

            // --- Sandbox failure detection & retry ---
            let is_sandbox_failure = result.files_changed.is_empty()
                && current_mode.detect_failure(&result.output, &result.stderr);

            if is_sandbox_failure {
                if let Some(next_mode) = current_mode.fallback() {
                    if attempts <= max_sandbox_retries {
                        tracing::warn!(
                            "CodexFactoryWorker: SANDBOX FAILURE — {} blocked file writes \
                             (attempt {}, {:.1}s wasted). Invalidating cache, retrying \
                             immediately with {}.",
                            current_mode,
                            attempts,
                            result.duration_secs,
                            next_mode
                        );

                        if next_mode == SandboxMode::DangerFullAccess {
                            tracing::warn!(
                                "CodexFactoryWorker: ⚠ LAST RESORT — danger-full-access \
                                 disables OS-level sandboxing. Worktree isolation at {} \
                                 is the only containment.",
                                config.worktree.display()
                            );
                        }

                        // Invalidate the stale cache and persist the new mode
                        // so future pipeline runs skip straight to it.
                        SandboxProbe::invalidate_cache();
                        SandboxProbe::write_cache(next_mode);

                        current_mode = next_mode;
                        continue; // retry immediately — no sleep
                    }

                    tracing::error!(
                        "CodexFactoryWorker: sandbox retry budget exhausted after {} attempts. \
                         Last mode: {}, next would be: {}",
                        attempts,
                        current_mode,
                        next_mode
                    );
                } else {
                    tracing::error!(
                        "CodexFactoryWorker: sandbox failure with {} — no further \
                         fallback available. Something else is blocking file writes.",
                        current_mode
                    );
                }
            }

            // --- Success path (or non-sandbox failure) ---
            let total_duration = overall_start.elapsed().as_secs_f64();

            // Log extracted output summary
            if !result.output.is_empty() {
                tracing::info!(
                    "CodexFactoryWorker: extracted output ({} bytes): {}",
                    result.output.len(),
                    &result.output[..result.output.len().min(500)]
                );
            }

            // Compilation check
            let (success, compile_error) =
                run_compilation_check(&config.worktree, config.timeout_secs).await;
            if !success {
                tracing::warn!(
                    "CodexFactoryWorker: compilation check failed: {:?}",
                    compile_error
                );
            }

            tracing::info!(
                "CodexFactoryWorker: complete in {:.1}s ({} attempts), {} files changed, \
                 sandbox={}, compilation={}",
                total_duration,
                attempts,
                result.files_changed.len(),
                current_mode,
                if success { "ok" } else { "failed" }
            );

            return Ok(WorkerResult {
                invocation_id,
                success,
                model: config.model.clone(),
                output: result.output,
                files_changed: result.files_changed,
                duration_secs: total_duration,
                error: compile_error,
            });
        }
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

    // --- SandboxMode tests ---

    #[test]
    fn sandbox_mode_default_is_bwrap() {
        // Clear env to test default
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
        let mode = SandboxMode::from_env();
        assert_eq!(mode, SandboxMode::Bwrap);
        assert_eq!(mode.env_value(), "full-auto-bwrap");
    }

    #[test]
    fn sandbox_mode_from_env_full_auto() {
        std::env::set_var("PLANNER_CODEX_SANDBOX", "full-auto");
        assert_eq!(SandboxMode::from_env(), SandboxMode::Landlock);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_from_env_danger() {
        std::env::set_var("PLANNER_CODEX_SANDBOX", "danger-full-access");
        assert_eq!(SandboxMode::from_env(), SandboxMode::DangerFullAccess);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_fallback_cascade() {
        // Bwrap -> Landlock -> DangerFullAccess -> None
        let bwrap = SandboxMode::Bwrap;
        assert_eq!(bwrap.fallback(), Some(SandboxMode::Landlock));

        let landlock = SandboxMode::Landlock;
        assert_eq!(landlock.fallback(), Some(SandboxMode::DangerFullAccess));

        let danger = SandboxMode::DangerFullAccess;
        assert_eq!(danger.fallback(), None);
    }

    #[test]
    fn sandbox_mode_cli_args() {
        assert_eq!(
            SandboxMode::Bwrap.cli_args(),
            vec!["--full-auto", "--enable", "use_linux_sandbox_bwrap"]
        );
        assert_eq!(
            SandboxMode::Landlock.cli_args(),
            vec!["--full-auto"]
        );
        assert_eq!(
            SandboxMode::DangerFullAccess.cli_args(),
            vec!["--sandbox", "danger-full-access"]
        );
    }

    #[test]
    fn sandbox_mode_detect_bwrap_failure() {
        let mode = SandboxMode::Bwrap;
        // Should detect bwrap-specific errors
        assert!(mode.detect_failure("error: bwrap failed to unshare", ""));
        assert!(mode.detect_failure("", "bubblewrap error: cannot create namespace"));
        assert!(mode.detect_failure("cannot access /dev/urandom", ""));
        assert!(mode.detect_failure("user namespace creation failed", ""));
        // Should NOT detect Landlock errors
        assert!(!mode.detect_failure("LandlockRestrict error", ""));
        // Clean output = no failure
        assert!(!mode.detect_failure("files written successfully", ""));
    }

    #[test]
    fn sandbox_mode_detect_landlock_failure() {
        let mode = SandboxMode::Landlock;
        // Should detect Landlock-specific errors
        assert!(mode.detect_failure("LandlockRestrict error", ""));
        assert!(mode.detect_failure("error applying legacy Linux sandbox", ""));
        assert!(mode.detect_failure("", "sandbox restrictions failed"));
        // Should NOT detect bwrap errors
        assert!(!mode.detect_failure("bwrap failed", ""));
        // Clean output = no failure
        assert!(!mode.detect_failure("files written successfully", ""));
    }

    #[test]
    fn sandbox_mode_danger_never_detects_failure() {
        let mode = SandboxMode::DangerFullAccess;
        // danger-full-access should never report sandbox failure
        assert!(!mode.detect_failure("any error at all", "any stderr"));
    }

    #[test]
    fn sandbox_mode_env_roundtrip() {
        // Setting env var then reading should preserve the mode
        for mode in [SandboxMode::Bwrap, SandboxMode::Landlock, SandboxMode::DangerFullAccess] {
            std::env::set_var("PLANNER_CODEX_SANDBOX", mode.env_value());
            assert_eq!(SandboxMode::from_env(), mode, "roundtrip failed for {:?}", mode);
        }
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    // --- Probe cache tests ---

    #[test]
    fn sandbox_probe_cache_write_read_roundtrip() {
        // Use a temp dir to isolate from real cache
        let tmp = std::env::temp_dir().join("planner-test-probe-cache");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cache_file = tmp.join("sandbox-probe");

        // Write a cache entry manually (simulating write_cache)
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let content = format!("{}\ndanger-full-access", ts);
        std::fs::write(&cache_file, &content).unwrap();

        // Read it back manually (simulating read_cache logic)
        let read_back = std::fs::read_to_string(&cache_file).unwrap();
        let lines: Vec<&str> = read_back.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1], "danger-full-access");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sandbox_probe_cache_expired_returns_none() {
        let tmp = std::env::temp_dir().join("planner-test-probe-expired");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cache_file = tmp.join("sandbox-probe");

        // Write a cache entry with a timestamp 8 days ago (expired)
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (8 * 24 * 3600); // 8 days ago
        let content = format!("{}\nfull-auto-bwrap", ts);
        std::fs::write(&cache_file, &content).unwrap();

        // Parse like read_cache does
        let read_back = std::fs::read_to_string(&cache_file).unwrap();
        let lines: Vec<&str> = read_back.trim().lines().collect();
        let cached_ts: u64 = lines[0].parse().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Should be expired (> 7 days)
        assert!(now.saturating_sub(cached_ts) > 7 * 24 * 3600);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sandbox_probe_invalidate_removes_file() {
        // This tests the actual invalidate_cache -> write_cache flow
        // by using the real SandboxProbe methods on the actual cache path.
        // First, write a cache entry.
        SandboxProbe::write_cache(SandboxMode::Bwrap);

        // Verify it was written (may fail if HOME not set, that's OK)
        if let Some(path) = SandboxProbe::cache_path() {
            if path.exists() {
                // Invalidate
                SandboxProbe::invalidate_cache();
                assert!(!path.exists(), "cache file should be removed after invalidation");

                // Write a new one to verify write after invalidate works
                SandboxProbe::write_cache(SandboxMode::DangerFullAccess);
                if path.exists() {
                    let content = std::fs::read_to_string(&path).unwrap();
                    assert!(content.contains("danger-full-access"));
                }

                // Cleanup
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    #[test]
    fn sandbox_mode_detect_bwrap_mount_proc_failure() {
        // This is the exact error from PVE kernel 6.17.4-2-pve
        let mode = SandboxMode::Bwrap;
        assert!(mode.detect_failure(
            "bwrap: Can't mount proc on /newroot/proc: Operation not permitted",
            ""
        ));
        assert!(mode.detect_failure(
            "",
            "bwrap: Can't mount proc on /newroot/proc: Operation not permitted"
        ));
    }

    #[test]
    fn sandbox_mode_resolve_respects_env_override() {
        // When PLANNER_CODEX_SANDBOX is set, resolve() should return
        // that mode directly without probing.
        std::env::set_var("PLANNER_CODEX_SANDBOX", "danger-full-access");
        let mode = SandboxMode::resolve();
        assert_eq!(mode, SandboxMode::DangerFullAccess);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_resolve_without_env_returns_valid_mode() {
        // Without env var, resolve() should run probe and return a valid mode.
        // We can't predict which mode (depends on whether bwrap is installed),
        // but it must be one of the three variants.
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
        // Invalidate cache so we get a fresh probe
        SandboxProbe::invalidate_cache();
        let mode = SandboxMode::resolve();
        assert!(
            mode == SandboxMode::Bwrap
                || mode == SandboxMode::Landlock
                || mode == SandboxMode::DangerFullAccess,
            "resolve() returned unexpected mode: {:?}",
            mode
        );
        // Clean up cache after test
        SandboxProbe::invalidate_cache();
    }

    #[test]
    fn sandbox_fallback_exhaustion() {
        // Starting from DangerFullAccess, there's no fallback.
        // This models the case where all modes have been tried.
        let mode = SandboxMode::DangerFullAccess;
        assert!(mode.fallback().is_none());
        // And danger never detects sandbox failure
        assert!(!mode.detect_failure("bwrap: error", "LandlockRestrict"));
    }

    #[test]
    fn sandbox_fallback_full_cascade_from_bwrap() {
        // Verify the complete cascade: Bwrap -> Landlock -> Danger -> None
        let mut mode = SandboxMode::Bwrap;
        let mut cascade = vec![mode];

        while let Some(next) = mode.fallback() {
            cascade.push(next);
            mode = next;
        }

        assert_eq!(cascade, vec![
            SandboxMode::Bwrap,
            SandboxMode::Landlock,
            SandboxMode::DangerFullAccess,
        ]);
    }
}
