//! # Pipeline Bridge — TUI ↔ Background Pipeline Task
//!
//! This module defines the `PipelineEvent` enum (sent from the background
//! pipeline task to the TUI) and the `spawn_pipeline` helper that main.rs
//! calls when the user submits their first message.
//!
//! Design: `App` is not `Send` (Ratatui types aren't), so we never put the
//! pipeline task inside `App`. Instead:
//!   1. `App::submit_input()` stores the description in `pending_pipeline_description`.
//!   2. The main loop calls `App::take_pending_pipeline()`.
//!   3. If `Some(description)` comes back, main.rs calls `spawn_pipeline()`.
//!   4. The spawned task sends `PipelineEvent`s through an unbounded channel.
//!   5. `App::tick()` drains the channel and updates state.

use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Pipeline Event
// ---------------------------------------------------------------------------

/// Events emitted by the background pipeline task to the TUI.
#[derive(Debug)]
pub enum PipelineEvent {
    /// Pipeline task started (fires immediately after spawn).
    Started,
    /// Pipeline completed successfully — carries a summary string.
    Completed(String),
    /// Pipeline failed — carries the error message.
    Failed(String),
}

// ---------------------------------------------------------------------------
// Channel alias
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub type PipelineSender = mpsc::UnboundedSender<PipelineEvent>;
pub type PipelineReceiver = mpsc::UnboundedReceiver<PipelineEvent>;

// ---------------------------------------------------------------------------
// Spawner
// ---------------------------------------------------------------------------

/// Spawn the full pipeline in a background tokio task.
///
/// Returns the receiver end of the channel. The caller should store it in
/// `App::pipeline_rx`. Events arrive on the next `tick()` poll.
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
