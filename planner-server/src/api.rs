//! # API Routes — REST Endpoints for Planner Server
//!
//! Provides REST API for the Socratic Lobby web frontend.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{auth_middleware, Claims};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<crate::session::SessionSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetSessionResponse {
    pub session: Session,
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

/// API response for the event log.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintEventsResponse {
    pub events: Vec<BlueprintEventPayload>,
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
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryRunResponse {
    pub results: Vec<DiscoveryScanResult>,
    pub total_proposed: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposedNodesResponse {
    pub proposals: Vec<planner_core::discovery::ProposedNode>,
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
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session).patch(update_session))
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
        .route("/blueprint/exports", post(record_blueprint_export))
        .route("/blueprint/impact-preview", post(impact_preview))
        .route("/blueprint/reconverge", post(reconverge_blueprint))
        .route("/blueprint/reconverge/ws", get(reconverge_ws_handler))
        .route("/blueprint/discovery/scan", post(run_discovery_scan))
        .route("/blueprint/discovery/proposals", get(list_proposals))
        .route(
            "/blueprint/discovery/proposals/{id}/accept",
            post(accept_proposal),
        )
        .route(
            "/blueprint/discovery/proposals/{id}/reject",
            post(reject_proposal),
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
        .snapshot_all_events()
        .into_iter()
        .flat_map(|(_, events)| events)
        .filter(|e| {
            if let Some(ref lvl) = filter_level {
                if &e.level != lvl {
                    return false;
                }
            }
            if let Some(ref sid) = filter_session_id {
                match e.session_id {
                    Some(ref esid) => {
                        if esid != sid {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        })
        .map(|e| AdminEventEntry {
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
            session_id: e.session_id.map(|id| id.to_string()),
            step: e.step,
            message: e.message,
            duration_ms: e.duration_ms,
            metadata: e.metadata,
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
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session = state.sessions.create(&claims.sub);
    tracing::info!("Created session: {} for user: {}", session.id, claims.sub);

    (StatusCode::CREATED, Json(CreateSessionResponse { session }))
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

fn stop_session_runtime(state: &Arc<AppState>, session_id: Uuid) {
    if let Some(runtime) = state.socratic_runtimes.remove(session_id) {
        runtime.close_input();
        runtime.signal_closed();
    }
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

    stop_session_runtime(&state, id);

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

    stop_session_runtime(&state, id);

    let description = session.project_description.clone().unwrap_or_default();
    let session = state.sessions.update(id, |s| {
        s.prepare_for_pipeline_retry();
        s.add_message("planner", "Retrying pipeline from the saved description.");
    });

    match session {
        Some(session) => {
            let state_clone = state.clone();
            tokio::spawn(async move {
                run_pipeline_for_session(state_clone, id, description).await;
            });
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
        Some(session) => {
            // Touch to extend expiry after a real user interaction.
            state.sessions.touch(id);

            // Spawn pipeline only if this request transitioned it to running.
            if should_spawn_pipeline {
                let state_clone = state.clone();
                let session_id = id;
                let description = content.clone();

                tokio::spawn(async move {
                    run_pipeline_for_session(state_clone, session_id, description).await;
                });
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
            state.sessions.update(session_id, |s| {
                s.add_message("planner", &format!("Pipeline setup failed: {}", e));
                if let Some(stage) = s.stages.iter_mut().find(|stage| stage.status == "running") {
                    stage.status = "failed".into();
                }
                s.pipeline_running = false;
                s.intake_phase = "error".into();
                s.error_message = Some(format!("Pipeline setup failed: {}", e));
            });
            return;
        }
    };

    let project_id = Uuid::new_v4();

    // Store the project_id in the session so list_turns/list_runs can query it.
    state.sessions.update(session_id, |s| {
        s.cxdb_project_id = Some(project_id);
    });

    // Build PipelineConfig with durable storage if available.
    // We branch on whether CXDB is available to avoid holding a borrow
    // across the async pipeline call.
    let cxdb_ref = state.cxdb.as_ref();

    match cxdb_ref {
        Some(engine) => {
            // Register this run in CXDB.
            let run_id = Uuid::new_v4();
            if let Err(e) = engine.register_run(project_id, run_id) {
                tracing::warn!("CXDB: failed to register run: {}", e);
            }

            let config = planner_core::pipeline::PipelineConfig {
                router: router.as_ref(),
                store: Some(engine),
                dtu_registry: None,
                blueprints: Some(&state.blueprints),
            };

            match planner_core::pipeline::run_full_pipeline(
                &config,
                &worker,
                project_id,
                &description,
            )
            .await
            {
                Ok(output) => {
                    state.sessions.update(session_id, |s| {
                        for stage in &mut s.stages {
                            stage.status = "complete".into();
                        }
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
                Err(e) => {
                    state.sessions.update(session_id, |s| {
                        s.add_message("planner", &format!("Pipeline failed: {}", e));
                        for stage in &mut s.stages {
                            if stage.status == "running" {
                                stage.status = "failed".into();
                                break;
                            }
                        }
                        s.pipeline_running = false;
                        s.intake_phase = "error".into();
                        s.error_message = Some(format!("Pipeline failed: {}", e));
                    });
                    tracing::warn!("Session {}: pipeline failed: {}", session_id, e);
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
            };

            match planner_core::pipeline::run_full_pipeline(
                &config,
                &worker,
                project_id,
                &description,
            )
            .await
            {
                Ok(output) => {
                    state.sessions.update(session_id, |s| {
                        for stage in &mut s.stages {
                            stage.status = "complete".into();
                        }
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
                Err(e) => {
                    state.sessions.update(session_id, |s| {
                        s.add_message("planner", &format!("Pipeline failed: {}", e));
                        for stage in &mut s.stages {
                            if stage.status == "running" {
                                stage.status = "failed".into();
                                break;
                            }
                        }
                        s.pipeline_running = false;
                        s.intake_phase = "error".into();
                        s.error_message = Some(format!("Pipeline failed: {}", e));
                    });
                    tracing::warn!("Session {}: pipeline failed: {}", session_id, e);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CXDB Read API handlers (Change 4)
// ---------------------------------------------------------------------------

/// List all Turns for a session (metadata only).
///
/// Queries the durable CXDB engine using the session's stored project_id.
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

    // Query CXDB for turns belonging to this session's project.
    let turns = match (&state.cxdb, session.cxdb_project_id) {
        (Some(engine), Some(project_id)) => engine
            .list_turn_metadata_for_project(project_id)
            .into_iter()
            .map(|m| TurnResponse {
                turn_id: m.turn_id,
                type_id: m.type_id,
                timestamp: m.timestamp,
                produced_by: m.produced_by,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    let count = turns.len();
    Ok(Json(ListTurnsResponse { turns, count }))
}

/// List all pipeline run IDs for a session.
///
/// Queries the durable CXDB engine using the session's stored project_id.
/// Returns an empty list if no CXDB is configured or no pipeline has run.
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

    // Query CXDB for runs belonging to this session's project.
    let runs = match (&state.cxdb, session.cxdb_project_id) {
        (Some(engine), Some(project_id)) => engine
            .list_runs(project_id)
            .into_iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

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

    // Store the initial description in the session for reference.
    if let Some(runtime) = state.socratic_runtimes.remove(id) {
        runtime.close_input();
        runtime.signal_closed();
    }
    state.sessions.update(id, |s| {
        s.project_description = Some(req.description.clone());
        s.ensure_title_from_description();
        s.set_archived(false);
        s.intake_phase = "interviewing".into();
        s.interview_live_attached = false;
        s.interview_runtime_active = false;
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
        BlueprintNode::Decision(n) => &mut n.scope,
        BlueprintNode::Technology(n) => &mut n.scope,
        BlueprintNode::Component(n) => &mut n.scope,
        BlueprintNode::Constraint(n) => &mut n.scope,
        BlueprintNode::Pattern(n) => &mut n.scope,
        BlueprintNode::QualityRequirement(n) => &mut n.scope,
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

    Ok(())
}

// ---------------------------------------------------------------------------
// Blueprint API handlers
// ---------------------------------------------------------------------------

/// GET /blueprint — Full blueprint graph summary.
async fn get_blueprint(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Query(query): Query<NodesQuery>,
) -> Json<BlueprintResponse> {
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
    Query(query): Query<NodesQuery>,
) -> Json<NodeListResponse> {
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
            BlueprintEventPayload {
                event_type: event_type.to_string(),
                summary: e.summary(),
                timestamp: e.timestamp().to_string(),
                data: serde_json::to_value(e).unwrap_or_default(),
            }
        })
        .collect();

    Json(BlueprintEventsResponse { events, total })
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
    let requested = if req.scanners.iter().any(|scanner| scanner == "all") {
        vec!["cargo_toml".to_string(), "directory_structure".to_string()]
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
        let scan_output = match scanner.as_str() {
            "cargo_toml" => {
                planner_core::discovery::scan_cargo_toml(&project_root, &state.blueprints)
            }
            "directory_structure" => {
                planner_core::discovery::scan_directory_structure(&project_root, &state.blueprints)
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
        };

        let (inserted, deduped) =
            state
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

        results.push(DiscoveryScanResult {
            scanner,
            proposed_count: inserted,
            skipped_count: scan_output.skipped_count + deduped,
            errors: scan_output.errors,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    let total_proposed = results.iter().map(|result| result.proposed_count).sum();
    Ok(Json(DiscoveryRunResponse {
        results,
        total_proposed,
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

async fn accept_proposal(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(proposal_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
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

    state.blueprints.upsert_node(proposal.node.clone());
    let _ = state.proposals.mark_merged(&proposal_id);

    Ok(Json(serde_json::json!({
        "node_id": proposal.node.id().0,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthConfig;
    use crate::session::SessionStore;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            blueprints: planner_core::blueprint::BlueprintStore::new(),
            proposals: planner_core::discovery::ProposalStore::new(),
            auth_config: None, // dev mode for tests
            event_store: None,
            cxdb: None, // no durable storage in unit tests
            llm_router: Arc::new(planner_core::llm::providers::LlmRouter::from_env()),
            socratic_runtimes: crate::runtime::SessionRuntimeRegistry::new(
                std::time::Duration::from_secs(30),
            ),
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
}
