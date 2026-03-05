//! # LLM Provider Implementations — CLI Native
//!
//! Shells out to native CLI tools instead of calling HTTP APIs.
//! Each CLI uses its native sandbox/permission system properly:
//!
//! - **Anthropic** → `claude -p --permission-mode acceptEdits --output-format stream-json --verbose --model <model>`
//!   Uses `acceptEdits` permission mode: auto-approves file edits, still sandboxes bash commands.
//!   In `-p` (print) mode with stdin piping, only text completion is needed — no file/shell access.
//!   Installed via native binary (self-contained, no Node.js).
//!
//! - **Google**    → `gemini --prompt "<prompt>" --output-format json --model <model>`
//!   In headless `--prompt` mode, tool calls are disabled by default unless
//!   explicitly enabled via `coreTools` config — no sandbox needed.
//!   Installed via npm (`@google/gemini-cli`).
//!
//! - **OpenAI**    → `codex exec --json --full-auto -m <model> "<prompt>"`
//!   Uses `--full-auto`: workspace-write sandbox + auto-approve in exec mode.
//!   Installed via npm (`@openai/codex`).

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, Role};
use crate::observability;

/// Default timeout for CLI invocations (5 minutes).
const DEFAULT_TIMEOUT_SECS: u64 = 300;

// ===========================================================================
// CLI Isolation
// ===========================================================================
//
// Each LLM CLI tool (claude, gemini, codex) loads plugins, MCP servers,
// project-level configs, and extensions from various locations:
//
//   claude: ~/.claude/, ~/.claude.json, CWD/.claude/, CLAUDE.md
//   gemini: ~/.gemini/settings.json, CWD/.gemini/, extensions
//   codex:  $CODEX_HOME (~/.codex/), CWD/.codex/config.toml
//
// When the planner service invokes these CLIs, we must guarantee a clean,
// deterministic execution environment — no personal configs, no MCP servers,
// no plugins from whoever installed the binary. This is achieved by:
//
//   1. env_clear() — start with a blank environment (no inherited vars)
//   2. Inject only the vars each CLI needs (PATH, HOME, auth config paths)
//   3. Set provider-specific isolation flags (CLAUDE_CODE_SIMPLE, -e none, etc.)
//   4. Set CWD to a known-empty directory (/opt/planner/cli-sandbox)
//
// Auth credentials are stored in /opt/planner/cli-home/<provider>/ and
// are set up once via `sudo -u planner -H <cli> login`. The isolation
// env ensures the CLI reads ONLY from that directory, not from any
// project or user-level config.

/// Base directory for isolated CLI environments.
/// Each provider gets a subdirectory: claude/, gemini/, codex/
const CLI_HOME_BASE: &str = "/opt/planner/cli-home";

/// An empty directory used as CWD to prevent project-level config discovery.
const CLI_SANDBOX_DIR: &str = "/opt/planner/cli-sandbox";

/// Directory where CLI binaries are installed by deploy/install.sh.
/// The installer places claude, gemini, codex, and node here.
const CLI_BIN_DIR: &str = "/opt/planner/bin";

/// Ensure the CLI sandbox directory exists and is a git repository.
///
/// The Claude CLI requires running inside a "trusted directory" (a git
/// repo). Since we use an isolated empty directory as CWD for all CLI
/// invocations, we must `git init` it so Claude doesn't refuse to start.
///
/// This is idempotent — `git init` on an existing repo is a no-op.
/// Called lazily on first CLI invocation via `std::sync::Once`.
pub fn ensure_sandbox_git_init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let sandbox = std::path::Path::new(CLI_SANDBOX_DIR);

        // Create the directory if it doesn't exist (dev mode without install.sh)
        if !sandbox.exists() {
            if let Err(e) = std::fs::create_dir_all(sandbox) {
                tracing::warn!(
                    "Failed to create CLI sandbox dir {}: {}",
                    CLI_SANDBOX_DIR, e
                );
                return;
            }
        }

        // Check if already a git repo
        let git_dir = sandbox.join(".git");
        if git_dir.exists() {
            tracing::debug!("CLI sandbox {} is already a git repo", CLI_SANDBOX_DIR);
            return;
        }

        // git init
        let result = std::process::Command::new("git")
            .arg("init")
            .current_dir(sandbox)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match result {
            Ok(s) if s.success() => {
                // Configure git user (required for any future commits)
                let _ = std::process::Command::new("git")
                    .args(["config", "user.email", "planner@localhost"])
                    .current_dir(sandbox)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                let _ = std::process::Command::new("git")
                    .args(["config", "user.name", "Planner"])
                    .current_dir(sandbox)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                // Create an empty initial commit so it's a fully valid repo
                let _ = std::process::Command::new("git")
                    .args(["commit", "--allow-empty", "-m", "planner: cli sandbox init"])
                    .current_dir(sandbox)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                tracing::info!(
                    "CLI sandbox git-init'd at {} (Claude CLI requires trusted directory)",
                    CLI_SANDBOX_DIR
                );
            }
            Ok(s) => {
                tracing::warn!(
                    "git init in {} exited with {} — Claude CLI may refuse to run",
                    CLI_SANDBOX_DIR, s
                );
            }
            Err(e) => {
                tracing::warn!(
                    "git init failed in {}: {} — Claude CLI may refuse to run",
                    CLI_SANDBOX_DIR, e
                );
            }
        }
    });
}

/// Execution environment for an isolated CLI invocation.
#[derive(Debug, Clone)]
pub struct CliEnvironment {
    /// Environment variables (replaces the inherited environment entirely).
    pub env: HashMap<String, String>,
    /// Working directory for the CLI process.
    pub cwd: PathBuf,
}

impl CliEnvironment {
    /// Build base environment shared by all providers.
    /// Starts from an empty env and adds only what's needed.
    fn base() -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Minimal PATH: system paths + planner bin directory
        env.insert(
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/opt/planner/bin".into(),
        );

        // Temp directory (PrivateTmp in systemd, or system default)
        let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
        env.insert("TMPDIR".into(), tmp.clone());
        env.insert("TMP".into(), tmp.clone());
        env.insert("TEMP".into(), tmp);

        // Locale — prevents CLI tools from complaining
        env.insert("LANG".into(), "en_US.UTF-8".into());
        env.insert("LC_ALL".into(), "en_US.UTF-8".into());

        // Disable color codes in CLI output (we parse structured output)
        env.insert("NO_COLOR".into(), "1".into());
        env.insert("TERM".into(), "dumb".into());

        env
    }

    /// Build an isolated environment for the Anthropic `claude` CLI.
    ///
    /// Isolation strategy:
    /// - `CLAUDE_CODE_SIMPLE=true` — disables MCP servers, plugins, hooks,
    ///   CLAUDE.md loading, session memory, attachments. Fully minimal mode.
    /// - `CLAUDE_CONFIG_DIR` — points to our isolated config directory
    ///   where auth credentials are stored.
    /// - `HOME` — set to the provider's isolated home so ~/.claude.json
    ///   and ~/.claude/ resolve to our controlled directory.
    /// - CWD — empty sandbox directory prevents .claude/ and CLAUDE.md
    ///   discovery from any project.
    pub fn for_claude() -> Self {
        let mut env = Self::base();
        let home = format!("{}/claude", CLI_HOME_BASE);

        env.insert("HOME".into(), home.clone());
        env.insert("CLAUDE_CONFIG_DIR".into(), format!("{}/.claude", home));
        env.insert("CLAUDE_CODE_SIMPLE".into(), "true".into());

        // Disable MCP servers explicitly (belt and suspenders with SIMPLE mode)
        env.insert("ENABLE_CLAUDEAI_MCP_SERVERS".into(), "false".into());

        // Pass through API key from planner.env if configured (headless auth)
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            env.insert("ANTHROPIC_API_KEY".into(), key);
        }

        CliEnvironment {
            env,
            cwd: PathBuf::from(CLI_SANDBOX_DIR),
        }
    }

    /// Build an isolated environment for the Google `gemini` CLI.
    ///
    /// Isolation strategy:
    /// - `GEMINI_CLI_SYSTEM_SETTINGS_PATH` — points to our controlled
    ///   settings.json with extensions and tools disabled.
    /// - `-e none` is passed as a CLI arg (not env) to disable extensions.
    /// - `HOME` — set to the provider's isolated home so ~/.gemini/
    ///   resolves to our controlled directory.
    /// - CWD — empty sandbox directory prevents .gemini/ project config
    ///   discovery.
    pub fn for_gemini() -> Self {
        let mut env = Self::base();
        let home = format!("{}/gemini", CLI_HOME_BASE);

        env.insert("HOME".into(), home.clone());

        // Point system settings to our controlled file (highest precedence)
        let settings_path = format!("{}/settings.json", home);
        env.insert("GEMINI_CLI_SYSTEM_SETTINGS_PATH".into(), settings_path);

        // Disable sandbox (we run in headless --prompt mode, no tool execution)
        env.insert("GEMINI_SANDBOX".into(), "false".into());

        // Pass through API key from planner.env if configured (headless auth)
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            env.insert("GOOGLE_API_KEY".into(), key);
        }

        CliEnvironment {
            env,
            cwd: PathBuf::from(CLI_SANDBOX_DIR),
        }
    }

    /// Build an isolated environment for the OpenAI `codex` CLI.
    ///
    /// Isolation strategy:
    /// - `CODEX_HOME` — overrides the config directory. Points to our
    ///   controlled directory with auth but no MCP servers in config.toml.
    /// - `HOME` — set to the provider's isolated home.
    /// - CWD — empty sandbox directory prevents .codex/config.toml
    ///   discovery from any project directory.
    pub fn for_codex() -> Self {
        let mut env = Self::base();
        let home = format!("{}/codex", CLI_HOME_BASE);

        env.insert("HOME".into(), home.clone());
        env.insert("CODEX_HOME".into(), format!("{}/.codex", home));

        // XDG dirs — prevent any fallback to unexpected locations
        env.insert("XDG_CONFIG_HOME".into(), format!("{}/.config", home));
        env.insert("XDG_DATA_HOME".into(), format!("{}/.local/share", home));
        env.insert("XDG_CACHE_HOME".into(), format!("{}/.cache", home));

        // Pass through API key from planner.env if configured (headless auth)
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            env.insert("OPENAI_API_KEY".into(), key);
        }

        CliEnvironment {
            env,
            cwd: PathBuf::from(CLI_SANDBOX_DIR),
        }
    }
}

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Resolve the absolute path to a CLI binary.
///
/// First checks the planner install directory (`/opt/planner/bin/<name>`).
/// If not found there, falls back to the system PATH via `which`.
/// Returns `None` if the binary cannot be found anywhere.
pub fn resolve_cli_binary(name: &str) -> Option<String> {
    // Primary: check the planner install directory
    let installed_path = format!("{}/{}", CLI_BIN_DIR, name);
    if std::path::Path::new(&installed_path).is_file() {
        return Some(installed_path);
    }

    // Fallback: check system PATH (for dev mode / non-deployed setups)
    let output = std::process::Command::new("which")
        .arg(name)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    None
}

/// Check if a CLI binary is available (either installed or on PATH).
pub fn cli_available(name: &str) -> bool {
    resolve_cli_binary(name).is_some()
}

/// Build a single prompt string from a CompletionRequest.
///
/// CLIs like `claude -p` and `gemini -p` take a single prompt string.
/// We prepend the system prompt (if any) and concatenate all messages.
fn build_prompt(request: &CompletionRequest) -> String {
    let mut parts = Vec::new();

    if let Some(system) = &request.system {
        parts.push(format!("<system>\n{}\n</system>\n", system));
    }

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                parts.push(format!("<system>\n{}\n</system>\n", msg.content));
            }
            Role::User => {
                parts.push(format!("<user>\n{}\n</user>\n", msg.content));
            }
            Role::Assistant => {
                parts.push(format!("<assistant>\n{}\n</assistant>\n", msg.content));
            }
        }
    }

    parts.join("\n")
}

/// Run a CLI command with timeout and return (stdout, stderr).
///
/// When `cli_env` is `Some`, the process environment is fully replaced
/// (via `env_clear()` + `envs()`) and CWD is set to the sandbox directory.
/// This ensures complete isolation from the host user's config, plugins,
/// and MCP servers.
pub async fn run_cli(
    binary: &str,
    args: &[&str],
    stdin_input: Option<&str>,
    timeout_secs: u64,
    cli_env: Option<&CliEnvironment>,
) -> Result<(String, String), LlmError> {
    // Ensure the CLI sandbox is a git repo (Claude CLI requires "trusted directory")
    ensure_sandbox_git_init();

    let mut cmd = Command::new(binary);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Apply isolated environment if provided
    if let Some(env) = cli_env {
        cmd.env_clear();
        cmd.envs(&env.env);
        // Only set CWD if the directory exists (graceful fallback)
        if env.cwd.exists() {
            cmd.current_dir(&env.cwd);
        }
    }

    if stdin_input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LlmError::CliBinaryNotFound {
                binary: binary.into(),
            }
        } else {
            LlmError::CliExecError {
                exit_code: None,
                stderr: format!("Failed to spawn {}: {}", binary, e),
            }
        }
    })?;

    // Write stdin if provided
    if let Some(input) = stdin_input {
        use tokio::io::AsyncWriteExt;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).await.map_err(|e| {
                LlmError::CliExecError {
                    exit_code: None,
                    stderr: format!("Failed to write stdin: {}", e),
                }
            })?;
            // Drop stdin to signal EOF
            drop(stdin);
        }
    }

    // Wait with timeout
    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| LlmError::Timeout { timeout_secs })?
        .map_err(|e| LlmError::CliExecError {
            exit_code: None,
            stderr: format!("Process wait failed: {}", e),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(LlmError::CliExecError {
            exit_code: output.status.code(),
            stderr: if stderr.is_empty() {
                stdout.clone()
            } else {
                stderr
            },
        });
    }

    Ok((stdout, stderr))
}

/// Instrumented version of `run_cli` that emits events via EventSink.
pub async fn run_cli_instrumented(
    sink: &dyn observability::EventSink,
    session_id: Option<uuid::Uuid>,
    binary: &str,
    args: &[&str],
    stdin_input: Option<&str>,
    timeout_secs: u64,
    model: &str,
    cli_env: Option<&CliEnvironment>,
) -> Result<(String, String), LlmError> {
    use observability::{PlannerEvent, EventSource};

    let prompt_len = stdin_input.map(|s| s.len()).unwrap_or(0);
    let mut start_event = PlannerEvent::info(
        EventSource::LlmRouter,
        "llm.call.start",
        format!("Starting {} call to {}", binary, model),
    ).with_metadata(serde_json::json!({
        "model": model,
        "provider": binary,
        "prompt_len": prompt_len,
    }));
    if let Some(sid) = session_id {
        start_event = start_event.with_session(sid);
    }
    sink.emit(start_event);

    let start = std::time::Instant::now();
    let result = run_cli(binary, args, stdin_input, timeout_secs, cli_env).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match &result {
        Ok((stdout, _stderr)) => {
            let mut event = PlannerEvent::info(
                EventSource::LlmRouter,
                "llm.call.complete",
                format!("{} call to {} completed in {}ms", binary, model, elapsed_ms),
            ).with_duration(elapsed_ms)
            .with_metadata(serde_json::json!({
                "model": model,
                "provider": binary,
                "response_len": stdout.len(),
            }));
            if let Some(sid) = session_id {
                event = event.with_session(sid);
            }
            sink.emit(event);
        }
        Err(e) => {
            let (exit_code, stderr_preview) = match e {
                LlmError::CliExecError { exit_code, stderr } => {
                    (*exit_code, stderr.chars().take(200).collect::<String>())
                }
                LlmError::Timeout { timeout_secs } => {
                    (None, format!("Timed out after {}s", timeout_secs))
                }
                LlmError::CliBinaryNotFound { binary } => {
                    (None, format!("Binary not found: {}", binary))
                }
                other => (None, format!("{}", other)),
            };

            let mut event = PlannerEvent::error(
                EventSource::LlmRouter,
                "llm.call.error",
                format!("{} call to {} failed after {}ms", binary, model, elapsed_ms),
            ).with_duration(elapsed_ms)
            .with_metadata(serde_json::json!({
                "model": model,
                "provider": binary,
                "exit_code": exit_code,
                "stderr_preview": stderr_preview,
            }));
            if let Some(sid) = session_id {
                event = event.with_session(sid);
            }
            sink.emit(event);
        }
    }

    result
}

// ===========================================================================
// Anthropic — `claude` CLI
// ===========================================================================
//
// Invocation pattern:
//   claude -p --permission-mode acceptEdits --output-format stream-json \
//     --verbose --model <model>
//
// Permission mode `acceptEdits` auto-approves file edits while still
// requiring confirmation for bash commands. In `-p` (print) mode with
// stdin piping, Claude only returns text completions — no file writes
// or bash commands are executed, so this is safe and non-bypassing.
//
// The CLI sandbox directory (/opt/planner/cli-sandbox) is git-init'd
// at startup via `ensure_sandbox_git_init()` so the Claude CLI treats
// it as a trusted directory. Without this, Claude refuses to run with:
// "Not inside a trusted directory".
//
// stream-json format emits one JSON object per line. The final "result"
// message contains the assistant's response text and token usage.

pub struct AnthropicCliClient {
    timeout_secs: u64,
    env: CliEnvironment,
    /// Absolute path to the `claude` binary.
    binary_path: String,
}

impl AnthropicCliClient {
    pub fn new() -> Result<Self, LlmError> {
        let binary_path = resolve_cli_binary("claude").ok_or_else(|| {
            LlmError::CliBinaryNotFound {
                binary: "claude".into(),
            }
        })?;
        Ok(AnthropicCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            env: CliEnvironment::for_claude(),
            binary_path,
        })
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// A single event from Claude's stream-json output.
#[allow(dead_code)] // Constructed via serde deserialization of Claude CLI stream-json output
#[derive(Debug, Deserialize)]
struct ClaudeStreamEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    /// Present on "result" events.
    result: Option<String>,
    /// Token usage (present on some event types).
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    /// Cost tracking (if --verbose).
    cost_usd: Option<f64>,
    /// Subtype for content blocks.
    subtype: Option<String>,
}

/// Parsed from the stream-json "result" message.
#[derive(Debug, Deserialize)]
struct ClaudeResult {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    result_type: Option<String>,
    /// The full assistant text (in the final result block).
    result: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost_usd: Option<f64>,
}

#[async_trait]
impl LlmClient for AnthropicCliClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let prompt = build_prompt(&request);

        // Use acceptEdits permission mode: auto-approves file edits,
        // still sandboxes bash commands. In -p (print) mode with stdin
        // piping we only get text back, so no file/shell access occurs.
        let model_arg = request.model.clone();
        let args = vec![
            "-p",
            "--permission-mode",
            "acceptEdits",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            &model_arg,
        ];

        let (stdout, _stderr) = run_cli(&self.binary_path, &args, Some(&prompt), self.timeout_secs, Some(&self.env)).await?;

        // Parse stream-json: one JSON object per line.
        // We want the final result that contains the complete text.
        let mut content = String::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut cost_usd: f32 = 0.0;

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try parsing as a result object
            if let Ok(evt) = serde_json::from_str::<ClaudeResult>(trimmed) {
                if let Some(text) = evt.result {
                    content = text;
                }
                if let Some(t) = evt.input_tokens {
                    input_tokens = t;
                }
                if let Some(t) = evt.output_tokens {
                    output_tokens = t;
                }
                if let Some(c) = evt.cost_usd {
                    cost_usd = c as f32;
                }
            }
        }

        if content.is_empty() {
            // Fallback: maybe non-streaming output, entire stdout is the response
            content = stdout.trim().to_string();
        }

        Ok(CompletionResponse {
            content,
            model: request.model,
            input_tokens,
            output_tokens,
            estimated_cost_usd: cost_usd,
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}

// ===========================================================================
// Google — `gemini` CLI
// ===========================================================================
//
// Invocation pattern:
//   gemini --prompt "<prompt>" --output-format json --model <model>
//
// In headless --prompt mode, tool calls (shell, file writes) are disabled
// by default unless explicitly enabled via `coreTools` config. This means
// no sandbox is needed — the model can only return text.
//
// NOTE: --sandbox requires a Docker image (gemini-cli-sandbox) that may
// not be available on all systems, and is unnecessary for text-only
// headless completions. Only use --sandbox for interactive/agentic mode.
//
// --output-format json returns a single JSON object.
// NOTE: Gemini CLI's -p/--prompt requires the prompt as its VALUE
// (it does NOT read from stdin like Claude's -p).

pub struct GoogleCliClient {
    timeout_secs: u64,
    env: CliEnvironment,
    /// Absolute path to the `gemini` binary.
    binary_path: String,
}

impl GoogleCliClient {
    pub fn new() -> Result<Self, LlmError> {
        let binary_path = resolve_cli_binary("gemini").ok_or_else(|| {
            LlmError::CliBinaryNotFound {
                binary: "gemini".into(),
            }
        })?;
        Ok(GoogleCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            env: CliEnvironment::for_gemini(),
            binary_path,
        })
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Gemini CLI `--output-format json` response.
///
/// Shape: `{ "response": "<model text>", ... }` — a single JSON object.
/// May also include token counts depending on the Gemini CLI version.
#[allow(dead_code)] // Constructed via serde deserialization of Gemini CLI json output
#[derive(Debug, Deserialize)]
struct GeminiJsonResponse {
    /// The model's text output.
    #[serde(default)]
    response: Option<String>,
    /// Alternative field name used by some Gemini CLI versions.
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

#[async_trait]
impl LlmClient for GoogleCliClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let prompt = build_prompt(&request);

        // Gemini CLI requires the prompt as the VALUE to --prompt / -p.
        // Unlike Claude, it does NOT read from stdin when -p is bare.
        // Error without this: "Not enough arguments following: p"
        //
        // No --sandbox needed: in headless --prompt mode, tool calls
        // (shell, file writes) are disabled by default. The --sandbox
        // flag requires a Docker image that may not be available, and
        // is unnecessary for text-only completions.
        let model_arg = request.model.clone();
        let args = vec![
            "--prompt",
            &prompt,
            "--output-format",
            "json",
            "--model",
            &model_arg,
        ];

        let (stdout, _stderr) = run_cli(&self.binary_path, &args, None, self.timeout_secs, Some(&self.env)).await?;

        // Gemini --output-format json emits a single JSON object.
        // Try structured parse first, fall back to raw stdout.
        let (content, input_tokens, output_tokens) =
            if let Ok(resp) = serde_json::from_str::<GeminiJsonResponse>(stdout.trim()) {
                let text = resp
                    .response
                    .or(resp.result)
                    .unwrap_or_else(|| stdout.trim().to_string());
                (text, resp.input_tokens.unwrap_or(0), resp.output_tokens.unwrap_or(0))
            } else {
                // Fallback: entire stdout is the response text
                (stdout.trim().to_string(), 0u64, 0u64)
            };

        Ok(CompletionResponse {
            content,
            model: request.model,
            input_tokens,
            output_tokens,
            estimated_cost_usd: 0.0, // subscription-based
        })
    }

    fn provider_name(&self) -> &str {
        "google"
    }
}

// ===========================================================================
// OpenAI — `codex` CLI
// ===========================================================================
//
// Kilroy pattern:
//   codex exec --json --sandbox workspace-write -m <model> -C <worktree> "<prompt>"
//
// `codex exec --json` returns JSONL (newline-delimited JSON events).
// Event types: thread.started, turn.started, item.completed, turn.completed.
// The assistant's text is in item.completed events where item.type == "message".
// We also use --output-last-message for reliable final text extraction.

pub struct OpenAiCliClient {
    timeout_secs: u64,
    env: CliEnvironment,
    /// Absolute path to the `codex` binary.
    binary_path: String,
}

impl OpenAiCliClient {
    pub fn new() -> Result<Self, LlmError> {
        let binary_path = resolve_cli_binary("codex").ok_or_else(|| {
            LlmError::CliBinaryNotFound {
                binary: "codex".into(),
            }
        })?;
        Ok(OpenAiCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            env: CliEnvironment::for_codex(),
            binary_path,
        })
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// A single event from Codex's `--json` JSONL stream.
///
/// `codex exec --json` emits newline-delimited JSON events:
/// - `{"type": "thread.started", ...}`
/// - `{"type": "turn.started"}`
/// - `{"type": "item.completed", "item": {"type": "reasoning", "text": "..."}}`
/// - `{"type": "item.completed", "item": {"type": "message", "content": [{"type": "output_text", "text": "..."}]}}`
/// - `{"type": "turn.completed"}`
///
/// We extract text from `item.completed` events where `item.type == "message"`.
#[derive(Debug, Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    item: Option<CodexItem>,
}

#[derive(Debug, Deserialize)]
struct CodexItem {
    #[serde(rename = "type")]
    item_type: String,
    /// Present on message items — array of content blocks.
    #[serde(default)]
    content: Option<Vec<CodexContentBlock>>,
    /// Present on reasoning items — raw text (not extracted for pipeline use).
    #[serde(default)]
    #[allow(dead_code)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[async_trait]
impl LlmClient for OpenAiCliClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let prompt = build_prompt(&request);

        let model_arg = request.model.clone();

        // Use --output-last-message to a temp file for reliable extraction,
        // plus --json so we still get structured events on stdout.
        // The prompt goes via stdin (use "-" as the PROMPT arg).
        let output_file = std::env::temp_dir().join(format!("codex-out-{}.txt", uuid::Uuid::new_v4()));
        let output_path = output_file.to_string_lossy().to_string();

        let args = vec![
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "-m",
            &model_arg,
            "--output-last-message",
            &output_path,
            "-",  // read prompt from stdin
        ];

        let (stdout, _stderr) = run_cli(&self.binary_path, &args, Some(&prompt), self.timeout_secs, Some(&self.env)).await?;

        // Strategy 1: Read from --output-last-message file (most reliable)
        let content = if output_file.exists() {
            let file_content = std::fs::read_to_string(&output_file).unwrap_or_default();
            let _ = std::fs::remove_file(&output_file);
            if !file_content.trim().is_empty() {
                file_content.trim().to_string()
            } else {
                extract_codex_message_from_jsonl(&stdout)
            }
        } else {
            // Strategy 2: Parse JSONL events from stdout
            extract_codex_message_from_jsonl(&stdout)
        };

        Ok(CompletionResponse {
            content,
            model: request.model,
            input_tokens: 0,  // codex exec doesn't report token counts
            output_tokens: 0,
            estimated_cost_usd: 0.0, // subscription-based
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

/// Parse Codex JSONL stream and extract the assistant's message text.
///
/// Looks for `item.completed` events where `item.type == "message"`,
/// then concatenates all `output_text` content blocks.
pub fn extract_codex_message_from_jsonl(jsonl: &str) -> String {
    let mut messages = Vec::new();

    for line in jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(evt) = serde_json::from_str::<CodexEvent>(trimmed) {
            if evt.event_type == "item.completed" {
                if let Some(item) = &evt.item {
                    if item.item_type == "message" {
                        // Extract text from content blocks
                        if let Some(blocks) = &item.content {
                            for block in blocks {
                                if let Some(text) = &block.text {
                                    messages.push(text.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if messages.is_empty() {
        // Last resort: return all stdout (maybe not JSON mode)
        jsonl.trim().to_string()
    } else {
        messages.join("\n")
    }
}

// ===========================================================================
// Router — picks the right CLI provider based on model ID
// ===========================================================================

/// Routes requests to the correct CLI provider based on model ID.
pub struct LlmRouter {
    anthropic: Option<AnthropicCliClient>,
    google: Option<GoogleCliClient>,
    openai: Option<OpenAiCliClient>,
    /// Optional mock client — when set, ALL requests are routed here
    /// regardless of model ID.  Used by integration tests.
    mock: Option<Box<dyn LlmClient>>,
}

impl LlmRouter {
    /// Create a router, initializing providers by checking CLI availability.
    /// Missing CLI binaries produce None for that provider (fails at call time).
    pub fn from_env() -> Self {
        LlmRouter {
            anthropic: AnthropicCliClient::new().ok(),
            google: GoogleCliClient::new().ok(),
            openai: OpenAiCliClient::new().ok(),
            mock: None,
        }
    }

    /// Create a router backed entirely by a mock LLM client.
    /// Every `complete()` call is forwarded to this mock regardless of model.
    pub fn with_mock(client: Box<dyn LlmClient>) -> Self {
        LlmRouter {
            anthropic: None,
            google: None,
            openai: None,
            mock: Some(client),
        }
    }

    /// Report which CLI providers are available.
    pub fn available_providers(&self) -> Vec<&str> {
        let mut providers = Vec::new();
        if self.anthropic.is_some() {
            providers.push("anthropic (claude)");
        }
        if self.google.is_some() {
            providers.push("google (gemini)");
        }
        if self.openai.is_some() {
            providers.push("openai (codex)");
        }
        providers
    }

    /// Route a request to the appropriate provider.
    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        let provider = self.resolve_provider(&request.model)?;
        provider.complete(request).await
    }

    /// Route a request to the appropriate provider, emitting observability
    /// events before and after the LLM call via `sink`.
    ///
    /// Identical to `complete()` in behaviour; adds structured start/complete/error
    /// events so callers can track LLM latency without wrapping every call site.
    pub async fn complete_instrumented(
        &self,
        request: CompletionRequest,
        sink: &dyn observability::EventSink,
        session_id: Option<uuid::Uuid>,
    ) -> Result<CompletionResponse, LlmError> {
        use observability::{PlannerEvent, EventSource};

        // Determine which binary would be used for this model, for event metadata.
        let binary = self.binary_for_model(&request.model);
        let model = request.model.clone();
        let prompt_len = request.messages.iter().map(|m| m.content.len()).sum::<usize>();

        let mut start_event = PlannerEvent::info(
            EventSource::LlmRouter,
            "llm.call.start",
            format!("Starting {} call to {}", binary, model),
        ).with_metadata(serde_json::json!({
            "model": model,
            "provider": binary,
            "prompt_len": prompt_len,
        }));
        if let Some(sid) = session_id {
            start_event = start_event.with_session(sid);
        }
        sink.emit(start_event);

        let start = std::time::Instant::now();
        let result = self.complete(request).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(resp) => {
                let mut event = PlannerEvent::info(
                    EventSource::LlmRouter,
                    "llm.call.complete",
                    format!("{} call to {} completed in {}ms", binary, model, elapsed_ms),
                ).with_duration(elapsed_ms)
                .with_metadata(serde_json::json!({
                    "model": model,
                    "provider": binary,
                    "response_len": resp.content.len(),
                    "input_tokens": resp.input_tokens,
                    "output_tokens": resp.output_tokens,
                }));
                if let Some(sid) = session_id {
                    event = event.with_session(sid);
                }
                sink.emit(event);
            }
            Err(e) => {
                let stderr_preview = format!("{}", e).chars().take(200).collect::<String>();
                let mut event = PlannerEvent::error(
                    EventSource::LlmRouter,
                    "llm.call.error",
                    format!("{} call to {} failed after {}ms", binary, model, elapsed_ms),
                ).with_duration(elapsed_ms)
                .with_metadata(serde_json::json!({
                    "model": model,
                    "provider": binary,
                    "error": stderr_preview,
                }));
                if let Some(sid) = session_id {
                    event = event.with_session(sid);
                }
                sink.emit(event);
            }
        }

        result
    }

    /// Return the CLI binary name that would be used for a given model ID.
    /// Used for event metadata in `complete_instrumented`.
    fn binary_for_model(&self, model: &str) -> &'static str {
        if let Some(ref _mock) = self.mock {
            return "mock";
        }
        if model.starts_with("claude-") {
            "claude"
        } else if model.starts_with("gemini-") {
            "gemini"
        } else if model.starts_with("gpt-") {
            "codex"
        } else {
            "claude" // default
        }
    }

    fn resolve_provider(&self, model: &str) -> Result<&dyn LlmClient, LlmError> {
        // If a mock is installed, always use it.
        if let Some(ref mock) = self.mock {
            return Ok(mock.as_ref());
        }
        if model.starts_with("claude-") {
            self.anthropic
                .as_ref()
                .map(|c| c as &dyn LlmClient)
                .ok_or_else(|| LlmError::CliBinaryNotFound {
                    binary: "claude".into(),
                })
        } else if model.starts_with("gemini-") {
            self.google
                .as_ref()
                .map(|c| c as &dyn LlmClient)
                .ok_or_else(|| LlmError::CliBinaryNotFound {
                    binary: "gemini".into(),
                })
        } else if model.starts_with("gpt-") {
            self.openai
                .as_ref()
                .map(|c| c as &dyn LlmClient)
                .ok_or_else(|| LlmError::CliBinaryNotFound {
                    binary: "codex".into(),
                })
        } else {
            // Default to Anthropic for unknown models
            self.anthropic
                .as_ref()
                .map(|c| c as &dyn LlmClient)
                .ok_or_else(|| LlmError::Other(format!("No provider for model: {}", model)))
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{CompletionRequest, Message, Role};

    #[test]
    fn build_prompt_includes_system() {
        let request = CompletionRequest {
            system: Some("You are a helpful assistant.".into()),
            messages: vec![Message {
                role: Role::User,
                content: "Hello".into(),
            }],
            max_tokens: 1024,
            temperature: 0.0,
            model: "claude-opus-4-6".into(),
        };
        let prompt = build_prompt(&request);
        assert!(prompt.contains("<system>"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("<user>"));
        assert!(prompt.contains("Hello"));
    }

    #[test]
    fn build_prompt_multi_turn() {
        let request = CompletionRequest {
            system: None,
            messages: vec![
                Message {
                    role: Role::User,
                    content: "What is Rust?".into(),
                },
                Message {
                    role: Role::Assistant,
                    content: "A systems language.".into(),
                },
                Message {
                    role: Role::User,
                    content: "Tell me more.".into(),
                },
            ],
            max_tokens: 1024,
            temperature: 0.7,
            model: "claude-sonnet-4-6".into(),
        };
        let prompt = build_prompt(&request);
        assert!(prompt.contains("<user>\nWhat is Rust?\n</user>"));
        assert!(prompt.contains("<assistant>\nA systems language.\n</assistant>"));
        assert!(prompt.contains("<user>\nTell me more.\n</user>"));
    }

    #[test]
    fn router_model_resolution() {
        // We can't actually test CLI availability in this sandbox,
        // but we can verify the resolution logic by constructing manually.
        let router = LlmRouter {
            anthropic: None,
            google: None,
            openai: None,
            mock: None,
        };

        // All providers absent → should get CliBinaryNotFound
        assert!(router.resolve_provider("claude-opus-4-6").is_err());
        assert!(router.resolve_provider("gemini-3.1-pro-preview").is_err());
        assert!(router.resolve_provider("gpt-5.3-codex").is_err());
    }

    #[test]
    fn find_model_known() {
        use crate::llm::find_model;
        let m = find_model("claude-opus-4-6").unwrap();
        assert_eq!(m.provider, "anthropic");
        assert_eq!(m.cli_binary, "claude");

        let m = find_model("gemini-3.1-pro-preview").unwrap();
        assert_eq!(m.provider, "google");
        assert_eq!(m.cli_binary, "gemini");

        let m = find_model("gpt-5.3-codex").unwrap();
        assert_eq!(m.provider, "openai");
        assert_eq!(m.cli_binary, "codex");
    }

    #[test]
    fn find_model_unknown() {
        use crate::llm::find_model;
        assert!(find_model("llama-3-70b").is_none());
    }

    #[test]
    fn cli_available_detects_common_binary() {
        // `ls` should always exist
        assert!(cli_available("ls"));
        // something nonsensical should not
        assert!(!cli_available("zzz_nonexistent_binary_999"));
    }

    #[test]
    fn binary_for_model_routes_correctly() {
        let router = LlmRouter {
            anthropic: None,
            google: None,
            openai: None,
            mock: None,
        };
        assert_eq!(router.binary_for_model("claude-opus-4-6"), "claude");
        assert_eq!(router.binary_for_model("gemini-3.1-pro-preview"), "gemini");
        assert_eq!(router.binary_for_model("gpt-5.3-codex"), "codex");
        assert_eq!(router.binary_for_model("unknown-model"), "claude");
    }

    #[tokio::test]
    async fn complete_instrumented_emits_events_on_success() {
        use crate::llm::{CompletionRequest, CompletionResponse, Message, Role, LlmError};
        use crate::observability::{CollectorEventSink, EventLevel};
        use async_trait::async_trait;

        struct MockClient;
        #[async_trait]
        impl LlmClient for MockClient {
            async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
                Ok(CompletionResponse {
                    content: "mock response".into(),
                    model: req.model,
                    input_tokens: 10,
                    output_tokens: 5,
                    estimated_cost_usd: 0.0,
                })
            }
            fn provider_name(&self) -> &str { "mock" }
        }

        let router = LlmRouter::with_mock(Box::new(MockClient));
        let sink = CollectorEventSink::new();
        let request = CompletionRequest {
            system: None,
            messages: vec![Message { role: Role::User, content: "Hello".into() }],
            max_tokens: 100,
            temperature: 0.0,
            model: "claude-opus-4-6".into(),
        };

        let result = router.complete_instrumented(request, &sink, None).await;
        assert!(result.is_ok());
        assert_eq!(sink.count(), 2); // start + complete

        let events = sink.events();
        assert_eq!(events[0].step.as_deref(), Some("llm.call.start"));
        assert_eq!(events[0].level, EventLevel::Info);
        assert_eq!(events[1].step.as_deref(), Some("llm.call.complete"));
        assert_eq!(events[1].level, EventLevel::Info);
        assert!(events[1].duration_ms.is_some());
    }

    #[tokio::test]
    async fn complete_instrumented_emits_error_event_on_failure() {
        use crate::llm::{CompletionRequest, CompletionResponse, Message, Role, LlmError};
        use crate::observability::{CollectorEventSink, EventLevel};
        use async_trait::async_trait;

        struct FailingClient;
        #[async_trait]
        impl LlmClient for FailingClient {
            async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
                Err(LlmError::CliBinaryNotFound { binary: "claude".into() })
            }
            fn provider_name(&self) -> &str { "mock" }
        }

        let router = LlmRouter::with_mock(Box::new(FailingClient));
        let sink = CollectorEventSink::new();
        let request = CompletionRequest {
            system: None,
            messages: vec![Message { role: Role::User, content: "Hello".into() }],
            max_tokens: 100,
            temperature: 0.0,
            model: "claude-opus-4-6".into(),
        };

        let result = router.complete_instrumented(request, &sink, None).await;
        assert!(result.is_err());
        assert_eq!(sink.count(), 2); // start + error

        let events = sink.events();
        assert_eq!(events[0].step.as_deref(), Some("llm.call.start"));
        assert_eq!(events[1].step.as_deref(), Some("llm.call.error"));
        assert_eq!(events[1].level, EventLevel::Error);
    }
}
