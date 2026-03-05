//! # UI Rendering — Ratatui Layout + Widgets
//!
//! Renders the TUI layout using Ratatui.
//!
//! ## Layout modes
//!
//! ### Interviewing (split-pane)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Planner v2 — Socratic Planning Session  |  Project: ...   │ ← Header (3)
//! ├───────────────────────────┬─────────────────────────────────┤
//! │                           │  Domain: Web App (Standard)     │
//! │  [System] Welcome…        │  ▓▓▓▓▓▓▓░░░░░ 48%              │
//! │  [You] Build a tracker    │  ✓ Goal                         │
//! │  [Planner] What's the…    │  ✓ Core Features                │
//! │                           │  ? Stakeholders (guessing…)     │
//! │                           │  ○ Auth                         │
//! ├───────────────────────────┴─────────────────────────────────┤
//! │  [Tab] Pane  [Esc] Skip  [Ctrl+D] Done  [1-9] Quick pick   │ ← Status
//! │  > Type your answer…                                        │ ← Input
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Pipeline running (full-width, same as original)
//! ```text
//! Header | Chat (full width) | Pipeline Status | Input
//! ```

use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};

use planner_schemas::ComplexityTier;

use crate::app::{App, FocusMode, IntakePhase, MessageRole, RightPaneMode, StageStatus};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Draw the complete TUI interface. Layout switches on `intake_phase`.
pub fn draw(frame: &mut Frame, app: &App) {
    match app.intake_phase {
        IntakePhase::WaitingForInput => draw_waiting_for_input(frame, app),
        IntakePhase::Interviewing => draw_interviewing(frame, app),
        _ => draw_pipeline(frame, app),
    }
}

// ---------------------------------------------------------------------------
// WaitingForInput layout — provider check screen
// ---------------------------------------------------------------------------

fn draw_waiting_for_input(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Vertical slices: Header | Provider panel | Input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(8),    // Provider status panel
            Constraint::Length(3), // Input
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_provider_status(frame, chunks[1], app);
    draw_input(frame, chunks[2], app);
}

/// Provider check panel shown during WaitingForInput.
fn draw_provider_status(frame: &mut Frame, area: Rect, app: &App) {
    let available_count = app.providers.iter().filter(|p| p.available).count();
    let total_count = app.providers.len();
    let all_missing = available_count == 0;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PLANNER v2 — Socratic Planning Engine ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Inner layout: top padding | "LLM Providers:" heading | provider rows |
    //               blank | summary line | blank | warning (if any) | prompt
    //
    // We build all content as a Paragraph with styled Lines rather than
    // a dynamic Layout so it degrades gracefully on small terminals.
    let mut lines: Vec<Line> = Vec::new();

    // Blank line for visual breathing room
    lines.push(Line::from(""));

    // Heading
    lines.push(Line::from(Span::styled(
        "  LLM Providers:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));

    // One row per provider
    for p in &app.providers {
        let (icon, icon_style, name_style) = if p.available {
            (
                "✓",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Green),
            )
        } else {
            (
                "✗",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Red),
            )
        };

        let detail = if p.available {
            format!("({} found)", p.binary)
        } else {
            format!("({} not found)", p.binary)
        };
        let detail_style = if p.available {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(icon, icon_style),
            Span::raw(" "),
            Span::styled(format!("{:<10}", p.name), name_style),
            Span::raw(" "),
            Span::styled(detail, detail_style),
        ]));
    }

    // Blank line
    lines.push(Line::from(""));

    // Summary line
    let summary_text = format!("  {} of {} providers available", available_count, total_count);
    let summary_style = if all_missing {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if available_count < total_count {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    lines.push(Line::from(Span::styled(summary_text, summary_style)));

    // Warning when no providers are available
    if all_missing {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ⚠  No LLM providers found. Install claude, gemini, or codex into /opt/planner/bin/ via the install script.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    // Blank line before prompt hint
    lines.push(Line::from(""));

    // Prompt hint
    lines.push(Line::from(Span::styled(
        "  Describe what you want to build below to start.",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

// ---------------------------------------------------------------------------
// Interviewing layout — split pane
// ---------------------------------------------------------------------------

fn draw_interviewing(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Vertical slices: Header | Body | StatusBar | Input
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(6),    // Body (chat + belief pane)
            Constraint::Length(1), // Keybind status bar
            Constraint::Length(3), // Input
        ])
        .split(area);

    draw_header(frame, rows[0], app);

    // Horizontal split: Chat (50%) | Right Pane (50%)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(rows[1]);

    draw_chat(frame, columns[0], app);
    match app.right_pane_mode {
        RightPaneMode::BeliefState => draw_belief_state(frame, columns[1], app),
        RightPaneMode::Logs => draw_logs_panel(frame, columns[1], app),
    }

    draw_status_bar(frame, rows[2], app);
    draw_input(frame, rows[3], app);
}

// ---------------------------------------------------------------------------
// Pipeline layout — full-width chat
// ---------------------------------------------------------------------------

fn draw_pipeline(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(8),    // Chat history
            Constraint::Length(3), // Pipeline status bar
            Constraint::Length(3), // Input
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_chat(frame, chunks[1], app);
    draw_pipeline_status(frame, chunks[2], app);
    draw_input(frame, chunks[3], app);
}

// ---------------------------------------------------------------------------
// Shared widgets
// ---------------------------------------------------------------------------

/// Header bar — project ID, session time, current phase badge.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let phase_badge = match app.intake_phase {
        IntakePhase::WaitingForInput => "[ Intake ]",
        IntakePhase::Interviewing   => "[ Interview ]",
        IntakePhase::PipelineRunning => "[ Pipeline ]",
        IntakePhase::Complete       => "[ Complete ]",
    };

    let header_text = format!(
        " Planner v2 — Socratic Planning Session  |  Project: {}  |  {}  |  {}",
        &app.project_id.to_string()[..8],
        app.session_start,
        phase_badge,
    );

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

/// Chat history panel — rendered in both layout modes.
fn draw_chat(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == FocusMode::ChatScroll;

    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title(" Chat ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = chat_block.inner(area);

    // Build lines
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (role_color, prefix) = match msg.role {
            MessageRole::System  => (Color::DarkGray, format!("[{}] ", msg.role.label())),
            MessageRole::User    => (Color::Green,    format!("[{}] ", msg.role.label())),
            MessageRole::Planner => (Color::Cyan,     format!("[{}] ", msg.role.label())),
        };

        let time_span = Span::styled(
            format!("{} ", msg.timestamp),
            Style::default().fg(Color::DarkGray),
        );
        let role_span = Span::styled(
            prefix,
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        );

        let content_lines: Vec<&str> = msg.content.split('\n').collect();

        if let Some(first) = content_lines.first() {
            lines.push(Line::from(vec![time_span, role_span, Span::raw(*first)]));
        }
        for line in content_lines.iter().skip(1) {
            lines.push(Line::from(vec![
                Span::raw("       "),
                Span::raw(*line),
            ]));
        }

        // Blank line between messages
        lines.push(Line::from(""));
    }

    // Render current question quick-options below the chat if Interviewing
    if app.intake_phase == IntakePhase::Interviewing {
        if let Some(ref q) = app.current_question {
            if !q.quick_options.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  Quick options:",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                let mut option_spans: Vec<Span> = Vec::new();
                for (i, opt) in q.quick_options.iter().enumerate() {
                    option_spans.push(Span::styled(
                        format!("[{}] {} ", i + 1, opt.label),
                        Style::default().fg(Color::White),
                    ));
                }
                if q.allow_skip {
                    option_spans.push(Span::styled(
                        "[Esc] Skip",
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                lines.push(Line::from(option_spans));
                lines.push(Line::from(""));
            }
        }
    }

    let chat_paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(chat_block, area);
    frame.render_widget(chat_paragraph, inner);

    if app.messages.len() > 5 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(app.messages.len() * 3)
            .position(app.scroll_offset as usize);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Belief-state panel — right pane during Interviewing.
fn draw_belief_state(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == FocusMode::BeliefStatePane;

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" Requirements ")
        .title_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // Further split inner: domain badge + gauge | dimension list
    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Domain badge
            Constraint::Length(2), // Convergence gauge
            Constraint::Min(3),    // Dimension lists
        ])
        .split(inner);

    // --- Domain badge ---
    let domain_text = if let Some(ref cls) = app.classification {
        format!(
            " {} | {} ",
            cls.project_type,
            match cls.complexity {
                ComplexityTier::Light    => "Light",
                ComplexityTier::Standard => "Standard",
                ComplexityTier::Deep     => "Deep",
            }
        )
    } else {
        " Classifying…".to_string()
    };

    let domain_badge = Paragraph::new(domain_text)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(domain_badge, inner_rows[0]);

    // --- Convergence gauge ---
    let pct_u16 = (app.convergence_pct * 100.0).round() as u16;
    let gauge_label = format!(" Convergence  {}%", pct_u16);

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(convergence_color(app.convergence_pct))
                .bg(Color::DarkGray),
        )
        .percent(pct_u16)
        .label(gauge_label);
    frame.render_widget(gauge, inner_rows[1]);

    // --- Dimension list ---
    let mut items: Vec<ListItem> = Vec::new();

    if let Some(ref bs) = app.belief_state {
        // Filled dimensions (green ✓)
        if !bs.filled.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Filled:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))));
            for (dim, val) in &bs.filled {
                let label = dim.label();
                // Truncate value to fit pane
                let truncated = if val.value.len() > 30 {
                    format!("{}…", &val.value[..28])
                } else {
                    val.value.clone()
                };
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ✓ ", Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{}: ", label),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(truncated, Style::default().fg(Color::Gray)),
                ])));
            }
        }

        // Uncertain dimensions (yellow ?)
        if !bs.uncertain.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Uncertain:",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))));
            for (dim, (val, conf)) in &bs.uncertain {
                let label = dim.label();
                let truncated = if val.value.len() > 22 {
                    format!("{}…", &val.value[..20])
                } else {
                    val.value.clone()
                };
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ? ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}: ", label),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(truncated, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!(" ({:.0}%)", conf * 100.0),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])));
            }
        }

        // Missing dimensions (gray ○)
        if !bs.missing.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Missing:",
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            ))));
            for dim in &bs.missing {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ○ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(dim.label(), Style::default().fg(Color::DarkGray)),
                ])));
            }
        }

        // Out of scope (muted ✗)
        if !bs.out_of_scope.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Out of scope:",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
            ))));
            for dim in &bs.out_of_scope {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ✗ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        dim.label(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ),
                ])));
            }
        }

        // Contradiction warnings (red !)
        let active_contradictions: Vec<_> = bs.contradictions.iter()
            .filter(|c| !c.resolved)
            .collect();
        if !active_contradictions.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "Contradictions:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))));
            for c in active_contradictions {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ! ", Style::default().fg(Color::Red)),
                    Span::styled(
                        format!("{} vs {}", c.dimension_a.label(), c.dimension_b.label()),
                        Style::default().fg(Color::Red),
                    ),
                ])));
            }
        }
    } else {
        items.push(ListItem::new(Line::from(Span::styled(
            "  Waiting for first response…",
            Style::default().fg(Color::DarkGray),
        ))));
    }

    let dim_list = List::new(items);
    frame.render_widget(dim_list, inner_rows[2]);
}

/// Enriched status bar — 1-line strip showing phase, step, LLM count, and hints.
fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    // [phase: INTERVIEWING]  [step: classify_domain 2.3s]  [llm: 3 calls]  [L: logs]
    let phase_label = match app.intake_phase {
        IntakePhase::WaitingForInput  => "WAITING",
        IntakePhase::Interviewing     => "INTERVIEWING",
        IntakePhase::PipelineRunning  => "PIPELINE",
        IntakePhase::Complete         => "COMPLETE",
    };

    let mut spans: Vec<Span> = Vec::new();

    // Phase segment
    spans.push(Span::styled(
        format!(" [phase: {}]", phase_label),
        Style::default().fg(Color::Cyan),
    ));
    spans.push(Span::styled("  ", Style::default()));

    // Step segment
    let step_text = if let Some(ref step) = app.current_step {
        let elapsed = app.current_step_started
            .map(|inst| inst.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        format!("[step: {} {:.1}s]", step, elapsed)
    } else {
        "[step: —]".to_string()
    };
    spans.push(Span::styled(
        step_text,
        Style::default().fg(Color::Yellow),
    ));
    spans.push(Span::styled("  ", Style::default()));

    // LLM call count segment
    spans.push(Span::styled(
        format!("[llm: {} calls]", app.llm_call_count),
        Style::default().fg(Color::Magenta),
    ));
    spans.push(Span::styled("  ", Style::default()));

    // Hint: toggle logs
    let logs_hint = match app.right_pane_mode {
        RightPaneMode::BeliefState => "[L: logs]",
        RightPaneMode::Logs        => "[L: belief]",
    };
    spans.push(Span::styled(
        logs_hint,
        Style::default().fg(Color::DarkGray),
    ));

    // Separator hints
    spans.push(Span::styled(
        "  [Tab] pane  [Esc] skip  [Ctrl+D] done  [1-9] quick",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(bar, area);
}

/// Logs panel — shown in the right pane when `right_pane_mode == Logs`.
fn draw_logs_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == FocusMode::LogsPane;

    // Title reflects active filter
    let filter_label = match app.logs_filter {
        None                                                  => "All",
        Some(planner_core::observability::EventLevel::Error) => "Errors",
        Some(planner_core::observability::EventLevel::Warn)  => "Warnings",
        Some(planner_core::observability::EventLevel::Info)  => "Info",
    };
    let title = format!(" Logs [{}] [f: filter] ", filter_label);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let events = app.filtered_events();

    if events.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  No events yet",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    let total = events.len() as u16;
    // Clamp scroll offset so it cannot exceed the list length
    let clamped_scroll = app.logs_scroll_offset.min(total.saturating_sub(1));

    // Build lines from filtered events
    let lines: Vec<Line> = events
        .iter()
        .map(|ev| {
            let (level_color, level_str) = match ev.level {
                planner_core::observability::EventLevel::Info  => (Color::DarkGray, "INFO "),
                planner_core::observability::EventLevel::Warn  => (Color::Yellow,   "WARN "),
                planner_core::observability::EventLevel::Error => (Color::Red,      "ERROR"),
            };

            let ts = ev.timestamp.format("%H:%M:%S").to_string();
            let source_str = format!("{}", ev.source);
            let step_str = ev.step.as_deref().unwrap_or("");

            // Truncate long messages to fit the pane
            let msg = if ev.message.len() > 60 {
                format!("{}…", &ev.message[..58])
            } else {
                ev.message.clone()
            };

            let body = if step_str.is_empty() {
                msg
            } else {
                format!("{}: {}", step_str, msg)
            };

            Line::from(vec![
                Span::styled(
                    format!("{} ", ts),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} ", level_str),
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<12} ", source_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(body, Style::default().fg(level_color)),
            ])
        })
        .collect();

    let logs_para = Paragraph::new(lines).scroll((clamped_scroll, 0));
    frame.render_widget(logs_para, inner);

    // Scrollbar when there are enough events to scroll
    if total > 4 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(total as usize)
            .position(clamped_scroll as usize);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Pipeline status bar — shown in pipeline layout only.
fn draw_pipeline_status(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = vec![Span::raw(" Pipeline: ")];

    for (i, stage) in app.stages.iter().enumerate() {
        let (symbol, color) = match stage.status {
            StageStatus::Pending  => ("□", Color::DarkGray),
            StageStatus::Running  => ("◆", Color::Yellow),
            StageStatus::Complete => ("■", Color::Green),
            StageStatus::Failed   => ("✗", Color::Red),
        };

        spans.push(Span::styled(
            format!("{} {}", symbol, stage.name),
            Style::default().fg(color),
        ));

        if i < app.stages.len() - 1 {
            spans.push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
        }
    }

    let status_line = Line::from(spans);
    let status = Paragraph::new(status_line).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(status, area);
}

/// Input area — shared by both layouts.
fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == FocusMode::Input;

    let input_style = if is_focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", app.status_message))
        .title_style(Style::default().fg(Color::DarkGray))
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let input = Paragraph::new(format!("> {}", app.input))
        .style(input_style)
        .block(input_block);

    frame.render_widget(input, area);

    // Cursor
    if is_focused {
        frame.set_cursor_position(Position::new(
            area.x + app.cursor_position as u16 + 3, // border (1) + "> " (2)
            area.y + 1,
        ));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn convergence_color(pct: f32) -> Color {
    if pct >= 0.8 {
        Color::Green
    } else if pct >= 0.5 {
        Color::Yellow
    } else {
        Color::Red
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn make_app() -> App {
        App::new()
    }

    #[test]
    fn draw_waiting_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = make_app();
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_waiting_with_all_providers_available_does_not_panic() {
        use crate::app::ProviderStatus;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        // Simulate all three providers available
        app.providers = vec![
            ProviderStatus { name: "anthropic".into(), binary: "claude".into(), available: true },
            ProviderStatus { name: "google".into(),    binary: "gemini".into(), available: true },
            ProviderStatus { name: "openai".into(),    binary: "codex".into(),  available: true },
        ];
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_waiting_with_no_providers_shows_warning_does_not_panic() {
        use crate::app::ProviderStatus;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        // Simulate no providers — should render the red warning line
        app.providers = vec![
            ProviderStatus { name: "anthropic".into(), binary: "claude".into(), available: false },
            ProviderStatus { name: "google".into(),    binary: "gemini".into(), available: false },
            ProviderStatus { name: "openai".into(),    binary: "codex".into(),  available: false },
        ];
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;
        app.add_user_message("Build me a task tracker widget");
        app.add_planner_message("Let me ask some clarifying questions…");
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_with_belief_state_does_not_panic() {
        use planner_schemas::{DomainClassification, ProjectType, ComplexityTier, Dimension,
                              RequirementsBeliefState, SlotValue};

        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;
        app.convergence_pct = 0.4;

        let cls = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };
        app.classification = Some(cls.clone());

        let mut bs = RequirementsBeliefState::from_classification(&cls);
        bs.fill(Dimension::Goal, SlotValue {
            value: "Task tracker for team".into(),
            source_turn: 1,
            source_quote: None,
        });
        bs.mark_uncertain(
            Dimension::Stakeholders,
            SlotValue { value: "dev team".into(), source_turn: 1, source_quote: None },
            0.6,
        );
        app.belief_state = Some(bs);

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_with_question_options_does_not_panic() {
        use planner_schemas::{QuestionOutput, Dimension, QuickOption};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;

        app.current_question = Some(QuestionOutput {
            question: "What is the primary goal?".into(),
            target_dimension: Dimension::Goal,
            quick_options: vec![
                QuickOption { label: "A".into(), value: "Option A".into() },
                QuickOption { label: "B".into(), value: "Option B".into() },
                QuickOption { label: "Not sure".into(), value: "unsure".into() },
            ],
            allow_skip: true,
        });

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_pipeline_running_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::PipelineRunning;
        app.pipeline_running = true;
        app.set_stage_status(0, StageStatus::Complete);
        app.set_stage_status(1, StageStatus::Complete);
        app.set_stage_status(2, StageStatus::Running);
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_complete_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Complete;
        for stage in &mut app.stages {
            stage.status = StageStatus::Complete;
        }
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_logs_panel_empty_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;
        app.right_pane_mode = RightPaneMode::Logs;
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_logs_panel_with_events_does_not_panic() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;
        app.right_pane_mode = RightPaneMode::Logs;

        app.record_planner_event(PlannerEvent::info(
            EventSource::Pipeline,
            "compile",
            "Compiling NLSpec",
        ));
        app.record_planner_event(PlannerEvent::warn(
            EventSource::SocraticEngine,
            "interview",
            "Low confidence",
        ));
        app.record_planner_event(PlannerEvent::error(
            EventSource::LlmRouter,
            "llm.call.complete",
            "LLM timed out",
        ));

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_interviewing_logs_panel_filter_errors_does_not_panic() {
        use planner_core::observability::{EventLevel, EventSource, PlannerEvent};

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;
        app.right_pane_mode = RightPaneMode::Logs;
        app.logs_filter = Some(EventLevel::Error);

        app.record_planner_event(PlannerEvent::info(
            EventSource::Pipeline,
            "compile",
            "Compiling NLSpec",
        ));
        app.record_planner_event(PlannerEvent::error(
            EventSource::LlmRouter,
            "llm.call.complete",
            "Timeout",
        ));

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_small_terminal_does_not_panic() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = make_app();
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_with_messages_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.add_user_message("Build me a task tracker widget");
        app.add_planner_message("Let me ask some clarifying questions…");
        app.add_user_message("It should support due dates and priorities");
        app.add_planner_message("Great. What about categories or tags?");
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_belief_pane_with_out_of_scope_and_contradiction() {
        use planner_schemas::{
            DomainClassification, ProjectType, ComplexityTier, Dimension,
            RequirementsBeliefState, SlotValue, Contradiction,
        };

        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.intake_phase = IntakePhase::Interviewing;

        let cls = DomainClassification {
            project_type: ProjectType::ApiBackend,
            complexity: ComplexityTier::Deep,
            detected_signals: vec![],
            required_dimensions: Dimension::required_for(&ProjectType::ApiBackend),
        };
        app.classification = Some(cls.clone());

        let mut bs = RequirementsBeliefState::from_classification(&cls);
        bs.mark_out_of_scope(Dimension::Budget);
        bs.add_contradiction(Contradiction {
            dimension_a: Dimension::Performance,
            value_a: "latency < 10ms".into(),
            dimension_b: Dimension::Budget,
            value_b: "free tier only".into(),
            explanation: "Low latency and free tier are mutually exclusive at scale".into(),
            resolved: false,
        });
        app.belief_state = Some(bs);

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_with_pipeline_progress() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.set_stage_status(0, StageStatus::Complete);
        app.set_stage_status(1, StageStatus::Complete);
        app.set_stage_status(2, StageStatus::Running);
        terminal.draw(|f| draw(f, &app)).unwrap();
    }
}
