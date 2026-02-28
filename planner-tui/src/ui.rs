//! # UI Rendering — Ratatui Layout + Widgets
//!
//! Renders the TUI layout using Ratatui.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::app::{App, FocusMode, MessageRole, StageStatus};

/// Draw the complete TUI interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: Header | Chat | Pipeline Status | Input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(8),      // Chat history
            Constraint::Length(3),   // Pipeline status bar
            Constraint::Length(3),   // Input
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_chat(frame, chunks[1], app);
    draw_pipeline_status(frame, chunks[2], app);
    draw_input(frame, chunks[3], app);
}

/// Draw the header bar.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let header_text = format!(
        " Planner v2 — Socratic Planning Session  |  Project: {}  |  {}",
        &app.project_id.to_string()[..8],
        app.session_start,
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

/// Draw the chat history panel.
fn draw_chat(frame: &mut Frame, area: Rect, app: &App) {
    let chat_block = Block::default()
        .borders(Borders::ALL)
        .title(" Chat ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(if app.focus == FocusMode::ChatScroll {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = chat_block.inner(area);

    // Build chat lines with styled spans
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        // Timestamp + role prefix
        let (role_color, prefix) = match msg.role {
            MessageRole::System => (Color::DarkGray, format!("[{}] ", msg.role.label())),
            MessageRole::User => (Color::Green, format!("[{}] ", msg.role.label())),
            MessageRole::Planner => (Color::Cyan, format!("[{}] ", msg.role.label())),
        };

        let time_span = Span::styled(
            format!("{} ", msg.timestamp),
            Style::default().fg(Color::DarkGray),
        );
        let role_span = Span::styled(
            prefix,
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        );

        // Split content by newlines
        let content_lines: Vec<&str> = msg.content.split('\n').collect();

        if let Some(first) = content_lines.first() {
            lines.push(Line::from(vec![
                time_span,
                role_span,
                Span::raw(*first),
            ]));
        }

        // Continuation lines (indented)
        for line in content_lines.iter().skip(1) {
            lines.push(Line::from(vec![
                Span::raw("       "),
                Span::raw(*line),
            ]));
        }

        // Blank line between messages
        lines.push(Line::from(""));
    }

    let chat_paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(chat_block, area);
    frame.render_widget(chat_paragraph, inner);

    // Scrollbar
    if app.messages.len() > 5 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let mut scrollbar_state = ScrollbarState::new(app.messages.len() * 3)
            .position(app.scroll_offset as usize);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Draw the pipeline status bar.
fn draw_pipeline_status(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = vec![Span::raw(" Pipeline: ")];

    for (i, stage) in app.stages.iter().enumerate() {
        let (symbol, color) = match stage.status {
            StageStatus::Pending => ("□", Color::DarkGray),
            StageStatus::Running => ("◆", Color::Yellow),
            StageStatus::Complete => ("■", Color::Green),
            StageStatus::Failed => ("✗", Color::Red),
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
    let status = Paragraph::new(status_line)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));

    frame.render_widget(status, area);
}

/// Draw the input area.
fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let input_style = if app.focus == FocusMode::Input {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", app.status_message))
        .title_style(Style::default().fg(Color::DarkGray))
        .border_style(if app.focus == FocusMode::Input {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let input = Paragraph::new(format!("> {}", app.input))
        .style(input_style)
        .block(input_block);

    frame.render_widget(input, area);

    // Set cursor position
    if app.focus == FocusMode::Input {
        frame.set_cursor_position(Position::new(
            area.x + app.cursor_position as u16 + 3, // +3 for border + "> "
            area.y + 1, // +1 for border
        ));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn draw_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new();

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_with_messages_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        app.add_user_message("Build me a task tracker widget");
        app.add_planner_message("Let me ask some clarifying questions...");
        app.add_user_message("It should support due dates and priorities");
        app.add_planner_message("Great. What about categories or tags?");

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_with_pipeline_progress() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        app.set_stage_status(0, StageStatus::Complete);
        app.set_stage_status(1, StageStatus::Complete);
        app.set_stage_status(2, StageStatus::Running);

        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_small_terminal() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new();

        terminal.draw(|f| draw(f, &app)).unwrap();
    }
}
