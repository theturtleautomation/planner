//! # Session Store — Memory-First, Disk-Backed Session Management
//!
//! Tracks active Socratic planning sessions with their chat history
//! and pipeline state. Sessions live in memory for fast access and
//! are periodically flushed to disk as MessagePack for crash safety.
//!
//! ## Persistence Model
//!
//! - **Hot path**: All reads/writes go through an in-memory `HashMap` behind
//!   a `RwLock`. Zero overhead compared to the previous in-memory-only store.
//! - **Dirty tracking**: Mutations mark sessions dirty via a `HashSet<Uuid>`.
//! - **Background flush**: A Tokio task runs every 5 seconds, writing dirty
//!   sessions to `{data_dir}/sessions/{id}.msgpack` with atomic rename.
//! - **Startup load**: `SessionStore::open()` reads all `.msgpack` files from
//!   the sessions directory back into memory.
//! - **Atomic writes**: Each flush writes to a `.tmp` file then renames,
//!   ensuring a crash mid-write never corrupts the on-disk copy.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use planner_schemas::artifacts::socratic::{
    Contradiction, DomainClassification, PromptEnvelope, PromptItem, PromptItemKind, PromptKind,
    PromptOption, PromptPreferredLayout, PromptResponseMode, PromptUiHints, QuestionOutput,
    RequirementsBeliefState, SocraticCategorySnapshot, SpeculativeDraft, UiCapabilities,
};

fn normalize_title(value: &str) -> Option<String> {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(120).collect())
    }
}

fn suggested_title_from_description(description: &str) -> Option<String> {
    let first_line = description
        .lines()
        .find_map(normalize_title)
        .or_else(|| normalize_title(description))?;

    let title: String = first_line.chars().take(72).collect();
    Some(title)
}

// ---------------------------------------------------------------------------
// Session Types
// ---------------------------------------------------------------------------

/// A single chat message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Uuid,
    pub role: String, // "user", "planner", "system"
    pub content: String,
    pub timestamp: String,
}

/// Pipeline stage status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageInfo {
    pub name: String,
    pub status: String, // "pending", "running", "complete", "failed"
}

/// Backend-computed resume state exposed to the web UI.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStatus {
    ReadyToStart,
    LiveAttachAvailable,
    InterviewAttached,
    InterviewRestartOnly,
    InterviewResumeUnknown,
    InterviewCheckpointResumable,
}

impl Default for ResumeStatus {
    fn default() -> Self {
        Self::InterviewResumeUnknown
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// Durable interview checkpoint used for detached session recovery UX.
#[derive(Debug, Clone, Serialize)]
pub struct InterviewCheckpoint {
    /// Stable run identifier reused for CXDB and checkpoint persistence.
    pub socratic_run_id: Uuid,
    /// Latest domain classification, when available.
    pub classification: Option<DomainClassification>,
    /// Latest belief-state snapshot.
    pub belief_state: Option<RequirementsBeliefState>,
    /// Active prompt envelope, if waiting for user input.
    pub current_prompt: Option<PromptEnvelope>,
    /// Latest category-navigation snapshot, if waiting on category selection.
    pub current_category_snapshot: Option<SocraticCategorySnapshot>,
    /// Active contradictions captured so far.
    #[serde(default)]
    pub contradictions: Vec<Contradiction>,
    /// Consecutive stale-turn counter.
    #[serde(default)]
    pub stale_turns: u32,
    /// Turn index where the last draft was shown.
    pub draft_shown_at_turn: Option<u32>,
    /// RFC3339 timestamp of the last checkpoint write.
    pub last_checkpoint_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InterviewCheckpointCurrentWire {
    pub socratic_run_id: Uuid,
    #[serde(default)]
    pub classification: Option<DomainClassification>,
    #[serde(default)]
    pub belief_state: Option<RequirementsBeliefState>,
    #[serde(default)]
    pub current_prompt: Option<PromptEnvelope>,
    #[serde(default)]
    pub current_category_snapshot: Option<SocraticCategorySnapshot>,
    #[serde(default)]
    pub contradictions: Vec<Contradiction>,
    #[serde(default)]
    pub stale_turns: u32,
    #[serde(default)]
    pub draft_shown_at_turn: Option<u32>,
    #[serde(default = "now_rfc3339")]
    pub last_checkpoint_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InterviewCheckpointLegacyWire {
    pub socratic_run_id: Uuid,
    #[serde(default)]
    pub classification: Option<DomainClassification>,
    #[serde(default)]
    pub belief_state: Option<RequirementsBeliefState>,
    #[serde(default)]
    pub current_question: Option<QuestionOutput>,
    #[serde(default)]
    pub pending_draft: Option<SpeculativeDraft>,
    #[serde(default)]
    pub contradictions: Vec<Contradiction>,
    #[serde(default)]
    pub stale_turns: u32,
    #[serde(default)]
    pub draft_shown_at_turn: Option<u32>,
    #[serde(default = "now_rfc3339")]
    pub last_checkpoint_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InterviewCheckpointWire {
    Current(InterviewCheckpointCurrentWire),
    Legacy(InterviewCheckpointLegacyWire),
}

// Explicit migration adapter for historical checkpoints.
//
// Remove this module after the legacy checkpoint migration window closes.
mod legacy_checkpoint_prompt_adapter {
    use super::*;

    pub fn promote(
        current_question: Option<QuestionOutput>,
        pending_draft: Option<SpeculativeDraft>,
        draft_shown_at_turn: Option<u32>,
        fallback_turn: u32,
        created_at: &str,
    ) -> Option<PromptEnvelope> {
        if let Some(draft) = pending_draft {
            return Some(from_legacy_draft(
                draft,
                draft_shown_at_turn.unwrap_or(fallback_turn),
                created_at,
            ));
        }

        current_question.map(|question| from_legacy_question(question, fallback_turn, created_at))
    }

    fn from_legacy_question(
        output: QuestionOutput,
        based_on_turn: u32,
        created_at: &str,
    ) -> PromptEnvelope {
        let item_id = String::from("legacy-question-item");
        let required = !output.allow_skip;
        let required_item_ids = required.then(|| vec![item_id.clone()]).unwrap_or_default();

        PromptEnvelope {
            prompt_id: String::from("legacy-question"),
            kind: PromptKind::QuestionBatch,
            title: String::from("Continue interview"),
            instructions: None,
            origin_category_id: None,
            category_path: Vec::new(),
            items: vec![PromptItem {
                item_id,
                kind: PromptItemKind::Discovery,
                target_dimension: Some(output.target_dimension),
                section_ref: None,
                text: output.question,
                options: output
                    .quick_options
                    .into_iter()
                    .enumerate()
                    .map(|(index, option)| PromptOption {
                        option_id: format!("legacy-option-{}", index + 1),
                        label: option.label,
                        semantic_value: option.value,
                        direct_effect: None,
                    })
                    .collect(),
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required,
                priority: 100,
                dependency_item_ids: Vec::new(),
            }],
            draft_snapshot: None,
            required_item_ids,
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Cards,
                show_draft_sidebar: false,
            },
            based_on_turn,
            created_at: created_at.to_string(),
        }
    }

    fn from_legacy_draft(
        draft: SpeculativeDraft,
        based_on_turn: u32,
        created_at: &str,
    ) -> PromptEnvelope {
        let item_id = String::from("legacy-draft-item");
        let section_ref = draft
            .sections
            .first()
            .map(|section| section.heading.clone());
        let text = section_ref
            .as_ref()
            .map(|heading| format!("Review section '{}'. Confirm or correct it.", heading))
            .unwrap_or_else(|| {
                String::from("Review this draft and share confirmations or corrections.")
            });

        PromptEnvelope {
            prompt_id: String::from("legacy-draft-review"),
            kind: PromptKind::DraftReview,
            title: String::from("Review draft"),
            instructions: Some(String::from(
                "Confirm accurate sections and provide corrections where needed.",
            )),
            origin_category_id: None,
            category_path: Vec::new(),
            items: vec![PromptItem {
                item_id,
                kind: PromptItemKind::DraftSection,
                target_dimension: None,
                section_ref,
                text,
                options: vec![
                    PromptOption {
                        option_id: String::from("confirm"),
                        label: String::from("Looks correct"),
                        semantic_value: String::from("confirm"),
                        direct_effect: None,
                    },
                    PromptOption {
                        option_id: String::from("correct"),
                        label: String::from("Needs correction"),
                        semantic_value: String::from("correct"),
                        direct_effect: None,
                    },
                    PromptOption {
                        option_id: String::from("surprise"),
                        label: String::from("Unexpected but useful"),
                        semantic_value: String::from("surprise"),
                        direct_effect: None,
                    },
                    PromptOption {
                        option_id: String::from("reject"),
                        label: String::from("Fundamentally wrong"),
                        semantic_value: String::from("reject"),
                        direct_effect: None,
                    },
                ],
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority: 100,
                dependency_item_ids: Vec::new(),
            }],
            draft_snapshot: Some(draft),
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Review,
                show_draft_sidebar: true,
            },
            based_on_turn,
            created_at: created_at.to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for InterviewCheckpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = InterviewCheckpointWire::deserialize(deserializer)?;
        match wire {
            InterviewCheckpointWire::Current(wire) => Ok(Self {
                socratic_run_id: wire.socratic_run_id,
                classification: wire.classification,
                belief_state: wire.belief_state,
                current_prompt: wire.current_prompt,
                current_category_snapshot: wire.current_category_snapshot,
                contradictions: wire.contradictions,
                stale_turns: wire.stale_turns,
                draft_shown_at_turn: wire.draft_shown_at_turn,
                last_checkpoint_at: wire.last_checkpoint_at,
            }),
            InterviewCheckpointWire::Legacy(wire) => {
                let fallback_turn = wire
                    .belief_state
                    .as_ref()
                    .map(|state| state.turn_count)
                    .unwrap_or(0);
                let current_prompt = legacy_checkpoint_prompt_adapter::promote(
                    wire.current_question,
                    wire.pending_draft,
                    wire.draft_shown_at_turn,
                    fallback_turn,
                    &wire.last_checkpoint_at,
                );

                Ok(Self {
                    socratic_run_id: wire.socratic_run_id,
                    classification: wire.classification,
                    belief_state: wire.belief_state,
                    current_prompt,
                    current_category_snapshot: None,
                    contradictions: wire.contradictions,
                    stale_turns: wire.stale_turns,
                    draft_shown_at_turn: wire.draft_shown_at_turn,
                    last_checkpoint_at: wire.last_checkpoint_at,
                })
            }
        }
    }
}

impl InterviewCheckpoint {
    pub fn new(socratic_run_id: Uuid) -> Self {
        Self {
            socratic_run_id,
            classification: None,
            belief_state: None,
            current_prompt: None,
            current_category_snapshot: None,
            contradictions: Vec::new(),
            stale_turns: 0,
            draft_shown_at_turn: None,
            last_checkpoint_at: now_rfc3339(),
        }
    }

    pub fn touch(&mut self) {
        self.last_checkpoint_at = now_rfc3339();
    }
}

/// A planning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    /// Auth0 sub claim of the owning user (or "dev|local" in dev mode).
    pub user_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub archived_at: Option<String>,
    pub created_at: String,
    /// RFC3339 timestamp of the last get() or update() access.
    pub last_accessed: String,
    pub messages: Vec<SessionMessage>,
    pub stages: Vec<PipelineStageInfo>,
    pub pipeline_running: bool,
    pub project_description: Option<String>,
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub project_slug: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,

    // -----------------------------------------------------------------------
    // Socratic interview state
    // -----------------------------------------------------------------------
    /// Current belief state from the Socratic interview.
    pub belief_state: Option<RequirementsBeliefState>,

    /// Domain classification produced at the start of the interview.
    pub classification: Option<DomainClassification>,

    /// Stable identifier for this session's Socratic run.
    #[serde(default)]
    pub socratic_run_id: Option<Uuid>,

    /// Durable checkpoint for detached interview recovery.
    #[serde(default)]
    pub checkpoint: Option<InterviewCheckpoint>,

    /// Phase of the intake process.
    /// One of: "waiting", "interviewing", "pipeline_running", "complete".
    pub intake_phase: String,

    /// Whether an interview websocket is currently attached.
    /// This is only meaningful while `intake_phase == "interviewing"`.
    #[serde(default)]
    pub interview_live_attached: bool,

    /// Most recent client-advertised UI capabilities for prompt sizing.
    #[serde(default)]
    pub ui_capabilities: Option<UiCapabilities>,

    /// Whether an in-memory interview runtime currently exists for this session.
    ///
    /// This is server-local state and should not be persisted or exposed.
    #[serde(skip)]
    pub interview_runtime_active: bool,

    /// Whether this session can currently be resumed via a live runtime attach.
    #[serde(default)]
    pub can_resume_live: bool,

    /// Whether this session can currently be resumed from a durable checkpoint.
    #[serde(default)]
    pub can_resume_checkpoint: bool,

    /// Whether restart-from-description is available for this session.
    #[serde(default)]
    pub can_restart_from_description: bool,

    /// Whether retrying the pipeline is currently supported for this session.
    #[serde(default)]
    pub can_retry_pipeline: bool,

    /// Whether a durable interview checkpoint exists.
    #[serde(default)]
    pub has_checkpoint: bool,

    /// High-level backend truth for resume UX.
    #[serde(default)]
    pub resume_status: ResumeStatus,

    /// Structured event log for this session.
    #[serde(default)]
    pub events: Vec<planner_core::observability::PlannerEvent>,

    /// What step is currently executing (for quick status display).
    pub current_step: Option<String>,

    /// Last error message (for quick display without scanning events).
    pub error_message: Option<String>,

    /// Legacy CXDB project ID field retained for migration compatibility.
    /// New session/project ownership should use `project_id`.
    #[serde(default)]
    pub cxdb_project_id: Option<Uuid>,
    /// Session-owned index of pipeline run IDs.
    #[serde(default)]
    pub run_ids: Vec<Uuid>,
}

impl Session {
    pub fn new(user_id: &str) -> Self {
        let now = Utc::now();
        let mut session = Session {
            id: Uuid::new_v4(),
            user_id: user_id.to_string(),
            title: None,
            archived: false,
            archived_at: None,
            created_at: now.to_rfc3339(),
            last_accessed: now.to_rfc3339(),
            messages: vec![SessionMessage {
                id: Uuid::new_v4(),
                role: "system".into(),
                content: "Welcome to Planner v2 — Socratic Planning Session. \
                         Describe what you want to build."
                    .into(),
                timestamp: now.to_rfc3339(),
            }],
            stages: vec![
                PipelineStageInfo {
                    name: "Intake".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Chunk".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Compile".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Lint".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "AR Review".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Refine".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Scenarios".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Ralph".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Graph".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Factory".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Validate".into(),
                    status: "pending".into(),
                },
                PipelineStageInfo {
                    name: "Git".into(),
                    status: "pending".into(),
                },
            ],
            pipeline_running: false,
            project_description: None,
            project_id: None,
            project_slug: None,
            project_name: None,
            belief_state: None,
            classification: None,
            socratic_run_id: None,
            checkpoint: None,
            intake_phase: "waiting".into(),
            interview_live_attached: false,
            ui_capabilities: None,
            interview_runtime_active: false,
            can_resume_live: false,
            can_resume_checkpoint: false,
            can_restart_from_description: false,
            can_retry_pipeline: false,
            has_checkpoint: false,
            resume_status: ResumeStatus::default(),
            events: Vec::new(),
            current_step: None,
            error_message: None,
            cxdb_project_id: None,
            run_ids: Vec::new(),
        };
        session.recompute_capabilities();
        session
    }

    fn has_saved_description(&self) -> bool {
        self.project_description
            .as_deref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .or_else(|| {
                self.project_description
                    .as_deref()
                    .and_then(suggested_title_from_description)
            })
            .unwrap_or_else(|| format!("Session {}", self.id))
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title.as_deref().and_then(normalize_title);
    }

    pub fn ensure_title_from_description(&mut self) {
        if self.title.is_none() {
            self.title = self
                .project_description
                .as_deref()
                .and_then(suggested_title_from_description);
        }
    }

    pub fn set_archived(&mut self, archived: bool) {
        self.archived = archived;
        self.archived_at = archived.then(|| Utc::now().to_rfc3339());
    }

    pub fn pipeline_has_failed(&self) -> bool {
        self.stages.iter().any(|stage| stage.status == "failed")
    }

    fn reset_stage_statuses(&mut self) {
        for stage in &mut self.stages {
            stage.status = "pending".into();
        }
    }

    fn normalize_stage_name(raw: &str) -> Option<&'static str> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "intake" => Some("Intake"),
            "chunk" | "chunk planning" => Some("Chunk"),
            "compile" | "specification compilation" => Some("Compile"),
            "lint" => Some("Lint"),
            "ar review" | "adversarial review" => Some("AR Review"),
            "refine" | "refinement" => Some("Refine"),
            "scenarios" | "scenario generation" => Some("Scenarios"),
            "ralph" | "ralph advisory" => Some("Ralph"),
            "graph" | "graph compilation" => Some("Graph"),
            "factory" => Some("Factory"),
            "validate" | "validation" => Some("Validate"),
            "git" | "git projection" => Some("Git"),
            _ => None,
        }
    }

    fn stage_from_message(message: &str) -> Option<&'static str> {
        let lower = message.to_ascii_lowercase();
        const PATTERNS: [(&str, &str); 17] = [
            ("intake stage", "Intake"),
            ("chunk planning stage", "Chunk"),
            ("chunk stage", "Chunk"),
            ("specification compilation stage", "Compile"),
            ("compile stage", "Compile"),
            ("lint stage", "Lint"),
            ("adversarial review stage", "AR Review"),
            ("ar review stage", "AR Review"),
            ("refinement stage", "Refine"),
            ("refine stage", "Refine"),
            ("scenario generation stage", "Scenarios"),
            ("scenarios stage", "Scenarios"),
            ("ralph advisory stage", "Ralph"),
            ("graph compilation stage", "Graph"),
            ("factory stage", "Factory"),
            ("validation stage", "Validate"),
            ("git projection stage", "Git"),
        ];

        PATTERNS
            .iter()
            .find_map(|(needle, stage)| lower.contains(needle).then_some(*stage))
    }

    fn stage_from_event(event: &planner_core::observability::PlannerEvent) -> Option<String> {
        let metadata_stage = event
            .metadata
            .get("stage")
            .and_then(|value| value.as_str())
            .or_else(|| {
                event
                    .metadata
                    .get("stage_name")
                    .and_then(|value| value.as_str())
            })
            .or_else(|| {
                event
                    .metadata
                    .get("details")
                    .and_then(|details| details.get("stage"))
                    .and_then(|value| value.as_str())
            })
            .or_else(|| {
                event
                    .metadata
                    .get("details")
                    .and_then(|details| details.get("stage_name"))
                    .and_then(|value| value.as_str())
            })
            .and_then(Self::normalize_stage_name)
            .map(str::to_string);

        metadata_stage.or_else(|| Self::stage_from_message(&event.message).map(str::to_string))
    }

    fn sole_running_stage(&self) -> Option<String> {
        let mut running = self.stages.iter().filter(|stage| stage.status == "running");
        let first = running.next()?;
        if running.next().is_some() {
            return None;
        }
        Some(first.name.clone())
    }

    fn event_bool(
        event: &planner_core::observability::PlannerEvent,
        key: &'static str,
    ) -> Option<bool> {
        event
            .metadata
            .get(key)
            .and_then(|value| value.as_bool())
            .or_else(|| {
                event
                    .metadata
                    .get("details")
                    .and_then(|details| details.get(key))
                    .and_then(|value| value.as_bool())
            })
    }

    fn set_stage_status(&mut self, stage_name: &str, status: &str) {
        if let Some(stage) = self
            .stages
            .iter_mut()
            .find(|stage| stage.name == stage_name)
        {
            stage.status = status.into();
        }
    }

    fn apply_pipeline_event(&mut self, event: &planner_core::observability::PlannerEvent) {
        let Some(step) = event.step.as_deref() else {
            return;
        };

        match step {
            "pipeline.stage.started" => {
                if let Some(stage_name) = Self::stage_from_event(event) {
                    self.set_stage_status(stage_name.as_str(), "running");
                }
                self.pipeline_running = true;
                if self.intake_phase != "complete" {
                    self.intake_phase = "pipeline_running".into();
                }
            }
            "pipeline.stage.completed" => {
                let stage_name =
                    Self::stage_from_event(event).or_else(|| self.sole_running_stage());
                if let Some(stage_name) = stage_name {
                    self.set_stage_status(stage_name.as_str(), "complete");
                    if stage_name == "Git" {
                        self.pipeline_running = false;
                        self.intake_phase = "complete".into();
                        self.error_message = None;
                    }
                }
            }
            "pipeline.stage.failed" => {
                let stage_name =
                    Self::stage_from_event(event).or_else(|| self.sole_running_stage());
                if let Some(stage_name) = stage_name {
                    self.set_stage_status(stage_name.as_str(), "failed");
                }

                let retry_planned = Self::event_bool(event, "retry_planned").unwrap_or(false);
                let terminal = Self::event_bool(event, "terminal")
                    .unwrap_or(event.level == planner_core::observability::EventLevel::Error);
                if terminal && !retry_planned {
                    self.pipeline_running = false;
                    self.intake_phase = "error".into();
                } else if retry_planned {
                    self.pipeline_running = true;
                    self.intake_phase = "pipeline_running".into();
                }
            }
            "pipeline.retry.started" => {
                self.pipeline_running = true;
                self.intake_phase = "pipeline_running".into();
                let stage_name = Self::stage_from_event(event)
                    .or_else(|| self.sole_running_stage())
                    .unwrap_or_else(|| String::from("Factory"));
                self.set_stage_status(stage_name.as_str(), "running");
            }
            "pipeline.validation.completed" => {
                let stage_name =
                    Self::stage_from_event(event).unwrap_or_else(|| String::from("Validate"));
                if let Some(gates_passed) = Self::event_bool(event, "gates_passed") {
                    self.set_stage_status(
                        stage_name.as_str(),
                        if gates_passed { "complete" } else { "failed" },
                    );
                }
            }
            _ => {}
        }
    }

    pub fn reset_for_interview_restart(&mut self) {
        self.pipeline_running = false;
        self.belief_state = None;
        self.classification = None;
        self.socratic_run_id = None;
        self.checkpoint = None;
        self.has_checkpoint = false;
        self.intake_phase = "interviewing".into();
        self.interview_live_attached = false;
        self.ui_capabilities = None;
        self.interview_runtime_active = false;
        self.events.clear();
        self.current_step = None;
        self.error_message = None;
        self.cxdb_project_id = None;
        self.set_archived(false);
        self.reset_stage_statuses();

        if self.messages.len() > 1 {
            self.messages.truncate(1);
        }
    }

    pub fn prepare_for_pipeline_retry(&mut self) {
        self.pipeline_running = true;
        self.intake_phase = "pipeline_running".into();
        self.interview_live_attached = false;
        self.interview_runtime_active = false;
        self.events.clear();
        self.current_step = None;
        self.error_message = None;
        self.cxdb_project_id = None;
        self.set_archived(false);
        self.reset_stage_statuses();

        if let Some(stage) = self.stages.first_mut() {
            stage.status = "running".into();
        }
    }

    /// Ensure this session has a stable Socratic run ID and return it.
    pub fn ensure_socratic_run_id(&mut self) -> Uuid {
        if let Some(id) = self.socratic_run_id {
            id
        } else {
            let id = Uuid::new_v4();
            self.socratic_run_id = Some(id);
            id
        }
    }

    /// Ensure this session has a mutable checkpoint and return it.
    pub fn ensure_checkpoint(&mut self) -> &mut InterviewCheckpoint {
        let run_id = self.ensure_socratic_run_id();
        let checkpoint = self
            .checkpoint
            .get_or_insert_with(|| InterviewCheckpoint::new(run_id));
        checkpoint.socratic_run_id = run_id;
        self.has_checkpoint = true;
        checkpoint
    }

    pub fn duplicate_for_branch(&self, title_override: Option<String>) -> Session {
        let now = Utc::now().to_rfc3339();
        let mut duplicate = Session::new(&self.user_id);
        let checkpoint = self.checkpoint.clone().map(|mut checkpoint| {
            checkpoint.socratic_run_id = Uuid::new_v4();
            checkpoint.touch();
            checkpoint
        });

        duplicate.created_at = now.clone();
        duplicate.last_accessed = now;
        duplicate.title = title_override
            .as_deref()
            .and_then(normalize_title)
            .or_else(|| normalize_title(&format!("{} (Copy)", self.display_title())));
        duplicate.archived = false;
        duplicate.archived_at = None;
        duplicate.project_description = self.project_description.clone();
        duplicate.project_id = self.project_id;
        duplicate.project_slug = self.project_slug.clone();
        duplicate.project_name = self.project_name.clone();
        duplicate.classification = checkpoint
            .as_ref()
            .and_then(|saved| saved.classification.clone())
            .or_else(|| self.classification.clone());
        duplicate.belief_state = checkpoint
            .as_ref()
            .and_then(|saved| saved.belief_state.clone())
            .or_else(|| self.belief_state.clone());
        duplicate.socratic_run_id = checkpoint.as_ref().map(|saved| saved.socratic_run_id);
        duplicate.checkpoint = checkpoint;
        duplicate.intake_phase = if duplicate.checkpoint.is_some() {
            "interviewing".into()
        } else {
            "waiting".into()
        };
        duplicate.pipeline_running = false;
        duplicate.interview_live_attached = false;
        duplicate.ui_capabilities = self.ui_capabilities.clone();
        duplicate.interview_runtime_active = false;
        duplicate.events.clear();
        duplicate.current_step = None;
        duplicate.error_message = None;
        duplicate.cxdb_project_id = None;
        duplicate.run_ids.clear();
        duplicate.reset_stage_statuses();

        let source_title = self.display_title();
        if duplicate.checkpoint.is_some() {
            duplicate.add_message(
                "planner",
                &format!(
                    "Duplicated from \"{}\". Resume from the copied checkpoint or restart from the saved description.",
                    source_title
                ),
            );
        } else if duplicate.has_saved_description() {
            duplicate.add_message(
                "planner",
                &format!(
                    "Duplicated from \"{}\". The saved description was copied into this new session.",
                    source_title
                ),
            );
        } else {
            duplicate.add_message("planner", &format!("Duplicated from \"{}\".", source_title));
        }

        duplicate.recompute_capabilities();
        duplicate
    }

    /// Recompute capability flags from the current session state.
    ///
    /// This keeps UI-facing workflow controls derived from backend truth,
    /// rather than client-side phase inference.
    pub fn recompute_capabilities(&mut self) {
        let has_description = self.has_saved_description();
        self.has_checkpoint = self.checkpoint.is_some();
        if self.interview_live_attached {
            self.interview_runtime_active = true;
        }

        self.can_resume_checkpoint = false;
        self.can_retry_pipeline = false;

        match self.intake_phase.as_str() {
            "waiting" => {
                self.interview_runtime_active = false;
                self.can_resume_live = false;
                self.can_restart_from_description = false;
                self.resume_status = ResumeStatus::ReadyToStart;
                self.interview_live_attached = false;
            }
            "interviewing" => {
                self.can_resume_live = false;
                self.can_restart_from_description = has_description;
                if self.interview_live_attached {
                    self.resume_status = ResumeStatus::InterviewAttached;
                } else if self.interview_runtime_active {
                    self.can_resume_live = true;
                    self.resume_status = ResumeStatus::LiveAttachAvailable;
                } else if self.has_checkpoint {
                    self.can_resume_checkpoint = true;
                    self.resume_status = ResumeStatus::InterviewCheckpointResumable;
                } else if has_description {
                    self.resume_status = ResumeStatus::InterviewRestartOnly;
                } else {
                    self.resume_status = ResumeStatus::InterviewResumeUnknown;
                }
            }
            "pipeline_running" => {
                self.interview_runtime_active = false;
                self.can_resume_live = true;
                self.can_restart_from_description = false;
                self.resume_status = ResumeStatus::LiveAttachAvailable;
                self.interview_live_attached = false;
            }
            "complete" | "error" => {
                self.interview_runtime_active = false;
                self.can_resume_live = true;
                self.can_restart_from_description = has_description;
                self.resume_status = ResumeStatus::LiveAttachAvailable;
                self.interview_live_attached = false;
            }
            _ => {
                self.interview_runtime_active = false;
                self.can_resume_live = false;
                self.can_restart_from_description = has_description;
                self.resume_status = ResumeStatus::InterviewResumeUnknown;
                self.interview_live_attached = false;
            }
        }

        self.can_retry_pipeline =
            self.has_saved_description() && !self.pipeline_running && self.pipeline_has_failed();
    }

    /// Count LLM calls from the event log.
    pub fn llm_call_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| {
                e.source == planner_core::observability::EventSource::LlmRouter
                    && e.step
                        .as_deref()
                        .map(|s| s.starts_with("llm.call.complete"))
                        .unwrap_or(false)
            })
            .count()
    }

    /// Total LLM latency from the event log.
    pub fn llm_total_latency_ms(&self) -> u64 {
        self.events
            .iter()
            .filter(|e| {
                e.source == planner_core::observability::EventSource::LlmRouter
                    && e.step
                        .as_deref()
                        .map(|s| s.starts_with("llm.call.complete"))
                        .unwrap_or(false)
            })
            .filter_map(|e| e.duration_ms)
            .sum()
    }

    /// Count errors from the event log.
    pub fn error_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.level == planner_core::observability::EventLevel::Error)
            .count()
    }

    /// Count warnings from the event log.
    pub fn warning_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.level == planner_core::observability::EventLevel::Warn)
            .count()
    }

    /// Return the most recent user-visible workflow activity timestamp.
    pub fn last_activity_at(&self) -> String {
        let mut candidates = vec![self.created_at.clone(), self.last_accessed.clone()];

        if let Some(message) = self.messages.last() {
            candidates.push(message.timestamp.clone());
        }

        if let Some(event) = self.events.last() {
            candidates.push(event.timestamp.to_rfc3339());
        }

        if let Some(checkpoint) = &self.checkpoint {
            candidates.push(checkpoint.last_checkpoint_at.clone());
        }

        if let Some(archived_at) = &self.archived_at {
            candidates.push(archived_at.clone());
        }

        candidates
            .into_iter()
            .filter_map(|timestamp| {
                DateTime::parse_from_rfc3339(&timestamp)
                    .ok()
                    .map(|parsed| (parsed.with_timezone(&Utc), timestamp))
            })
            .max_by(|left, right| left.0.cmp(&right.0))
            .map(|(_, timestamp)| timestamp)
            .unwrap_or_else(|| self.created_at.clone())
    }

    /// Push an event into this session's log and update current_step/error_message.
    pub fn record_event(&mut self, event: planner_core::observability::PlannerEvent) {
        self.apply_pipeline_event(&event);

        let retryable_stage_failure = event.step.as_deref() == Some("pipeline.stage.failed")
            && Self::event_bool(&event, "terminal") == Some(false);

        if event.level == planner_core::observability::EventLevel::Error && !retryable_stage_failure
        {
            self.error_message = Some(event.message.clone());
        }
        if let Some(ref step) = event.step {
            self.current_step = Some(step.clone());
        }
        self.events.push(event);
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, role: &str, content: &str) -> SessionMessage {
        let msg = SessionMessage {
            id: Uuid::new_v4(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.messages.push(msg.clone());
        msg
    }
}

// ---------------------------------------------------------------------------
// Session Summary (lightweight projection)
// ---------------------------------------------------------------------------

/// Lightweight session summary for list endpoints.
///
/// Excludes the full `messages` and `events` vectors to avoid
/// cloning potentially large payloads when only metadata is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub user_id: String,
    pub title: Option<String>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub last_accessed: String,
    pub last_activity_at: String,
    pub pipeline_running: bool,
    pub intake_phase: String,
    pub interview_live_attached: bool,
    pub project_description: Option<String>,
    pub project_id: Option<Uuid>,
    pub project_slug: Option<String>,
    pub project_name: Option<String>,
    pub message_count: usize,
    pub event_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub current_step: Option<String>,
    pub error_message: Option<String>,
    pub can_resume_live: bool,
    pub can_resume_checkpoint: bool,
    pub can_restart_from_description: bool,
    pub can_retry_pipeline: bool,
    pub has_checkpoint: bool,
    pub resume_status: ResumeStatus,
    pub classification: Option<DomainClassification>,
    pub convergence_pct: Option<f32>,
    pub checkpoint_last_saved_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Session Store
// ---------------------------------------------------------------------------

/// Thread-safe, memory-first, disk-backed store for planning sessions.
///
/// All operations hit the in-memory HashMap directly. Mutations mark sessions
/// dirty; a background task periodically flushes dirty sessions to disk as
/// MessagePack with atomic rename for crash safety.
pub struct SessionStore {
    pub(crate) sessions: RwLock<HashMap<Uuid, Session>>,
    dirty: RwLock<HashSet<Uuid>>,
    sessions_dir: Option<PathBuf>,
}

impl SessionStore {
    /// Create a purely in-memory store with no disk backing.
    /// Used in tests or when persistence is not needed.
    pub fn new() -> Self {
        SessionStore {
            sessions: RwLock::new(HashMap::new()),
            dirty: RwLock::new(HashSet::new()),
            sessions_dir: None,
        }
    }

    /// Open a disk-backed store, loading existing sessions from `data_dir/sessions/`.
    ///
    /// Creates the sessions directory if it doesn't exist. Any `.msgpack` files
    /// in the directory are deserialized into memory on startup.
    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let sessions_dir = data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir)?;

        // Validate directory is writable.
        let probe = sessions_dir.join(".write_probe");
        std::fs::write(&probe, b"ok")?;
        std::fs::remove_file(&probe)?;

        let mut sessions = HashMap::new();
        let mut load_errors = 0u32;

        for entry in std::fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if !name.ends_with(".msgpack") {
                continue;
            }

            let id_str = match name.strip_suffix(".msgpack") {
                Some(s) => s,
                None => continue,
            };

            let id = match Uuid::parse_str(id_str) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!("Skipping non-UUID session file: {}", name);
                    continue;
                }
            };

            match std::fs::read(&path) {
                Ok(bytes) => match rmp_serde::from_slice::<Session>(&bytes) {
                    Ok(mut session) => {
                        if session.intake_phase == "interviewing" {
                            session.interview_live_attached = false;
                            session.interview_runtime_active = false;
                        }
                        session.recompute_capabilities();
                        tracing::debug!("Loaded session {} from disk", id);
                        sessions.insert(id, session);
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode session {}: {}", id, e);
                        load_errors += 1;
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read session file {}: {}", name, e);
                    load_errors += 1;
                }
            }
        }

        let count = sessions.len();
        if load_errors > 0 {
            tracing::warn!(
                "Session store: loaded {} sessions, {} files had errors",
                count,
                load_errors,
            );
        } else if count > 0 {
            tracing::info!("Session store: loaded {} sessions from disk", count);
        }

        Ok(SessionStore {
            sessions: RwLock::new(sessions),
            dirty: RwLock::new(HashSet::new()),
            sessions_dir: Some(sessions_dir),
        })
    }

    /// Create a new session owned by `user_id` and return it.
    pub fn create(&self, user_id: &str) -> Session {
        let session = Session::new(user_id);
        let id = session.id;
        self.sessions.write().insert(id, session.clone());
        self.mark_dirty(id);
        session
    }

    /// Insert a fully constructed session into the store.
    pub fn insert(&self, mut session: Session) -> Session {
        session.recompute_capabilities();
        let id = session.id;
        self.sessions.write().insert(id, session.clone());
        self.mark_dirty(id);
        session
    }

    /// Get a session by ID (read-only, no side effects).
    ///
    /// Does NOT update `last_accessed` and does NOT mark dirty.
    /// Use this for ownership checks, status reads, and any path
    /// that doesn't need to extend the session's expiry window.
    pub fn get(&self, id: Uuid) -> Option<Session> {
        self.sessions.read().get(&id).cloned()
    }

    /// Get a session if it exists AND belongs to `user_id`.
    ///
    /// Returns `None` if the session doesn't exist.
    /// Returns `Err(())` if the session exists but belongs to a different user.
    /// Read-only — does not mark dirty.
    pub fn get_if_owned(&self, id: Uuid, user_id: &str) -> Result<Session, Option<()>> {
        match self.sessions.read().get(&id) {
            Some(session) if session.user_id == user_id => Ok(session.clone()),
            Some(_) => Err(Some(())), // exists but wrong owner
            None => Err(None),        // does not exist
        }
    }

    /// Touch a session, updating `last_accessed` and marking dirty.
    ///
    /// Call this after a meaningful user interaction (message send,
    /// WebSocket connect) to extend the session's expiry window.
    pub fn touch(&self, id: Uuid) {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&id) {
            session.last_accessed = Utc::now().to_rfc3339();
            drop(sessions);
            self.mark_dirty(id);
        }
    }

    /// Update a session. Updates `last_accessed`.
    pub fn update<F>(&self, id: Uuid, f: F) -> Option<Session>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(&id) {
            f(session);
            session.recompute_capabilities();
            session.last_accessed = Utc::now().to_rfc3339();
            self.mark_dirty(id);
            Some(session.clone())
        } else {
            None
        }
    }

    /// List all sessions owned by `user_id`.
    pub fn list_for_user(&self, user_id: &str) -> Vec<Session> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    /// List all sessions owned by `user_id` for a specific project.
    pub fn list_for_user_project(&self, user_id: &str, project_id: Uuid) -> Vec<Session> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.user_id == user_id && s.project_id == Some(project_id))
            .cloned()
            .collect()
    }

    /// List all sessions assigned to a specific project ID across all users.
    pub fn list_for_project(&self, project_id: Uuid) -> Vec<Session> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.project_id == Some(project_id))
            .cloned()
            .collect()
    }

    /// Delete all sessions assigned to a project ID.
    pub fn delete_project_session_set(&self, project_id: Uuid) -> usize {
        let ids: Vec<Uuid> = self
            .sessions
            .read()
            .iter()
            .filter_map(|(id, session)| (session.project_id == Some(project_id)).then_some(*id))
            .collect();

        let mut removed = 0usize;
        for id in &ids {
            if matches!(self.delete(*id), Ok(true)) {
                removed += 1;
            }
        }

        removed
    }

    /// List all session IDs.
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.sessions.read().keys().copied().collect()
    }

    /// Count active sessions.
    pub fn count(&self) -> usize {
        self.sessions.read().len()
    }

    /// Remove sessions that have not been accessed within `max_age_secs` seconds.
    /// Also removes their on-disk files.
    pub fn cleanup_expired(&self, max_age_secs: u64) {
        let now = Utc::now();
        let mut sessions = self.sessions.write();
        let before = sessions.len();

        let mut removed_ids = Vec::new();
        sessions.retain(|id, session| {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&session.last_accessed) {
                let age = now.signed_duration_since(last).num_seconds();
                if age >= max_age_secs as i64 {
                    removed_ids.push(*id);
                    return false;
                }
            }
            true
        });

        let removed = before - sessions.len();
        // Drop the lock before doing I/O.
        drop(sessions);

        // Clean up dirty set and disk files for removed sessions.
        if !removed_ids.is_empty() {
            let mut dirty = self.dirty.write();
            for id in &removed_ids {
                dirty.remove(id);
                if let Err(error) = self.delete_from_disk(*id) {
                    tracing::warn!("Failed to delete expired session file {}: {}", id, error);
                }
            }
        }

        if removed > 0 {
            tracing::info!("Session cleanup: removed {} expired session(s)", removed);
        }
    }

    /// Explicitly delete a session by ID.
    /// Removes from memory, dirty set, and disk.
    pub fn delete(&self, id: Uuid) -> std::io::Result<bool> {
        if !self.sessions.read().contains_key(&id) {
            return Ok(false);
        }

        self.delete_from_disk(id)?;
        self.sessions.write().remove(&id);
        self.dirty.write().remove(&id);
        Ok(true)
    }

    // -----------------------------------------------------------------------
    // Persistence internals
    // -----------------------------------------------------------------------

    /// Mark a session as needing a flush to disk.
    fn mark_dirty(&self, id: Uuid) {
        if self.sessions_dir.is_some() {
            self.dirty.write().insert(id);
        }
    }

    /// Flush all dirty sessions to disk. Called by the background flush task.
    ///
    /// Uses atomic write-then-rename: data goes to `{id}.msgpack.tmp` first,
    /// then is renamed to `{id}.msgpack`. This means a crash mid-write leaves
    /// the previous good copy intact.
    ///
    /// IDs are removed from the dirty set only after a successful write.
    /// This means mutations that land between snapshot and write are never lost.
    pub fn flush_dirty(&self) -> (usize, usize) {
        let sessions_dir = match &self.sessions_dir {
            Some(d) => d,
            None => return (0, 0),
        };

        // Snapshot dirty IDs without clearing — we remove only on success.
        let dirty_ids: Vec<Uuid> = { self.dirty.read().iter().copied().collect() };

        if dirty_ids.is_empty() {
            return (0, 0);
        }

        let mut flushed = 0usize;
        let mut errors = 0usize;

        // Snapshot dirty sessions under read lock, then release.
        // This prevents the read lock from being held during disk I/O.
        let session_snapshots: Vec<(Uuid, Vec<u8>)> = {
            let sessions = self.sessions.read();
            let mut snapshots = Vec::with_capacity(dirty_ids.len());
            for id in &dirty_ids {
                match sessions.get(id) {
                    Some(session) => match rmp_serde::to_vec(session) {
                        Ok(bytes) => snapshots.push((*id, bytes)),
                        Err(e) => {
                            tracing::error!("Failed to encode session {}: {}", id, e);
                            errors += 1;
                        }
                    },
                    None => {
                        // Session deleted between mark and flush — clear dirty.
                        self.dirty.write().remove(id);
                    }
                }
            }
            snapshots
        };
        // Read lock released here — mutations are unblocked during I/O.

        for (id, bytes) in &session_snapshots {
            let final_path = sessions_dir.join(format!("{}.msgpack", id));
            let tmp_path = sessions_dir.join(format!("{}.msgpack.tmp", id));

            // Write + fsync + rename for crash durability.
            let write_result = (|| -> std::io::Result<()> {
                let mut file = std::fs::File::create(&tmp_path)?;
                std::io::Write::write_all(&mut file, bytes)?;
                file.sync_all()?;
                Ok(())
            })();

            if let Err(e) = write_result {
                tracing::error!("Failed to write/fsync session {}: {}", id, e);
                errors += 1;
                continue;
            }
            if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
                tracing::error!("Failed to rename session {}: {}", id, e);
                errors += 1;
                continue;
            }
            // Success — remove from dirty set.
            self.dirty.write().remove(id);
            flushed += 1;
        }

        if flushed > 0 || errors > 0 {
            tracing::debug!("Session flush: {} written, {} errors", flushed, errors);
        }

        (flushed, errors)
    }

    /// Delete a session's file from disk.
    fn delete_from_disk(&self, id: Uuid) -> std::io::Result<()> {
        if let Some(dir) = &self.sessions_dir {
            let path = dir.join(format!("{}.msgpack", id));
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            // Also clean up any lingering tmp file.
            let tmp = dir.join(format!("{}.msgpack.tmp", id));
            if tmp.exists() {
                std::fs::remove_file(&tmp)?;
            }
        }
        Ok(())
    }

    /// Returns true if this store has disk backing enabled.
    pub fn is_persistent(&self) -> bool {
        self.sessions_dir.is_some()
    }

    /// Number of sessions currently marked dirty.
    pub fn dirty_count(&self) -> usize {
        self.dirty.read().len()
    }

    /// Snapshot all events from all sessions under a single read lock.
    ///
    /// Returns `(session_id, events)` pairs. Does NOT mark anything dirty.
    /// Use this for admin endpoints that need to aggregate events.
    pub fn snapshot_all_events(
        &self,
    ) -> Vec<(Uuid, Vec<planner_core::observability::PlannerEvent>)> {
        self.sessions
            .read()
            .iter()
            .map(|(id, s)| (*id, s.events.clone()))
            .collect()
    }

    /// Snapshot all events with project context for admin aggregations.
    ///
    /// Returns `(session_id, project_id, project_name, events)` tuples. Does
    /// NOT mark anything dirty.
    pub fn snapshot_all_events_with_context(
        &self,
    ) -> Vec<(
        Uuid,
        Option<Uuid>,
        Option<String>,
        Vec<planner_core::observability::PlannerEvent>,
    )> {
        self.sessions
            .read()
            .iter()
            .map(|(id, s)| (*id, s.project_id, s.project_name.clone(), s.events.clone()))
            .collect()
    }

    /// Return lightweight session summaries for a user, without cloning event logs.
    ///
    /// Use this for list endpoints where the full Session payload is wasteful.
    pub fn list_summaries_for_user(
        &self,
        user_id: &str,
        include_archived: bool,
    ) -> Vec<SessionSummary> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.user_id == user_id)
            .filter(|s| include_archived || !s.archived)
            .map(|s| SessionSummary {
                id: s.id,
                user_id: s.user_id.clone(),
                title: s.title.clone(),
                archived: s.archived,
                archived_at: s.archived_at.clone(),
                created_at: s.created_at.clone(),
                last_accessed: s.last_accessed.clone(),
                last_activity_at: s.last_activity_at(),
                pipeline_running: s.pipeline_running,
                intake_phase: s.intake_phase.clone(),
                interview_live_attached: s.interview_live_attached,
                project_description: s.project_description.clone(),
                project_id: s.project_id,
                project_slug: s.project_slug.clone(),
                project_name: s.project_name.clone(),
                message_count: s.messages.len(),
                event_count: s.events.len(),
                warning_count: s.warning_count(),
                error_count: s.error_count(),
                current_step: s.current_step.clone(),
                error_message: s.error_message.clone(),
                can_resume_live: s.can_resume_live,
                can_resume_checkpoint: s.can_resume_checkpoint,
                can_restart_from_description: s.can_restart_from_description,
                can_retry_pipeline: s.can_retry_pipeline,
                has_checkpoint: s.has_checkpoint,
                resume_status: s.resume_status,
                classification: s.classification.clone(),
                convergence_pct: s.belief_state.as_ref().map(|state| state.convergence_pct()),
                checkpoint_last_saved_at: s
                    .checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.last_checkpoint_at.clone()),
            })
            .collect()
    }

    /// Return lightweight session summaries for a user within one project.
    pub fn list_summaries_for_user_project(
        &self,
        user_id: &str,
        project_id: Uuid,
        include_archived: bool,
    ) -> Vec<SessionSummary> {
        self.sessions
            .read()
            .values()
            .filter(|s| s.user_id == user_id)
            .filter(|s| s.project_id == Some(project_id))
            .filter(|s| include_archived || !s.archived)
            .map(|s| SessionSummary {
                id: s.id,
                user_id: s.user_id.clone(),
                title: s.title.clone(),
                archived: s.archived,
                archived_at: s.archived_at.clone(),
                created_at: s.created_at.clone(),
                last_accessed: s.last_accessed.clone(),
                last_activity_at: s.last_activity_at(),
                pipeline_running: s.pipeline_running,
                intake_phase: s.intake_phase.clone(),
                interview_live_attached: s.interview_live_attached,
                project_description: s.project_description.clone(),
                project_id: s.project_id,
                project_slug: s.project_slug.clone(),
                project_name: s.project_name.clone(),
                message_count: s.messages.len(),
                event_count: s.events.len(),
                warning_count: s.warning_count(),
                error_count: s.error_count(),
                current_step: s.current_step.clone(),
                error_message: s.error_message.clone(),
                can_resume_live: s.can_resume_live,
                can_resume_checkpoint: s.can_resume_checkpoint,
                can_restart_from_description: s.can_restart_from_description,
                can_retry_pipeline: s.can_retry_pipeline,
                has_checkpoint: s.has_checkpoint,
                resume_status: s.resume_status,
                classification: s.classification.clone(),
                convergence_pct: s.belief_state.as_ref().map(|state| state.convergence_pct()),
                checkpoint_last_saved_at: s
                    .checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.last_checkpoint_at.clone()),
            })
            .collect()
    }
} // impl SessionStore

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_question_prompt(question: &str) -> PromptEnvelope {
        PromptEnvelope {
            prompt_id: "prompt-test-question".into(),
            kind: PromptKind::QuestionBatch,
            title: "Continue interview".into(),
            instructions: None,
            origin_category_id: None,
            category_path: Vec::new(),
            items: vec![PromptItem {
                item_id: "item-1".into(),
                kind: PromptItemKind::Discovery,
                target_dimension: Some(
                    planner_schemas::artifacts::socratic::Dimension::Stakeholders,
                ),
                section_ref: None,
                text: question.into(),
                options: Vec::new(),
                response_mode: PromptResponseMode::SingleSelectWithCustomText,
                required: false,
                priority: 100,
                dependency_item_ids: Vec::new(),
            }],
            draft_snapshot: None,
            required_item_ids: Vec::new(),
            allow_partial_submit: true,
            ui_hints: PromptUiHints {
                preferred_layout: PromptPreferredLayout::Cards,
                show_draft_sidebar: false,
            },
            based_on_turn: 0,
            created_at: "2026-03-08T00:00:00Z".into(),
        }
    }

    #[test]
    fn session_creation() {
        let session = Session::new("dev|local");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "system");
        assert_eq!(session.stages.len(), 12);
        assert!(!session.pipeline_running);
        assert_eq!(session.user_id, "dev|local");
        assert!(session.title.is_none());
        assert!(!session.archived);
        assert_eq!(session.events.len(), 0);
        assert!(session.current_step.is_none());
        assert!(session.error_message.is_none());
        assert_eq!(session.resume_status, ResumeStatus::ReadyToStart);
        assert!(!session.interview_live_attached);
        assert!(!session.can_resume_live);
        assert!(!session.can_resume_checkpoint);
        assert!(!session.has_checkpoint);
        assert!(session.socratic_run_id.is_none());
        assert!(session.checkpoint.is_none());
    }

    #[test]
    fn session_capabilities_follow_phase_truth() {
        let store = SessionStore::new();
        let created = store.create("dev|local");
        let id = created.id;

        let waiting = store.get(id).unwrap();
        assert_eq!(waiting.resume_status, ResumeStatus::ReadyToStart);
        assert!(!waiting.can_resume_live);

        let interviewing_attached = store
            .update(id, |s| {
                s.intake_phase = "interviewing".into();
                s.project_description = Some("Build timer".into());
                s.interview_live_attached = true;
            })
            .unwrap();
        assert_eq!(
            interviewing_attached.resume_status,
            ResumeStatus::InterviewAttached
        );
        assert!(interviewing_attached.interview_live_attached);
        assert!(!interviewing_attached.can_resume_live);
        assert!(!interviewing_attached.can_resume_checkpoint);

        let interviewing_live_detached = store
            .update(id, |s| {
                s.intake_phase = "interviewing".into();
                s.project_description = Some("Build timer".into());
                s.interview_live_attached = false;
            })
            .unwrap();
        assert_eq!(
            interviewing_live_detached.resume_status,
            ResumeStatus::LiveAttachAvailable
        );
        assert!(interviewing_live_detached.can_resume_live);
        assert!(!interviewing_live_detached.can_resume_checkpoint);

        let interviewing_restart = store
            .update(id, |s| {
                s.intake_phase = "interviewing".into();
                s.project_description = Some("Build timer".into());
                s.interview_runtime_active = false;
                s.interview_live_attached = false;
            })
            .unwrap();
        assert_eq!(
            interviewing_restart.resume_status,
            ResumeStatus::InterviewRestartOnly
        );
        assert!(!interviewing_restart.can_resume_live);
        assert!(interviewing_restart.can_restart_from_description);

        let interviewing_unknown = store
            .update(id, |s| {
                s.intake_phase = "interviewing".into();
                s.project_description = None;
                s.has_checkpoint = false;
                s.interview_runtime_active = false;
                s.interview_live_attached = false;
            })
            .unwrap();
        assert_eq!(
            interviewing_unknown.resume_status,
            ResumeStatus::InterviewResumeUnknown
        );
        assert!(!interviewing_unknown.can_resume_live);
        assert!(!interviewing_unknown.can_restart_from_description);

        let interviewing_checkpoint = store
            .update(id, |s| {
                s.intake_phase = "interviewing".into();
                s.project_description = Some("Build timer".into());
                s.ensure_checkpoint();
                s.interview_runtime_active = false;
                s.interview_live_attached = false;
            })
            .unwrap();
        assert_eq!(
            interviewing_checkpoint.resume_status,
            ResumeStatus::InterviewCheckpointResumable
        );
        assert!(interviewing_checkpoint.has_checkpoint);
        assert!(interviewing_checkpoint.can_resume_checkpoint);

        let live_attach = store
            .update(id, |s| {
                s.intake_phase = "pipeline_running".into();
                s.project_description = Some("Build timer".into());
            })
            .unwrap();
        assert_eq!(live_attach.resume_status, ResumeStatus::LiveAttachAvailable);
        assert!(live_attach.can_resume_live);
        assert!(!live_attach.can_restart_from_description);

        let retryable_failure = store
            .update(id, |s| {
                s.intake_phase = "error".into();
                s.project_description = Some("Build timer".into());
                s.stages[2].status = "failed".into();
                s.pipeline_running = false;
            })
            .unwrap();
        assert!(retryable_failure.can_retry_pipeline);
    }

    #[test]
    fn session_title_falls_back_to_description() {
        let mut session = Session::new("dev|local");
        session.project_description =
            Some("A multi-tenant SaaS dashboard for field operations and approvals".into());
        session.ensure_title_from_description();

        assert_eq!(
            session.title.as_deref(),
            Some("A multi-tenant SaaS dashboard for field operations and approvals")
        );
    }

    #[test]
    fn duplicate_for_branch_copies_saved_context_without_live_runtime() {
        let mut session = Session::new("dev|local");
        session.set_title(Some("Operations Console".into()));
        session.project_description = Some("Build an ops console".into());
        session.intake_phase = "interviewing".into();
        session.ensure_checkpoint().current_prompt =
            Some(test_question_prompt("Who approves changes?"));
        session.interview_live_attached = true;
        session.interview_runtime_active = true;
        session.record_event(planner_core::observability::PlannerEvent::info(
            planner_core::observability::EventSource::System,
            "session.test",
            "source event",
        ));

        let duplicate = session.duplicate_for_branch(None);

        assert_ne!(duplicate.id, session.id);
        assert_eq!(
            duplicate.title.as_deref(),
            Some("Operations Console (Copy)")
        );
        assert_eq!(duplicate.project_description, session.project_description);
        assert_eq!(duplicate.intake_phase, "interviewing");
        assert!(!duplicate.interview_live_attached);
        assert!(!duplicate.interview_runtime_active);
        assert!(duplicate.can_resume_checkpoint);
        assert!(duplicate.events.is_empty());
        assert!(duplicate.messages.iter().any(|message| message
            .content
            .contains("Duplicated from \"Operations Console\"")));
        assert_ne!(duplicate.socratic_run_id, session.socratic_run_id);
    }

    #[test]
    fn list_summaries_can_hide_archived_sessions() {
        let store = SessionStore::new();
        let active = store.create("dev|local");
        let archived = store.create("dev|local");
        store.update(archived.id, |session| {
            session.set_archived(true);
        });

        let visible = store.list_summaries_for_user("dev|local", false);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, active.id);

        let all = store.list_summaries_for_user("dev|local", true);
        assert_eq!(all.len(), 2);
        assert!(all
            .iter()
            .any(|session| session.id == archived.id && session.archived));
    }

    #[test]
    fn session_summaries_include_activity_and_attention_metadata() {
        let store = SessionStore::new();
        let session = store.create("dev|local");
        let id = session.id;

        store
            .update(id, |s| {
                let classification = planner_schemas::artifacts::socratic::DomainClassification {
                    project_type: planner_schemas::artifacts::socratic::ProjectType::WebApp,
                    complexity: planner_schemas::artifacts::socratic::ComplexityTier::Standard,
                    detected_signals: vec!["browser".into()],
                    required_dimensions: Vec::new(),
                };

                s.project_description = Some("Build timer".into());
                s.intake_phase = "interviewing".into();
                s.classification = Some(classification.clone());
                s.belief_state = Some(
                    planner_schemas::artifacts::socratic::RequirementsBeliefState::from_classification(
                        &classification,
                    ),
                );
                s.current_step = Some("draft.generate".into());
                s.error_message = Some("spec generation failed".into());
                s.messages[0].timestamp = "2026-03-06T12:00:00Z".into();
                s.messages.push(SessionMessage {
                    id: Uuid::new_v4(),
                    role: "user".into(),
                    content: "Build timer".into(),
                    timestamp: "2026-03-06T12:01:00Z".into(),
                });
                s.events = vec![
                    planner_core::observability::PlannerEvent {
                        id: Uuid::new_v4(),
                        timestamp: chrono::DateTime::parse_from_rfc3339("2026-03-06T12:03:00Z")
                            .unwrap()
                            .with_timezone(&chrono::Utc),
                        level: planner_core::observability::EventLevel::Warn,
                        source: planner_core::observability::EventSource::Pipeline,
                        session_id: Some(id),
                        step: Some("pipeline.warn".into()),
                        message: "retry suggested".into(),
                        duration_ms: None,
                        metadata: serde_json::Value::Null,
                    },
                    planner_core::observability::PlannerEvent {
                        id: Uuid::new_v4(),
                        timestamp: chrono::DateTime::parse_from_rfc3339("2026-03-06T12:04:00Z")
                            .unwrap()
                            .with_timezone(&chrono::Utc),
                        level: planner_core::observability::EventLevel::Error,
                        source: planner_core::observability::EventSource::Pipeline,
                        session_id: Some(id),
                        step: Some("pipeline.error".into()),
                        message: "spec generation failed".into(),
                        duration_ms: None,
                        metadata: serde_json::Value::Null,
                    },
                ];
                s.ensure_checkpoint().last_checkpoint_at = "2026-03-06T12:05:00Z".into();
            })
            .unwrap();

        {
            let mut sessions = store.sessions.write();
            let stored = sessions.get_mut(&id).expect("session should exist");
            stored.created_at = "2026-03-06T11:59:00Z".into();
            stored.last_accessed = "2026-03-06T12:02:00Z".into();
        }

        let summaries = store.list_summaries_for_user("dev|local", true);
        let summary = summaries
            .into_iter()
            .find(|candidate| candidate.id == id)
            .expect("session summary should exist");

        assert_eq!(summary.message_count, 2);
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.current_step.as_deref(), Some("draft.generate"));
        assert_eq!(
            summary.checkpoint_last_saved_at.as_deref(),
            Some("2026-03-06T12:05:00Z")
        );
        assert_eq!(summary.last_activity_at, "2026-03-06T12:05:00Z");
        assert!(summary.classification.is_some());
        assert_eq!(summary.convergence_pct, Some(1.0));
    }

    #[test]
    fn session_add_message() {
        let mut session = Session::new("dev|local");
        let msg = session.add_message("user", "Build me a widget");

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Build me a widget");
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn session_store_crud() {
        let store = SessionStore::new();

        // Create
        let session = store.create("user1");
        let id = session.id;
        assert_eq!(store.count(), 1);
        assert_eq!(session.user_id, "user1");

        // Get
        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.user_id, "user1");

        // Update
        let updated = store
            .update(id, |s| {
                s.add_message("user", "Hello");
                s.pipeline_running = true;
            })
            .unwrap();
        assert_eq!(updated.messages.len(), 2);
        assert!(updated.pipeline_running);

        // List
        let ids = store.list_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&id));
    }

    #[test]
    fn session_store_list_for_user() {
        let store = SessionStore::new();

        store.create("user_a");
        store.create("user_a");
        store.create("user_b");

        let user_a_sessions = store.list_for_user("user_a");
        assert_eq!(user_a_sessions.len(), 2);

        let user_b_sessions = store.list_for_user("user_b");
        assert_eq!(user_b_sessions.len(), 1);

        let user_c_sessions = store.list_for_user("user_c");
        assert_eq!(user_c_sessions.len(), 0);
    }

    #[test]
    fn session_store_delete_project_session_set() {
        let store = SessionStore::new();
        let project_id = Uuid::new_v4();
        let other_project_id = Uuid::new_v4();

        let session_a = store.create("user_a");
        let session_b = store.create("user_b");
        let session_c = store.create("user_c");

        store.update(session_a.id, |s| s.project_id = Some(project_id));
        store.update(session_b.id, |s| s.project_id = Some(project_id));
        store.update(session_c.id, |s| s.project_id = Some(other_project_id));

        let removed = store.delete_project_session_set(project_id);
        assert_eq!(removed, 2);
        assert!(store.get(session_a.id).is_none());
        assert!(store.get(session_b.id).is_none());
        assert!(store.get(session_c.id).is_some());
    }

    #[test]
    fn session_store_delete_errors_when_disk_removal_fails() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_session_delete_err_{}", Uuid::new_v4()));
        let store = SessionStore::open(&data_dir).unwrap();
        let session = store.create("user_a");
        let (flushed, errors) = store.flush_dirty();
        assert_eq!(flushed, 1);
        assert_eq!(errors, 0);

        let session_path = data_dir
            .join("sessions")
            .join(format!("{}.msgpack", session.id));
        std::fs::remove_file(&session_path).unwrap();
        std::fs::create_dir_all(&session_path).unwrap();

        let error = store.delete(session.id).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::IsADirectory);
        assert!(store.get(session.id).is_some());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn session_store_get_missing() {
        let store = SessionStore::new();
        assert!(store.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn session_store_update_missing() {
        let store = SessionStore::new();
        let result = store.update(Uuid::new_v4(), |_| {});
        assert!(result.is_none());
    }

    #[test]
    fn session_serialization() {
        use planner_core::observability::{EventSource, PlannerEvent};
        let mut session = Session::new("auth0|abc123");
        // Add an event so we can verify round-trip.
        let event = PlannerEvent::info(EventSource::Pipeline, "test.step", "Test event");
        session.record_event(event);
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.stages.len(), 12);
        assert_eq!(deserialized.user_id, "auth0|abc123");
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.events[0].message, "Test event");
        assert_eq!(deserialized.current_step.as_deref(), Some("test.step"));
    }

    #[test]
    fn session_helper_methods() {
        use planner_core::observability::{EventSource, PlannerEvent};
        let mut session = Session::new("auth0|test");

        // Initially zero.
        assert_eq!(session.llm_call_count(), 0);
        assert_eq!(session.llm_total_latency_ms(), 0);
        assert_eq!(session.error_count(), 0);

        // Record an LLM complete event.
        let llm_event = PlannerEvent::info(EventSource::LlmRouter, "llm.call.complete", "LLM done")
            .with_duration(123);
        session.record_event(llm_event);
        assert_eq!(session.llm_call_count(), 1);
        assert_eq!(session.llm_total_latency_ms(), 123);
        assert_eq!(session.current_step.as_deref(), Some("llm.call.complete"));

        // Record an error event.
        let err_event =
            PlannerEvent::error(EventSource::Pipeline, "pipeline.error", "Something failed");
        session.record_event(err_event);
        assert_eq!(session.error_count(), 1);
        assert_eq!(session.error_message.as_deref(), Some("Something failed"));

        // LLM start event should NOT count toward llm_call_count.
        let start_event = PlannerEvent::info(EventSource::LlmRouter, "llm.call.start", "Starting");
        session.record_event(start_event);
        assert_eq!(session.llm_call_count(), 1); // still 1
    }

    #[test]
    fn session_record_event_updates_pipeline_stages_from_event_metadata() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut session = Session::new("auth0|events");

        session.record_event(
            PlannerEvent::info(
                EventSource::Pipeline,
                "pipeline.stage.started",
                "Compile stage started",
            )
            .with_metadata(serde_json::json!({
                "stage": "Compile",
                "terminal": false,
            })),
        );
        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Compile")
                .map(|stage| stage.status.as_str()),
            Some("running")
        );
        assert_eq!(session.intake_phase, "pipeline_running");
        assert!(session.pipeline_running);

        session.record_event(
            PlannerEvent::info(
                EventSource::Pipeline,
                "pipeline.stage.completed",
                "Compile stage completed",
            )
            .with_metadata(serde_json::json!({
                "stage": "Compile",
                "terminal": false,
            })),
        );
        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Compile")
                .map(|stage| stage.status.as_str()),
            Some("complete")
        );

        session.record_event(
            PlannerEvent::error(
                EventSource::Pipeline,
                "pipeline.stage.failed",
                "Validate stage failed, retry planned",
            )
            .with_metadata(serde_json::json!({
                "stage": "Validate",
                "terminal": false,
                "retry_planned": true,
            })),
        );
        assert_eq!(session.intake_phase, "pipeline_running");
        assert!(session.pipeline_running);
        assert!(session.error_message.is_none());

        session.record_event(
            PlannerEvent::error(
                EventSource::Pipeline,
                "pipeline.stage.failed",
                "Validate stage failed permanently",
            )
            .with_metadata(serde_json::json!({
                "stage": "Validate",
                "terminal": true,
                "retry_planned": false,
            })),
        );
        assert_eq!(session.intake_phase, "error");
        assert!(!session.pipeline_running);
        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Validate")
                .map(|stage| stage.status.as_str()),
            Some("failed")
        );
    }

    #[test]
    fn session_record_event_marks_complete_when_git_stage_completes() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut session = Session::new("auth0|events");
        session.pipeline_running = true;
        session.intake_phase = "pipeline_running".into();

        session.record_event(
            PlannerEvent::info(
                EventSource::Pipeline,
                "pipeline.stage.completed",
                "Git stage completed",
            )
            .with_metadata(serde_json::json!({
                "stage": "Git",
                "terminal": false,
            })),
        );

        assert_eq!(session.intake_phase, "complete");
        assert!(!session.pipeline_running);
        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Git")
                .map(|stage| stage.status.as_str()),
            Some("complete")
        );
    }

    #[test]
    fn session_record_event_derives_stage_from_message_when_metadata_is_missing() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut session = Session::new("auth0|events");
        session.pipeline_running = true;
        session.intake_phase = "pipeline_running".into();

        session.record_event(PlannerEvent::info(
            EventSource::Pipeline,
            "pipeline.stage.started",
            "Factory stage started (attempt 1/3)",
        ));

        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Factory")
                .map(|stage| stage.status.as_str()),
            Some("running")
        );
    }

    #[test]
    fn session_record_event_falls_back_to_running_stage_for_missing_metadata() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut session = Session::new("auth0|events");
        if let Some(stage) = session
            .stages
            .iter_mut()
            .find(|stage| stage.name == "Compile")
        {
            stage.status = "running".into();
        }
        session.pipeline_running = true;
        session.intake_phase = "pipeline_running".into();

        session.record_event(
            PlannerEvent::error(
                EventSource::Pipeline,
                "pipeline.stage.failed",
                "Stage failed; retry planned",
            )
            .with_metadata(serde_json::json!({
                "terminal": false,
                "retry_planned": true,
            })),
        );

        assert_eq!(
            session
                .stages
                .iter()
                .find(|stage| stage.name == "Compile")
                .map(|stage| stage.status.as_str()),
            Some("failed")
        );
        assert_eq!(session.intake_phase, "pipeline_running");
        assert!(session.pipeline_running);
    }

    #[test]
    fn pipeline_stage_info_serde() {
        let stage = PipelineStageInfo {
            name: "Intake".into(),
            status: "running".into(),
        };
        let json = serde_json::to_string(&stage).unwrap();
        assert!(json.contains("Intake"));
        assert!(json.contains("running"));
    }

    #[test]
    fn cleanup_expired_removes_old_sessions() {
        let store = SessionStore::new();

        // Create two sessions
        let s1 = store.create("user_cleanup_1");
        let s2 = store.create("user_cleanup_2");

        // Manually back-date s1's last_accessed to over 1 hour ago
        {
            let old_time = (chrono::Utc::now() - chrono::Duration::seconds(7200)).to_rfc3339();
            let mut sessions = store.sessions.write();
            sessions.get_mut(&s1.id).unwrap().last_accessed = old_time;
        }

        assert_eq!(store.count(), 2);

        // Cleanup sessions older than 3600 seconds (1 hour)
        store.cleanup_expired(3600);

        // s1 should be removed, s2 should remain
        assert_eq!(store.sessions.read().len(), 1);
        assert!(store.sessions.read().get(&s1.id).is_none());
        assert!(store.sessions.read().get(&s2.id).is_some());
    }

    // -----------------------------------------------------------------------
    // Persistence tests — real disk I/O, simulates server restart
    // -----------------------------------------------------------------------

    fn temp_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("planner_session_test_{}", Uuid::new_v4()))
    }

    #[test]
    fn disk_backed_store_persists_across_restart() {
        let data_dir = temp_data_dir();

        let session_id;
        let user_msg_content = "Build a CLI tool for managing Docker containers";

        // --- First "server lifetime" ---
        {
            let store = SessionStore::open(&data_dir).unwrap();
            assert!(store.is_persistent());
            assert_eq!(store.count(), 0);

            let session = store.create("dev|local");
            session_id = session.id;

            // Add real messages simulating a Socratic interview.
            store.update(session_id, |s| {
                s.add_message("user", user_msg_content);
                s.add_message("planner", "What programming language would you prefer?");
                s.add_message("user", "Rust, obviously.");
                s.project_description = Some("Docker CLI manager".into());
                s.intake_phase = "interviewing".into();
            });

            // Record an event.
            store.update(session_id, |s| {
                let event = planner_core::observability::PlannerEvent::info(
                    planner_core::observability::EventSource::SocraticEngine,
                    "socratic.question.asked",
                    "Asked about programming language preference",
                );
                s.record_event(event);
            });

            assert_eq!(store.count(), 1);
            assert_eq!(store.dirty_count(), 1);

            // Flush to disk.
            let (flushed, errors) = store.flush_dirty();
            assert_eq!(flushed, 1);
            assert_eq!(errors, 0);
            assert_eq!(store.dirty_count(), 0);
        }
        // Store dropped here — simulates server shutdown.

        // --- Second "server lifetime" ---
        {
            let store = SessionStore::open(&data_dir).unwrap();

            // Verify the session survived the "restart".
            assert_eq!(store.count(), 1);

            let session = store
                .get(session_id)
                .expect("session should survive restart");
            assert_eq!(session.user_id, "dev|local");
            assert_eq!(
                session.project_description.as_deref(),
                Some("Docker CLI manager")
            );
            assert_eq!(session.intake_phase, "interviewing");

            // 1 system welcome + 3 user/planner messages = 4 total
            assert_eq!(session.messages.len(), 4);
            assert_eq!(session.messages[1].content, user_msg_content);
            assert_eq!(session.messages[1].role, "user");
            assert_eq!(
                session.messages[2].content,
                "What programming language would you prefer?"
            );
            assert_eq!(session.messages[3].content, "Rust, obviously.");

            // Event survived too.
            assert_eq!(session.events.len(), 1);
            assert_eq!(
                session.events[0].step.as_deref(),
                Some("socratic.question.asked")
            );

            // IDs list works.
            let ids = store.list_ids();
            assert_eq!(ids.len(), 1);
            assert!(ids.contains(&session_id));
        }

        // Cleanup.
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn disk_backed_store_persists_interview_checkpoint() {
        let data_dir = temp_data_dir();
        let run_id = Uuid::new_v4();
        let session_id;

        {
            let store = SessionStore::open(&data_dir).unwrap();
            let created = store.create("dev|local");
            session_id = created.id;

            store.update(session_id, |s| {
                s.socratic_run_id = Some(run_id);
                let checkpoint = s.ensure_checkpoint();
                checkpoint.current_prompt =
                    Some(test_question_prompt("What are the core user roles?"));
                checkpoint.stale_turns = 2;
                checkpoint.draft_shown_at_turn = Some(4);
                checkpoint.touch();
            });

            let (flushed, errors) = store.flush_dirty();
            assert_eq!(flushed, 1);
            assert_eq!(errors, 0);
        }

        {
            let store = SessionStore::open(&data_dir).unwrap();
            let loaded = store.get(session_id).expect("session should load");
            assert_eq!(loaded.socratic_run_id, Some(run_id));
            assert!(loaded.has_checkpoint);

            let checkpoint = loaded.checkpoint.expect("checkpoint should persist");
            assert_eq!(checkpoint.socratic_run_id, run_id);
            assert_eq!(
                checkpoint
                    .current_prompt
                    .as_ref()
                    .and_then(|prompt| prompt.items.first())
                    .map(|item| item.text.as_str()),
                Some("What are the core user roles?")
            );
            assert_eq!(checkpoint.stale_turns, 2);
            assert_eq!(checkpoint.draft_shown_at_turn, Some(4));
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn disk_backed_store_persists_deep_category_checkpoint() {
        let data_dir = temp_data_dir();
        let run_id = Uuid::new_v4();
        let session_id;

        {
            let store = SessionStore::open(&data_dir).unwrap();
            let created = store.create("dev|local");
            session_id = created.id;

            store.update(session_id, |s| {
                s.socratic_run_id = Some(run_id);
                let checkpoint = s.ensure_checkpoint();
                checkpoint.current_category_snapshot = Some(SocraticCategorySnapshot {
                    revision: "category-deep-1".into(),
                    root_category_ids: vec!["root-discovery".into()],
                    nodes: vec![
                        planner_schemas::SocraticCategoryNode {
                            category_id: "root-discovery".into(),
                            parent_category_id: None,
                            title: "Explore missing areas".into(),
                            summary: "1 area still needs discovery.".into(),
                            status: planner_schemas::SocraticCategoryStatus::Active,
                            depth: 0,
                            mapped_dimensions: Vec::new(),
                            has_children: true,
                            has_prompt_ready: false,
                            item_count_hint: 1,
                        },
                        planner_schemas::SocraticCategoryNode {
                            category_id: "root-discovery::dimension::security".into(),
                            parent_category_id: Some("root-discovery".into()),
                            title: "Security".into(),
                            summary: "Authentication model still needs definition.".into(),
                            status: planner_schemas::SocraticCategoryStatus::Ready,
                            depth: 1,
                            mapped_dimensions: vec![planner_schemas::Dimension::Security],
                            has_children: false,
                            has_prompt_ready: true,
                            item_count_hint: 1,
                        },
                    ],
                    active_category_path: vec![planner_schemas::SocraticCategoryPathEntry {
                        category_id: "root-discovery".into(),
                        title: "Explore missing areas".into(),
                    }],
                    newly_available_category_ids: vec![
                        "root-discovery::dimension::security".into(),
                    ],
                    build_ready: false,
                    build_readiness_message:
                        "Build is blocked until the remaining category is explored.".into(),
                });
                checkpoint.touch();
            });

            let (flushed, errors) = store.flush_dirty();
            assert_eq!(flushed, 1);
            assert_eq!(errors, 0);
        }

        {
            let store = SessionStore::open(&data_dir).unwrap();
            let loaded = store.get(session_id).expect("session should load");
            let checkpoint = loaded.checkpoint.expect("checkpoint should persist");
            let snapshot = checkpoint
                .current_category_snapshot
                .expect("category snapshot should persist");
            assert_eq!(checkpoint.socratic_run_id, run_id);
            assert_eq!(snapshot.revision, "category-deep-1");
            assert_eq!(snapshot.active_category_path.len(), 1);
            assert_eq!(
                snapshot
                    .active_category_path
                    .first()
                    .map(|entry| entry.category_id.as_str()),
                Some("root-discovery")
            );
            assert_eq!(snapshot.newly_available_category_ids.len(), 1);
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn interview_checkpoint_promotes_legacy_question_on_read() {
        let payload = serde_json::json!({
            "socratic_run_id": Uuid::new_v4(),
            "current_question": {
                "question": "What platform are you targeting?",
                "target_dimension": "platform",
                "quick_options": [
                    { "label": "Web", "value": "web" }
                ],
                "allow_skip": true
            },
            "last_checkpoint_at": "2026-03-08T00:00:00Z"
        });

        let checkpoint: InterviewCheckpoint =
            serde_json::from_value(payload).expect("legacy question checkpoint should decode");
        let prompt = checkpoint
            .current_prompt
            .expect("legacy question should promote to current_prompt");
        assert_eq!(prompt.kind, PromptKind::QuestionBatch);
        assert_eq!(prompt.items.len(), 1);
        assert_eq!(prompt.items[0].text, "What platform are you targeting?");
    }

    #[test]
    fn interview_checkpoint_promotes_legacy_draft_on_read() {
        let payload = serde_json::json!({
            "socratic_run_id": Uuid::new_v4(),
            "pending_draft": {
                "sections": [
                    {
                        "heading": "Goal",
                        "content": "Build a resilient task tracker",
                        "dimensions": ["goal"]
                    }
                ],
                "assumptions": [],
                "not_discussed": []
            },
            "draft_shown_at_turn": 7,
            "last_checkpoint_at": "2026-03-08T00:00:00Z"
        });

        let checkpoint: InterviewCheckpoint =
            serde_json::from_value(payload).expect("legacy draft checkpoint should decode");
        let prompt = checkpoint
            .current_prompt
            .expect("legacy draft should promote to current_prompt");
        assert_eq!(prompt.kind, PromptKind::DraftReview);
        assert!(prompt.draft_snapshot.is_some());
        assert_eq!(prompt.based_on_turn, 7);
    }

    #[test]
    fn disk_backed_store_multiple_sessions_multiple_flushes() {
        let data_dir = temp_data_dir();

        let id_a;
        let id_b;

        {
            let store = SessionStore::open(&data_dir).unwrap();

            let sa = store.create("user_alpha");
            let sb = store.create("user_beta");
            id_a = sa.id;
            id_b = sb.id;

            store.update(id_a, |s| {
                s.add_message("user", "Hello from alpha");
            });

            // First flush — both dirty.
            let (flushed, _) = store.flush_dirty();
            assert_eq!(flushed, 2);

            // Now only update B.
            store.update(id_b, |s| {
                s.add_message("user", "Hello from beta");
                s.pipeline_running = true;
            });

            // Second flush — only B is dirty.
            let (flushed, _) = store.flush_dirty();
            assert_eq!(flushed, 1);
        }

        // Reload.
        {
            let store = SessionStore::open(&data_dir).unwrap();
            assert_eq!(store.count(), 2);

            let sa = store.get(id_a).unwrap();
            assert_eq!(sa.user_id, "user_alpha");
            assert_eq!(sa.messages.len(), 2); // system + 1 user msg

            let sb = store.get(id_b).unwrap();
            assert_eq!(sb.user_id, "user_beta");
            assert_eq!(sb.messages.len(), 2);
            assert!(sb.pipeline_running);
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn disk_backed_store_cleanup_removes_files() {
        let data_dir = temp_data_dir();

        let expired_id;
        let fresh_id;

        {
            let store = SessionStore::open(&data_dir).unwrap();

            let expired = store.create("user_old");
            let fresh = store.create("user_new");
            expired_id = expired.id;
            fresh_id = fresh.id;

            // Flush both to disk.
            store.flush_dirty();

            // Back-date the expired session.
            {
                let old_time = (chrono::Utc::now() - chrono::Duration::seconds(7200)).to_rfc3339();
                store
                    .sessions
                    .write()
                    .get_mut(&expired_id)
                    .unwrap()
                    .last_accessed = old_time;
            }

            // Run cleanup.
            store.cleanup_expired(3600);

            assert_eq!(store.count(), 1);

            // Verify the file was deleted.
            let expired_path = data_dir
                .join("sessions")
                .join(format!("{}.msgpack", expired_id));
            assert!(
                !expired_path.exists(),
                "expired session file should be deleted"
            );
        }

        // Reload — only fresh session should exist.
        {
            let store = SessionStore::open(&data_dir).unwrap();
            assert_eq!(store.count(), 1);
            assert!(store.get(fresh_id).is_some());
            assert!(store.get(expired_id).is_none());
        }

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn disk_backed_store_atomic_write_safety() {
        // Verify that .tmp files don't interfere with loading.
        let data_dir = temp_data_dir();
        let sessions_dir = data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        // Write a valid session.
        let mut session = Session::new("dev|test_atomic");
        session.add_message("user", "Testing atomicity");
        let bytes = rmp_serde::to_vec(&session).unwrap();
        std::fs::write(sessions_dir.join(format!("{}.msgpack", session.id)), &bytes).unwrap();

        // Write a stale .tmp file (should be ignored on load).
        std::fs::write(
            sessions_dir.join(format!("{}.msgpack.tmp", Uuid::new_v4())),
            b"garbage data",
        )
        .unwrap();

        let store = SessionStore::open(&data_dir).unwrap();
        assert_eq!(store.count(), 1);
        let loaded = store.get(session.id).unwrap();
        assert_eq!(loaded.messages.len(), 2);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn disk_backed_store_handles_corrupt_file_gracefully() {
        let data_dir = temp_data_dir();
        let sessions_dir = data_dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();

        // Write a valid session.
        let valid = Session::new("dev|valid");
        let valid_bytes = rmp_serde::to_vec(&valid).unwrap();
        std::fs::write(
            sessions_dir.join(format!("{}.msgpack", valid.id)),
            &valid_bytes,
        )
        .unwrap();

        // Write a corrupt file.
        let corrupt_id = Uuid::new_v4();
        std::fs::write(
            sessions_dir.join(format!("{}.msgpack", corrupt_id)),
            b"this is not messagepack",
        )
        .unwrap();

        // Store should load the valid session and skip the corrupt one.
        let store = SessionStore::open(&data_dir).unwrap();
        assert_eq!(store.count(), 1);
        assert!(store.get(valid.id).is_some());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn messagepack_round_trip_fidelity() {
        // Verify that MessagePack encoding preserves all session fields
        // with the same fidelity as JSON (which is already tested elsewhere).
        use planner_core::observability::{EventSource, PlannerEvent};

        let mut session = Session::new("auth0|roundtrip");
        session.add_message("user", "Build me something complex");
        session.project_description = Some("A complex system".into());
        session.intake_phase = "interviewing".into();
        session.pipeline_running = true;

        let event = PlannerEvent::info(EventSource::LlmRouter, "llm.call.complete", "Done")
            .with_duration(456)
            .with_metadata(serde_json::json!({"model": "gemini-2.5-pro", "tokens": 1500}));
        session.record_event(event);

        let error_event = PlannerEvent::error(EventSource::Pipeline, "pipeline.fail", "Timeout");
        session.record_event(error_event);

        // Encode → decode via MessagePack.
        let bytes = rmp_serde::to_vec(&session).unwrap();
        let decoded: Session = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(decoded.id, session.id);
        assert_eq!(decoded.user_id, "auth0|roundtrip");
        assert_eq!(
            decoded.project_description.as_deref(),
            Some("A complex system")
        );
        assert_eq!(decoded.intake_phase, "interviewing");
        assert!(decoded.pipeline_running);
        assert_eq!(decoded.messages.len(), 2);
        assert_eq!(decoded.events.len(), 2);
        assert_eq!(decoded.events[0].duration_ms, Some(456));
        assert_eq!(decoded.events[0].metadata["model"], "gemini-2.5-pro");
        assert_eq!(decoded.events[0].metadata["tokens"], 1500);
        assert_eq!(
            decoded.events[1].level,
            planner_core::observability::EventLevel::Error
        );
        assert_eq!(decoded.error_message.as_deref(), Some("Timeout"));
        assert_eq!(decoded.current_step.as_deref(), Some("pipeline.fail"));
        assert_eq!(decoded.stages.len(), 12);
    }

    #[test]
    fn in_memory_store_has_no_persistence() {
        let store = SessionStore::new();
        assert!(!store.is_persistent());

        store.create("dev|local");
        assert_eq!(store.dirty_count(), 0); // Not tracked when no disk backing.

        let (flushed, errors) = store.flush_dirty();
        assert_eq!(flushed, 0);
        assert_eq!(errors, 0);
    }
}
