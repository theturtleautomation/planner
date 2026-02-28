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

export type MessageRole = 'system' | 'user' | 'planner';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: string;
}

// ─── Session ─────────────────────────────────────────────────────────────────

export interface Session {
  id: string;
  messages: ChatMessage[];
  stages: PipelineStage[];
  pipeline_running: boolean;
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

export interface Model {
  id: string;
  name: string;
  description?: string;
}

export interface ListModelsResponse {
  models: Model[];
}

// ─── WebSocket Messages ───────────────────────────────────────────────────────

export type ServerWsMessage =
  | { type: 'stage_update'; stage: PipelineStageName; status: StageStatus }
  | { type: 'message'; role: MessageRole; content: string }
  | { type: 'pipeline_complete'; success: boolean; summary: string }
  | { type: 'error'; message: string };

export type ClientWsMessage =
  | { type: 'user_message'; content: string }
  | { type: 'start_pipeline'; description: string };
