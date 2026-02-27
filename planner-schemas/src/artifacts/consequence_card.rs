//! # planner.consequence_card.v1
//!
//! Plain-English trade-off card surfaced in the Impact Inbox.
//! Produced by the Telemetry Presenter when the factory hits an
//! intent gap, a budget warning, or a gate failure.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::turn::ArtifactPayload;

// ---------------------------------------------------------------------------
// ConsequenceCardV1
// ---------------------------------------------------------------------------

/// A plain-English trade-off card presented to the Smart Tinkerer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceCardV1 {
    /// Unique card identifier.
    pub card_id: Uuid,

    /// Which project this card belongs to.
    pub project_id: Uuid,

    /// What triggered this card.
    pub trigger: CardTrigger,

    /// Plain-English problem description.
    pub problem: String,

    /// Proposed solution or trade-off.
    pub proposed_solution: String,

    /// Impact assessment in plain English.
    pub impact: String,

    /// Available actions the user can take.
    pub actions: Vec<CardAction>,

    /// Current card status.
    pub status: CardStatus,

    /// User's chosen action, once resolved.
    pub resolution: Option<CardResolution>,
}

impl ArtifactPayload for ConsequenceCardV1 {
    const TYPE_ID: &'static str = "planner.consequence_card.v1";
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// What triggered the Consequence Card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardTrigger {
    /// Factory coding agent hit an ambiguity not resolvable from artifacts.
    IntentGap,
    /// Run budget warning threshold crossed.
    BudgetWarning,
    /// Run budget hard cap reached.
    BudgetExhausted,
    /// Critical scenario gate failed.
    CriticalGateFailure,
    /// High scenario gate failed (after retries).
    HighGateFailure,
    /// Open Question from spec that needs user input.
    OpenQuestion,
    /// Ralph Loop finding that needs user decision.
    RalphFinding,
    /// AR blocking finding that needs user clarification.
    ArBlockingFinding,
}

/// An action the user can take on a Consequence Card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAction {
    /// Action label (e.g. "Approve", "Discuss", "Dismiss").
    pub label: String,
    /// What this action does, in plain English.
    pub description: String,
}

/// Consequence Card lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardStatus {
    /// Awaiting user action.
    Pending,
    /// User has taken action.
    Resolved,
    /// Card was superseded by a newer card or event.
    Superseded,
}

/// The user's resolution of a Consequence Card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardResolution {
    /// Which action the user chose.
    pub chosen_action: String,
    /// Any additional input from the user.
    pub user_input: Option<String>,
    /// ISO 8601 timestamp of resolution.
    pub resolved_at: String,
}
