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

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;
use uuid::Uuid;

use planner_core::llm::providers::LlmRouter;
use planner_core::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
use planner_server::api;
use planner_server::session::{ResumeStatus, SessionStore};
use planner_server::ws_socratic;
use planner_server::AppState;

// ===========================================================================
// Helpers
// ===========================================================================

/// Create shared state in dev mode (no auth required).
fn test_state() -> Arc<AppState> {
    test_state_with_router(LlmRouter::from_env())
}

fn test_state_with_router(router: LlmRouter) -> Arc<AppState> {
    test_state_with_router_and_lease(router, Duration::from_secs(30))
}

fn test_state_with_router_and_lease(router: LlmRouter, lease: Duration) -> Arc<AppState> {
    Arc::new(AppState {
        sessions: SessionStore::new(),
        auth_config: None,
        event_store: None,
        cxdb: None,
        llm_router: Arc::new(router),
        socratic_runtimes: planner_server::runtime::SessionRuntimeRegistry::new(lease),
        started_at: std::time::Instant::now(),
        blueprints: planner_core::blueprint::BlueprintStore::new(),
        proposals: planner_core::discovery::ProposalStore::new(),
    })
}

struct ResumeFlowMockLlm;

#[async_trait]
impl LlmClient for ResumeFlowMockLlm {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let system = request.system.as_deref().unwrap_or("");
        let content = if system.contains("Belief State Verifier") {
            r#"{
              "filled_updates": [
                {
                  "dimension": "goal",
                  "value": "Build a countdown timer for workouts",
                  "source_quote": "I want a countdown timer for workouts."
                }
              ],
              "uncertain_updates": [],
              "out_of_scope": [],
              "contradictions": [],
              "expertise_level": "intermediate",
              "user_wants_to_stop": false
            }"#
            .to_string()
        } else if system.contains("Generate ONE focused question about the target dimension") {
            r#"{
              "question": "What are the must-have features in the first version?",
              "quick_options": [],
              "allow_skip": true
            }"#
            .to_string()
        } else {
            return Err(LlmError::Other(format!(
                "unexpected mock request system prompt: {}",
                &system[..system.len().min(120)]
            )));
        };

        Ok(CompletionResponse {
            content,
            model: request.model,
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
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

async fn wait_for_ws_message_type(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    message_type: &str,
) -> serde_json::Value {
    loop {
        let next = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("timed out waiting for ws message")
            .expect("websocket closed unexpectedly")
            .expect("websocket error");

        let text = match next {
            Message::Text(t) => t,
            Message::Close(_) => {
                panic!("websocket closed before receiving message type {message_type}")
            }
            _ => continue,
        };

        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        if parsed["type"] == message_type {
            return parsed;
        }
    }
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

    let interviewing_live_detached = state.sessions.create("dev|local");
    state.sessions.update(interviewing_live_detached.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build timer".into());
        s.interview_runtime_active = true;
        s.interview_live_attached = false;
    });

    let interviewing_checkpoint = state.sessions.create("dev|local");
    state.sessions.update(interviewing_checkpoint.id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build timer".into());
        s.ensure_checkpoint();
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

    let interviewing_live_detached_session = fetch_session(interviewing_live_detached.id).await;
    assert_eq!(
        interviewing_live_detached_session["resume_status"],
        "live_attach_available"
    );
    assert_eq!(interviewing_live_detached_session["can_resume_live"], true);
    assert_eq!(
        interviewing_live_detached_session["can_resume_checkpoint"],
        false
    );
    assert_eq!(
        interviewing_live_detached_session["interview_live_attached"],
        false
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

/// Test 4b: Session payload exposes durable interview checkpoint fields.
#[tokio::test]
async fn tier2_session_exposes_interview_checkpoint_payload() {
    use planner_schemas::{Dimension, DraftSection, QuestionOutput, SpeculativeDraft};

    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;
    let run_id = Uuid::new_v4();

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.socratic_run_id = Some(run_id);
        let checkpoint = s.ensure_checkpoint();
        checkpoint.current_question = Some(QuestionOutput {
            question: "What are the main user roles?".into(),
            target_dimension: Dimension::Stakeholders,
            quick_options: Vec::new(),
            allow_skip: true,
        });
        checkpoint.pending_draft = Some(SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Draft goal summary".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: Vec::new(),
        });
        checkpoint.stale_turns = 1;
        checkpoint.touch();
    });

    let req = Request::builder()
        .uri(format!("/sessions/{}", session_id))
        .body(Body::empty())
        .unwrap();
    let resp = test_app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let checkpoint = &parsed["session"]["checkpoint"];

    assert_eq!(parsed["session"]["socratic_run_id"], run_id.to_string());
    assert!(parsed["session"]["has_checkpoint"]
        .as_bool()
        .unwrap_or(false));
    assert_eq!(
        checkpoint["current_question"]["question"],
        "What are the main user roles?"
    );
    assert_eq!(
        checkpoint["pending_draft"]["sections"][0]["heading"],
        "Goal"
    );
    assert_eq!(checkpoint["stale_turns"], 1);
    assert!(checkpoint["last_checkpoint_at"].is_string());
}

/// Test 4c: Restart-from-description resets transient interview state without
/// requiring the client to resend the saved description to the REST API.
#[tokio::test]
async fn tier2_restart_from_description_resets_session_state() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.project_description = Some("Build a timer app".into());
        s.intake_phase = "interviewing".into();
        s.add_message("user", "Old description");
        s.add_message("planner", "Old follow-up");
        s.current_step = Some("socratic.question.generated".into());
        s.error_message = Some("stale error".into());
        s.ensure_checkpoint();
        s.record_event(planner_core::observability::PlannerEvent::warn(
            planner_core::observability::EventSource::SocraticEngine,
            "socratic.detached",
            "Detached",
        ));
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/restart-from-description", session_id))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let resp = test_app(state.clone()).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let session = &parsed["session"];

    assert_eq!(session["intake_phase"], "interviewing");
    assert_eq!(session["project_description"], "Build a timer app");
    assert_eq!(session["messages"].as_array().unwrap().len(), 1);
    assert_eq!(session["events"].as_array().unwrap().len(), 0);
    assert!(session["checkpoint"].is_null());
    assert_eq!(session["current_step"], serde_json::Value::Null);
    assert_eq!(session["error_message"], serde_json::Value::Null);
}

/// Test 4d: Retry-pipeline is only available for sessions with a failed
/// pipeline state and immediately returns the session to pipeline_running.
#[tokio::test]
async fn tier2_retry_pipeline_restarts_failed_pipeline_state() {
    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.project_description = Some("Build a timer app".into());
        s.intake_phase = "error".into();
        s.pipeline_running = false;
        s.stages[0].status = "complete".into();
        s.stages[1].status = "failed".into();
        s.error_message = Some("Pipeline failed".into());
        s.record_event(planner_core::observability::PlannerEvent::error(
            planner_core::observability::EventSource::Pipeline,
            "pipeline.error",
            "Pipeline failed",
        ));
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/retry-pipeline", session_id))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let resp = test_app(state.clone()).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let session = &parsed["session"];

    assert_eq!(session["intake_phase"], "pipeline_running");
    assert_eq!(session["pipeline_running"], true);
    assert_eq!(session["error_message"], serde_json::Value::Null);
    assert_eq!(session["events"].as_array().unwrap().len(), 0);
    assert_eq!(session["stages"][0]["status"], "running");
    assert_eq!(session["stages"][1]["status"], "pending");
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

/// Test 10: Reconnecting to a checkpoint-resumable interview re-emits the
/// pending question without requiring a new initial socratic_response.
#[tokio::test]
async fn tier2_socratic_ws_reconnect_checkpoint_reemits_question() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, QuestionOutput,
        RequirementsBeliefState,
    };

    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.interview_live_attached = false;
        s.socratic_run_id = Some(Uuid::new_v4());
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = Some(QuestionOutput {
            question: "Who is the primary user?".into(),
            target_dimension: Dimension::Stakeholders,
            quick_options: Vec::new(),
            allow_skip: true,
        });
        checkpoint.pending_draft = None;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let question = wait_for_ws_message_type(&mut ws, "question").await;
    assert_eq!(question["text"], "Who is the primary user?");

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "interviewing");
    assert_eq!(after.interview_live_attached, false);
    assert_eq!(after.resume_status, ResumeStatus::LiveAttachAvailable);
    assert_eq!(after.can_resume_live, true);
    assert_eq!(after.can_resume_checkpoint, false);

    handle.abort();
}

/// Test 11: Reconnecting to a checkpoint-resumable interview re-emits the
/// pending draft when no question is pending.
#[tokio::test]
async fn tier2_socratic_ws_reconnect_checkpoint_reemits_draft() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, DraftSection, ProjectType,
        RequirementsBeliefState, SpeculativeDraft,
    };

    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.interview_live_attached = false;
        s.socratic_run_id = Some(Uuid::new_v4());
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = None;
        checkpoint.pending_draft = Some(SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a timer app with presets.".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: Vec::new(),
        });
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let draft = wait_for_ws_message_type(&mut ws, "speculative_draft").await;
    assert_eq!(draft["sections"][0]["heading"], "Goal");

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "interviewing");
    assert_eq!(after.interview_live_attached, false);
    assert_eq!(after.resume_status, ResumeStatus::LiveAttachAvailable);
    assert_eq!(after.can_resume_live, true);
    assert_eq!(after.can_resume_checkpoint, false);

    handle.abort();
}

/// Test 12: Reconnecting to a checkpoint-resumable interview can accept an
/// answer to the resumed question and continue the interview loop.
#[tokio::test]
async fn tier2_socratic_ws_resume_answer_progresses_to_next_question() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, QuestionOutput,
        RequirementsBeliefState,
    };

    let state = test_state_with_router(LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)));
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.interview_live_attached = false;
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = Some(QuestionOutput {
            question: "What is the main goal of this tool?".into(),
            target_dimension: Dimension::Goal,
            quick_options: Vec::new(),
            allow_skip: false,
        });
        checkpoint.pending_draft = None;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let resumed_question = wait_for_ws_message_type(&mut ws, "question").await;
    assert_eq!(
        resumed_question["text"],
        "What is the main goal of this tool?"
    );

    ws.send(Message::Text(
        serde_json::json!({
            "type": "socratic_response",
            "content": "I want a countdown timer for workouts."
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let belief_update = wait_for_ws_message_type(&mut ws, "belief_state_update").await;
    assert_eq!(
        belief_update["filled"]["Goal / Purpose"]["value"],
        "Build a countdown timer for workouts"
    );

    let next_question = wait_for_ws_message_type(&mut ws, "question").await;
    assert_eq!(
        next_question["text"],
        "What are the must-have features in the first version?"
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    let after = state.sessions.get(session_id).unwrap();
    assert_eq!(after.intake_phase, "interviewing");
    assert_eq!(after.interview_live_attached, true);

    let checkpoint = after
        .checkpoint
        .as_ref()
        .expect("checkpoint should remain present after resumed answer");
    let checkpoint_state = checkpoint
        .belief_state
        .as_ref()
        .expect("checkpoint belief state should be updated after resumed answer");
    assert_eq!(
        checkpoint_state
            .filled
            .get(&Dimension::Goal)
            .expect("goal should be filled")
            .value,
        "Build a countdown timer for workouts"
    );
    assert_eq!(
        checkpoint
            .current_question
            .as_ref()
            .expect("next question should be checkpointed")
            .question,
        "What are the must-have features in the first version?"
    );
    assert!(checkpoint.pending_draft.is_none());

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    handle.abort();
}

/// Test 13: Disconnecting from a live interview runtime keeps the runtime
/// available for fast reattach within the lease window.
#[tokio::test]
async fn tier2_socratic_ws_live_runtime_reattach_within_lease() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, QuestionOutput,
        RequirementsBeliefState,
    };

    let state = test_state_with_router_and_lease(
        LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)),
        Duration::from_secs(5),
    );
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = Some(QuestionOutput {
            question: "What is the main goal of this tool?".into(),
            target_dimension: Dimension::Goal,
            quick_options: Vec::new(),
            allow_skip: false,
        });
        checkpoint.pending_draft = None;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let resumed_question = wait_for_ws_message_type(&mut ws1, "question").await;
    assert_eq!(
        resumed_question["text"],
        "What is the main goal of this tool?"
    );

    let _ = ws1.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let detached = state.sessions.get(session_id).unwrap();
    assert_eq!(detached.intake_phase, "interviewing");
    assert_eq!(detached.resume_status, ResumeStatus::LiveAttachAvailable);
    assert!(detached.can_resume_live);
    assert!(!detached.can_resume_checkpoint);
    assert!(!detached.interview_live_attached);

    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    ws2.send(Message::Text(
        serde_json::json!({
            "type": "socratic_response",
            "content": "I want a countdown timer for workouts."
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let belief_update = wait_for_ws_message_type(&mut ws2, "belief_state_update").await;
    assert_eq!(
        belief_update["filled"]["Goal / Purpose"]["value"],
        "Build a countdown timer for workouts"
    );
    let next_question = wait_for_ws_message_type(&mut ws2, "question").await;
    assert_eq!(
        next_question["text"],
        "What are the must-have features in the first version?"
    );

    let attached_again = state.sessions.get(session_id).unwrap();
    assert_eq!(
        attached_again.resume_status,
        ResumeStatus::InterviewAttached
    );
    assert!(attached_again.interview_live_attached);

    let _ = ws2.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 14: When the live runtime lease expires, the session falls back to
/// checkpoint-only resume and the next attach restores from checkpoint.
#[tokio::test]
async fn tier2_socratic_ws_live_runtime_lease_expiry_falls_back_to_checkpoint() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, QuestionOutput,
        RequirementsBeliefState,
    };

    let state = test_state_with_router_and_lease(
        LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)),
        Duration::from_millis(50),
    );
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = Some(QuestionOutput {
            question: "What is the main goal of this tool?".into(),
            target_dimension: Dimension::Goal,
            quick_options: Vec::new(),
            allow_skip: false,
        });
        checkpoint.pending_draft = None;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let _ = wait_for_ws_message_type(&mut ws1, "question").await;
    let _ = ws1.close(None).await;

    tokio::time::sleep(Duration::from_millis(100)).await;
    ws_socratic::expire_detached_runtimes(&state);
    tokio::time::sleep(Duration::from_millis(250)).await;

    let fallback = state.sessions.get(session_id).unwrap();
    assert_eq!(
        fallback.resume_status,
        ResumeStatus::InterviewCheckpointResumable
    );
    assert!(!fallback.can_resume_live);
    assert!(fallback.can_resume_checkpoint);
    assert!(!fallback.interview_live_attached);

    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();
    let resumed_question = wait_for_ws_message_type(&mut ws2, "question").await;
    assert_eq!(
        resumed_question["text"],
        "What is the main goal of this tool?"
    );

    let _ = ws2.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 15: A second websocket cannot steal an actively attached live
/// interview runtime from the current client.
#[tokio::test]
async fn tier2_socratic_ws_duplicate_live_attach_is_rejected() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, QuestionOutput,
        RequirementsBeliefState,
    };

    let state = test_state_with_router_and_lease(
        LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)),
        Duration::from_secs(5),
    );
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    let classification = DomainClassification {
        project_type: ProjectType::WebApp,
        complexity: ComplexityTier::Standard,
        detected_signals: vec!["web".into()],
        required_dimensions: Dimension::required_for(&ProjectType::WebApp),
    };
    let belief_state = RequirementsBeliefState::from_classification(&classification);

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        s.classification = Some(classification.clone());
        s.belief_state = Some(belief_state.clone());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.classification = Some(classification.clone());
        checkpoint.belief_state = Some(belief_state.clone());
        checkpoint.current_question = Some(QuestionOutput {
            question: "What is the main goal of this tool?".into(),
            target_dimension: Dimension::Goal,
            quick_options: Vec::new(),
            allow_skip: false,
        });
        checkpoint.pending_draft = None;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let _ = wait_for_ws_message_type(&mut ws1, "question").await;

    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();
    let error = wait_for_ws_message_type(&mut ws2, "error").await;
    assert!(error["message"]
        .as_str()
        .unwrap_or("")
        .contains("already attached"));

    ws1.send(Message::Text(
        serde_json::json!({
            "type": "socratic_response",
            "content": "I want a countdown timer for workouts."
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    let belief_update = wait_for_ws_message_type(&mut ws1, "belief_state_update").await;
    assert_eq!(
        belief_update["filled"]["Goal / Purpose"]["value"],
        "Build a countdown timer for workouts"
    );

    let _ = ws2.close(None).await;
    let _ = ws1.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}
