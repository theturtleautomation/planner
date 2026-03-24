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

export type NodeType =
  | "project"
  | "decision"
  | "technology"
  | "component"
  | "constraint"
  | "pattern"
  | "quality_requirement";

export type ScopeClass = "global" | "project" | "project_contextual" | "unscoped";
export type ScopeVisibility = "shared" | "project_local" | "unscoped";
export type NodeLifecycle = "active" | "archived";
export type EdgeType =
  | "contains"
  | "decided_by"
  | "supersedes"
  | "depends_on"
  | "uses"
  | "constrains"
  | "implements"
  | "satisfies"
  | "affects";

export interface SecondaryScopeRefs {
  feature?: string;
  widget?: string;
  artifact?: string;
  component?: string;
}

export interface NodeSummary {
  id: string;
  name: string;
  node_type: string;
  status: string;
  scope_class: ScopeClass;
  scope_visibility: ScopeVisibility;
  is_shared: boolean;
  lifecycle: NodeLifecycle;
  project_id?: string;
  project_name?: string;
  secondary_scope: SecondaryScopeRefs;
  linked_project_ids: string[];
  override_source_id?: string;
  override_reason?: string;
  override_effective_from?: string;
  scope_review_deferred_reason?: string;
  scope_review_owner?: string;
  scope_review_due_at?: string;
  tags: string[];
  has_documentation: boolean;
  updated_at: string;
}

export interface EdgePayload {
  source: string;
  target: string;
  edge_type: EdgeType;
  metadata?: string;
}

export interface BlueprintResponse {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  counts: Record<string, number>;
  total_nodes: number;
  total_edges: number;
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

export type ImportStatus = "queued" | "cloning" | "analyzing" | "review_pending" | "applied" | "failed";

export interface ProjectSourceBinding {
  project_id: string;
  provider: string;
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
  provider: string;
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
  provider: string;
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
