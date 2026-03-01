//! # LLM Provider Implementations — CLI Native
//!
//! Shells out to native CLI tools instead of calling HTTP APIs.
//! Mirrors Kilroy's backend CLI patterns exactly:
//!
//! - **Anthropic** → `claude -p --dangerously-skip-permissions --output-format stream-json --verbose --model <model> "<prompt)"`
//! - **Google**    → `gemini --prompt "<prompt>" --output-format json --yolo --model <model>`
//! - **OpenAI**    → `codex exec --json --sandbox workspace-write -m <model> "<prompt>"`

use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, Role};

/// Default timeout for CLI invocations (5 minutes).
const DEFAULT_TIMEOUT_SECS: u64 = 300;

// ===========================================================================
// Shared helpers
// ===========================================================================

/// Check if a CLI binary exists on the system PATH.
pub fn cli_available(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
pub async fn run_cli(
    binary: &str,
    args: &[&str],
    stdin_input: Option<&str>,
    timeout_secs: u64,
) -> Result<(String, String), LlmError> {
    let mut cmd = Command::new(binary);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

// ===========================================================================
// Anthropic — `claude` CLI
// ===========================================================================
//
// Kilroy pattern:
//   claude -p --dangerously-skip-permissions --output-format stream-json \
//     --verbose --model <model> "<prompt>"
//
// stream-json format emits one JSON object per line. The final "result"
// message contains the assistant's response text and token usage.

pub struct AnthropicCliClient {
    timeout_secs: u64,
}

impl AnthropicCliClient {
    pub fn new() -> Result<Self, LlmError> {
        if !cli_available("claude") {
            return Err(LlmError::CliBinaryNotFound {
                binary: "claude".into(),
            });
        }
        Ok(AnthropicCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
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

        // Build args matching Kilroy's Anthropic backend pattern
        let model_arg = request.model.clone();
        let args = vec![
            "-p",
            "--dangerously-skip-permissions",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            &model_arg,
        ];

        // Pipe prompt via stdin (avoids shell escaping issues with -p)
        let (stdout, _stderr) = run_cli("claude", &args, Some(&prompt), self.timeout_secs).await?;

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
// Kilroy pattern:
//   gemini --prompt "<prompt>" --output-format json --yolo --model <model>
//
// --output-format json returns a single JSON object.
// NOTE: Gemini CLI's -p/--prompt requires the prompt as its VALUE
// (it does NOT read from stdin like Claude's -p).

pub struct GoogleCliClient {
    timeout_secs: u64,
}

impl GoogleCliClient {
    pub fn new() -> Result<Self, LlmError> {
        if !cli_available("gemini") {
            return Err(LlmError::CliBinaryNotFound {
                binary: "gemini".into(),
            });
        }
        Ok(GoogleCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
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
        let model_arg = request.model.clone();
        let args = vec![
            "--prompt",
            &prompt,
            "--output-format",
            "json",
            "--yolo",
            "--model",
            &model_arg,
        ];

        let (stdout, _stderr) = run_cli("gemini", &args, None, self.timeout_secs).await?;

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
// `codex exec` returns a JSON object with the result.
// For our purposes (non-coding LLM calls), we use a simpler invocation
// without the -C worktree flag.

pub struct OpenAiCliClient {
    timeout_secs: u64,
}

impl OpenAiCliClient {
    pub fn new() -> Result<Self, LlmError> {
        if !cli_available("codex") {
            return Err(LlmError::CliBinaryNotFound {
                binary: "codex".into(),
            });
        }
        Ok(OpenAiCliClient {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Codex exec JSON response.
#[derive(Debug, Deserialize)]
struct CodexExecResponse {
    /// The output text from codex.
    #[serde(default)]
    output: Option<String>,
    /// Alternative field name.
    #[serde(default)]
    result: Option<String>,
    /// Alternative: response text.
    #[serde(default)]
    response: Option<String>,
}

#[async_trait]
impl LlmClient for OpenAiCliClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let prompt = build_prompt(&request);

        let model_arg = request.model.clone();
        // Deliver prompt via stdin (matching Anthropic/Google pattern).
        // Avoids shell escaping issues with long prompts as positional args.
        let args = vec![
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "-m",
            &model_arg,
        ];

        let (stdout, _stderr) = run_cli("codex", &args, Some(&prompt), self.timeout_secs).await?;

        // Try to parse structured JSON response
        let content = if let Ok(resp) = serde_json::from_str::<CodexExecResponse>(&stdout) {
            resp.output
                .or(resp.result)
                .or(resp.response)
                .unwrap_or_else(|| stdout.trim().to_string())
        } else {
            // Fallback: entire stdout is the response
            stdout.trim().to_string()
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
}
