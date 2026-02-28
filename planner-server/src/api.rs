//! # API Routes — REST Endpoints for Planner Server
//!
//! Provides REST API for the Socratic Lobby web frontend.

use std::sync::Arc;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::auth::{auth_middleware, Claims};
use crate::session::Session;
use crate::ws;

// ---------------------------------------------------------------------------
// Request/Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub sessions_active: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session: Session,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<Session>,
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
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

pub fn routes(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/health", get(health))
        .with_state(state.clone());

    let protected = Router::new()
        .route("/models", get(models))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/message", post(send_message))
        .route("/sessions/{id}/ws", get(ws_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    public.merge(protected)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: "0.1.0".into(),
        sessions_active: state.sessions.count(),
    })
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
            id: "gemini-3.1-pro".into(),
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
) -> Json<ListSessionsResponse> {
    let sessions = state.sessions.list_for_user(&claims.sub);
    Json(ListSessionsResponse { sessions })
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session = state.sessions.create(&claims.sub);
    tracing::info!("Created session: {} for user: {}", session.id, claims.sub);

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse { session }),
    )
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<Uuid>,
) -> Result<Json<Session>, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.get(id) {
        Some(session) => {
            if session.user_id != claims.sub {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Access denied".into(),
                    }),
                ));
            }
            Ok(Json(session))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
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
    // Verify ownership before proceeding
    match state.sessions.get(id) {
        Some(session) => {
            if session.user_id != claims.sub {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Access denied".into(),
                    }),
                ));
            }
        }
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                }),
            ));
        }
    }

    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Message content cannot be empty".into(),
            }),
        ));
    }

    // Add user message and generate the initial planner acknowledgement.
    // The actual pipeline runs in a background task — clients poll
    // GET /api/sessions/:id or connect to the WebSocket for live updates.
    let result = state.sessions.update(id, |session| {
        session.add_message("user", &content);

        if !session.pipeline_running {
            session.pipeline_running = true;
            session.project_description = Some(content.clone());
            session.stages[0].status = "running".into();

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
            // Spawn the pipeline task only on the first message
            // (pipeline_running was false before the update above set it true,
            //  so we check project_description being freshly set).
            if session.pipeline_running && session.project_description.as_deref() == Some(&content) {
                let state_clone = state.clone();
                let session_id = id;
                let description = content.clone();

                tokio::spawn(async move {
                    run_pipeline_for_session(state_clone, session_id, description).await;
                });
            }

            let msgs = &session.messages;
            let user_msg = msgs[msgs.len() - 2].clone();
            let planner_msg = msgs[msgs.len() - 1].clone();

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
            }),
        )),
    }
}

// ---------------------------------------------------------------------------
// Pipeline background task
// ---------------------------------------------------------------------------

/// Background task: runs the full pipeline and writes results back to the
/// session store. Clients observe progress via REST polling or WebSocket.
async fn run_pipeline_for_session(state: Arc<AppState>, session_id: Uuid, description: String) {
    tracing::info!("Session {}: pipeline task started", session_id);

    let router = planner_core::llm::providers::LlmRouter::from_env();

    let worker =
        match planner_core::pipeline::steps::factory_worker::CodexFactoryWorker::new() {
            Ok(w) => w,
            Err(e) => {
                state.sessions.update(session_id, |s| {
                    s.add_message(
                        "planner",
                        &format!("Pipeline setup failed: {}", e),
                    );
                    s.pipeline_running = false;
                });
                return;
            }
        };

    let config =
        planner_core::pipeline::PipelineConfig::<planner_core::cxdb::CxdbEngine>::minimal(
            &router,
        );
    let project_id = Uuid::new_v4();

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
            });
            tracing::info!("Session {}: pipeline complete", session_id);
        }
        Err(e) => {
            state.sessions.update(session_id, |s| {
                s.add_message("planner", &format!("Pipeline failed: {}", e));
                // Mark the first running stage as failed
                for stage in &mut s.stages {
                    if stage.status == "running" {
                        stage.status = "failed".into();
                        break;
                    }
                }
                s.pipeline_running = false;
            });
            tracing::warn!("Session {}: pipeline failed: {}", session_id, e);
        }
    }
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
    // Verify the session exists and belongs to the user
    match state.sessions.get(id) {
        Some(session) if session.user_id == claims.sub => {
            ws.on_upgrade(move |socket| ws::handle_ws(socket, state, id))
        }
        Some(_) => {
            // Session exists but belongs to a different user
            (StatusCode::FORBIDDEN, "Access denied").into_response()
        }
        None => {
            (StatusCode::NOT_FOUND, "Session not found").into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStore;
    use crate::auth::AuthConfig;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
            auth_config: None, // dev mode for tests
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "ok");
        assert_eq!(health.sessions_active, 0);
    }

    #[tokio::test]
    async fn test_health_no_auth_required() {
        // Health endpoint must work with no token even when auth is configured
        let state = Arc::new(AppState {
            sessions: SessionStore::new(),
            auth_config: Some(AuthConfig {
                domain: "test.auth0.com".into(),
                audience: "test".into(),
                decoding_key: None,
            }),
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let listed: ListSessionsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.sessions.len(), 2);
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let retrieved: Session = serde_json::from_slice(&body).unwrap();
        assert_eq!(retrieved.id, id);
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

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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
            auth_config: Some(AuthConfig {
                domain: "test.auth0.com".into(),
                audience: "test".into(),
                decoding_key: None,
            }),
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
}
