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

export type EventLevel = "info" | "warn" | "error";
export type EventSourceType = "socratic_engine" | "llm_router" | "factory" | "pipeline" | "system";

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

export interface SessionExportResponse {
  exported_at: string;
  session: Session & {
    messages?: Array<Record<string, unknown>>;
  };
}

export interface SessionEventsResponse {
  session_id: string;
  events: PlannerEvent[];
  count: number;
}

export interface RunListResponse {
  runs: string[];
}

export interface BlueprintExportHistoryEntry {
  export_id: string;
  kind: string;
  actor?: string | null;
  node_id?: string | null;
  node_count: number;
  edge_count: number;
  project_id?: string | null;
  project_name?: string | null;
  scope_snapshot?: Record<string, unknown> | null;
  scope_snapshot_redacted: boolean;
  scope_snapshot_redacted_fields: string[];
  retention_expires_at?: string | null;
  summary: string;
  timestamp: string;
}

export interface BlueprintExportHistoryResponse {
  entries: BlueprintExportHistoryEntry[];
  total: number;
}

export interface SnapshotEntry {
  timestamp: string;
  filename: string;
}

export interface HistoryListResponse {
  snapshots: SnapshotEntry[];
}

export interface BlueprintEventPayload {
  event_type: string;
  summary: string;
  timestamp: string;
  data: Record<string, unknown>;
}

export interface BlueprintEventsResponse {
  events: BlueprintEventPayload[];
  total: number;
}

export type DiscoverySource = "cargo_toml" | "directory_scan" | "pipeline_run" | "manual" | "code_graph_context";
export type ProposalStatus = "pending" | "accepted" | "rejected" | "merged";

export interface DiscoveryProjectScope {
  project_id: string;
  project_name?: string;
}

export interface DiscoveryNodeScope {
  project?: DiscoveryProjectScope;
  secondary: SecondaryScopeRefs;
}

export interface DiscoveryNodePayload {
  id: string;
  node_type: NodeType;
  name?: string;
  title?: string;
  label?: string;
  scenario?: string;
  scope: DiscoveryNodeScope;
}

export interface ProposedNode {
  id: string;
  node: DiscoveryNodePayload;
  source: DiscoverySource;
  reason: string;
  status: ProposalStatus;
  proposed_at: string;
  reviewed_at?: string;
  confidence: number;
  source_artifact?: string;
}

export interface ProposedEdge {
  id: string;
  edge: EdgePayload;
  source: DiscoverySource;
  reason: string;
  status: ProposalStatus;
  proposed_at: string;
  reviewed_at?: string;
  confidence: number;
  source_artifact?: string;
}

export interface DiscoveryScanResult {
  scanner: string;
  proposed_count: number;
  skipped_count: number;
  proposed_edge_count: number;
  skipped_edge_count: number;
  errors: string[];
  duration_ms: number;
}

export interface DiscoveryRunResponse {
  results: DiscoveryScanResult[];
  total_proposed: number;
  total_edge_proposed: number;
}

export interface ProposedNodesResponse {
  proposals: ProposedNode[];
  total: number;
}

export interface ProposedEdgesResponse {
  proposals: ProposedEdge[];
  total: number;
}

export interface AdminProviderInfo {
  name: string;
  binary: string;
  available: boolean;
}

export interface AdminStatusResponse {
  status: string;
  version: string;
  uptime_secs: number;
  sessions: {
    active: number;
    total_events: number;
  };
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
