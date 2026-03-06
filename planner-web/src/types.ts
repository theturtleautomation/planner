// ─── API Error ──────────────────────────────────────────────────────────────

export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
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

export interface CheckpointQuestion {
  question: string;
  target_dimension: string | Record<string, unknown>;
  quick_options: QuickOption[];
  allow_skip: boolean;
}

export interface CheckpointDraft {
  sections: Array<{ heading: string; content: string }>;
  assumptions: Array<{
    dimension: string | Record<string, unknown>;
    assumption: string;
    confidence: number;
  }>;
  not_discussed: Array<string | Record<string, unknown>>;
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
  current_question?: CheckpointQuestion | null;
  pending_draft?: CheckpointDraft | null;
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

export interface SessionExportResponse {
  exported_at: string;
  session: Session;
}

export interface ListSessionsResponse {
  sessions: SessionSummary[];
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
  | { type: 'question'; text: string; target_dimension: string; quick_options: QuickOption[]; allow_skip: boolean }
  | { type: 'speculative_draft'; sections: DraftSection[]; assumptions: DraftAssumption[]; not_discussed: string[] }
  | { type: 'converged'; reason: string; convergence_pct: number }
  // Contradiction detection
  | { type: 'contradiction_detected'; dimension_a: string; value_a: string; dimension_b: string; value_b: string; explanation: string }
  // Draft reaction acknowledgment
  | { type: 'draft_reaction_ack'; target: string; action: string }
  // Observability
  | { type: 'planner_event'; id: string; timestamp: string; level: string; source: string; step?: string; message: string; duration_ms?: number; metadata: Record<string, unknown> };

// --- Client → Server ---

export type ClientWsMessage =
  // Pipeline messages (existing)
  | { type: 'user_message'; content: string }
  | { type: 'start_pipeline'; description: string }
  // Socratic interview messages
  | { type: 'socratic_response'; content: string }
  | { type: 'skip_question' }
  | { type: 'done' }
  // Draft reactions
  | { type: 'draft_reaction'; target: string; action: string; correction?: string }
  // Dimension editing
  | { type: 'dimension_edit'; dimension: string; new_value: string };
