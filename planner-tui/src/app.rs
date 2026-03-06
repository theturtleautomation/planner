//! # App State — TUI Application Model
//!
//! Manages the state of the Socratic planning TUI session.
//!
//! ## Phase lifecycle
//!
//! ```text
//! WaitingForInput  ─► Interviewing  ─► PipelineRunning  ─► Complete
//! ```
//!
//! - **WaitingForInput** — initial state, waiting for the user's first message.
//! - **Interviewing** — Socratic interview loop is running in a background task.
//! - **PipelineRunning** — interview converged, full pipeline is executing.
//! - **Complete** — pipeline finished.

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::Arc;
use uuid::Uuid;

use planner_core::blueprint::BlueprintStore;
use planner_schemas::{
    DomainClassification, QuestionOutput, RequirementsBeliefState, SocraticEvent, SpeculativeDraft,
};

use crate::blueprint_table::BlueprintTableState;
use crate::pipeline::{PipelineEvent, PipelineReceiver};

fn open_blueprint_store() -> BlueprintStore {
    let data_dir = std::env::var("PLANNER_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    BlueprintStore::open(std::path::Path::new(&data_dir)).unwrap_or_else(|_| BlueprintStore::new())
}

// ---------------------------------------------------------------------------
// Provider Status
// ---------------------------------------------------------------------------

/// Detected LLM provider status.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub name: String,
    pub binary: String,
    pub available: bool,
}

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
    /// Used when a pipeline stage fails.
    #[allow(dead_code)]
    Failed,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub status: StageStatus,
}

// ---------------------------------------------------------------------------
// Intake Phase
// ---------------------------------------------------------------------------

/// The Socratic interview phase — drives which UI mode is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntakePhase {
    /// Waiting for the user's first message.
    WaitingForInput,
    /// Running the Socratic interview (split-pane layout).
    Interviewing,
    /// Interview complete, full pipeline is running (full-width layout).
    PipelineRunning,
    /// Everything done.
    Complete,
}

// ---------------------------------------------------------------------------
// App Focus Mode
// ---------------------------------------------------------------------------

/// Which panel has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusMode {
    /// User is typing in the input box.
    Input,
    /// User is scrolling through chat history.
    ChatScroll,
    /// User is browsing the belief-state panel (right pane).
    BeliefStatePane,
    /// User is browsing the logs panel.
    LogsPane,
}

// ---------------------------------------------------------------------------
// Right Pane Mode
// ---------------------------------------------------------------------------

/// Which view is shown in the right pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RightPaneMode {
    BeliefState,
    Logs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppView {
    Socratic,
    Blueprint,
}

// ---------------------------------------------------------------------------
// App State
// ---------------------------------------------------------------------------

/// The main TUI application state.
pub struct App {
    /// Which top-level view is active.
    pub view: AppView,
    /// Should the app exit?
    pub should_quit: bool,
    /// Current input buffer.
    pub input: String,
    /// Cursor position in the input buffer (character index, not byte offset).
    pub cursor_position: usize,
    /// Chat message history.
    pub messages: Vec<ChatMessage>,
    /// Pipeline stages and their status.
    pub stages: Vec<PipelineStage>,
    /// Current keyboard focus.
    pub focus: FocusMode,
    /// Scroll offset for chat history.
    pub scroll_offset: u16,
    /// Project ID for this session.
    pub project_id: Uuid,
    /// Session start time (formatted string).
    pub session_start: String,
    /// Whether the pipeline is actively running.
    pub pipeline_running: bool,
    /// Status message for the bottom bar.
    pub status_message: String,

    // -----------------------------------------------------------------------
    // Intake phase
    // -----------------------------------------------------------------------
    /// Current intake phase — governs layout and input routing.
    pub intake_phase: IntakePhase,

    /// The latest belief state received from the Socratic engine.
    pub belief_state: Option<RequirementsBeliefState>,

    /// Domain classification, received after the first message is processed.
    pub classification: Option<DomainClassification>,

    /// The current question being asked by the Socratic engine.
    pub current_question: Option<QuestionOutput>,

    /// The most recent speculative draft, if any.
    pub speculative_draft: Option<SpeculativeDraft>,

    /// Convergence percentage (0.0–1.0), derived from the latest belief state.
    pub convergence_pct: f32,

    // -----------------------------------------------------------------------
    // Pipeline orchestration
    // -----------------------------------------------------------------------
    /// Pending pipeline description — set by `submit_input()` on the first
    /// message (WaitingForInput phase) so the main loop can spawn the
    /// Socratic background task.
    pub pending_socratic_message: Option<String>,

    /// Pending pipeline description — consumed by main loop after interview
    /// converges, to spawn the full planning pipeline.
    pub pending_pipeline_description: Option<String>,

    /// Channel receiver for pipeline events from the background pipeline task.
    /// `None` until the Socratic interview completes and the pipeline starts.
    pub pipeline_rx: Option<PipelineReceiver>,

    // -----------------------------------------------------------------------
    // Socratic IO channels
    // -----------------------------------------------------------------------
    /// Send user replies into the Socratic engine.
    /// Set by `pipeline::spawn_socratic_interview()`.
    pub socratic_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,

    /// Receive `SocraticEvent`s from the Socratic engine.
    /// Set by `pipeline::spawn_socratic_interview()`.
    pub socratic_events_rx: Option<tokio::sync::mpsc::UnboundedReceiver<SocraticEvent>>,

    // -----------------------------------------------------------------------
    // Observability
    // -----------------------------------------------------------------------
    /// Structured observability events for this session.
    pub planner_events: Vec<planner_core::observability::PlannerEvent>,

    /// Current step being executed (for status bar display).
    pub current_step: Option<String>,

    /// When the current step started (for elapsed time display).
    pub current_step_started: Option<std::time::Instant>,

    /// LLM call count (derived from events for quick access).
    pub llm_call_count: u32,

    /// Which right-pane view is active.
    pub right_pane_mode: RightPaneMode,

    /// Scroll offset for the logs panel.
    pub logs_scroll_offset: u16,

    /// Event log filter: None = all, Some = filtered level.
    pub logs_filter: Option<planner_core::observability::EventLevel>,

    /// LLM provider status detected at startup.
    pub providers: Vec<ProviderStatus>,

    /// Receive structured PlannerEvents from the Socratic engine.
    /// Set by `pipeline::spawn_socratic_interview()`.
    pub planner_events_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<planner_core::observability::PlannerEvent>>,

    /// Shared blueprint store for TUI browsing and pipeline emission.
    pub blueprint_store: Arc<BlueprintStore>,

    /// Blueprint table/browser state.
    pub blueprint: BlueprintTableState,
}

impl App {
    pub fn new() -> Self {
        let now = Utc::now();
        let blueprint_store = Arc::new(open_blueprint_store());

        let stages = vec![
            PipelineStage {
                name: "Intake".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Chunk".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Compile".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Lint".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "AR Review".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Refine".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Scenarios".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Ralph".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Graph".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Factory".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Validate".into(),
                status: StageStatus::Pending,
            },
            PipelineStage {
                name: "Git".into(),
                status: StageStatus::Pending,
            },
        ];

        let providers = vec![
            ProviderStatus {
                name: "anthropic".into(),
                binary: "claude".into(),
                available: planner_core::llm::providers::cli_available("claude"),
            },
            ProviderStatus {
                name: "google".into(),
                binary: "gemini".into(),
                available: planner_core::llm::providers::cli_available("gemini"),
            },
            ProviderStatus {
                name: "openai".into(),
                binary: "codex".into(),
                available: planner_core::llm::providers::cli_available("codex"),
            },
        ];

        let mut app = App {
            view: AppView::Socratic,
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
            intake_phase: IntakePhase::WaitingForInput,
            belief_state: None,
            classification: None,
            current_question: None,
            speculative_draft: None,
            convergence_pct: 0.0,
            pending_socratic_message: None,
            pending_pipeline_description: None,
            pipeline_rx: None,
            socratic_tx: None,
            socratic_events_rx: None,
            planner_events: Vec::new(),
            current_step: None,
            current_step_started: None,
            llm_call_count: 0,
            right_pane_mode: RightPaneMode::BeliefState,
            logs_scroll_offset: 0,
            logs_filter: None,
            providers,
            planner_events_rx: None,
            blueprint_store,
            blueprint: BlueprintTableState::default(),
        };

        app.add_system_message(
            "Welcome to Planner v2 — Socratic Planning Session\n\
             \n\
             Describe what you want to build and I'll guide you through\n\
             a structured requirements elicitation process.\n\
             \n\
             Press Enter to submit, Tab to switch panes, Ctrl+C or Esc to quit.",
        );

        app
    }

    // -----------------------------------------------------------------------
    // Message helpers
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Observability helpers
    // -----------------------------------------------------------------------

    /// Record a PlannerEvent and update derived state.
    pub fn record_planner_event(&mut self, event: planner_core::observability::PlannerEvent) {
        if let Some(ref step) = event.step {
            self.current_step = Some(step.clone());
            self.current_step_started = Some(std::time::Instant::now());
        }
        if event.source == planner_core::observability::EventSource::LlmRouter
            && event
                .step
                .as_deref()
                .map(|s| s.starts_with("llm.call.complete"))
                .unwrap_or(false)
        {
            self.llm_call_count += 1;
        }
        self.planner_events.push(event);
    }

    /// Get filtered events for the logs panel.
    pub fn filtered_events(&self) -> Vec<&planner_core::observability::PlannerEvent> {
        match self.logs_filter {
            None => self.planner_events.iter().collect(),
            Some(level) => self
                .planner_events
                .iter()
                .filter(|e| e.level == level)
                .collect(),
        }
    }

    /// Drain the planner events channel and update TUI state.
    ///
    /// Returns true if any events were processed (useful for forced redraws).
    pub fn tick_planner_events(&mut self) -> bool {
        let events: Vec<planner_core::observability::PlannerEvent> = {
            if let Some(ref mut rx) = self.planner_events_rx {
                let mut buf = Vec::new();
                while let Ok(ev) = rx.try_recv() {
                    buf.push(ev);
                }
                buf
            } else {
                return false;
            }
        };

        if events.is_empty() {
            return false;
        }

        for event in events {
            self.record_planner_event(event);
        }

        true
    }

    // -----------------------------------------------------------------------
    // Key handling
    // -----------------------------------------------------------------------

    /// Dispatch a key event to the focused handler.
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl+C always quits regardless of focus
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.toggle_view();
            return;
        }

        if self.view == AppView::Blueprint {
            self.handle_blueprint_key(key);
            return;
        }

        // Tab cycles focus regardless of current mode (when not in pipeline-only phase)
        if key.code == KeyCode::Tab {
            self.cycle_focus();
            return;
        }

        match self.focus {
            FocusMode::Input => self.handle_input_key(key),
            FocusMode::ChatScroll => self.handle_scroll_key(key),
            FocusMode::BeliefStatePane => self.handle_belief_pane_key(key),
            FocusMode::LogsPane => self.handle_logs_pane_key(key),
        }
    }

    fn toggle_view(&mut self) {
        self.view = match self.view {
            AppView::Socratic => {
                self.blueprint.load_blueprint(self.blueprint_store.as_ref());
                AppView::Blueprint
            }
            AppView::Blueprint => AppView::Socratic,
        };
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusMode::Input => FocusMode::ChatScroll,
            FocusMode::ChatScroll => {
                if self.intake_phase == IntakePhase::Interviewing {
                    FocusMode::BeliefStatePane
                } else {
                    FocusMode::Input
                }
            }
            FocusMode::BeliefStatePane => {
                if self.intake_phase == IntakePhase::Interviewing {
                    FocusMode::LogsPane
                } else {
                    FocusMode::Input
                }
            }
            FocusMode::LogsPane => FocusMode::Input,
        };
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                match self.intake_phase {
                    IntakePhase::Interviewing => {
                        // Skip the current question
                        self.skip_current_question();
                    }
                    _ => {
                        if self.input.is_empty() {
                            self.should_quit = true;
                        } else {
                            self.input.clear();
                            self.cursor_position = 0;
                        }
                    }
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.intake_phase == IntakePhase::Interviewing {
                    self.stop_interview();
                }
            }
            KeyCode::Enter => {
                self.submit_input();
            }
            // 1-9: Quick-select answer options (Interviewing phase only)
            KeyCode::Char(c @ '1'..='9') if self.intake_phase == IntakePhase::Interviewing => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(ref question) = self.current_question.clone() {
                    if idx < question.quick_options.len() {
                        let value = question.quick_options[idx].value.clone();
                        self.input = value;
                        self.cursor_position = self.input.chars().count();
                        self.submit_input();
                        return;
                    }
                }
                // No matching quick-option — treat as normal character input
                self.insert_char(c);
            }
            // 'L' toggles the right pane between BeliefState and Logs (Interviewing only)
            KeyCode::Char('l') if self.intake_phase == IntakePhase::Interviewing => {
                self.right_pane_mode = match self.right_pane_mode {
                    RightPaneMode::BeliefState => RightPaneMode::Logs,
                    RightPaneMode::Logs => RightPaneMode::BeliefState,
                };
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    let byte_pos = self
                        .input
                        .char_indices()
                        .nth(self.cursor_position)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input.remove(byte_pos);
                }
            }
            KeyCode::Delete => {
                let char_count = self.input.chars().count();
                if self.cursor_position < char_count {
                    let byte_pos = self
                        .input
                        .char_indices()
                        .nth(self.cursor_position)
                        .map(|(i, _)| i)
                        .unwrap_or(self.input.len());
                    self.input.remove(byte_pos);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.chars().count() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input.chars().count();
            }
            KeyCode::Up => {
                self.focus = FocusMode::ChatScroll;
            }
            _ => {}
        }
    }

    fn handle_blueprint_key(&mut self, key: KeyEvent) {
        if self.blueprint.search_mode {
            match key.code {
                KeyCode::Esc => self.blueprint.clear_search(),
                KeyCode::Enter => {
                    self.blueprint.search_mode = false;
                    self.blueprint
                        .load_selected_detail(self.blueprint_store.as_ref());
                }
                KeyCode::Backspace => self.blueprint.pop_filter_char(),
                KeyCode::Char(c) => self.blueprint.push_filter_char(c),
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.blueprint.filter.is_empty() {
                    self.view = AppView::Socratic;
                } else {
                    self.blueprint.clear_search();
                    self.blueprint
                        .load_selected_detail(self.blueprint_store.as_ref());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.blueprint.move_down();
                self.blueprint
                    .load_selected_detail(self.blueprint_store.as_ref());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.blueprint.move_up();
                self.blueprint
                    .load_selected_detail(self.blueprint_store.as_ref());
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.blueprint.jump_top();
                self.blueprint
                    .load_selected_detail(self.blueprint_store.as_ref());
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.blueprint.jump_bottom();
                self.blueprint
                    .load_selected_detail(self.blueprint_store.as_ref());
            }
            KeyCode::Char('/') => {
                self.blueprint.search_mode = true;
            }
            KeyCode::Char('t') => {
                self.blueprint.cycle_type_filter();
                self.blueprint
                    .load_selected_detail(self.blueprint_store.as_ref());
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.blueprint.detail_expanded = !self.blueprint.detail_expanded;
                if self.blueprint.detail_expanded {
                    self.blueprint
                        .load_selected_detail(self.blueprint_store.as_ref());
                }
            }
            _ => {}
        }
    }

    fn handle_scroll_key(&mut self, key: KeyEvent) {
        match key.code {
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

    fn handle_belief_pane_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.focus = FocusMode::Input;
            }
            _ => {
                self.focus = FocusMode::Input;
            }
        }
    }

    fn handle_logs_pane_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.focus = FocusMode::Input;
            }
            // j / Down — scroll logs down (toward newer events)
            KeyCode::Char('j') | KeyCode::Down => {
                if self.logs_scroll_offset > 0 {
                    self.logs_scroll_offset -= 1;
                } else {
                    self.focus = FocusMode::Input;
                }
            }
            // k / Up — scroll logs up (toward older events)
            KeyCode::Char('k') | KeyCode::Up => {
                self.logs_scroll_offset += 1;
            }
            KeyCode::PageUp => {
                self.logs_scroll_offset += 10;
            }
            KeyCode::PageDown => {
                self.logs_scroll_offset = self.logs_scroll_offset.saturating_sub(10);
            }
            // f — cycle filter: None → Error → Warn → None
            KeyCode::Char('f') => {
                self.logs_filter = match self.logs_filter {
                    None => Some(planner_core::observability::EventLevel::Error),
                    Some(planner_core::observability::EventLevel::Error) => {
                        Some(planner_core::observability::EventLevel::Warn)
                    }
                    Some(planner_core::observability::EventLevel::Warn) => None,
                    Some(planner_core::observability::EventLevel::Info) => None,
                };
                // Reset scroll when filter changes
                self.logs_scroll_offset = 0;
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Input helpers
    // -----------------------------------------------------------------------

    fn insert_char(&mut self, c: char) {
        let byte_pos = self
            .input
            .char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len());
        self.input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    // -----------------------------------------------------------------------
    // Submit / skip / stop
    // -----------------------------------------------------------------------

    /// Submit the current input, routing behaviour by `intake_phase`.
    fn submit_input(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        self.add_user_message(&text);
        self.input.clear();
        self.cursor_position = 0;
        self.scroll_offset = 0;

        match self.intake_phase {
            IntakePhase::WaitingForInput => {
                // Store first message — main loop will spawn the Socratic task
                self.intake_phase = IntakePhase::Interviewing;
                self.status_message = "Interview starting…".into();
                self.pending_socratic_message = Some(text);
            }
            IntakePhase::Interviewing => {
                // Forward the reply to the Socratic engine via the channel
                if let Some(ref tx) = self.socratic_tx {
                    let _ = tx.send(text);
                } else {
                    self.add_system_message("(Socratic engine not yet ready — please wait)");
                }
            }
            IntakePhase::PipelineRunning => {
                self.add_planner_message(
                    "Pipeline is currently running. \
                     Interactive follow-up during execution will be available in a future version.",
                );
            }
            IntakePhase::Complete => {
                self.add_planner_message(
                    "Session is complete. Start a new session to plan another project.",
                );
            }
        }
    }

    /// Skip the current Socratic question (Esc in Interviewing phase).
    fn skip_current_question(&mut self) {
        if let Some(ref tx) = self.socratic_tx {
            let _ = tx.send("skip".to_string());
            self.add_system_message("(Question skipped)");
            self.current_question = None;
        }
    }

    /// Signal the Socratic engine that the user wants to stop early (Ctrl+D).
    fn stop_interview(&mut self) {
        if let Some(ref tx) = self.socratic_tx {
            let _ = tx.send("just build it".to_string());
            self.add_system_message("(Interview stopped — proceeding to pipeline)");
        }
    }

    // -----------------------------------------------------------------------
    // Channel accessors called by main loop
    // -----------------------------------------------------------------------

    /// Take the pending first message for the Socratic task (consumed once).
    pub fn take_pending_socratic(&mut self) -> Option<String> {
        self.pending_socratic_message.take()
    }

    /// Take the pending pipeline description (consumed once).
    pub fn take_pending_pipeline(&mut self) -> Option<String> {
        self.pending_pipeline_description.take()
    }

    // -----------------------------------------------------------------------
    // Stage helpers (internal / test API)
    // -----------------------------------------------------------------------

    /// Update a pipeline stage status.
    #[allow(dead_code)]
    pub(crate) fn set_stage_status(&mut self, index: usize, status: StageStatus) {
        if index < self.stages.len() {
            self.stages[index].status = status;
        }
    }

    // -----------------------------------------------------------------------
    // Tick — drain pipeline events
    // -----------------------------------------------------------------------

    /// Drain the pipeline event channel and update state.
    pub fn tick(&mut self) {
        let events: Vec<PipelineEvent> = {
            if let Some(ref mut rx) = self.pipeline_rx {
                let mut buf = Vec::new();
                while let Ok(ev) = rx.try_recv() {
                    buf.push(ev);
                }
                buf
            } else {
                Vec::new()
            }
        };

        for event in events {
            self.apply_pipeline_event(event);
        }
    }

    fn apply_pipeline_event(&mut self, event: PipelineEvent) {
        match event {
            PipelineEvent::Started => {
                self.status_message = "Pipeline running…".into();
            }
            PipelineEvent::StepComplete(name) => {
                let mut found_idx: Option<usize> = None;
                for (i, stage) in self.stages.iter_mut().enumerate() {
                    if stage.name == name {
                        stage.status = StageStatus::Complete;
                        found_idx = Some(i);
                        break;
                    }
                }
                if let Some(idx) = found_idx {
                    let next = idx + 1;
                    if next < self.stages.len() && self.stages[next].status == StageStatus::Pending
                    {
                        self.stages[next].status = StageStatus::Running;
                    }
                    self.status_message = format!("Completed: {}", name);
                }
            }
            PipelineEvent::Completed(summary) => {
                self.pipeline_running = false;
                self.intake_phase = IntakePhase::Complete;
                for stage in &mut self.stages {
                    stage.status = StageStatus::Complete;
                }
                self.add_planner_message(&format!("Pipeline complete!\n\n{}", summary));
                self.status_message = "Pipeline complete — ready for next session".into();
            }
            PipelineEvent::Failed(err) => {
                self.pipeline_running = false;
                self.add_planner_message(&format!("Pipeline failed: {}", err));
                self.status_message = format!("Pipeline failed: {}", err);
                for stage in &mut self.stages {
                    if stage.status == StageStatus::Running {
                        stage.status = StageStatus::Failed;
                        break;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tick — drain Socratic events
    // -----------------------------------------------------------------------

    /// Drain the Socratic event channel and update TUI state.
    ///
    /// Returns true if any events were processed (useful for forced redraws).
    pub fn tick_socratic(&mut self) -> bool {
        let events: Vec<SocraticEvent> = {
            if let Some(ref mut rx) = self.socratic_events_rx {
                let mut buf = Vec::new();
                while let Ok(ev) = rx.try_recv() {
                    buf.push(ev);
                }
                buf
            } else {
                return false;
            }
        };

        if events.is_empty() {
            return false;
        }

        for event in events {
            self.apply_socratic_event(event);
        }

        true
    }

    fn apply_socratic_event(&mut self, event: SocraticEvent) {
        match event {
            SocraticEvent::Classified { classification } => {
                self.status_message = format!(
                    "{} project ({})",
                    classification.project_type,
                    match classification.complexity {
                        planner_schemas::ComplexityTier::Light => "simple",
                        planner_schemas::ComplexityTier::Standard => "standard",
                        planner_schemas::ComplexityTier::Deep => "complex",
                    }
                );
                self.add_planner_message(&format!(
                    "Classified as: {} ({}).",
                    classification.project_type,
                    match classification.complexity {
                        planner_schemas::ComplexityTier::Light => "simple",
                        planner_schemas::ComplexityTier::Standard => "standard",
                        planner_schemas::ComplexityTier::Deep => "complex",
                    }
                ));
                self.classification = Some(classification);
            }

            SocraticEvent::BeliefStateUpdate { state } => {
                self.convergence_pct = state.convergence_pct();
                self.belief_state = Some(state);
            }

            SocraticEvent::Question { output } => {
                self.add_planner_message(&output.question.clone());
                self.current_question = Some(output);
                self.status_message =
                    "Answering — [Esc] Skip  [Ctrl+D] Done  [1-9] Quick pick".into();
            }

            SocraticEvent::SpeculativeDraftReady { draft } => {
                // Render draft sections as a planner message for the chat pane
                let mut text =
                    String::from("Here's a speculative draft based on what I know so far:\n");
                for section in &draft.sections {
                    text.push_str(&format!("\n**{}**\n{}\n", section.heading, section.content));
                }
                if !draft.assumptions.is_empty() {
                    text.push_str("\nAssumptions (please correct if wrong):\n");
                    for a in &draft.assumptions {
                        text.push_str(&format!("  • {} — {}\n", a.dimension.label(), a.assumption));
                    }
                }
                self.add_planner_message(&text);
                self.speculative_draft = Some(draft);
            }

            SocraticEvent::ContradictionDetected { contradiction } => {
                self.add_planner_message(&format!(
                    "Contradiction detected: {} vs {}\n{}",
                    contradiction.dimension_a.label(),
                    contradiction.dimension_b.label(),
                    contradiction.explanation
                ));
            }

            SocraticEvent::Converged { result } => {
                let pct = (result.convergence_pct * 100.0).round() as u32;
                self.add_planner_message(&format!(
                    "Requirements gathering complete ({pct}% converged). Starting the planning pipeline…"
                ));
                self.convergence_pct = result.convergence_pct;
                self.current_question = None;
                self.intake_phase = IntakePhase::PipelineRunning;
                self.pipeline_running = true;
                self.stages[0].status = StageStatus::Running;
                self.status_message = "Pipeline starting…".into();

                // Signal the main loop to spawn the pipeline.
                // We use the belief-state goal as the description if available.
                let description = self
                    .belief_state
                    .as_ref()
                    .and_then(|bs| bs.filled.get(&planner_schemas::Dimension::Goal))
                    .map(|sv| sv.value.clone())
                    .unwrap_or_else(|| "project from Socratic interview".to_string());
                self.pending_pipeline_description = Some(description);
            }

            SocraticEvent::SystemMessage { content } => {
                self.add_planner_message(&content);
            }

            SocraticEvent::Error { message } => {
                self.add_planner_message(&format!("Socratic engine error: {}", message));
                self.status_message = format!("Error: {}", message);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::PipelineEvent;
    use tokio::sync::mpsc;

    // -----------------------------------------------------------------------
    // Preserved tests (updated where needed for new phase model)
    // -----------------------------------------------------------------------

    #[test]
    fn app_starts_with_welcome_message() {
        let app = App::new();
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
        assert!(app.messages[0].content.contains("Welcome"));
    }

    #[test]
    fn app_starts_in_waiting_phase() {
        let app = App::new();
        assert_eq!(app.intake_phase, IntakePhase::WaitingForInput);
        assert_eq!(app.focus, FocusMode::Input);
    }

    #[test]
    fn app_input_handling() {
        let mut app = App::new();

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
    fn app_first_submit_enters_interviewing_phase() {
        let mut app = App::new();

        for c in "Build me a widget".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.input.is_empty());
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.intake_phase, IntakePhase::Interviewing);

        // welcome + user message = 2
        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[1].role, MessageRole::User);

        // pending_socratic_message should be set
        assert_eq!(
            app.pending_socratic_message,
            Some("Build me a widget".to_string())
        );
        // pipeline should NOT start yet
        assert!(!app.pipeline_running);
    }

    #[test]
    fn app_empty_submit_ignored() {
        let mut app = App::new();
        let msg_count = app.messages.len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.messages.len(), msg_count);
    }

    #[test]
    fn app_esc_clears_or_quits_in_waiting_phase() {
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
    fn app_esc_in_interviewing_skips_question() {
        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        // Provide a channel so skip actually sends
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        app.socratic_tx = Some(tx);

        let msg_count_before = app.messages.len();
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        // Should add a system "(Question skipped)" message
        assert!(app.messages.len() > msg_count_before);
        assert!(app.messages.last().unwrap().content.contains("skipped"));
    }

    #[test]
    fn app_ctrl_d_in_interviewing_stops_interview() {
        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        app.socratic_tx = Some(tx);

        let msg_count_before = app.messages.len();
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(app.messages.len() > msg_count_before);
    }

    #[test]
    fn app_ctrl_c_always_quits() {
        let mut app = App::new();
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn app_tab_cycles_focus() {
        let mut app = App::new();
        assert_eq!(app.focus, FocusMode::Input);

        // In WaitingForInput, Tab should skip BeliefStatePane/LogsPane (no interview yet)
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        // Input → ChatScroll
        assert_eq!(app.focus, FocusMode::ChatScroll);

        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        // ChatScroll → Input (not Interviewing, so skip belief/logs)
        assert_eq!(app.focus, FocusMode::Input);
    }

    #[test]
    fn app_tab_includes_belief_pane_during_interviewing() {
        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;
        assert_eq!(app.focus, FocusMode::Input);

        // Input → ChatScroll
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::ChatScroll);

        // ChatScroll → BeliefStatePane
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::BeliefStatePane);

        // BeliefStatePane → LogsPane
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::LogsPane);

        // LogsPane → Input
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::Input);
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

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::ChatScroll);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusMode::Input);
    }

    #[test]
    fn app_utf8_multibyte_cursor() {
        let mut app = App::new();

        for c in "a©中b".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.cursor_position, 4);
        assert_eq!(app.input.len(), 7);
        assert_eq!(app.input, "a©中b");

        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 3);
        assert_eq!(app.input, "a©中");

        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 2);
        assert_eq!(app.input, "a©");
        assert_eq!(app.input.len(), 3);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 1);

        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 0);

        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char('€'), KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 3);
        assert_eq!(app.input, "a©€");

        // Delete at end — no-op
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.input, "a©€");
        assert_eq!(app.cursor_position, 3);

        // Move left one, then delete '€'
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.cursor_position, 2);
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.input, "a©");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn message_role_labels() {
        assert_eq!(MessageRole::System.label(), "System");
        assert_eq!(MessageRole::User.label(), "You");
        assert_eq!(MessageRole::Planner.label(), "Planner");
    }

    #[test]
    fn take_pending_socratic_consumed_once() {
        let mut app = App::new();
        for c in "My project".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let msg = app.take_pending_socratic();
        assert_eq!(msg, Some("My project".to_string()));
        assert_eq!(app.take_pending_socratic(), None);
    }

    #[test]
    fn take_pending_pipeline_consumed_once() {
        let mut app = App::new();
        app.pending_pipeline_description = Some("test".to_string());

        let desc = app.take_pending_pipeline();
        assert_eq!(desc, Some("test".to_string()));
        assert_eq!(app.take_pending_pipeline(), None);
    }

    // -----------------------------------------------------------------------
    // Pipeline tick tests (preserved)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn tick_step_complete_advances_stages() {
        let mut app = App::new();
        app.pipeline_running = true;
        app.stages[0].status = StageStatus::Running;

        let (tx, rx) = mpsc::unbounded_channel::<PipelineEvent>();
        app.pipeline_rx = Some(rx);

        tx.send(PipelineEvent::StepComplete("Intake".to_string()))
            .unwrap();
        app.tick();

        assert_eq!(app.stages[0].status, StageStatus::Complete);
        assert_eq!(app.stages[1].status, StageStatus::Running);
        assert!(app.status_message.contains("Intake"));

        tx.send(PipelineEvent::StepComplete("Chunk".to_string()))
            .unwrap();
        app.tick();

        assert_eq!(app.stages[1].status, StageStatus::Complete);
        assert_eq!(app.stages[2].status, StageStatus::Running);
    }

    #[tokio::test]
    async fn tick_step_complete_unknown_name_is_noop() {
        let mut app = App::new();
        app.pipeline_running = true;

        let (tx, rx) = mpsc::unbounded_channel::<PipelineEvent>();
        app.pipeline_rx = Some(rx);

        tx.send(PipelineEvent::StepComplete("NonExistentStage".to_string()))
            .unwrap();
        app.tick();

        assert!(app.stages.iter().all(|s| s.status == StageStatus::Pending));
    }

    #[tokio::test]
    async fn tick_processes_pipeline_events() {
        let mut app = App::new();
        app.pipeline_running = true;
        app.stages[0].status = StageStatus::Running;

        let (tx, rx) = mpsc::unbounded_channel::<PipelineEvent>();
        app.pipeline_rx = Some(rx);

        tx.send(PipelineEvent::Started).unwrap();
        app.tick();
        assert!(app.status_message.contains("running"));

        tx.send(PipelineEvent::Completed(
            "Project: Test\nSpecs: 1 chunk(s)".into(),
        ))
        .unwrap();
        app.tick();

        assert!(!app.pipeline_running);
        assert_eq!(app.intake_phase, IntakePhase::Complete);
        assert!(app.stages.iter().all(|s| s.status == StageStatus::Complete));
        let last_msg = app.messages.last().unwrap();
        assert_eq!(last_msg.role, MessageRole::Planner);
        assert!(last_msg.content.contains("Pipeline complete!"));
        assert!(last_msg.content.contains("Project: Test"));
    }

    #[tokio::test]
    async fn tick_handles_pipeline_failure() {
        let mut app = App::new();
        app.pipeline_running = true;
        app.stages[0].status = StageStatus::Running;

        let (tx, rx) = mpsc::unbounded_channel::<PipelineEvent>();
        app.pipeline_rx = Some(rx);

        tx.send(PipelineEvent::Failed("LLM CLI not found".into()))
            .unwrap();
        app.tick();

        assert!(!app.pipeline_running);
        assert_eq!(app.stages[0].status, StageStatus::Failed);
        let last_msg = app.messages.last().unwrap();
        assert!(last_msg.content.contains("Pipeline failed"));
        assert!(last_msg.content.contains("LLM CLI not found"));
    }

    // -----------------------------------------------------------------------
    // Socratic tick tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn tick_socratic_classified_event() {
        use planner_schemas::{ComplexityTier, Dimension, DomainClassification, ProjectType};

        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, rx) = mpsc::unbounded_channel::<SocraticEvent>();
        app.socratic_events_rx = Some(rx);

        let classification = DomainClassification {
            project_type: ProjectType::WebApp,
            complexity: ComplexityTier::Standard,
            detected_signals: vec!["web".into()],
            required_dimensions: Dimension::required_for(&ProjectType::WebApp),
        };

        tx.send(SocraticEvent::Classified {
            classification: classification.clone(),
        })
        .unwrap();
        let had_events = app.tick_socratic();

        assert!(had_events);
        assert!(app.classification.is_some());
        assert!(app.status_message.contains("Web App"));
    }

    #[tokio::test]
    async fn tick_socratic_belief_state_update() {
        use planner_schemas::{
            ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
        };

        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, rx) = mpsc::unbounded_channel::<SocraticEvent>();
        app.socratic_events_rx = Some(rx);

        let classification = DomainClassification {
            project_type: ProjectType::CliTool,
            complexity: ComplexityTier::Light,
            detected_signals: vec![],
            required_dimensions: Dimension::required_for(&ProjectType::CliTool),
        };
        let belief_state = RequirementsBeliefState::from_classification(&classification);
        let expected_pct = belief_state.convergence_pct();

        tx.send(SocraticEvent::BeliefStateUpdate {
            state: belief_state,
        })
        .unwrap();
        app.tick_socratic();

        assert!(app.belief_state.is_some());
        assert!((app.convergence_pct - expected_pct).abs() < 0.001);
    }

    #[tokio::test]
    async fn tick_socratic_question_event() {
        use planner_schemas::{Dimension, QuestionOutput, QuickOption};

        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, rx) = mpsc::unbounded_channel::<SocraticEvent>();
        app.socratic_events_rx = Some(rx);

        let question = QuestionOutput {
            question: "What is the main goal of your project?".into(),
            target_dimension: Dimension::Goal,
            quick_options: vec![QuickOption {
                label: "Productivity".into(),
                value: "Improve productivity".into(),
            }],
            allow_skip: true,
        };

        tx.send(SocraticEvent::Question {
            output: question.clone(),
        })
        .unwrap();
        app.tick_socratic();

        assert!(app.current_question.is_some());
        assert_eq!(
            app.current_question.as_ref().unwrap().question,
            question.question
        );
        // A planner message should have been added
        let last = app.messages.last().unwrap();
        assert_eq!(last.role, MessageRole::Planner);
        assert!(last.content.contains("goal"));
    }

    #[tokio::test]
    async fn tick_socratic_converged_starts_pipeline() {
        use planner_schemas::{ConvergenceResult, StoppingReason};

        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, rx) = mpsc::unbounded_channel::<SocraticEvent>();
        app.socratic_events_rx = Some(rx);

        let result = ConvergenceResult {
            is_done: true,
            reason: StoppingReason::CompletenessGate,
            convergence_pct: 0.9,
        };

        tx.send(SocraticEvent::Converged { result }).unwrap();
        app.tick_socratic();

        assert_eq!(app.intake_phase, IntakePhase::PipelineRunning);
        assert!(app.pipeline_running);
        assert!(app.pending_pipeline_description.is_some());
        assert_eq!(app.stages[0].status, StageStatus::Running);
    }

    #[tokio::test]
    async fn tick_socratic_error_event() {
        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        let (tx, rx) = mpsc::unbounded_channel::<SocraticEvent>();
        app.socratic_events_rx = Some(rx);

        tx.send(SocraticEvent::Error {
            message: "LLM timeout".into(),
        })
        .unwrap();
        app.tick_socratic();

        let last = app.messages.last().unwrap();
        assert!(last.content.contains("LLM timeout"));
    }

    #[test]
    fn quick_select_fills_input_and_submits() {
        use planner_schemas::{Dimension, QuestionOutput, QuickOption};

        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;

        // Provide a sender so submit_input can route the reply
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        app.socratic_tx = Some(tx);

        app.current_question = Some(QuestionOutput {
            question: "Which best describes your goal?".into(),
            target_dimension: Dimension::Goal,
            quick_options: vec![
                QuickOption {
                    label: "Option A".into(),
                    value: "Improve productivity".into(),
                },
                QuickOption {
                    label: "Option B".into(),
                    value: "Save time".into(),
                },
            ],
            allow_skip: true,
        });

        // Press '2' → quick-select second option
        app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));

        // The channel should have received "Save time"
        let sent = rx.try_recv().expect("expected message sent to socratic_tx");
        assert_eq!(sent, "Save time");
        // Input buffer should be cleared after submit
        assert!(app.input.is_empty());
    }

    // -----------------------------------------------------------------------
    // Observability tests
    // -----------------------------------------------------------------------

    #[test]
    fn record_planner_event_updates_current_step() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut app = App::new();
        assert!(app.current_step.is_none());

        let event = PlannerEvent::info(EventSource::Pipeline, "compile", "Compiling NLSpec");
        app.record_planner_event(event);

        assert_eq!(app.current_step.as_deref(), Some("compile"));
        assert!(app.current_step_started.is_some());
        assert_eq!(app.planner_events.len(), 1);
        assert_eq!(app.llm_call_count, 0);
    }

    #[test]
    fn record_planner_event_counts_llm_calls() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut app = App::new();
        assert_eq!(app.llm_call_count, 0);

        // A non-LLM-complete event should NOT increment the counter
        let ev1 = PlannerEvent::info(EventSource::LlmRouter, "llm.call.start", "Starting");
        app.record_planner_event(ev1);
        assert_eq!(app.llm_call_count, 0);

        // An LlmRouter event with step starting with "llm.call.complete" should increment
        let ev2 = PlannerEvent::info(EventSource::LlmRouter, "llm.call.complete", "Done");
        app.record_planner_event(ev2);
        assert_eq!(app.llm_call_count, 1);

        // Another complete event
        let ev3 = PlannerEvent::info(EventSource::LlmRouter, "llm.call.complete.extra", "Done");
        app.record_planner_event(ev3);
        assert_eq!(app.llm_call_count, 2);
    }

    #[test]
    fn filtered_events_none_returns_all() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut app = App::new();
        app.record_planner_event(PlannerEvent::info(EventSource::Pipeline, "a", "Info"));
        app.record_planner_event(PlannerEvent::warn(EventSource::Pipeline, "b", "Warn"));
        app.record_planner_event(PlannerEvent::error(EventSource::Pipeline, "c", "Error"));

        assert!(app.logs_filter.is_none());
        assert_eq!(app.filtered_events().len(), 3);
    }

    #[test]
    fn filtered_events_error_filter() {
        use planner_core::observability::{EventLevel, EventSource, PlannerEvent};

        let mut app = App::new();
        app.logs_filter = Some(EventLevel::Error);

        app.record_planner_event(PlannerEvent::info(EventSource::Pipeline, "a", "Info"));
        app.record_planner_event(PlannerEvent::warn(EventSource::Pipeline, "b", "Warn"));
        app.record_planner_event(PlannerEvent::error(EventSource::Pipeline, "c", "Error"));

        let filtered = app.filtered_events();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "Error");
    }

    #[test]
    fn logs_pane_key_scrolls_and_filters() {
        use planner_core::observability::{EventLevel, EventSource, PlannerEvent};

        let mut app = App::new();
        app.focus = FocusMode::LogsPane;
        app.intake_phase = IntakePhase::Interviewing;

        // Add some events
        for i in 0..5 {
            app.record_planner_event(PlannerEvent::info(
                EventSource::Pipeline,
                format!("step.{}", i),
                format!("Event {}", i),
            ));
        }

        // k → scroll up
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.logs_scroll_offset, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.logs_scroll_offset, 2);

        // j → scroll down
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.logs_scroll_offset, 1);

        // f → cycle filter to Error
        app.focus = FocusMode::LogsPane;
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert_eq!(app.logs_filter, Some(EventLevel::Error));
        assert_eq!(app.logs_scroll_offset, 0); // reset on filter change

        // f → cycle to Warn
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert_eq!(app.logs_filter, Some(EventLevel::Warn));

        // f → cycle back to None
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.logs_filter.is_none());
    }

    #[test]
    fn l_key_toggles_right_pane_mode() {
        let mut app = App::new();
        app.intake_phase = IntakePhase::Interviewing;
        assert_eq!(app.right_pane_mode, RightPaneMode::BeliefState);

        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(app.right_pane_mode, RightPaneMode::Logs);

        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(app.right_pane_mode, RightPaneMode::BeliefState);
    }
}
