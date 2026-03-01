//! # Unified LLM Client — CLI-Native
//!
//! Model-agnostic async LLM client supporting Anthropic (Opus/Sonnet/Haiku),
//! Google (Gemini), and OpenAI (Codex) via their **native CLI tools**.
//!
//! No HTTP API keys are used. Each provider shells out to the user's
//! locally-installed CLI binary, matching Kilroy's own backend patterns:
//!
//! - **Anthropic** → `claude -p --output-format stream-json ...`
//! - **Google**    → `gemini -p --output-format stream-json ...`
//! - **OpenAI**    → `codex exec --json ...`
//!
//! The user must have these CLIs installed and authenticated via their
//! own subscriptions (Max, Pro, etc.).
//!
//! ## Model Routing (from models.md — Feb 2026)
//!
//! | Component              | Default Model         | Provider   | Rationale (models.md)                    |
//! |------------------------|-----------------------|------------|------------------------------------------|
//! | Intake Gateway         | Claude Opus 4.6       | Anthropic  | Structured "Principal Engineer" planning |
//! | Compiler (NLSpec)      | Claude Opus 4.6       | Anthropic  | Architectural reasoning, edge cases      |
//! | Compiler (graph.dot)   | Claude Opus 4.6       | Anthropic  | Long-horizon plan mapping                |
//! | Factory Worker (code)  | GPT-5.3-Codex         | OpenAI     | SotA agentic software engineering        |
//! | Scenario Validator     | Gemini 3.1 Pro        | Google     | 1M context, cross-ref PRDs + code        |
//! | Telemetry Presenter    | Claude Haiku 4.5      | Anthropic  | Fast, low-cost summarization             |
//! | Ralph Loops            | Claude Sonnet 4.6     | Anthropic  | Near-Opus intelligence, lower latency    |
//! | AR Reviewer (Opus)     | Claude Opus 4.6       | Anthropic  | Intent completeness, anchor coverage     |
//! | AR Reviewer (GPT)      | GPT-5.2               | OpenAI     | Mathematical proofs, contradiction detect|
//! | AR Reviewer (Gemini)   | Gemini 3.1 Pro        | Google     | Scope integrity, massive context window  |
//! | AR Refiner             | Claude Opus 4.6       | Anthropic  | High-precision spec amendments           |

pub mod providers;
pub mod json_repair;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Request to an LLM.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// System prompt.
    pub system: Option<String>,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Maximum tokens to generate (advisory — some CLIs don't support this).
    pub max_tokens: u32,
    /// Temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
    /// Model identifier (provider-specific, e.g. "claude-opus-4-6").
    pub model: String,
}

/// Response from an LLM.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// The generated text.
    pub content: String,
    /// Model that was used.
    pub model: String,
    /// Input tokens consumed (0 if CLI doesn't report).
    pub input_tokens: u64,
    /// Output tokens generated (0 if CLI doesn't report).
    pub output_tokens: u64,
    /// Estimated cost in USD (0.0 for CLI subscription-based usage).
    pub estimated_cost_usd: f32,
}

/// LLM client errors.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("CLI binary not found: {binary}. Install it or check your PATH.")]
    CliBinaryNotFound { binary: String },

    #[error("CLI process failed (exit code {exit_code:?}): {stderr}")]
    CliExecError {
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Response parsing failed: {0}")]
    ParseError(String),

    #[error("CLI invocation timed out after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// LlmClient trait — the boundary all components talk to
// ---------------------------------------------------------------------------

/// The unified LLM interface. All pipeline components use this trait.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a completion request and get a response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Provider name (e.g. "anthropic", "google", "openai").
    fn provider_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Model catalog — known models and their routing
// ---------------------------------------------------------------------------

/// Known model identifiers with metadata.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: &'static str,
    pub provider: &'static str,
    /// CLI binary name for this provider.
    pub cli_binary: &'static str,
}

/// Known models catalog — aligned with models.md (Feb 2026).
///
/// Model IDs use hyphens for Rust identifiers. The CLI providers
/// translate to the correct model flag format when invoking the binary.
pub const MODELS: &[ModelInfo] = &[
    // Anthropic — uses `claude` CLI
    ModelInfo { id: "claude-opus-4-6",   provider: "anthropic", cli_binary: "claude" },
    ModelInfo { id: "claude-sonnet-4-6", provider: "anthropic", cli_binary: "claude" },
    ModelInfo { id: "claude-haiku-4-5",  provider: "anthropic", cli_binary: "claude" },
    // Google — uses `gemini` CLI
    ModelInfo { id: "gemini-3.1-pro",    provider: "google",    cli_binary: "gemini" },
    ModelInfo { id: "gemini-3.1-flash",  provider: "google",    cli_binary: "gemini" },
    // OpenAI — uses `codex` CLI
    ModelInfo { id: "gpt-5.3-codex",     provider: "openai",    cli_binary: "codex"  },
    ModelInfo { id: "gpt-5.2",           provider: "openai",    cli_binary: "codex"  },
];

/// Look up a model by ID.
pub fn find_model(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

/// Default model assignments per pipeline component.
pub struct DefaultModels;

impl DefaultModels {
    // -- Front Office: Socratic Planning (Claude Opus — "Principal Engineer") --
    pub const INTAKE_GATEWAY: &'static str = "claude-opus-4-6";
    pub const COMPILER_SPEC: &'static str = "claude-opus-4-6";
    pub const COMPILER_GRAPH_DOT: &'static str = "claude-opus-4-6";

    // -- Factory: Code Generation (GPT-5.3-Codex — SotA agentic SWE) --
    pub const FACTORY_WORKER: &'static str = "gpt-5.3-codex";

    // -- Return Trip: Validation (Gemini 3.1 Pro — 1M context, cross-ref) --
    pub const SCENARIO_VALIDATOR: &'static str = "gemini-3.1-pro";

    // -- Telemetry: Summarization (Haiku — fast, low cost) --
    pub const TELEMETRY_PRESENTER: &'static str = "claude-haiku-4-5";

    // -- Ralph Loops: Advisory (Sonnet — near-Opus, lower latency) --
    pub const RALPH_LOOPS: &'static str = "claude-sonnet-4-6";

    // -- Adversarial Review: Three model families for diverse perspectives --
    pub const AR_REVIEWER_OPUS: &'static str = "claude-opus-4-6";
    pub const AR_REVIEWER_GPT: &'static str = "gpt-5.2";
    pub const AR_REVIEWER_GEMINI: &'static str = "gemini-3.1-pro";

    // -- AR Refinement: High-precision spec amendments --
    pub const AR_REFINER: &'static str = "claude-opus-4-6";
}
