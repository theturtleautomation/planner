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
        pipeline_runtimes: planner_server::runtime::SessionPipelineRegistry::new(),
        started_at: std::time::Instant::now(),
        blueprints: planner_core::blueprint::BlueprintStore::new(),
        proposals: planner_core::discovery::ProposalStore::new(),
        projects: planner_server::project::ProjectStore::new(),
        imports: planner_server::import::ProjectImportStore::new(),
        import_acquirer: planner_server::import::default_import_acquirer(),
        import_analyzer: planner_server::import::default_import_analyzer(),
    })
}

struct ResumeFlowMockLlm;
const EXPECTED_RESUME_BANK_PROMPT: &str =
    "What is the main outcome this project needs to deliver first?";

fn mock_classification_response() -> String {
    serde_json::json!({
        "project_type": "cli_tool",
        "complexity": "light",
        "detected_signals": ["timer", "workout", "countdown"]
    })
    .to_string()
}

fn extract_target_dimension_label(content: &str) -> Option<String> {
    let marker = "## Target Dimension: ";
    let start = content.find(marker)? + marker.len();
    let suffix = &content[start..];
    let end = suffix.find(" (").or_else(|| suffix.find('\n'))?;
    Some(suffix[..end].trim().to_ascii_lowercase())
}

fn mock_question_response_for_dimension(label: &str) -> String {
    let payload = match label {
        "goal" => serde_json::json!({
            "question": "What is the main goal of this tool?",
            "quick_options": [],
            "allow_skip": false
        }),
        "core features" => serde_json::json!({
            "question": "What are the must-have features in the first version?",
            "quick_options": [],
            "allow_skip": true
        }),
        "user flows" => serde_json::json!({
            "question": "Which user flow needs to feel reliable first?",
            "quick_options": [],
            "allow_skip": true
        }),
        "success criteria" => serde_json::json!({
            "question": "How will you know the first release is successful?",
            "quick_options": [],
            "allow_skip": false
        }),
        "out of scope" => serde_json::json!({
            "question": "What should stay out of scope for the first release?",
            "quick_options": [],
            "allow_skip": true
        }),
        "security" => serde_json::json!({
            "question": "What is the minimum security bar for the first release?",
            "quick_options": [],
            "allow_skip": false
        }),
        "error handling" => serde_json::json!({
            "question": "What failure cases must the first release handle cleanly?",
            "quick_options": [],
            "allow_skip": false
        }),
        _ => serde_json::json!({
            "question": "What are the must-have features in the first version?",
            "quick_options": [],
            "allow_skip": true
        }),
    };

    payload.to_string()
}

#[async_trait]
impl LlmClient for ResumeFlowMockLlm {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let system = request.system.as_deref().unwrap_or("");
        let content = if system
            .contains("project classifier for a Socratic requirements elicitation system")
        {
            mock_classification_response()
        } else if system.contains("Belief State Adjudicator") {
            let item_ids = request
                .messages
                .last()
                .and_then(|message| {
                    let marker = "## Prompt Answers To Adjudicate:\n";
                    let start = message.content.find(marker)?;
                    let payload_with_suffix = &message.content[start + marker.len()..];
                    let end = payload_with_suffix.find("\n\nReturn JSON now.")?;
                    let payload = &payload_with_suffix[..end];
                    let parsed: serde_json::Value = serde_json::from_str(payload).ok()?;
                    let items = parsed["items"].as_array()?;
                    Some(
                        items
                            .iter()
                            .filter_map(|item| item["item_id"].as_str().map(str::to_string))
                            .collect::<Vec<_>>(),
                    )
                })
                .filter(|ids| !ids.is_empty())
                .unwrap_or_else(|| vec!["item-1".to_string()]);

            serde_json::json!({
                "items": item_ids
                    .into_iter()
                    .map(|item_id| {
                        serde_json::json!({
                            "item_id": item_id,
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
                            "user_wants_to_stop": false
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .to_string()
        } else if system.contains("Belief State Verifier") {
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
            let label = request
                .messages
                .last()
                .and_then(|message| extract_target_dimension_label(&message.content))
                .unwrap_or_else(|| "core features".to_string());
            mock_question_response_for_dimension(&label)
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

async fn wait_for_ws_prompt_or_bank_message(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        let next = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("timed out waiting for ws prompt or prompt_bank message")
            .expect("websocket closed unexpectedly")
            .expect("websocket error");

        let text = match next {
            Message::Text(t) => t,
            Message::Close(_) => {
                panic!("websocket closed before receiving prompt or prompt_bank message")
            }
            _ => continue,
        };

        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        if parsed["type"] == "prompt" || parsed["type"] == "prompt_bank" {
            return parsed;
        }
    }
}

async fn wait_for_ws_answerable_prompt_message(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        let message = wait_for_ws_prompt_or_bank_message(ws).await;
        if message["type"] == "prompt" {
            return message;
        }
        if message["type"] == "prompt_bank"
            && message["bank"]["banked_threads"]
                .as_array()
                .map(|threads| !threads.is_empty())
                .unwrap_or(false)
        {
            return message;
        }
    }
}

async fn wait_for_ws_prompt_bank_where<F>(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    predicate: F,
) -> serde_json::Value
where
    F: Fn(&serde_json::Value) -> bool,
{
    loop {
        let parsed = wait_for_ws_message_type(ws, "prompt_bank").await;
        if predicate(&parsed) {
            return parsed;
        }
    }
}

fn prompt_payload<'a>(message: &'a serde_json::Value) -> &'a serde_json::Value {
    if message["type"] == "prompt" {
        &message["prompt"]
    } else {
        &message["bank"]["banked_threads"][0]["prompt"]
    }
}

fn checkpoint_question_prompt(
    question: &str,
    target_dimension: planner_schemas::Dimension,
    allow_skip: bool,
) -> planner_schemas::PromptEnvelope {
    let item_id = "item-1".to_string();
    planner_schemas::PromptEnvelope {
        prompt_id: "prompt-question".into(),
        kind: planner_schemas::PromptKind::QuestionBatch,
        title: "Continue interview".into(),
        instructions: None,
        origin_category_id: None,
        category_path: Vec::new(),
        items: vec![planner_schemas::PromptItem {
            item_id: item_id.clone(),
            kind: planner_schemas::PromptItemKind::Discovery,
            target_dimension: Some(target_dimension),
            section_ref: None,
            text: question.into(),
            options: Vec::new(),
            response_mode: planner_schemas::PromptResponseMode::SingleSelectWithCustomText,
            required: !allow_skip,
            priority: 100,
            dependency_item_ids: Vec::new(),
        }],
        draft_snapshot: None,
        required_item_ids: if allow_skip {
            Vec::new()
        } else {
            vec![item_id]
        },
        allow_partial_submit: true,
        ui_hints: planner_schemas::PromptUiHints {
            preferred_layout: planner_schemas::PromptPreferredLayout::Cards,
            show_draft_sidebar: false,
        },
        based_on_turn: 0,
        created_at: "2026-03-08T00:00:00Z".into(),
    }
}

fn checkpoint_draft_prompt(
    draft: planner_schemas::SpeculativeDraft,
) -> planner_schemas::PromptEnvelope {
    planner_schemas::PromptEnvelope {
        prompt_id: "prompt-draft".into(),
        kind: planner_schemas::PromptKind::DraftReview,
        title: "Review draft".into(),
        instructions: Some(
            "Confirm accurate sections and provide corrections where needed.".into(),
        ),
        origin_category_id: None,
        category_path: Vec::new(),
        items: vec![planner_schemas::PromptItem {
            item_id: "item-draft-1".into(),
            kind: planner_schemas::PromptItemKind::DraftSection,
            target_dimension: None,
            section_ref: draft
                .sections
                .first()
                .map(|section| section.heading.clone()),
            text: "Review this draft.".into(),
            options: Vec::new(),
            response_mode: planner_schemas::PromptResponseMode::SingleSelectWithCustomText,
            required: false,
            priority: 100,
            dependency_item_ids: Vec::new(),
        }],
        draft_snapshot: Some(draft),
        required_item_ids: Vec::new(),
        allow_partial_submit: true,
        ui_hints: planner_schemas::PromptUiHints {
            preferred_layout: planner_schemas::PromptPreferredLayout::Review,
            show_draft_sidebar: true,
        },
        based_on_turn: 0,
        created_at: "2026-03-08T00:00:00Z".into(),
    }
}

fn banked_checkpoint_prompt(
    category_id: &str,
    question: &str,
    target_dimension: planner_schemas::Dimension,
    allow_skip: bool,
) -> planner_schemas::PromptBankEntry {
    let mut prompt = checkpoint_question_prompt(question, target_dimension, allow_skip);
    prompt.origin_category_id = Some(category_id.to_string());
    planner_schemas::PromptBankEntry {
        category_id: category_id.to_string(),
        prompt,
    }
}

fn filled_slot(value: &str, source_turn: u32) -> planner_schemas::SlotValue {
    planner_schemas::SlotValue {
        value: value.to_string(),
        source_turn,
        source_quote: None,
    }
}

fn build_resume_belief_state(
    missing: Vec<planner_schemas::Dimension>,
    required_dimensions: Vec<planner_schemas::Dimension>,
) -> planner_schemas::RequirementsBeliefState {
    use planner_schemas::{Dimension, RequirementsBeliefState};

    let mut filled = std::collections::HashMap::new();
    filled.insert(
        Dimension::Security,
        filled_slot("Passwordless sign-in is enough for launch.", 1),
    );
    filled.insert(
        Dimension::ErrorHandling,
        filled_slot("Show a clear retry path when sync fails.", 1),
    );
    filled.insert(
        Dimension::SuccessCriteria,
        filled_slot(
            "The first release is successful when one workout timer can be completed cleanly.",
            1,
        ),
    );
    filled.insert(
        Dimension::OutOfScope,
        filled_slot(
            "Team collaboration and admin controls stay out of scope.",
            1,
        ),
    );

    RequirementsBeliefState {
        filled,
        uncertain: Default::default(),
        missing,
        out_of_scope: Vec::new(),
        contradictions: Vec::new(),
        required_dimensions,
        turn_count: 1,
        classification: None,
    }
}

async fn wait_for_session<F>(
    state: &Arc<AppState>,
    session_id: Uuid,
    timeout: Duration,
    predicate: F,
) -> planner_server::session::Session
where
    F: Fn(&planner_server::session::Session) -> bool,
{
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let session = state
            .sessions
            .get(session_id)
            .expect("session should remain present during integration test");
        if predicate(&session) {
            return session;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for session {} to satisfy integration-test predicate",
            session_id
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ===========================================================================
// Tier 2: Server Integration Tests
// ===========================================================================

/// Test 1: Health endpoint returns 200 with correct JSON fields.
#[tokio::test]
async fn tier2_health_endpoint() {
    let state = test_state();
    let app = test_app(state.clone());

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
    let app = test_app(state.clone());

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

#[tokio::test]
async fn tier2_session_prompt_bank_reports_banked_and_queued_threads_truthfully() {
    let state = test_state();
    let session = state.sessions.create("dev|local");

    state.sessions.update(session.id, |draft| {
        draft.intake_phase = "interviewing".into();
        draft.ensure_socratic_run_id();
        let checkpoint = draft.ensure_checkpoint();
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "Should this ship as a web app first?",
            planner_schemas::Dimension::Custom("platform".into()),
            true,
        ));
        checkpoint
            .current_prompt
            .as_mut()
            .unwrap()
            .origin_category_id = Some("verify-platform".into());
        checkpoint.prompt_bank = vec![
            planner_schemas::PromptBankEntry {
                category_id: "verify-platform".into(),
                prompt: checkpoint.current_prompt.clone().expect("platform prompt"),
            },
            planner_schemas::PromptBankEntry {
                category_id: "explore-user-flows".into(),
                prompt: planner_schemas::PromptEnvelope {
                    prompt_id: "prompt-user-flows".into(),
                    kind: planner_schemas::PromptKind::QuestionBatch,
                    title: "Explore User Flows".into(),
                    instructions: Some("Clarify the primary user flows.".into()),
                    origin_category_id: Some("explore-user-flows".into()),
                    category_path: vec![],
                    items: vec![planner_schemas::PromptItem {
                        item_id: "item-user-flows".into(),
                        kind: planner_schemas::PromptItemKind::Discovery,
                        target_dimension: Some(planner_schemas::Dimension::Custom(
                            "user_flows".into(),
                        )),
                        section_ref: None,
                        text: "Which user flows matter most?".into(),
                        options: vec![],
                        response_mode:
                            planner_schemas::PromptResponseMode::SingleSelectWithCustomText,
                        required: false,
                        priority: 100,
                        dependency_item_ids: vec![],
                    }],
                    draft_snapshot: None,
                    required_item_ids: vec![],
                    allow_partial_submit: true,
                    ui_hints: planner_schemas::PromptUiHints {
                        preferred_layout: planner_schemas::PromptPreferredLayout::Cards,
                        show_draft_sidebar: false,
                    },
                    based_on_turn: 1,
                    created_at: "2026-03-24T00:00:00Z".into(),
                },
            },
        ];
        checkpoint.active_thread_id = Some("verify-platform".into());
        checkpoint.initial_prompt_bank_complete = true;
        checkpoint.current_category_snapshot = Some(planner_schemas::SocraticCategorySnapshot {
            revision: "rev-1".into(),
            root_category_ids: vec!["platform".into()],
            nodes: vec![
                planner_schemas::SocraticCategoryNode {
                    category_id: "verify-platform".into(),
                    parent_category_id: Some("platform".into()),
                    title: "Verify Platform".into(),
                    summary: "Confirm the delivery surface.".into(),
                    status: planner_schemas::SocraticCategoryStatus::Active,
                    depth: 1,
                    mapped_dimensions: vec![planner_schemas::Dimension::Custom("platform".into())],
                    has_children: false,
                    has_prompt_ready: true,
                    item_count_hint: 1,
                    revision_kind: None,
                    revision_area_id: None,
                    revision_conflict: false,
                    low_risk_update: false,
                },
                planner_schemas::SocraticCategoryNode {
                    category_id: "explore-user-flows".into(),
                    parent_category_id: Some("platform".into()),
                    title: "Explore User Flows".into(),
                    summary: "Discover the primary flows.".into(),
                    status: planner_schemas::SocraticCategoryStatus::Pending,
                    depth: 1,
                    mapped_dimensions: vec![planner_schemas::Dimension::Custom(
                        "user_flows".into(),
                    )],
                    has_children: false,
                    has_prompt_ready: true,
                    item_count_hint: 2,
                    revision_kind: None,
                    revision_area_id: None,
                    revision_conflict: false,
                    low_risk_update: false,
                },
            ],
            active_category_path: vec![planner_schemas::SocraticCategoryPathEntry {
                category_id: "platform".into(),
                title: "Platform".into(),
            }],
            newly_available_category_ids: vec![],
            build_ready: false,
            build_readiness_message: "Question bank is still loading.".into(),
        });
    });

    let app = test_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(parsed["session_id"], session.id.to_string());
    assert_eq!(parsed["active_thread_id"], "verify-platform");
    assert_eq!(parsed["initial_bank_complete"], true);
    assert_eq!(parsed["banked_threads"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["queued_threads"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["banked_threads"][0]["title"], "Verify Platform");
    assert_eq!(parsed["banked_threads"][1]["title"], "Explore User Flows");
}

#[tokio::test]
async fn tier2_session_prompt_drafts_persist_without_mutating_prompt_bank() {
    let state = test_state();
    let session = state.sessions.create("dev|local");

    state.sessions.update(session.id, |draft| {
        draft.intake_phase = "interviewing".into();
        draft.ensure_socratic_run_id();
        let checkpoint = draft.ensure_checkpoint();
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "Should this ship as a web app first?",
            planner_schemas::Dimension::Custom("platform".into()),
            true,
        ));
        checkpoint
            .current_prompt
            .as_mut()
            .unwrap()
            .origin_category_id = Some("verify-platform".into());
        checkpoint.prompt_bank = vec![planner_schemas::PromptBankEntry {
            category_id: "verify-platform".into(),
            prompt: checkpoint.current_prompt.clone().expect("platform prompt"),
        }];
        checkpoint.active_thread_id = Some("verify-platform".into());
        checkpoint.initial_prompt_bank_complete = true;
    });

    let app = test_app(state.clone());
    let save_request = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/prompt-drafts", session.id))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "prompt_id": "prompt-question",
                "answers": [
                    {
                        "item_id": "item-1",
                        "selected_option_id": "web",
                        "custom_text": "Web first for launch."
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let save_response = app.oneshot(save_request).await.unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);
    let save_body = axum::body::to_bytes(save_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&save_body).unwrap();
    assert_eq!(saved["prompt_id"], "prompt-question");
    assert_eq!(saved["saved_count"], 1);
    assert_eq!(saved["cleared_count"], 0);

    let app = test_app(state.clone());
    let prompt_bank_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(prompt_bank_response.status(), StatusCode::OK);
    let prompt_bank_body = axum::body::to_bytes(prompt_bank_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let prompt_bank: serde_json::Value = serde_json::from_slice(&prompt_bank_body).unwrap();

    assert_eq!(prompt_bank["banked_threads"].as_array().unwrap().len(), 1);
    assert_eq!(prompt_bank["active_thread_id"], "verify-platform");
    assert_eq!(
        prompt_bank["saved_drafts"]["item-1"]["custom_text"],
        "Web first for launch."
    );
    assert_eq!(
        prompt_bank["saved_drafts"]["item-1"]["selected_option_id"],
        "web"
    );

    let app = test_app(state.clone());
    let clear_request = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/prompt-drafts", session.id))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "prompt_id": "prompt-question",
                "answers": [
                    {
                        "item_id": "item-1",
                        "skipped": true
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let clear_response = app.oneshot(clear_request).await.unwrap();
    assert_eq!(clear_response.status(), StatusCode::OK);
    let clear_body = axum::body::to_bytes(clear_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let cleared: serde_json::Value = serde_json::from_slice(&clear_body).unwrap();
    assert_eq!(cleared["saved_count"], 0);
    assert_eq!(cleared["cleared_count"], 1);

    let app = test_app(state);
    let prompt_bank_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let prompt_bank_body = axum::body::to_bytes(prompt_bank_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let prompt_bank: serde_json::Value = serde_json::from_slice(&prompt_bank_body).unwrap();
    assert!(prompt_bank["saved_drafts"]
        .as_object()
        .expect("saved drafts object")
        .is_empty());
}

#[tokio::test]
async fn tier2_session_prompt_bank_does_not_treat_legacy_current_prompt_as_complete_bank() {
    let state = test_state();
    let session = state.sessions.create("dev|local");

    state.sessions.update(session.id, |draft| {
        draft.intake_phase = "interviewing".into();
        draft.ensure_socratic_run_id();
        let checkpoint = draft.ensure_checkpoint();
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the primary goal?",
            planner_schemas::Dimension::Goal,
            false,
        ));
        checkpoint.initial_prompt_bank_complete = true;
    });

    let app = test_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(parsed["banked_threads"].as_array().unwrap().is_empty());
    assert_eq!(parsed["active_thread_id"], serde_json::Value::Null);
    assert_eq!(parsed["initial_bank_complete"], false);
    assert_eq!(parsed["build_ready"], false);
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
    assert_eq!(
        waiting_session["workspace_status"]["state"],
        "ready_to_start"
    );
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
    assert_eq!(
        interviewing_restart_session["workspace_status"]["state"],
        "attention_required"
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
    assert_eq!(
        waiting_summary["workspace_status"]["state"],
        "ready_to_start"
    );
    assert_eq!(waiting_summary["can_resume_live"], false);
}

/// Test 4b: Session payload exposes durable interview checkpoint fields.
#[tokio::test]
async fn tier2_session_exposes_interview_checkpoint_payload() {
    use planner_schemas::Dimension;

    let state = test_state();
    let session = state.sessions.create("dev|local");
    let session_id = session.id;
    let run_id = Uuid::new_v4();

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.socratic_run_id = Some(run_id);
        let checkpoint = s.ensure_checkpoint();
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What are the main user roles?",
            Dimension::Stakeholders,
            true,
        ));
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
        checkpoint["current_prompt"]["items"][0]["text"],
        "What are the main user roles?"
    );
    assert_eq!(checkpoint["current_prompt"]["kind"], "question_batch");
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

    assert_eq!(session["intake_phase"], "waiting");
    assert_eq!(session["project_description"], "Build a timer app");
    assert_eq!(session["messages"].as_array().unwrap().len(), 1);
    assert_eq!(session["events"].as_array().unwrap().len(), 0);
    assert!(session["checkpoint"].is_null());
    assert_eq!(session["current_step"], serde_json::Value::Null);
    assert_eq!(session["error_message"], serde_json::Value::Null);
    assert_eq!(session["resume_status"], "ready_to_start");
    assert_eq!(session["workspace_status"]["state"], "ready_to_start");
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
/// pending prompt without requiring a new initial prompt_response.
#[tokio::test]
async fn tier2_socratic_ws_reconnect_checkpoint_reemits_question() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "Who is the primary user?",
            Dimension::Stakeholders,
            true,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let prompt = wait_for_ws_answerable_prompt_message(&mut ws).await;
    let prompt_payload = prompt_payload(&prompt);
    assert_eq!(
        prompt_payload["items"][0]["text"],
        "Who is the primary user?"
    );

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
        checkpoint.current_prompt = Some(checkpoint_draft_prompt(SpeculativeDraft {
            sections: vec![DraftSection {
                heading: "Goal".into(),
                content: "Build a timer app with presets.".into(),
                dimensions: vec![Dimension::Goal],
            }],
            assumptions: Vec::new(),
            not_discussed: Vec::new(),
        }));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let prompt = wait_for_ws_answerable_prompt_message(&mut ws).await;
    let prompt_payload = prompt_payload(&prompt);
    assert_eq!(prompt_payload["kind"], "draft_review");
    assert_eq!(
        prompt_payload["draft_snapshot"]["sections"][0]["heading"],
        "Goal"
    );

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
/// answer to the resumed question and continue the interview loop, either by
/// checkpointing the next prompt directly or by restoring the live category
/// workspace with the next branch ready.
#[tokio::test]
async fn tier2_socratic_ws_resume_answer_progresses_to_next_question() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let resumed_prompt = wait_for_ws_answerable_prompt_message(&mut ws).await;
    let resumed_prompt_payload = prompt_payload(&resumed_prompt);
    assert_eq!(
        resumed_prompt_payload["items"][0]["text"],
        "What is the main goal of this tool?"
    );
    let prompt_id = resumed_prompt_payload["prompt_id"]
        .as_str()
        .expect("prompt id should be present");
    let item_id = resumed_prompt_payload["items"][0]["item_id"]
        .as_str()
        .expect("prompt item id should be present");

    ws.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "I want a countdown timer for workouts."
            }],
            "submitted_at": "2026-03-08T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let after = wait_for_session(&state, session_id, Duration::from_secs(2), |session| {
        let Some(checkpoint) = session.checkpoint.as_ref() else {
            return false;
        };
        let goal_filled = checkpoint
            .belief_state
            .as_ref()
            .and_then(|belief_state| belief_state.filled.get(&Dimension::Goal))
            .map(|slot| slot.value.as_str() == "Build a countdown timer for workouts")
            .unwrap_or(false);
        let has_next_prompt = checkpoint.current_prompt.is_some();
        let has_core_features_branch = checkpoint
            .current_category_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot.nodes.iter().any(|node| {
                    node.has_prompt_ready
                        && node
                            .mapped_dimensions
                            .iter()
                            .any(|dimension| *dimension == Dimension::CoreFeatures)
                })
            })
            .unwrap_or(false);

        session.intake_phase == "interviewing"
            && session.interview_live_attached
            && goal_filled
            && (has_next_prompt || has_core_features_branch)
    })
    .await;
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
    let checkpointed_next_prompt = checkpoint.current_prompt.as_ref();
    let checkpointed_banked_threads = !checkpoint.prompt_bank.is_empty();
    let checkpointed_core_features_branch = checkpoint
        .current_category_snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot.nodes.iter().any(|node| {
                node.has_prompt_ready
                    && node
                        .mapped_dimensions
                        .iter()
                        .any(|dimension| *dimension == Dimension::CoreFeatures)
            })
        })
        .unwrap_or(false);
    assert!(
        checkpointed_next_prompt.is_some()
            || checkpointed_banked_threads
            || checkpointed_core_features_branch,
        "expected resumed answer handling to leave either a checkpointed prompt bank, an active prompt, or a Core Features branch"
    );
    assert!(checkpoint
        .current_prompt
        .as_ref()
        .and_then(|prompt| prompt.draft_snapshot.as_ref())
        .is_none());

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    handle.abort();
}

#[tokio::test]
async fn tier2_socratic_ws_waiting_start_reaches_bank_first_reveal() {
    let state = test_state_with_router(LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)));
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "waiting".into();
        s.project_description = Some("Build a workout countdown timer".into());
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    ws.send(Message::Text(
        serde_json::json!({
            "type": "start_socratic",
            "description": "Build a workout countdown timer"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let bank_message = wait_for_ws_prompt_bank_where(&mut ws, |message| {
        message["bank"]["initial_bank_complete"]
            .as_bool()
            .unwrap_or(false)
            && !message["bank"]["banked_threads"]
                .as_array()
                .map(|threads| threads.is_empty())
                .unwrap_or(true)
    })
    .await;
    let bank = &bank_message["bank"];
    assert_eq!(bank["session_id"], session_id.to_string());
    assert!(bank["initial_bank_complete"].as_bool().unwrap_or(false));
    assert!(!bank["banked_threads"].as_array().unwrap().is_empty());
    assert!(bank["active_thread_id"].is_string());

    let after = wait_for_session(&state, session_id, Duration::from_secs(3), |session| {
        session.intake_phase == "interviewing"
            && session
                .checkpoint
                .as_ref()
                .map(|checkpoint| {
                    checkpoint.initial_prompt_bank_complete
                        && !checkpoint.prompt_bank.is_empty()
                        && checkpoint.current_prompt.is_none()
                })
                .unwrap_or(false)
    })
    .await;
    let checkpoint = after
        .checkpoint
        .as_ref()
        .expect("fresh-start checkpoint should be present");
    assert!(checkpoint.current_prompt.is_none());
    assert!(!checkpoint.prompt_bank.is_empty());

    let prompt_bank_response = test_app(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(prompt_bank_response.status(), StatusCode::OK);
    let prompt_bank_body = axum::body::to_bytes(prompt_bank_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let prompt_bank: serde_json::Value = serde_json::from_slice(&prompt_bank_body).unwrap();
    assert!(prompt_bank["initial_bank_complete"]
        .as_bool()
        .unwrap_or(false));
    assert!(!prompt_bank["banked_threads"].as_array().unwrap().is_empty());

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

#[tokio::test]
async fn tier2_socratic_ws_waiting_start_ignores_start_pipeline_alias() {
    let state = test_state_with_router(LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)));
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "waiting".into();
        s.project_description = Some("Build a workout countdown timer".into());
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    ws.send(Message::Text(
        serde_json::json!({
            "type": "start_pipeline",
            "description": "Build a workout countdown timer"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    assert!(
        tokio::time::timeout(Duration::from_millis(300), ws.next())
            .await
            .is_err(),
        "socratic websocket should ignore legacy start_pipeline startup messages"
    );

    let untouched = state.sessions.get(session_id).unwrap();
    assert_eq!(untouched.intake_phase, "waiting");
    assert!(!untouched.interview_runtime_active);
    assert!(!untouched.interview_live_attached);

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

#[tokio::test]
async fn tier2_socratic_ws_resume_answer_rebuilds_the_next_prompt_bank() {
    use planner_schemas::Dimension;

    let state = test_state_with_router(LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)));
    let session = state.sessions.create("dev|local");
    let session_id = session.id;
    let initial_category_id = "goal-thread";

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.belief_state = Some(build_resume_belief_state(
            vec![
                Dimension::Goal,
                Dimension::CoreFeatures,
                Dimension::UserFlows,
            ],
            vec![Dimension::Goal, Dimension::CoreFeatures],
        ));
        checkpoint.prompt_bank = vec![banked_checkpoint_prompt(
            initial_category_id,
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        )];
        checkpoint.active_thread_id = Some(initial_category_id.into());
        checkpoint.initial_prompt_bank_complete = true;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let replayed_bank = wait_for_ws_message_type(&mut ws, "prompt_bank").await;
    let replay_prompt = &replayed_bank["bank"]["banked_threads"][0]["prompt"];
    let prompt_id = replay_prompt["prompt_id"]
        .as_str()
        .expect("replayed prompt id should be present");
    let item_id = replay_prompt["items"][0]["item_id"]
        .as_str()
        .expect("replayed prompt item id should be present");

    ws.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "It should help someone complete one workout timer cleanly."
            }],
            "submitted_at": "2026-03-26T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let next_bank_message = wait_for_ws_prompt_bank_where(&mut ws, |message| {
        message["bank"]["banked_threads"]
            .as_array()
            .map(|threads| {
                !threads.is_empty()
                    && threads
                        .iter()
                        .all(|thread| thread["category_id"] != initial_category_id)
            })
            .unwrap_or(false)
    })
    .await;
    let next_bank = &next_bank_message["bank"];
    let next_banked_threads = next_bank["banked_threads"]
        .as_array()
        .expect("next prompt bank should expose banked threads");
    assert!(next_bank["initial_bank_complete"]
        .as_bool()
        .unwrap_or(false));
    assert!(!next_banked_threads.is_empty());
    assert!(next_banked_threads
        .iter()
        .all(|thread| thread["category_id"] != initial_category_id));
    let next_active_thread_id = next_bank["active_thread_id"]
        .as_str()
        .expect("next active thread id should be present");
    assert!(next_banked_threads.iter().any(|thread| {
        thread["category_id"]
            .as_str()
            .map(|id| id == next_active_thread_id)
            .unwrap_or(false)
    }));

    let after = wait_for_session(&state, session_id, Duration::from_secs(3), |session| {
        session
            .checkpoint
            .as_ref()
            .map(|checkpoint| {
                checkpoint.current_prompt.is_none()
                    && checkpoint.initial_prompt_bank_complete
                    && !checkpoint.prompt_bank.is_empty()
                    && checkpoint
                        .prompt_bank
                        .iter()
                        .all(|entry| entry.category_id != initial_category_id)
            })
            .unwrap_or(false)
    })
    .await;
    assert_eq!(after.intake_phase, "interviewing");
    assert!(after.interview_live_attached);

    let response = test_app(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}/prompt-bank", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let mirrored_bank: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(mirrored_bank["active_thread_id"], next_active_thread_id);
    assert!(!mirrored_bank["banked_threads"]
        .as_array()
        .unwrap()
        .is_empty());

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

#[tokio::test]
async fn tier2_socratic_ws_resume_answer_transitions_to_build_ready_state() {
    use planner_schemas::Dimension;

    let state = test_state_with_router(LlmRouter::with_mock(Box::new(ResumeFlowMockLlm)));
    let session = state.sessions.create("dev|local");
    let session_id = session.id;

    state.sessions.update(session_id, |s| {
        s.intake_phase = "interviewing".into();
        s.project_description = Some("Build a timer app".into());
        let checkpoint = s.ensure_checkpoint();
        checkpoint.belief_state = Some(build_resume_belief_state(
            vec![Dimension::Goal],
            vec![Dimension::Goal],
        ));
        checkpoint.prompt_bank = vec![banked_checkpoint_prompt(
            "goal-thread",
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        )];
        checkpoint.active_thread_id = Some("goal-thread".into());
        checkpoint.initial_prompt_bank_complete = true;
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws, _) = connect_async(ws_url).await.unwrap();
    let replayed_bank = wait_for_ws_message_type(&mut ws, "prompt_bank").await;
    let replay_prompt = &replayed_bank["bank"]["banked_threads"][0]["prompt"];
    let prompt_id = replay_prompt["prompt_id"]
        .as_str()
        .expect("replayed prompt id should be present");
    let item_id = replay_prompt["items"][0]["item_id"]
        .as_str()
        .expect("replayed prompt item id should be present");

    ws.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "It should guide one person through a clean workout countdown."
            }],
            "submitted_at": "2026-03-26T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    let build_ready_message = wait_for_ws_prompt_bank_where(&mut ws, |message| {
        message["bank"]["build_ready"].as_bool().unwrap_or(false)
    })
    .await;
    let build_ready_bank = &build_ready_message["bank"];
    assert!(build_ready_bank["build_ready"].as_bool().unwrap_or(false));
    assert!(build_ready_bank["banked_threads"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(build_ready_bank["queued_threads"]
        .as_array()
        .unwrap()
        .is_empty());

    let _after = wait_for_session(&state, session_id, Duration::from_secs(3), |session| {
        session
            .checkpoint
            .as_ref()
            .map(|checkpoint| {
                checkpoint
                    .current_category_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.build_ready && checkpoint.prompt_bank.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    })
    .await;

    let response = test_app(state.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/sessions/{}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        parsed["session"]["workspace_status"]["state"],
        "build_ready"
    );

    let _ = ws.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 13: Disconnecting from a live interview runtime keeps the runtime
/// available for fast reattach within the lease window.
#[tokio::test]
async fn tier2_socratic_ws_live_runtime_reattach_within_lease() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let resumed_prompt = wait_for_ws_answerable_prompt_message(&mut ws1).await;
    let resumed_prompt_payload = prompt_payload(&resumed_prompt);
    assert_eq!(
        resumed_prompt_payload["items"][0]["text"],
        EXPECTED_RESUME_BANK_PROMPT
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
    let replayed_prompt = wait_for_ws_answerable_prompt_message(&mut ws2).await;
    let replayed_prompt_payload = prompt_payload(&replayed_prompt);
    let prompt_id = replayed_prompt_payload["prompt_id"]
        .as_str()
        .expect("prompt id should be present");
    let item_id = replayed_prompt_payload["items"][0]["item_id"]
        .as_str()
        .expect("prompt item id should be present");

    ws2.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "I want a countdown timer for workouts."
            }],
            "submitted_at": "2026-03-08T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let attached_again = state.sessions.get(session_id).unwrap();
    assert_eq!(attached_again.intake_phase, "interviewing");
    assert!(attached_again.interview_runtime_active);
    let checkpoint = attached_again
        .checkpoint
        .as_ref()
        .expect("checkpoint should remain present after live reattach");
    let goal = checkpoint
        .belief_state
        .as_ref()
        .and_then(|belief| belief.filled.get(&Dimension::Goal))
        .map(|slot| slot.value.clone());
    assert_eq!(
        goal.as_deref(),
        Some("Build a countdown timer for workouts")
    );

    let _ = ws2.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 13b: Reconnect-heavy attach/detach cycles keep prompt replay stable
/// until an answer is submitted.
#[tokio::test]
async fn tier2_socratic_ws_reconnect_heavy_cycles_keep_prompt_replay_stable() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let mut expected_prompt_id: Option<String> = None;
    let mut expected_item_id: Option<String> = None;
    for _ in 0..8 {
        let (mut ws, _) = connect_async(&ws_url).await.unwrap();
        let prompt = wait_for_ws_answerable_prompt_message(&mut ws).await;
        let prompt_payload = prompt_payload(&prompt);
        assert_eq!(
            prompt_payload["items"][0]["text"],
            EXPECTED_RESUME_BANK_PROMPT
        );
        let prompt_id = prompt_payload["prompt_id"]
            .as_str()
            .expect("prompt id should be present")
            .to_string();
        let item_id = prompt_payload["items"][0]["item_id"]
            .as_str()
            .expect("prompt item id should be present")
            .to_string();
        if let Some(expected) = expected_prompt_id.as_ref() {
            assert_eq!(prompt_id, *expected);
        } else {
            expected_prompt_id = Some(prompt_id);
        }
        if let Some(expected) = expected_item_id.as_ref() {
            assert_eq!(item_id, *expected);
        } else {
            expected_item_id = Some(item_id);
        }
        let _ = ws.close(None).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        let detached = state.sessions.get(session_id).unwrap();
        assert_eq!(detached.resume_status, ResumeStatus::LiveAttachAvailable);
        assert!(detached.can_resume_live);
    }

    let (mut ws3, _) = connect_async(&ws_url).await.unwrap();
    let prompt = wait_for_ws_answerable_prompt_message(&mut ws3).await;
    let prompt_payload = prompt_payload(&prompt);
    let prompt_id = prompt_payload["prompt_id"]
        .as_str()
        .expect("prompt id should be present");
    let item_id = prompt_payload["items"][0]["item_id"]
        .as_str()
        .expect("prompt item id should be present");
    assert_eq!(Some(prompt_id.to_string()), expected_prompt_id);
    assert_eq!(Some(item_id.to_string()), expected_item_id);

    ws3.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "I want a countdown timer for workouts."
            }],
            "submitted_at": "2026-03-08T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;
    let after = state.sessions.get(session_id).unwrap();
    let checkpoint = after
        .checkpoint
        .as_ref()
        .expect("checkpoint should remain present");
    let goal = checkpoint
        .belief_state
        .as_ref()
        .and_then(|belief| belief.filled.get(&Dimension::Goal))
        .map(|slot| slot.value.clone());
    assert_eq!(
        goal.as_deref(),
        Some("Build a countdown timer for workouts")
    );

    let _ = ws3.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 14: When the live runtime lease expires, the session falls back to
/// checkpoint-only resume.
#[tokio::test]
async fn tier2_socratic_ws_live_runtime_lease_expiry_falls_back_to_checkpoint() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let _ = wait_for_ws_answerable_prompt_message(&mut ws1).await;
    let _ = ws1.close(None).await;

    let _detached = wait_for_session(&state, session_id, Duration::from_secs(2), |session| {
        session.intake_phase == "interviewing"
            && !session.interview_live_attached
            && session.resume_status == ResumeStatus::LiveAttachAvailable
    })
    .await;

    tokio::time::sleep(Duration::from_millis(75)).await;
    ws_socratic::expire_detached_runtimes(&state);

    let fallback = wait_for_session(&state, session_id, Duration::from_secs(2), |session| {
        session.resume_status == ResumeStatus::InterviewCheckpointResumable
            && !session.can_resume_live
            && session.can_resume_checkpoint
            && !session.interview_live_attached
    })
    .await;
    assert_eq!(
        fallback.resume_status,
        ResumeStatus::InterviewCheckpointResumable
    );
    assert!(!fallback.can_resume_live);
    assert!(fallback.can_resume_checkpoint);
    assert!(!fallback.interview_live_attached);

    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

/// Test 15: A second websocket cannot steal an actively attached live
/// interview runtime from the current client.
#[tokio::test]
async fn tier2_socratic_ws_duplicate_live_attach_is_rejected() {
    use planner_schemas::{
        ComplexityTier, Dimension, DomainClassification, ProjectType, RequirementsBeliefState,
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
        checkpoint.current_prompt = Some(checkpoint_question_prompt(
            "What is the main goal of this tool?",
            Dimension::Goal,
            false,
        ));
        checkpoint.touch();
    });

    let app = test_app(state.clone());
    let (addr, handle) = spawn_test_server(app).await;
    let ws_url = format!("ws://{}/sessions/{}/socratic/ws", addr, session_id);

    let (mut ws1, _) = connect_async(&ws_url).await.unwrap();
    let prompt = wait_for_ws_answerable_prompt_message(&mut ws1).await;
    let prompt_payload = prompt_payload(&prompt);
    let prompt_id = prompt_payload["prompt_id"]
        .as_str()
        .expect("prompt id should be present")
        .to_string();
    let item_id = prompt_payload["items"][0]["item_id"]
        .as_str()
        .expect("prompt item id should be present")
        .to_string();

    let (mut ws2, _) = connect_async(&ws_url).await.unwrap();
    let error = wait_for_ws_message_type(&mut ws2, "error").await;
    assert!(error["message"]
        .as_str()
        .unwrap_or("")
        .contains("already attached"));

    ws1.send(Message::Text(
        serde_json::json!({
            "type": "prompt_response",
            "prompt_id": prompt_id,
            "answers": [{
                "item_id": item_id,
                "custom_text": "I want a countdown timer for workouts."
            }],
            "submitted_at": "2026-03-08T00:00:00Z"
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let after = state.sessions.get(session_id).unwrap();
    let checkpoint = after
        .checkpoint
        .as_ref()
        .expect("checkpoint should remain present");
    let goal = checkpoint
        .belief_state
        .as_ref()
        .and_then(|belief| belief.filled.get(&Dimension::Goal))
        .map(|slot| slot.value.clone());
    assert_eq!(
        goal.as_deref(),
        Some("Build a countdown timer for workouts")
    );

    let _ = ws2.close(None).await;
    let _ = ws1.close(None).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}

#[tokio::test]
async fn tier2_archive_project() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Archive Integration",
        None,
        None,
        Vec::new(),
        None,
    );
    let app = test_app(state);

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
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["project"]["archived_at"].is_string());
}

#[tokio::test]
async fn tier2_unarchive_project() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Unarchive Integration",
        None,
        None,
        Vec::new(),
        None,
    );
    let _ = state.projects.set_archived(project.id, true).unwrap();
    let app = test_app(state);

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
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["project"]["archived_at"].is_null());
}

#[tokio::test]
async fn tier2_delete_project_with_no_sessions() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Delete No Sessions",
        None,
        None,
        Vec::new(),
        None,
    );
    let app = test_app(state.clone());

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(state.projects.resolve_ref(&project.slug).is_none());
}

#[tokio::test]
async fn tier2_delete_project_with_multiple_sessions() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Delete Multi Sessions",
        None,
        None,
        Vec::new(),
        None,
    );
    let s1 = state.sessions.create("dev|local");
    let s2 = state.sessions.create("dev|local");
    state
        .sessions
        .update(s1.id, |s| s.project_id = Some(project.id));
    state
        .sessions
        .update(s2.id, |s| s.project_id = Some(project.id));
    let app = test_app(state.clone());

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(state.sessions.get(s1.id).is_none());
    assert!(state.sessions.get(s2.id).is_none());
}

#[tokio::test]
async fn tier2_delete_project_with_active_pipeline_work() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Delete Active Pipeline",
        None,
        None,
        Vec::new(),
        None,
    );
    let session = state.sessions.create("dev|local");
    state.sessions.update(session.id, |s| {
        s.project_id = Some(project.id);
        s.project_slug = Some(project.slug.clone());
        s.project_name = Some(project.name.clone());
        s.pipeline_running = true;
        s.intake_phase = "pipeline_running".into();
        s.project_description = Some("Active pipeline".into());
    });
    let app = test_app(state.clone());

    // Transition through the public API path that starts pipeline runtime work.
    state.sessions.update(session.id, |s| {
        s.pipeline_running = false;
        s.intake_phase = "waiting".into();
    });
    let start_req = Request::builder()
        .method("POST")
        .uri(format!("/sessions/{}/message", session.id))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "content": "Active pipeline" }).to_string(),
        ))
        .unwrap();
    let start_resp = app.clone().oneshot(start_req).await.unwrap();
    assert_eq!(start_resp.status(), StatusCode::OK);
    assert!(state.pipeline_runtimes.get(session.id).is_some());

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(state.pipeline_runtimes.get(session.id).is_none());
}

#[tokio::test]
async fn tier2_delete_project_with_active_interview_runtime() {
    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Delete Active Interview",
        None,
        None,
        Vec::new(),
        None,
    );
    let session = state.sessions.create("dev|local");
    state.sessions.update(session.id, |s| {
        s.project_id = Some(project.id);
        s.project_slug = Some(project.slug.clone());
        s.project_name = Some(project.name.clone());
        s.project_description = Some("Active interview".into());
        s.intake_phase = "interviewing".into();
        s.interview_runtime_active = true;
        s.interview_live_attached = true;
    });
    let (runtime, _input_rx) = planner_server::runtime::SessionRuntime::new();
    assert!(state
        .socratic_runtimes
        .try_insert(session.id, runtime)
        .is_ok());
    assert!(state.socratic_runtimes.get(session.id).is_some());
    let app = test_app(state.clone());

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
    let summary: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(summary["stopped_live_sessions"], 1);
    assert!(state.socratic_runtimes.get(session.id).is_none());
}

#[tokio::test]
async fn tier2_delete_project_removes_event_files() {
    let data_dir = std::env::temp_dir().join(format!("planner_tier2_events_{}", Uuid::new_v4()));
    let state = Arc::new(AppState {
        sessions: SessionStore::new(),
        auth_config: None,
        event_store: Some(planner_core::observability::EventStore::new(&data_dir).unwrap()),
        cxdb: None,
        llm_router: Arc::new(LlmRouter::from_env()),
        socratic_runtimes: planner_server::runtime::SessionRuntimeRegistry::new(
            Duration::from_secs(30),
        ),
        pipeline_runtimes: planner_server::runtime::SessionPipelineRegistry::new(),
        started_at: std::time::Instant::now(),
        blueprints: planner_core::blueprint::BlueprintStore::new(),
        proposals: planner_core::discovery::ProposalStore::new(),
        projects: planner_server::project::ProjectStore::new(),
        imports: planner_server::import::ProjectImportStore::new(),
        import_acquirer: planner_server::import::default_import_acquirer(),
        import_analyzer: planner_server::import::default_import_analyzer(),
    });

    let project = state.projects.create(
        "dev|local",
        "Delete Event Files",
        None,
        None,
        Vec::new(),
        None,
    );
    let session = state.sessions.create("dev|local");
    state
        .sessions
        .update(session.id, |s| s.project_id = Some(project.id));
    let event_store = state.event_store.as_ref().unwrap();
    let events = vec![planner_core::observability::PlannerEvent::info(
        planner_core::observability::EventSource::System,
        "tier2.delete",
        "remove events",
    )];
    event_store
        .save_session_events(session.id, &events)
        .unwrap();
    let event_path = data_dir
        .join("events")
        .join(format!("{}.msgpack", session.id));
    assert!(event_path.exists());
    let app = test_app(state.clone());

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(!event_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn tier2_delete_project_removes_cxdb_data() {
    let data_dir = std::env::temp_dir().join(format!("planner_tier2_cxdb_{}", Uuid::new_v4()));
    let cxdb = planner_core::cxdb::durable::DurableCxdbEngine::open(data_dir.join("cxdb")).unwrap();
    let state = Arc::new(AppState {
        sessions: SessionStore::new(),
        auth_config: None,
        event_store: None,
        cxdb: Some(cxdb),
        llm_router: Arc::new(LlmRouter::from_env()),
        socratic_runtimes: planner_server::runtime::SessionRuntimeRegistry::new(
            Duration::from_secs(30),
        ),
        pipeline_runtimes: planner_server::runtime::SessionPipelineRegistry::new(),
        started_at: std::time::Instant::now(),
        blueprints: planner_core::blueprint::BlueprintStore::new(),
        proposals: planner_core::discovery::ProposalStore::new(),
        projects: planner_server::project::ProjectStore::new(),
        imports: planner_server::import::ProjectImportStore::new(),
        import_acquirer: planner_server::import::default_import_acquirer(),
        import_analyzer: planner_server::import::default_import_analyzer(),
    });

    let project = state
        .projects
        .create("dev|local", "Delete CXDB", None, None, Vec::new(), None);
    let other_project =
        state
            .projects
            .create("dev|local", "Other CXDB", None, None, Vec::new(), None);
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
    let app = test_app(state.clone());

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
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
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn tier2_delete_project_local_and_shared_blueprint_behavior() {
    use planner_schemas::artifacts::blueprint::{
        BlueprintNode, Decision, DecisionStatus, NodeId, NodeLifecycle, NodeScope, ProjectScope,
        ScopeClass, SecondaryScopeRefs, SharedScope,
    };

    let state = test_state();
    let project = state.projects.create(
        "dev|local",
        "Delete Blueprint",
        None,
        None,
        Vec::new(),
        None,
    );
    let other_project =
        state
            .projects
            .create("dev|local", "Keep Blueprint", None, None, Vec::new(), None);

    state
        .blueprints
        .upsert_node(BlueprintNode::Decision(Decision {
            id: NodeId::from_raw("dec-tier2-local"),
            title: "Local node".into(),
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
            id: NodeId::from_raw("dec-tier2-shared"),
            title: "Shared node".into(),
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
                    linked_project_ids: vec![project.id.to_string(), other_project.id.to_string()],
                    inherit_to_linked_projects: true,
                }),
                lifecycle: NodeLifecycle::Active,
                override_scope: None,
                scope_review: None,
            },
            created_at: "2026-03-08T00:00:00Z".into(),
            updated_at: "2026-03-08T00:00:00Z".into(),
        }));

    let app = test_app(state.clone());
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(state.blueprints.get_node("dec-tier2-local").is_none());
    let shared = state.blueprints.get_node("dec-tier2-shared").unwrap();
    let linked = shared
        .scope()
        .shared
        .as_ref()
        .map(|scope| scope.linked_project_ids.clone())
        .unwrap_or_default();
    assert_eq!(linked, vec![other_project.id.to_string()]);
}

#[tokio::test]
async fn tier2_delete_project_forbidden_for_non_owner() {
    let state = test_state();
    let project =
        state
            .projects
            .create("other_user|123", "Not Owned", None, None, Vec::new(), None);
    let app = test_app(state);

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/projects/{}", project.slug))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tier2_delete_project_not_found() {
    let state = test_state();
    let app = test_app(state);

    let req = Request::builder()
        .method("DELETE")
        .uri("/projects/missing-project")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
