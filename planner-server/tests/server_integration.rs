//! # Tier 2 — Server Integration Tests
//!
//! These tests exercise the Axum HTTP API layer via `tower::ServiceExt::oneshot`,
//! verifying that the full route + middleware + handler stack works correctly
//! in dev mode (auth_config = None → user = "dev|local").
//!
//! 5 tests:
//! 1. `tier2_health_endpoint`        — GET /health returns 200 + correct JSON
//! 2. `tier2_models_endpoint`        — GET /models returns 200 + lists all models
//! 3. `tier2_create_session`         — POST /sessions creates a session, returns 201
//! 4. `tier2_send_message_triggers_pipeline` — POST /sessions/:id/message sets pipeline_running
//! 5. `tier2_session_not_found`      — GET /sessions/:nonexistent returns 404
//!
//! This file is NEW and does NOT modify any existing test files.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use uuid::Uuid;

use planner_server::api;
use planner_server::session::SessionStore;
use planner_server::AppState;

// ===========================================================================
// Helpers
// ===========================================================================

/// Create shared state in dev mode (no auth required).
fn test_state() -> Arc<AppState> {
    Arc::new(AppState {
        sessions: SessionStore::new(),
        auth_config: None,
        event_store: None,
        cxdb: None,
        started_at: std::time::Instant::now(),
        blueprints: planner_core::blueprint::BlueprintStore::new(),
        proposals: planner_core::discovery::ProposalStore::new(),
    })
}

/// Build the full API router with the given state.
fn test_app(state: Arc<AppState>) -> axum::Router {
    api::routes(state)
}

// ===========================================================================
// Tier 2: Server Integration Tests
// ===========================================================================

/// Test 1: Health endpoint returns 200 with correct JSON fields.
#[tokio::test]
async fn tier2_health_endpoint() {
    let state = test_state();
    let app = test_app(state);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Server status may be "ok" or "degraded" depending on LLM availability.
    let status = health["status"].as_str().unwrap();
    assert!(
        status == "ok" || status == "degraded",
        "Expected 'ok' or 'degraded', got '{}'",
        status,
    );
    assert_eq!(health["version"], "0.1.0");
    assert_eq!(health["sessions_active"], 0);
}

/// Test 2: Models endpoint returns 200 with all model definitions.
#[tokio::test]
async fn tier2_models_endpoint() {
    let state = test_state();
    let app = test_app(state);

    let req = Request::builder()
        .uri("/models")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let models: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let model_list = models["models"].as_array().unwrap();
    assert!(
        model_list.len() >= 6,
        "Expected at least 6 models, got {}",
        model_list.len()
    );

    // Verify known models are present
    let model_ids: Vec<&str> = model_list
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert!(model_ids.contains(&"claude-opus-4-6"));
    assert!(model_ids.contains(&"gpt-5.3-codex"));
    assert!(model_ids.contains(&"gemini-3.1-pro-preview"));

    // Each model should have all required fields
    for model in model_list {
        assert!(model["id"].is_string(), "Model missing id: {:?}", model);
        assert!(
            model["provider"].is_string(),
            "Model missing provider: {:?}",
            model
        );
        assert!(
            model["cli_binary"].is_string(),
            "Model missing cli_binary: {:?}",
            model
        );
        assert!(
            model["role"].is_string(),
            "Model missing role: {:?}",
            model
        );
    }
}

/// Test 3: Creating a session returns 201 and a valid session object.
#[tokio::test]
async fn tier2_create_session() {
    let state = test_state();
    let app = test_app(state.clone());

    // Verify no sessions exist initially
    assert_eq!(state.sessions.count(), 0);

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
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let session = &created["session"];
    assert!(session["id"].is_string(), "Session missing id");
    assert_eq!(session["user_id"], "dev|local");
    assert!(!session["pipeline_running"].as_bool().unwrap());

    // Should have the welcome system message
    let messages = session["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "system");

    // Should have all 12 pipeline stages
    let stages = session["stages"].as_array().unwrap();
    assert_eq!(stages.len(), 12);
    for stage in stages {
        assert_eq!(stage["status"], "pending");
    }

    // Session store should now contain 1 session
    assert_eq!(state.sessions.count(), 1);

    // Create another session and verify list
    let app2 = test_app(state.clone());
    let req2 = Request::builder()
        .method("GET")
        .uri("/sessions")
        .body(Body::empty())
        .unwrap();
    let resp2 = app2.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
        .await
        .unwrap();
    let listed: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(listed["sessions"].as_array().unwrap().len(), 1);
}

/// Test 4: Sending a message triggers the pipeline.
///
/// Verifies the full request→handler→session-update flow:
/// - User message is stored
/// - Planner acknowledgement message is generated
/// - `pipeline_running` is set to true
/// - Intake stage is marked "running"
#[tokio::test]
async fn tier2_send_message_triggers_pipeline() {
    let state = test_state();

    // Pre-create a session
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let app = test_app(state.clone());

    let msg_body = serde_json::json!({
        "content": "Build me a countdown timer widget"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/message", session_id))
        .header("content-type", "application/json")
        .body(Body::from(msg_body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // User message should be captured
    assert_eq!(response["user_message"]["role"], "user");
    assert!(response["user_message"]["content"]
        .as_str()
        .unwrap()
        .contains("countdown timer"));

    // Planner message should acknowledge pipeline start
    assert_eq!(response["planner_message"]["role"], "planner");
    assert!(response["planner_message"]["content"]
        .as_str()
        .unwrap()
        .contains("pipeline"));

    // Session should be marked as running
    let session_state = &response["session"];
    assert!(session_state["pipeline_running"].as_bool().unwrap());

    // Intake stage should be "running"
    let stages = session_state["stages"].as_array().unwrap();
    assert_eq!(stages[0]["name"], "Intake");
    assert_eq!(stages[0]["status"], "running");

    // Messages: system welcome + user + planner = 3
    let messages = session_state["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
}

/// Test 5: Getting a non-existent session returns 404.
#[tokio::test]
async fn tier2_session_not_found() {
    let state = test_state();
    let app = test_app(state);

    let fake_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/sessions/{}", fake_id))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(error["error"]
        .as_str()
        .unwrap()
        .contains("not found"));

    // Also test that sending a message to non-existent session returns 404
    let app2 = test_app(test_state());
    let msg_body = serde_json::json!({ "content": "hello" }).to_string();
    let req2 = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/message", fake_id))
        .header("content-type", "application/json")
        .body(Body::from(msg_body))
        .unwrap();

    let resp2 = app2.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::NOT_FOUND);
}
