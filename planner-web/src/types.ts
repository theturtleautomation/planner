// ─── API Error ──────────────────────────────────────────────────────────────

export class ApiError extends Error {
  status: number;
  details?: unknown;
  constructor(message: string, status: number, details?: unknown) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.details = details;
  }
}

// ─── Pipeline ───────────────────────────────────────────────────────────────

export type PipelineStageName =
  | 'Intake'
  | 'Chunk'
  | 'Compile'
  | 'Lint'
  | 'AR Review'
  | 'Refine'
  | 'Scenarios'
  | 'Ralph'
  | 'Graph'
  | 'Factory'
  | 'Validate'
  | 'Git';

export type StageStatus = 'pending' | 'running' | 'complete' | 'failed';

export interface PipelineStage {
  name: PipelineStageName;
  status: StageStatus;
}

// ─── Messages ────────────────────────────────────────────────────────────────

export type MessageRole = 'system' | 'user' | 'planner' | 'event';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
}

// ─── Intake Phase ────────────────────────────────────────────────────────────

/** Mirrors the server-side intake_phase field on Session. */
export type IntakePhase = 'waiting' | 'interviewing' | 'pipeline_running' | 'complete' | 'error';
export type ResumeStatus =
  | 'ready_to_start'
  | 'live_attach_available'
  | 'interview_attached'
  | 'interview_restart_only'
  | 'interview_resume_unknown'
  | 'interview_checkpoint_resumable';

// ─── Socratic Types ──────────────────────────────────────────────────────────

export interface QuickOption {
  label: string;
  value: string;
}

export interface SlotValue {
  value: string;
  confidence: number;
  source_turn?: number;
  source_quote?: string;
}

/** A single section in a speculative draft. */
export interface DraftSection {
  heading: string;
  content: string;
}

/** An assumption surfaced by the speculative draft. */
export interface DraftAssumption {
  dimension: string;
  assumption: string;
}

/** A detected contradiction between two dimensions. */
export interface Contradiction {
  dimension_a: string;
  value_a: string;
  dimension_b: string;
  value_b: string;
  explanation: string;
}

/** The structured belief state sent over WebSocket. */
export interface BeliefState {
  filled: Record<string, SlotValue>;
  uncertain: Record<string, { value: SlotValue; confidence: number }>;
  missing: string[];
  out_of_scope: string[];
  convergence_pct: number;
}

/** Domain classification produced at interview start. */
export interface Classification {
  project_type: string;
  complexity: string;
}

/** A speculative draft for user review. */
export interface SpeculativeDraft {
  sections: DraftSection[];
  assumptions: DraftAssumption[];
  not_discussed: string[];
}

export type SocraticCategoryStatus = 'pending' | 'active' | 'ready' | 'complete' | 'blocked';

export interface SocraticCategoryPathEntry {
  category_id: string;
  title: string;
}

export interface SocraticCategoryNode {
  category_id: string;
  parent_category_id?: string | null;
  title: string;
  summary: string;
  status: SocraticCategoryStatus;
  depth: number;
  mapped_dimensions: Array<string | Record<string, unknown>>;
  has_children: boolean;
  has_prompt_ready: boolean;
  item_count_hint: number;
}

export interface SocraticCategorySnapshot {
  revision: string;
  root_category_ids: string[];
  nodes: SocraticCategoryNode[];
  active_category_path: SocraticCategoryPathEntry[];
  newly_available_category_ids: string[];
  build_ready: boolean;
  build_readiness_message: string;
}

export interface SocraticWorkspaceItemPreview {
  item_id: string;
  kind: PromptItemKind;
  text: string;
}

export interface SocraticWorkspaceGroup {
  category_id: string;
  title: string;
  summary: string;
  status: SocraticCategoryStatus;
  question_count: number;
  preview_items: SocraticWorkspaceItemPreview[];
  is_focused: boolean;
  is_new: boolean;
}

export interface SocraticWorkspaceSnapshot {
  category_snapshot: SocraticCategorySnapshot;
  groups: SocraticWorkspaceGroup[];
  focused_category_id?: string | null;
  branch_notice?: string | null;
}

export type PromptKind =
  | 'question_batch'
  | 'verification_batch'
  | 'contradiction_batch'
  | 'draft_review';

export type PromptItemKind = 'discovery' | 'verification' | 'contradiction' | 'draft_section';

export type PromptResponseMode = 'single_select_with_custom_text';

export type ViewportClass = 'mobile' | 'tablet' | 'desktop';

export interface PromptDirectEffect {
  type: 'set_dimension_value' | 'mark_dimension_uncertain' | 'mark_dimension_out_of_scope';
  dimension: string | Record<string, unknown>;
  value?: string;
}

export interface PromptOption {
  option_id: string;
  label: string;
  semantic_value: string;
  direct_effect?: PromptDirectEffect | null;
}

export interface PromptItem {
  item_id: string;
  kind: PromptItemKind;
  target_dimension?: string | Record<string, unknown> | null;
  section_ref?: string | null;
  text: string;
  options: PromptOption[];
  response_mode: PromptResponseMode;
  required: boolean;
  priority: number;
  dependency_item_ids: string[];
}

export interface PromptUiHints {
  preferred_layout: 'cards' | 'review';
  show_draft_sidebar: boolean;
}

export interface PromptEnvelope {
  prompt_id: string;
  kind: PromptKind;
  title: string;
  instructions?: string | null;
  origin_category_id?: string | null;
  category_path: SocraticCategoryPathEntry[];
  items: PromptItem[];
  draft_snapshot?: SpeculativeDraft | null;
  required_item_ids: string[];
  allow_partial_submit: boolean;
  ui_hints: PromptUiHints;
  based_on_turn: number;
  created_at: string;
}

export interface PromptAnswer {
  item_id: string;
  selected_option_id?: string | null;
  custom_text?: string | null;
  skipped?: boolean;
}

export interface PromptResponse {
  prompt_id: string;
  answers: PromptAnswer[];
  submitted_at: string;
  client_context?: {
    viewport_class?: ViewportClass;
  };
}

export interface UiCapabilities {
  viewport_class: ViewportClass;
  max_visible_items: number;
  supports_split_draft_view: boolean;
}

export interface CheckpointContradiction {
  dimension_a: string | Record<string, unknown>;
  value_a: string;
  dimension_b: string | Record<string, unknown>;
  value_b: string;
  explanation: string;
  resolved?: boolean;
}

export interface InterviewCheckpoint {
  socratic_run_id: string;
  classification?: Classification | null;
  belief_state?: BeliefState | null;
  current_prompt?: PromptEnvelope | null;
  current_category_snapshot?: SocraticCategorySnapshot | null;
  contradictions: CheckpointContradiction[];
  stale_turns: number;
  draft_shown_at_turn?: number | null;
  last_checkpoint_at: string;
}

// ─── Observability ──────────────────────────────────────────────────────────

export type EventLevel = 'info' | 'warn' | 'error';
export type EventSourceType = 'socratic_engine' | 'llm_router' | 'factory' | 'pipeline' | 'system';

export interface PlannerEvent {
  id: string;
  timestamp: string;
  level: EventLevel;
  source: EventSourceType;
  session_id?: string;
  step?: string;
  message: string;
  duration_ms?: number;
  metadata: Record<string, unknown>;
}

// ─── Session ─────────────────────────────────────────────────────────────────

export interface Session {
  id: string;
  user_id?: string;
  title?: string | null;
  archived: boolean;
  archived_at?: string | null;
  messages: ChatMessage[];
  stages: PipelineStage[];
  pipeline_running: boolean;
  intake_phase: IntakePhase;
  interview_live_attached: boolean;
  can_resume_live: boolean;
  can_resume_checkpoint: boolean;
  can_restart_from_description: boolean;
  can_retry_pipeline: boolean;
  has_checkpoint: boolean;
  resume_status: ResumeStatus;
  belief_state?: BeliefState | null;
  classification?: Classification | null;
  socratic_run_id?: string | null;
  checkpoint?: InterviewCheckpoint | null;
  project_description?: string | null;
  project_id?: string | null;
  project_slug?: string | null;
  project_name?: string | null;
  run_ids?: string[];
  events?: PlannerEvent[];
  current_step?: string | null;
  error_message?: string | null;
}

export interface SessionSummary {
  id: string;
  user_id: string;
  title?: string | null;
  archived: boolean;
  archived_at?: string | null;
  created_at: string;
  last_accessed: string;
  last_activity_at: string;
  pipeline_running: boolean;
  intake_phase: IntakePhase;
  interview_live_attached: boolean;
  project_description?: string | null;
  project_id?: string | null;
  project_slug?: string | null;
  project_name?: string | null;
  message_count: number;
  event_count: number;
  warning_count: number;
  error_count: number;
  current_step?: string | null;
  error_message?: string | null;
  can_resume_live: boolean;
  can_resume_checkpoint: boolean;
  can_restart_from_description: boolean;
  can_retry_pipeline: boolean;
  has_checkpoint: boolean;
  resume_status: ResumeStatus;
  classification?: Classification | null;
  convergence_pct?: number | null;
  checkpoint_last_saved_at?: string | null;
}

// ─── API Responses ───────────────────────────────────────────────────────────

export interface HealthResponse {
  status: string;
  version: string;
  sessions_active: number;
}

export interface CreateSessionResponse {
  session: Session;
}

export interface GetSessionResponse {
  session: Session;
}

export interface SessionEventsResponse {
  session_id: string;
  events: PlannerEvent[];
  count: number;
}

export interface SessionExportResponse {
  exported_at: string;
  session: Session;
}

export interface ListSessionsResponse {
  sessions: SessionSummary[];
}

export interface Project {
  id: string;
  slug: string;
  name: string;
  description?: string | null;
  owner_user_id: string;
  team_label?: string | null;
  created_at: string;
  updated_at: string;
  archived_at?: string | null;
  legacy_scope_keys: string[];
}

export interface ProjectResponse {
  project: Project;
}

export interface ListProjectsResponse {
  projects: Project[];
}

export interface DeleteProjectResponse {
  project_id: string;
  project_name: string;
  stopped_live_sessions: number;
  stopped_pipeline_sessions: number;
  deleted_sessions: number;
  deleted_session_event_files: number;
  deleted_cxdb_runs: number;
  deleted_blueprint_nodes: number;
  unlinked_shared_blueprint_nodes: number;
  deleted_project_record: boolean;
  blueprint_events_pruned: number;
  blueprint_history_snapshots_pruned: number;
  deleted_import_jobs: number;
  deleted_import_drafts: number;
  deleted_import_managed_roots: number;
}

export type ImportProvider = 'github' | 'local';
export type ImportStatus = 'queued' | 'cloning' | 'analyzing' | 'review_pending' | 'applied' | 'failed';

export interface ProjectSourceBinding {
  project_id: string;
  provider: ImportProvider;
  canonical_ref: string;
  default_branch?: string | null;
  head_revision?: string | null;
  local_root?: string | null;
  managed_checkout: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProjectImportJob {
  id: string;
  project_id: string;
  provider: ImportProvider;
  requested_ref: string;
  status: ImportStatus;
  restored_from_job_id?: string | null;
  seed_session_id?: string | null;
  analysis_summary?: string | null;
  progress_message?: string | null;
  error_message?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ImportDraftSourceMetadata {
  provider: ImportProvider;
  canonical_ref: string;
  local_root: string;
  default_branch?: string | null;
  head_revision?: string | null;
}

export interface ProjectImportDraft {
  job_id: string;
  project_id: string;
  analysis_summary: string;
  source_metadata: ImportDraftSourceMetadata;
  discovered_nodes: unknown[];
  created_at: string;
  updated_at: string;
}

export interface ProjectImportReviewSelection {
  job_id: string;
  excluded_node_ids: string[];
  included_node_count: number;
  excluded_node_count: number;
}

export interface ProjectImportReviewNodeSummary {
  node_id: string;
  node_name: string;
  node_type: string;
  included: boolean;
}

export interface ProjectImportResponse {
  project: Project;
  import_job: ProjectImportJob;
  source_binding: ProjectSourceBinding;
  import_draft?: ProjectImportDraft | null;
  import_review_selection?: ProjectImportReviewSelection | null;
  review_nodes?: ProjectImportReviewNodeSummary[] | null;
}

export interface ProjectImportConflictResponse {
  message: string;
  project: Project;
  source_binding: ProjectSourceBinding;
}

export interface ProjectImportHistoryEntry {
  import_job: ProjectImportJob;
  source_metadata?: ImportDraftSourceMetadata | null;
  discovered_node_count?: number | null;
  effective_included_node_count?: number | null;
  effective_excluded_node_count?: number | null;
}

export interface ProjectImportDiffNodeSummary {
  node_id: string;
  node_name: string;
  node_type: string;
}

export interface ProjectImportNodeTypeCount {
  node_type: string;
  count: number;
}

export interface ProjectImportDiffSummary {
  current_job_id: string;
  compared_to_job_id: string;
  added_nodes: ProjectImportDiffNodeSummary[];
  removed_nodes: ProjectImportDiffNodeSummary[];
  added_node_types: ProjectImportNodeTypeCount[];
  removed_node_types: ProjectImportNodeTypeCount[];
  current_head_revision?: string | null;
  compared_head_revision?: string | null;
}

export interface ProjectImportHistoryResponse {
  project: Project;
  source_binding: ProjectSourceBinding;
  history: ProjectImportHistoryEntry[];
  diff_summary?: ProjectImportDiffSummary | null;
}

export interface ProjectImportHistoryComparisonResponse {
  project: Project;
  source_binding: ProjectSourceBinding;
  selected_entry: ProjectImportHistoryEntry;
  current_import_job: ProjectImportJob;
  selected_entry_uses_selection_filter: boolean;
  current_import_job_uses_selection_filter: boolean;
  diff_summary: ProjectImportDiffSummary;
}

export interface ProjectImportHistoryPairComparisonResponse {
  project: Project;
  source_binding: ProjectSourceBinding;
  baseline_entry: ProjectImportHistoryEntry;
  compared_entry: ProjectImportHistoryEntry;
  baseline_entry_uses_selection_filter: boolean;
  compared_entry_uses_selection_filter: boolean;
  diff_summary: ProjectImportDiffSummary;
}

export interface SendMessageResponse {
  user_message: ChatMessage;
  planner_message: ChatMessage;
  session: Session;
}

export interface StartSocraticResponse {
  session_id: string;
  ws_url: string;
}

export interface BeliefStateResponse {
  session_id: string;
  intake_phase: IntakePhase;
  belief_state: BeliefState | null;
}

export interface Model {
  id: string;
  name: string;
  description?: string;
}

export interface ListModelsResponse {
  models: Model[];
}

// ─── Admin ───────────────────────────────────────────────────────────────────

export interface AdminProviderInfo {
  name: string;
  binary: string;
  available: boolean;
}

export interface AdminStatusResponse {
  status: string;
  version: string;
  uptime_secs: number;
  sessions: { active: number; total_events: number };
  providers: AdminProviderInfo[];
}

export interface AdminEventEntry {
  id: string;
  timestamp: string;
  level: string;
  source: string;
  session_id?: string;
  project_id?: string;
  project_name?: string;
  step?: string;
  message: string;
  duration_ms?: number;
  metadata: Record<string, unknown>;
}

export interface AdminEventsResponse {
  events: AdminEventEntry[];
  total: number;
}

// ─── WebSocket Messages ───────────────────────────────────────────────────────

// --- Server → Client ---

export type ServerWsMessage =
  // Pipeline messages (existing)
  | { type: 'stage_update'; stage: PipelineStageName; status: StageStatus }
  | { type: 'message'; id: string; role: MessageRole; content: string; timestamp: string }
  | { type: 'pipeline_complete'; success: boolean; summary: string }
  | { type: 'error'; message: string }
  // Socratic interview messages
  | { type: 'classified'; project_type: string; complexity: string }
  | { type: 'belief_state_update'; filled: Record<string, unknown>; uncertain: Record<string, unknown>; missing: string[]; out_of_scope: string[]; convergence_pct: number }
  | { type: 'category_state'; snapshot: SocraticCategorySnapshot }
  | { type: 'workspace_state'; workspace: SocraticWorkspaceSnapshot }
  | { type: 'prompt'; prompt: PromptEnvelope }
  | { type: 'converged'; reason: string; convergence_pct: number }
  // Contradiction detection
  | { type: 'contradiction_detected'; dimension_a: string; value_a: string; dimension_b: string; value_b: string; explanation: string }
  // Observability
  | { type: 'planner_event'; id: string; timestamp: string; level: string; source: string; step?: string; message: string; duration_ms?: number; metadata: Record<string, unknown> };

// --- Client → Server ---

export type ClientWsMessage =
  // Pipeline messages (existing)
  | { type: 'user_message'; content: string }
  | { type: 'start_pipeline'; description: string }
  // Socratic interview messages
  | {
    type: 'prompt_response';
    prompt_id: string;
    answers: PromptAnswer[];
    submitted_at: string;
    client_context?: { viewport_class?: ViewportClass };
  }
  | {
    type: 'ui_capabilities';
    viewport_class: ViewportClass;
    max_visible_items: number;
    supports_split_draft_view: boolean;
  }
  | { type: 'enter_category'; category_id: string; revision: string }
  | { type: 'back_to_categories' }
  | { type: 'done' }
  // Dimension editing
  | { type: 'dimension_edit'; dimension: string; new_value: string };
