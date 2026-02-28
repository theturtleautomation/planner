//! # Planner TUI — Ratatui Socratic Interview
//!
//! Terminal-based interface for conducting Socratic planning sessions.
//! Uses Ratatui for rendering and Crossterm for input handling.
//!
//! Layout:
//!   ┌─────────────────────────────────────────────┐
//!   │  Planner v2 — Socratic Planning Session     │ ← Header
//!   ├─────────────────────────────────────────────┤
//!   │                                             │
//!   │  [System] Welcome to Planner v2...          │ ← Chat history
//!   │  [You] Build me a task tracker              │
//!   │  [Planner] Let me ask some questions...     │
//!   │                                             │
//!   ├─────────────────────────────────────────────┤
//!   │  Pipeline: Intake ■ Compile □ ...           │ ← Status bar
//!   ├─────────────────────────────────────────────┤
//!   │  > Type your response...                    │ ← Input
//!   └─────────────────────────────────────────────┘

mod app;
mod ui;
mod events;

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
                app.tick();
            }
            events::Event::Key(key) => {
                app.handle_key(key);
            }
            events::Event::Resize(_, _) => {}
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
