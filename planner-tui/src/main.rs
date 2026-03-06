//! # Planner TUI — Ratatui Socratic Interview
//!
//! Terminal-based interface for conducting Socratic planning sessions.
//! Uses Ratatui for rendering and Crossterm for input handling.
//!
//! ## Layout (Interviewing phase)
//!   ┌──────────────────────────────────────────────────────────┐
//!   │  Planner v2 — Socratic Planning Session | Project: …    │ ← Header
//!   ├──────────────────────────┬───────────────────────────────┤
//!   │                          │  Domain: Web App (Standard)   │
//!   │  [System] Welcome…       │  ▓▓▓▓░░░░ 32%                │
//!   │  [You] Build a tracker   │  ✓ Goal                       │
//!   │  [Planner] What's the…   │  ? Stakeholders               │
//!   │                          │  ○ Auth                       │
//!   ├──────────────────────────┴───────────────────────────────┤
//!   │  [Tab] Pane  [Esc] Skip  [Ctrl+D] Done  [1-9] Quick pick│ ← Keybinds
//!   │  > Type your answer…                                     │ ← Input
//!   └──────────────────────────────────────────────────────────┘

mod app;
mod blueprint_table;
mod ui;
mod events;
mod pipeline;

use std::io;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::backend::CrosstermBackend;

use app::App;
use events::EventHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("planner_tui=info".parse().unwrap()),
        )
        .with_writer(io::stderr)
        .init();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        eprintln!("Usage: planner-tui [--project-id <uuid>]");
        eprintln!();
        eprintln!("Socratic planning session in the terminal.");
        eprintln!("Interactive TUI for conducting requirement elicitation,");
        eprintln!("spec compilation, and factory execution.");
        std::process::exit(0);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    let event_handler = EventHandler::new(250); // 250ms tick rate
    let res = run_app(&mut terminal, &mut app, &event_handler).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &EventHandler,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        match events.next().await {
            events::Event::Tick => {
                // Drain pipeline events
                app.tick();
                // Drain Socratic events
                app.tick_socratic();
                // Drain planner observability events
                app.tick_planner_events();
            }
            events::Event::Key(key) => {
                app.handle_key(key);
            }
            events::Event::Resize(_, _) => {}
        }

        // ── Socratic interview spawn ────────────────────────────────────────
        // The user submitted their first message → spawn the Socratic engine.
        if let Some(initial_message) = app.take_pending_socratic() {
            let (user_tx, events_rx, planner_events_rx) = pipeline::spawn_socratic_interview(initial_message);
            app.socratic_tx = Some(user_tx);
            app.socratic_events_rx = Some(events_rx);
            app.planner_events_rx = Some(planner_events_rx);
        }

        // ── Pipeline spawn ──────────────────────────────────────────────────
        // Interview converged → spawn the full planning pipeline.
        if let Some(description) = app.take_pending_pipeline() {
            let rx = pipeline::spawn_pipeline(description, Some(app.blueprint_store.clone()));
            app.pipeline_rx = Some(rx);
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
