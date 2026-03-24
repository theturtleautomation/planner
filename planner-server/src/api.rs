//! # API Routes — REST Endpoints for Planner Server
//!
//! Provides REST API for the Socratic Lobby web frontend.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use planner_schemas::{PromptEnvelope, SocraticCategorySnapshot};

use crate::auth::{auth_middleware, Claims};
use crate::import::{
    inspect_local_import_source, ImportAnalysisRequest, ImportDraftSourceMetadata, ImportProvider,
    ImportStatus, ProjectImportDraft, ProjectImportJob, ProjectImportReviewSelection,
    ProjectSourceBinding,
};
use crate::project::Project;
use crate::session::Session;
use crate::ws;
use crate::ws_socratic;
use crate::AppState;

fn apply_json_merge_patch(target: &mut serde_json::Value, patch: serde_json::Value) {
    match patch {
        serde_json::Value::Object(patch_map) => {
            if !target.is_object() {
                *target = serde_json::Value::Object(serde_json::Map::new());
            }

            let target_map = target
                .as_object_mut()
                .expect("target must be object after initialization");
            for (key, value) in patch_map {
                if value.is_null() {
                    target_map.remove(&key);
                    continue;
                }

                match target_map.get_mut(&key) {
                    Some(existing) => apply_json_merge_patch(existing, value),
                    None => {
                        target_map.insert(key, value);
                    }
                }
            }
        }
        other => *target = other,
    }
}

// ---------------------------------------------------------------------------
// Request/Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub sessions_active: usize,
    pub llm_providers: Vec<String>,
    pub persistence_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session: Session,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateSessionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<crate::session::SessionSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionResponse {
    pub session: Session,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionPromptBankResponse {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_thread_id: Option<String>,
    pub banked_threads: Vec<PromptBankThread>,
    pub queued_threads: Vec<QueuedPromptThread>,
    #[serde(default)]
    pub build_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_readiness_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBankThread {
    pub category_id: String,
    pub title: String,
    pub summary: String,
    pub question_count: usize,
    pub prompt: PromptEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedPromptThread {
    pub category_id: String,
    pub title: String,
    pub summary: String,
    pub question_count: usize,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProjectImportRequest {
    pub provider: ImportProvider,
    pub source_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectImportResponse {
    pub project: Project,
    pub import_job: ProjectImportJob,
    pub source_binding: ProjectSourceBinding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_draft: Option<ProjectImportDraft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_review_selection: Option<ProjectImportReviewSelectionResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_nodes: Option<Vec<ProjectImportReviewNodeSummary>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectImportConflictResponse {
    pub message: String,
    pub project: Project,
    pub source_binding: ProjectSourceBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportHistoryEntry {
    pub import_job: ProjectImportJob,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_metadata: Option<ImportDraftSourceMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovered_node_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_included_node_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_excluded_node_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportDiffNodeSummary {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportReviewSelectionResponse {
    pub job_id: Uuid,
    pub excluded_node_ids: Vec<String>,
    pub included_node_count: usize,
    pub excluded_node_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportReviewNodeSummary {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub included: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProjectImportReviewSelectionRequest {
    pub node_id: String,
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportNodeTypeCount {
    pub node_type: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectImportDiffSummary {
    pub current_job_id: String,
    pub compared_to_job_id: String,
    pub added_nodes: Vec<ProjectImportDiffNodeSummary>,
    pub removed_nodes: Vec<ProjectImportDiffNodeSummary>,
    pub added_node_types: Vec<ProjectImportNodeTypeCount>,
    pub removed_node_types: Vec<ProjectImportNodeTypeCount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_head_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compared_head_revision: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectImportHistoryResponse {
    pub project: Project,
    pub source_binding: ProjectSourceBinding,
    pub history: Vec<ProjectImportHistoryEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<ProjectImportDiffSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectImportHistoryComparisonResponse {
    pub project: Project,
    pub source_binding: ProjectSourceBinding,
    pub selected_entry: ProjectImportHistoryEntry,
    pub current_import_job: ProjectImportJob,
    #[serde(default)]
    pub selected_entry_uses_selection_filter: bool,
    #[serde(default)]
    pub current_import_job_uses_selection_filter: bool,
    pub diff_summary: ProjectImportDiffSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectImportHistoryPairComparisonResponse {
    pub project: Project,
    pub source_binding: ProjectSourceBinding,
    pub baseline_entry: ProjectImportHistoryEntry,
    pub compared_entry: ProjectImportHistoryEntry,
    #[serde(default)]
    pub baseline_entry_uses_selection_filter: bool,
    #[serde(default)]
    pub compared_entry_uses_selection_filter: bool,
    pub diff_summary: ProjectImportDiffSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteProjectResponse {
    pub project_id: String,
    pub project_name: String,
    pub stopped_live_sessions: usize,
    pub stopped_pipeline_sessions: usize,
    pub deleted_sessions: usize,
    pub deleted_session_event_files: usize,
    pub deleted_cxdb_runs: usize,
    pub deleted_blueprint_nodes: usize,
    pub unlinked_shared_blueprint_nodes: usize,
    pub deleted_project_record: bool,
    #[serde(default)]
    pub blueprint_events_pruned: usize,
    #[serde(default)]
    pub blueprint_history_snapshots_pruned: usize,
    #[serde(default)]
    pub deleted_import_jobs: usize,
    #[serde(default)]
    pub deleted_import_drafts: usize,
    #[serde(default)]
    pub deleted_import_managed_roots: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_label: Option<String>,
    #[serde(default)]
    pub legacy_scope_keys: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_scope_keys: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProjectSessionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectEventsResponse {
    pub project_id: String,
    pub events: Vec<planner_core::observability::PlannerEvent>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionExportResponse {
    pub exported_at: String,
    pub session: Session,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSessionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DuplicateSessionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub user_message: crate::session::SessionMessage,
    pub planner_message: crate::session::SessionMessage,
    pub session: Session,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub cli_binary: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

// ---------------------------------------------------------------------------
// CXDB Read API types (Change 4)
// ---------------------------------------------------------------------------

/// Metadata-only view of a single Turn for the list endpoint.
/// Full payload retrieval is deferred to when durable storage is wired.
#[derive(Debug, Serialize, Deserialize)]
pub struct TurnResponse {
    pub turn_id: String,
    pub type_id: String,
    pub timestamp: String,
    pub produced_by: String,
}

/// Response for `GET /sessions/{id}/turns`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListTurnsResponse {
    pub turns: Vec<TurnResponse>,
    pub count: usize,
}

/// Response for `GET /sessions/{id}/runs`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunListResponse {
    pub runs: Vec<String>,
}

/// Request body for starting a Socratic interview.
#[derive(Debug, Deserialize, Serialize)]
pub struct StartSocraticRequest {
    /// Initial project description from the user.
    pub description: String,
    /// Optional project reference (UUID, slug, or legacy alias).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<String>,
}

/// Response from starting a Socratic interview.
#[derive(Debug, Serialize, Deserialize)]
pub struct StartSocraticResponse {
    /// Session ID to connect to.
    pub session_id: String,
    /// WebSocket URL path for the Socratic interview.
    pub ws_url: String,
}

/// Response for the belief-state endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct BeliefStateResponse {
    pub session_id: String,
    pub intake_phase: String,
    /// The belief state JSON, or `null` if the interview hasn't started yet.
    pub belief_state: serde_json::Value,
}

/// Response for `GET /sessions/{id}/events`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionEventsResponse {
    pub session_id: String,
    pub events: Vec<planner_core::observability::PlannerEvent>,
    pub count: usize,
}

/// Query parameters for `GET /sessions/{id}/events`.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub level: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ---------------------------------------------------------------------------
// Blueprint API request/response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintResponse {
    pub nodes: Vec<planner_schemas::artifacts::blueprint::NodeSummary>,
    pub edges: Vec<EdgePayload>,
    pub counts: std::collections::HashMap<String, usize>,
    pub total_nodes: usize,
    pub total_edges: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeListResponse {
    pub nodes: Vec<planner_schemas::artifacts::blueprint::NodeSummary>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgePayload {
    pub source: String,
    pub target: String,
    pub edge_type: planner_schemas::artifacts::blueprint::EdgeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NodesQuery {
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    pub scope_class: Option<planner_schemas::artifacts::blueprint::ScopeClass>,
    pub scope_visibility: Option<planner_schemas::artifacts::blueprint::ScopeVisibility>,
    pub lifecycle: Option<planner_schemas::artifacts::blueprint::NodeLifecycle>,
    pub project_id: Option<String>,
    pub feature: Option<String>,
    pub widget: Option<String>,
    pub artifact: Option<String>,
    pub component: Option<String>,
    #[serde(default = "default_true")]
    pub include_shared: bool,
    #[serde(default)]
    pub include_global: bool,
}

fn default_true() -> bool {
    true
}

fn canonicalize_nodes_query_project_ref(query: &mut NodesQuery, state: &AppState) {
    let Some(project_ref) = query.project_id.clone() else {
        return;
    };
    if let Some(project) = state.projects.resolve_ref(&project_ref) {
        query.project_id = Some(project.id.to_string());
    }
}

#[derive(Debug, Deserialize)]
pub struct ImpactPreviewRequest {
    pub node_id: String,
    pub change_description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotEntry {
    pub timestamp: String,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryListResponse {
    pub snapshots: Vec<SnapshotEntry>,
}

/// Query parameters for `GET /blueprint/events`.
#[derive(Debug, Deserialize)]
pub struct BlueprintEventsQuery {
    /// Filter to events for a specific node.
    pub node_id: Option<String>,
    /// Maximum number of events to return (default: all).
    pub limit: Option<usize>,
}

/// Query parameters for `GET /blueprint/export-history`.
#[derive(Debug, Deserialize)]
pub struct BlueprintExportHistoryQuery {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub scope_class: Option<String>,
    #[serde(default)]
    pub feature: Option<String>,
    #[serde(default)]
    pub widget: Option<String>,
    #[serde(default)]
    pub artifact: Option<String>,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// API response for the event log.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintEventsResponse {
    pub events: Vec<BlueprintEventPayload>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintExportHistoryEntry {
    pub export_id: String,
    pub kind: planner_schemas::artifacts::blueprint::BlueprintExportKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub node_count: usize,
    pub edge_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_snapshot: Option<serde_json::Value>,
    #[serde(default)]
    pub scope_snapshot_redacted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope_snapshot_redacted_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_expires_at: Option<String>,
    pub summary: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintExportHistoryResponse {
    pub entries: Vec<BlueprintExportHistoryEntry>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct RecordBlueprintExportRequest {
    pub kind: planner_schemas::artifacts::blueprint::BlueprintExportKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub node_count: usize,
    #[serde(default)]
    pub edge_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordBlueprintExportResponse {
    pub export_id: String,
    pub recorded_at: String,
}

/// A single event in the API response.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintEventPayload {
    pub event_type: String,
    pub summary: String,
    pub timestamp: String,
    /// Full event data.
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveryScanRequest {
    pub scanners: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryScanResult {
    pub scanner: String,
    pub proposed_count: usize,
    pub skipped_count: usize,
    #[serde(default)]
    pub proposed_edge_count: usize,
    #[serde(default)]
    pub skipped_edge_count: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryRunResponse {
    pub results: Vec<DiscoveryScanResult>,
    pub total_proposed: usize,
    #[serde(default)]
    pub total_edge_proposed: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposedNodesResponse {
    pub proposals: Vec<planner_core::discovery::ProposedNode>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposedEdgesResponse {
    pub proposals: Vec<planner_core::discovery::ProposedEdge>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ProposedNodesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectProposalRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AcceptProposalRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_patch: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ImportEdgeProposalsRequest {
    pub proposals: Vec<planner_core::discovery::ImportedEdgeProposal>,
}

// ---------------------------------------------------------------------------
// Admin response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminStatusResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub sessions: AdminSessionStats,
    pub providers: Vec<AdminProviderInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminSessionStats {
    pub active: usize,
    pub total_events: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminProviderInfo {
    pub name: String,
    pub binary: String,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminEventsResponse {
    pub events: Vec<AdminEventEntry>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminEventEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub step: Option<String>,
    pub message: String,
    pub duration_ms: Option<u64>,
    pub metadata: serde_json::Value,
}

/// Query parameters for `GET /admin/events`.
#[derive(Debug, Deserialize)]
pub struct AdminEventsQuery {
    pub limit: Option<usize>,
    pub level: Option<String>,
    pub session_id: Option<String>,
}

pub fn routes(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .route("/admin/status", get(admin_status))
        .route("/admin/events", get(admin_events))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/models", get(models))
        .route("/projects/imports", post(create_project_import))
        .route("/projects/imports/{jobId}", get(get_project_import))
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/{projectRef}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .route(
            "/projects/{projectRef}/import-review",
            get(get_project_import_review).post(apply_project_import_review),
        )
        .route(
            "/projects/{projectRef}/import-review-selection",
            post(update_project_import_review_selection),
        )
        .route(
            "/projects/{projectRef}/import-state",
            get(get_project_import_state),
        )
        .route(
            "/projects/{projectRef}/import-history",
            get(get_project_import_history),
        )
        .route(
            "/projects/{projectRef}/import-history/{baseJobId}/compare/{jobId}",
            get(compare_project_import_history_entries),
        )
        .route(
            "/projects/{projectRef}/import-history/{jobId}/compare",
            get(compare_project_import_history_entry),
        )
        .route(
            "/projects/{projectRef}/import-history/{jobId}/restore",
            post(restore_project_import_history_entry),
        )
        .route(
            "/projects/{projectRef}/import-history/{jobId}/restore-for-review",
            post(restore_project_import_history_entry_for_review),
        )
        .route(
            "/projects/{projectRef}/import-history/{jobId}/restore-review-draft",
            post(restore_project_import_review_draft),
        )
        .route("/projects/{projectRef}/reimport", post(reimport_project))
        .route(
            "/projects/{projectRef}/sessions",
            get(list_project_sessions).post(create_project_session),
        )
        .route("/projects/{projectRef}/events", get(get_project_events))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session).patch(update_session))
        .route("/sessions/{id}/prompt-bank", get(get_session_prompt_bank))
        .route("/sessions/{id}/message", post(send_message))
        .route("/sessions/{id}/duplicate", post(duplicate_session))
        .route("/sessions/{id}/export", get(export_session))
        .route(
            "/sessions/{id}/restart-from-description",
            post(restart_from_description),
        )
        .route("/sessions/{id}/retry-pipeline", post(retry_pipeline))
        .route("/sessions/{id}/ws", get(ws_handler))
        .route("/sessions/{id}/socratic", post(start_socratic))
        .route("/sessions/{id}/socratic/ws", get(socratic_ws_handler))
        .route("/sessions/{id}/belief-state", get(get_belief_state))
        // CXDB read API — returns real data from durable CXDB storage
        .route("/sessions/{id}/turns", get(list_turns))
        .route("/sessions/{id}/runs", get(list_runs))
        .route("/sessions/{id}/events", get(get_session_events))
        // Blueprint API — Living System Blueprint graph management
        .route("/blueprint", get(get_blueprint))
        .route(
            "/blueprint/nodes",
            get(list_blueprint_nodes).post(create_blueprint_node),
        )
        .route(
            "/blueprint/nodes/{nodeId}",
            get(get_blueprint_node)
                .patch(update_blueprint_node)
                .delete(delete_blueprint_node),
        )
        .route(
            "/blueprint/edges",
            post(create_blueprint_edge).delete(delete_blueprint_edge),
        )
        .route("/blueprint/history", get(list_blueprint_history))
        .route("/blueprint/events", get(list_blueprint_events))
        .route(
            "/blueprint/export-history",
            get(list_blueprint_export_history),
        )
        .route("/blueprint/exports", post(record_blueprint_export))
        .route("/blueprint/impact-preview", post(impact_preview))
        .route("/blueprint/reconverge", post(reconverge_blueprint))
        .route("/blueprint/reconverge/ws", get(reconverge_ws_handler))
        .route("/blueprint/discovery/scan", post(run_discovery_scan))
        .route("/blueprint/discovery/proposals", get(list_proposals))
        .route(
            "/blueprint/discovery/component-proposals",
            get(list_proposals),
        )
        .route(
            "/blueprint/discovery/edge-proposals",
            get(list_edge_proposals),
        )
        .route(
            "/blueprint/discovery/edge-proposals/import",
            post(import_edge_proposals_endpoint),
        )
        .route(
            "/blueprint/discovery/proposals/{id}/accept",
            post(accept_proposal),
        )
        .route(
            "/blueprint/discovery/edge-proposals/{id}/accept",
            post(accept_edge_proposal),
        )
        .route(
            "/blueprint/discovery/proposals/{id}/reject",
            post(reject_proposal),
        )
        .route(
            "/blueprint/discovery/edge-proposals/{id}/reject",
            post(reject_edge_proposal),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024)) // 1 MB
        .with_state(state);

    public.merge(protected)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn project_visible_to_user(project: &Project, claims: &Claims) -> bool {
    project.owner_user_id == claims.sub
        || project.owner_user_id == crate::project::MIGRATION_OWNER_USER_ID
}

fn resolve_project_for_user(
    state: &Arc<AppState>,
    claims: &Claims,
    project_ref: &str,
) -> Result<Project, (StatusCode, Json<ErrorResponse>)> {
    let project = state.projects.resolve_ref(project_ref).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", project_ref),
                code: Some("PROJECT_NOT_FOUND".into()),
            }),
        )
    })?;

    if !project_visible_to_user(&project, claims) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Access denied".into(),
                code: None,
            }),
        ));
    }

    Ok(project)
}

fn ensure_session_project_assignment(
    state: &Arc<AppState>,
    session_id: Uuid,
    fallback_description: &str,
) -> Result<Project, String> {
    let session = state
        .sessions
        .get(session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    if let Some(project_id) = session.project_id {
        if let Some(project) = state.projects.get(project_id) {
            if session.project_slug.as_deref() != Some(project.slug.as_str())
                || session.project_name.as_deref() != Some(project.name.as_str())
            {
                state.sessions.update(session_id, |draft| {
                    draft.project_slug = Some(project.slug.clone());
                    draft.project_name = Some(project.name.clone());
                });
            }
            return Ok(project);
        }
    }

    let description = session
        .project_description
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback_description)
        .trim()
        .to_string();

    let suggested_name = if description.is_empty() {
        session
            .project_name
            .clone()
            .unwrap_or_else(|| crate::project::derive_project_name(&session.display_title()))
    } else {
        crate::project::derive_project_name(&description)
    };

    let project = state.projects.create(
        &session.user_id,
        &suggested_name,
        if description.is_empty() {
            None
        } else {
            Some(description.clone())
        },
        None,
        Vec::new(),
        Some("session_seed".into()),
    );

    state.sessions.update(session_id, |draft| {
        draft.project_id = Some(project.id);
        draft.project_slug = Some(project.slug.clone());
        draft.project_name = Some(project.name.clone());
        if draft.cxdb_project_id.is_none() {
            draft.cxdb_project_id = Some(project.id);
        }
    });

    Ok(project)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let providers: Vec<String> = state
        .llm_router
        .available_providers()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let status = if providers.is_empty() {
        "degraded"
    } else {
        "ok"
    };

    Json(HealthResponse {
        status: status.into(),
        version: "0.1.0".into(),
        sessions_active: state.sessions.count(),
        llm_providers: providers,
        persistence_enabled: state.sessions.is_persistent(),
    })
}

async fn admin_status(State(state): State<Arc<AppState>>) -> Json<AdminStatusResponse> {
    // Uptime calculation
    let uptime_secs = state.started_at.elapsed().as_secs();

    // Session stats — use snapshot to avoid marking all sessions dirty.
    let active = state.sessions.count();
    let total_events: usize = state
        .sessions
        .snapshot_all_events()
        .iter()
        .map(|(_, events)| events.len())
        .sum();

    // Provider availability
    let providers = vec![
        AdminProviderInfo {
            name: "anthropic".into(),
            binary: "claude".into(),
            available: planner_core::llm::providers::cli_available("claude"),
        },
        AdminProviderInfo {
            name: "google".into(),
            binary: "gemini".into(),
            available: planner_core::llm::providers::cli_available("gemini"),
        },
        AdminProviderInfo {
            name: "openai".into(),
            binary: "codex".into(),
            available: planner_core::llm::providers::cli_available("codex"),
        },
    ];

    let status = if providers.iter().any(|p| p.available) {
        "ok"
    } else {
        "degraded"
    };

    Json(AdminStatusResponse {
        status: status.into(),
        version: "0.1.0".into(),
        uptime_secs,
        sessions: AdminSessionStats {
            active,
            total_events,
        },
        providers,
    })
}

async fn admin_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminEventsQuery>,
) -> Result<Json<AdminEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    use planner_core::observability::{EventLevel, EventSource};

    // Parse optional session_id filter
    let filter_session_id: Option<uuid::Uuid> = match query.session_id {
        Some(ref raw) => Some(uuid::Uuid::parse_str(raw).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid session_id: not a valid UUID".into(),
                    code: None,
                }),
            )
        })?),
        None => None,
    };

    // Parse optional level filter
    let filter_level: Option<EventLevel> = match query.level.as_deref() {
        Some("info") => Some(EventLevel::Info),
        Some("warn") => Some(EventLevel::Warn),
        Some("error") => Some(EventLevel::Error),
        Some(_) => None,
        None => None,
    };

    let limit = query.limit.unwrap_or(100).min(1000);

    // Collect events from all in-memory sessions via single read-lock snapshot.
    let mut all_events: Vec<AdminEventEntry> = state
        .sessions
        .snapshot_all_events_with_context()
        .into_iter()
        .flat_map(|(session_id, project_id, project_name, events)| {
            let project_id = project_id.map(|id| id.to_string());
            let session_id = session_id.to_string();
            events.into_iter().map(move |e| AdminEventEntry {
                id: e.id.to_string(),
                timestamp: e.timestamp.to_rfc3339(),
                level: match e.level {
                    EventLevel::Info => "info".into(),
                    EventLevel::Warn => "warn".into(),
                    EventLevel::Error => "error".into(),
                },
                source: match e.source {
                    EventSource::SocraticEngine => "socratic_engine".into(),
                    EventSource::LlmRouter => "llm_router".into(),
                    EventSource::Pipeline => "pipeline".into(),
                    EventSource::Factory => "factory".into(),
                    EventSource::System => "system".into(),
                },
                session_id: Some(session_id.clone()),
                project_id: project_id.clone(),
                project_name: project_name.clone(),
                step: e.step,
                message: e.message,
                duration_ms: e.duration_ms,
                metadata: e.metadata,
            })
        })
        .filter(|e| {
            if let Some(ref lvl) = filter_level {
                let expected_level = match lvl {
                    EventLevel::Info => "info",
                    EventLevel::Warn => "warn",
                    EventLevel::Error => "error",
                };
                if e.level != expected_level {
                    return false;
                }
            }
            if let Some(ref sid) = filter_session_id {
                let expected_session_id = sid.to_string();
                match e.session_id.as_deref() {
                    Some(esid) => {
                        if esid != expected_session_id {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        })
        .collect();

    // Sort by timestamp descending (newest first)
    all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let total = all_events.len();
    let events: Vec<AdminEventEntry> = all_events.into_iter().take(limit).collect();

    Ok(Json(AdminEventsResponse { events, total }))
}

async fn models() -> Json<ModelsResponse> {
    let models = vec![
        ModelInfo {
            id: "claude-opus-4-6".into(),
            provider: "anthropic".into(),
            cli_binary: "claude".into(),
            role: "Intake Gateway, Compiler, AR Reviewer, AR Refiner".into(),
        },
        ModelInfo {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            cli_binary: "claude".into(),
            role: "Ralph Loops".into(),
        },
        ModelInfo {
            id: "claude-haiku-4-5".into(),
            provider: "anthropic".into(),
            cli_binary: "claude".into(),
            role: "Telemetry Presenter".into(),
        },
        ModelInfo {
            id: "gpt-5.3-codex".into(),
            provider: "openai".into(),
            cli_binary: "codex".into(),
            role: "Factory Worker (code generation)".into(),
        },
        ModelInfo {
            id: "gpt-5.2".into(),
            provider: "openai".into(),
            cli_binary: "codex".into(),
            role: "AR Reviewer (GPT)".into(),
        },
        ModelInfo {
            id: "gemini-3.1-pro-preview".into(),
            provider: "google".into(),
            cli_binary: "gemini".into(),
            role: "Scenario Validator, AR Reviewer (Gemini)".into(),
        },
    ];

    Json(ModelsResponse { models })
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(query): Query<ListProjectsQuery>,
) -> Json<ListProjectsResponse> {
    let mut projects = state
        .projects
        .list_for_user(&claims.sub, query.include_archived);
    projects.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.name.cmp(&right.name))
    });
    Json(ListProjectsResponse { projects })
}

fn normalize_import_source_ref(
    provider: ImportProvider,
    raw: &str,
) -> Result<(String, bool), (StatusCode, Json<ErrorResponse>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "source_ref is required".into(),
                code: Some("IMPORT_SOURCE_REQUIRED".into()),
            }),
        ));
    }

    match provider {
        ImportProvider::GitHub => {
            let normalized = trimmed
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .replace("http://github.com/", "https://github.com/");
            let Some(path) = normalized.strip_prefix("https://github.com/") else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "GitHub imports require an https://github.com/... URL".into(),
                        code: Some("INVALID_GITHUB_IMPORT_REF".into()),
                    }),
                ));
            };
            let segments = path
                .split('/')
                .filter(|segment| !segment.trim().is_empty())
                .collect::<Vec<_>>();
            if segments.len() != 2 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "GitHub imports require a repository URL like https://github.com/org/repo".into(),
                        code: Some("INVALID_GITHUB_IMPORT_REF".into()),
                    }),
                ));
            }
            let normalized = format!("https://github.com/{}/{}", segments[0], segments[1]);
            Ok((normalized, true))
        }
        ImportProvider::Local => {
            let path = PathBuf::from(trimmed);
            if !path.is_absolute() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Local imports require an absolute path".into(),
                        code: Some("INVALID_LOCAL_IMPORT_REF".into()),
                    }),
                ));
            }
            Ok((path.to_string_lossy().to_string(), false))
        }
    }
}

fn derive_import_project_name(provider: ImportProvider, canonical_ref: &str) -> String {
    let seed = match provider {
        ImportProvider::GitHub => canonical_ref
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("Imported Project"),
        ImportProvider::Local => std::path::Path::new(canonical_ref)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Imported Project"),
    };
    crate::project::derive_project_name(seed.trim_end_matches(".git"))
}

fn should_start_import_processing(provider: ImportProvider) -> bool {
    matches!(provider, ImportProvider::GitHub | ImportProvider::Local)
}

fn build_project_import_response(
    state: &Arc<AppState>,
    project: Project,
    import_job: ProjectImportJob,
    source_binding: ProjectSourceBinding,
) -> ProjectImportResponse {
    let import_draft = state.imports.get_draft(import_job.id);
    let (import_review_selection, review_nodes) = if let Some(draft) = import_draft.as_ref() {
        if matches!(import_job.status, ImportStatus::ReviewPending) {
            let selection = state.imports.get_review_selection(import_job.id).unwrap_or(
                ProjectImportReviewSelection {
                    job_id: import_job.id,
                    project_id: project.id,
                    excluded_node_ids: Vec::new(),
                    created_at: draft.created_at.clone(),
                    updated_at: draft.updated_at.clone(),
                },
            );
            let review_nodes = build_import_review_node_summaries(draft, &selection);
            let review_selection = build_import_review_selection_response(draft, &selection);
            (Some(review_selection), Some(review_nodes))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    ProjectImportResponse {
        project,
        import_job,
        source_binding,
        import_draft,
        import_review_selection,
        review_nodes,
    }
}

fn seeded_import_session_description(project_name: &str, analysis_summary: &str) -> String {
    format!(
        "Imported planning brief for {}.\n\n{}",
        project_name, analysis_summary
    )
}

fn import_review_not_found_response(project_ref: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!(
                "No reviewable import draft found for project {}",
                project_ref
            ),
            code: Some("PROJECT_IMPORT_REVIEW_NOT_FOUND".into()),
        }),
    )
}

fn import_state_not_found_response(project_ref: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("No import state found for project {}", project_ref),
            code: Some("PROJECT_IMPORT_STATE_NOT_FOUND".into()),
        }),
    )
}

fn import_history_not_found_response(project_ref: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("No import history found for project {}", project_ref),
            code: Some("PROJECT_IMPORT_HISTORY_NOT_FOUND".into()),
        }),
    )
}

fn import_restore_pending_review_response(project_ref: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: format!(
                "Project {} has a pending import review. Resolve it before restoring history.",
                project_ref
            ),
            code: Some("PROJECT_IMPORT_RESTORE_BLOCKED_BY_PENDING_REVIEW".into()),
        }),
    )
}

fn import_review_node_not_found_response(
    project_ref: &str,
    node_id: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: format!(
                "Import review node {} does not exist on the current review draft for project {}",
                node_id, project_ref
            ),
            code: Some("PROJECT_IMPORT_REVIEW_NODE_NOT_FOUND".into()),
        }),
    )
}

fn summarize_import_diff_node(
    node: &planner_schemas::artifacts::blueprint::BlueprintNode,
) -> ProjectImportDiffNodeSummary {
    ProjectImportDiffNodeSummary {
        node_id: node.id().to_string(),
        node_name: node.name().to_string(),
        node_type: node.type_name().to_string(),
    }
}

fn build_import_review_selection_response(
    draft: &ProjectImportDraft,
    selection: &ProjectImportReviewSelection,
) -> ProjectImportReviewSelectionResponse {
    let excluded = selection
        .excluded_node_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let excluded_node_count = draft
        .discovered_nodes
        .iter()
        .filter(|node| excluded.contains(&node.id().to_string()))
        .count();
    let included_node_count = draft
        .discovered_nodes
        .len()
        .saturating_sub(excluded_node_count);
    ProjectImportReviewSelectionResponse {
        job_id: selection.job_id,
        excluded_node_ids: selection.excluded_node_ids.clone(),
        included_node_count,
        excluded_node_count,
    }
}

fn build_import_review_node_summaries(
    draft: &ProjectImportDraft,
    selection: &ProjectImportReviewSelection,
) -> Vec<ProjectImportReviewNodeSummary> {
    let excluded = selection
        .excluded_node_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut nodes = draft
        .discovered_nodes
        .iter()
        .map(|node| ProjectImportReviewNodeSummary {
            node_id: node.id().to_string(),
            node_name: node.name().to_string(),
            node_type: node.type_name().to_string(),
            included: !excluded.contains(&node.id().to_string()),
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_name.cmp(&right.node_name));
    nodes
}

fn summarize_node_types(nodes: &[ProjectImportDiffNodeSummary]) -> Vec<ProjectImportNodeTypeCount> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for node in nodes {
        *counts.entry(node.node_type.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(node_type, count)| ProjectImportNodeTypeCount { node_type, count })
        .collect()
}

fn build_import_draft_diff_summary(
    current_draft: &ProjectImportDraft,
    compared_draft: &ProjectImportDraft,
) -> ProjectImportDiffSummary {
    let compared_nodes = compared_draft
        .discovered_nodes
        .iter()
        .map(|node| (node.id().to_string(), summarize_import_diff_node(node)))
        .collect::<std::collections::HashMap<_, _>>();
    let current_nodes = current_draft
        .discovered_nodes
        .iter()
        .map(|node| (node.id().to_string(), summarize_import_diff_node(node)))
        .collect::<std::collections::HashMap<_, _>>();

    let mut added_nodes = current_nodes
        .iter()
        .filter(|(node_id, _)| !compared_nodes.contains_key(*node_id))
        .map(|(_, summary)| (*summary).clone())
        .collect::<Vec<_>>();
    let mut removed_nodes = compared_nodes
        .iter()
        .filter(|(node_id, _)| !current_nodes.contains_key(*node_id))
        .map(|(_, summary)| (*summary).clone())
        .collect::<Vec<_>>();

    added_nodes.sort_by(|left, right| left.node_name.cmp(&right.node_name));
    removed_nodes.sort_by(|left, right| left.node_name.cmp(&right.node_name));

    ProjectImportDiffSummary {
        current_job_id: current_draft.job_id.to_string(),
        compared_to_job_id: compared_draft.job_id.to_string(),
        added_node_types: summarize_node_types(&added_nodes),
        removed_node_types: summarize_node_types(&removed_nodes),
        added_nodes,
        removed_nodes,
        current_head_revision: current_draft.source_metadata.head_revision.clone(),
        compared_head_revision: compared_draft.source_metadata.head_revision.clone(),
    }
}

fn build_selected_import_draft(
    state: &Arc<AppState>,
    import_job: &ProjectImportJob,
    draft: &ProjectImportDraft,
) -> ProjectImportDraft {
    let excluded = state
        .imports
        .get_review_selection(import_job.id)
        .map(|selection| {
            selection
                .excluded_node_ids
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    let mut selected_draft = draft.clone();
    selected_draft
        .discovered_nodes
        .retain(|node| !excluded.contains(&node.id().to_string()));
    selected_draft
}

fn build_selection_aware_import_draft(
    state: &Arc<AppState>,
    import_job: &ProjectImportJob,
    draft: &ProjectImportDraft,
) -> (ProjectImportDraft, bool) {
    let selection = state.imports.get_review_selection(import_job.id);
    let uses_selection_filter = selection
        .as_ref()
        .map(|selection| !selection.excluded_node_ids.is_empty())
        .unwrap_or(false);
    if uses_selection_filter {
        (build_selected_import_draft(state, import_job, draft), true)
    } else {
        (draft.clone(), false)
    }
}

fn build_import_history_selection_summary(
    state: &Arc<AppState>,
    import_job: &ProjectImportJob,
    draft: &ProjectImportDraft,
) -> Option<(usize, usize)> {
    state
        .imports
        .get_review_selection(import_job.id)
        .map(|selection| {
            let node_ids = draft
                .discovered_nodes
                .iter()
                .map(|node| node.id().to_string())
                .collect::<std::collections::HashSet<_>>();
            let excluded_count = selection
                .excluded_node_ids
                .into_iter()
                .filter(|node_id| node_ids.contains(node_id))
                .count();
            let included_count = draft.discovered_nodes.len().saturating_sub(excluded_count);
            (included_count, excluded_count)
        })
}

fn build_project_import_history_response(
    state: &Arc<AppState>,
    project: Project,
    source_binding: ProjectSourceBinding,
) -> Option<ProjectImportHistoryResponse> {
    let history = state.imports.history_for_project(project.id);
    if history.is_empty() {
        return None;
    }

    let entries = history
        .iter()
        .map(|entry| {
            let selection_summary = entry
                .draft
                .as_ref()
                .and_then(|draft| build_import_history_selection_summary(state, &entry.job, draft));
            ProjectImportHistoryEntry {
                import_job: entry.job.clone(),
                source_metadata: entry
                    .draft
                    .as_ref()
                    .map(|draft| draft.source_metadata.clone()),
                discovered_node_count: entry
                    .draft
                    .as_ref()
                    .map(|draft| draft.discovered_nodes.len()),
                effective_included_node_count: selection_summary.map(|(included, _)| included),
                effective_excluded_node_count: selection_summary.map(|(_, excluded)| excluded),
            }
        })
        .collect::<Vec<_>>();

    let latest_pending = history.iter().find_map(|entry| {
        if matches!(entry.job.status, ImportStatus::ReviewPending) {
            entry.draft.clone().map(|draft| (entry.job.clone(), draft))
        } else {
            None
        }
    });
    let latest_applied = history.iter().find_map(|entry| {
        if matches!(entry.job.status, ImportStatus::Applied) {
            entry.draft.clone().map(|draft| (entry.job.clone(), draft))
        } else {
            None
        }
    });
    let diff_summary = match (latest_pending.as_ref(), latest_applied.as_ref()) {
        (Some((current_job, current_draft)), Some((compared_job, compared_draft)))
            if current_draft.job_id != compared_draft.job_id =>
        {
            let (current_draft, _) =
                build_selection_aware_import_draft(state, current_job, current_draft);
            let (compared_draft, _) =
                build_selection_aware_import_draft(state, compared_job, compared_draft);
            Some(build_import_draft_diff_summary(
                &current_draft,
                &compared_draft,
            ))
        }
        _ => None,
    };

    Some(ProjectImportHistoryResponse {
        project,
        source_binding,
        history: entries,
        diff_summary,
    })
}

fn current_import_draft_for_comparison(
    state: &Arc<AppState>,
    project_id: Uuid,
) -> Option<(ProjectImportJob, ProjectImportDraft)> {
    if let Some(job) = state.imports.latest_review_job_for_project(project_id) {
        if let Some(draft) = state.imports.get_draft(job.id) {
            return Some((job, draft));
        }
    }

    state
        .imports
        .latest_applied_job_for_project(project_id)
        .and_then(|job| state.imports.get_draft(job.id).map(|draft| (job, draft)))
}

fn project_import_history_entry_for_comparison(
    state: &Arc<AppState>,
    project_ref: &str,
    project_id: Uuid,
    job_id: Uuid,
) -> Result<(ProjectImportJob, ProjectImportDraft), (StatusCode, Json<ErrorResponse>)> {
    let historical_job = state.imports.get_job(job_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        )
    })?;
    if historical_job.project_id != project_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        ));
    }

    let historical_draft = state.imports.get_draft(historical_job.id).ok_or_else(|| {
        (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!(
                    "Import history entry {} does not have a durable draft for comparison",
                    historical_job.id
                ),
                code: Some("PROJECT_IMPORT_HISTORY_COMPARE_DRAFT_MISSING".into()),
            }),
        )
    })?;

    Ok((historical_job, historical_draft))
}

const IMPORT_DRAFT_OWNED_TAG: &str = "import-draft-owned";
const IMPORT_REVIEW_METADATA_PREFIX: &str = "import-review:";

fn import_project_root_node_id(project_id: &str) -> planner_schemas::artifacts::blueprint::NodeId {
    let slug = project_id
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-")
        .trim_matches('-')
        .to_string();
    planner_schemas::artifacts::blueprint::NodeId::from_raw(format!("proj-{}", slug))
}

fn project_blueprint_scope(project: &Project) -> planner_schemas::artifacts::blueprint::NodeScope {
    planner_schemas::artifacts::blueprint::NodeScope {
        scope_class: planner_schemas::artifacts::blueprint::ScopeClass::Project,
        project: Some(planner_schemas::artifacts::blueprint::ProjectScope {
            project_id: project.id.to_string(),
            project_name: Some(project.name.clone()),
        }),
        secondary: planner_schemas::artifacts::blueprint::SecondaryScopeRefs::default(),
        is_shared: false,
        shared: None,
        lifecycle: planner_schemas::artifacts::blueprint::NodeLifecycle::Active,
        override_scope: None,
        scope_review: None,
    }
}

fn ensure_project_root_blueprint_node(
    state: &Arc<AppState>,
    project: &Project,
) -> planner_schemas::artifacts::blueprint::NodeId {
    let root_id = import_project_root_node_id(&project.id.to_string());
    let now = chrono::Utc::now().to_rfc3339();
    state.blueprints.upsert_node(
        planner_schemas::artifacts::blueprint::BlueprintNode::Project(
            planner_schemas::artifacts::blueprint::Project {
                id: root_id.clone(),
                name: project.name.clone(),
                description: format!("Blueprint root for project {}", project.slug),
                tags: vec!["project-root".into(), "import-owned".into()],
                documentation: None,
                scope: project_blueprint_scope(project),
                created_at: now.clone(),
                updated_at: now,
            },
        ),
    );
    root_id
}

fn node_tags(node: &planner_schemas::artifacts::blueprint::BlueprintNode) -> &[String] {
    use planner_schemas::artifacts::blueprint::BlueprintNode;
    match node {
        BlueprintNode::Project(n) => &n.tags,
        BlueprintNode::Decision(n) => &n.tags,
        BlueprintNode::Technology(n) => &n.tags,
        BlueprintNode::Component(n) => &n.tags,
        BlueprintNode::Constraint(n) => &n.tags,
        BlueprintNode::Pattern(n) => &n.tags,
        BlueprintNode::QualityRequirement(n) => &n.tags,
    }
}

fn node_updated_at_mut(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) -> &mut String {
    use planner_schemas::artifacts::blueprint::BlueprintNode;
    match node {
        BlueprintNode::Project(n) => &mut n.updated_at,
        BlueprintNode::Decision(n) => &mut n.updated_at,
        BlueprintNode::Technology(n) => &mut n.updated_at,
        BlueprintNode::Component(n) => &mut n.updated_at,
        BlueprintNode::Constraint(n) => &mut n.updated_at,
        BlueprintNode::Pattern(n) => &mut n.updated_at,
        BlueprintNode::QualityRequirement(n) => &mut n.updated_at,
    }
}

fn import_review_metadata(job_id: uuid::Uuid) -> String {
    format!("{IMPORT_REVIEW_METADATA_PREFIX}{job_id}")
}

fn add_tag_if_missing(tags: &mut Vec<String>, tag: &str) {
    if !tags
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(tag))
    {
        tags.push(tag.to_string());
    }
}

fn is_import_review_membership_edge(
    edge: &planner_schemas::artifacts::blueprint::Edge,
    root_id: &planner_schemas::artifacts::blueprint::NodeId,
) -> bool {
    edge.source == *root_id
        && matches!(
            edge.edge_type,
            planner_schemas::artifacts::blueprint::EdgeType::Contains
        )
        && edge
            .metadata
            .as_deref()
            .map(|value| value.starts_with(IMPORT_REVIEW_METADATA_PREFIX))
            .unwrap_or(false)
}

fn is_project_local_import_draft_owned_node(
    node: &planner_schemas::artifacts::blueprint::BlueprintNode,
    project_id: &str,
) -> bool {
    node.scope().is_project_local_to(project_id)
        && node_tags(node)
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(IMPORT_DRAFT_OWNED_TAG))
}

fn stamp_import_draft_provenance(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
    job_id: uuid::Uuid,
) {
    let tags = node_tags_mut(node);
    tags.retain(|tag| {
        !tag.trim()
            .to_ascii_lowercase()
            .starts_with(IMPORT_REVIEW_METADATA_PREFIX)
    });
    add_tag_if_missing(tags, IMPORT_DRAFT_OWNED_TAG);
    add_tag_if_missing(tags, &import_review_metadata(job_id));
}

fn preserve_import_draft_provenance(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) {
    let tags = node_tags_mut(node);
    add_tag_if_missing(tags, IMPORT_DRAFT_OWNED_TAG);
}

fn promote_import_draft_to_blueprint(
    state: &Arc<AppState>,
    project: &Project,
    draft: &ProjectImportDraft,
) -> Result<(), String> {
    use planner_schemas::artifacts::blueprint::{Edge, EdgeType, NodeLifecycle};

    let project_scope = project_blueprint_scope(project);
    let root_id = ensure_project_root_blueprint_node(state, project);
    let project_id = project.id.to_string();
    let current_node_ids = draft
        .discovered_nodes
        .iter()
        .map(|node| node.id().to_string())
        .collect::<std::collections::HashSet<_>>();
    let snapshot = state.blueprints.snapshot();
    let mut prior_import_owned_node_ids = snapshot
        .edges
        .iter()
        .filter(|edge| is_import_review_membership_edge(edge, &root_id))
        .map(|edge| edge.target.to_string())
        .collect::<std::collections::HashSet<_>>();
    prior_import_owned_node_ids.extend(
        snapshot
            .nodes
            .values()
            .filter(|node| is_project_local_import_draft_owned_node(node, &project_id))
            .map(|node| node.id().to_string()),
    );
    let stale_import_owned_node_ids = prior_import_owned_node_ids
        .into_iter()
        .filter(|node_id| !current_node_ids.contains(node_id))
        .collect::<Vec<_>>();
    let archived_at = chrono::Utc::now().to_rfc3339();

    for node_id in stale_import_owned_node_ids {
        state.blueprints.update_node(&node_id, |node| {
            preserve_import_draft_provenance(node);
            node_scope_mut(node).lifecycle = NodeLifecycle::Archived;
            *node_updated_at_mut(node) = archived_at.clone();
        });
    }

    state
        .blueprints
        .remove_edges_where(|edge| is_import_review_membership_edge(edge, &root_id));

    for draft_node in &draft.discovered_nodes {
        let mut node = draft_node.clone();
        *node_scope_mut(&mut node) = project_scope.clone();
        node_scope_mut(&mut node).lifecycle = NodeLifecycle::Active;
        stamp_import_draft_provenance(&mut node, draft.job_id);
        normalize_blueprint_node_metadata(&mut node);
        validate_blueprint_node_scope(&node)?;
        state.blueprints.upsert_node(node.clone());
        state.blueprints.add_edge(Edge {
            source: root_id.clone(),
            target: node.id().clone(),
            edge_type: EdgeType::Contains,
            metadata: Some(import_review_metadata(draft.job_id)),
        });
    }

    state.blueprints.flush().map_err(|error| {
        format!(
            "Failed to flush blueprint store after applying import draft {}: {}",
            draft.job_id, error
        )
    })?;
    Ok(())
}

fn create_seeded_import_session(
    state: &Arc<AppState>,
    project: &Project,
    user_id: &str,
    analysis_summary: &str,
) -> Result<Session, String> {
    let seeded_description = seeded_import_session_description(&project.name, analysis_summary);
    let session = state.sessions.create(user_id);
    state
        .sessions
        .update(session.id, |draft| {
            draft.project_id = Some(project.id);
            draft.project_slug = Some(project.slug.clone());
            draft.project_name = Some(project.name.clone());
            draft.cxdb_project_id = Some(project.id);
            draft.project_description = Some(seeded_description.clone());
            draft.ensure_title_from_description();
        })
        .ok_or_else(|| format!("failed to update seeded session {}", session.id))
}

fn validate_local_import_root(local_root: &std::path::Path) -> Result<(), String> {
    if !local_root.is_absolute() {
        return Err(format!(
            "local import root must remain absolute: {}",
            local_root.display()
        ));
    }
    let metadata = std::fs::metadata(local_root).map_err(|error| {
        format!(
            "local import root is unavailable at {}: {}",
            local_root.display(),
            error
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "local import root is not a directory: {}",
            local_root.display()
        ));
    }
    std::fs::read_dir(local_root).map_err(|error| {
        format!(
            "local import root is not readable at {}: {}",
            local_root.display(),
            error
        )
    })?;
    Ok(())
}

fn spawn_project_import_processing(state: Arc<AppState>, import_job_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = run_project_import_processing(state.clone(), import_job_id).await {
            tracing::warn!(
                "project import processing failed for {}: {}",
                import_job_id,
                error
            );
            let _ = state.imports.mark_job_failed(import_job_id, error);
        }
    });
}

async fn run_project_import_processing(
    state: Arc<AppState>,
    import_job_id: Uuid,
) -> Result<(), String> {
    let import_job = state
        .imports
        .get_job(import_job_id)
        .ok_or_else(|| format!("import job not found: {}", import_job_id))?;
    let source_binding = state
        .imports
        .get_binding(import_job.project_id)
        .ok_or_else(|| format!("import binding not found for {}", import_job.project_id))?;
    let project = state
        .projects
        .get(import_job.project_id)
        .ok_or_else(|| format!("project not found for import {}", import_job.project_id))?;

    let (local_root, default_branch, head_revision, analyzing_message) = match import_job.provider {
        ImportProvider::GitHub => {
            state
                .imports
                .mark_job_cloning(import_job_id, "Cloning default branch into managed storage")
                .map_err(|err| format!("failed to mark import job cloning: {}", err))?;

            let checkout_path = state
                .imports
                .managed_checkout_path(import_job.project_id, import_job.provider);
            let acquired = match state
                .import_acquirer
                .acquire_github(&source_binding.canonical_ref, &checkout_path)
                .await
            {
                Ok(acquired) => acquired,
                Err(error) => {
                    if checkout_path.exists() {
                        let _ = std::fs::remove_dir_all(&checkout_path);
                    }
                    return Err(error);
                }
            };

            if !checkout_path.exists() {
                std::fs::create_dir_all(&checkout_path).map_err(|err| {
                    format!(
                        "failed to materialize managed checkout {}: {}",
                        checkout_path.display(),
                        err
                    )
                })?;
            }

            state
                .imports
                .update_binding_source_metadata(
                    import_job.project_id,
                    Some(acquired.default_branch.clone()),
                    Some(acquired.head_revision.clone()),
                    checkout_path.to_string_lossy().to_string(),
                )
                .map_err(|err| format!("failed to persist checkout metadata: {}", err))?;

            (
                checkout_path,
                Some(acquired.default_branch),
                Some(acquired.head_revision),
                "Analyzing checkout and seeding planning session",
            )
        }
        ImportProvider::Local => {
            let local_root = PathBuf::from(&source_binding.canonical_ref);
            validate_local_import_root(&local_root)?;
            let metadata = inspect_local_import_source(&local_root).await?;
            state
                .imports
                .update_binding_source_metadata(
                    import_job.project_id,
                    metadata.default_branch.clone(),
                    metadata.head_revision.clone(),
                    local_root.to_string_lossy().to_string(),
                )
                .map_err(|err| format!("failed to persist local source metadata: {}", err))?;
            (
                local_root,
                metadata.default_branch,
                metadata.head_revision,
                "Analyzing local source and seeding planning session",
            )
        }
    };

    state
        .imports
        .mark_job_analyzing(import_job_id, analyzing_message)
        .map_err(|err| format!("failed to mark import job analyzing: {}", err))?;

    let analysis = state
        .import_analyzer
        .analyze(ImportAnalysisRequest {
            project_id: project.id,
            project_name: project.name.clone(),
            provider: import_job.provider,
            canonical_ref: source_binding.canonical_ref.clone(),
            local_root: local_root.clone(),
            default_branch: default_branch.clone(),
            head_revision: head_revision.clone(),
        })
        .await?;

    state
        .imports
        .save_draft(crate::import::ProjectImportDraft {
            job_id: import_job_id,
            project_id: project.id,
            analysis_summary: analysis.analysis_summary.clone(),
            source_metadata: crate::import::ImportDraftSourceMetadata {
                provider: import_job.provider,
                canonical_ref: source_binding.canonical_ref.clone(),
                local_root: local_root.to_string_lossy().to_string(),
                default_branch: default_branch.clone(),
                head_revision: head_revision.clone(),
            },
            discovered_nodes: analysis.discovered_nodes.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .map_err(|err| format!("failed to persist import draft: {}", err))?;

    let seeded_session = create_seeded_import_session(
        &state,
        &project,
        &project.owner_user_id,
        &analysis.analysis_summary,
    )?;

    state
        .imports
        .mark_job_review_pending(
            import_job_id,
            "Import draft ready. Review imported context in the seeded session.",
            analysis.analysis_summary,
            seeded_session.id,
        )
        .map_err(|err| format!("failed to mark import job review_pending: {}", err))?;
    Ok(())
}

async fn create_project_import(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<CreateProjectImportRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let (canonical_ref, managed_checkout) =
        normalize_import_source_ref(req.provider, &req.source_ref)?;
    if let Some(source_binding) = state
        .imports
        .find_binding_by_source(req.provider, &canonical_ref)
    {
        let project = state
            .projects
            .get(source_binding.project_id)
            .ok_or_else(|| {
                internal_error_response(
                    format!(
                        "Import binding for {} points to missing project {}",
                        source_binding.canonical_ref, source_binding.project_id
                    ),
                    "PROJECT_IMPORT_BOUND_PROJECT_MISSING",
                )
            })?;
        if project_visible_to_user(&project, &claims) {
            return Ok((
                StatusCode::CONFLICT,
                Json(ProjectImportConflictResponse {
                    message: format!(
                        "Source {} is already bound to project {}. Open that project and use re-import instead.",
                        source_binding.canonical_ref, project.slug
                    ),
                    project,
                    source_binding,
                }),
            )
                .into_response());
        }
    }

    let project_name = derive_import_project_name(req.provider, &canonical_ref);
    let project = state
        .projects
        .create(&claims.sub, &project_name, None, None, Vec::new(), None);
    let (import_job, source_binding) = state
        .imports
        .create(
            project.id,
            req.provider,
            req.source_ref.clone(),
            canonical_ref,
            managed_checkout,
        )
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to persist import records for {}: {}",
                    project.id, error
                ),
                "PROJECT_IMPORT_PERSIST_FAILED",
            )
        })?;

    if should_start_import_processing(req.provider) {
        spawn_project_import_processing(state.clone(), import_job.id);
    }

    Ok((
        StatusCode::CREATED,
        Json(build_project_import_response(
            &state,
            project,
            import_job,
            source_binding,
        )),
    )
        .into_response())
}

async fn get_project_import(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Some(import_job) = state.imports.get_job(job_id) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import job not found: {}", job_id),
                code: Some("PROJECT_IMPORT_NOT_FOUND".into()),
            }),
        ));
    };

    let Some(project) = state.projects.get(import_job.project_id) else {
        return Err(internal_error_response(
            format!(
                "Import job {} references missing project {}",
                import_job.id, import_job.project_id
            ),
            "PROJECT_IMPORT_PROJECT_MISSING",
        ));
    };

    if !project_visible_to_user(&project, &claims) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Forbidden".into(),
                code: Some("FORBIDDEN".into()),
            }),
        ));
    }

    let Some(source_binding) = state.imports.get_binding(project.id) else {
        return Err(internal_error_response(
            format!(
                "Import job {} has no source binding for {}",
                import_job.id, project.id
            ),
            "PROJECT_IMPORT_BINDING_MISSING",
        ));
    };

    Ok(Json(build_project_import_response(
        &state,
        project,
        import_job,
        source_binding,
    )))
}

async fn get_project_import_state(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let import_job = state
        .imports
        .latest_job_for_project(project.id)
        .ok_or_else(|| import_state_not_found_response(&project_ref))?;
    let source_binding = state.imports.get_binding(project.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Latest import job {} has no source binding for {}",
                import_job.id, project.id
            ),
            "PROJECT_IMPORT_BINDING_MISSING",
        )
    })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        import_job,
        source_binding,
    )))
}

async fn get_project_import_history(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectImportHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let source_binding = state
        .imports
        .get_binding(project.id)
        .ok_or_else(|| import_history_not_found_response(&project_ref))?;
    let response = build_project_import_history_response(&state, project, source_binding)
        .ok_or_else(|| import_history_not_found_response(&project_ref))?;
    Ok(Json(response))
}

async fn compare_project_import_history_entry(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path((project_ref, job_id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<ProjectImportHistoryComparisonResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let source_binding = state
        .imports
        .get_binding(project.id)
        .ok_or_else(|| import_history_not_found_response(&project_ref))?;
    let (historical_job, historical_draft) =
        project_import_history_entry_for_comparison(&state, &project_ref, project.id, job_id)?;
    let (current_import_job, current_draft) =
        current_import_draft_for_comparison(&state, project.id).ok_or_else(|| {
            (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!(
                        "Project {} does not have a current import draft state available for comparison",
                        project_ref
                    ),
                    code: Some("PROJECT_IMPORT_CURRENT_COMPARE_STATE_MISSING".into()),
                }),
            )
        })?;

    let selected_entry = ProjectImportHistoryEntry {
        import_job: historical_job.clone(),
        source_metadata: Some(historical_draft.source_metadata.clone()),
        discovered_node_count: Some(historical_draft.discovered_nodes.len()),
        effective_included_node_count: None,
        effective_excluded_node_count: None,
    };
    let (current_draft, current_import_job_uses_selection_filter) =
        build_selection_aware_import_draft(&state, &current_import_job, &current_draft);
    let (historical_draft, selected_entry_uses_selection_filter) =
        build_selection_aware_import_draft(&state, &historical_job, &historical_draft);
    let diff_summary = build_import_draft_diff_summary(&current_draft, &historical_draft);

    Ok(Json(ProjectImportHistoryComparisonResponse {
        project,
        source_binding,
        selected_entry,
        current_import_job,
        selected_entry_uses_selection_filter,
        current_import_job_uses_selection_filter,
        diff_summary,
    }))
}

async fn compare_project_import_history_entries(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path((project_ref, base_job_id, job_id)): Path<(String, uuid::Uuid, uuid::Uuid)>,
) -> Result<Json<ProjectImportHistoryPairComparisonResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let source_binding = state
        .imports
        .get_binding(project.id)
        .ok_or_else(|| import_history_not_found_response(&project_ref))?;
    let (baseline_job, baseline_draft) =
        project_import_history_entry_for_comparison(&state, &project_ref, project.id, base_job_id)?;
    let (compared_job, compared_draft) =
        project_import_history_entry_for_comparison(&state, &project_ref, project.id, job_id)?;

    let baseline_entry = ProjectImportHistoryEntry {
        import_job: baseline_job.clone(),
        source_metadata: Some(baseline_draft.source_metadata.clone()),
        discovered_node_count: Some(baseline_draft.discovered_nodes.len()),
        effective_included_node_count: None,
        effective_excluded_node_count: None,
    };
    let compared_entry = ProjectImportHistoryEntry {
        import_job: compared_job.clone(),
        source_metadata: Some(compared_draft.source_metadata.clone()),
        discovered_node_count: Some(compared_draft.discovered_nodes.len()),
        effective_included_node_count: None,
        effective_excluded_node_count: None,
    };
    let (baseline_draft, baseline_entry_uses_selection_filter) =
        build_selection_aware_import_draft(&state, &baseline_job, &baseline_draft);
    let (compared_draft, compared_entry_uses_selection_filter) =
        build_selection_aware_import_draft(&state, &compared_job, &compared_draft);
    let diff_summary = build_import_draft_diff_summary(&compared_draft, &baseline_draft);

    Ok(Json(ProjectImportHistoryPairComparisonResponse {
        project,
        source_binding,
        baseline_entry,
        compared_entry,
        baseline_entry_uses_selection_filter,
        compared_entry_uses_selection_filter,
        diff_summary,
    }))
}

async fn get_project_import_review(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let import_job = state
        .imports
        .latest_review_job_for_project(project.id)
        .ok_or_else(|| import_review_not_found_response(&project_ref))?;
    if !matches!(
        import_job.status,
        ImportStatus::ReviewPending | ImportStatus::Applied
    ) {
        return Err(import_review_not_found_response(&project_ref));
    }
    let source_binding = state.imports.get_binding(project.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Reviewable import job {} has no source binding for {}",
                import_job.id, project.id
            ),
            "PROJECT_IMPORT_BINDING_MISSING",
        )
    })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        import_job,
        source_binding,
    )))
}

async fn reimport_project(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<(StatusCode, Json<ProjectImportResponse>), (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let (import_job, source_binding) =
        state
            .imports
            .create_reimport_job(project.id)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    (
                        StatusCode::NOT_FOUND,
                        Json(ErrorResponse {
                            error: format!("No import binding found for project {}", project_ref),
                            code: Some("PROJECT_IMPORT_BINDING_NOT_FOUND".into()),
                        }),
                    )
                } else {
                    internal_error_response(
                        format!(
                            "Failed to persist re-import records for {}: {}",
                            project.id, error
                        ),
                        "PROJECT_REIMPORT_PERSIST_FAILED",
                    )
                }
            })?;

    if should_start_import_processing(import_job.provider) {
        spawn_project_import_processing(state.clone(), import_job.id);
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(build_project_import_response(
            &state,
            project,
            import_job,
            source_binding,
        )),
    ))
}

async fn restore_project_import_history_entry(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path((project_ref, job_id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    if state
        .imports
        .latest_review_job_for_project(project.id)
        .is_some_and(|job| matches!(job.status, ImportStatus::ReviewPending))
    {
        return Err(import_restore_pending_review_response(&project_ref));
    }

    let historical_job = state.imports.get_job(job_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        )
    })?;
    if historical_job.project_id != project.id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        ));
    }
    if !matches!(historical_job.status, ImportStatus::Applied) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!(
                    "Import history entry {} is not eligible for restore",
                    historical_job.id
                ),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_RESTORABLE".into()),
            }),
        ));
    }

    let historical_draft = state.imports.get_draft(historical_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Applied import history entry {} is missing draft payload",
                historical_job.id
            ),
            "PROJECT_IMPORT_HISTORY_DRAFT_MISSING",
        )
    })?;
    let (restore_job, source_binding) = state
        .imports
        .create_restore_job(project.id, historical_job.id)
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to persist restore job for historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_RESTORE_PERSIST_FAILED",
            )
        })?;

    let now = chrono::Utc::now().to_rfc3339();
    let restore_draft = state
        .imports
        .save_draft(ProjectImportDraft {
            job_id: restore_job.id,
            project_id: project.id,
            analysis_summary: historical_draft.analysis_summary.clone(),
            source_metadata: historical_draft.source_metadata.clone(),
            discovered_nodes: historical_draft.discovered_nodes.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
        .map_err(|error| {
            let _ = state.imports.mark_job_failed(
                restore_job.id,
                format!(
                    "Historical restore failed while cloning draft from {}: {}",
                    historical_job.id, error
                ),
            );
            internal_error_response(
                format!(
                    "Failed to persist restore draft for historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_RESTORE_DRAFT_PERSIST_FAILED",
            )
        })?;

    if let Err(error) = promote_import_draft_to_blueprint(&state, &project, &restore_draft) {
        let _ = state.imports.mark_job_failed(
            restore_job.id,
            format!(
                "Historical restore failed while applying import {}: {}",
                historical_job.id, error
            ),
        );
        return Err(internal_error_response(
            error,
            "PROJECT_IMPORT_RESTORE_APPLY_FAILED",
        ));
    }

    let restored_job = state
        .imports
        .mark_job_applied(
            restore_job.id,
            format!(
                "Historical import restored from {} into the canonical project blueprint.",
                historical_job.id
            ),
            Some(historical_job.id),
        )
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to mark restore job {} applied: {}",
                    restore_job.id, error
                ),
                "PROJECT_IMPORT_RESTORE_STATUS_FAILED",
            )
        })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        restored_job,
        source_binding,
    )))
}

async fn restore_project_import_review_draft(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path((project_ref, job_id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    if state
        .imports
        .latest_review_job_for_project(project.id)
        .is_some_and(|job| matches!(job.status, ImportStatus::ReviewPending))
    {
        return Err(import_restore_pending_review_response(&project_ref));
    }

    let historical_job = state.imports.get_job(job_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        )
    })?;
    if historical_job.project_id != project.id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        ));
    }
    if !matches!(historical_job.status, ImportStatus::ReviewPending) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!(
                    "Import history entry {} is not eligible for review draft restore",
                    historical_job.id
                ),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_REVIEW_RESTORABLE".into()),
            }),
        ));
    }

    let historical_draft = state.imports.get_draft(historical_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Historical review draft {} is missing draft payload",
                historical_job.id
            ),
            "PROJECT_IMPORT_HISTORY_DRAFT_MISSING",
        )
    })?;
    let (restore_job, source_binding) = state
        .imports
        .create_restore_review_job(
            project.id,
            historical_job.id,
            historical_job.analysis_summary.clone(),
            historical_job.seed_session_id,
        )
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to persist restore-review job for historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_REVIEW_RESTORE_PERSIST_FAILED",
            )
        })?;

    let now = chrono::Utc::now().to_rfc3339();
    state
        .imports
        .save_draft(ProjectImportDraft {
            job_id: restore_job.id,
            project_id: project.id,
            analysis_summary: historical_draft.analysis_summary.clone(),
            source_metadata: historical_draft.source_metadata.clone(),
            discovered_nodes: historical_draft.discovered_nodes.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
        .map_err(|error| {
            let _ = state.imports.mark_job_failed(
                restore_job.id,
                format!(
                    "Historical review draft restore failed while cloning draft from {}: {}",
                    historical_job.id, error
                ),
            );
            internal_error_response(
                format!(
                    "Failed to persist restored review draft for historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_REVIEW_RESTORE_DRAFT_PERSIST_FAILED",
            )
        })?;

    let restored_job = state.imports.get_job(restore_job.id).ok_or_else(|| {
        internal_error_response(
            format!("Restored review draft job {} disappeared", restore_job.id),
            "PROJECT_IMPORT_REVIEW_RESTORE_JOB_MISSING",
        )
    })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        restored_job,
        source_binding,
    )))
}

async fn restore_project_import_history_entry_for_review(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path((project_ref, job_id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    if state
        .imports
        .latest_review_job_for_project(project.id)
        .is_some_and(|job| matches!(job.status, ImportStatus::ReviewPending))
    {
        return Err(import_restore_pending_review_response(&project_ref));
    }

    let historical_job = state.imports.get_job(job_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        )
    })?;
    if historical_job.project_id != project.id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Import history entry not found for project {}", project_ref),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_FOUND".into()),
            }),
        ));
    }
    if !matches!(historical_job.status, ImportStatus::Applied) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!(
                    "Import history entry {} is not eligible for restore-for-review",
                    historical_job.id
                ),
                code: Some("PROJECT_IMPORT_HISTORY_ENTRY_NOT_REVIEWABLE_RESTORE".into()),
            }),
        ));
    }

    let historical_draft = state.imports.get_draft(historical_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Applied import history entry {} is missing draft payload",
                historical_job.id
            ),
            "PROJECT_IMPORT_HISTORY_DRAFT_MISSING",
        )
    })?;
    let (restore_job, source_binding) = state
        .imports
        .create_restore_review_job(
            project.id,
            historical_job.id,
            historical_job.analysis_summary.clone(),
            historical_job.seed_session_id,
        )
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to persist restore-for-review job for historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_RESTORE_FOR_REVIEW_PERSIST_FAILED",
            )
        })?;

    let now = chrono::Utc::now().to_rfc3339();
    state
        .imports
        .save_draft(ProjectImportDraft {
            job_id: restore_job.id,
            project_id: project.id,
            analysis_summary: historical_draft.analysis_summary.clone(),
            source_metadata: historical_draft.source_metadata.clone(),
            discovered_nodes: historical_draft.discovered_nodes.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
        .map_err(|error| {
            let _ = state.imports.mark_job_failed(
                restore_job.id,
                format!(
                    "Historical applied import restore-for-review failed while cloning draft from {}: {}",
                    historical_job.id, error
                ),
            );
            internal_error_response(
                format!(
                    "Failed to persist restored review draft for applied historical import {}: {}",
                    historical_job.id, error
                ),
                "PROJECT_IMPORT_RESTORE_FOR_REVIEW_DRAFT_PERSIST_FAILED",
            )
        })?;

    let restored_job = state.imports.get_job(restore_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Restored applied import review job {} disappeared",
                restore_job.id
            ),
            "PROJECT_IMPORT_RESTORE_FOR_REVIEW_JOB_MISSING",
        )
    })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        restored_job,
        source_binding,
    )))
}

async fn update_project_import_review_selection(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
    Json(req): Json<UpdateProjectImportReviewSelectionRequest>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let import_job = state
        .imports
        .latest_review_job_for_project(project.id)
        .ok_or_else(|| import_review_not_found_response(&project_ref))?;
    let source_binding = state.imports.get_binding(project.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Reviewable import job {} has no source binding for {}",
                import_job.id, project.id
            ),
            "PROJECT_IMPORT_BINDING_MISSING",
        )
    })?;
    let draft = state.imports.get_draft(import_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Reviewable import job {} is missing draft payload",
                import_job.id
            ),
            "PROJECT_IMPORT_DRAFT_MISSING",
        )
    })?;

    if !draft
        .discovered_nodes
        .iter()
        .any(|node| node.id().to_string() == req.node_id)
    {
        return Err(import_review_node_not_found_response(
            &project_ref,
            &req.node_id,
        ));
    }

    state
        .imports
        .set_review_node_included(import_job.id, project.id, &req.node_id, req.included)
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to update import review selection for job {}: {}",
                    import_job.id, error
                ),
                "PROJECT_IMPORT_REVIEW_SELECTION_UPDATE_FAILED",
            )
        })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        import_job,
        source_binding,
    )))
}

async fn apply_project_import_review(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let import_job = state
        .imports
        .latest_review_job_for_project(project.id)
        .ok_or_else(|| import_review_not_found_response(&project_ref))?;
    let source_binding = state.imports.get_binding(project.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Reviewable import job {} has no source binding for {}",
                import_job.id, project.id
            ),
            "PROJECT_IMPORT_BINDING_MISSING",
        )
    })?;

    if matches!(import_job.status, ImportStatus::Applied) {
        return Ok(Json(build_project_import_response(
            &state,
            project,
            import_job,
            source_binding,
        )));
    }

    let draft = state.imports.get_draft(import_job.id).ok_or_else(|| {
        internal_error_response(
            format!(
                "Reviewable import job {} is missing draft payload",
                import_job.id
            ),
            "PROJECT_IMPORT_DRAFT_MISSING",
        )
    })?;
    if draft.project_id != project.id {
        return Err(internal_error_response(
            format!(
                "Import draft {} belongs to {} instead of {}",
                draft.job_id, draft.project_id, project.id
            ),
            "PROJECT_IMPORT_DRAFT_PROJECT_MISMATCH",
        ));
    }

    let selected_draft = build_selected_import_draft(&state, &import_job, &draft);

    promote_import_draft_to_blueprint(&state, &project, &selected_draft)
        .map_err(|error| internal_error_response(error, "PROJECT_IMPORT_APPLY_FAILED"))?;

    let import_job = state
        .imports
        .mark_job_applied(
            import_job.id,
            "Import draft applied and reconciled against the canonical project blueprint.",
            None,
        )
        .map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to mark import job {} applied: {}",
                    import_job.id, error
                ),
                "PROJECT_IMPORT_APPLY_STATUS_FAILED",
            )
        })?;

    Ok(Json(build_project_import_response(
        &state,
        project,
        import_job,
        source_binding,
    )))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), (StatusCode, Json<ErrorResponse>)> {
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Project name cannot be empty".into(),
                code: Some("INVALID_PROJECT_NAME".into()),
            }),
        ));
    }

    let mut project = state.projects.create(
        &claims.sub,
        &req.name,
        req.description.clone(),
        req.team_label.clone(),
        req.legacy_scope_keys.clone(),
        None,
    );

    if let Some(slug) = req.slug {
        if let Some(updated) = state.projects.update(project.id, |draft| {
            draft.slug = slug;
        }) {
            project = updated;
        }
    }

    Ok((StatusCode::CREATED, Json(ProjectResponse { project })))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    Ok(Json(ProjectResponse { project }))
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;

    if req.name.is_none()
        && req.slug.is_none()
        && req.description.is_none()
        && req.team_label.is_none()
        && req.legacy_scope_keys.is_none()
        && req.archived.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No project changes were requested".into(),
                code: Some("EMPTY_PROJECT_UPDATE".into()),
            }),
        ));
    }

    let has_metadata_patch = req.name.is_some()
        || req.slug.is_some()
        || req.description.is_some()
        || req.team_label.is_some()
        || req.legacy_scope_keys.is_some();

    let mut updated = if has_metadata_patch {
        state.projects.update(project.id, |draft| {
            if let Some(name) = req.name.as_ref() {
                draft.name = name.clone();
            }
            if let Some(slug) = req.slug.as_ref() {
                draft.slug = slug.clone();
            }
            if let Some(description) = req.description.as_ref() {
                draft.description = Some(description.clone());
            }
            if let Some(team_label) = req.team_label.as_ref() {
                draft.team_label = Some(team_label.clone());
            }
            if let Some(legacy_scope_keys) = req.legacy_scope_keys.as_ref() {
                draft.legacy_scope_keys = legacy_scope_keys.clone();
            }
        })
    } else {
        Some(project.clone())
    };

    if let Some(archived) = req.archived {
        updated = state.projects.set_archived(project.id, archived);
    }

    match updated {
        Some(project) => Ok(Json(ProjectResponse { project })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", project_ref),
                code: Some("PROJECT_NOT_FOUND".into()),
            }),
        )),
    }
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<DeleteProjectResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let project_sessions = state.sessions.list_for_project(project.id);

    let mut stopped_live_sessions = 0usize;
    let mut stopped_pipeline_sessions = 0usize;
    let mut deleted_sessions = 0usize;
    let mut deleted_session_event_files = 0usize;

    for session in project_sessions {
        let stop_report = stop_active_session_work(&state, session.id);
        if stop_report.stopped_live_session {
            stopped_live_sessions += 1;
        }
        if stop_report.stopped_pipeline_session {
            stopped_pipeline_sessions += 1;
        }

        if let Some(store) = state.event_store.as_ref() {
            let had_persisted_events = store
                .load_session_events(session.id)
                .map(|events| !events.is_empty())
                .map_err(|error| {
                    internal_error_response(
                        format!(
                            "Failed to inspect session events for {} during project delete: {}",
                            session.id, error
                        ),
                        "PROJECT_DELETE_EVENT_INSPECTION_FAILED",
                    )
                })?;
            store.delete_session_events(session.id).map_err(|error| {
                internal_error_response(
                    format!(
                        "Failed to delete session events for {} during project delete: {}",
                        session.id, error
                    ),
                    "PROJECT_DELETE_EVENT_DELETE_FAILED",
                )
            })?;
            if had_persisted_events {
                deleted_session_event_files += 1;
            }
        }

        match state.sessions.delete(session.id).map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to delete session {} during project delete: {}",
                    session.id, error
                ),
                "PROJECT_DELETE_SESSION_FAILED",
            )
        })? {
            true => deleted_sessions += 1,
            false => {
                return Err(internal_error_response(
                    format!(
                        "Session {} disappeared during project delete for {}",
                        session.id, project.id
                    ),
                    "PROJECT_DELETE_SESSION_MISSING",
                ));
            }
        }
    }

    let mut deleted_cxdb_runs = 0usize;
    if let Some(cxdb) = state.cxdb.as_ref() {
        let report = cxdb.delete_project(project.id).map_err(|error| {
            internal_error_response(
                format!(
                    "Failed to delete CXDB project data for {}: {}",
                    project.id, error
                ),
                "PROJECT_DELETE_CXDB_FAILED",
            )
        })?;
        deleted_cxdb_runs = report.runs_deleted;
    }

    let blueprint_report = state.blueprints.purge_project(&project.id.to_string());
    state.blueprints.flush().map_err(|error| {
        internal_error_response(
            format!(
                "Failed to flush blueprint store after project purge {}: {}",
                project.id, error
            ),
            "PROJECT_DELETE_BLUEPRINT_FLUSH_FAILED",
        )
    })?;
    let import_cleanup = state.imports.purge_project(project.id).map_err(|error| {
        internal_error_response(
            format!(
                "Failed to purge import artifacts for project {}: {}",
                project.id, error
            ),
            "PROJECT_DELETE_IMPORT_PURGE_FAILED",
        )
    })?;

    let deleted_project_record = state.projects.delete(project.id).map_err(|error| {
        internal_error_response(
            format!("Failed to delete project record {}: {}", project.id, error),
            "PROJECT_DELETE_PROJECT_RECORD_FAILED",
        )
    })?;
    if !deleted_project_record {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project not found: {}", project_ref),
                code: Some("PROJECT_NOT_FOUND".into()),
            }),
        ));
    }

    Ok(Json(DeleteProjectResponse {
        project_id: project.id.to_string(),
        project_name: project.name,
        stopped_live_sessions,
        stopped_pipeline_sessions,
        deleted_sessions,
        deleted_session_event_files,
        deleted_cxdb_runs,
        deleted_blueprint_nodes: blueprint_report.local_nodes_deleted,
        unlinked_shared_blueprint_nodes: blueprint_report.shared_nodes_unlinked,
        deleted_project_record,
        blueprint_events_pruned: blueprint_report.event_entries_pruned,
        blueprint_history_snapshots_pruned: blueprint_report.history_snapshots_pruned,
        deleted_import_jobs: import_cleanup.jobs_deleted,
        deleted_import_drafts: import_cleanup.drafts_deleted,
        deleted_import_managed_roots: import_cleanup.managed_roots_deleted,
    }))
}

async fn list_project_sessions(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<ListSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let sessions = state.sessions.list_summaries_for_user_project(
        &claims.sub,
        project.id,
        query.include_archived,
    );
    Ok(Json(ListSessionsResponse { sessions }))
}

async fn create_project_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
    req: Option<Json<CreateProjectSessionRequest>>,
) -> Result<(StatusCode, Json<CreateSessionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let req = req
        .map(|Json(body)| body)
        .unwrap_or(CreateProjectSessionRequest {
            title: None,
            description: None,
        });
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let session = state.sessions.create(&claims.sub);
    let updated = state.sessions.update(session.id, |draft| {
        draft.project_id = Some(project.id);
        draft.project_slug = Some(project.slug.clone());
        draft.project_name = Some(project.name.clone());
        draft.cxdb_project_id = Some(project.id);
        if let Some(description) = req.description.as_deref() {
            if !description.trim().is_empty() {
                draft.project_description = Some(description.trim().to_string());
                draft.ensure_title_from_description();
            }
        }
        if let Some(title) = req.title.as_deref() {
            if !title.trim().is_empty() {
                draft.set_title(Some(title.trim().to_string()));
            }
        }
    });

    let session = updated.unwrap_or(session);
    Ok((StatusCode::CREATED, Json(CreateSessionResponse { session })))
}

async fn get_project_events(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(project_ref): Path<String>,
) -> Result<Json<ProjectEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = resolve_project_for_user(&state, &claims, &project_ref)?;
    let sessions = state
        .sessions
        .list_for_user_project(&claims.sub, project.id);

    let mut events = sessions
        .into_iter()
        .flat_map(|session| session.events)
        .collect::<Vec<_>>();
    events.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    let count = events.len();

    Ok(Json(ProjectEventsResponse {
        project_id: project.id.to_string(),
        events,
        count,
    }))
}

async fn list_sessions(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(query): Query<ListSessionsQuery>,
) -> Json<ListSessionsResponse> {
    let sessions = state
        .sessions
        .list_summaries_for_user(&claims.sub, query.include_archived);
    Json(ListSessionsResponse { sessions })
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    req: Option<Json<CreateSessionRequest>>,
) -> Result<(StatusCode, Json<CreateSessionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let requested_project_ref = req.and_then(|Json(body)| body.project_ref);
    let resolved_project = requested_project_ref
        .as_deref()
        .map(|project_ref| resolve_project_for_user(&state, &claims, project_ref))
        .transpose()?;

    let session = state.sessions.create(&claims.sub);

    if let Some(project) = resolved_project {
        let _ = state.sessions.update(session.id, |draft| {
            draft.project_id = Some(project.id);
            draft.project_slug = Some(project.slug.clone());
            draft.project_name = Some(project.name.clone());
            draft.cxdb_project_id = Some(project.id);
        });
    }

    let session = state.sessions.get(session.id).unwrap_or(session);
    tracing::info!("Created session: {} for user: {}", session.id, claims.sub);

    Ok((StatusCode::CREATED, Json(CreateSessionResponse { session })))
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => Ok(Json(GetSessionResponse { session })),
        Err(Some(())) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Access denied".into(),
                code: None,
            }),
        )),
        Err(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

fn prompt_focus_category_id(prompt: &PromptEnvelope) -> Option<&str> {
    prompt
        .origin_category_id
        .as_deref()
        .or_else(|| prompt.category_path.last().map(|entry| entry.category_id.as_str()))
}

fn category_snapshot_node<'a>(
    snapshot: &'a SocraticCategorySnapshot,
    category_id: &str,
) -> Option<&'a planner_schemas::SocraticCategoryNode> {
    snapshot
        .nodes
        .iter()
        .find(|node| node.category_id == category_id)
}

fn category_status_label(status: &planner_schemas::SocraticCategoryStatus) -> &'static str {
    match status {
        planner_schemas::SocraticCategoryStatus::Pending => "pending",
        planner_schemas::SocraticCategoryStatus::Active => "active",
        planner_schemas::SocraticCategoryStatus::Ready => "ready",
        planner_schemas::SocraticCategoryStatus::Complete => "complete",
        planner_schemas::SocraticCategoryStatus::Blocked => "blocked",
    }
}

fn prompt_bank_response(session: &Session) -> GetSessionPromptBankResponse {
    let checkpoint = session.checkpoint.as_ref();
    let prompt = checkpoint.and_then(|checkpoint| checkpoint.current_prompt.clone());
    let snapshot = checkpoint.and_then(|checkpoint| checkpoint.current_category_snapshot.as_ref());

    let active_thread_id = prompt.as_ref().and_then(prompt_focus_category_id).map(str::to_string);

    let banked_threads = prompt
        .map(|prompt| {
            let category_id = prompt_focus_category_id(&prompt)
                .map(str::to_string)
                .unwrap_or_else(|| prompt.prompt_id.clone());
            let fallback_title = prompt
                .category_path
                .last()
                .map(|entry| entry.title.clone())
                .unwrap_or_else(|| prompt.title.clone());
            let title = snapshot
                .and_then(|snapshot| category_snapshot_node(snapshot, &category_id))
                .map(|node| node.title.clone())
                .unwrap_or(fallback_title);
            let summary = snapshot
                .and_then(|snapshot| category_snapshot_node(snapshot, &category_id))
                .map(|node| node.summary.clone())
                .unwrap_or_else(|| {
                    prompt
                        .instructions
                        .clone()
                        .unwrap_or_else(|| "Questions are ready to answer.".into())
                });

            vec![PromptBankThread {
                category_id,
                title,
                summary,
                question_count: prompt.items.len().max(1),
                prompt,
            }]
        })
        .unwrap_or_default();

    let queued_threads = snapshot
        .map(|snapshot| {
            snapshot
                .nodes
                .iter()
                .filter(|node| node.has_prompt_ready)
                .filter(|node| Some(node.category_id.as_str()) != active_thread_id.as_deref())
                .map(|node| QueuedPromptThread {
                    category_id: node.category_id.clone(),
                    title: node.title.clone(),
                    summary: node.summary.clone(),
                    question_count: node.item_count_hint.max(1) as usize,
                    status: category_status_label(&node.status).to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    GetSessionPromptBankResponse {
        session_id: session.id.to_string(),
        active_thread_id,
        banked_threads,
        queued_threads,
        build_ready: snapshot.map(|snapshot| snapshot.build_ready).unwrap_or(false),
        build_readiness_message: snapshot.map(|snapshot| snapshot.build_readiness_message.clone()),
    }
}

async fn get_session_prompt_bank(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSessionPromptBankResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => Ok(Json(prompt_bank_response(&session))),
        Err(Some(())) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Access denied".into(),
                code: None,
            }),
        )),
        Err(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

async fn update_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<Json<GetSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => session,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    if req.title.is_none() && req.archived.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No session changes were requested".into(),
                code: Some("empty_update".into()),
            }),
        ));
    }

    if req
        .title
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(false)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Session title cannot be empty".into(),
                code: Some("invalid_title".into()),
            }),
        ));
    }

    if req.archived == Some(true)
        && (session.intake_phase == "interviewing" || session.pipeline_running)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Active sessions cannot be archived".into(),
                code: Some("archive_conflict".into()),
            }),
        ));
    }

    let updated = state.sessions.update(id, |session| {
        if let Some(title) = req.title.as_ref() {
            session.set_title(Some(title.clone()));
        }
        if let Some(archived) = req.archived {
            session.set_archived(archived);
        }
    });

    match updated {
        Some(session) => Ok(Json(GetSessionResponse { session })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

async fn duplicate_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    Json(req): Json<DuplicateSessionRequest>,
) -> Result<(StatusCode, Json<GetSessionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => session,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    if req
        .title
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(false)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Duplicate session title cannot be empty".into(),
                code: Some("invalid_title".into()),
            }),
        ));
    }

    let duplicate = state
        .sessions
        .insert(session.duplicate_for_branch(req.title));
    Ok((
        StatusCode::CREATED,
        Json(GetSessionResponse { session: duplicate }),
    ))
}

async fn export_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => Ok(Json(SessionExportResponse {
            exported_at: chrono::Utc::now().to_rfc3339(),
            session,
        })),
        Err(Some(())) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Access denied".into(),
                code: None,
            }),
        )),
        Err(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StopActiveSessionWorkReport {
    pub stopped_live_session: bool,
    pub stopped_pipeline_session: bool,
}

fn stop_session_runtime(state: &Arc<AppState>, session_id: Uuid) -> bool {
    if let Some(runtime) = state.socratic_runtimes.remove(session_id) {
        runtime.close_input();
        runtime.signal_closed();
        true
    } else {
        false
    }
}

pub(crate) fn stop_active_session_work(
    state: &Arc<AppState>,
    session_id: Uuid,
) -> StopActiveSessionWorkReport {
    let stopped_live_session = stop_session_runtime(state, session_id);
    let stopped_pipeline_session = state.pipeline_runtimes.stop(session_id).is_some();
    if stopped_live_session || stopped_pipeline_session {
        let _ = state.sessions.update(session_id, |session| {
            if session.pipeline_running || stopped_pipeline_session {
                session.pipeline_running = false;
            }
            if session.intake_phase == "pipeline_running" {
                session.intake_phase = "waiting".into();
            }
            if stopped_live_session {
                session.interview_live_attached = false;
                session.interview_runtime_active = false;
            }
        });
    }
    StopActiveSessionWorkReport {
        stopped_live_session,
        stopped_pipeline_session,
    }
}

fn internal_error_response(
    message: impl Into<String>,
    code: &'static str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
            code: Some(code.into()),
        }),
    )
}

fn persist_session_events_if_available(state: &Arc<AppState>, session_id: Uuid) {
    if let Some(ref store) = state.event_store {
        if let Some(session) = state.sessions.get(session_id) {
            if let Err(error) = store.save_session_events(session_id, &session.events) {
                tracing::warn!(
                    "Failed to persist events for session {}: {}",
                    session_id,
                    error
                );
            }
        }
    }
}

fn record_session_event(
    state: &Arc<AppState>,
    session_id: Uuid,
    mut event: planner_core::observability::PlannerEvent,
) {
    if event.session_id.is_none() {
        event = event.with_session(session_id);
    }

    state.sessions.update(session_id, |s| {
        s.record_event(event.clone());
    });
    persist_session_events_if_available(state, session_id);
}

pub(crate) fn spawn_pipeline_runtime(
    state: Arc<AppState>,
    session_id: Uuid,
    description: String,
) -> bool {
    let (runtime, mut shutdown_rx) = crate::runtime::SessionPipelineRuntime::new();
    if state
        .pipeline_runtimes
        .insert(session_id, runtime.clone())
        .is_err()
    {
        tracing::warn!(
            "Session {}: pipeline runtime already registered; skipping duplicate spawn",
            session_id
        );
        return false;
    }

    let state_for_task = state.clone();
    let join_handle = tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                tracing::info!("Session {}: pipeline runtime shutdown signal received", session_id);
            }
            _ = run_pipeline_for_session(state_for_task.clone(), session_id, description) => {}
        }
        let _ = state_for_task.pipeline_runtimes.remove(session_id);
    });
    runtime.set_abort_handle(join_handle.abort_handle());
    true
}

async fn restart_from_description(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => session,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    if !session
        .project_description
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Restart from description is unavailable for this session".into(),
                code: Some("restart_unavailable".into()),
            }),
        ));
    }

    if session.pipeline_running {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Cannot restart while the pipeline is still running".into(),
                code: Some("pipeline_running".into()),
            }),
        ));
    }

    stop_active_session_work(&state, id);

    let session = state.sessions.update(id, |s| {
        s.reset_for_interview_restart();
    });

    match session {
        Some(session) => Ok(Json(GetSessionResponse { session })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

async fn retry_pipeline(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => session,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    if session.pipeline_running {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Pipeline is already running for this session".into(),
                code: Some("pipeline_running".into()),
            }),
        ));
    }

    if !session
        .project_description
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || !session.pipeline_has_failed()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Retry pipeline is unavailable for this session".into(),
                code: Some("retry_unavailable".into()),
            }),
        ));
    }

    stop_active_session_work(&state, id);

    let description = session.project_description.clone().unwrap_or_default();
    let session = state.sessions.update(id, |s| {
        s.prepare_for_pipeline_retry();
        s.add_message("planner", "Retrying pipeline from the saved description.");
    });

    match session {
        Some(session) => {
            let _ = spawn_pipeline_runtime(state.clone(), id, description);
            Ok(Json(GetSessionResponse { session }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Message content cannot be empty".into(),
                code: None,
            }),
        ));
    }

    // Ownership check first (read-only — no dirty marking).
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(_) => {}
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    }

    // Now update — ownership is verified, no wasted dirty marking.
    let mut should_spawn_pipeline = false;

    let result = state.sessions.update(id, |session| {
        session.add_message("user", &content);
        session.set_archived(false);

        if !session.pipeline_running {
            session.pipeline_running = true;
            session.project_description = Some(content.clone());
            session.ensure_title_from_description();
            session.stages[0].status = "running".into();
            should_spawn_pipeline = true;

            session.add_message(
                "planner",
                &format!(
                    "Starting pipeline for: \"{}\". Running the full Dark Factory pipeline — \
                     this may take several minutes.\n\n\
                     Poll GET /api/sessions/{} to check progress, or connect to the WebSocket \
                     at /api/sessions/{}/ws.",
                    content, session.id, session.id
                ),
            );
        } else {
            session.add_message(
                "planner",
                "Pipeline is currently running. Interactive follow-up during execution \
                 will be available in a future version.",
            );
        }
    });

    match result {
        Some(mut session) => {
            // Touch to extend expiry after a real user interaction.
            state.sessions.touch(id);

            // Spawn pipeline only if this request transitioned it to running.
            if should_spawn_pipeline {
                if let Err(error) = ensure_session_project_assignment(&state, id, &content) {
                    tracing::warn!(
                        "Session {}: failed to assign project before pipeline start: {}",
                        id,
                        error
                    );
                } else if let Some(refreshed) = state.sessions.get(id) {
                    session = refreshed;
                }

                let _ = spawn_pipeline_runtime(state.clone(), id, content.clone());
            }

            // Use safe index access for the response messages.
            let msgs = &session.messages;
            let planner_msg =
                msgs.last()
                    .cloned()
                    .unwrap_or_else(|| crate::session::SessionMessage {
                        id: uuid::Uuid::new_v4(),
                        role: "planner".into(),
                        content: "(no response)".into(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
            let user_msg = msgs.iter().rev().nth(1).cloned().unwrap_or_else(|| {
                crate::session::SessionMessage {
                    id: uuid::Uuid::new_v4(),
                    role: "user".into(),
                    content: content.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            });

            Ok(Json(SendMessageResponse {
                user_message: user_msg,
                planner_message: planner_msg,
                session,
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

// ---------------------------------------------------------------------------
// Pipeline background task
// ---------------------------------------------------------------------------

/// Background task: runs the full pipeline and writes results back to the
/// session store. Clients observe progress via REST polling or WebSocket.
pub async fn run_pipeline_for_session(state: Arc<AppState>, session_id: Uuid, description: String) {
    tracing::info!("Session {}: pipeline task started", session_id);

    let router = state.llm_router.clone();

    let worker = match planner_core::pipeline::steps::factory_worker::CodexFactoryWorker::new() {
        Ok(w) => w,
        Err(e) => {
            record_session_event(
                &state,
                session_id,
                planner_core::observability::PlannerEvent::error(
                    planner_core::observability::EventSource::Pipeline,
                    "pipeline.stage.failed",
                    format!("Pipeline setup failed: {}", e),
                )
                .with_metadata(serde_json::json!({
                    "stage": "Intake",
                    "terminal": true,
                    "retry_planned": false,
                    "details": {
                        "error": e.to_string(),
                        "kind": "factory_worker_init",
                    }
                })),
            );
            state.sessions.update(session_id, |s| {
                s.add_message("planner", &format!("Pipeline setup failed: {}", e));
                s.pipeline_running = false;
                s.intake_phase = "error".into();
                s.error_message = Some(format!("Pipeline setup failed: {}", e));
            });
            return;
        }
    };

    let project = match ensure_session_project_assignment(&state, session_id, &description) {
        Ok(project) => project,
        Err(error) => {
            record_session_event(
                &state,
                session_id,
                planner_core::observability::PlannerEvent::error(
                    planner_core::observability::EventSource::Pipeline,
                    "pipeline.stage.failed",
                    format!("Project assignment failed: {}", error),
                )
                .with_metadata(serde_json::json!({
                    "stage": "Intake",
                    "terminal": true,
                    "retry_planned": false,
                    "details": {
                        "error": error.to_string(),
                        "kind": "project_assignment",
                    }
                })),
            );
            state.sessions.update(session_id, |s| {
                s.add_message("planner", &format!("Project assignment failed: {}", error));
                s.pipeline_running = false;
                s.intake_phase = "error".into();
                s.error_message = Some(format!("Project assignment failed: {}", error));
            });
            return;
        }
    };
    let project_id = project.id;
    let run_id = Uuid::new_v4();

    state.sessions.update(session_id, |s| {
        s.project_id = Some(project_id);
        s.project_slug = Some(project.slug.clone());
        s.project_name = Some(project.name.clone());
        s.cxdb_project_id = Some(project_id);
        if !s.run_ids.contains(&run_id) {
            s.run_ids.push(run_id);
        }
    });

    // Build PipelineConfig with durable storage if available.
    // We branch on whether CXDB is available to avoid holding a borrow
    // across the async pipeline call.
    let cxdb_ref = state.cxdb.as_ref();

    if let Some(engine) = cxdb_ref {
        if let Err(e) = engine.register_run(project_id, run_id) {
            tracing::warn!("CXDB: failed to register run: {}", e);
        }
    }

    let (pipeline_event_sink, mut pipeline_event_rx) =
        planner_core::observability::ChannelEventSink::new();
    let pipeline_event_sink = Arc::new(pipeline_event_sink);

    let run_result = match cxdb_ref {
        Some(engine) => {
            let config = planner_core::pipeline::PipelineConfig {
                router: router.as_ref(),
                store: Some(engine),
                dtu_registry: None,
                blueprints: Some(&state.blueprints),
                event_sink: Some(pipeline_event_sink.as_ref()),
            };

            let mut pipeline_future =
                Box::pin(planner_core::pipeline::run_full_pipeline_with_run_id(
                    &config,
                    &worker,
                    project_id,
                    run_id,
                    &description,
                ));

            loop {
                tokio::select! {
                    maybe_event = pipeline_event_rx.recv() => {
                        if let Some(event) = maybe_event {
                            record_session_event(&state, session_id, event);
                        }
                    }
                    result = &mut pipeline_future => {
                        break result;
                    }
                }
            }
        }
        None => {
            // No durable storage — run with in-memory CxdbEngine (store: None).
            let config = planner_core::pipeline::PipelineConfig::<planner_core::cxdb::CxdbEngine> {
                router: router.as_ref(),
                store: None,
                dtu_registry: None,
                blueprints: Some(&state.blueprints),
                event_sink: Some(pipeline_event_sink.as_ref()),
            };

            let mut pipeline_future =
                Box::pin(planner_core::pipeline::run_full_pipeline_with_run_id(
                    &config,
                    &worker,
                    project_id,
                    run_id,
                    &description,
                ));

            loop {
                tokio::select! {
                    maybe_event = pipeline_event_rx.recv() => {
                        if let Some(event) = maybe_event {
                            record_session_event(&state, session_id, event);
                        }
                    }
                    result = &mut pipeline_future => {
                        break result;
                    }
                }
            }
        }
    };

    while let Ok(event) = pipeline_event_rx.try_recv() {
        record_session_event(&state, session_id, event);
    }

    match run_result {
        Ok(output) => {
            state.sessions.update(session_id, |s| {
                s.add_message(
                    "planner",
                    &format!(
                        "Pipeline complete!\n\nProject: {}\nSpecs: {} chunk(s)\nFactory: {:?}",
                        output.front_office.intake.project_name,
                        output.front_office.specs.len(),
                        output.factory_output.build_status,
                    ),
                );
                s.pipeline_running = false;
                s.intake_phase = "complete".into();
                s.error_message = None;
            });
            tracing::info!("Session {}: pipeline complete", session_id);
        }
        Err(error) => {
            let failing_stage = state
                .sessions
                .get(session_id)
                .and_then(|s| {
                    s.stages
                        .iter()
                        .find(|stage| stage.status == "running")
                        .map(|stage| stage.name.clone())
                })
                .unwrap_or_else(|| String::from("Intake"));

            record_session_event(
                &state,
                session_id,
                planner_core::observability::PlannerEvent::error(
                    planner_core::observability::EventSource::Pipeline,
                    "pipeline.stage.failed",
                    format!(
                        "Pipeline failed during stage '{}': {}",
                        failing_stage, error
                    ),
                )
                .with_metadata(serde_json::json!({
                    "stage": failing_stage,
                    "terminal": true,
                    "retry_planned": false,
                    "details": {
                        "error": error.to_string(),
                    }
                })),
            );

            state.sessions.update(session_id, |s| {
                s.add_message("planner", &format!("Pipeline failed: {}", error));
                s.pipeline_running = false;
                s.intake_phase = "error".into();
                s.error_message = Some(format!("Pipeline failed: {}", error));
            });
            tracing::warn!("Session {}: pipeline failed: {}", session_id, error);
        }
    }
}

// ---------------------------------------------------------------------------
// CXDB Read API handlers (Change 4)
// ---------------------------------------------------------------------------

/// List all Turns for a session (metadata only).
///
/// Queries the durable CXDB engine using the session's run_ids index.
/// Returns an empty list if no CXDB is configured or no pipeline has run.
async fn list_turns(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<ListTurnsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify session exists and belongs to the requesting user (read-only).
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(s) => s,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    // Query CXDB for turns belonging to this session's run IDs.
    let turns =
        match &state.cxdb {
            Some(engine) => {
                let mut entries = Vec::new();
                for run_id in &session.run_ids {
                    entries.extend(engine.list_turn_metadata_for_run(*run_id).into_iter().map(
                        |m| TurnResponse {
                            turn_id: m.turn_id,
                            type_id: m.type_id,
                            timestamp: m.timestamp,
                            produced_by: m.produced_by,
                        },
                    ));
                }
                entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
                entries
            }
            None => Vec::new(),
        };

    let count = turns.len();
    Ok(Json(ListTurnsResponse { turns, count }))
}

/// List all pipeline run IDs for a session.
///
/// Returns the session-owned run index. This keeps history session-local
/// even when multiple sessions share a canonical project UUID.
async fn list_runs(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<RunListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify session exists and belongs to the requesting user (read-only).
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(s) => s,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    let runs = session
        .run_ids
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>();

    Ok(Json(RunListResponse { runs }))
}

// ---------------------------------------------------------------------------
// Events endpoint
// ---------------------------------------------------------------------------

/// GET /sessions/{id}/events
///
/// Return the structured observability event log for a session,
/// with optional filtering by level, source, and pagination.
async fn get_session_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    claims: Claims,
    axum::extract::Query(query): axum::extract::Query<EventsQuery>,
) -> Result<Json<SessionEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid session ID".into(),
                code: None,
            }),
        )
    })?;

    let session = match state.sessions.get_if_owned(session_id, &claims.sub) {
        Ok(s) => s,
        Err(Some(())) => {
            // Allow dev|local sessions to be read by anyone (dev mode compat).
            match state.sessions.get(session_id) {
                Some(s) if s.user_id == "dev|local" => s,
                _ => {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            error: "Not your session".into(),
                            code: None,
                        }),
                    ))
                }
            }
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Session not found".into(),
                    code: None,
                }),
            ));
        }
    };

    let mut events: Vec<planner_core::observability::PlannerEvent> = session.events.clone();

    // Filter by level
    if let Some(ref level) = query.level {
        let target_level = match level.to_lowercase().as_str() {
            "info" => Some(planner_core::observability::EventLevel::Info),
            "warn" => Some(planner_core::observability::EventLevel::Warn),
            "error" => Some(planner_core::observability::EventLevel::Error),
            _ => None,
        };
        if let Some(target) = target_level {
            events.retain(|e| e.level == target);
        }
    }

    // Filter by source
    if let Some(ref source) = query.source {
        let target_source = match source.to_lowercase().as_str() {
            "socratic" | "socratic_engine" => {
                Some(planner_core::observability::EventSource::SocraticEngine)
            }
            "llm" | "llm_router" => Some(planner_core::observability::EventSource::LlmRouter),
            "pipeline" => Some(planner_core::observability::EventSource::Pipeline),
            "factory" => Some(planner_core::observability::EventSource::Factory),
            "system" => Some(planner_core::observability::EventSource::System),
            _ => None,
        };
        if let Some(target) = target_source {
            events.retain(|e| e.source == target);
        }
    }

    // Pagination
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(500);
    let total = events.len();
    let events: Vec<_> = events.into_iter().skip(offset).take(limit).collect();

    Ok(Json(SessionEventsResponse {
        session_id: id,
        events,
        count: total,
    }))
}

// ---------------------------------------------------------------------------
// WebSocket handler
// ---------------------------------------------------------------------------

async fn ws_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Verify the session exists and belongs to the user (read-only).
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(_) => {
            // Touch to extend expiry — WebSocket connect is a real interaction.
            state.sessions.touch(id);
            ws.on_upgrade(move |socket| ws::handle_ws(socket, state, id))
        }
        Err(Some(())) => (StatusCode::FORBIDDEN, "Access denied").into_response(),
        Err(None) => (StatusCode::NOT_FOUND, "Session not found").into_response(),
    }
}

// ---------------------------------------------------------------------------
// Socratic interview handlers
// ---------------------------------------------------------------------------

/// POST /api/sessions/:id/socratic
///
/// Start a Socratic interview for an existing session, or create a new one.
/// Returns the session ID and the WebSocket URL to connect to.
async fn start_socratic(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    Json(req): Json<StartSocraticRequest>,
) -> Result<Json<StartSocraticResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership (read-only).
    let session = match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => session,
        Err(Some(())) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Access denied".into(),
                    code: None,
                }),
            ));
        }
        Err(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                    code: None,
                }),
            ));
        }
    };

    let project = if let Some(project_ref) = req.project_ref.as_deref() {
        let project = resolve_project_for_user(&state, &claims, project_ref)?;
        if session.project_id != Some(project.id)
            && (!session.run_ids.is_empty() || session.pipeline_running)
        {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Project reassignment is only allowed before pipeline execution starts"
                        .into(),
                    code: Some("PROJECT_REASSIGNMENT_BLOCKED".into()),
                }),
            ));
        }
        project
    } else {
        ensure_session_project_assignment(&state, id, &req.description).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error,
                    code: Some("PROJECT_ASSIGNMENT_FAILED".into()),
                }),
            )
        })?
    };

    // Store the initial description in the session for reference.
    stop_active_session_work(&state, id);
    state.sessions.update(id, |s| {
        s.project_description = Some(req.description.clone());
        s.ensure_title_from_description();
        s.set_archived(false);
        s.intake_phase = "interviewing".into();
        s.interview_live_attached = false;
        s.interview_runtime_active = false;
        s.project_id = Some(project.id);
        s.project_slug = Some(project.slug.clone());
        s.project_name = Some(project.name.clone());
        if s.cxdb_project_id.is_none() {
            s.cxdb_project_id = Some(project.id);
        }
        s.ensure_socratic_run_id();
        s.checkpoint = None;
        s.has_checkpoint = false;
    });

    // Touch to extend expiry.
    state.sessions.touch(id);

    Ok(Json(StartSocraticResponse {
        session_id: id.to_string(),
        ws_url: format!("/api/sessions/{}/socratic/ws", id),
    }))
}

/// GET /api/sessions/:id/socratic/ws
///
/// WebSocket upgrade for the Socratic interview handler.
async fn socratic_ws_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(_) => {
            state.sessions.touch(id);
            ws.on_upgrade(move |socket| ws_socratic::handle_socratic_ws(socket, state, id))
        }
        Err(Some(())) => (StatusCode::FORBIDDEN, "Access denied").into_response(),
        Err(None) => (StatusCode::NOT_FOUND, "Session not found").into_response(),
    }
}

/// GET /api/sessions/:id/belief-state
///
/// Return the current belief state for a session.
async fn get_belief_state(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<BeliefStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get_if_owned(id, &claims.sub) {
        Ok(session) => {
            let belief_state = match &session.belief_state {
                Some(bs) => serde_json::to_value(bs).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            Ok(Json(BeliefStateResponse {
                session_id: id.to_string(),
                intake_phase: session.intake_phase.clone(),
                belief_state,
            }))
        }
        Err(Some(())) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Access denied".into(),
                code: None,
            }),
        )),
        Err(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
                code: None,
            }),
        )),
    }
}

fn has_any_secondary_scope(
    scope: &planner_schemas::artifacts::blueprint::SecondaryScopeRefs,
) -> bool {
    [
        scope.feature.as_deref(),
        scope.widget.as_deref(),
        scope.artifact.as_deref(),
        scope.component.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}

fn scope_visibility_for_node(
    node: &planner_schemas::artifacts::blueprint::NodeSummary,
) -> planner_schemas::artifacts::blueprint::ScopeVisibility {
    if matches!(
        node.scope_class,
        planner_schemas::artifacts::blueprint::ScopeClass::Unscoped
    ) {
        planner_schemas::artifacts::blueprint::ScopeVisibility::Unscoped
    } else if node.is_shared {
        planner_schemas::artifacts::blueprint::ScopeVisibility::Shared
    } else {
        planner_schemas::artifacts::blueprint::ScopeVisibility::ProjectLocal
    }
}

fn matches_project_scope(
    node: &planner_schemas::artifacts::blueprint::NodeSummary,
    query: &NodesQuery,
) -> bool {
    let Some(project_id) = query.project_id.as_deref() else {
        return true;
    };

    let is_project_local = node.project_id.as_deref() == Some(project_id)
        && matches!(
            node.scope_class,
            planner_schemas::artifacts::blueprint::ScopeClass::Project
                | planner_schemas::artifacts::blueprint::ScopeClass::ProjectContextual
        );

    let is_inherited_shared = query.include_shared
        && node.is_shared
        && node
            .linked_project_ids
            .iter()
            .any(|linked| linked == project_id);

    let is_global = query.include_global
        && matches!(
            node.scope_class,
            planner_schemas::artifacts::blueprint::ScopeClass::Global
        );

    is_project_local || is_inherited_shared || is_global
}

fn matches_secondary_scope(
    node: &planner_schemas::artifacts::blueprint::NodeSummary,
    query: &NodesQuery,
) -> bool {
    if query.feature.as_deref() != node.secondary_scope.feature.as_deref()
        && query.feature.is_some()
    {
        return false;
    }
    if query.widget.as_deref() != node.secondary_scope.widget.as_deref() && query.widget.is_some() {
        return false;
    }
    if query.artifact.as_deref() != node.secondary_scope.artifact.as_deref()
        && query.artifact.is_some()
    {
        return false;
    }
    if query.component.as_deref() != node.secondary_scope.component.as_deref()
        && query.component.is_some()
    {
        return false;
    }
    true
}

fn filter_node_summaries(
    mut summaries: Vec<planner_schemas::artifacts::blueprint::NodeSummary>,
    query: &NodesQuery,
) -> Vec<planner_schemas::artifacts::blueprint::NodeSummary> {
    summaries.retain(|node| {
        if query
            .node_type
            .as_deref()
            .is_some_and(|t| node.node_type != t)
        {
            return false;
        }
        if query
            .scope_class
            .as_ref()
            .is_some_and(|scope_class| node.scope_class != *scope_class)
        {
            return false;
        }
        if query
            .scope_visibility
            .as_ref()
            .is_some_and(|visibility| scope_visibility_for_node(node) != *visibility)
        {
            return false;
        }
        if query
            .lifecycle
            .as_ref()
            .is_some_and(|lifecycle| node.lifecycle != *lifecycle)
        {
            return false;
        }
        if !matches_project_scope(node, query) {
            return false;
        }
        matches_secondary_scope(node, query)
    });
    summaries
}

fn node_tags_mut(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) -> &mut Vec<String> {
    use planner_schemas::artifacts::blueprint::BlueprintNode;
    match node {
        BlueprintNode::Project(n) => &mut n.tags,
        BlueprintNode::Decision(n) => &mut n.tags,
        BlueprintNode::Technology(n) => &mut n.tags,
        BlueprintNode::Component(n) => &mut n.tags,
        BlueprintNode::Constraint(n) => &mut n.tags,
        BlueprintNode::Pattern(n) => &mut n.tags,
        BlueprintNode::QualityRequirement(n) => &mut n.tags,
    }
}

fn node_scope_mut(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) -> &mut planner_schemas::artifacts::blueprint::NodeScope {
    use planner_schemas::artifacts::blueprint::BlueprintNode;
    match node {
        BlueprintNode::Project(n) => &mut n.scope,
        BlueprintNode::Decision(n) => &mut n.scope,
        BlueprintNode::Technology(n) => &mut n.scope,
        BlueprintNode::Component(n) => &mut n.scope,
        BlueprintNode::Constraint(n) => &mut n.scope,
        BlueprintNode::Pattern(n) => &mut n.scope,
        BlueprintNode::QualityRequirement(n) => &mut n.scope,
    }
}

fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn patch_declares_component_name_source(patch: &serde_json::Value) -> bool {
    patch
        .get("naming")
        .and_then(|naming| naming.get("source"))
        .is_some()
}

fn normalize_component_naming(
    previous: Option<&planner_schemas::artifacts::blueprint::BlueprintNode>,
    patch: &serde_json::Value,
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) {
    use planner_schemas::artifacts::blueprint::{
        BlueprintNode, ComponentNameSource, ComponentNaming, ComponentNamingStrategy,
    };

    let Some(next_component) = (match node {
        BlueprintNode::Component(component) => Some(component),
        _ => None,
    }) else {
        return;
    };

    let previous_component = previous.and_then(|existing| match existing {
        BlueprintNode::Component(component) => Some(component),
        _ => None,
    });

    let previous_name = previous_component
        .map(|component| normalize_ws(&component.name))
        .unwrap_or_default();
    let next_name = normalize_ws(&next_component.name);
    let name_changed = previous_component
        .map(|_| !next_name.is_empty() && previous_name != next_name)
        .unwrap_or(false);

    if next_component.naming.is_none() {
        let origin_key = previous_component
            .and_then(|component| {
                component
                    .naming
                    .as_ref()
                    .map(|naming| naming.origin_key.clone())
            })
            .unwrap_or_else(|| format!("manual:{}", next_component.id));
        let generated_name = previous_component
            .and_then(|component| {
                component
                    .naming
                    .as_ref()
                    .map(|naming| naming.generated_name.clone())
            })
            .unwrap_or_else(|| next_component.name.clone());
        let strategy = previous_component
            .and_then(|component| {
                component
                    .naming
                    .as_ref()
                    .map(|naming| naming.strategy.clone())
            })
            .unwrap_or(ComponentNamingStrategy::ManualCreate);
        let source = previous_component
            .and_then(|component| {
                component
                    .naming
                    .as_ref()
                    .map(|naming| naming.source.clone())
            })
            .unwrap_or(ComponentNameSource::Manual);

        next_component.naming = Some(ComponentNaming {
            origin_key,
            source,
            strategy,
            generated_name,
            naming_version: 1,
            last_generated_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    if let Some(naming) = next_component.naming.as_mut() {
        if naming.origin_key.trim().is_empty() {
            naming.origin_key = previous_component
                .and_then(|component| {
                    component
                        .naming
                        .as_ref()
                        .map(|previous_naming| previous_naming.origin_key.clone())
                })
                .unwrap_or_else(|| format!("manual:{}", next_component.id));
        }

        if naming.generated_name.trim().is_empty() {
            naming.generated_name = previous_component
                .and_then(|component| {
                    component
                        .naming
                        .as_ref()
                        .map(|previous_naming| previous_naming.generated_name.clone())
                })
                .unwrap_or_else(|| next_component.name.clone());
        }

        if naming.naming_version == 0 {
            naming.naming_version = 1;
        }

        if name_changed && !patch_declares_component_name_source(patch) {
            naming.source = ComponentNameSource::Manual;
            if let Some(previous_generated) = previous_component.and_then(|component| {
                component
                    .naming
                    .as_ref()
                    .map(|previous_naming| previous_naming.generated_name.clone())
            }) {
                naming.generated_name = previous_generated;
            }
        }

        naming.last_generated_at = chrono::Utc::now().to_rfc3339();
    }
}

fn normalize_blueprint_node_metadata(
    node: &mut planner_schemas::artifacts::blueprint::BlueprintNode,
) {
    const ARCHIVED_TAG: &str = "archived";
    const OVERRIDE_PREFIX: &str = "overrides:";

    let mut seen = std::collections::HashSet::new();
    let mut migrated_archived = false;
    let mut migrated_override_source: Option<String> = None;
    let tags = node_tags_mut(node);
    let mut normalized_tags = Vec::with_capacity(tags.len());

    for raw_tag in tags.iter() {
        let trimmed = raw_tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower == ARCHIVED_TAG {
            migrated_archived = true;
            continue;
        }
        if lower.starts_with(OVERRIDE_PREFIX) {
            if migrated_override_source.is_none() {
                let source = trimmed[OVERRIDE_PREFIX.len()..].trim();
                if !source.is_empty() {
                    migrated_override_source = Some(source.to_string());
                }
            }
            continue;
        }
        if seen.insert(lower) {
            normalized_tags.push(trimmed.to_string());
        }
    }
    *tags = normalized_tags;

    let scope = node_scope_mut(node);
    if migrated_archived
        && matches!(
            scope.lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        )
    {
        scope.lifecycle = planner_schemas::artifacts::blueprint::NodeLifecycle::Archived;
    }
    if scope.override_scope.is_none() {
        if let Some(source) = migrated_override_source {
            scope.override_scope = Some(planner_schemas::artifacts::blueprint::OverrideScope {
                shared_source_id: source,
                override_reason: Some("migrated from legacy override tag".into()),
                effective_from: None,
            });
        }
    }
    if let Some(scope_review) = scope.scope_review.as_mut() {
        scope_review.deferred_reason = scope_review.deferred_reason.trim().to_string();
        scope_review.owner = scope_review.owner.trim().to_string();
        scope_review.due_at = scope_review.due_at.trim().to_string();
        let trimmed_deferred_at = scope_review
            .deferred_at
            .as_deref()
            .map(str::trim)
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        scope_review.deferred_at = trimmed_deferred_at;
        if scope_review.deferred_at.is_none() {
            scope_review.deferred_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}

fn is_valid_scope_review_due_at(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
        || chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn validate_blueprint_node_scope(
    node: &planner_schemas::artifacts::blueprint::BlueprintNode,
) -> Result<(), String> {
    let scope = node.scope();
    let project_id = scope
        .project
        .as_ref()
        .map(|project| project.project_id.trim());
    let has_project = project_id.is_some_and(|id| !id.is_empty());
    let has_secondary = has_any_secondary_scope(&scope.secondary);

    use planner_schemas::artifacts::blueprint::ScopeClass;
    match scope.scope_class {
        ScopeClass::Global => {
            if scope.project.is_some() {
                return Err("global scope cannot include project reference".into());
            }
            if has_secondary {
                return Err("global scope cannot include contextual scope".into());
            }
        }
        ScopeClass::Project => {
            if !has_project {
                return Err("project scope requires project.project_id".into());
            }
            if has_secondary {
                return Err(
                    "project scope cannot include contextual refs; use project_contextual".into(),
                );
            }
        }
        ScopeClass::ProjectContextual => {
            if !has_project {
                return Err("project_contextual scope requires project.project_id".into());
            }
            if !has_secondary {
                return Err("project_contextual scope requires at least one contextual ref".into());
            }
        }
        ScopeClass::Unscoped => {
            if scope.project.is_some() || has_secondary {
                return Err("unscoped records cannot include project or contextual scope".into());
            }
            if scope.is_shared {
                return Err("unscoped records cannot be marked shared".into());
            }
        }
    }

    if scope.is_shared {
        let shared = scope
            .shared
            .as_ref()
            .ok_or_else(|| "shared records require shared metadata".to_string())?;
        if shared.linked_project_ids.is_empty() {
            return Err("shared records require at least one linked project id".into());
        }
        if shared
            .linked_project_ids
            .iter()
            .any(|project| project.trim().is_empty())
        {
            return Err("shared linked_project_ids cannot contain blank values".into());
        }
    } else if scope.shared.is_some() {
        return Err("shared metadata is only allowed when is_shared=true".into());
    }

    if let Some(override_scope) = &scope.override_scope {
        if override_scope.shared_source_id.trim().is_empty() {
            return Err("override_scope.shared_source_id cannot be blank".into());
        }
        if scope.is_shared {
            return Err("shared records cannot define override_scope".into());
        }
        if matches!(
            scope.scope_class,
            planner_schemas::artifacts::blueprint::ScopeClass::Unscoped
                | planner_schemas::artifacts::blueprint::ScopeClass::Global
        ) {
            return Err("override_scope requires project or project_contextual scope".into());
        }
    }

    if let Some(scope_review) = &scope.scope_review {
        if !matches!(
            scope.scope_class,
            planner_schemas::artifacts::blueprint::ScopeClass::Unscoped
        ) {
            return Err("scope_review is only allowed on unscoped records".into());
        }
        if scope_review.deferred_reason.trim().is_empty() {
            return Err("scope_review.deferred_reason cannot be blank".into());
        }
        if scope_review.owner.trim().is_empty() {
            return Err("scope_review.owner cannot be blank".into());
        }
        if scope_review.due_at.trim().is_empty() {
            return Err("scope_review.due_at cannot be blank".into());
        }
        if !is_valid_scope_review_due_at(scope_review.due_at.trim()) {
            return Err("scope_review.due_at must be YYYY-MM-DD or RFC3339".into());
        }
        if let Some(deferred_at) = &scope_review.deferred_at {
            if chrono::DateTime::parse_from_rfc3339(deferred_at.trim()).is_err() {
                return Err("scope_review.deferred_at must be RFC3339".into());
            }
        }
    }

    Ok(())
}

fn validate_blueprint_override_source(
    state: &AppState,
    node: &planner_schemas::artifacts::blueprint::BlueprintNode,
) -> Result<(), String> {
    let Some(override_scope) = &node.scope().override_scope else {
        return Ok(());
    };

    let source_id = override_scope.shared_source_id.trim();
    if source_id == node.id().0 {
        return Err("override_scope.shared_source_id cannot reference the node itself".into());
    }

    let source = state
        .blueprints
        .get_node(source_id)
        .ok_or_else(|| format!("override_scope.shared_source_id not found: {}", source_id))?;

    if !source.scope().is_shared {
        return Err("override_scope.shared_source_id must reference a shared record".into());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Blueprint API handlers
// ---------------------------------------------------------------------------

/// GET /blueprint — Full blueprint graph summary.
async fn get_blueprint(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(mut query): Query<NodesQuery>,
) -> Json<BlueprintResponse> {
    canonicalize_nodes_query_project_ref(&mut query, &state);
    let bp = state.blueprints.snapshot();
    let nodes = filter_node_summaries(bp.list_summaries(), &query);
    let included_node_ids: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.id.as_str()).collect();
    let edges: Vec<EdgePayload> = bp
        .edges
        .iter()
        .filter(|edge| {
            included_node_ids.contains(edge.source.as_str())
                && included_node_ids.contains(edge.target.as_str())
        })
        .map(|e| EdgePayload {
            source: e.source.0.clone(),
            target: e.target.0.clone(),
            edge_type: e.edge_type,
            metadata: e.metadata.clone(),
        })
        .collect();

    let mut counts = std::collections::HashMap::new();
    for node in &nodes {
        *counts.entry(node.node_type.clone()).or_insert(0usize) += 1;
    }

    Json(BlueprintResponse {
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        nodes,
        edges,
        counts,
    })
}

/// GET /blueprint/nodes?type=decision — List blueprint nodes, optionally filtered.
async fn list_blueprint_nodes(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(mut query): Query<NodesQuery>,
) -> Json<NodeListResponse> {
    canonicalize_nodes_query_project_ref(&mut query, &state);
    let summaries = filter_node_summaries(state.blueprints.list_summaries(), &query);
    let count = summaries.len();
    Json(NodeListResponse {
        nodes: summaries,
        count,
    })
}

/// POST /blueprint/nodes — Create a new blueprint node.
async fn create_blueprint_node(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(mut node): Json<planner_schemas::artifacts::blueprint::BlueprintNode>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    normalize_component_naming(
        None,
        &serde_json::Value::Object(serde_json::Map::new()),
        &mut node,
    );
    normalize_blueprint_node_metadata(&mut node);
    validate_blueprint_node_scope(&node).map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message,
                code: Some("INVALID_SCOPE".into()),
            }),
        )
    })?;
    validate_blueprint_override_source(&state, &node).map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message,
                code: Some("INVALID_SCOPE".into()),
            }),
        )
    })?;

    let id = node.id().0.clone();
    state.blueprints.upsert_node(node.clone());
    tracing::info!("Blueprint node created: {}", id);
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(&node).unwrap_or_default()),
    ))
}

/// GET /blueprint/nodes/{nodeId} — Get a single blueprint node.
async fn get_blueprint_node(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(node_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.blueprints.get_node(&node_id) {
        Some(node) => Ok(Json(serde_json::to_value(&node).unwrap_or_default())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Blueprint node not found: {}", node_id),
                code: Some("NODE_NOT_FOUND".into()),
            }),
        )),
    }
}

/// PATCH /blueprint/nodes/{nodeId} — Apply a JSON Merge Patch to a node.
async fn update_blueprint_node(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(node_id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let existing_node = state.blueprints.get_node(&node_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Blueprint node not found: {}", node_id),
                code: Some("NODE_NOT_FOUND".into()),
            }),
        )
    })?;

    if let Some(patch_type) = patch.get("node_type").and_then(|value| value.as_str()) {
        if patch_type != existing_node.type_name() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "Blueprint node type mismatch: expected '{}', got '{}'",
                        existing_node.type_name(),
                        patch_type,
                    ),
                    code: Some("NODE_TYPE_MISMATCH".into()),
                }),
            ));
        }
    }

    let mut merged = serde_json::to_value(&existing_node).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to serialize existing blueprint node: {}", err),
                code: Some("SERIALIZE_FAILED".into()),
            }),
        )
    })?;

    let patch_for_component_naming = patch.clone();
    apply_json_merge_patch(&mut merged, patch);

    let mut node: planner_schemas::artifacts::blueprint::BlueprintNode =
        serde_json::from_value(merged).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid blueprint patch payload: {}", err),
                    code: Some("INVALID_NODE_PATCH".into()),
                }),
            )
        })?;

    if node.id().0 != node_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Blueprint node ID mismatch: payload '{}' does not match path '{}'",
                    node.id(),
                    node_id,
                ),
                code: Some("NODE_ID_MISMATCH".into()),
            }),
        ));
    }

    normalize_component_naming(Some(&existing_node), &patch_for_component_naming, &mut node);
    normalize_blueprint_node_metadata(&mut node);

    validate_blueprint_node_scope(&node).map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message,
                code: Some("INVALID_SCOPE".into()),
            }),
        )
    })?;
    validate_blueprint_override_source(&state, &node).map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message,
                code: Some("INVALID_SCOPE".into()),
            }),
        )
    })?;

    state.blueprints.upsert_node(node.clone());
    tracing::info!("Blueprint node updated: {}", node_id);
    Ok(Json(serde_json::to_value(&node).unwrap_or_default()))
}

/// DELETE /blueprint/nodes/{nodeId} — Delete a node and incident edges.
async fn delete_blueprint_node(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(node_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.blueprints.remove_node(&node_id) {
        Some(_) => {
            tracing::info!("Blueprint node deleted: {}", node_id);
            Ok(StatusCode::NO_CONTENT)
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Blueprint node not found: {}", node_id),
                code: Some("NODE_NOT_FOUND".into()),
            }),
        )),
    }
}

/// POST /blueprint/edges — Add an edge between two nodes.
async fn create_blueprint_edge(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(payload): Json<EdgePayload>,
) -> Result<(StatusCode, Json<EdgePayload>), (StatusCode, Json<ErrorResponse>)> {
    use planner_schemas::artifacts::blueprint::{Edge, NodeId};

    // Validate both endpoints exist.
    if state.blueprints.get_node(&payload.source).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Source node not found: {}", payload.source),
                code: Some("SOURCE_NOT_FOUND".into()),
            }),
        ));
    }
    if state.blueprints.get_node(&payload.target).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Target node not found: {}", payload.target),
                code: Some("TARGET_NOT_FOUND".into()),
            }),
        ));
    }

    let edge = Edge {
        source: NodeId::from_raw(&payload.source),
        target: NodeId::from_raw(&payload.target),
        edge_type: payload.edge_type,
        metadata: payload.metadata.clone(),
    };
    state.blueprints.add_edge(edge);
    tracing::info!(
        "Blueprint edge created: {} -[{}]-> {}",
        payload.source,
        payload.edge_type,
        payload.target
    );
    Ok((StatusCode::CREATED, Json(payload)))
}

/// DELETE /blueprint/edges — Remove an edge by source+target+edge_type.
async fn delete_blueprint_edge(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(payload): Json<EdgePayload>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let source = payload.source.clone();
    let target = payload.target.clone();
    let edge_type = payload.edge_type;

    let removed = state.blueprints.remove_edges_where(|e| {
        e.source.0 == source && e.target.0 == target && e.edge_type == edge_type
    });

    if removed > 0 {
        tracing::info!(
            "Blueprint edge(s) deleted: {} -[{}]-> {} ({})",
            source,
            edge_type,
            target,
            removed
        );
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("No matching edge: {} -[{}]-> {}", source, edge_type, target),
                code: Some("EDGE_NOT_FOUND".into()),
            }),
        ))
    }
}

/// GET /blueprint/history — List history snapshots (timestamps).
async fn list_blueprint_history(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
) -> Json<HistoryListResponse> {
    let raw = state.blueprints.list_history();
    let snapshots = raw
        .into_iter()
        .map(|(ts, fname)| SnapshotEntry {
            timestamp: ts,
            filename: fname,
        })
        .collect();
    Json(HistoryListResponse { snapshots })
}

/// GET /blueprint/events — List the event log, optionally filtered by node.
async fn list_blueprint_events(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(query): Query<BlueprintEventsQuery>,
) -> Json<BlueprintEventsResponse> {
    let all_events = match &query.node_id {
        Some(nid) => state.blueprints.events_for_node(nid),
        None => state.blueprints.events(),
    };

    let total = all_events.len();

    // Most recent first, with optional limit.
    let events: Vec<BlueprintEventPayload> = all_events
        .iter()
        .rev()
        .take(query.limit.unwrap_or(usize::MAX))
        .map(|e| {
            // Derive event_type tag from the variant.
            let event_type = match e {
                planner_schemas::artifacts::blueprint::BlueprintEvent::NodeCreated { .. } => {
                    "node_created"
                }
                planner_schemas::artifacts::blueprint::BlueprintEvent::NodeUpdated { .. } => {
                    "node_updated"
                }
                planner_schemas::artifacts::blueprint::BlueprintEvent::NodeDeleted { .. } => {
                    "node_deleted"
                }
                planner_schemas::artifacts::blueprint::BlueprintEvent::EdgeCreated { .. } => {
                    "edge_created"
                }
                planner_schemas::artifacts::blueprint::BlueprintEvent::EdgesDeleted { .. } => {
                    "edges_deleted"
                }
                planner_schemas::artifacts::blueprint::BlueprintEvent::ExportRecorded {
                    ..
                } => "export_recorded",
            };
            let mut data = serde_json::to_value(e).unwrap_or_default();
            normalize_blueprint_event_payload_data(event_type, &mut data);
            BlueprintEventPayload {
                event_type: event_type.to_string(),
                summary: e.summary(),
                timestamp: e.timestamp().to_string(),
                data,
            }
        })
        .collect();

    Json(BlueprintEventsResponse { events, total })
}

fn normalize_blueprint_event_payload_data(event_type: &str, data: &mut serde_json::Value) {
    let Some(object) = data.as_object_mut() else {
        return;
    };

    match event_type {
        "node_created" => {
            if let Some(node) = object.get_mut("node") {
                normalize_blueprint_event_payload_node(node);
            }
        }
        "node_updated" => {
            if let Some(before) = object.get_mut("before") {
                normalize_blueprint_event_payload_node(before);
            }
            if let Some(after) = object.get_mut("after") {
                normalize_blueprint_event_payload_node(after);
            }
        }
        _ => {}
    }
}

fn normalize_blueprint_event_payload_node(node: &mut serde_json::Value) {
    let Ok(mut decoded) = serde_json::from_value::<
        planner_schemas::artifacts::blueprint::BlueprintNode,
    >(node.clone()) else {
        return;
    };

    normalize_blueprint_node_metadata(&mut decoded);
    if let Ok(normalized) = serde_json::to_value(decoded) {
        *node = normalized;
    }
}

fn normalized_export_history_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .map(|value| value.to_ascii_lowercase())
}

fn export_scope_snapshot_filter(
    snapshot: Option<&serde_json::Value>,
    field: &str,
) -> Option<String> {
    snapshot
        .and_then(|value| value.get("filters"))
        .and_then(|value| value.get(field))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .map(|value| value.to_ascii_lowercase())
}

const EXPORT_AUDIT_RETENTION_DAYS: i64 = 90;
const EXPORT_SCOPE_FILTER_ALLOWLIST: &[&str] = &[
    "knowledgetype",
    "scopeclass",
    "scopevisibility",
    "feature",
    "widget",
    "artifact",
    "component",
    "lifecycle",
    "stale",
    "orphan",
    "documentation",
    "updateddate",
];

fn sanitize_export_scope_snapshot(
    snapshot: Option<&serde_json::Value>,
) -> (Option<serde_json::Value>, bool, Vec<String>) {
    let Some(snapshot) = snapshot else {
        return (None, false, Vec::new());
    };

    let Some(snapshot_object) = snapshot.as_object() else {
        return (None, true, vec!["scope_snapshot".into()]);
    };

    let mut sanitized = serde_json::Map::new();
    let mut redacted_fields = Vec::new();

    for (key, value) in snapshot_object {
        match key.as_str() {
            "filters" => {
                let Some(filters) = value.as_object() else {
                    redacted_fields.push("filters".into());
                    continue;
                };
                let mut sanitized_filters = serde_json::Map::new();
                for (filter_key, filter_value) in filters {
                    if EXPORT_SCOPE_FILTER_ALLOWLIST
                        .iter()
                        .any(|allowed| filter_key.eq_ignore_ascii_case(allowed))
                    {
                        sanitized_filters.insert(filter_key.clone(), filter_value.clone());
                    } else if !filter_value.is_null() {
                        redacted_fields.push(format!("filters.{}", filter_key));
                    }
                }
                if !sanitized_filters.is_empty() {
                    sanitized.insert(
                        "filters".into(),
                        serde_json::Value::Object(sanitized_filters),
                    );
                }
            }
            "section" => {
                sanitized.insert("section".into(), value.clone());
            }
            other => {
                if !value.is_null() {
                    redacted_fields.push(other.to_string());
                }
            }
        }
    }

    (
        (!sanitized.is_empty()).then_some(serde_json::Value::Object(sanitized)),
        !redacted_fields.is_empty(),
        redacted_fields,
    )
}

fn export_history_retention_expires_at(timestamp: &str) -> Option<String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
    Some((parsed + chrono::Duration::days(EXPORT_AUDIT_RETENTION_DAYS)).to_rfc3339())
}

fn export_history_matches_query(
    project_id: Option<&str>,
    scope_snapshot: Option<&serde_json::Value>,
    query: &BlueprintExportHistoryQuery,
) -> bool {
    let Some(requested_project_id) = normalized_export_history_filter(query.project_id.as_deref())
    else {
        return [
            ("scopeClass", query.scope_class.as_deref()),
            ("feature", query.feature.as_deref()),
            ("widget", query.widget.as_deref()),
            ("artifact", query.artifact.as_deref()),
            ("component", query.component.as_deref()),
        ]
        .into_iter()
        .all(|(field, value)| {
            let Some(expected) = normalized_export_history_filter(value) else {
                return true;
            };
            export_scope_snapshot_filter(scope_snapshot, field) == Some(expected)
        });
    };

    if normalized_export_history_filter(project_id) != Some(requested_project_id) {
        return false;
    }

    [
        ("scopeClass", query.scope_class.as_deref()),
        ("feature", query.feature.as_deref()),
        ("widget", query.widget.as_deref()),
        ("artifact", query.artifact.as_deref()),
        ("component", query.component.as_deref()),
    ]
    .into_iter()
    .all(|(field, value)| {
        let Some(expected) = normalized_export_history_filter(value) else {
            return true;
        };
        export_scope_snapshot_filter(scope_snapshot, field) == Some(expected)
    })
}

/// GET /blueprint/export-history — List durable export events with project/scope filtering.
async fn list_blueprint_export_history(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(query): Query<BlueprintExportHistoryQuery>,
) -> Json<BlueprintExportHistoryResponse> {
    let filtered: Vec<BlueprintExportHistoryEntry> = state
        .blueprints
        .events()
        .into_iter()
        .filter_map(|event| match event {
            planner_schemas::artifacts::blueprint::BlueprintEvent::ExportRecorded {
                export_id,
                kind,
                actor,
                node_id,
                node_count,
                edge_count,
                project_id,
                project_name,
                scope_snapshot,
                timestamp,
            } => {
                if !export_history_matches_query(
                    project_id.as_deref(),
                    scope_snapshot.as_ref(),
                    &query,
                ) {
                    return None;
                }

                let (scope_snapshot, scope_snapshot_redacted, scope_snapshot_redacted_fields) =
                    sanitize_export_scope_snapshot(scope_snapshot.as_ref());
                let retention_expires_at = export_history_retention_expires_at(&timestamp);

                let summary =
                    planner_schemas::artifacts::blueprint::BlueprintEvent::ExportRecorded {
                        export_id: export_id.clone(),
                        kind: kind.clone(),
                        actor: actor.clone(),
                        node_id: node_id.clone(),
                        node_count,
                        edge_count,
                        project_id: project_id.clone(),
                        project_name: project_name.clone(),
                        scope_snapshot: scope_snapshot.clone(),
                        timestamp: timestamp.clone(),
                    }
                    .summary();

                Some(BlueprintExportHistoryEntry {
                    export_id,
                    kind,
                    actor,
                    node_id,
                    node_count,
                    edge_count,
                    project_id,
                    project_name,
                    scope_snapshot,
                    scope_snapshot_redacted,
                    scope_snapshot_redacted_fields,
                    retention_expires_at,
                    summary,
                    timestamp,
                })
            }
            _ => None,
        })
        .collect();

    let total = filtered.len();
    let entries = filtered
        .into_iter()
        .rev()
        .take(query.limit.unwrap_or(usize::MAX))
        .collect();

    Json(BlueprintExportHistoryResponse { entries, total })
}

/// POST /blueprint/exports — Record a durable export activity event.
async fn record_blueprint_export(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<RecordBlueprintExportRequest>,
) -> Result<(StatusCode, Json<RecordBlueprintExportResponse>), (StatusCode, Json<ErrorResponse>)> {
    if req.node_count == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "node_count must be greater than zero".into(),
                code: Some("INVALID_EXPORT_PAYLOAD".into()),
            }),
        ));
    }

    if matches!(
        req.kind,
        planner_schemas::artifacts::blueprint::BlueprintExportKind::SingleRecord
    ) && req.node_id.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "single_record exports require node_id".into(),
                code: Some("INVALID_EXPORT_PAYLOAD".into()),
            }),
        ));
    }

    let export_id = format!("exp-{}", Uuid::new_v4());
    state.blueprints.record_export_event(
        export_id.clone(),
        req.kind,
        Some(claims.sub),
        req.node_id,
        req.node_count,
        req.edge_count,
        req.project_id,
        req.project_name,
        req.scope_snapshot,
    );

    Ok((
        StatusCode::CREATED,
        Json(RecordBlueprintExportResponse {
            export_id,
            recorded_at: chrono::Utc::now().to_rfc3339(),
        }),
    ))
}

/// POST /blueprint/impact-preview — Analyze downstream impact of a node change.
async fn impact_preview(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<ImpactPreviewRequest>,
) -> Result<
    Json<planner_schemas::artifacts::blueprint::ImpactReport>,
    (StatusCode, Json<ErrorResponse>),
> {
    match state
        .blueprints
        .impact_analysis(&req.node_id, &req.change_description)
    {
        Some(report) => Ok(Json(report)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Blueprint node not found: {}", req.node_id),
                code: Some("NODE_NOT_FOUND".into()),
            }),
        )),
    }
}

// ─── Reconvergence ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ReconvergeRequest {
    source_node_id: String,
    impact_report: planner_schemas::artifacts::blueprint::ImpactReport,
    auto_apply: bool,
}

#[derive(Debug, Serialize)]
struct ReconvergeStepResponse {
    step_id: String,
    node_id: String,
    node_name: String,
    node_type: String,
    action: String,
    severity: String,
    description: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReconvergeSummary {
    total: usize,
    applied: usize,
    skipped: usize,
    errors: usize,
    needs_review: usize,
}

#[derive(Debug, Serialize)]
struct ReconvergeResponse {
    steps: Vec<ReconvergeStepResponse>,
    summary: ReconvergeSummary,
    timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ReconvergeWsMessage {
    #[serde(rename = "step")]
    Step(ReconvergeStepResponse),
    #[serde(rename = "summary")]
    Summary(ReconvergeSummary),
    #[serde(rename = "error")]
    Error { message: String },
}

fn ensure_reconverge_source_exists(
    state: &AppState,
    source_node_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if state.blueprints.get_node(source_node_id).is_none() {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Blueprint node not found: {}", source_node_id),
                code: Some("NODE_NOT_FOUND".into()),
            }),
        ))
    } else {
        Ok(())
    }
}

fn build_reconverge_response(req: &ReconvergeRequest) -> ReconvergeResponse {
    let mut steps = Vec::new();
    let mut applied = 0usize;
    let skipped = 0usize;
    let mut needs_review = 0usize;

    for (i, entry) in req.impact_report.entries.iter().enumerate() {
        let severity_str = match entry.severity {
            planner_schemas::artifacts::blueprint::ImpactSeverity::Shallow => "shallow",
            planner_schemas::artifacts::blueprint::ImpactSeverity::Medium => "medium",
            planner_schemas::artifacts::blueprint::ImpactSeverity::Deep => "deep",
        };
        let action_str = match entry.action {
            planner_schemas::artifacts::blueprint::ImpactAction::Reconverge => "reconverge",
            planner_schemas::artifacts::blueprint::ImpactAction::Update => "update",
            planner_schemas::artifacts::blueprint::ImpactAction::Invalidate => "invalidate",
            planner_schemas::artifacts::blueprint::ImpactAction::Add => "add",
            planner_schemas::artifacts::blueprint::ImpactAction::Remove => "remove",
        };

        let is_deep = matches!(
            entry.severity,
            planner_schemas::artifacts::blueprint::ImpactSeverity::Deep
        );

        let status = if !req.auto_apply || is_deep {
            needs_review += 1;
            "pending"
        } else {
            applied += 1;
            "done"
        };

        steps.push(ReconvergeStepResponse {
            step_id: format!("recon-step-{}", i),
            node_id: entry.node_id.to_string(),
            node_name: entry.node_name.clone(),
            node_type: entry.node_type.clone(),
            action: action_str.to_string(),
            severity: severity_str.to_string(),
            description: entry.explanation.clone(),
            status: status.to_string(),
            error: None,
        });
    }

    let total = steps.len();
    let timestamp = chrono::Utc::now().to_rfc3339();

    ReconvergeResponse {
        steps,
        summary: ReconvergeSummary {
            total,
            applied,
            skipped,
            errors: 0,
            needs_review,
        },
        timestamp,
    }
}

/// POST /blueprint/reconverge — Execute reconvergence based on an impact report.
///
/// Policy: auto_apply=true -> shallow/medium auto-accepted, deep requires review.
async fn reconverge_blueprint(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<ReconvergeRequest>,
) -> Result<Json<ReconvergeResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_reconverge_source_exists(&state, &req.source_node_id)?;
    Ok(Json(build_reconverge_response(&req)))
}

async fn reconverge_ws_handler(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_reconverge_ws(socket, state))
}

async fn send_reconverge_ws_message(
    socket: &mut WebSocket,
    message: ReconvergeWsMessage,
) -> Result<(), ()> {
    let payload = serde_json::to_string(&message).map_err(|_| ())?;
    socket
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}

async fn handle_reconverge_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let Some(Ok(Message::Text(text))) = socket.recv().await else {
        let _ = send_reconverge_ws_message(
            &mut socket,
            ReconvergeWsMessage::Error {
                message: "Expected an initial JSON text message".into(),
            },
        )
        .await;
        return;
    };

    let req: ReconvergeRequest = match serde_json::from_str(&text) {
        Ok(req) => req,
        Err(err) => {
            let _ = send_reconverge_ws_message(
                &mut socket,
                ReconvergeWsMessage::Error {
                    message: format!("Invalid reconvergence request: {}", err),
                },
            )
            .await;
            return;
        }
    };

    if let Err((_, Json(error))) = ensure_reconverge_source_exists(&state, &req.source_node_id) {
        let _ = send_reconverge_ws_message(
            &mut socket,
            ReconvergeWsMessage::Error {
                message: error.error,
            },
        )
        .await;
        return;
    }

    let response = build_reconverge_response(&req);
    for step in response.steps {
        if send_reconverge_ws_message(&mut socket, ReconvergeWsMessage::Step(step))
            .await
            .is_err()
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let _ = send_reconverge_ws_message(&mut socket, ReconvergeWsMessage::Summary(response.summary))
        .await;
}

fn parse_proposal_status(
    status: Option<&str>,
) -> Result<Option<planner_core::discovery::ProposalStatus>, String> {
    match status {
        None => Ok(None),
        Some("pending") => Ok(Some(planner_core::discovery::ProposalStatus::Pending)),
        Some("accepted") => Ok(Some(planner_core::discovery::ProposalStatus::Accepted)),
        Some("rejected") => Ok(Some(planner_core::discovery::ProposalStatus::Rejected)),
        Some("merged") => Ok(Some(planner_core::discovery::ProposalStatus::Merged)),
        Some(other) => Err(format!("Unknown proposal status '{}'", other)),
    }
}

async fn run_discovery_scan(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<DiscoveryScanRequest>,
) -> Result<Json<DiscoveryRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cgc_scan_enabled = planner_core::discovery::code_graph_context_available();
    let requested = if req.scanners.iter().any(|scanner| scanner == "all") {
        let mut scanners = vec!["cargo_toml".to_string(), "directory_structure".to_string()];
        if cgc_scan_enabled {
            scanners.push("code_graph_context".to_string());
        }
        scanners
    } else {
        req.scanners.clone()
    };

    if requested.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "At least one discovery scanner must be requested".into(),
                code: Some("NO_SCANNERS_REQUESTED".into()),
            }),
        ));
    }

    let project_root = req
        .root_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    if !project_root.exists() || !project_root.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "Discovery scan root does not exist or is not a directory: {}",
                    project_root.display()
                ),
                code: Some("INVALID_SCAN_ROOT".into()),
            }),
        ));
    }

    let mut results = Vec::new();

    for scanner in requested {
        let started = std::time::Instant::now();
        let mut proposed_count = 0usize;
        let mut skipped_count = 0usize;
        let mut proposed_edge_count = 0usize;
        let mut skipped_edge_count = 0usize;
        let mut errors = Vec::new();

        match scanner.as_str() {
            "cargo_toml" => {
                let scan_output =
                    planner_core::discovery::scan_cargo_toml(&project_root, &state.blueprints);
                let (inserted, deduped) = state
                    .proposals
                    .insert_many(scan_output.proposals)
                    .map_err(|err| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: format!("Failed to persist discovery proposals: {}", err),
                                code: Some("PROPOSAL_PERSIST_FAILED".into()),
                            }),
                        )
                    })?;
                proposed_count = inserted;
                skipped_count = scan_output.skipped_count + deduped;
                errors = scan_output.errors;
            }
            "directory_structure" => {
                let scan_output = planner_core::discovery::scan_directory_structure(
                    &project_root,
                    &state.blueprints,
                );
                let (inserted, deduped) = state
                    .proposals
                    .insert_many(scan_output.proposals)
                    .map_err(|err| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: format!("Failed to persist discovery proposals: {}", err),
                                code: Some("PROPOSAL_PERSIST_FAILED".into()),
                            }),
                        )
                    })?;
                proposed_count = inserted;
                skipped_count = scan_output.skipped_count + deduped;
                errors = scan_output.errors;
            }
            "code_graph_context" => {
                if !cgc_scan_enabled {
                    errors.push("CodeGraphContext is not available".into());
                } else {
                    match planner_core::discovery::collect_code_graph_edge_proposals(
                        &project_root,
                        &state.blueprints,
                    ) {
                        Ok(imports) => {
                            let import_result = planner_core::discovery::import_edge_proposals(
                                &state.proposals,
                                &state.blueprints,
                                imports,
                            )
                            .map_err(|err| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(ErrorResponse {
                                        error: format!(
                                            "Failed to import discovery edge proposals: {}",
                                            err
                                        ),
                                        code: Some("EDGE_PROPOSAL_IMPORT_FAILED".into()),
                                    }),
                                )
                            })?;
                            proposed_edge_count = import_result.inserted;
                            skipped_edge_count = import_result.skipped;
                            errors.extend(import_result.errors);
                        }
                        Err(err) => errors.push(err),
                    }
                }
            }
            other => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Unknown discovery scanner '{}'", other),
                        code: Some("UNKNOWN_SCANNER".into()),
                    }),
                ));
            }
        }

        results.push(DiscoveryScanResult {
            scanner,
            proposed_count,
            skipped_count,
            proposed_edge_count,
            skipped_edge_count,
            errors,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    let total_proposed = results.iter().map(|result| result.proposed_count).sum();
    let total_edge_proposed = results
        .iter()
        .map(|result| result.proposed_edge_count)
        .sum();
    Ok(Json(DiscoveryRunResponse {
        results,
        total_proposed,
        total_edge_proposed,
    }))
}

async fn list_proposals(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(query): Query<ProposedNodesQuery>,
) -> Result<Json<ProposedNodesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let status = parse_proposal_status(query.status.as_deref()).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err,
                code: Some("INVALID_PROPOSAL_STATUS".into()),
            }),
        )
    })?;

    let proposals = state.proposals.list(status);
    let total = proposals.len();
    Ok(Json(ProposedNodesResponse { proposals, total }))
}

async fn list_edge_proposals(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(query): Query<ProposedNodesQuery>,
) -> Result<Json<ProposedEdgesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let status = parse_proposal_status(query.status.as_deref()).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err,
                code: Some("INVALID_PROPOSAL_STATUS".into()),
            }),
        )
    })?;

    let proposals = state.proposals.list_edge_proposals(status);
    let total = proposals.len();
    Ok(Json(ProposedEdgesResponse { proposals, total }))
}

async fn import_edge_proposals_endpoint(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Json(req): Json<ImportEdgeProposalsRequest>,
) -> Result<Json<planner_core::discovery::EdgeImportResult>, (StatusCode, Json<ErrorResponse>)> {
    let result = planner_core::discovery::import_edge_proposals(
        &state.proposals,
        &state.blueprints,
        req.proposals,
    )
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to import edge proposals: {}", err),
                code: Some("EDGE_PROPOSAL_IMPORT_FAILED".into()),
            }),
        )
    })?;

    Ok(Json(result))
}

async fn accept_proposal(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(proposal_id): Path<String>,
    req: Option<Json<AcceptProposalRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let request = req.map(|Json(value)| value).unwrap_or_default();
    let Some(proposal) = state.proposals.mark_accepted(&proposal_id).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update proposal state: {}", err),
                code: Some("PROPOSAL_UPDATE_FAILED".into()),
            }),
        )
    })?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Discovery proposal not found: {}", proposal_id),
                code: Some("PROPOSAL_NOT_FOUND".into()),
            }),
        ));
    };

    if proposal.status == planner_core::discovery::ProposalStatus::Merged {
        return Ok(Json(serde_json::json!({
            "node_id": proposal.node.id().0,
            "message": "Proposal was already merged"
        })));
    }

    let mut final_node = proposal.node.clone();

    if let Some(node_patch) = request.node_patch {
        let mut merged = serde_json::to_value(&proposal.node).map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to serialize proposal node: {}", err),
                    code: Some("SERIALIZE_FAILED".into()),
                }),
            )
        })?;

        apply_json_merge_patch(&mut merged, node_patch.clone());

        final_node = serde_json::from_value(merged).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid proposal node_patch payload: {}", err),
                    code: Some("INVALID_NODE_PATCH".into()),
                }),
            )
        })?;

        if final_node.id().0 != proposal.node.id().0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "node_patch cannot change node id".into(),
                    code: Some("NODE_ID_MISMATCH".into()),
                }),
            ));
        }

        if final_node.type_name() != proposal.node.type_name() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "node_patch cannot change node_type".into(),
                    code: Some("NODE_TYPE_MISMATCH".into()),
                }),
            ));
        }

        normalize_component_naming(Some(&proposal.node), &node_patch, &mut final_node);
    } else {
        normalize_component_naming(
            Some(&proposal.node),
            &serde_json::Value::Object(serde_json::Map::new()),
            &mut final_node,
        );
    }

    normalize_blueprint_node_metadata(&mut final_node);
    validate_blueprint_node_scope(&final_node).map_err(|message| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message,
                code: Some("INVALID_SCOPE".into()),
            }),
        )
    })?;

    state.blueprints.upsert_node(final_node.clone());
    let _ = state.proposals.mark_merged(&proposal_id);

    Ok(Json(serde_json::json!({
        "node_id": final_node.id().0,
        "message": "Proposal accepted and merged into blueprint"
    })))
}

async fn reject_proposal(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(proposal_id): Path<String>,
    Json(req): Json<RejectProposalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let Some(proposal) = state
        .proposals
        .mark_rejected(&proposal_id, req.reason)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update proposal state: {}", err),
                    code: Some("PROPOSAL_UPDATE_FAILED".into()),
                }),
            )
        })?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Discovery proposal not found: {}", proposal_id),
                code: Some("PROPOSAL_NOT_FOUND".into()),
            }),
        ));
    };

    Ok(Json(serde_json::json!({
        "proposal_id": proposal.id,
        "message": "Proposal rejected"
    })))
}

async fn accept_edge_proposal(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(proposal_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let Some(proposal) = state
        .proposals
        .mark_edge_accepted(&proposal_id)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update edge proposal state: {}", err),
                    code: Some("PROPOSAL_UPDATE_FAILED".into()),
                }),
            )
        })?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Edge proposal not found: {}", proposal_id),
                code: Some("PROPOSAL_NOT_FOUND".into()),
            }),
        ));
    };

    if proposal.status == planner_core::discovery::ProposalStatus::Merged {
        return Ok(Json(serde_json::json!({
            "edge": proposal.edge,
            "message": "Edge proposal was already merged"
        })));
    }

    if state
        .blueprints
        .get_node(proposal.edge.source.as_str())
        .is_none()
        || state
            .blueprints
            .get_node(proposal.edge.target.as_str())
            .is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Edge proposal endpoints no longer exist".into(),
                code: Some("EDGE_ENDPOINT_NOT_FOUND".into()),
            }),
        ));
    }

    state.blueprints.add_edge(proposal.edge.clone());
    let _ = state.proposals.mark_edge_merged(&proposal_id);

    Ok(Json(serde_json::json!({
        "edge": proposal.edge,
        "message": "Edge proposal accepted and merged into blueprint"
    })))
}

async fn reject_edge_proposal(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(proposal_id): Path<String>,
    req: Option<Json<RejectProposalRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let request = req
        .map(|Json(value)| value)
        .unwrap_or(RejectProposalRequest { reason: None });
    let Some(proposal) = state
        .proposals
        .mark_edge_rejected(&proposal_id, request.reason)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update edge proposal state: {}", err),
                    code: Some("PROPOSAL_UPDATE_FAILED".into()),
                }),
            )
        })?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Edge proposal not found: {}", proposal_id),
                code: Some("PROPOSAL_NOT_FOUND".into()),
            }),
        ));
    };

    Ok(Json(serde_json::json!({
        "proposal_id": proposal.id,
        "message": "Edge proposal rejected"
    })))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthConfig;
    use crate::session::SessionStore;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use std::path::{Path, PathBuf};
    use std::process::Command as StdCommand;
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    struct ImmediateSuccessImportAcquirer;

    #[async_trait]
    impl crate::import::ImportAcquirer for ImmediateSuccessImportAcquirer {
        async fn acquire_github(
            &self,
            _canonical_ref: &str,
            checkout_path: &Path,
        ) -> Result<crate::import::AcquiredImportSource, String> {
            std::fs::create_dir_all(checkout_path.join("planner-server/src"))
                .map_err(|err| err.to_string())?;
            std::fs::write(
                checkout_path.join("README.md"),
                "# Task Tracker\nTrack work across teams.\n",
            )
            .map_err(|err| err.to_string())?;
            std::fs::write(
                checkout_path.join("Cargo.toml"),
                "[workspace]\nmembers = [\"planner-server\"]\n",
            )
            .map_err(|err| err.to_string())?;
            std::fs::write(
                checkout_path.join("planner-server/Cargo.toml"),
                "[package]\nname = \"planner-server\"\nversion = \"0.1.0\"\n[dependencies]\naxum = \"0.7\"\nserde = \"1\"\n",
            )
            .map_err(|err| err.to_string())?;
            std::fs::write(
                checkout_path.join("planner-server/src/lib.rs"),
                "pub fn ready() {}\n",
            )
            .map_err(|err| err.to_string())?;
            Ok(crate::import::AcquiredImportSource {
                default_branch: "main".into(),
                head_revision: "deadbeef".into(),
            })
        }
    }

    struct ImmediateFailureImportAcquirer;

    #[async_trait]
    impl crate::import::ImportAcquirer for ImmediateFailureImportAcquirer {
        async fn acquire_github(
            &self,
            _canonical_ref: &str,
            _checkout_path: &Path,
        ) -> Result<crate::import::AcquiredImportSource, String> {
            Err("simulated clone failure".into())
        }
    }

    struct ImmediateFailureImportAnalyzer;

    #[async_trait]
    impl crate::import::ImportAnalyzer for ImmediateFailureImportAnalyzer {
        async fn analyze(
            &self,
            _request: crate::import::ImportAnalysisRequest,
        ) -> Result<crate::import::AnalyzedImportDraft, String> {
            Err("simulated analysis failure".into())
        }
    }

    fn test_state() -> Arc<AppState> {
        test_state_with_import_workers(
            Arc::new(ImmediateSuccessImportAcquirer),
            crate::import::default_import_analyzer(),
        )
    }

    fn test_state_with_import_acquirer(
        import_acquirer: Arc<dyn crate::import::ImportAcquirer>,
    ) -> Arc<AppState> {
        test_state_with_import_workers(import_acquirer, crate::import::default_import_analyzer())
    }

    fn test_state_with_import_workers(
        import_acquirer: Arc<dyn crate::import::ImportAcquirer>,
        import_analyzer: Arc<dyn crate::import::ImportAnalyzer>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer,
            import_analyzer,
            auth_config: None, // dev mode for tests
            event_store: None,
            cxdb: None, // no durable storage in unit tests
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        })
    }

    fn test_state_with_event_store(data_dir: &std::path::Path) -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: Arc::new(ImmediateSuccessImportAcquirer),
            import_analyzer: crate::import::default_import_analyzer(),
            auth_config: None,
            event_store: Some(planner_core::observability::EventStore::new(data_dir).unwrap()),
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        })
    }

    fn test_state_with_persistent_blueprints(data_dir: &std::path::Path) -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::open(data_dir).unwrap(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: Arc::new(ImmediateSuccessImportAcquirer),
            import_analyzer: crate::import::default_import_analyzer(),
            auth_config: None,
            event_store: None,
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        })
    }

    #[tokio::test]
    async fn test_health() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert!(health.status == "ok" || health.status == "degraded");
        assert_eq!(health.sessions_active, 0);
    }

    #[tokio::test]
    async fn test_health_no_auth_required() {
        // Health endpoint must work with no token even when auth is configured
        let state = Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: Arc::new(ImmediateSuccessImportAcquirer),
            import_analyzer: crate::import::default_import_analyzer(),
            auth_config: Some(AuthConfig {
                domain: "test.auth0.com".into(),
                audience: "test".into(),
                decoding_key: None,
            }),
            event_store: None,
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        });
        let app = routes(state);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_project_import_creates_project_job_and_binding() {
        let state = test_state();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example/task-tracker.git"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.project.slug, "task-tracker");
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::Queued
        );
        assert_eq!(payload.source_binding.provider, ImportProvider::GitHub);
        assert_eq!(
            payload.source_binding.canonical_ref,
            "https://github.com/example/task-tracker"
        );
        assert!(state.imports.get_job(payload.import_job.id).is_some());
    }

    #[tokio::test]
    async fn test_create_project_import_rejects_non_absolute_local_path() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "local",
                    "source_ref": "relative/path"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_project_import_rejects_incomplete_github_repo_url() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_project_import_returns_owner_scoped_job() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Imported Repo", None, None, Vec::new(), None);
        let (job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::Local,
                "/tmp/repo".into(),
                "/tmp/repo".into(),
                false,
            )
            .unwrap();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/projects/imports/{}", job.id))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.import_job.id, job.id);
        assert_eq!(payload.source_binding.canonical_ref, "/tmp/repo");
    }

    #[tokio::test]
    async fn test_create_project_import_returns_conflict_for_existing_visible_source() {
        let state = test_state();
        let existing =
            state
                .projects
                .create("dev|local", "Existing Import", None, None, Vec::new(), None);
        let (_job, binding) = state
            .imports
            .create(
                existing.id,
                ImportProvider::GitHub,
                "https://github.com/example/task-tracker".into(),
                "https://github.com/example/task-tracker".into(),
                true,
            )
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example/task-tracker.git"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportConflictResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.project.id, existing.id);
        assert_eq!(payload.project.slug, existing.slug);
        assert_eq!(payload.source_binding.canonical_ref, binding.canonical_ref);
        assert_eq!(state.projects.count(), 1);
        assert_eq!(state.imports.count_jobs(), 1);
    }

    #[tokio::test]
    async fn test_get_project_import_state_returns_latest_job_for_project() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Import State", None, None, Vec::new(), None);
        let (first_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-state".into(),
                "https://github.com/example/import-state".into(),
                true,
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let (latest_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/projects/{}/import-state", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.project.id, project.id);
        assert_eq!(payload.import_job.id, latest_job.id);
        assert_ne!(payload.import_job.id, first_job.id);
    }

    #[tokio::test]
    async fn test_get_project_import_history_returns_descending_entries_and_diff_summary() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Import History", None, None, Vec::new(), None);
        let seeded_applied = state.sessions.create("dev|local");
        let (applied_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-history".into(),
                "https://github.com/example/import-history".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: applied_job.id,
                project_id: project.id,
                analysis_summary: "Earlier applied import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-history".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let applied_job = state
            .imports
            .mark_job_review_pending(
                applied_job.id,
                "Applied draft ready.",
                "Earlier applied import.".into(),
                seeded_applied.id,
            )
            .unwrap();
        let _ = state
            .imports
            .mark_job_applied(
                applied_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let seeded_pending = state.sessions.create("dev|local");
        let (pending_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending import with Rust added.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-history".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let _ = state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Pending import with Rust added.".into(),
                seeded_pending.id,
            )
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri(format!("/projects/{}/import-history", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.project.id, project.id);
        assert_eq!(payload.history.len(), 2);
        assert_eq!(payload.history[0].import_job.id, pending_job.id);
        assert_eq!(payload.history[1].import_job.id, applied_job.id);
        assert_eq!(payload.history[0].discovered_node_count, Some(2));
        assert_eq!(payload.history[1].discovered_node_count, Some(1));

        let diff_summary = payload.diff_summary.expect("diff summary should exist");
        assert_eq!(diff_summary.current_job_id, pending_job.id.to_string());
        assert_eq!(diff_summary.compared_to_job_id, applied_job.id.to_string());
        assert_eq!(diff_summary.added_nodes.len(), 1);
        assert_eq!(diff_summary.added_nodes[0].node_name, "Rust");
        assert_eq!(diff_summary.added_node_types[0].node_type, "technology");
        assert_eq!(diff_summary.added_node_types[0].count, 1);
        assert!(diff_summary.removed_nodes.is_empty());
        assert_eq!(
            diff_summary.current_head_revision.as_deref(),
            Some("deadbeef")
        );
        assert_eq!(
            diff_summary.compared_head_revision.as_deref(),
            Some("cafebabe")
        );
    }

    #[tokio::test]
    async fn test_get_project_import_history_includes_selection_summary_counts() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import History Selection Summary",
            None,
            None,
            Vec::new(),
            None,
        );
        let seeded_pending = state.sessions.create("dev|local");
        let (pending_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-history-selection-summary".into(),
                "https://github.com/example/import-history-selection-summary".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending import with exclusions.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-history-selection-summary"
                        .into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let pending_job = state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Pending import with exclusions.".into(),
                seeded_pending.id,
            )
            .unwrap();
        let _ = state
            .imports
            .set_review_node_included(pending_job.id, project.id, "tech-rust-b2c3d4e5", false)
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri(format!("/projects/{}/import-history", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.history.len(), 1);
        assert_eq!(payload.history[0].discovered_node_count, Some(2));
        assert_eq!(payload.history[0].effective_included_node_count, Some(1));
        assert_eq!(payload.history[0].effective_excluded_node_count, Some(1));
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entry_returns_selected_vs_current_diff_without_mutation(
    ) {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Import Compare", None, None, Vec::new(), None);
        let seeded_applied = state.sessions.create("dev|local");
        let (applied_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-compare".into(),
                "https://github.com/example/import-compare".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: applied_job.id,
                project_id: project.id,
                analysis_summary: "Earlier applied import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-compare".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let applied_job = state
            .imports
            .mark_job_review_pending(
                applied_job.id,
                "Applied draft ready.",
                "Earlier applied import.".into(),
                seeded_applied.id,
            )
            .unwrap();
        let _ = state
            .imports
            .mark_job_applied(
                applied_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let seeded_pending = state.sessions.create("dev|local");
        let (pending_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending import with Rust added.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-compare".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let _ = state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Pending import with Rust added.".into(),
                seeded_pending.id,
            )
            .unwrap();

        let counts_before = state.blueprints.counts();
        let app = routes(state.clone());
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryComparisonResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.selected_entry.import_job.id, applied_job.id);
        assert_eq!(payload.current_import_job.id, pending_job.id);
        assert!(!payload.selected_entry_uses_selection_filter);
        assert!(!payload.current_import_job_uses_selection_filter);
        assert_eq!(
            payload.diff_summary.current_job_id,
            pending_job.id.to_string()
        );
        assert_eq!(
            payload.diff_summary.compared_to_job_id,
            applied_job.id.to_string()
        );
        assert_eq!(payload.diff_summary.added_nodes.len(), 1);
        assert_eq!(payload.diff_summary.added_nodes[0].node_name, "Rust");
        assert!(payload.diff_summary.removed_nodes.is_empty());
        assert_eq!(state.blueprints.counts(), counts_before);
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entry_rejects_missing_historical_draft() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import Compare Missing",
            None,
            None,
            Vec::new(),
            None,
        );
        let seeded_pending = state.sessions.create("dev|local");
        let (pending_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-compare-missing".into(),
                "https://github.com/example/import-compare-missing".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Current pending import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-compare-missing".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let _ = state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Current pending import.".into(),
                seeded_pending.id,
            )
            .unwrap();

        let (historical_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let historical_job = state
            .imports
            .mark_job_applied(
                historical_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare",
                project.slug, historical_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entries_returns_pair_diff_without_mutation() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import Pair Compare",
            None,
            None,
            Vec::new(),
            None,
        );
        let seeded_first = state.sessions.create("dev|local");
        let (first_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-pair-compare".into(),
                "https://github.com/example/import-pair-compare".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: first_job.id,
                project_id: project.id,
                analysis_summary: "Current reviewable import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-pair-compare".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let first_job = state
            .imports
            .mark_job_review_pending(
                first_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Current reviewable import.".into(),
                seeded_first.id,
            )
            .unwrap();

        let seeded_second = state.sessions.create("dev|local");
        let (second_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: second_job.id,
                project_id: project.id,
                analysis_summary: "Older applied import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-pair-compare".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let second_job = state
            .imports
            .mark_job_review_pending(
                second_job.id,
                "Import draft ready.",
                "Older applied import.".into(),
                seeded_second.id,
            )
            .unwrap();
        let second_job = state
            .imports
            .mark_job_applied(
                second_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let counts_before = state.blueprints.counts();
        let app = routes(state.clone());
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare/{}",
                project.slug, first_job.id, second_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryPairComparisonResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.baseline_entry.import_job.id, first_job.id);
        assert_eq!(payload.compared_entry.import_job.id, second_job.id);
        assert!(!payload.baseline_entry_uses_selection_filter);
        assert!(!payload.compared_entry_uses_selection_filter);
        assert_eq!(
            payload.diff_summary.current_job_id,
            second_job.id.to_string()
        );
        assert_eq!(
            payload.diff_summary.compared_to_job_id,
            first_job.id.to_string()
        );
        assert!(payload.diff_summary.added_nodes.is_empty());
        assert_eq!(payload.diff_summary.removed_nodes.len(), 1);
        assert_eq!(payload.diff_summary.removed_nodes[0].node_name, "Rust");
        assert_eq!(state.blueprints.counts(), counts_before);
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entries_rejects_missing_baseline_draft() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import Pair Compare Missing",
            None,
            None,
            Vec::new(),
            None,
        );
        let seeded_compared = state.sessions.create("dev|local");
        let (compared_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-pair-compare-missing".into(),
                "https://github.com/example/import-pair-compare-missing".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: compared_job.id,
                project_id: project.id,
                analysis_summary: "Compared historical import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-pair-compare-missing".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let compared_job = state
            .imports
            .mark_job_review_pending(
                compared_job.id,
                "Compared historical import ready.",
                "Compared historical import.".into(),
                seeded_compared.id,
            )
            .unwrap();
        let compared_job = state
            .imports
            .mark_job_applied(
                compared_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let (baseline_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let baseline_job = state
            .imports
            .mark_job_applied(
                baseline_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare/{}",
                project.slug, baseline_job.id, compared_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entry_uses_selection_filtering_for_current_review_job(
    ) {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import Compare Selection Aware",
            None,
            None,
            Vec::new(),
            None,
        );

        let seeded_applied = state.sessions.create("dev|local");
        let (applied_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-compare-selection".into(),
                "https://github.com/example/import-compare-selection".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: applied_job.id,
                project_id: project.id,
                analysis_summary: "Applied import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-compare-selection".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let applied_job = state
            .imports
            .mark_job_review_pending(
                applied_job.id,
                "Applied draft ready.",
                "Applied import.".into(),
                seeded_applied.id,
            )
            .unwrap();
        let applied_job = state
            .imports
            .mark_job_applied(
                applied_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let seeded_pending = state.sessions.create("dev|local");
        let (pending_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending import with merge controls.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-compare-selection".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let pending_job = state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Pending import with merge controls.".into(),
                seeded_pending.id,
            )
            .unwrap();
        let _ = state
            .imports
            .set_review_node_included(pending_job.id, project.id, "tech-rust-b2c3d4e5", false)
            .unwrap();

        let counts_before = state.blueprints.counts();
        let app = routes(state.clone());
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryComparisonResponse =
            serde_json::from_slice(&body).unwrap();
        assert!(payload.current_import_job_uses_selection_filter);
        assert!(!payload.selected_entry_uses_selection_filter);
        assert!(payload.diff_summary.added_nodes.is_empty());
        assert!(payload.diff_summary.removed_nodes.is_empty());
        assert_eq!(state.blueprints.counts(), counts_before);
    }

    #[tokio::test]
    async fn test_compare_project_import_history_entries_uses_selection_filtering_on_baseline_entry(
    ) {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Import Pair Compare Selection Aware",
            None,
            None,
            Vec::new(),
            None,
        );

        let seeded_baseline = state.sessions.create("dev|local");
        let (baseline_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/import-pair-selection".into(),
                "https://github.com/example/import-pair-selection".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: baseline_job.id,
                project_id: project.id,
                analysis_summary: "Baseline reviewable import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-pair-selection".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let baseline_job = state
            .imports
            .mark_job_review_pending(
                baseline_job.id,
                "Baseline draft ready.",
                "Baseline reviewable import.".into(),
                seeded_baseline.id,
            )
            .unwrap();
        let _ = state
            .imports
            .set_review_node_included(baseline_job.id, project.id, "tech-rust-b2c3d4e5", false)
            .unwrap();

        let seeded_compared = state.sessions.create("dev|local");
        let (compared_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: compared_job.id,
                project_id: project.id,
                analysis_summary: "Compared applied import.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/import-pair-selection".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let compared_job = state
            .imports
            .mark_job_review_pending(
                compared_job.id,
                "Compared draft ready.",
                "Compared applied import.".into(),
                seeded_compared.id,
            )
            .unwrap();
        let compared_job = state
            .imports
            .mark_job_applied(
                compared_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let counts_before = state.blueprints.counts();
        let app = routes(state.clone());
        let req = Request::builder()
            .uri(format!(
                "/projects/{}/import-history/{}/compare/{}",
                project.slug, baseline_job.id, compared_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportHistoryPairComparisonResponse =
            serde_json::from_slice(&body).unwrap();
        assert!(payload.baseline_entry_uses_selection_filter);
        assert!(!payload.compared_entry_uses_selection_filter);
        assert!(payload.diff_summary.added_nodes.is_empty());
        assert!(payload.diff_summary.removed_nodes.is_empty());
        assert_eq!(state.blueprints.counts(), counts_before);
    }

    #[tokio::test]
    async fn test_github_import_eventually_reaches_review_pending_with_seeded_session_and_draft() {
        let state = test_state_with_import_acquirer(Arc::new(ImmediateSuccessImportAcquirer));
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example/ready-repo"
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::ReviewPending,
        )
        .await;
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::ReviewPending
        );
        assert_eq!(
            payload.source_binding.default_branch.as_deref(),
            Some("main")
        );
        assert_eq!(
            payload.source_binding.head_revision.as_deref(),
            Some("deadbeef")
        );
        assert!(payload.source_binding.local_root.is_some());
        assert!(payload.import_job.seed_session_id.is_some());
        assert!(payload
            .import_job
            .analysis_summary
            .as_deref()
            .unwrap_or_default()
            .contains("Imported draft for"));
        let draft = payload
            .import_draft
            .expect("import draft should be present");
        assert_eq!(draft.project_id, payload.project.id);
        assert_eq!(draft.job_id, payload.import_job.id);
        assert!(!draft.discovered_nodes.is_empty());
        assert!(draft.discovered_nodes.iter().all(|node| {
            matches!(
                node.scope().project.as_ref().map(|scope| scope.project_id.as_str()),
                Some(project_id) if project_id == payload.project.id.to_string()
            ) && matches!(
                node.scope().scope_class,
                planner_schemas::artifacts::blueprint::ScopeClass::Project
            )
        }));
        assert!(state.proposals.list(None).is_empty());
        let (blueprint_nodes, blueprint_edges) = state.blueprints.counts();
        assert_eq!((blueprint_nodes, blueprint_edges), (0, 0));

        let seeded_session_id = payload
            .import_job
            .seed_session_id
            .expect("seeded session should exist");
        let seeded_session = state
            .sessions
            .get(seeded_session_id)
            .expect("seeded session should be persisted");
        assert_eq!(seeded_session.project_id, Some(payload.project.id));
        assert_eq!(
            seeded_session.project_slug.as_deref(),
            Some(payload.project.slug.as_str())
        );
        assert!(seeded_session
            .project_description
            .as_deref()
            .unwrap_or_default()
            .starts_with("Imported planning brief for"));
    }

    #[tokio::test]
    async fn test_github_import_failure_is_durable_and_truthful() {
        let state = test_state_with_import_acquirer(Arc::new(ImmediateFailureImportAcquirer));
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example/failing-repo"
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::Failed,
        )
        .await;
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::Failed
        );
        assert_eq!(
            payload.import_job.error_message.as_deref(),
            Some("simulated clone failure")
        );
        assert!(payload.source_binding.local_root.is_none());
        assert!(payload.import_draft.is_none());
    }

    #[tokio::test]
    async fn test_github_import_analysis_failure_is_durable_without_review_handoff() {
        let state = test_state_with_import_workers(
            Arc::new(ImmediateSuccessImportAcquirer),
            Arc::new(ImmediateFailureImportAnalyzer),
        );
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "github",
                    "source_ref": "https://github.com/example/fails-during-analysis"
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::Failed,
        )
        .await;
        assert_eq!(
            payload.import_job.error_message.as_deref(),
            Some("simulated analysis failure")
        );
        assert!(payload.source_binding.local_root.is_some());
        assert!(payload.import_job.seed_session_id.is_none());
        assert!(payload.import_draft.is_none());
    }

    #[tokio::test]
    async fn test_local_import_eventually_reaches_review_pending_with_seeded_session_and_draft() {
        let local_root = create_temp_local_git_repo("success");
        let state = test_state();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "local",
                    "source_ref": local_root.to_string_lossy(),
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::ReviewPending,
        )
        .await;
        assert_eq!(payload.import_job.provider, ImportProvider::Local);
        assert_eq!(
            payload.source_binding.local_root.as_deref(),
            Some(local_root.to_string_lossy().as_ref())
        );
        assert!(!payload.source_binding.managed_checkout);
        assert_eq!(
            payload.source_binding.default_branch.as_deref(),
            Some("main")
        );
        assert!(payload
            .source_binding
            .head_revision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        assert!(payload.import_job.seed_session_id.is_some());
        assert!(payload.import_draft.is_some());
        assert!(state.proposals.list(None).is_empty());
    }

    #[tokio::test]
    async fn test_local_import_missing_directory_fails_without_review_state() {
        let missing_root =
            std::env::temp_dir().join(format!("planner-missing-local-import-{}", Uuid::new_v4()));
        let state = test_state();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects/imports")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "provider": "local",
                    "source_ref": missing_root.to_string_lossy(),
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::Failed,
        )
        .await;
        assert_eq!(payload.import_job.provider, ImportProvider::Local);
        assert!(payload
            .import_job
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("local import root is unavailable"));
        assert!(payload.source_binding.local_root.is_none());
        assert!(payload.import_job.seed_session_id.is_none());
        assert!(payload.import_draft.is_none());
        assert!(state.proposals.list(None).is_empty());
    }

    #[tokio::test]
    async fn test_reimport_project_for_github_binding_reaches_review_pending() {
        let state = test_state_with_import_acquirer(Arc::new(ImmediateSuccessImportAcquirer));
        let project =
            state
                .projects
                .create("dev|local", "Reimport GitHub", None, None, Vec::new(), None);
        let (initial_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/reimport-github".into(),
                "https://github.com/example/reimport-github".into(),
                true,
            )
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/reimport", project.slug))
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_ne!(created.import_job.id, initial_job.id);
        assert_eq!(created.project.id, project.id);

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::ReviewPending,
        )
        .await;
        assert_eq!(payload.import_job.project_id, project.id);
        assert!(payload.import_draft.is_some());
        assert!(payload.import_job.seed_session_id.is_some());
    }

    #[tokio::test]
    async fn test_reimport_project_for_local_binding_reaches_review_pending() {
        let local_root = create_temp_local_git_repo("reimport-local");
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Reimport Local", None, None, Vec::new(), None);
        let (initial_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::Local,
                local_root.to_string_lossy().to_string(),
                local_root.to_string_lossy().to_string(),
                false,
            )
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/reimport", project.slug))
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_ne!(created.import_job.id, initial_job.id);
        assert_eq!(created.import_job.provider, ImportProvider::Local);

        let payload = wait_for_import_status(
            &app,
            created.import_job.id,
            crate::import::ImportStatus::ReviewPending,
        )
        .await;
        assert_eq!(
            payload.source_binding.local_root.as_deref(),
            Some(local_root.to_string_lossy().as_ref())
        );
        assert!(payload.import_draft.is_some());
    }

    #[tokio::test]
    async fn test_failed_jobs_appear_in_history_without_breaking_review_lookup() {
        let state = test_state();
        let (project, applied_job, _draft) = seed_review_pending_import(&state, "Failed History");
        let _ = state
            .imports
            .mark_job_applied(
                applied_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();
        let (failed_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let _ = state
            .imports
            .mark_job_failed(failed_job.id, "simulated re-import failure")
            .unwrap();
        let app = routes(state.clone());

        let history_req = Request::builder()
            .uri(format!("/projects/{}/import-history", project.slug))
            .body(Body::empty())
            .unwrap();
        let history_resp = app.clone().oneshot(history_req).await.unwrap();
        assert_eq!(history_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(history_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let history: ProjectImportHistoryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(history.history.len(), 2);
        assert_eq!(history.history[0].import_job.id, failed_job.id);
        assert_eq!(history.history[0].import_job.status, ImportStatus::Failed);
        assert_eq!(history.history[1].import_job.id, applied_job.id);
        assert!(history.diff_summary.is_none());

        let review_req = Request::builder()
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let review_resp = app.oneshot(review_req).await.unwrap();
        assert_eq!(review_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(review_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let review: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(review.import_job.id, applied_job.id);
        assert_eq!(review.import_job.status, ImportStatus::Applied);
    }

    async fn wait_for_import_status(
        app: &Router,
        job_id: Uuid,
        expected_status: crate::import::ImportStatus,
    ) -> ProjectImportResponse {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let req = Request::builder()
                .uri(format!("/projects/imports/{}", job_id))
                .body(Body::empty())
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
            if payload.import_job.status == expected_status {
                return payload;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for import job {} to reach {:?}; latest status was {:?}",
                job_id,
                expected_status,
                payload.import_job.status
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    fn create_temp_local_git_repo(prefix: &str) -> PathBuf {
        let repo_root = std::env::temp_dir().join(format!(
            "planner-local-import-{}-{}",
            prefix,
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(repo_root.join("planner-server/src")).unwrap();
        std::fs::write(
            repo_root.join("README.md"),
            "# Task Tracker\nTrack work across teams.\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"planner-server\"]\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("planner-server/Cargo.toml"),
            "[package]\nname = \"planner-server\"\nversion = \"0.1.0\"\n[dependencies]\naxum = \"0.7\"\nserde = \"1\"\n",
        )
        .unwrap();
        std::fs::write(
            repo_root.join("planner-server/src/lib.rs"),
            "pub fn ready() {}\n",
        )
        .unwrap();

        let output = StdCommand::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&repo_root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = StdCommand::new("git")
            .args(["config", "user.email", "planner-tests@example.com"])
            .current_dir(&repo_root)
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = StdCommand::new("git")
            .args(["config", "user.name", "Planner Tests"])
            .current_dir(&repo_root)
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = StdCommand::new("git")
            .args(["add", "."])
            .current_dir(&repo_root)
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = StdCommand::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(&repo_root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        repo_root
    }

    fn seed_review_pending_import(
        state: &Arc<AppState>,
        project_name: &str,
    ) -> (Project, ProjectImportJob, ProjectImportDraft) {
        let project =
            state
                .projects
                .create("dev|local", project_name, None, None, Vec::new(), None);
        let seeded_session = state.sessions.create("dev|local");
        state.sessions.update(seeded_session.id, |draft| {
            draft.project_id = Some(project.id);
            draft.project_slug = Some(project.slug.clone());
            draft.project_name = Some(project.name.clone());
            draft.cxdb_project_id = Some(project.id);
            draft.project_description = Some(format!(
                "Imported planning brief for {}.\n\nRepository brief: Task tracker.",
                project.name
            ));
            draft.ensure_title_from_description();
        });

        let (job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/imported-repo".into(),
                "https://github.com/example/imported-repo".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .update_binding_source_metadata(
                project.id,
                Some("main".into()),
                Some("deadbeef".into()),
                format!("/tmp/imports/{}", project.slug),
            )
            .unwrap();
        let draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: job.id,
                project_id: project.id,
                analysis_summary: format!(
                    "Imported draft for {} from https://github.com/example/imported-repo.",
                    project.name
                ),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![
                    serde_json::from_value(sample_component_json()).unwrap(),
                    serde_json::from_value(sample_technology_json()).unwrap(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let job = state
            .imports
            .mark_job_review_pending(
                job.id,
                "Import draft ready. Review imported context in the seeded session.",
                draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();
        (project, job, draft)
    }

    #[tokio::test]
    async fn test_get_project_import_review_returns_project_scoped_review_payload() {
        let state = test_state();
        let (project, job, _) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.project.id, project.id);
        assert_eq!(payload.import_job.id, job.id);
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::ReviewPending
        );
        assert_eq!(
            payload.source_binding.default_branch.as_deref(),
            Some("main")
        );
        assert_eq!(
            payload
                .import_draft
                .as_ref()
                .map(|draft| draft.discovered_nodes.len()),
            Some(2)
        );
        assert_eq!(
            payload
                .import_review_selection
                .as_ref()
                .map(|selection| selection.included_node_count),
            Some(2)
        );
        assert_eq!(
            payload
                .review_nodes
                .as_ref()
                .map(|nodes| nodes.iter().filter(|node| node.included).count()),
            Some(2)
        );
    }

    #[tokio::test]
    async fn test_update_project_import_review_selection_excludes_node_on_latest_review() {
        let state = test_state();
        let (project, _job, draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());
        let target_node_id = draft.discovered_nodes[0].id().to_string();

        let req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-review-selection",
                project.slug
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "node_id": target_node_id,
                    "included": false,
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload
                .import_review_selection
                .as_ref()
                .map(|selection| selection.excluded_node_ids.clone()),
            Some(vec![draft.discovered_nodes[0].id().to_string()])
        );
        assert!(payload
            .review_nodes
            .as_ref()
            .and_then(|nodes| nodes.iter().find(|node| node.node_id == target_node_id))
            .is_some_and(|node| !node.included));
    }

    #[tokio::test]
    async fn test_apply_project_import_review_promotes_draft_and_is_idempotent() {
        let state = test_state();
        let (project, job, draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::Applied
        );
        assert!(state.proposals.list(None).is_empty());

        let root_id = import_project_root_node_id(&project.id.to_string());
        let root = state
            .blueprints
            .get_node(root_id.as_str())
            .expect("project root should exist after apply");
        assert!(matches!(
            root.scope().project.as_ref().map(|scope| scope.project_id.as_str()),
            Some(project_id) if project_id == project.id.to_string()
        ));

        let snapshot = state.blueprints.snapshot();
        let applied_review_metadata = import_review_metadata(job.id);
        for node in &draft.discovered_nodes {
            let applied = state
                .blueprints
                .get_node(node.id().as_str())
                .expect("draft node should be promoted");
            assert!(matches!(
                applied.scope().project.as_ref().map(|scope| scope.project_id.as_str()),
                Some(project_id) if project_id == project.id.to_string()
            ));
            assert!(
                applied.scope().lifecycle
                    == planner_schemas::artifacts::blueprint::NodeLifecycle::Active
            );
            assert!(node_tags(&applied)
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(IMPORT_DRAFT_OWNED_TAG)));
            assert!(node_tags(&applied)
                .iter()
                .any(|tag| { tag.eq_ignore_ascii_case(&import_review_metadata(job.id)) }));
            assert!(snapshot.edges.iter().any(|edge| {
                edge.source.as_str() == root_id.as_str()
                    && edge.target.as_str() == node.id().as_str()
                    && matches!(
                        edge.edge_type,
                        planner_schemas::artifacts::blueprint::EdgeType::Contains
                    )
                    && edge.metadata.as_deref() == Some(applied_review_metadata.as_str())
            }));
        }

        let (node_count, edge_count) = state.blueprints.counts();
        assert_eq!(node_count, draft.discovered_nodes.len() + 1);
        assert_eq!(edge_count, draft.discovered_nodes.len());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::Applied
        );
        assert_eq!(state.blueprints.counts(), (node_count, edge_count));
        assert_eq!(
            state.imports.get_job(job.id).map(|current| current.status),
            Some(crate::import::ImportStatus::Applied)
        );
    }

    #[tokio::test]
    async fn test_apply_project_import_review_promotes_only_selected_nodes() {
        let state = test_state();
        let (project, job, draft) = seed_review_pending_import(&state, "Imported Repo");
        let excluded_node_id = draft.discovered_nodes[0].id().to_string();
        state
            .imports
            .set_review_node_included(job.id, project.id, &excluded_node_id, false)
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(state.blueprints.get_node(&excluded_node_id).is_none());
        assert!(state
            .blueprints
            .get_node(draft.discovered_nodes[1].id().as_str())
            .is_some());
    }

    #[tokio::test]
    async fn test_apply_project_import_review_leaves_job_review_pending_when_flush_fails() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_import_apply_flush_{}", Uuid::new_v4()));
        let state = test_state_with_persistent_blueprints(&data_dir);
        let (project, job, draft) = seed_review_pending_import(&state, "Imported Repo");
        std::fs::remove_dir_all(data_dir.join("blueprint/nodes")).unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let current_job = state
            .imports
            .get_job(job.id)
            .expect("job should remain persisted");
        assert_eq!(
            current_job.status,
            crate::import::ImportStatus::ReviewPending
        );
        assert!(state
            .blueprints
            .get_node(draft.discovered_nodes[0].id().as_str())
            .is_some());
        assert!(state.proposals.list(None).is_empty());
    }

    #[tokio::test]
    async fn test_apply_project_import_review_archives_stale_import_owned_nodes_and_preserves_manual_nodes(
    ) {
        let state = test_state();
        let (project, first_job, first_draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let first_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(first_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let mut manual_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_component_json()).unwrap();
        if let planner_schemas::artifacts::blueprint::BlueprintNode::Component(component) =
            &mut manual_node
        {
            component.id =
                planner_schemas::artifacts::blueprint::NodeId::from_raw("comp-manual-9f8e7d6c");
            component.name = "Manual Workflow".into();
            component.tags = vec!["manual".into()];
            component.scope = project_blueprint_scope(&project);
        }
        state.blueprints.upsert_node(manual_node.clone());

        let seeded_session = state.sessions.create("dev|local");
        state.sessions.update(seeded_session.id, |draft| {
            draft.project_id = Some(project.id);
            draft.project_slug = Some(project.slug.clone());
            draft.project_name = Some(project.name.clone());
            draft.cxdb_project_id = Some(project.id);
            draft.project_description = Some(format!(
                "Imported planning brief for {}.\n\nRepository brief: Task tracker refresh.",
                project.name
            ));
            draft.ensure_title_from_description();
        });

        let (second_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let second_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: second_job.id,
                project_id: project.id,
                analysis_summary: format!(
                    "Imported draft refresh for {} from https://github.com/example/imported-repo.",
                    project.name
                ),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("beadfeed".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_technology_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                second_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                second_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let archived = state
            .blueprints
            .get_node(first_draft.discovered_nodes[0].id().as_str())
            .expect("first component should remain as archived history");
        assert_eq!(
            archived.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Archived
        );
        assert!(node_tags(&archived)
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(IMPORT_DRAFT_OWNED_TAG)));

        let retained = state
            .blueprints
            .get_node(first_draft.discovered_nodes[1].id().as_str())
            .expect("shared import node id should stay active");
        assert_eq!(
            retained.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        );
        assert!(node_tags(&retained)
            .iter()
            .any(|tag| { tag.eq_ignore_ascii_case(&import_review_metadata(second_job.id)) }));

        let manual = state
            .blueprints
            .get_node(manual_node.id().as_str())
            .expect("manual node should remain untouched");
        assert_eq!(
            manual.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        );
        assert!(!node_tags(&manual)
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(IMPORT_DRAFT_OWNED_TAG)));

        let root_id = import_project_root_node_id(&project.id.to_string());
        let snapshot = state.blueprints.snapshot();
        let second_review_metadata = import_review_metadata(second_job.id);
        assert!(!snapshot.edges.iter().any(|edge| {
            edge.source.as_str() == root_id.as_str()
                && edge.target.as_str() == first_draft.discovered_nodes[0].id().as_str()
        }));
        assert!(snapshot.edges.iter().any(|edge| {
            edge.source.as_str() == root_id.as_str()
                && edge.target.as_str() == first_draft.discovered_nodes[1].id().as_str()
                && edge.metadata.as_deref() == Some(second_review_metadata.as_str())
        }));

        assert_eq!(
            state.imports.get_job(first_job.id).map(|job| job.status),
            Some(crate::import::ImportStatus::Applied)
        );
        assert_eq!(
            state.imports.get_job(second_job.id).map(|job| job.status),
            Some(crate::import::ImportStatus::Applied)
        );
    }

    #[tokio::test]
    async fn test_restore_project_import_history_entry_reactivates_historical_import_state() {
        let state = test_state();
        let (project, first_job, first_draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let first_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(first_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let mut manual_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_component_json()).unwrap();
        if let planner_schemas::artifacts::blueprint::BlueprintNode::Component(component) =
            &mut manual_node
        {
            component.id =
                planner_schemas::artifacts::blueprint::NodeId::from_raw("comp-manual-restore");
            component.name = "Manual Workflow".into();
            component.tags = vec!["manual".into()];
            component.scope = project_blueprint_scope(&project);
        }
        state.blueprints.upsert_node(manual_node.clone());

        let seeded_session = state.sessions.create("dev|local");
        state.sessions.update(seeded_session.id, |draft| {
            draft.project_id = Some(project.id);
            draft.project_slug = Some(project.slug.clone());
            draft.project_name = Some(project.name.clone());
            draft.cxdb_project_id = Some(project.id);
            draft.project_description = Some(format!(
                "Imported planning brief for {}.\n\nRepository brief: Task tracker refresh.",
                project.name
            ));
            draft.ensure_title_from_description();
        });

        let (second_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let second_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: second_job.id,
                project_id: project.id,
                analysis_summary: format!(
                    "Imported draft refresh for {} from https://github.com/example/imported-repo.",
                    project.name
                ),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("beadfeed".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_technology_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                second_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                second_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();

        let second_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(second_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore",
                project.slug, first_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::Applied
        );
        assert_eq!(payload.import_job.restored_from_job_id, Some(first_job.id));

        let reactivated = state
            .blueprints
            .get_node(first_draft.discovered_nodes[0].id().as_str())
            .expect("historical component should be restored");
        assert_eq!(
            reactivated.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        );

        let retained = state
            .blueprints
            .get_node(first_draft.discovered_nodes[1].id().as_str())
            .expect("technology should remain present");
        assert_eq!(
            retained.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        );

        let manual = state
            .blueprints
            .get_node(manual_node.id().as_str())
            .expect("manual node should remain untouched");
        assert_eq!(
            manual.scope().lifecycle,
            planner_schemas::artifacts::blueprint::NodeLifecycle::Active
        );
        assert!(!node_tags(&manual)
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(IMPORT_DRAFT_OWNED_TAG)));
    }

    #[tokio::test]
    async fn test_restore_project_import_history_entry_is_blocked_by_pending_review() {
        let state = test_state();
        let (project, applied_job, _draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let first_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(first_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let seeded_session = state.sessions.create("dev|local");
        let (pending_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let pending_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending restore blocker.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("feedface".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                pending_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();

        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_restore_project_import_history_entry_for_review_reopens_applied_job_without_mutating_blueprint(
    ) {
        let state = test_state();
        let (project, applied_job, applied_draft) =
            seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let first_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(first_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let counts_before = state.blueprints.counts();
        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore-for-review",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::ReviewPending
        );
        assert_eq!(
            payload.import_job.restored_from_job_id,
            Some(applied_job.id)
        );
        assert_eq!(state.blueprints.counts(), counts_before);
        assert_eq!(
            payload
                .import_draft
                .as_ref()
                .map(|draft| draft.discovered_nodes.len()),
            Some(applied_draft.discovered_nodes.len())
        );
        assert_eq!(
            payload
                .import_review_selection
                .as_ref()
                .map(|selection| selection.excluded_node_ids.len()),
            Some(0)
        );
        assert_eq!(
            state
                .imports
                .latest_review_job_for_project(project.id)
                .map(|job| job.id),
            Some(payload.import_job.id)
        );
    }

    #[tokio::test]
    async fn test_restore_project_import_history_entry_for_review_is_blocked_by_pending_review() {
        let state = test_state();
        let (project, applied_job, _draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let first_apply = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(first_apply).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let seeded_session = state.sessions.create("dev|local");
        let (pending_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let pending_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: pending_job.id,
                project_id: project.id,
                analysis_summary: "Pending restore-for-review blocker.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("feedface".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                pending_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                pending_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();

        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore-for-review",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_restore_project_import_review_draft_reopens_historical_review_pending_job() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Imported Repo", None, None, Vec::new(), None);
        let (_initial_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/imported-repo".into(),
                "https://github.com/example/imported-repo".into(),
                true,
            )
            .unwrap();
        state
            .imports
            .update_binding_source_metadata(
                project.id,
                Some("main".into()),
                Some("deadbeef".into()),
                format!("/tmp/imports/{}", project.slug),
            )
            .unwrap();

        let seeded_session = state.sessions.create("dev|local");
        let (historical_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let historical_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: historical_job.id,
                project_id: project.id,
                analysis_summary: "Historical review draft.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let historical_job = state
            .imports
            .mark_job_review_pending(
                historical_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                historical_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();
        state
            .imports
            .set_review_node_included(
                historical_job.id,
                project.id,
                historical_draft.discovered_nodes[0].id().as_str(),
                false,
            )
            .unwrap();

        let (latest_applied_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let latest_applied_job = state
            .imports
            .mark_job_applied(
                latest_applied_job.id,
                "Import draft applied and reconciled against the canonical project blueprint.",
                None,
            )
            .unwrap();

        let counts_before = state.blueprints.counts();
        let app = routes(state.clone());
        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore-review-draft",
                project.slug, historical_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: ProjectImportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload.import_job.status,
            crate::import::ImportStatus::ReviewPending
        );
        assert_eq!(
            payload.import_job.restored_from_job_id,
            Some(historical_job.id)
        );
        assert_eq!(
            payload.import_job.seed_session_id,
            historical_job.seed_session_id
        );
        assert_eq!(
            payload
                .import_draft
                .as_ref()
                .map(|draft| draft.analysis_summary.as_str()),
            Some("Historical review draft.")
        );
        assert_eq!(
            payload
                .import_review_selection
                .as_ref()
                .map(|selection| selection.excluded_node_ids.len()),
            Some(0)
        );
        assert_eq!(state.blueprints.counts(), counts_before);
        assert_eq!(
            state
                .imports
                .latest_job_for_project(project.id)
                .map(|job| job.id),
            Some(payload.import_job.id)
        );
        assert_eq!(
            state
                .imports
                .latest_review_job_for_project(project.id)
                .map(|job| job.id),
            Some(payload.import_job.id)
        );
        assert_eq!(
            state
                .imports
                .get_job(latest_applied_job.id)
                .map(|job| job.status),
            Some(crate::import::ImportStatus::Applied)
        );
    }

    #[tokio::test]
    async fn test_restore_project_import_review_draft_is_blocked_by_current_pending_review() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Imported Repo", None, None, Vec::new(), None);
        let (_initial_job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/imported-repo".into(),
                "https://github.com/example/imported-repo".into(),
                true,
            )
            .unwrap();

        let seeded_session = state.sessions.create("dev|local");
        let (historical_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let historical_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: historical_job.id,
                project_id: project.id,
                analysis_summary: "Historical review draft.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("cafebabe".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                historical_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                historical_draft.analysis_summary.clone(),
                seeded_session.id,
            )
            .unwrap();

        let current_seeded_session = state.sessions.create("dev|local");
        let (current_job, _) = state.imports.create_reimport_job(project.id).unwrap();
        let current_draft = state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: current_job.id,
                project_id: project.id,
                analysis_summary: "Current review draft.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/imported-repo".into(),
                    local_root: format!("/tmp/imports/{}", project.slug),
                    default_branch: Some("main".into()),
                    head_revision: Some("feedface".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        state
            .imports
            .mark_job_review_pending(
                current_job.id,
                "Import draft ready. Review imported context in the seeded session.",
                current_draft.analysis_summary.clone(),
                current_seeded_session.id,
            )
            .unwrap();

        let app = routes(state.clone());
        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore-review-draft",
                project.slug, historical_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_restore_project_import_review_draft_rejects_applied_job_target() {
        let state = test_state();
        let (project, applied_job, _draft) = seed_review_pending_import(&state, "Imported Repo");
        let app = routes(state.clone());

        let apply_req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/import-review", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(apply_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let restore_req = Request::builder()
            .method("POST")
            .uri(format!(
                "/projects/{}/import-history/{}/restore-review-draft",
                project.slug, applied_job.id
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(restore_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_models() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/models")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let models: ModelsResponse = serde_json::from_slice(&body).unwrap();
        assert!(models.models.len() >= 6);
    }

    #[tokio::test]
    async fn test_create_and_get_project_by_slug() {
        let state = test_state();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/projects")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "name": "Task Tracker",
                    "description": "Planner project container"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: ProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(created.project.name, "Task Tracker");
        assert_eq!(created.project.owner_user_id, "dev|local");

        let req = Request::builder()
            .uri(format!("/projects/{}", created.project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let fetched: ProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(fetched.project.id, created.project.id);
    }

    #[tokio::test]
    async fn test_list_projects_excludes_archived_by_default() {
        let state = test_state();
        let active =
            state
                .projects
                .create("dev|local", "Active Project", None, None, Vec::new(), None);
        let archived = state.projects.create(
            "dev|local",
            "Archived Project",
            None,
            None,
            Vec::new(),
            None,
        );
        let _ = state.projects.set_archived(archived.id, true).unwrap();

        let app = routes(state);
        let req = Request::builder()
            .method("GET")
            .uri("/projects")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListProjectsResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(listed.projects.len(), 1);
        assert_eq!(listed.projects[0].id, active.id);
    }

    #[tokio::test]
    async fn test_list_projects_can_include_archived() {
        let state = test_state();
        state
            .projects
            .create("dev|local", "Active Project", None, None, Vec::new(), None);
        let archived = state.projects.create(
            "dev|local",
            "Archived Project",
            None,
            None,
            Vec::new(),
            None,
        );
        let _ = state.projects.set_archived(archived.id, true).unwrap();

        let app = routes(state);
        let req = Request::builder()
            .method("GET")
            .uri("/projects?include_archived=true")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListProjectsResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(listed.projects.len(), 2);
        assert!(listed
            .projects
            .iter()
            .any(|project| project.archived_at.is_some()));
    }

    #[tokio::test]
    async fn test_update_project_can_archive() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Archive Toggle", None, None, Vec::new(), None);
        let app = routes(state);

        let req = Request::builder()
            .method("PATCH")
            .uri(format!("/projects/{}", project.slug))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "archived": true }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: ProjectResponse = serde_json::from_slice(&body).unwrap();
        assert!(updated.project.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_update_project_can_unarchive() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Archive Toggle", None, None, Vec::new(), None);
        let _ = state.projects.set_archived(project.id, true).unwrap();
        let app = routes(state);

        let req = Request::builder()
            .method("PATCH")
            .uri(format!("/projects/{}", project.slug))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "archived": false }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: ProjectResponse = serde_json::from_slice(&body).unwrap();
        assert!(updated.project.archived_at.is_none());
    }

    #[tokio::test]
    async fn test_get_archived_project_by_slug_still_works() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Archived Direct Fetch",
            None,
            None,
            Vec::new(),
            None,
        );
        let _ = state.projects.set_archived(project.id, true).unwrap();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let fetched: ProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(fetched.project.id, project.id);
        assert!(fetched.project.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_project_removes_project_record() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Delete Target", None, None, Vec::new(), None);
        let app = routes(state.clone());

        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.project_id, project.id.to_string());
        assert_eq!(summary.project_name, "Delete Target");
        assert_eq!(summary.deleted_sessions, 0);
        assert!(summary.deleted_project_record);
        assert!(state.projects.resolve_ref(&project.slug).is_none());
    }

    #[tokio::test]
    async fn test_delete_project_purges_import_artifacts_and_managed_checkout() {
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Delete Import Artifacts",
            None,
            None,
            Vec::new(),
            None,
        );
        let seeded_session = state.sessions.create("dev|local");
        let (job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::GitHub,
                "https://github.com/example/delete-import".into(),
                "https://github.com/example/delete-import".into(),
                true,
            )
            .unwrap();
        let managed_root = state
            .imports
            .managed_checkout_path(project.id, ImportProvider::GitHub);
        std::fs::create_dir_all(managed_root.join("planner-server")).unwrap();
        state
            .imports
            .update_binding_source_metadata(
                project.id,
                Some("main".into()),
                Some("deadbeef".into()),
                managed_root.to_string_lossy().to_string(),
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: job.id,
                project_id: project.id,
                analysis_summary: "Imported draft for delete coverage.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::GitHub,
                    canonical_ref: "https://github.com/example/delete-import".into(),
                    local_root: managed_root.to_string_lossy().to_string(),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let _ = state
            .imports
            .mark_job_review_pending(
                job.id,
                "Import draft ready. Review imported context in the seeded session.",
                "Imported draft for delete coverage.".into(),
                seeded_session.id,
            )
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.deleted_import_jobs, 1);
        assert_eq!(summary.deleted_import_drafts, 1);
        assert_eq!(summary.deleted_import_managed_roots, 1);
        assert!(state.imports.get_job(job.id).is_none());
        assert!(state.imports.get_draft(job.id).is_none());
        assert!(state.imports.get_binding(project.id).is_none());
        assert!(!managed_root.exists());
    }

    #[tokio::test]
    async fn test_delete_project_preserves_external_local_import_root() {
        let local_root = create_temp_local_git_repo("delete-local-root");
        let state = test_state();
        let project = state.projects.create(
            "dev|local",
            "Delete Local Root",
            None,
            None,
            Vec::new(),
            None,
        );
        let (job, _) = state
            .imports
            .create(
                project.id,
                ImportProvider::Local,
                local_root.to_string_lossy().to_string(),
                local_root.to_string_lossy().to_string(),
                false,
            )
            .unwrap();
        state
            .imports
            .update_binding_source_metadata(
                project.id,
                Some("main".into()),
                Some("deadbeef".into()),
                local_root.to_string_lossy().to_string(),
            )
            .unwrap();
        state
            .imports
            .save_draft(ProjectImportDraft {
                job_id: job.id,
                project_id: project.id,
                analysis_summary: "Imported local draft.".into(),
                source_metadata: crate::import::ImportDraftSourceMetadata {
                    provider: ImportProvider::Local,
                    canonical_ref: local_root.to_string_lossy().to_string(),
                    local_root: local_root.to_string_lossy().to_string(),
                    default_branch: Some("main".into()),
                    head_revision: Some("deadbeef".into()),
                },
                discovered_nodes: vec![serde_json::from_value(sample_component_json()).unwrap()],
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .unwrap();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.deleted_import_jobs, 1);
        assert_eq!(summary.deleted_import_drafts, 1);
        assert_eq!(summary.deleted_import_managed_roots, 0);
        assert!(local_root.exists());
    }

    #[tokio::test]
    async fn test_delete_project_removes_owned_sessions() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Delete Sessions", None, None, Vec::new(), None);
        let session_a = state.sessions.create("dev|local");
        let session_b = state.sessions.create("dev|local");
        state.sessions.update(session_a.id, |s| {
            s.project_id = Some(project.id);
            s.project_slug = Some(project.slug.clone());
            s.project_name = Some(project.name.clone());
        });
        state.sessions.update(session_b.id, |s| {
            s.project_id = Some(project.id);
            s.project_slug = Some(project.slug.clone());
            s.project_name = Some(project.name.clone());
        });

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.deleted_sessions, 2);
        assert!(state.sessions.get(session_a.id).is_none());
        assert!(state.sessions.get(session_b.id).is_none());
    }

    #[tokio::test]
    async fn test_delete_project_removes_session_event_files() {
        let data_dir =
            std::env::temp_dir().join(format!("planner_delete_events_{}", Uuid::new_v4()));
        let state = test_state_with_event_store(&data_dir);
        let project =
            state
                .projects
                .create("dev|local", "Delete Events", None, None, Vec::new(), None);
        let session = state.sessions.create("dev|local");
        state.sessions.update(session.id, |s| {
            s.project_id = Some(project.id);
            s.project_slug = Some(project.slug.clone());
            s.project_name = Some(project.name.clone());
        });

        let store = state.event_store.as_ref().unwrap();
        let events = vec![planner_core::observability::PlannerEvent::info(
            planner_core::observability::EventSource::System,
            "delete.events",
            "Persist me",
        )];
        store.save_session_events(session.id, &events).unwrap();
        let event_path = data_dir
            .join("events")
            .join(format!("{}.msgpack", session.id));
        assert!(event_path.exists());

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.deleted_session_event_files, 1);
        assert!(!event_path.exists());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_delete_project_stops_active_session_work() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Delete Active", None, None, Vec::new(), None);
        let session = state.sessions.create("dev|local");
        state.sessions.update(session.id, |s| {
            s.project_id = Some(project.id);
            s.project_slug = Some(project.slug.clone());
            s.project_name = Some(project.name.clone());
            s.pipeline_running = true;
            s.intake_phase = "pipeline_running".into();
            s.project_description = Some("Long-running pipeline".into());
        });
        let _ = spawn_pipeline_runtime(state.clone(), session.id, "Long-running pipeline".into());
        assert!(state.pipeline_runtimes.get(session.id).is_some());

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.stopped_pipeline_sessions, 1);
        assert_eq!(summary.stopped_live_sessions, 0);
        assert!(state.pipeline_runtimes.get(session.id).is_none());
    }

    #[tokio::test]
    async fn test_delete_project_counts_only_registry_backed_live_work() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Delete Detached", None, None, Vec::new(), None);
        let session = state.sessions.create("dev|local");
        state.sessions.update(session.id, |s| {
            s.project_id = Some(project.id);
            s.project_slug = Some(project.slug.clone());
            s.project_name = Some(project.name.clone());
            s.pipeline_running = true;
            s.intake_phase = "interviewing".into();
            s.interview_live_attached = false;
            s.interview_runtime_active = false;
        });

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.stopped_live_sessions, 0);
        assert_eq!(summary.stopped_pipeline_sessions, 0);
    }

    #[tokio::test]
    async fn test_delete_project_returns_500_when_blueprint_flush_fails() {
        use planner_schemas::artifacts::blueprint::{
            BlueprintNode, Decision, DecisionStatus, NodeId, NodeLifecycle, NodeScope,
            ProjectScope, ScopeClass, SecondaryScopeRefs,
        };

        let data_dir =
            std::env::temp_dir().join(format!("planner_delete_phase6_flush_{}", Uuid::new_v4()));
        let state = test_state_with_persistent_blueprints(&data_dir);
        let project = state.projects.create(
            "dev|local",
            "Delete Flush Fail",
            None,
            None,
            Vec::new(),
            None,
        );

        state
            .blueprints
            .upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-delete-flush-fail"),
                title: "Project local".into(),
                status: DecisionStatus::Proposed,
                context: "local".into(),
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: project.id.to_string(),
                        project_name: Some(project.name.clone()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
                    scope_review: None,
                },
                created_at: "2026-03-08T00:00:00Z".into(),
                updated_at: "2026-03-08T00:00:00Z".into(),
            }));
        state.blueprints.flush().unwrap();

        let events_path = data_dir.join("blueprint/events.msgpack");
        std::fs::remove_file(&events_path).unwrap();
        std::fs::create_dir_all(&events_path).unwrap();

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(state.projects.resolve_ref(&project.slug).is_some());

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_delete_project_forbidden_for_non_owner() {
        let state = test_state();
        let project =
            state
                .projects
                .create("other_user|123", "Not Yours", None, None, Vec::new(), None);
        let app = routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_delete_project_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/projects/missing-project")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_project_purges_cxdb_and_blueprint_scope() {
        use planner_schemas::artifacts::blueprint::{
            BlueprintNode, Decision, DecisionStatus, NodeId, NodeLifecycle, NodeScope,
            ProjectScope, ScopeClass, SecondaryScopeRefs, SharedScope,
        };

        let data_dir =
            std::env::temp_dir().join(format!("planner_delete_phase6d_{}", Uuid::new_v4()));
        let cxdb =
            planner_core::cxdb::durable::DurableCxdbEngine::open(data_dir.join("cxdb")).unwrap();
        let state = Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: Arc::new(ImmediateSuccessImportAcquirer),
            import_analyzer: crate::import::default_import_analyzer(),
            auth_config: None,
            event_store: None,
            cxdb: Some(cxdb),
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        });

        let project = state.projects.create(
            "dev|local",
            "Delete Project 6D",
            None,
            None,
            Vec::new(),
            None,
        );
        let other_project = state.projects.create(
            "dev|local",
            "Other Project 6D",
            None,
            None,
            Vec::new(),
            None,
        );
        let run_a = Uuid::new_v4();
        let run_b = Uuid::new_v4();
        state
            .cxdb
            .as_ref()
            .unwrap()
            .register_run(project.id, run_a)
            .unwrap();
        state
            .cxdb
            .as_ref()
            .unwrap()
            .register_run(other_project.id, run_b)
            .unwrap();

        state
            .blueprints
            .upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-delete-local"),
                title: "Project local".into(),
                status: DecisionStatus::Proposed,
                context: "local".into(),
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: project.id.to_string(),
                        project_name: Some(project.name.clone()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: false,
                    shared: None,
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
                    scope_review: None,
                },
                created_at: "2026-03-08T00:00:00Z".into(),
                updated_at: "2026-03-08T00:00:00Z".into(),
            }));

        state
            .blueprints
            .upsert_node(BlueprintNode::Decision(Decision {
                id: NodeId::from_raw("dec-delete-shared"),
                title: "Shared knowledge".into(),
                status: DecisionStatus::Accepted,
                context: "shared".into(),
                options: vec![],
                consequences: vec![],
                assumptions: vec![],
                supersedes: None,
                tags: vec![],
                documentation: None,
                scope: NodeScope {
                    scope_class: ScopeClass::Project,
                    project: Some(ProjectScope {
                        project_id: other_project.id.to_string(),
                        project_name: Some(other_project.name.clone()),
                    }),
                    secondary: SecondaryScopeRefs::default(),
                    is_shared: true,
                    shared: Some(SharedScope {
                        linked_project_ids: vec![
                            project.id.to_string(),
                            other_project.id.to_string(),
                        ],
                        inherit_to_linked_projects: true,
                    }),
                    lifecycle: NodeLifecycle::Active,
                    override_scope: None,
                    scope_review: None,
                },
                created_at: "2026-03-08T00:00:00Z".into(),
                updated_at: "2026-03-08T00:00:00Z".into(),
            }));

        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/projects/{}", project.slug))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let summary: DeleteProjectResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.deleted_cxdb_runs, 1);
        assert_eq!(summary.deleted_blueprint_nodes, 1);
        assert_eq!(summary.unlinked_shared_blueprint_nodes, 1);

        assert!(state
            .cxdb
            .as_ref()
            .unwrap()
            .list_runs(project.id)
            .is_empty());
        assert_eq!(
            state.cxdb.as_ref().unwrap().list_runs(other_project.id),
            vec![run_b]
        );
        assert!(state.blueprints.get_node("dec-delete-local").is_none());
        let shared = state.blueprints.get_node("dec-delete-shared").unwrap();
        let linked = shared
            .scope()
            .shared
            .as_ref()
            .map(|scope| scope.linked_project_ids.clone())
            .unwrap_or_default();
        assert_eq!(linked, vec![other_project.id.to_string()]);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn test_create_project_session_assigns_project_context() {
        let state = test_state();
        let project =
            state
                .projects
                .create("dev|local", "Ops Console", None, None, Vec::new(), None);
        let app = routes(state);

        let req = Request::builder()
            .method("POST")
            .uri(format!("/projects/{}/sessions", project.slug))
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: CreateSessionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(created.session.project_id, Some(project.id));
        assert_eq!(
            created.session.project_slug.as_deref(),
            Some(project.slug.as_str())
        );
        assert_eq!(
            created.session.project_name.as_deref(),
            Some(project.name.as_str())
        );
    }

    #[tokio::test]
    async fn test_create_session() {
        let state = test_state();
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: CreateSessionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(created.session.messages.len(), 1);
        // In dev mode, user_id is "dev|local"
        assert_eq!(created.session.user_id, "dev|local");
        assert_eq!(state.sessions.count(), 1);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let state = test_state();
        // Pre-create two sessions in dev mode (user "dev|local")
        state.sessions.create("dev|local");
        state.sessions.create("dev|local");

        let app = routes(state);

        let req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_list_sessions_hides_archived_by_default() {
        let state = test_state();
        let active = state.sessions.create("dev|local");
        let archived = state.sessions.create("dev|local");
        state.sessions.update(archived.id, |session| {
            session.set_archived(true);
        });

        let app = routes(state);

        let req = Request::builder()
            .method("GET")
            .uri("/sessions")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.sessions.len(), 1);
        assert_eq!(listed.sessions[0].id, active.id);
    }

    #[tokio::test]
    async fn test_list_sessions_can_include_archived() {
        let state = test_state();
        state.sessions.create("dev|local");
        let archived = state.sessions.create("dev|local");
        state.sessions.update(archived.id, |session| {
            session.set_archived(true);
        });

        let app = routes(state);

        let req = Request::builder()
            .method("GET")
            .uri("/sessions?include_archived=true")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.sessions.len(), 2);
        assert!(listed.sessions.iter().any(|session| session.archived));
    }

    #[tokio::test]
    async fn test_get_session() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;

        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let wrapped: GetSessionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(wrapped.session.id, id);
    }

    #[tokio::test]
    async fn test_get_session_includes_current_prompt_in_checkpoint_payload() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;

        state.sessions.update(id, |draft| {
            let checkpoint = draft.ensure_checkpoint();
            checkpoint.current_prompt = Some(planner_schemas::PromptEnvelope {
                prompt_id: "prompt-123".into(),
                kind: planner_schemas::PromptKind::QuestionBatch,
                title: "Continue interview".into(),
                instructions: None,
                origin_category_id: None,
                category_path: Vec::new(),
                items: vec![planner_schemas::PromptItem {
                    item_id: "item-1".into(),
                    kind: planner_schemas::PromptItemKind::Discovery,
                    target_dimension: Some(planner_schemas::Dimension::Goal),
                    section_ref: None,
                    text: "What is the primary goal?".into(),
                    options: vec![planner_schemas::PromptOption {
                        option_id: "opt-1".into(),
                        label: "Ship MVP".into(),
                        semantic_value: "ship_mvp".into(),
                        direct_effect: None,
                    }],
                    response_mode: planner_schemas::PromptResponseMode::SingleSelectWithCustomText,
                    required: true,
                    priority: 100,
                    dependency_item_ids: vec![],
                }],
                draft_snapshot: None,
                required_item_ids: vec!["item-1".into()],
                allow_partial_submit: true,
                ui_hints: planner_schemas::PromptUiHints {
                    preferred_layout: planner_schemas::PromptPreferredLayout::Cards,
                    show_draft_sidebar: false,
                },
                based_on_turn: 1,
                created_at: "2026-03-08T00:00:00Z".into(),
            });
        });

        let app = routes(state);
        let req = Request::builder()
            .uri(format!("/sessions/{}", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let wrapped: GetSessionResponse = serde_json::from_slice(&body).unwrap();
        let prompt = wrapped
            .session
            .checkpoint
            .and_then(|checkpoint| checkpoint.current_prompt)
            .expect("current_prompt should be present in session payload");
        assert_eq!(prompt.prompt_id, "prompt-123");
        assert_eq!(prompt.items.len(), 1);
    }

    #[tokio::test]
    async fn test_get_session_wrong_user() {
        let state = test_state();
        // Create a session belonging to a different user
        let session = state.sessions.create("other_user|123");
        let id = session.id;

        let app = routes(state);

        // Request is in dev mode (claims.sub = "dev|local"), but session owner is "other_user|123"
        let req = Request::builder()
            .uri(format!("/sessions/{}", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_session_title_and_archive() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        let app = routes(state);

        let body = serde_json::to_string(&UpdateSessionRequest {
            title: Some("Renamed session".into()),
            archived: Some(true),
        })
        .unwrap();

        let req = Request::builder()
            .method("PATCH")
            .uri(format!("/sessions/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let wrapped: GetSessionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(wrapped.session.title.as_deref(), Some("Renamed session"));
        assert!(wrapped.session.archived);
    }

    #[tokio::test]
    async fn test_duplicate_session_creates_branch_copy() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        state.sessions.update(id, |session| {
            session.project_description = Some("Build an operations dashboard".into());
            session.ensure_title_from_description();
        });
        let app = routes(state.clone());

        let body = serde_json::to_string(&DuplicateSessionRequest {
            title: Some("Branch copy".into()),
        })
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/duplicate", id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let wrapped: GetSessionResponse = serde_json::from_slice(&body).unwrap();
        assert_ne!(wrapped.session.id, id);
        assert_eq!(wrapped.session.title.as_deref(), Some("Branch copy"));
        assert_eq!(
            wrapped.session.project_description.as_deref(),
            Some("Build an operations dashboard")
        );
        assert_eq!(state.sessions.count(), 2);
    }

    #[tokio::test]
    async fn test_export_session_returns_full_payload() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        state.sessions.update(id, |session| {
            session.set_title(Some("Exportable session".into()));
            session.add_message("planner", "Export me");
        });
        let app = routes(state);

        let req = Request::builder()
            .method("GET")
            .uri(format!("/sessions/{}/export", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let export: SessionExportResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(export.session.id, id);
        assert_eq!(export.session.title.as_deref(), Some("Exportable session"));
        assert_eq!(export.session.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_send_message() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;

        let app = routes(state);

        let body = serde_json::to_string(&SendMessageRequest {
            content: "Build me a task tracker".into(),
        })
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/message", id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: SendMessageResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.user_message.role, "user");
        assert_eq!(response.planner_message.role, "planner");
        assert!(response.session.pipeline_running);
        // The planner message should mention the pipeline start
        assert!(response.planner_message.content.contains("pipeline"));
        // system + user + planner = 3
        assert_eq!(response.session.messages.len(), 3);
    }

    #[tokio::test]
    async fn send_message_registers_pipeline_runtime_when_pipeline_starts() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/message", id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "content": "Build me a planner dashboard" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.pipeline_runtimes.get(id).is_some());

        stop_active_session_work(&state, id);
    }

    #[tokio::test]
    async fn retry_pipeline_registers_pipeline_runtime() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        state.sessions.update(id, |s| {
            s.project_description = Some("Retry me".into());
            s.intake_phase = "error".into();
            if let Some(stage) = s.stages.first_mut() {
                stage.status = "failed".into();
            }
        });
        let app = routes(state.clone());

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/retry-pipeline", id))
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.pipeline_runtimes.get(id).is_some());

        stop_active_session_work(&state, id);
    }

    #[tokio::test]
    async fn test_send_message_wrong_user() {
        let state = test_state();
        // Session belongs to a different user
        let session = state.sessions.create("other_user|456");
        let id = session.id;

        let app = routes(state);

        let body = serde_json::to_string(&SendMessageRequest {
            content: "Build me something".into(),
        })
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/message", id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_send_empty_message() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;

        let app = routes(state);

        let body = serde_json::to_string(&SendMessageRequest {
            content: "   ".into(), // whitespace only
        })
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/sessions/{}/message", id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_protected_endpoint_requires_token_when_auth_enabled() {
        // When auth_config is set, missing token should return 401
        let state = Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            projects: crate::project::ProjectStore::new(),
            imports: crate::import::ProjectImportStore::new(),
            import_acquirer: Arc::new(ImmediateSuccessImportAcquirer),
            import_analyzer: crate::import::default_import_analyzer(),
            auth_config: Some(AuthConfig {
                domain: "test.auth0.com".into(),
                audience: "test".into(),
                decoding_key: None,
            }),
            event_store: None,
            cxdb: None,
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
            pipeline_runtimes: crate::runtime::SessionPipelineRegistry::new(),
            started_at: std::time::Instant::now(),
        });
        let app = routes(state);

        let req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // -----------------------------------------------------------------------
    // CXDB Read API tests (Change 4)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_turns_empty() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/turns", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: ListTurnsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.turns.len(), 0);
        assert_eq!(listed.count, 0);
    }

    #[tokio::test]
    async fn test_list_turns_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/turns", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_turns_wrong_user() {
        let state = test_state();
        // Session owned by a different user
        let session = state.sessions.create("other_user|789");
        let id = session.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/turns", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_list_runs_empty() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/runs", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let run_list: RunListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(run_list.runs.len(), 0);
    }

    #[tokio::test]
    async fn test_list_runs_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/runs", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_runs_wrong_user() {
        let state = test_state();
        let session = state.sessions.create("other_user|runs");
        let id = session.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/runs", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // -----------------------------------------------------------------------
    // Events endpoint tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_events_empty() {
        let state = test_state();
        let session = state.sessions.create("dev|local");
        let id = session.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SessionEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.count, 0);
        assert_eq!(result.session_id, id.to_string());
    }

    #[tokio::test]
    async fn test_get_events_with_data() {
        use planner_core::observability::{EventSource, PlannerEvent};
        let state = test_state();
        let session_obj = state.sessions.create("dev|local");
        let id = session_obj.id;

        // Add events to the session.
        state.sessions.update(id, |s| {
            s.record_event(PlannerEvent::info(
                EventSource::Pipeline,
                "step.start",
                "Pipeline started",
            ));
            s.record_event(PlannerEvent::error(
                EventSource::LlmRouter,
                "llm.call.error",
                "LLM failed",
            ));
        });

        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SessionEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.count, 2);
        assert_eq!(result.events.len(), 2);
    }

    #[tokio::test]
    async fn test_get_events_includes_retry_and_validation_feedback() {
        use planner_core::observability::{EventSource, PlannerEvent};
        let state = test_state();
        let session_obj = state.sessions.create("dev|local");
        let id = session_obj.id;

        state.sessions.update(id, |s| {
            s.record_event(
                PlannerEvent::warn(
                    EventSource::Pipeline,
                    "pipeline.retry.started",
                    "Retrying validation loop",
                )
                .with_metadata(serde_json::json!({
                    "next_attempt": 2,
                    "max_attempts": 3,
                })),
            );
            s.record_event(
                PlannerEvent::info(
                    EventSource::Pipeline,
                    "pipeline.validation.completed",
                    "Validation attempt failed",
                )
                .with_metadata(serde_json::json!({
                    "stage": "Validate",
                    "attempt": 1,
                    "gates_passed": false,
                    "passed_count": 3,
                    "total_count": 7,
                })),
            );
        });

        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events?source=pipeline", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SessionEventsResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.count, 2);
        assert!(result
            .events
            .iter()
            .any(|event| event.step.as_deref() == Some("pipeline.retry.started")));
        let validation_event = result
            .events
            .iter()
            .find(|event| event.step.as_deref() == Some("pipeline.validation.completed"))
            .expect("validation event should be present");
        assert_eq!(validation_event.metadata["gates_passed"], false);
    }

    #[tokio::test]
    async fn test_get_events_filter_level() {
        use planner_core::observability::{EventSource, PlannerEvent};
        let state = test_state();
        let session_obj = state.sessions.create("dev|local");
        let id = session_obj.id;

        state.sessions.update(id, |s| {
            s.record_event(PlannerEvent::info(EventSource::System, "a", "info event"));
            s.record_event(PlannerEvent::error(EventSource::System, "b", "error event"));
            s.record_event(PlannerEvent::warn(EventSource::System, "c", "warn event"));
        });

        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events?level=error", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: SessionEventsResponse = serde_json::from_slice(&body).unwrap();
        // count = total matching filter, events = paginated slice
        assert_eq!(result.count, 1);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].message, "error event");
    }

    #[tokio::test]
    async fn test_admin_events_include_project_identity_for_project_sessions() {
        use planner_core::observability::{EventSource, PlannerEvent};

        let state = test_state();
        let session_obj = state.sessions.create("dev|local");
        let id = session_obj.id;
        let project = state.projects.create(
            "dev|local",
            "Admin Knowledge Project",
            None,
            None,
            Vec::new(),
            None,
        );

        state.sessions.update(id, |s| {
            s.project_id = Some(project.id);
            s.project_name = Some(project.name.clone());
            s.record_event(PlannerEvent::info(
                EventSource::Pipeline,
                "pipeline.compile",
                "Compiled project blueprint",
            ));
        });

        let app = routes(state);
        let req = Request::builder()
            .uri("/admin/events?limit=10")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: AdminEventsResponse = serde_json::from_slice(&body).unwrap();
        let session_id = id.to_string();
        let project_id = project.id.to_string();
        assert_eq!(result.total, 1);
        assert_eq!(result.events.len(), 1);
        assert_eq!(
            result.events[0].session_id.as_deref(),
            Some(session_id.as_str())
        );
        assert_eq!(
            result.events[0].project_id.as_deref(),
            Some(project_id.as_str())
        );
        assert_eq!(
            result.events[0].project_name.as_deref(),
            Some(project.name.as_str())
        );
    }

    #[tokio::test]
    async fn test_get_events_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_events_wrong_user() {
        let state = test_state();
        let session_obj = state.sessions.create("other_user|evts");
        let id = session_obj.id;
        let app = routes(state);

        let req = Request::builder()
            .uri(format!("/sessions/{}/events", id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // -----------------------------------------------------------------------
    // Blueprint API tests
    // -----------------------------------------------------------------------

    fn sample_decision_json() -> serde_json::Value {
        serde_json::json!({
            "node_type": "decision",
            "id": "dec-use-msgpack-a1b2c3d4",
            "title": "Use MessagePack for disk serialization",
            "status": "accepted",
            "context": "CXDB needs a fast, compact disk format",
            "options": [
                {
                    "name": "MessagePack",
                    "pros": ["Fast binary", "Compact"],
                    "cons": ["Not human-readable"],
                    "chosen": true
                }
            ],
            "consequences": [],
            "assumptions": [],
            "tags": ["storage", "performance"],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })
    }

    fn sample_technology_json() -> serde_json::Value {
        serde_json::json!({
            "node_type": "technology",
            "id": "tech-rust-b2c3d4e5",
            "name": "Rust",
            "version": "1.79.0",
            "category": "language",
            "ring": "adopt",
            "rationale": "Memory safety without GC",
            "tags": ["core"],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })
    }

    fn sample_component_json() -> serde_json::Value {
        serde_json::json!({
            "node_type": "component",
            "id": "comp-auth-a1b2c3d4",
            "name": "Authentication Service",
            "component_type": "service",
            "naming": {
                "origin_key": "spec:proj:root:auth",
                "source": "generated",
                "strategy": "spec_group",
                "generated_name": "Authentication Service",
                "naming_version": 1,
                "last_generated_at": "2026-03-01T00:00:00Z"
            },
            "description": "Handles sign-in and token issuance.",
            "provides": [],
            "consumes": [],
            "status": "planned",
            "tags": ["spec", "root"],
            "created_at": "2026-03-01T00:00:00Z",
            "updated_at": "2026-03-01T00:00:00Z"
        })
    }

    fn temp_scan_root(prefix: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("planner-api-{}-{}", prefix, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[tokio::test]
    async fn test_get_blueprint_empty() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/blueprint")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let bp: BlueprintResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(bp.total_nodes, 0);
        assert_eq!(bp.total_edges, 0);
        assert!(bp.nodes.is_empty());
        assert!(bp.edges.is_empty());
    }

    #[tokio::test]
    async fn test_create_blueprint_node() {
        let state = test_state();
        let app = routes(state);

        let json_body = serde_json::to_string(&sample_decision_json()).unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(json_body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let node: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(node["node_type"], "decision");
        assert_eq!(node["id"], "dec-use-msgpack-a1b2c3d4");
    }

    #[tokio::test]
    async fn test_get_blueprint_node() {
        let state = test_state();

        // Pre-insert a node.
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state);

        let req = Request::builder()
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let returned: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(returned["title"], "Use MessagePack for disk serialization");
    }

    #[tokio::test]
    async fn test_get_blueprint_node_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/blueprint/nodes/nonexistent-node")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_blueprint_nodes() {
        let state = test_state();

        // Insert two nodes of different types.
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        let app = routes(state);

        // Unfiltered list.
        let req = Request::builder()
            .uri("/blueprint/nodes")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: NodeListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.count, 2);
    }

    #[tokio::test]
    async fn test_list_blueprint_nodes_filtered() {
        let state = test_state();

        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        let app = routes(state);

        // Filter by decision only.
        let req = Request::builder()
            .uri("/blueprint/nodes?type=decision")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: NodeListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.count, 1);
        assert_eq!(list.nodes[0].node_type, "decision");
    }

    #[tokio::test]
    async fn test_list_blueprint_nodes_project_scope_includes_shared() {
        let state = test_state();

        let mut project_local = sample_decision_json();
        project_local["id"] = serde_json::json!("dec-proj-local-a1b2c3d4");
        project_local["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha"
            },
            "secondary": {},
            "is_shared": false
        });

        let mut shared = sample_decision_json();
        shared["id"] = serde_json::json!("dec-shared-a1b2c3d5");
        shared["title"] = serde_json::json!("Shared guidance");
        shared["scope"] = serde_json::json!({
            "scope_class": "global",
            "secondary": {},
            "is_shared": true,
            "shared": {
                "linked_project_ids": ["proj-alpha"],
                "inherit_to_linked_projects": true
            }
        });

        let mut other_project = sample_decision_json();
        other_project["id"] = serde_json::json!("dec-proj-local-b1b2c3d4");
        other_project["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-beta",
                "project_name": "Beta"
            },
            "secondary": {},
            "is_shared": false
        });

        let local_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(project_local).unwrap();
        let shared_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(shared).unwrap();
        let other_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(other_project).unwrap();

        state.blueprints.upsert_node(local_node);
        state.blueprints.upsert_node(shared_node);
        state.blueprints.upsert_node(other_node);

        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/nodes?project_id=proj-alpha")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: NodeListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.count, 2);
        assert!(list
            .nodes
            .iter()
            .any(|node| node.id.as_str() == "dec-proj-local-a1b2c3d4"));
        assert!(list
            .nodes
            .iter()
            .any(|node| node.id.as_str() == "dec-shared-a1b2c3d5"));

        let app = routes(state);
        let req = Request::builder()
            .uri("/blueprint/nodes?project_id=proj-alpha&include_shared=false")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: NodeListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.count, 1);
        assert_eq!(list.nodes[0].id.as_str(), "dec-proj-local-a1b2c3d4");
    }

    #[tokio::test]
    async fn test_create_blueprint_node_invalid_scope_rejected() {
        let state = test_state();
        let app = routes(state);

        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-invalid-scope-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "project",
            "secondary": {},
            "is_shared": false
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_legacy_archive_tag_migrates_to_lifecycle() {
        let state = test_state();
        let app = routes(state);

        let mut legacy = sample_decision_json();
        legacy["id"] = serde_json::json!("dec-legacy-archive-a1b2c3d4");
        legacy["tags"] = serde_json::json!(["storage", "archived"]);

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&legacy).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let node: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(node["scope"]["lifecycle"], "archived");
        assert_eq!(node["tags"], serde_json::json!(["storage"]));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_invalid_scope_review_rejected() {
        let state = test_state();
        let app = routes(state);

        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-invalid-scope-review-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "unscoped",
            "secondary": {},
            "is_shared": false,
            "lifecycle": "active",
            "scope_review": {
                "deferred_reason": "Need product input",
                "owner": "",
                "due_at": "2026-03-31"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
        assert!(err.error.contains("scope_review.owner"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_valid_scope_review_persisted() {
        let state = test_state();
        let app = routes(state);

        let mut valid = sample_decision_json();
        valid["id"] = serde_json::json!("dec-valid-scope-review-a1b2c3d4");
        valid["scope"] = serde_json::json!({
            "scope_class": "unscoped",
            "secondary": {},
            "is_shared": false,
            "lifecycle": "active",
            "scope_review": {
                "deferred_reason": "Need product input",
                "owner": "alice",
                "due_at": "2026-03-31"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&valid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let node: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            node["scope"]["scope_review"]["deferred_reason"],
            "Need product input"
        );
        assert_eq!(node["scope"]["scope_review"]["owner"], "alice");
        assert_eq!(node["scope"]["scope_review"]["due_at"], "2026-03-31");
        assert!(node["scope"]["scope_review"]["deferred_at"]
            .as_str()
            .is_some());
    }

    #[tokio::test]
    async fn test_create_blueprint_node_invalid_override_scope_rejected() {
        let state = test_state();
        let app = routes(state);

        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-invalid-override-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "global",
            "secondary": {},
            "is_shared": false,
            "override_scope": {
                "shared_source_id": "shared-guidance"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
        assert!(err.error.contains("override_scope"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_override_source_must_exist() {
        let state = test_state();
        let app = routes(state);

        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-missing-override-source-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": false,
            "shared": null,
            "lifecycle": "active",
            "override_scope": {
                "shared_source_id": "dec-shared-missing"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
        assert!(err.error.contains("not found"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_override_source_must_reference_shared_node() {
        let state = test_state();
        let mut local_source = sample_decision_json();
        local_source["id"] = serde_json::json!("dec-local-source-a1b2c3d4");
        local_source["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": false,
            "shared": null,
            "lifecycle": "active"
        });
        let local_source_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(local_source).unwrap();
        state.blueprints.upsert_node(local_source_node);

        let app = routes(state);
        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-invalid-override-target-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": false,
            "shared": null,
            "lifecycle": "active",
            "override_scope": {
                "shared_source_id": "dec-local-source-a1b2c3d4"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
        assert!(err.error.contains("shared record"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_override_source_cannot_self_reference() {
        let state = test_state();
        let app = routes(state);

        let mut invalid = sample_decision_json();
        invalid["id"] = serde_json::json!("dec-self-override-a1b2c3d4");
        invalid["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": false,
            "shared": null,
            "lifecycle": "active",
            "override_scope": {
                "shared_source_id": "dec-self-override-a1b2c3d4"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&invalid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(err.code.as_deref(), Some("INVALID_SCOPE"));
        assert!(err.error.contains("cannot reference the node itself"));
    }

    #[tokio::test]
    async fn test_create_blueprint_node_valid_override_source_accepted() {
        let state = test_state();
        let mut shared_source = sample_decision_json();
        shared_source["id"] = serde_json::json!("dec-shared-source-a1b2c3d4");
        shared_source["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": true,
            "shared": {
                "linked_project_ids": ["proj-alpha"],
                "inherit_to_linked_projects": true
            },
            "lifecycle": "active"
        });
        let shared_source_node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(shared_source).unwrap();
        state.blueprints.upsert_node(shared_source_node);

        let app = routes(state);
        let mut valid = sample_decision_json();
        valid["id"] = serde_json::json!("dec-valid-override-a1b2c3d4");
        valid["scope"] = serde_json::json!({
            "scope_class": "project",
            "project": {
                "project_id": "proj-alpha",
                "project_name": "Alpha Project"
            },
            "secondary": {},
            "is_shared": false,
            "shared": null,
            "lifecycle": "active",
            "override_scope": {
                "shared_source_id": "dec-shared-source-a1b2c3d4",
                "override_reason": "Project-specific deviation"
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&valid).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let node: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            node["scope"]["override_scope"]["shared_source_id"],
            "dec-shared-source-a1b2c3d4"
        );
    }

    #[tokio::test]
    async fn test_update_blueprint_node() {
        let state = test_state();

        // Insert original.
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        // Update via PATCH (full replacement).
        let mut updated_json = sample_decision_json();
        updated_json["title"] = serde_json::json!("Use MessagePack v2");

        let app = routes(state);

        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&updated_json).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let returned: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(returned["title"], "Use MessagePack v2");
    }

    #[tokio::test]
    async fn test_update_blueprint_node_not_found() {
        let state = test_state();
        let app = routes(state);

        let json_body = serde_json::to_string(&sample_decision_json()).unwrap();
        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/nonexistent")
            .header("content-type", "application/json")
            .body(Body::from(json_body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_component_name_marks_manual_naming_source() {
        let state = test_state();
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_component_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state.clone());
        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/comp-auth-a1b2c3d4")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Identity Service"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let updated = state
            .blueprints
            .get_node("comp-auth-a1b2c3d4")
            .expect("component should exist");

        match updated {
            planner_schemas::artifacts::blueprint::BlueprintNode::Component(component) => {
                assert_eq!(component.name, "Identity Service");
                let naming = component.naming.expect("naming metadata should exist");
                assert_eq!(
                    naming.source,
                    planner_schemas::artifacts::blueprint::ComponentNameSource::Manual
                );
                assert_eq!(naming.origin_key, "spec:proj:root:auth");
                assert_eq!(naming.generated_name, "Authentication Service");
            }
            other => panic!("expected component node, got {:?}", other.type_name()),
        }
    }

    #[tokio::test]
    async fn test_partial_patch_merges_tags() {
        let state = test_state();
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state.clone());
        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"tags":["new-tag"]}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let updated = state
            .blueprints
            .get_node("dec-use-msgpack-a1b2c3d4")
            .unwrap();
        assert_eq!(updated.tags(), &["new-tag"]);
        assert_eq!(updated.name(), "Use MessagePack for disk serialization");
    }

    #[tokio::test]
    async fn test_partial_patch_invalid_field_returns_400() {
        let state = test_state();
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state);
        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"status":"bogus"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_node_with_documentation() {
        let state = test_state();
        let app = routes(state.clone());

        let mut node = sample_decision_json();
        node["documentation"] = serde_json::json!("# Decision Notes\n\nDocumented");

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&node).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let stored = state
            .blueprints
            .get_node("dec-use-msgpack-a1b2c3d4")
            .unwrap();
        assert_eq!(
            stored.documentation(),
            Some("# Decision Notes\n\nDocumented")
        );

        let summaries = state.blueprints.list_summaries();
        assert!(summaries
            .iter()
            .any(|summary| summary.id.as_str() == "dec-use-msgpack-a1b2c3d4"
                && summary.has_documentation));
    }

    #[tokio::test]
    async fn test_patch_documentation_only() {
        let state = test_state();
        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state.clone());
        let req = Request::builder()
            .method("PATCH")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "documentation": "## Updated docs" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let stored = state
            .blueprints
            .get_node("dec-use-msgpack-a1b2c3d4")
            .unwrap();
        assert_eq!(stored.documentation(), Some("## Updated docs"));
    }

    #[tokio::test]
    async fn test_delete_blueprint_node() {
        let state = test_state();

        let node: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(node);

        let app = routes(state.clone());

        let req = Request::builder()
            .method("DELETE")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify it's actually gone.
        assert!(state
            .blueprints
            .get_node("dec-use-msgpack-a1b2c3d4")
            .is_none());
    }

    #[tokio::test]
    async fn test_delete_blueprint_node_not_found() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/blueprint/nodes/nonexistent")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_blueprint_edge() {
        let state = test_state();

        // Insert two nodes first.
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        let app = routes(state);

        let edge_json = serde_json::json!({
            "source": "tech-rust-b2c3d4e5",
            "target": "dec-use-msgpack-a1b2c3d4",
            "edge_type": "decided_by"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/edges")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&edge_json).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_create_blueprint_edge_source_missing() {
        let state = test_state();

        // Only insert target.
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        state.blueprints.upsert_node(dec);

        let app = routes(state);

        let edge_json = serde_json::json!({
            "source": "nonexistent-source",
            "target": "dec-use-msgpack-a1b2c3d4",
            "edge_type": "depends_on"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/edges")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&edge_json).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_impact_preview() {
        let state = test_state();

        // Build a small graph: dec -> tech (via affects edge).
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        state
            .blueprints
            .add_edge(planner_schemas::artifacts::blueprint::Edge {
                source: planner_schemas::artifacts::blueprint::NodeId::from_raw(
                    "dec-use-msgpack-a1b2c3d4",
                ),
                target: planner_schemas::artifacts::blueprint::NodeId::from_raw(
                    "tech-rust-b2c3d4e5",
                ),
                edge_type: planner_schemas::artifacts::blueprint::EdgeType::Affects,
                metadata: None,
            });

        let app = routes(state);

        let impact_req = serde_json::json!({
            "node_id": "dec-use-msgpack-a1b2c3d4",
            "change_description": "Switch to CBOR instead of MessagePack"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/impact-preview")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&impact_req).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(report["source_node_id"], "dec-use-msgpack-a1b2c3d4");
        // tech-rust should be affected.
        let entries = report["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["node_id"], "tech-rust-b2c3d4e5");
    }

    #[tokio::test]
    async fn test_impact_preview_node_not_found() {
        let state = test_state();
        let app = routes(state);

        let impact_req = serde_json::json!({
            "node_id": "nonexistent",
            "change_description": "Some change"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/impact-preview")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&impact_req).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_blueprint_full_lifecycle() {
        // E2E lifecycle: create nodes -> list -> create edges -> impact preview -> delete -> verify.
        let state = test_state();

        // 1. Create Decision node.
        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&sample_decision_json()).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // 2. Create Technology node.
        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&sample_technology_json()).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // 3. Verify full blueprint shows 2 nodes.
        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let bp: BlueprintResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(bp.total_nodes, 2);

        // 4. Create edge: decision affects technology.
        let app = routes(state.clone());
        let edge = serde_json::json!({
            "source": "dec-use-msgpack-a1b2c3d4",
            "target": "tech-rust-b2c3d4e5",
            "edge_type": "affects"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/edges")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&edge).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // 5. Impact preview — decision change affects technology.
        let app = routes(state.clone());
        let impact = serde_json::json!({
            "node_id": "dec-use-msgpack-a1b2c3d4",
            "change_description": "Switch serialization format"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/impact-preview")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&impact).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!report["entries"].as_array().unwrap().is_empty());

        // 6. Delete the decision node (should also remove the edge).
        let app = routes(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri("/blueprint/nodes/dec-use-msgpack-a1b2c3d4")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // 7. Verify: blueprint now has 1 node, 0 edges.
        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let bp: BlueprintResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(bp.total_nodes, 1);
        assert_eq!(bp.total_edges, 0);
    }

    #[tokio::test]
    async fn test_delete_blueprint_edge() {
        let state = test_state();

        // Insert two nodes and create an edge.
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        let edge = planner_schemas::artifacts::blueprint::Edge {
            source: planner_schemas::artifacts::blueprint::NodeId::from_raw("tech-rust-b2c3d4e5"),
            target: planner_schemas::artifacts::blueprint::NodeId::from_raw(
                "dec-use-msgpack-a1b2c3d4",
            ),
            edge_type: planner_schemas::artifacts::blueprint::EdgeType::DecidedBy,
            metadata: None,
        };
        state.blueprints.add_edge(edge);

        // Verify edge exists.
        let (_, edge_count) = state.blueprints.counts();
        assert_eq!(edge_count, 1);

        // Delete the edge.
        let app = routes(state.clone());
        let payload = serde_json::json!({
            "source": "tech-rust-b2c3d4e5",
            "target": "dec-use-msgpack-a1b2c3d4",
            "edge_type": "decided_by"
        });
        let req = Request::builder()
            .method("DELETE")
            .uri("/blueprint/edges")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify edge removed.
        let (_, edge_count) = state.blueprints.counts();
        assert_eq!(edge_count, 0);
    }

    #[tokio::test]
    async fn test_delete_blueprint_edge_not_found() {
        let state = test_state();
        let app = routes(state);

        let payload = serde_json::json!({
            "source": "nonexistent-a",
            "target": "nonexistent-b",
            "edge_type": "depends_on"
        });
        let req = Request::builder()
            .method("DELETE")
            .uri("/blueprint/edges")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_blueprint_history_empty() {
        // In-memory store has no disk — should return empty list.
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/blueprint/history")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let history: HistoryListResponse = serde_json::from_slice(&body).unwrap();
        assert!(history.snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_list_blueprint_events_empty() {
        let state = test_state();
        let app = routes(state);

        let req = Request::builder()
            .uri("/blueprint/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: BlueprintEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(events.total, 0);
        assert!(events.events.is_empty());
    }

    #[tokio::test]
    async fn test_blueprint_events_after_crud() {
        let state = test_state();

        // Create a node — should produce a NodeCreated event.
        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/nodes")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&sample_decision_json()).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // Check events.
        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: BlueprintEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(events.total, 1);
        assert_eq!(events.events[0].event_type, "node_created");
    }

    #[tokio::test]
    async fn test_record_blueprint_export_event() {
        let state = test_state();
        let app = routes(state.clone());

        let payload = serde_json::json!({
            "kind": "scoped_view",
            "node_count": 3,
            "edge_count": 1,
            "project_id": "proj-alpha",
            "project_name": "Alpha Project",
            "scope_snapshot": {
                "filters": {
                    "feature": "task-tracker",
                    "component": "task-widget"
                }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/exports")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: BlueprintEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(events.total, 1);
        assert_eq!(events.events[0].event_type, "export_recorded");
        assert_eq!(events.events[0].data["project_id"], "proj-alpha");
        assert_eq!(events.events[0].data["node_count"], 3);
    }

    #[tokio::test]
    async fn test_blueprint_export_history_filtered_by_project_and_scope() {
        let state = test_state();
        state.blueprints.record_export_event(
            "exp-alpha".into(),
            planner_schemas::artifacts::blueprint::BlueprintExportKind::ScopedView,
            Some("alice".into()),
            None,
            3,
            1,
            Some("proj-alpha".into()),
            Some("Alpha Project".into()),
            Some(serde_json::json!({
                "filters": {
                    "scopeClass": "project_contextual",
                    "feature": "task-tracker",
                    "component": "task-widget"
                }
            })),
        );
        state.blueprints.record_export_event(
            "exp-beta".into(),
            planner_schemas::artifacts::blueprint::BlueprintExportKind::ScopedView,
            Some("bob".into()),
            None,
            2,
            0,
            Some("proj-beta".into()),
            Some("Beta Project".into()),
            Some(serde_json::json!({
                "filters": {
                    "scopeClass": "project",
                    "feature": "billing",
                    "component": "ledger-widget"
                }
            })),
        );

        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/export-history?project_id=proj-alpha&scope_class=project_contextual&component=task-widget")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let history: BlueprintExportHistoryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].export_id, "exp-alpha");
        assert_eq!(history.entries[0].project_id.as_deref(), Some("proj-alpha"));
        assert_eq!(
            history.entries[0]
                .scope_snapshot
                .as_ref()
                .and_then(|value| value.get("filters"))
                .and_then(|value| value.get("component"))
                .and_then(serde_json::Value::as_str),
            Some("task-widget")
        );
    }

    #[tokio::test]
    async fn test_blueprint_export_history_redacts_sensitive_scope_fields_and_exposes_retention() {
        let state = test_state();
        state.blueprints.record_export_event(
            "exp-governed".into(),
            planner_schemas::artifacts::blueprint::BlueprintExportKind::SingleRecord,
            Some("auth0|auditor".into()),
            Some("dec-use-msgpack-a1b2c3d4".into()),
            1,
            0,
            Some("proj-alpha".into()),
            Some("Alpha Project".into()),
            Some(serde_json::json!({
                "filters": {
                    "scopeClass": "project_contextual",
                    "feature": "task-tracker",
                    "component": "task-widget",
                    "owner": "alice",
                    "tag": "sensitive"
                },
                "section": "activity",
                "selected_node_id": "dec-use-msgpack-a1b2c3d4"
            })),
        );

        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/export-history?project_id=proj-alpha")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let history: BlueprintExportHistoryResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.entries.len(), 1);
        let entry = &history.entries[0];
        assert_eq!(entry.actor.as_deref(), Some("auth0|auditor"));
        assert_eq!(
            entry
                .scope_snapshot
                .as_ref()
                .and_then(|value| value.get("filters"))
                .and_then(|value| value.get("component"))
                .and_then(serde_json::Value::as_str),
            Some("task-widget")
        );
        assert!(entry
            .scope_snapshot
            .as_ref()
            .and_then(|value| value.get("filters"))
            .and_then(|value| value.get("owner"))
            .is_none());
        assert!(entry
            .scope_snapshot
            .as_ref()
            .and_then(|value| value.get("selected_node_id"))
            .is_none());
        assert!(entry.scope_snapshot_redacted);
        assert!(entry
            .scope_snapshot_redacted_fields
            .contains(&"filters.owner".to_string()));
        assert!(entry
            .scope_snapshot_redacted_fields
            .contains(&"filters.tag".to_string()));
        assert!(entry
            .scope_snapshot_redacted_fields
            .contains(&"selected_node_id".to_string()));

        let timestamp =
            chrono::DateTime::parse_from_rfc3339(&entry.timestamp).expect("entry timestamp");
        let retention = chrono::DateTime::parse_from_rfc3339(
            entry
                .retention_expires_at
                .as_deref()
                .expect("retention metadata"),
        )
        .expect("retention timestamp");
        assert_eq!(
            retention - timestamp,
            chrono::Duration::days(EXPORT_AUDIT_RETENTION_DAYS)
        );
    }

    #[tokio::test]
    async fn test_blueprint_events_filtered_by_node() {
        let state = test_state();

        // Create two nodes.
        let dec: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_decision_json()).unwrap();
        let tech: planner_schemas::artifacts::blueprint::BlueprintNode =
            serde_json::from_value(sample_technology_json()).unwrap();
        state.blueprints.upsert_node(dec);
        state.blueprints.upsert_node(tech);

        // Should have 2 events total.
        assert_eq!(state.blueprints.event_count(), 2);

        // Filter to decision node only.
        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/events?node_id=dec-use-msgpack-a1b2c3d4")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: BlueprintEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(events.total, 1);
        assert_eq!(events.events[0].event_type, "node_created");
    }

    #[tokio::test]
    async fn test_list_blueprint_events_normalizes_legacy_scope_tags_in_payloads() {
        use planner_schemas::artifacts::blueprint::{
            BlueprintNode, NodeLifecycle, NodeScope, ProjectScope, ScopeClass, SecondaryScopeRefs,
        };

        let state = test_state();

        let mut legacy: BlueprintNode = serde_json::from_value(sample_decision_json()).unwrap();
        if let BlueprintNode::Decision(decision) = &mut legacy {
            decision.tags = vec![
                "storage".into(),
                "archived".into(),
                "overrides:shared-guidance".into(),
            ];
            decision.scope = NodeScope {
                scope_class: ScopeClass::Project,
                project: Some(ProjectScope {
                    project_id: "proj-alpha".into(),
                    project_name: Some("Alpha Project".into()),
                }),
                secondary: SecondaryScopeRefs::default(),
                is_shared: false,
                shared: None,
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
                scope_review: None,
            };
        }
        state.blueprints.upsert_node(legacy);

        let app = routes(state.clone());
        let req = Request::builder()
            .uri("/blueprint/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: BlueprintEventsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(events.total, 1);
        assert_eq!(events.events[0].event_type, "node_created");
        assert_eq!(
            events.events[0].data["node"]["scope"]["lifecycle"],
            "archived"
        );
        assert_eq!(
            events.events[0].data["node"]["scope"]["override_scope"]["shared_source_id"],
            "shared-guidance"
        );
        assert_eq!(
            events.events[0].data["node"]["tags"],
            serde_json::json!(["storage"])
        );
    }

    #[tokio::test]
    async fn test_discovery_scan_endpoint() {
        let state = test_state();
        let scan_root = temp_scan_root("scan");
        std::fs::write(
            scan_root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .unwrap();

        let app = routes(state.clone());
        let payload = serde_json::json!({
            "scanners": ["cargo_toml"],
            "root_path": scan_root.to_string_lossy(),
        });
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/discovery/scan")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: DiscoveryRunResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.total_proposed, 1);
        assert_eq!(state.proposals.list(None).len(), 1);

        let _ = std::fs::remove_dir_all(scan_root);
    }

    #[tokio::test]
    async fn test_accept_proposal_creates_node() {
        let state = test_state();
        let proposal = planner_core::discovery::ProposedNode {
            id: "proposal-1".into(),
            node: serde_json::from_value(sample_technology_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::CargoToml,
            reason: "Dependency found".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.9,
            source_artifact: Some("Cargo.toml".into()),
            review_note: None,
        };
        state.proposals.insert_many(vec![proposal]).unwrap();

        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/discovery/proposals/proposal-1/accept")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.blueprints.get_node("tech-rust-b2c3d4e5").is_some());
        assert_eq!(
            state.proposals.get("proposal-1").unwrap().status,
            planner_core::discovery::ProposalStatus::Merged
        );
    }

    #[tokio::test]
    async fn test_accept_component_proposal_with_manual_name_override() {
        let state = test_state();
        let proposal = planner_core::discovery::ProposedNode {
            id: "proposal-component-1".into(),
            node: serde_json::from_value(sample_component_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::DirectoryScan,
            reason: "Component inferred from project tree".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.85,
            source_artifact: Some("src/auth".into()),
            review_note: None,
        };
        state.proposals.insert_many(vec![proposal]).unwrap();

        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/discovery/proposals/proposal-component-1/accept")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"node_patch":{"name":"Identity Service"}}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let node = state
            .blueprints
            .get_node("comp-auth-a1b2c3d4")
            .expect("accepted component should exist");

        match node {
            planner_schemas::artifacts::blueprint::BlueprintNode::Component(component) => {
                assert_eq!(component.name, "Identity Service");
                let naming = component.naming.expect("naming metadata should exist");
                assert_eq!(
                    naming.source,
                    planner_schemas::artifacts::blueprint::ComponentNameSource::Manual
                );
                assert_eq!(naming.origin_key, "spec:proj:root:auth");
                assert_eq!(naming.generated_name, "Authentication Service");
            }
            other => panic!("expected component node, got {:?}", other.type_name()),
        }

        assert_eq!(
            state
                .proposals
                .get("proposal-component-1")
                .expect("proposal should exist")
                .status,
            planner_core::discovery::ProposalStatus::Merged
        );
    }

    #[tokio::test]
    async fn test_reject_proposal() {
        let state = test_state();
        let proposal = planner_core::discovery::ProposedNode {
            id: "proposal-2".into(),
            node: serde_json::from_value(sample_technology_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::CargoToml,
            reason: "Dependency found".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.9,
            source_artifact: Some("Cargo.toml".into()),
            review_note: None,
        };
        state.proposals.insert_many(vec![proposal]).unwrap();

        let app = routes(state.clone());
        let req = Request::builder()
            .method("POST")
            .uri("/blueprint/discovery/proposals/proposal-2/reject")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"reason":"duplicate"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            state.proposals.get("proposal-2").unwrap().status,
            planner_core::discovery::ProposalStatus::Rejected
        );
    }

    #[tokio::test]
    async fn test_list_proposals_filter_by_status() {
        let state = test_state();
        let pending = planner_core::discovery::ProposedNode {
            id: "proposal-pending".into(),
            node: serde_json::from_value(sample_technology_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::CargoToml,
            reason: "Pending".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.9,
            source_artifact: Some("Cargo.toml".into()),
            review_note: None,
        };
        let rejected = planner_core::discovery::ProposedNode {
            id: "proposal-rejected".into(),
            node: serde_json::from_value(sample_technology_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::CargoToml,
            reason: "Rejected".into(),
            status: planner_core::discovery::ProposalStatus::Rejected,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: Some("2026-03-06T01:00:00Z".into()),
            confidence: 0.9,
            source_artifact: Some("workspace/Cargo.toml".into()),
            review_note: Some("duplicate".into()),
        };
        state
            .proposals
            .insert_many(vec![pending, rejected])
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri("/blueprint/discovery/proposals?status=rejected")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ProposedNodesResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.proposals[0].id, "proposal-rejected");
    }

    #[tokio::test]
    async fn test_list_component_proposals_alias_filter_by_status() {
        let state = test_state();
        let pending = planner_core::discovery::ProposedNode {
            id: "proposal-component-pending".into(),
            node: serde_json::from_value(sample_component_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::DirectoryScan,
            reason: "Pending".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.85,
            source_artifact: Some("src/auth".into()),
            review_note: None,
        };
        let rejected = planner_core::discovery::ProposedNode {
            id: "proposal-component-rejected".into(),
            node: serde_json::from_value(sample_component_json()).unwrap(),
            source: planner_core::discovery::DiscoverySource::DirectoryScan,
            reason: "Rejected".into(),
            status: planner_core::discovery::ProposalStatus::Rejected,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: Some("2026-03-06T01:00:00Z".into()),
            confidence: 0.85,
            source_artifact: Some("src/review".into()),
            review_note: Some("duplicate".into()),
        };
        state
            .proposals
            .insert_many(vec![pending, rejected])
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri("/blueprint/discovery/component-proposals?status=pending")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ProposedNodesResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.proposals[0].id, "proposal-component-pending");
    }

    #[tokio::test]
    async fn test_list_edge_proposals_filter_by_status() {
        let state = test_state();
        let pending = planner_core::discovery::ProposedEdge {
            id: "edge-proposal-pending".into(),
            edge: planner_schemas::artifacts::blueprint::Edge {
                source: planner_schemas::artifacts::blueprint::NodeId::from_raw("proj-root"),
                target: planner_schemas::artifacts::blueprint::NodeId::from_raw("comp-auth"),
                edge_type: planner_schemas::artifacts::blueprint::EdgeType::Contains,
                metadata: Some("directory".into()),
            },
            source: planner_core::discovery::DiscoverySource::CodeGraphContext,
            reason: "Pending".into(),
            status: planner_core::discovery::ProposalStatus::Pending,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: None,
            confidence: 0.9,
            source_artifact: Some("src/auth".into()),
            review_note: None,
        };
        let merged = planner_core::discovery::ProposedEdge {
            id: "edge-proposal-merged".into(),
            edge: planner_schemas::artifacts::blueprint::Edge {
                source: planner_schemas::artifacts::blueprint::NodeId::from_raw("proj-root"),
                target: planner_schemas::artifacts::blueprint::NodeId::from_raw("comp-review"),
                edge_type: planner_schemas::artifacts::blueprint::EdgeType::Contains,
                metadata: Some("cgc:indexed-package".into()),
            },
            source: planner_core::discovery::DiscoverySource::CodeGraphContext,
            reason: "Merged".into(),
            status: planner_core::discovery::ProposalStatus::Merged,
            proposed_at: "2026-03-06T00:00:00Z".into(),
            reviewed_at: Some("2026-03-06T01:00:00Z".into()),
            confidence: 0.95,
            source_artifact: Some("planner-core".into()),
            review_note: None,
        };
        state
            .proposals
            .insert_many_edges(vec![pending, merged])
            .unwrap();

        let app = routes(state);
        let req = Request::builder()
            .uri("/blueprint/discovery/edge-proposals?status=merged")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let response: ProposedEdgesResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.total, 1);
        assert_eq!(response.proposals[0].id, "edge-proposal-merged");
    }
}
