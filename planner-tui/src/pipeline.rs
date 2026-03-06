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
//!   returns `(user_tx, events_rx, planner_events_rx)` stored in the `App`.
//! - Subsequent user input is forwarded via `App::socratic_tx`.
//! - `App::tick_socratic()` drains `socratic_events_rx` on every tick.
//! - `App::tick_planner_events()` drains `planner_events_rx` on every tick.
//! - On `Converged` event, `App` transitions to `PipelineRunning` and sets
//!   `pending_pipeline_description` for main.rs to call `spawn_pipeline()`.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use planner_core::observability::{ChannelEventSink, EventSink, EventSource, PlannerEvent};
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
    /// Observability event sink for structured planner events.
    event_sink: Arc<dyn EventSink>,
    /// Session ID for tagging emitted events.
    session_id: uuid::Uuid,
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
        self.event_sink.emit(
            PlannerEvent::info(
                EventSource::SocraticEngine,
                "socratic.question.generated",
                format!(
                    "Question generated for dimension '{}'",
                    output.target_dimension.label()
                ),
            )
            .with_session(self.session_id)
            .with_metadata(serde_json::json!({
                "target_dimension": output.target_dimension.label(),
                "allow_skip": output.allow_skip,
            })),
        );
    }

    async fn send_belief_state(&self, state: &planner_schemas::RequirementsBeliefState) {
        let _ = self.event_tx.send(SocraticEvent::BeliefStateUpdate {
            state: state.clone(),
        });
        let convergence_pct = state.convergence_pct();
        self.event_sink.emit(
            PlannerEvent::info(
                EventSource::SocraticEngine,
                "socratic.verify.complete",
                format!(
                    "Belief state updated: {:.0}% convergence ({} filled, {} uncertain, {} missing)",
                    convergence_pct * 100.0,
                    state.filled.len(),
                    state.uncertain.len(),
                    state.missing.len(),
                ),
            )
            .with_session(self.session_id)
            .with_metadata(serde_json::json!({
                "convergence_pct": convergence_pct,
                "filled_count": state.filled.len(),
                "uncertain_count": state.uncertain.len(),
                "missing_count": state.missing.len(),
            })),
        );
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
        self.event_sink.emit(
            PlannerEvent::info(
                EventSource::SocraticEngine,
                "socratic.converged",
                format!(
                    "Socratic interview converged at {:.0}% (reason: {:?})",
                    result.convergence_pct * 100.0,
                    result.reason,
                ),
            )
            .with_session(self.session_id)
            .with_metadata(serde_json::json!({
                "convergence_pct": result.convergence_pct,
                "reason": format!("{:?}", result.reason),
            })),
        );
    }

    async fn send_classification(&self, classification: &planner_schemas::DomainClassification) {
        let _ = self.event_tx.send(SocraticEvent::Classified {
            classification: classification.clone(),
        });
        self.event_sink.emit(
            PlannerEvent::info(
                EventSource::SocraticEngine,
                "socratic.classify.complete",
                format!(
                    "Domain classified: {} ({})",
                    classification.project_type,
                    match classification.complexity {
                        planner_schemas::ComplexityTier::Light => "light",
                        planner_schemas::ComplexityTier::Standard => "standard",
                        planner_schemas::ComplexityTier::Deep => "deep",
                    },
                ),
            )
            .with_session(self.session_id)
            .with_metadata(serde_json::json!({
                "project_type": classification.project_type.to_string(),
                "complexity": match classification.complexity {
                    planner_schemas::ComplexityTier::Light => "light",
                    planner_schemas::ComplexityTier::Standard => "standard",
                    planner_schemas::ComplexityTier::Deep => "deep",
                },
            })),
        );
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
/// Returns a tuple `(user_tx, events_rx, planner_events_rx)`:
/// - `user_tx` — send user replies into the engine.
/// - `events_rx` — receive `SocraticEvent`s from the engine.
/// - `planner_events_rx` — receive structured `PlannerEvent`s for observability.
///
/// The caller (main.rs) stores all three in `App`:
/// ```rust,ignore
/// let (tx, rx, prx) = pipeline::spawn_socratic_interview(initial_message);
/// app.socratic_tx = Some(tx);
/// app.socratic_events_rx = Some(rx);
/// app.planner_events_rx = Some(prx);
/// ```
pub fn spawn_socratic_interview(
    initial_message: String,
) -> (
    mpsc::UnboundedSender<String>,
    mpsc::UnboundedReceiver<SocraticEvent>,
    mpsc::UnboundedReceiver<PlannerEvent>,
) {
    // user_tx → user_rx: TUI feeds user replies in; engine reads them out
    let (user_tx, user_rx) = mpsc::unbounded_channel::<String>();
    // events_tx → events_rx: engine pushes events in; TUI drains them out
    let (events_tx, events_rx) = mpsc::unbounded_channel::<SocraticEvent>();
    // planner event sink for observability
    let (event_sink, planner_events_rx) = ChannelEventSink::new();
    let event_sink: Arc<dyn EventSink> = Arc::new(event_sink);

    let session_id = uuid::Uuid::new_v4();

    let io = Arc::new(TuiSocraticIO {
        event_tx: events_tx.clone(),
        input_rx: Arc::new(Mutex::new(user_rx)),
        event_sink: event_sink.clone(),
        session_id,
    });

    // Emit session start event
    event_sink.emit(
        PlannerEvent::info(
            EventSource::System,
            "system.session.start",
            format!("TUI session {} starting Socratic interview", session_id),
        )
        .with_session(session_id),
    );

    tokio::spawn(async move {
        let router = planner_core::llm::providers::LlmRouter::from_env();

        // Open durable CXDB for persisting Socratic turns.
        let cxdb = open_tui_cxdb();
        let result = match &cxdb {
            Some(engine) => {
                planner_core::pipeline::steps::socratic::run_interview::<
                    TuiSocraticIO,
                    planner_core::cxdb::durable::DurableCxdbEngine,
                >(&router, &*io, Some(engine), &initial_message)
                .await
            }
            None => {
                planner_core::pipeline::steps::socratic::run_interview::<
                    TuiSocraticIO,
                    planner_core::cxdb::CxdbEngine,
                >(
                    &router,
                    &*io,
                    None::<&planner_core::cxdb::CxdbEngine>,
                    &initial_message,
                )
                .await
            }
        };

        match result {
            Ok(_session) => {
                // Convergence was already signalled via send_convergence() /
                // the Converged event. Nothing more to send here.
            }
            Err(e) => {
                let _ = events_tx.send(SocraticEvent::Error {
                    message: format!("{}", e),
                });
                event_sink.emit(
                    PlannerEvent::error(
                        EventSource::SocraticEngine,
                        "socratic.error",
                        format!("Socratic interview failed: {}", e),
                    )
                    .with_session(session_id),
                );
            }
        }
    });

    (user_tx, events_rx, planner_events_rx)
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
pub fn spawn_pipeline(
    description: String,
    blueprints: Option<Arc<planner_core::blueprint::BlueprintStore>>,
) -> PipelineReceiver {
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

        let project_id = uuid::Uuid::new_v4();

        // Open durable CXDB for persisting pipeline turns.
        let cxdb = open_tui_cxdb();

        let pipeline_result = match &cxdb {
            Some(engine) => {
                let run_id = uuid::Uuid::new_v4();
                if let Err(e) = engine.register_run(project_id, run_id) {
                    tracing::warn!("CXDB: failed to register run: {}", e);
                }
                let config = planner_core::pipeline::PipelineConfig {
                    router: &router,
                    store: Some(engine),
                    dtu_registry: None,
                    blueprints: blueprints.as_deref(),
                };
                planner_core::pipeline::run_full_pipeline(
                    &config,
                    &worker,
                    project_id,
                    &description,
                )
                .await
            }
            None => {
                let config =
                    planner_core::pipeline::PipelineConfig::<planner_core::cxdb::CxdbEngine> {
                        router: &router,
                        store: None,
                        dtu_registry: None,
                        blueprints: blueprints.as_deref(),
                    };
                planner_core::pipeline::run_full_pipeline(
                    &config,
                    &worker,
                    project_id,
                    &description,
                )
                .await
            }
        };

        match pipeline_result {
            Ok(output) => {
                // Emit StepComplete events for each stage so the TUI progress
                // tracker advances stage-by-stage on success.
                for stage in [
                    "Intake",
                    "Chunk",
                    "Compile",
                    "Lint",
                    "AR Review",
                    "Refine",
                    "Scenarios",
                    "Ralph",
                    "Graph",
                    "Factory",
                    "Validate",
                    "Git",
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

// ---------------------------------------------------------------------------
// Shared CXDB helper
// ---------------------------------------------------------------------------

/// Open a DurableCxdbEngine for TUI sessions.
///
/// Reads `PLANNER_DATA_DIR` (falling back to `~/.planner/`) and opens
/// `<data_dir>/cxdb/`. Returns `None` on any failure so callers can
/// gracefully degrade to in-memory operation.
fn open_tui_cxdb() -> Option<planner_core::cxdb::durable::DurableCxdbEngine> {
    let data_dir = std::env::var("PLANNER_DATA_DIR").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{}/.planner", h))
            .unwrap_or_else(|_| "./data".to_string())
    });
    let cxdb_path = std::path::Path::new(&data_dir).join("cxdb");
    match planner_core::cxdb::durable::DurableCxdbEngine::open(&cxdb_path) {
        Ok(engine) => {
            tracing::info!("TUI CXDB persistence: {}", cxdb_path.display());
            Some(engine)
        }
        Err(e) => {
            tracing::warn!("TUI CXDB unavailable ({}), running without persistence", e);
            None
        }
    }
}
