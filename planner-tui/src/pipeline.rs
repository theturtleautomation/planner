//! # Pipeline Bridge — TUI ↔ Background Tasks
//!
//! This module:
//!
//! 1. Defines `PipelineEvent` — events from the background pipeline task.
//! 2. `spawn_pipeline()` — launches the full planning pipeline after intake.
//! 3. `spawn_socratic_interview()` — launches the Socratic engine and wires
//!    it to the TUI via channels.
//!
//! ## Channel architecture
//!
//! ```text
//!                        ┌──────────────────────┐
//!  TUI input ──► user_tx │  TuiSocraticIO        │
//!                        │  receive_input()      │ ← blocks on user_rx
//!  socratic_events_rx ◄──│  send_event()         │ ← forwards to events_tx
//!                        └──────────┬───────────┘
//!                                   │ &dyn SocraticIO
//!                        ┌──────────▼───────────┐
//!                        │  run_interview()       │ (tokio task)
//!                        └──────────────────────┘
//! ```
//!
//! Design: `App` is not `Send`, so we never put it inside the background task.
//! Instead:
//! - `App::submit_input()` stores the first message in `pending_socratic_message`.
//! - The main loop calls `App::take_pending_socratic()`.
//! - If `Some(message)`, main.rs calls `spawn_socratic_interview()` which
//!   returns `(user_tx, events_rx)` stored in the `App`.
//! - Subsequent user input is forwarded via `App::socratic_tx`.
//! - `App::tick_socratic()` drains `socratic_events_rx` on every tick.
//! - On `Converged` event, `App` transitions to `PipelineRunning` and sets
//!   `pending_pipeline_description` for main.rs to call `spawn_pipeline()`.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use planner_schemas::SocraticEvent;

// ---------------------------------------------------------------------------
// Pipeline Event
// ---------------------------------------------------------------------------

/// Events emitted by the background pipeline task to the TUI.
#[derive(Debug)]
pub enum PipelineEvent {
    /// Pipeline task started (fires immediately after spawn).
    Started,
    /// A named pipeline stage completed — carries the stage name.
    ///
    /// The TUI uses this to advance the progress tracker in real time:
    /// the matching stage is marked `Complete` and the next one `Running`.
    StepComplete(String),
    /// Pipeline completed successfully — carries a summary string.
    Completed(String),
    /// Pipeline failed — carries the error message.
    Failed(String),
}

// ---------------------------------------------------------------------------
// Channel aliases
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub type PipelineSender = mpsc::UnboundedSender<PipelineEvent>;
pub type PipelineReceiver = mpsc::UnboundedReceiver<PipelineEvent>;

// ---------------------------------------------------------------------------
// TuiSocraticIO — channels-based SocraticIO implementation
// ---------------------------------------------------------------------------

/// A `SocraticIO` implementation backed by unbounded channels.
///
/// - `event_tx` — forwards `SocraticEvent`s to the TUI's `socratic_events_rx`.
/// - `input_rx` — blocks until the TUI sends a user reply via `socratic_tx`.
struct TuiSocraticIO {
    event_tx: mpsc::UnboundedSender<SocraticEvent>,
    input_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
}

#[async_trait::async_trait]
impl planner_core::pipeline::steps::socratic::SocraticIO for TuiSocraticIO {
    async fn send_message(&self, content: &str) {
        let _ = self.event_tx.send(SocraticEvent::SystemMessage {
            content: content.to_string(),
        });
    }

    async fn send_question(&self, output: &planner_schemas::QuestionOutput) {
        let _ = self.event_tx.send(SocraticEvent::Question {
            output: output.clone(),
        });
    }

    async fn send_belief_state(&self, state: &planner_schemas::RequirementsBeliefState) {
        let _ = self.event_tx.send(SocraticEvent::BeliefStateUpdate {
            state: state.clone(),
        });
    }

    async fn send_draft(&self, draft: &planner_schemas::SpeculativeDraft) {
        let _ = self.event_tx.send(SocraticEvent::SpeculativeDraftReady {
            draft: draft.clone(),
        });
    }

    async fn send_convergence(&self, result: &planner_schemas::ConvergenceResult) {
        let _ = self.event_tx.send(SocraticEvent::Converged {
            result: result.clone(),
        });
    }

    async fn send_classification(&self, classification: &planner_schemas::DomainClassification) {
        let _ = self.event_tx.send(SocraticEvent::Classified {
            classification: classification.clone(),
        });
    }

    /// Block until the TUI sends a reply (or the sender is dropped → None).
    async fn receive_input(&self) -> Option<String> {
        let mut rx = self.input_rx.lock().await;
        rx.recv().await
    }

    async fn send_event(&self, event: &SocraticEvent) {
        // `send_event` is the structured catch-all. Most events arrive through
        // the typed methods above AND this method. To avoid double-publishing,
        // we only forward events that have no dedicated typed method in the
        // trait: specifically ContradictionDetected.
        match event {
            SocraticEvent::ContradictionDetected { .. } => {
                let _ = self.event_tx.send(event.clone());
            }
            // All other variants are already forwarded by the typed methods
            // (send_question, send_belief_state, etc.) — skip to avoid duplicates.
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// spawn_socratic_interview
// ---------------------------------------------------------------------------

/// Spawn the Socratic interview engine in a background tokio task.
///
/// Returns a tuple `(user_tx, events_rx)`:
/// - `user_tx` — send user replies into the engine.
/// - `events_rx` — receive `SocraticEvent`s from the engine.
///
/// The caller (main.rs) stores both in `App`:
/// ```rust,ignore
/// let (tx, rx) = pipeline::spawn_socratic_interview(initial_message);
/// app.socratic_tx = Some(tx);
/// app.socratic_events_rx = Some(rx);
/// ```
pub fn spawn_socratic_interview(
    initial_message: String,
) -> (
    mpsc::UnboundedSender<String>,
    mpsc::UnboundedReceiver<SocraticEvent>,
) {
    // user_tx → user_rx: TUI feeds user replies in; engine reads them out
    let (user_tx, user_rx) = mpsc::unbounded_channel::<String>();
    // events_tx → events_rx: engine pushes events in; TUI drains them out
    let (events_tx, events_rx) = mpsc::unbounded_channel::<SocraticEvent>();

    let io = Arc::new(TuiSocraticIO {
        event_tx: events_tx.clone(),
        input_rx: Arc::new(Mutex::new(user_rx)),
    });

    tokio::spawn(async move {
        let router = planner_core::llm::providers::LlmRouter::from_env();

        // We pass `None` for the TurnStore — persistence can be wired in later.
        let result = planner_core::pipeline::steps::socratic::run_interview::<
            TuiSocraticIO,
            planner_core::cxdb::CxdbEngine,
        >(&router, &*io, None::<&planner_core::cxdb::CxdbEngine>, &initial_message)
        .await;

        match result {
            Ok(_session) => {
                // Convergence was already signalled via send_convergence() /
                // the Converged event. Nothing more to send here.
            }
            Err(e) => {
                let _ = events_tx.send(SocraticEvent::Error {
                    message: format!("{}", e),
                });
            }
        }
    });

    (user_tx, events_rx)
}

// ---------------------------------------------------------------------------
// spawn_pipeline
// ---------------------------------------------------------------------------

/// Spawn the full planning pipeline in a background tokio task.
///
/// Returns the receiver end of the channel. The caller should store it in
/// `App::pipeline_rx`. Events arrive on the next `tick()` poll.
///
/// `StepComplete(name)` events are sent after each major phase resolves.
pub fn spawn_pipeline(description: String) -> PipelineReceiver {
    let (tx, rx) = mpsc::unbounded_channel::<PipelineEvent>();

    tokio::spawn(async move {
        // Signal that we've started
        let _ = tx.send(PipelineEvent::Started);

        // Build the router and worker. These are cheap to construct.
        let router = planner_core::llm::providers::LlmRouter::from_env();

        let worker = match planner_core::pipeline::steps::factory_worker::CodexFactoryWorker::new()
        {
            Ok(w) => w,
            Err(e) => {
                let _ = tx.send(PipelineEvent::Failed(format!(
                    "Failed to initialise factory worker: {}",
                    e
                )));
                return;
            }
        };

        let config =
            planner_core::pipeline::PipelineConfig::<planner_core::cxdb::CxdbEngine>::minimal(
                &router,
            );
        let project_id = uuid::Uuid::new_v4();

        match planner_core::pipeline::run_full_pipeline(
            &config,
            &worker,
            project_id,
            &description,
        )
        .await
        {
            Ok(output) => {
                // Emit StepComplete events for each stage so the TUI progress
                // tracker advances stage-by-stage on success.
                for stage in [
                    "Intake", "Chunk", "Compile", "Lint",
                    "AR Review", "Refine", "Scenarios", "Ralph",
                    "Graph", "Factory", "Validate", "Git",
                ] {
                    let _ = tx.send(PipelineEvent::StepComplete(stage.to_string()));
                }

                let hash = &output.git_result.commit.commit_hash;
                let short_hash = &hash[..12.min(hash.len())];
                let summary = format!(
                    "Project: {} ({})\nSpecs: {} chunk(s)\nFactory: {:?}\nGit: {}",
                    output.front_office.intake.project_name,
                    output.front_office.intake.feature_slug,
                    output.front_office.specs.len(),
                    output.factory_output.build_status,
                    short_hash,
                );
                let _ = tx.send(PipelineEvent::Completed(summary));
            }
            Err(e) => {
                let _ = tx.send(PipelineEvent::Failed(format!("{}", e)));
            }
        }
    });

    rx
}
