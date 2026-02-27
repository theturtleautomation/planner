//! # Pipeline Step Executors
//!
//! Each step type has its own module. Phase 0 implementations
//! are minimal — just enough to prove the full loop.
//!
//! Modules:
//! - `intake`    — Socratic interview → IntakeV1
//! - `compile`   — IntakeV1 → NLSpecV1 → GraphDotV1 → ScenarioSetV1 → AgentsManifestV1
//! - `linter`    — 12-rule NLSpec validation (deterministic, no LLM)
//! - `factory`   — Artifact handoff + Kilroy CLI invocation + checkpoint polling
//! - `validate`  — Cross-model scenario evaluation (Gemini judges Claude)
//! - `telemetry` — Factory output → plain English + Consequence Cards
//! - `git`       — Behavioral approval → standard Git commit

pub mod intake;
pub mod compile;
pub mod linter;
pub mod factory;
pub mod validate;
pub mod telemetry;
pub mod git;

/// Placeholder result type for step execution.
pub type StepResult<T> = Result<T, StepError>;

/// Step execution error.
#[derive(Debug, thiserror::Error)]
pub enum StepError {
    #[error("LLM call failed: {0}")]
    LlmError(String),

    #[error("Spec linter failed: {violations:?}")]
    LintFailure { violations: Vec<String> },

    #[error("Kilroy invocation failed: {0}")]
    KilroyError(String),

    #[error("Scenario validation failed: {0}")]
    ValidationError(String),

    #[error("Sandbox deployment failed: {0}")]
    SandboxError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("JSON parsing error: {0}")]
    JsonError(String),

    #[error("Budget exhausted")]
    BudgetExhausted,

    #[error("{0}")]
    Other(String),
}

impl From<crate::llm::LlmError> for StepError {
    fn from(e: crate::llm::LlmError) -> Self {
        StepError::LlmError(e.to_string())
    }
}
