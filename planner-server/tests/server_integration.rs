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
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;
use uuid::Uuid;

use planner_server::api;
use planner_server::session::{ResumeStatus, SessionStore};
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

async fn spawn_test_server(
    app: axum::Router,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, handle)
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
        assert!(model["role"].is_string(), "Model missing role: {:?}", model);
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

/// Test 4: Session capability fields are backend-computed from current phase truth.
#[tokio::test]
async fn tier2_session_capability_mapping() {
    let state = test_state();

    let waiting = state.sessions.create("dev|local");

    let interviewing_restart = state.sessions.create("dev|local");
    state.sessions.update(interviewing_restart.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build timer".into());
        s.interview_live_attached = false;
    });

    let interviewing_unknown = state.sessions.create("dev|local");
    state.sessions.update(interviewing_unknown.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = None;
        s.interview_live_attached = false;
    });

    let interviewing_attached = state.sessions.create("dev|local");
    state.sessions.update(interviewing_attached.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build timer".into());
        s.interview_live_attached = true;
    });

    let interviewing_checkpoint = state.sessions.create("dev|local");
    state.sessions.update(interviewing_checkpoint.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build timer".into());
        s.has_checkpoint = true;
        s.interview_live_attached = false;
    });

    let pipeline_running = state.sessions.create("dev|local");
    state.sessions.update(pipeline_running.id, |s| {
        s.intake_phase = "pipeline_running".into();
        s.pipeline_running = true;
        s.project_description = Some("Build timer".into());
    });

    let complete = state.sessions.create("dev|local");
    state.sessions.update(complete.id, |s| {
        s.intake_phase = "complete".into();
        s.pipeline_running = false;
        s.project_description = Some("Build timer".into());
    });

    let errored = state.sessions.create("dev|local");
    state.sessions.update(errored.id, |s| {
        s.intake_phase = "error".into();
        s.error_message = Some("boom".into());
        s.project_description = Some("Build timer".into());
    });

    let fetch_session = |id: Uuid| {
        let state = state.clone();
        async move {
            let req = Request::builder()
                .uri(format!("/sessions/{}", id))
                .body(Body::empty())
                .unwrap();
            let resp = test_app(state).oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
            parsed["session"].clone()
        }
    };

    let waiting_session = fetch_session(waiting.id).await;
    assert_eq!(waiting_session["resume_status"], "ready_to_start");
    assert_eq!(waiting_session["can_resume_live"], false);
    assert_eq!(waiting_session["can_resume_checkpoint"], false);
    assert_eq!(waiting_session["can_restart_from_description"], false);
    assert_eq!(waiting_session["can_retry_pipeline"], false);
    assert_eq!(waiting_session["has_checkpoint"], false);

    let interviewing_restart_session = fetch_session(interviewing_restart.id).await;
    assert_eq!(
        interviewing_restart_session["resume_status"],
        "interview_restart_only"
    );
    assert_eq!(interviewing_restart_session["can_resume_live"], false);
    assert_eq!(interviewing_restart_session["can_resume_checkpoint"], false);
    assert_eq!(
        interviewing_restart_session["can_restart_from_description"],
        true
    );
    assert_eq!(interviewing_restart_session["has_checkpoint"], false);
    assert_eq!(
        interviewing_restart_session["interview_live_attached"],
        false
    );

    let interviewing_unknown_session = fetch_session(interviewing_unknown.id).await;
    assert_eq!(
        interviewing_unknown_session["resume_status"],
        "interview_resume_unknown"
    );
    assert_eq!(interviewing_unknown_session["can_resume_live"], false);
    assert_eq!(
        interviewing_unknown_session["can_restart_from_description"],
        false
    );
    assert_eq!(
        interviewing_unknown_session["interview_live_attached"],
        false
    );

    let interviewing_attached_session = fetch_session(interviewing_attached.id).await;
    assert_eq!(
        interviewing_attached_session["resume_status"],
        "interview_attached"
    );
    assert_eq!(interviewing_attached_session["can_resume_live"], false);
    assert_eq!(
        interviewing_attached_session["can_resume_checkpoint"],
        false
    );
    assert_eq!(
        interviewing_attached_session["interview_live_attached"],
        true
    );

    let interviewing_checkpoint_session = fetch_session(interviewing_checkpoint.id).await;
    assert_eq!(
        interviewing_checkpoint_session["resume_status"],
        "interview_checkpoint_resumable"
    );
    assert_eq!(
        interviewing_checkpoint_session["can_resume_checkpoint"],
        true
    );
    assert_eq!(interviewing_checkpoint_session["has_checkpoint"], true);
    assert_eq!(
        interviewing_checkpoint_session["interview_live_attached"],
        false
    );

    let pipeline_session = fetch_session(pipeline_running.id).await;
    assert_eq!(pipeline_session["resume_status"], "live_attach_available");
    assert_eq!(pipeline_session["can_resume_live"], true);

    let complete_session = fetch_session(complete.id).await;
    assert_eq!(complete_session["resume_status"], "live_attach_available");
    assert_eq!(complete_session["can_resume_live"], true);

    let errored_session = fetch_session(errored.id).await;
    assert_eq!(errored_session["resume_status"], "live_attach_available");
    assert_eq!(errored_session["can_resume_live"], true);

    // List endpoint should expose the same capability shape for dashboard cards.
    let list_req = Request::builder()
        .uri("/sessions")
        .body(Body::empty())
        .unwrap();
    let list_resp = test_app(state.clone()).oneshot(list_req).await.unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let listed: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
    let sessions = listed["sessions"].as_array().unwrap();

    let waiting_id = waiting.id.to_string();
    let waiting_summary = sessions
        .iter()
        .find(|s| s["id"] == waiting_id)
        .expect("waiting session should exist in list response");
    assert_eq!(waiting_summary["resume_status"], "ready_to_start");
    assert_eq!(waiting_summary["can_resume_live"], false);
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

    assert!(error["error"].as_str().unwrap().contains("not found"));

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

/// Test 6: Attaching to a pipeline-running Socratic websocket session does not
/// restart interviewing state or append a duplicate initial user message.
#[tokio::test]
async fn tier2_socratic_ws_attach_pipeline_running_is_idempotent() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "pipeline_running".into();
        s.pipeline_running = true;
        s.project_description = Some("Build a timer app".into());
    });

    let baseline = state.sessions.get(session_id).unwrap();
    let baseline_msg_count = baseline.messages.len();
    let baseline_phase = baseline.intake_phase.clone();

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, baseline_phase);
    assert_eq!(after.messages.len(), baseline_msg_count);
    assert_eq!(after.pipeline_running, true);

    let _ = ws.close(None).await;
    handle.abort();
}

/// Test 7: Attaching to a completed Socratic websocket session returns a
/// pipeline_complete event without restarting session state.
#[tokio::test]
async fn tier2_socratic_ws_attach_complete_returns_pipeline_complete() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "complete".into();
        s.pipeline_running = false;
        s.project_description = Some("Build a timer app".into());
        for stage in &mut s.stages {
            stage.status = "complete".into();
        }
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let next = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timed out waiting for ws message")
        .expect("websocket closed unexpectedly")
        .expect("websocket error");

    let text = match next {
        Message::Text(t) => t,
        other => panic!("expected text ws message, got {:?}", other),
    };
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["type"], "pipeline_complete");

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "complete");
    assert_eq!(after.pipeline_running, false);

    let _ = ws.close(None).await;
    handle.abort();
}

/// Test 8: Disconnecting during an interviewing websocket marks the interview
/// as detached without forcing pipeline_running.
#[tokio::test]
async fn tier2_socratic_ws_disconnect_mid_interview_preserves_interview_phase() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.interview_live_attached = true;
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "interviewing");
    assert_eq!(after.pipeline_running, false);
    assert_eq!(after.interview_live_attached, false);
    assert_eq!(after.resume_status, ResumeStatus::InterviewRestartOnly);

    handle.abort();
}

/// Test 9: Reconnecting to a detached interviewing session must keep the
/// detached restart-only state (no implicit resume).
#[tokio::test]
async fn tier2_socratic_ws_reconnect_detached_interview_stays_detached() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.interview_live_attached = false;
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "interviewing");
    assert_eq!(after.interview_live_attached, false);
    assert_eq!(after.resume_status, ResumeStatus::InterviewRestartOnly);
    assert_eq!(after.can_resume_live, false);
    assert_eq!(after.can_resume_checkpoint, false);

    handle.abort();
}
