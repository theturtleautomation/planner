//! # App State — TUI Application Model
//!
//! Manages the state of the Socratic planning TUI session.

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Chat Message
// ---------------------------------------------------------------------------

/// A single message in the chat history.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

/// Who sent the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Planner,
}

impl MessageRole {
    pub fn label(&self) -> &str {
        match self {
            MessageRole::System => "System",
            MessageRole::User => "You",
            MessageRole::Planner => "Planner",
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline Stage Tracking
// ---------------------------------------------------------------------------

/// Pipeline stages with progress tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub status: StageStatus,
}

// ---------------------------------------------------------------------------
// App Focus Mode
// ---------------------------------------------------------------------------

/// Which panel has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusMode {
    /// User is typing in the input box.
    Input,
    /// User is scrolling through chat history.
    ChatScroll,
}

// ---------------------------------------------------------------------------
// App State
// ---------------------------------------------------------------------------

/// The main TUI application state.
pub struct App {
    /// Should the app exit?
    pub should_quit: bool,
    /// Current input buffer.
    pub input: String,
    /// Cursor position in the input buffer.
    pub cursor_position: usize,
    /// Chat message history.
    pub messages: Vec<ChatMessage>,
    /// Pipeline stages and their status.
    pub stages: Vec<PipelineStage>,
    /// Current focus mode.
    pub focus: FocusMode,
    /// Scroll offset for chat history.
    pub scroll_offset: u16,
    /// Project ID for this session.
    pub project_id: Uuid,
    /// Session start time.
    pub session_start: String,
    /// Whether the pipeline is actively running.
    pub pipeline_running: bool,
    /// Status message for the bottom bar.
    pub status_message: String,
}

impl App {
    pub fn new() -> Self {
        let now = Utc::now();

        let stages = vec![
            PipelineStage { name: "Intake".into(), status: StageStatus::Pending },
            PipelineStage { name: "Chunk".into(), status: StageStatus::Pending },
            PipelineStage { name: "Compile".into(), status: StageStatus::Pending },
            PipelineStage { name: "Lint".into(), status: StageStatus::Pending },
            PipelineStage { name: "AR Review".into(), status: StageStatus::Pending },
            PipelineStage { name: "Refine".into(), status: StageStatus::Pending },
            PipelineStage { name: "Scenarios".into(), status: StageStatus::Pending },
            PipelineStage { name: "Ralph".into(), status: StageStatus::Pending },
            PipelineStage { name: "Graph".into(), status: StageStatus::Pending },
            PipelineStage { name: "Factory".into(), status: StageStatus::Pending },
            PipelineStage { name: "Validate".into(), status: StageStatus::Pending },
            PipelineStage { name: "Git".into(), status: StageStatus::Pending },
        ];

        let mut app = App {
            should_quit: false,
            input: String::new(),
            cursor_position: 0,
            messages: Vec::new(),
            stages,
            focus: FocusMode::Input,
            scroll_offset: 0,
            project_id: Uuid::new_v4(),
            session_start: now.format("%Y-%m-%d %H:%M UTC").to_string(),
            pipeline_running: false,
            status_message: "Ready — describe what you want to build".into(),
        };

        // Welcome message
        app.add_system_message(
            "Welcome to Planner v2 — Socratic Planning Session\n\
             \n\
             Describe what you want to build and I'll guide you through\n\
             a structured requirements elicitation process.\n\
             \n\
             Press Enter to submit, Ctrl+C or Esc to quit."
        );

        app
    }

    /// Add a system message.
    pub fn add_system_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: content.to_string(),
            timestamp: Utc::now().format("%H:%M").to_string(),
        });
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Utc::now().format("%H:%M").to_string(),
        });
    }

    /// Add a planner response.
    pub fn add_planner_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            role: MessageRole::Planner,
            content: content.to_string(),
            timestamp: Utc::now().format("%H:%M").to_string(),
        });
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.focus {
            FocusMode::Input => self.handle_input_key(key),
            FocusMode::ChatScroll => self.handle_scroll_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                if self.input.is_empty() {
                    self.should_quit = true;
                } else {
                    self.input.clear();
                    self.cursor_position = 0;
                }
            }
            KeyCode::Enter => {
                self.submit_input();
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
            }
            KeyCode::Up => {
                self.focus = FocusMode::ChatScroll;
            }
            _ => {}
        }
    }

    fn handle_scroll_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Esc | KeyCode::Down => {
                if self.scroll_offset == 0 {
                    self.focus = FocusMode::Input;
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Up => {
                self.scroll_offset += 1;
            }
            KeyCode::PageUp => {
                self.scroll_offset += 10;
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            _ => {
                self.focus = FocusMode::Input;
            }
        }
    }

    /// Submit the current input.
    fn submit_input(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        self.add_user_message(&text);
        self.input.clear();
        self.cursor_position = 0;
        self.scroll_offset = 0;

        // Process the user's input
        if !self.pipeline_running {
            // First message starts the pipeline
            self.pipeline_running = true;
            self.stages[0].status = StageStatus::Running;
            self.status_message = "Pipeline starting — Intake Gateway...".into();

            self.add_planner_message(&format!(
                "Starting Socratic planning for: \"{}\"\n\n\
                 Let me analyze your request and ask some clarifying questions.\n\
                 The pipeline will run through {} stages.\n\n\
                 [Pipeline execution would happen here in a real run.\n\
                  LLM calls require claude/gemini/codex CLI tools installed.]",
                text,
                self.stages.len()
            ));

            // Simulate stage progression for demo
            self.stages[0].status = StageStatus::Complete;
            self.stages[1].status = StageStatus::Running;
        } else {
            // Subsequent messages are part of the Socratic dialogue
            self.add_planner_message(
                "Thank you for that clarification. Let me incorporate that into \
                 the specification.\n\n\
                 [In a live session, this would trigger the Compiler to update \
                  the NLSpec with your additional context.]"
            );
        }
    }

    /// Update a pipeline stage status.
    pub fn set_stage_status(&mut self, index: usize, status: StageStatus) {
        if index < self.stages.len() {
            self.stages[index].status = status;
        }
    }

    /// Periodic tick handler (for async operations, animations, etc.)
    pub fn tick(&mut self) {
        // Future: check for async pipeline results here
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_starts_with_welcome_message() {
        let app = App::new();
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
        assert!(app.messages[0].content.contains("Welcome"));
    }

    #[test]
    fn app_input_handling() {
        let mut app = App::new();

        // Type "hello"
        for c in "hello".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.input, "hello");
        assert_eq!(app.cursor_position, 5);

        // Backspace
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.input, "hell");
        assert_eq!(app.cursor_position, 4);

        // Left arrow
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 3);

        // Home
        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 0);

        // End
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 4);
    }

    #[test]
    fn app_submit_clears_input() {
        let mut app = App::new();

        for c in "Build me a widget".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.input.is_empty());
        assert_eq!(app.cursor_position, 0);
        // Should have welcome + user + planner = 3 messages
        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.messages[1].role, MessageRole::User);
        assert_eq!(app.messages[2].role, MessageRole::Planner);
    }

    #[test]
    fn app_empty_submit_ignored() {
        let mut app = App::new();
        let msg_count = app.messages.len();

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.messages.len(), msg_count); // No new message
    }

    #[test]
    fn app_esc_clears_or_quits() {
        let mut app = App::new();

        // Type something then Esc → clears input
        for c in "test".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.input.is_empty());
        assert!(!app.should_quit);

        // Esc on empty input → quit
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn app_ctrl_c_quits() {
        let mut app = App::new();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn app_stages_initialized() {
        let app = App::new();
        assert_eq!(app.stages.len(), 12);
        assert!(app.stages.iter().all(|s| s.status == StageStatus::Pending));
    }

    #[test]
    fn app_set_stage_status() {
        let mut app = App::new();
        app.set_stage_status(0, StageStatus::Running);
        assert_eq!(app.stages[0].status, StageStatus::Running);

        app.set_stage_status(0, StageStatus::Complete);
        assert_eq!(app.stages[0].status, StageStatus::Complete);
    }

    #[test]
    fn app_scroll_mode_toggle() {
        let mut app = App::new();
        assert_eq!(app.focus, FocusMode::Input);

        // Up arrow → scroll mode
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::ChatScroll);

        // Down when at 0 → back to input
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::Input);
    }

    #[test]
    fn message_role_labels() {
        assert_eq!(MessageRole::System.label(), "System");
        assert_eq!(MessageRole::User.label(), "You");
        assert_eq!(MessageRole::Planner.label(), "Planner");
    }

    #[test]
    fn pipeline_running_flag() {
        let mut app = App::new();
        assert!(!app.pipeline_running);

        // Submit first message → pipeline starts
        for c in "Build a widget".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.pipeline_running);
    }
}
