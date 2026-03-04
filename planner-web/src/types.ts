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
  question_budget: number;
}

/** A speculative draft for user review. */
export interface SpeculativeDraft {
  sections: DraftSection[];
  assumptions: DraftAssumption[];
  not_discussed: string[];
}

// ─── Session ─────────────────────────────────────────────────────────────────

export interface Session {
  id: string;
  messages: ChatMessage[];
  stages: PipelineStage[];
  pipeline_running: boolean;
  intake_phase: IntakePhase;
  belief_state?: BeliefState | null;
  classification?: Classification | null;
  project_description?: string | null;
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

// ─── WebSocket Messages ───────────────────────────────────────────────────────

// --- Server → Client ---

export type ServerWsMessage =
  // Pipeline messages (existing)
  | { type: 'stage_update'; stage: PipelineStageName; status: StageStatus }
  | { type: 'message'; id: string; role: MessageRole; content: string; timestamp: string }
  | { type: 'pipeline_complete'; success: boolean; summary: string }
  | { type: 'error'; message: string }
  // Socratic interview messages
  | { type: 'classified'; project_type: string; complexity: string; question_budget: number }
  | { type: 'belief_state_update'; filled: Record<string, unknown>; uncertain: Record<string, unknown>; missing: string[]; out_of_scope: string[]; convergence_pct: number }
  | { type: 'question'; text: string; target_dimension: string; quick_options: QuickOption[]; allow_skip: boolean }
  | { type: 'speculative_draft'; sections: DraftSection[]; assumptions: DraftAssumption[]; not_discussed: string[] }
  | { type: 'converged'; reason: string; convergence_pct: number }
  // Contradiction detection
  | { type: 'contradiction_detected'; dimension_a: string; value_a: string; dimension_b: string; value_b: string; explanation: string };

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
