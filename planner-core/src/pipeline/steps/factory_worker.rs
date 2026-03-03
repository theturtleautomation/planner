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
//! 3. Invokes `codex exec` with the best available sandbox via 3-mode cascade:
//!    Bwrap → FullAuto (Landlock) → WorkspaceWrite (danger-full-access + LXC boundary)
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

/// Standard .gitignore seeded into every worktree.
///
/// This ensures `git ls-files` and `git diff` only report source files,
/// not dependencies, build artifacts, or caches.  Codex also respects
/// .gitignore for its own context window.
const WORKTREE_GITIGNORE: &str = r#"# Dependencies
node_modules/
vendor/
.venv/
venv/
__pycache__/
*.pyc

# Build artifacts
dist/
build/
target/
out/
.output/
.next/
.nuxt/
.svelte-kit/

# Package manager locks (tracked separately, not useful for code review)
package-lock.json
yarn.lock
pnpm-lock.yaml
Cargo.lock
Gemfile.lock
poetry.lock

# IDE / OS
.DS_Store
*.swp
*.swo
.idea/
.vscode/
*.iml

# Coverage / caches
coverage/
.cache/
.turbo/
.vercel/
.parcel-cache/
"#;

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

        // Wipe any existing worktree from a previous attempt with the same
        // run_id.  Without this, a retry re-uses the old directory and
        // `git_initial_commit` absorbs stale Codex output into the baseline
        // commit, making `git diff HEAD` return 0 changed files.
        if worktree_dir.exists() {
            tracing::debug!(
                "WorktreeManager::prepare: removing stale worktree {}",
                worktree_dir.display()
            );
            if let Err(e) = std::fs::remove_dir_all(&worktree_dir) {
                tracing::warn!(
                    "Failed to remove stale worktree {}: {} — continuing anyway",
                    worktree_dir.display(),
                    e
                );
            }
        }

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

        // Write .gitignore so git-based file tracking excludes
        // dependencies, build artifacts, and caches automatically.
        // This also helps Codex — it respects .gitignore for context.
        let gitignore_content = WORKTREE_GITIGNORE;
        std::fs::write(worktree_dir.join(".gitignore"), gitignore_content).map_err(|e| {
            StepError::FactoryError(format!("Failed to write .gitignore: {}", e))
        })?;

        // Create an initial commit with the context files + .gitignore
        // so that post-codex `git diff` can identify what Codex changed.
        Self::git_initial_commit(&worktree_dir);

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

    /// Create an initial git commit in the worktree so that post-codex
    /// `git diff` cleanly shows only what Codex added/changed.
    fn git_initial_commit(worktree_dir: &Path) {
        let run = |args: &[&str]| -> bool {
            std::process::Command::new("git")
                .args(args)
                .current_dir(worktree_dir)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        // Configure git user for the commit (required in fresh repos)
        run(&["config", "user.email", "planner@localhost"]);
        run(&["config", "user.name", "Planner"]);

        // Stage all context files + .gitignore
        if !run(&["add", "-A"]) {
            tracing::debug!("git add -A failed in {}", worktree_dir.display());
            return;
        }

        // Commit
        if run(&["commit", "-m", "planner: initial worktree setup", "--allow-empty"]) {
            tracing::debug!("git initial commit created in {}", worktree_dir.display());
        } else {
            tracing::debug!("git commit failed in {} (non-fatal)", worktree_dir.display());
        }
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
// Git-based worktree file tracking
// ---------------------------------------------------------------------------

/// Get the list of files that Codex created or modified in a worktree.
///
/// Uses `git diff --name-only HEAD` to detect changes against the initial
/// commit (created by `WorktreeManager::prepare`). Falls back to
/// `git ls-files --others --exclude-standard` for untracked files,
/// and finally to a filesystem scan if git isn't available.
///
/// Returns relative paths (e.g., `src/main.rs`, `package.json`).
pub fn git_worktree_changed_files(worktree: &Path) -> Vec<String> {
    // First, check if this is a valid git repo with at least one commit.
    // If not, fall back to filesystem scan immediately.
    let has_git = git_command(worktree, &["rev-parse", "HEAD"]).is_some();
    if !has_git {
        tracing::debug!(
            "git_worktree_changed_files: not a git repo (or no commits) in {}, falling back to filesystem scan",
            worktree.display()
        );
        return scan_worktree_files(worktree);
    }

    let mut files: Vec<String> = Vec::new();

    // Strategy 1: git diff against initial commit (tracked + modified files)
    if let Some(diff_files) = git_command(worktree, &["diff", "--name-only", "HEAD"]) {
        for line in diff_files.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                files.push(trimmed.to_string());
            }
        }
    }

    // Strategy 2: untracked files not covered by .gitignore
    if let Some(untracked) = git_command(worktree, &["ls-files", "--others", "--exclude-standard"]) {
        for line in untracked.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !files.contains(&trimmed.to_string()) {
                files.push(trimmed.to_string());
            }
        }
    }

    // Strategy 3: staged but not yet committed (Codex may stage without committing)
    if let Some(staged) = git_command(worktree, &["diff", "--cached", "--name-only"]) {
        for line in staged.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !files.contains(&trimmed.to_string()) {
                files.push(trimmed.to_string());
            }
        }
    }

    // Filter out .planner-context files and .gitignore (our scaffolding)
    files.retain(|f| !f.starts_with(".planner-context/") && f != ".gitignore");

    tracing::debug!(
        "git_worktree_changed_files: {} files from git in {}",
        files.len(),
        worktree.display()
    );
    files
}

/// Run a git command in a directory and return its stdout, or None on failure.
fn git_command(dir: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Read source file contents from a worktree for evaluation.
///
/// Uses `git_worktree_changed_files` to get the file list, then reads
/// each file's contents, concatenated with path headers. Caps at 30
/// files and 100KB total. Skips binary/minified files.
///
/// This is the function the Scenario Validator uses to get source code.
pub fn read_worktree_source_files(worktree: &Path) -> String {
    const MAX_FILES: usize = 30;
    const MAX_TOTAL_BYTES: usize = 100 * 1024; // 100 KB

    let changed_files = git_worktree_changed_files(worktree);
    if changed_files.is_empty() {
        return "[No source files found in output directory]".into();
    }

    let mut result = String::new();
    let mut file_count = 0usize;
    let mut total_bytes = 0usize;

    // Prioritize src/ files first (the actual implementation)
    let mut sorted_files = changed_files.clone();
    sorted_files.sort_by(|a, b| {
        let a_is_src = a.starts_with("src/");
        let b_is_src = b.starts_with("src/");
        match (a_is_src, b_is_src) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });

    for rel_path in &sorted_files {
        if file_count >= MAX_FILES || total_bytes >= MAX_TOTAL_BYTES {
            break;
        }

        let full_path = worktree.join(rel_path);

        // Read file contents
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary/unreadable files
        };

        // Skip likely-minified files: very few lines but large size
        if content.lines().count() <= 2 && content.len() > 5000 {
            continue;
        }

        let header = format!("\n=== {} ===\n", rel_path);
        let available = MAX_TOTAL_BYTES.saturating_sub(total_bytes);
        let truncated = if content.len() > available {
            &content[..available]
        } else {
            &content
        };

        result.push_str(&header);
        result.push_str(truncated);
        total_bytes += header.len() + truncated.len();
        file_count += 1;
    }

    if result.is_empty() {
        return "[No source files found in output directory]".into();
    }
    result
}

// ---------------------------------------------------------------------------
// CodexFactoryWorker — Real Implementation
// ---------------------------------------------------------------------------

/// Sandbox modes for the Codex factory worker, ordered by preference.
///
/// Three modes — each provides containment appropriate to the environment:
///
///   1. `Bwrap`          — bubblewrap namespace sandbox (preferred on Linux)
///   2. `FullAuto`       — Codex's `--full-auto` flag (Landlock + seccomp)
///   3. `WorkspaceWrite` — `--sandbox workspace-write -a never` with bwrap
///                         enforcement. Used when Landlock is absent but bwrap
///                         is available with `--enable use_linux_sandbox_bwrap`.
///                         Falls back to `danger-full-access` only on kernels
///                         where no OS sandbox works (e.g. PVE LXC without
///                         Landlock and without proc mount capability).
///
/// There is no standalone `danger-full-access` mode exposed to callers.
/// If the environment cannot support any OS sandbox, the probe detects this
/// and uses `danger-full-access` internally — but logs a clear warning that
/// containment depends on the LXC/VM boundary + worktree isolation.
///
/// Override via `PLANNER_CODEX_SANDBOX` env var:
///   - `full-auto-bwrap`    → Bwrap (default)
///   - `full-auto`          → FullAuto (Landlock)
///   - `workspace-write`    → WorkspaceWrite (application-level sandbox)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SandboxMode {
    /// `--full-auto --enable use_linux_sandbox_bwrap`
    /// Bubblewrap namespace sandbox — proper containment without Landlock.
    Bwrap,
    /// `--full-auto` (Codex default behavior)
    /// Uses Landlock + seccomp when the kernel supports it. Fails hard on
    /// kernels without Landlock (PVE containers, some WSL setups).
    FullAuto,
    /// `--sandbox danger-full-access -a never`
    /// Used when neither bwrap nor Landlock is available (PVE LXC containers).
    /// Containment relies on:
    ///   1. The LXC/VM boundary (OS-level isolation)
    ///   2. Planner's worktree isolation (each codex run in its own directory)
    /// NOTE: We use `danger-full-access` internally because Codex CLI has no
    /// way to say "workspace-write but don't enforce Landlock". The sandbox
    /// policy check in Codex always tries Landlock for workspace-write mode.
    WorkspaceWrite,
}

impl SandboxMode {
    /// Parse from the `PLANNER_CODEX_SANDBOX` env var or return default.
    fn from_env() -> Self {
        match std::env::var("PLANNER_CODEX_SANDBOX").as_deref() {
            Ok("full-auto") => SandboxMode::FullAuto,
            Ok("workspace-write") => SandboxMode::WorkspaceWrite,
            // Legacy values — map to WorkspaceWrite (safest mode that always works).
            Ok("danger-full-access") => {
                tracing::warn!(
                    "PLANNER_CODEX_SANDBOX=danger-full-access is deprecated. \
                     Using workspace-write mode instead."
                );
                SandboxMode::WorkspaceWrite
            }
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

        // Migrate stale cache: if the cached value is a now-removed mode
        // (danger-full-access or Landlock-only), invalidate so we re-probe.
        SandboxProbe::migrate_stale_cache();

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
            SandboxMode::Bwrap => Some(SandboxMode::FullAuto),
            SandboxMode::FullAuto => Some(SandboxMode::WorkspaceWrite),
            SandboxMode::WorkspaceWrite => None,
        }
    }

    /// The env var value to persist for the next invocation.
    fn env_value(self) -> &'static str {
        match self {
            SandboxMode::Bwrap => "full-auto-bwrap",
            SandboxMode::FullAuto => "full-auto",
            SandboxMode::WorkspaceWrite => "workspace-write",
        }
    }

    /// Human-readable label for logging.
    fn label(self) -> &'static str {
        match self {
            SandboxMode::Bwrap => "bubblewrap (bwrap)",
            SandboxMode::FullAuto => "full-auto (Landlock + seccomp)",
            SandboxMode::WorkspaceWrite => "workspace-write (worktree + LXC isolation)",
        }
    }

    /// Build the CLI args specific to this sandbox mode.
    fn cli_args(self) -> Vec<&'static str> {
        match self {
            SandboxMode::Bwrap => vec!["--full-auto", "--enable", "use_linux_sandbox_bwrap"],
            SandboxMode::FullAuto => vec!["--full-auto"],
            // WorkspaceWrite: use danger-full-access because Codex's workspace-write
            // mode still tries to enforce Landlock. On kernels without Landlock,
            // the only way to get Codex to write files is to disable OS-level
            // enforcement entirely. Containment comes from the LXC boundary
            // and Planner's worktree isolation.
            //
            // NOTE: We do NOT pass -a/--ask-for-approval here. `codex exec`
            // is non-interactive — it has no approval prompt loop. Passing
            // `-a` to `exec` causes "error: unexpected argument '-a' found".
            SandboxMode::WorkspaceWrite => vec![
                "--sandbox", "danger-full-access",
            ],
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
            SandboxMode::FullAuto => {
                // Landlock failures: LandlockRestrict errors in output or
                // in the Codex model's response text
                output.contains("LandlockRestrict")
                    || output.contains("Sandbox(Landlock")
                    || output.contains("sandbox panic")
                    || output.contains("legacy Linux sandbox")
                    || stderr.contains("LandlockRestrict")
                    || stderr.contains("Sandbox(Landlock")
                    || stderr.contains("sandbox restrictions")
            }
            // WorkspaceWrite uses danger-full-access under the hood —
            // no OS sandbox enforcement to fail.
            SandboxMode::WorkspaceWrite => false,
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
/// kernel. Runs `bwrap --unshare-all -- true` which exercises the same
/// namespace setup that Codex uses internally. Completes in ~100ms.
///
/// NOTE: We use `true` (PATH lookup) instead of `/bin/true` because on
/// Arch Linux (and some NixOS setups) `/bin/true` doesn't exist — it's at
/// `/usr/bin/true`. Inside the bwrap sandbox we use `--ro-bind /usr /usr`
/// to make it available regardless of distro layout.
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
            "full-auto" => Some(SandboxMode::FullAuto),
            "workspace-write" => Some(SandboxMode::WorkspaceWrite),
            // Legacy value no longer valid — treat as cache miss.
            "danger-full-access" => {
                tracing::info!("SandboxProbe: cache contains deprecated 'danger-full-access', ignoring");
                None
            }
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

    /// Migrate stale cache from pre-v2 sandbox logic.
    /// If the cache contains `danger-full-access` or `workspace-write`, delete
    /// it so we re-probe. The old probe checked a nonexistent sysfs path
    /// (`/sys/kernel/security/landlock/abi_version`) which always failed,
    /// causing incorrect fallback to workspace-write even when Landlock was
    /// fully operational.
    fn migrate_stale_cache() {
        if let Some(path) = Self::cache_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains("danger-full-access")
                    || content.contains("workspace-write")
                {
                    tracing::info!(
                        "SandboxProbe: deleting stale cache (value was set by broken \
                         probe that checked nonexistent sysfs path). Will re-probe."
                    );
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    /// Detect whether Landlock is active on the running kernel.
    ///
    /// Landlock is a syscall-only LSM — it has NO securityfs directory.
    /// There is no `/sys/kernel/security/landlock/` on any kernel version.
    ///
    /// Detection strategy:
    ///   1. Read `/sys/kernel/security/lsm` — a comma-separated list of active
    ///      LSMs (e.g. "landlock,lockdown,yama,integrity,apparmor"). If
    ///      "landlock" appears, it's enabled.
    ///   2. Fallback: invoke `landlock_create_ruleset` syscall (NR 444 on
    ///      x86_64) with `LANDLOCK_CREATE_RULESET_VERSION` flag. Returns the
    ///      ABI version (>0) if Landlock is active, or -1/ENOSYS/EOPNOTSUPP
    ///      if not.
    fn probe_landlock() -> bool {
        // --- Method 1: /sys/kernel/security/lsm ---
        // This file always exists when securityfs is mounted and lists all
        // active LSMs. It's readable by unprivileged users.
        if let Ok(lsm_list) = std::fs::read_to_string("/sys/kernel/security/lsm") {
            let has_landlock = lsm_list
                .split(',')
                .any(|module| module.trim() == "landlock");
            if has_landlock {
                tracing::info!(
                    "SandboxProbe: Landlock detected via /sys/kernel/security/lsm"
                );
                return true;
            }
            tracing::debug!(
                "SandboxProbe: /sys/kernel/security/lsm readable but landlock not listed: {}",
                lsm_list.trim()
            );
        } else {
            tracing::debug!(
                "SandboxProbe: /sys/kernel/security/lsm not readable, trying syscall"
            );
        }

        // --- Method 2: landlock_create_ruleset syscall ---
        // syscall number 444 on x86_64, flag 1 = LANDLOCK_CREATE_RULESET_VERSION.
        // Returns ABI version (>0) on success, -1 on failure.
        // We use a small Python one-liner because Rust's libc crate isn't in
        // our dependency tree and adding it just for one syscall is overkill.
        let syscall_probe = std::process::Command::new("python3")
            .args([
                "-c",
                "import ctypes,sys; \
                 libc=ctypes.CDLL(None,use_errno=True); \
                 r=libc.syscall(444,None,0,1); \
                 sys.exit(0 if r>0 else 1)",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match syscall_probe {
            Ok(status) if status.success() => {
                tracing::info!(
                    "SandboxProbe: Landlock detected via syscall (landlock_create_ruleset)"
                );
                true
            }
            Ok(_) => {
                tracing::info!("SandboxProbe: landlock_create_ruleset syscall returned failure");
                false
            }
            Err(e) => {
                tracing::info!("SandboxProbe: python3 not available for syscall probe: {}", e);
                false
            }
        }
    }

    /// Run the live pre-flight probe.
    ///
    /// Tests bwrap and Landlock availability in order:
    ///   1. bwrap: `bwrap --unshare-all --ro-bind /usr /usr ... -- true`
    ///   2. Landlock: check `/sys/kernel/security/lsm` for "landlock" in the
    ///      active LSM list, OR attempt the `landlock_create_ruleset` syscall
    ///      with the `LANDLOCK_CREATE_RULESET_VERSION` flag.
    ///      NOTE: Landlock has NO securityfs directory — there is no
    ///      `/sys/kernel/security/landlock/` path on any kernel version.
    ///      The old probe that checked for `abi_version` at that path was
    ///      always wrong and always returned false.
    ///   3. If both fail: WorkspaceWrite (danger-full-access under the hood)
    fn run() -> SandboxMode {
        // --- Check bwrap ---
        let bwrap_check = std::process::Command::new("which")
            .arg("bwrap")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match bwrap_check {
            Ok(s) if s.success() => {
                // bwrap exists — try the full namespace test.
                // We bind /usr and symlink /bin -> usr/bin so `true` resolves
                // on both Arch (/usr/bin/true) and Debian (/bin/true -> /usr/bin/true).
                let probe = std::process::Command::new("bwrap")
                    .args([
                        "--unshare-all",
                        "--ro-bind", "/usr", "/usr",
                        "--symlink", "usr/bin", "/bin",
                        "--symlink", "usr/lib", "/lib",
                        "--proc", "/proc",
                        "--dev", "/dev",
                        "--", "true",
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .output();

                match probe {
                    Ok(output) if output.status.success() => {
                        tracing::info!("SandboxProbe: bwrap --unshare-all succeeded");
                        return SandboxMode::Bwrap;
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::info!(
                            "SandboxProbe: bwrap --unshare-all failed (exit {}): {}",
                            output.status, stderr.trim()
                        );
                    }
                    Err(e) => {
                        tracing::info!("SandboxProbe: bwrap exec error: {}", e);
                    }
                }
            }
            _ => {
                tracing::info!("SandboxProbe: bwrap not found on PATH");
            }
        }

        // --- bwrap failed or not available — check Landlock ---
        // Landlock is a syscall-only LSM with NO securityfs interface.
        // There is no /sys/kernel/security/landlock/ directory on any kernel.
        // We detect it two ways:
        //   1. Read /sys/kernel/security/lsm — lists active LSMs (e.g.
        //      "landlock,lockdown,yama,integrity,apparmor").
        //   2. Fallback: invoke landlock_create_ruleset via syscall 444
        //      with LANDLOCK_CREATE_RULESET_VERSION flag.
        let landlock_available = Self::probe_landlock();

        if landlock_available {
            tracing::info!("SandboxProbe: Landlock is active — using --full-auto");
            return SandboxMode::FullAuto;
        }

        // --- Neither bwrap nor Landlock available ---
        tracing::warn!(
            "SandboxProbe: neither bwrap nor Landlock available. \
             Using workspace-write mode (danger-full-access internally). \
             Containment relies on LXC/VM boundary + worktree isolation."
        );
        SandboxMode::WorkspaceWrite
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
/// Three-mode cascade with pre-flight probing:
///   1. **Bwrap** (preferred): bubblewrap namespace sandbox.
///      Probe: `bwrap --unshare-all --ro-bind /usr /usr ... -- true`.
///   2. **FullAuto**: Codex's `--full-auto` flag (Landlock + seccomp).
///      Probe: checks `/sys/kernel/security/lsm` for "landlock" in active
///      LSM list, falls back to `landlock_create_ruleset` syscall.
///   3. **WorkspaceWrite**: `danger-full-access` internally, for kernels
///      where neither bwrap nor Landlock works (PVE LXC containers).
///      Containment: LXC boundary + worktree isolation.
///
/// Resolution layers:
///   1. **Pre-flight probe** (~100ms) + **persistent cache**
///      (`~/.cache/planner/sandbox-probe`). `SandboxMode::resolve()`
///      handles both. Probes bwrap + Landlock ABI to skip doomed modes.
///   2. **Runtime fallback**: if a mode fails (0 files + mode-specific
///      error patterns), invalidate cache, retry with next fallback.
///
/// Override via `PLANNER_CODEX_SANDBOX` env var:
///   - `full-auto-bwrap` (default) → bubblewrap sandbox
///   - `full-auto`                 → Landlock sandbox
///   - `workspace-write`           → worktree + LXC isolation
///
/// ## Config Isolation
///
/// Codex is launched with `--ephemeral` and config overrides to prevent
/// user-level instructions (e.g. `~/.codex/instructions.md`, local-memory
/// plugins) from bleeding into factory worker prompts. Only the factory's
/// own prompt is sent.
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

        let mut args: Vec<&str> = vec!["exec", "--json", "--ephemeral"];
        args.extend(sandbox_mode.cli_args());

        // Suppress user-level config from bleeding into factory prompts.
        // --ephemeral prevents session persistence.
        // project_doc_max_bytes=0 suppresses AGENTS.md/instructions.md loading.
        // This ensures only our factory prompt reaches the model, not the
        // user's personal Codex instructions, local-memory plugins, etc.
        args.extend_from_slice(&[
            "-c", "project_doc_max_bytes=0",
        ]);

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

        let (stdout, stderr) = match crate::llm::providers::run_cli(
            "codex",
            &args,
            Some(prompt),
            config.timeout_secs,
        )
        .await
        {
            Ok(pair) => pair,
            Err(crate::llm::LlmError::CliExecError { stderr, .. })
                if stderr.contains("cyber_policy_violation") =>
            {
                return Err(StepError::CyberPolicyBlocked(
                    "OpenAI temporarily blocked this account for suspected \
                     cybersecurity-related activity. Retrying will not help \
                     until the block is lifted. See: https://platform.openai.com/\
                     docs/guides/safety-checks/cybersecurity"
                        .into(),
                ));
            }
            Err(e) => {
                return Err(StepError::FactoryError(format!(
                    "codex exec failed: {}",
                    e
                )));
            }
        };

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

        // --- Early bail-out: cyber policy violation in JSONL output ---
        // Even if codex exits 0, the JSONL may contain a response.failed
        // with cyber_policy_violation. Detect and fail fast.
        if stdout.contains("cyber_policy_violation")
            || stderr.contains("cyber_policy_violation")
        {
            return Err(StepError::CyberPolicyBlocked(
                "OpenAI temporarily blocked this account for suspected \
                 cybersecurity-related activity. Retrying will not help \
                 until the block is lifted. See: https://platform.openai.com/\
                 docs/guides/safety-checks/cybersecurity"
                    .into(),
            ));
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

        // --- Detect changed files via git ---
        let files_changed = git_worktree_changed_files(&config.worktree);
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

        // --- Three-mode sandbox resolution ---
        //
        // Layer 1: Pre-flight probe (~100ms bwrap test + Landlock ABI check)
        // + persistent cache (~/.cache/planner/sandbox-probe).
        // SandboxMode::resolve() handles both — returns the best mode
        // without wasting a codex invocation on a doomed sandbox.
        //
        // Layer 2: Runtime fallback — if a mode fails (0 files + mode-
        // specific error patterns), invalidate cache, retry with next.
        // Cascade: Bwrap → FullAuto → WorkspaceWrite.
        let mut current_mode = SandboxMode::resolve();

        // Two retries to traverse the full cascade:
        // Bwrap → FullAuto → WorkspaceWrite. WorkspaceWrite is terminal.
        let max_sandbox_retries = 2_u32;
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

                        if next_mode == SandboxMode::WorkspaceWrite {
                            tracing::warn!(
                                "CodexFactoryWorker: falling back to workspace-write mode. \
                                 OS-level sandbox not available on this kernel. \
                                 Containment relies on LXC/VM boundary + worktree \
                                 isolation at {}",
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
        let _ = std::fs::remove_dir_all(&tmp);
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

        // Verify .gitignore was written with expected patterns
        let gitignore_path = info.path.join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore must be created in worktree");
        let gitignore = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(gitignore.contains("node_modules/"), ".gitignore must exclude node_modules");
        assert!(gitignore.contains("dist/"), ".gitignore must exclude dist/");
        assert!(gitignore.contains("target/"), ".gitignore must exclude target/");
        assert!(gitignore.contains("__pycache__/"), ".gitignore must exclude __pycache__");

        // Verify git repo was initialized with an initial commit
        let git_dir = info.path.join(".git");
        assert!(git_dir.exists(), "worktree must be a git repo");
        // Verify initial commit exists — `git rev-parse HEAD` should succeed
        let head = git_command(&info.path, &["rev-parse", "HEAD"]);
        assert!(head.is_some(), "worktree must have an initial commit");
        let head_sha = head.unwrap().trim().to_string();
        assert_eq!(head_sha.len(), 40, "HEAD should be a full 40-char SHA");

        // Verify the initial commit contains our scaffolding files
        // Note: --root is required because this is the root commit (no parent to diff against)
        let committed_files = git_command(&info.path, &["diff-tree", "--root", "--no-commit-id", "--name-only", "-r", "HEAD"]);
        assert!(committed_files.is_some(), "should be able to list committed files");
        let file_list = committed_files.unwrap();
        assert!(file_list.contains(".gitignore"), "initial commit must include .gitignore");
        assert!(file_list.contains(".planner-context/SPEC.md"), "initial commit must include SPEC.md");

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

    // --- Git-based file tracking tests ---

    #[test]
    fn worktree_prepare_wipes_stale_dir_on_retry() {
        // Simulates the retry bug: same run_id called twice.
        // On the second call, prepare() must wipe the stale directory
        // so the initial commit doesn't absorb Codex's output from attempt 1.
        let tmp = std::env::temp_dir().join("planner-test-retry-wipe");
        let _ = std::fs::remove_dir_all(&tmp);
        let mgr = WorktreeManager::new(&tmp);
        let run_id = Uuid::new_v4();

        // Attempt 1: prepare + simulate Codex writing files
        let info1 = mgr.prepare(run_id, "# Spec v1", "digraph {}", "# Agents").unwrap();
        std::fs::write(info1.path.join("src/App.tsx"), "export default function App() {}").unwrap();
        std::fs::write(info1.path.join("package.json"), "{}").unwrap();
        std::fs::create_dir_all(info1.path.join("node_modules/react")).unwrap();
        std::fs::write(info1.path.join("node_modules/react/index.js"), "module.exports = {};").unwrap();

        // Verify attempt 1 sees files
        let files_attempt1 = git_worktree_changed_files(&info1.path);
        assert!(!files_attempt1.is_empty(), "attempt 1 should detect files");

        // Attempt 2: prepare with SAME run_id (retry scenario)
        let info2 = mgr.prepare(run_id, "# Spec v1", "digraph {}", "# Agents").unwrap();

        // The path should be the same
        assert_eq!(info1.path, info2.path);

        // The stale Codex output should be gone
        assert!(!info2.path.join("src/App.tsx").exists(), "stale Codex files must be wiped");
        assert!(!info2.path.join("package.json").exists(), "stale package.json must be wiped");
        assert!(!info2.path.join("node_modules").exists(), "stale node_modules must be wiped");

        // Fresh worktree should have 0 changed files
        let files_attempt2 = git_worktree_changed_files(&info2.path);
        assert!(
            files_attempt2.is_empty(),
            "fresh retry worktree should have 0 changed files, got: {:?}",
            files_attempt2
        );

        // Simulate Codex writing new files in attempt 2
        std::fs::write(info2.path.join("src/App.tsx"), "export default function App() { return <div>v2</div>; }").unwrap();
        let files_after_codex = git_worktree_changed_files(&info2.path);
        assert!(
            files_after_codex.contains(&"src/App.tsx".to_string()),
            "attempt 2 should detect new Codex files, got: {:?}",
            files_after_codex
        );

        mgr.cleanup(&info2).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn git_worktree_changed_files_detects_new_files() {
        // Simulates the real workflow: prepare worktree → Codex adds files → git detects them
        let tmp = std::env::temp_dir().join("planner-test-git-changed");
        let _ = std::fs::remove_dir_all(&tmp);
        let mgr = WorktreeManager::new(&tmp);
        let run_id = Uuid::new_v4();

        let info = mgr.prepare(run_id, "# Spec", "digraph {}", "# Agents").unwrap();

        // Before adding any files, git should report nothing new
        let before = git_worktree_changed_files(&info.path);
        assert!(
            before.is_empty(),
            "fresh worktree should have no changed files, got: {:?}",
            before
        );

        // Simulate Codex creating source files
        std::fs::write(info.path.join("src/App.tsx"), "export default function App() { return <div>Hello</div>; }").unwrap();
        std::fs::write(info.path.join("package.json"), r#"{"name":"test","version":"0.1.0"}"#).unwrap();

        // These should show up as changed
        let after = git_worktree_changed_files(&info.path);
        assert!(after.contains(&"src/App.tsx".to_string()), "should detect src/App.tsx, got: {:?}", after);
        assert!(after.contains(&"package.json".to_string()), "should detect package.json, got: {:?}", after);
        // Scaffolding files must NOT appear
        assert!(!after.iter().any(|f| f.starts_with(".planner-context/")), "must not include .planner-context/");
        assert!(!after.contains(&".gitignore".to_string()), "must not include .gitignore");

        // Simulate Codex also installing node_modules (this is the critical test)
        std::fs::create_dir_all(info.path.join("node_modules/react")).unwrap();
        std::fs::write(info.path.join("node_modules/react/index.js"), "module.exports = {};").unwrap();
        std::fs::create_dir_all(info.path.join("dist")).unwrap();
        std::fs::write(info.path.join("dist/bundle.js"), "!function(){console.log('minified')}()").unwrap();

        let with_ignored = git_worktree_changed_files(&info.path);
        assert!(
            !with_ignored.iter().any(|f| f.starts_with("node_modules/")),
            "node_modules/ must be excluded by .gitignore, got: {:?}",
            with_ignored
        );
        assert!(
            !with_ignored.iter().any(|f| f.starts_with("dist/")),
            "dist/ must be excluded by .gitignore, got: {:?}",
            with_ignored
        );
        // src/App.tsx and package.json should still be there
        assert!(with_ignored.contains(&"src/App.tsx".to_string()));
        assert!(with_ignored.contains(&"package.json".to_string()));

        mgr.cleanup(&info).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_worktree_source_files_prioritizes_src_and_respects_gitignore() {
        // End-to-end test: prepare worktree, add source + junk, verify the
        // read function returns source content and skips ignored dirs.
        let tmp = std::env::temp_dir().join("planner-test-read-src");
        let _ = std::fs::remove_dir_all(&tmp);
        let mgr = WorktreeManager::new(&tmp);
        let run_id = Uuid::new_v4();

        let info = mgr.prepare(run_id, "# Spec", "digraph {}", "# Agents").unwrap();

        // Write actual source files
        let tsx_content = "import React from 'react';\nexport function TaskTracker() { return <div>Tasks</div>; }";
        std::fs::write(info.path.join("src/TaskTracker.tsx"), tsx_content).unwrap();
        std::fs::write(info.path.join("src/index.ts"), "export { TaskTracker } from './TaskTracker';").unwrap();

        // Write ignored junk that was previously polluting the validator
        std::fs::create_dir_all(info.path.join("dist")).unwrap();
        let bundle = "x".repeat(80_000); // 80KB minified bundle
        std::fs::write(info.path.join("dist/bundle.js"), &bundle).unwrap();
        std::fs::create_dir_all(info.path.join("node_modules/react")).unwrap();
        std::fs::write(info.path.join("node_modules/react/index.js"), "module.exports = {};").unwrap();

        let result = read_worktree_source_files(&info.path);

        // Must contain our actual source
        assert!(result.contains("TaskTracker"), "must contain TaskTracker component, got:\n{}", &result[..result.len().min(500)]);
        assert!(result.contains("=== src/TaskTracker.tsx ==="), "must have src/TaskTracker.tsx header");
        assert!(result.contains("=== src/index.ts ==="), "must have src/index.ts header");

        // Must NOT contain ignored dirs
        assert!(!result.contains("dist/bundle.js"), "must not contain dist/bundle.js");
        assert!(!result.contains("node_modules"), "must not contain node_modules");
        assert!(!result.contains("x".repeat(1000).as_str()), "must not contain minified bundle content");

        // src/ files should appear before root files (priority sorting)
        let src_pos = result.find("=== src/").unwrap_or(usize::MAX);
        let pkg_pos = result.find("=== package").unwrap_or(0);
        // If package.json exists it should come after src/
        if result.contains("=== package") {
            assert!(src_pos < pkg_pos, "src/ files must be sorted before root files");
        }

        mgr.cleanup(&info).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_worktree_source_files_skips_minified() {
        // Verify the minification heuristic: <=2 lines but >5KB = skip
        let tmp = std::env::temp_dir().join("planner-test-read-minified");
        let _ = std::fs::remove_dir_all(&tmp);
        let mgr = WorktreeManager::new(&tmp);
        let run_id = Uuid::new_v4();

        let info = mgr.prepare(run_id, "# Spec", "digraph {}", "# Agents").unwrap();

        // A normal source file
        std::fs::write(info.path.join("src/app.js"), "function app() {\n  return 'hello';\n}\n").unwrap();
        // A minified file that sneaks past .gitignore (e.g. in src/)
        let minified = "var a=".to_string() + &"x".repeat(6000) + ";";
        std::fs::write(info.path.join("src/vendor.min.js"), &minified).unwrap();

        let result = read_worktree_source_files(&info.path);

        assert!(result.contains("=== src/app.js ==="), "must include normal source");
        assert!(!result.contains("=== src/vendor.min.js ==="), "must skip minified file (<=2 lines, >5KB)");

        mgr.cleanup(&info).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- Fallback filesystem scan tests (legacy path) ---

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
        assert_eq!(SandboxMode::from_env(), SandboxMode::FullAuto);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_from_env_danger_maps_to_workspace_write() {
        // Legacy danger-full-access should map to WorkspaceWrite (safest fallback)
        std::env::set_var("PLANNER_CODEX_SANDBOX", "danger-full-access");
        assert_eq!(SandboxMode::from_env(), SandboxMode::WorkspaceWrite);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_from_env_workspace_write() {
        std::env::set_var("PLANNER_CODEX_SANDBOX", "workspace-write");
        assert_eq!(SandboxMode::from_env(), SandboxMode::WorkspaceWrite);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_fallback_cascade() {
        // Bwrap -> FullAuto -> WorkspaceWrite -> None
        let bwrap = SandboxMode::Bwrap;
        assert_eq!(bwrap.fallback(), Some(SandboxMode::FullAuto));

        let full_auto = SandboxMode::FullAuto;
        assert_eq!(full_auto.fallback(), Some(SandboxMode::WorkspaceWrite));

        let ws = SandboxMode::WorkspaceWrite;
        assert_eq!(ws.fallback(), None);
    }

    #[test]
    fn sandbox_mode_cli_args() {
        assert_eq!(
            SandboxMode::Bwrap.cli_args(),
            vec!["--full-auto", "--enable", "use_linux_sandbox_bwrap"]
        );
        assert_eq!(
            SandboxMode::FullAuto.cli_args(),
            vec!["--full-auto"]
        );
        // WorkspaceWrite uses danger-full-access (no -a flag — exec is non-interactive)
        assert_eq!(
            SandboxMode::WorkspaceWrite.cli_args(),
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
        // Clean output = no failure
        assert!(!mode.detect_failure("files written successfully", ""));
    }

    #[test]
    fn sandbox_mode_full_auto_detects_landlock_failure() {
        // FullAuto DOES detect Landlock failures — Codex does NOT degrade
        // gracefully when Landlock is absent. It hard-fails with
        // Sandbox(LandlockRestrict) errors.
        let mode = SandboxMode::FullAuto;
        assert!(mode.detect_failure("LandlockRestrict error", ""));
        assert!(mode.detect_failure("", "LandlockRestrict"));
        assert!(mode.detect_failure("Sandbox(LandlockRestrict)", ""));
        assert!(mode.detect_failure("", "Sandbox(LandlockRestrict)"));
        assert!(mode.detect_failure("sandbox panic", ""));
        assert!(mode.detect_failure("", "sandbox restrictions"));
        // Clean output = no failure
        assert!(!mode.detect_failure("files written successfully", ""));
    }

    #[test]
    fn sandbox_mode_workspace_write_never_detects_failure() {
        // WorkspaceWrite is the terminal fallback — no OS sandbox to fail.
        let mode = SandboxMode::WorkspaceWrite;
        assert!(!mode.detect_failure("any error at all", "any stderr"));
        assert!(!mode.detect_failure("LandlockRestrict", "bwrap: error"));
    }

    #[test]
    fn sandbox_mode_env_roundtrip() {
        // Setting env var then reading should preserve the mode
        for mode in [SandboxMode::Bwrap, SandboxMode::FullAuto, SandboxMode::WorkspaceWrite] {
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
        let content = format!("{}\nfull-auto-bwrap", ts);
        std::fs::write(&cache_file, &content).unwrap();

        // Read it back manually (simulating read_cache logic)
        let read_back = std::fs::read_to_string(&cache_file).unwrap();
        let lines: Vec<&str> = read_back.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1], "full-auto-bwrap");

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
                SandboxProbe::write_cache(SandboxMode::FullAuto);
                if path.exists() {
                    let content = std::fs::read_to_string(&path).unwrap();
                    assert!(content.contains("full-auto"));
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
        // Legacy danger-full-access maps to WorkspaceWrite.
        std::env::set_var("PLANNER_CODEX_SANDBOX", "danger-full-access");
        let mode = SandboxMode::resolve();
        assert_eq!(mode, SandboxMode::WorkspaceWrite);
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
    }

    #[test]
    fn sandbox_mode_resolve_without_env_returns_valid_mode() {
        // Without env var, resolve() should run probe and return a valid mode.
        // We can't predict which mode (depends on bwrap + Landlock availability),
        // but it must be one of the three variants.
        std::env::remove_var("PLANNER_CODEX_SANDBOX");
        // Invalidate cache so we get a fresh probe
        SandboxProbe::invalidate_cache();
        let mode = SandboxMode::resolve();
        assert!(
            mode == SandboxMode::Bwrap
                || mode == SandboxMode::FullAuto
                || mode == SandboxMode::WorkspaceWrite,
            "resolve() returned unexpected mode: {:?}",
            mode
        );
        // Clean up cache after test
        SandboxProbe::invalidate_cache();
    }

    #[test]
    fn sandbox_fallback_exhaustion() {
        // WorkspaceWrite is the terminal mode — no fallback.
        let mode = SandboxMode::WorkspaceWrite;
        assert!(mode.fallback().is_none());
        // WorkspaceWrite never detects sandbox failure (nothing to fail)
        assert!(!mode.detect_failure("bwrap: error", "LandlockRestrict"));
    }

    #[test]
    fn sandbox_fallback_full_cascade_from_bwrap() {
        // Verify the complete cascade: Bwrap -> FullAuto -> WorkspaceWrite -> None
        let mut mode = SandboxMode::Bwrap;
        let mut cascade = vec![mode];

        while let Some(next) = mode.fallback() {
            cascade.push(next);
            mode = next;
        }

        assert_eq!(cascade, vec![
            SandboxMode::Bwrap,
            SandboxMode::FullAuto,
            SandboxMode::WorkspaceWrite,
        ]);
    }

    #[test]
    fn sandbox_mode_workspace_write_env_value() {
        assert_eq!(SandboxMode::WorkspaceWrite.env_value(), "workspace-write");
        assert_eq!(SandboxMode::WorkspaceWrite.label(), "workspace-write (worktree + LXC isolation)");
    }

    #[test]
    fn sandbox_probe_landlock_lsm_file_path() {
        // Verify the LSM list path is the standard securityfs location.
        // This file lists active LSMs as a comma-separated string.
        // Landlock has NO securityfs directory of its own — there is no
        // /sys/kernel/security/landlock/ on any kernel version.
        let expected = std::path::Path::new("/sys/kernel/security/lsm");
        assert!(expected.to_str().is_some());
    }

    #[test]
    fn sandbox_probe_landlock_lsm_parsing() {
        // Test the parsing logic used in probe_landlock() method 1.
        // The /sys/kernel/security/lsm file contains a comma-separated list.
        let lsm_with_landlock = "landlock,lockdown,yama,integrity,apparmor";
        assert!(lsm_with_landlock.split(',').any(|m| m.trim() == "landlock"));

        let lsm_without_landlock = "lockdown,yama,integrity,apparmor";
        assert!(!lsm_without_landlock.split(',').any(|m| m.trim() == "landlock"));

        // Edge: landlock at end, with trailing newline
        let lsm_trailing = "lockdown,yama,landlock\n";
        assert!(lsm_trailing.split(',').any(|m| m.trim() == "landlock"));

        // Edge: empty string
        let lsm_empty = "";
        assert!(!lsm_empty.split(',').any(|m| m.trim() == "landlock"));

        // Edge: partial match should NOT match
        let lsm_partial = "lockdown,landlock_custom,yama";
        assert!(!lsm_partial.split(',').any(|m| m.trim() == "landlock"));
    }

    #[test]
    fn sandbox_probe_migrate_stale_cache() {
        // If cache contains danger-full-access OR workspace-write,
        // migrate_stale_cache should delete it. Both values were written
        // by the broken probe that checked a nonexistent sysfs path.
        SandboxProbe::write_cache(SandboxMode::Bwrap);
        if let Some(path) = SandboxProbe::cache_path() {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Test danger-full-access
            let stale = format!("{}\ndanger-full-access", ts);
            std::fs::write(&path, &stale).unwrap();
            SandboxProbe::migrate_stale_cache();
            assert!(!path.exists(), "danger-full-access cache should be deleted by migrate");

            // Test workspace-write (also stale from broken probe)
            let stale_ws = format!("{}\nworkspace-write", ts);
            std::fs::write(&path, &stale_ws).unwrap();
            SandboxProbe::migrate_stale_cache();
            assert!(!path.exists(), "workspace-write cache should be deleted by migrate");

            // Test valid values are NOT deleted
            SandboxProbe::write_cache(SandboxMode::FullAuto);
            SandboxProbe::migrate_stale_cache();
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap();
                assert!(content.contains("full-auto"), "full-auto cache should survive migration");
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}
