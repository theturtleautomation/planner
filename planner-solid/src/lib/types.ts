export type IntakePhase = "waiting" | "interviewing" | "pipeline_running" | "complete" | "error";

export interface SessionSummary {
  id: string;
  title?: string | null;
  archived: boolean;
  created_at: string;
  last_activity_at: string;
  pipeline_running: boolean;
  intake_phase: IntakePhase;
  project_description?: string | null;
  project_id?: string | null;
  project_slug?: string | null;
  project_name?: string | null;
  current_step?: string | null;
  error_message?: string | null;
}

export interface Session {
  id: string;
  title?: string | null;
  archived: boolean;
  created_at: string;
  last_activity_at: string;
  pipeline_running: boolean;
  intake_phase: IntakePhase;
  project_description?: string | null;
  project_id?: string | null;
  project_slug?: string | null;
  project_name?: string | null;
  current_step?: string | null;
  error_message?: string | null;
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

export interface CreateProjectRequest {
  name: string;
  description?: string | null;
  slug?: string | null;
  team_label?: string | null;
  legacy_scope_keys?: string[];
}

export interface ListSessionsResponse {
  sessions: SessionSummary[];
}

export interface CreateSessionResponse {
  session: Session;
}

export interface GetSessionResponse {
  session: Session;
}

export interface StartSocraticResponse {
  session_id: string;
  ws_url: string;
}

export interface PromptOption {
  option_id: string;
  label: string;
  semantic_value: string;
}

export interface PromptItem {
  item_id: string;
  kind: "discovery" | "verification" | "contradiction" | "draft_section";
  text: string;
  options: PromptOption[];
  required: boolean;
}

export interface PromptEnvelope {
  prompt_id: string;
  title: string;
  kind: "question_batch" | "verification_batch" | "contradiction_batch" | "draft_review";
  instructions?: string | null;
  origin_category_id?: string | null;
  items: PromptItem[];
  allow_partial_submit: boolean;
}

export interface PromptBankThread {
  category_id: string;
  title: string;
  summary: string;
  question_count: number;
  prompt: PromptEnvelope;
}

export interface QueuedPromptThread {
  category_id: string;
  title: string;
  summary: string;
  question_count: number;
  status: string;
}

export interface PromptBankResponse {
  session_id: string;
  active_thread_id?: string | null;
  banked_threads: PromptBankThread[];
  queued_threads: QueuedPromptThread[];
  build_ready: boolean;
  build_readiness_message?: string | null;
}

export interface PromptAnswer {
  item_id: string;
  selected_option_id?: string | null;
  custom_text?: string | null;
  skipped?: boolean;
}

export interface ClientPromptResponseMessage {
  type: "prompt_response";
  prompt_id: string;
  answers: PromptAnswer[];
  submitted_at: string;
  client_context?: {
    viewport_class?: "mobile" | "tablet" | "desktop";
  };
}
