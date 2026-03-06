//! # Blueprint Table — TUI split-pane node browser (Phase H)
//!
//! ## Overview
//!
//! A dedicated `BlueprintView` mode in the TUI that renders the project's
//! blueprint nodes in a `ratatui::Table` with keyboard navigation.
//! The left pane shows the node list; the right pane shows full detail
//! of the currently selected node.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────┬───────────────────────────────┐
//! │  Node Table (left)               │  Node Detail (right)         │
//! │                                  │                              │
//! │  ID       Type       Name   Sta  │  Node: auth-gateway-abc123   │
//! │ ►auth…    technology Actix  adop  │  Type: technology            │
//! │  db-…     technology Postgres…    │  Ring: adopt                 │
//! │  api-…    component  API Gate…    │  Rationale: Fast async       │
//! │  …                               │  framework with…             │
//! │                                  │                              │
//! │  [j/k] nav  [Enter] detail       │  Edges: 3 upstream,          │
//! │  [/] search [q] quit             │         2 downstream         │
//! ├──────────────────────────────────┴───────────────────────────────┤
//! │  Status: 42 nodes · 67 edges · Viewing: technology              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Implementation Plan
//!
//! ### H.1 — App state extension (`app.rs`)
//!
//! Add to the existing `App` struct:
//!
//! ```rust
//! use planner_core::blueprint::{BlueprintStore, NodeSummary, EdgePayload};
//!
//! pub enum AppView {
//!     Socratic,    // current default
//!     Blueprint,   // new blueprint table view
//! }
//!
//! pub struct BlueprintTableState {
//!     /// All node summaries loaded from the store.
//!     pub nodes: Vec<NodeSummary>,
//!     /// All edges from the store.
//!     pub edges: Vec<EdgePayload>,
//!     /// Currently selected row index.
//!     pub selected: usize,
//!     /// Search/filter string (empty = show all).
//!     pub filter: String,
//!     /// Node type filter (None = all types).
//!     pub type_filter: Option<String>,
//!     /// ratatui TableState for scrolling.
//!     pub table_state: ratatui::widgets::TableState,
//!     /// Whether the detail pane is expanded.
//!     pub detail_expanded: bool,
//!     /// Full node data for the selected node (loaded on demand).
//!     pub detail_node: Option<planner_schemas::artifacts::blueprint::BlueprintNode>,
//! }
//! ```
//!
//! ### H.2 — Key bindings (`app.rs::handle_key`)
//!
//! When `app.view == AppView::Blueprint`:
//!
//! | Key       | Action                                    |
//! |-----------|-------------------------------------------|
//! | `j`/`↓`   | Move selection down                      |
//! | `k`/`↑`   | Move selection up                        |
//! | `Enter`   | Toggle detail pane for selected node     |
//! | `/`       | Enter search mode (filter by name/id)    |
//! | `Esc`     | Clear search / exit blueprint view       |
//! | `t`       | Cycle type filter (all → decision → …)   |
//! | `Tab`     | Toggle focus: table ↔ detail pane        |
//! | `q`       | Return to Socratic view                  |
//! | `g`/`Home`| Jump to top                              |
//! | `G`/`End` | Jump to bottom                           |
//!
//! ### H.3 — Render function (`ui.rs`)
//!
//! New function: `fn render_blueprint_table(f: &mut Frame, app: &App, area: Rect)`
//!
//! ```rust
//! fn render_blueprint_table(f: &mut Frame, app: &App, area: Rect) {
//!     use ratatui::prelude::*;
//!     use ratatui::widgets::{Block, Borders, Table, Row, Cell, Paragraph};
//!     
//!     // Split area: left (60%) for table, right (40%) for detail
//!     let chunks = Layout::default()
//!         .direction(Direction::Horizontal)
//!         .constraints([
//!             Constraint::Percentage(if app.blueprint.detail_expanded { 55 } else { 100 }),
//!             Constraint::Percentage(if app.blueprint.detail_expanded { 45 } else { 0 }),
//!         ])
//!         .split(area);
//!
//!     // --- Left: Node table ---
//!     let header = Row::new(["ID", "Type", "Name", "Status"])
//!         .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
//!     
//!     let rows = app.blueprint.filtered_nodes().iter().map(|node| {
//!         Row::new([
//!             Cell::from(truncate(&node.id, 12)),
//!             Cell::from(node.node_type.clone())
//!                 .style(type_color(&node.node_type)),
//!             Cell::from(truncate(&node.name, 30)),
//!             Cell::from(node.status.as_deref().unwrap_or("-")),
//!         ])
//!     });
//!
//!     let table = Table::new(rows, [
//!         Constraint::Length(14),   // ID
//!         Constraint::Length(12),   // Type
//!         Constraint::Min(20),     // Name
//!         Constraint::Length(10),  // Status
//!     ])
//!     .header(header)
//!     .block(Block::default().borders(Borders::ALL).title(" Blueprint Nodes "))
//!     .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
//!     .highlight_symbol("► ");
//!
//!     f.render_stateful_widget(table, chunks[0], &mut app.blueprint.table_state);
//!
//!     // --- Right: Node detail ---
//!     if app.blueprint.detail_expanded {
//!         render_node_detail(f, app, chunks[1]);
//!     }
//! }
//! ```
//!
//! ### H.4 — Node detail pane (`ui.rs`)
//!
//! ```rust
//! fn render_node_detail(f: &mut Frame, app: &App, area: Rect) {
//!     // Show: node ID, type, name, all type-specific fields,
//!     // upstream/downstream connections, tags, timestamps.
//!     // Use Paragraph with Line::from(vec![Span]) for key-value pairs.
//!     // Color type badges using the same palette as the web UI.
//! }
//! ```
//!
//! ### H.5 — Data loading
//!
//! When entering blueprint view, load from `BlueprintStore`:
//!
//! ```rust
//! impl App {
//!     pub fn load_blueprint(&mut self, store: &BlueprintStore) {
//!         let snapshot = store.snapshot();
//!         self.blueprint.nodes = snapshot.nodes.iter().map(|n| n.summary()).collect();
//!         self.blueprint.edges = snapshot.edges.clone();
//!         self.blueprint.selected = 0;
//!         self.blueprint.table_state.select(Some(0));
//!     }
//!     
//!     pub fn load_selected_detail(&mut self, store: &BlueprintStore) {
//!         if let Some(node_summary) = self.blueprint.filtered_nodes().get(self.blueprint.selected) {
//!             self.blueprint.detail_node = store.get_node(&node_summary.id).ok();
//!         }
//!     }
//! }
//! ```
//!
//! ### H.6 — View switching
//!
//! In `main.rs`, add a keybinding (e.g., `Ctrl+B`) to toggle between
//! `AppView::Socratic` and `AppView::Blueprint`. The `ui.rs` `draw()`
//! function routes to the appropriate renderer based on `app.view`.
//!
//! ### Dependencies
//!
//! Already in Cargo.toml: `ratatui`, `crossterm`. No new deps needed.
//!
//! ### Testing
//!
//! - Unit test: `BlueprintTableState::filtered_nodes()` with various filter strings
//! - Unit test: Navigation wraps correctly at boundaries
//! - Unit test: Type filter cycles through all 6 node types + "all"
//! - Integration test: Load from a test `BlueprintStore`, verify row count matches
