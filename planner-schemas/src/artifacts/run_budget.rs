//! # planner.run_budget.v1
//!
//! Financial circuit breaker state. The Factory Diplomat enforces a hard
//! spend cap per run. Kilroy's `tool_hooks.pre` calls the spend check
//! endpoint before each LLM call. No override mechanism exists.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// RunBudgetV1
// ---------------------------------------------------------------------------

/// Financial circuit breaker for a single Kilroy run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBudgetV1 {
    /// Which project this budget applies to.
    pub project_id: Uuid,

    /// Which run this budget tracks.
    pub run_id: Uuid,

    /// Hard spend cap in USD. No overrides. Default $5.00 in Phase 0.
    pub hard_cap_usd: f32,

    /// Warning threshold in USD (default 80% of hard cap).
    /// Crossing this triggers a Consequence Card to the user.
    pub warn_threshold_usd: f32,

    /// Current cumulative spend in USD.
    pub current_spend_usd: f32,

    /// Current budget status.
    pub status: BudgetStatus,

    /// Spend event log.
    pub events: Vec<SpendEvent>,
}

impl ArtifactPayload for RunBudgetV1 {
    const TYPE_ID: &'static str = "planner.run_budget.v1";
}

impl RunBudgetV1 {
    /// Create a new budget with default Phase 0 values.
    pub fn new_phase0(project_id: Uuid, run_id: Uuid) -> Self {
        let hard_cap = 5.0;
        RunBudgetV1 {
            project_id,
            run_id,
            hard_cap_usd: hard_cap,
            warn_threshold_usd: hard_cap * 0.80,
            current_spend_usd: 0.0,
            status: BudgetStatus::Active,
            events: Vec::new(),
        }
    }

    /// Record a spend event and update status.
    pub fn record_spend(&mut self, event: SpendEvent) {
        self.current_spend_usd += event.amount_usd;
        self.events.push(event);
        self.update_status();
    }

    /// Recalculate status from current spend.
    fn update_status(&mut self) {
        if self.current_spend_usd >= self.hard_cap_usd {
            self.status = BudgetStatus::HardStop;
        } else if self.current_spend_usd >= self.warn_threshold_usd {
            self.status = BudgetStatus::Warning;
        } else {
            self.status = BudgetStatus::Active;
        }
    }

    /// Check whether the factory should proceed. Called by the
    /// Factory Diplomat's spend check endpoint.
    pub fn can_proceed(&self) -> bool {
        self.status != BudgetStatus::HardStop
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Budget status — drives Factory Diplomat behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BudgetStatus {
    /// Under warning threshold. Normal operation.
    Active,
    /// Warning threshold crossed. Consequence Card surfaced to user.
    Warning,
    /// Hard cap crossed. Kilroy process killed, run archived.
    HardStop,
}

/// A single LLM API spend event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendEvent {
    /// When the spend occurred.
    pub timestamp: DateTime<Utc>,

    /// Which graph.dot node incurred the spend.
    pub node_name: String,

    /// Which LLM model was called.
    pub model: String,

    /// Input tokens.
    pub input_tokens: u64,

    /// Output tokens.
    pub output_tokens: u64,

    /// USD cost of this call.
    pub amount_usd: f32,
}
