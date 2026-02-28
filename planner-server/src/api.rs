//! # API Routes — REST Endpoints for Planner Server
//!
//! Provides REST API for the Socratic Lobby web frontend.

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::session::Session;

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
    Router::new()
        .route("/health", get(health))
        .route("/models", get(models))
        .route("/sessions", post(create_session))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/message", post(send_message))
        .with_state(state)
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

async fn create_session(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let session = state.sessions.create();
    tracing::info!("Created session: {}", session.id);

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse { session }),
    )
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Session>, (StatusCode, Json<ErrorResponse>)> {
    state
        .sessions
        .get(id)
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                }),
            )
        })
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Message content cannot be empty".into(),
            }),
        ));
    }

    // Add user message and generate planner response
    let result = state.sessions.update(id, |session| {
        session.add_message("user", &content);

        // Generate planner response
        // In a real implementation, this would call the pipeline
        if !session.pipeline_running {
            session.pipeline_running = true;
            session.project_description = Some(content.clone());
            session.stages[0].status = "running".into();

            session.add_message(
                "planner",
                &format!(
                    "Starting Socratic planning for: \"{}\"\n\n\
                     Let me analyze your request and prepare some clarifying questions.\n\
                     The pipeline will run through {} stages.\n\n\
                     [Pipeline execution requires claude/gemini/codex CLI tools.]",
                    content,
                    session.stages.len()
                ),
            );

            session.stages[0].status = "complete".into();
            session.stages[1].status = "running".into();
        } else {
            session.add_message(
                "planner",
                "Thank you for that clarification. I've incorporated your \
                 feedback into the specification.",
            );
        }
    });

    match result {
        Some(session) => {
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
// WebSocket stub
// ---------------------------------------------------------------------------

// WebSocket handler will be implemented when we wire real-time pipeline updates.
// For now, the REST API provides the core functionality.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStore;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    use uuid::Uuid;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            sessions: SessionStore::new(),
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
        assert_eq!(state.sessions.count(), 1);
    }

    #[tokio::test]
    async fn test_get_session() {
        let state = test_state();
        let session = state.sessions.create();
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
        let session = state.sessions.create();
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
        // system + user + planner = 3
        assert_eq!(response.session.messages.len(), 3);
    }

    #[tokio::test]
    async fn test_send_empty_message() {
        let state = test_state();
        let session = state.sessions.create();
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
}
