use async_trait::async_trait;
use planner_core::llm::providers::LlmRouter;
use planner_core::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const PHASE26_LIVE_MODE: &str = "phase26_live";
const ANSWER_MARKER: &str = "## Prompt Answers To Adjudicate:\n";
const PHASE28_FAIL_ONCE_MARKER: &str = "[phase28-fail-once]";
const PHASE28_SLOW_STARTUP_MARKER: &str = "[phase28-slow-startup]";

static PHASE28_FAIL_ONCE_TRIGGERED: AtomicBool = AtomicBool::new(false);

pub fn router_from_env_or_default() -> LlmRouter {
    match std::env::var("PLANNER_E2E_LLM_MOCK") {
        Ok(value) if value.trim().eq_ignore_ascii_case(PHASE26_LIVE_MODE) => {
            tracing::info!(
                "Using Playwright E2E mock LLM router mode: {}",
                PHASE26_LIVE_MODE
            );
            LlmRouter::with_mock(Box::new(Phase26LiveMockLlm))
        }
        _ => LlmRouter::from_env(),
    }
}

struct Phase26LiveMockLlm;

#[async_trait]
impl LlmClient for Phase26LiveMockLlm {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let system = request.system.as_deref().unwrap_or("");
        maybe_delay_startup(&request).await;
        maybe_fail_startup_once(&request)?;
        let content = if system
            .contains("project classifier for a Socratic requirements elicitation system")
        {
            serde_json::json!({
                "project_type": "cli_tool",
                "complexity": "light",
                "detected_signals": ["cli", "timer", "workout"]
            })
            .to_string()
        } else if system.contains("Belief State Verifier") {
            serde_json::json!({
                "filled_updates": [],
                "uncertain_updates": [],
                "out_of_scope": [],
                "contradictions": [],
                "expertise_level": "intermediate",
                "user_wants_to_stop": false
            })
            .to_string()
        } else if system.contains("Belief State Adjudicator") {
            mock_adjudication_response(&request)?
        } else if system.contains("Generate ONE focused question about the target dimension") {
            let dimension = request
                .messages
                .last()
                .and_then(|message| extract_target_dimension_label(&message.content))
                .unwrap_or_else(|| "core features".into());
            mock_question_response_for_dimension(&dimension)
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
        "phase26-live-mock"
    }
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
            "question": "What is the main outcome this project needs to deliver first?",
            "quick_options": [],
            "allow_skip": false
        }),
        "success criteria" => serde_json::json!({
            "question": "How will you know the first release works?",
            "quick_options": [],
            "allow_skip": false
        }),
        "core features" => serde_json::json!({
            "question": "What capabilities must the first release include?",
            "quick_options": [],
            "allow_skip": false
        }),
        "error handling" => serde_json::json!({
            "question": "What failure cases must the first release handle cleanly?",
            "quick_options": [],
            "allow_skip": false
        }),
        "security" => serde_json::json!({
            "question": "What minimum security bar does the first release need?",
            "quick_options": [],
            "allow_skip": false
        }),
        "out of scope" => serde_json::json!({
            "question": "What should stay out of scope for the first release?",
            "quick_options": [],
            "allow_skip": true
        }),
        "platform" => serde_json::json!({
            "question": "Where should this tool run first?",
            "quick_options": [],
            "allow_skip": false
        }),
        "exit codes" => serde_json::json!({
            "question": "Which exit codes must the CLI expose in the first release?",
            "quick_options": [],
            "allow_skip": true
        }),
        "input formats" => serde_json::json!({
            "question": "What input formats should the first release accept?",
            "quick_options": [],
            "allow_skip": true
        }),
        _ => serde_json::json!({
            "question": "What capabilities must the first release include?",
            "quick_options": [],
            "allow_skip": false
        }),
    };

    payload.to_string()
}

fn mock_adjudication_response(request: &CompletionRequest) -> Result<String, LlmError> {
    let payload = request
        .messages
        .last()
        .and_then(|message| extract_batch_payload(&message.content))
        .ok_or_else(|| LlmError::Other("missing adjudication payload".into()))?;
    let parsed: Value = serde_json::from_str(payload)
        .map_err(|error| LlmError::Other(format!("invalid adjudication payload: {}", error)))?;
    let items = parsed["items"]
        .as_array()
        .ok_or_else(|| LlmError::Other("adjudication payload missing items".into()))?;

    let response_items = items
        .iter()
        .filter_map(|item| {
            let item_id = item["item_id"].as_str()?.to_string();
            let target_dimension = item["target_dimension"]
                .as_str()
                .and_then(parse_dimension_key);
            let user_value = item["custom_text"]
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    item["selected_option"]
                        .as_str()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| "Browser proof answer".into());
            let source_quote = user_value.clone();

            Some(match target_dimension.as_deref() {
                Some("out_of_scope") => serde_json::json!({
                    "item_id": item_id,
                    "filled_updates": [],
                    "uncertain_updates": [],
                    "out_of_scope": ["out_of_scope"],
                    "contradictions": [],
                    "user_wants_to_stop": false
                }),
                Some(dimension) => serde_json::json!({
                    "item_id": item_id,
                    "filled_updates": [{
                        "dimension": dimension,
                        "value": user_value,
                        "source_quote": source_quote
                    }],
                    "uncertain_updates": [],
                    "out_of_scope": [],
                    "contradictions": [],
                    "user_wants_to_stop": false
                }),
                None => serde_json::json!({
                    "item_id": item_id,
                    "filled_updates": [],
                    "uncertain_updates": [],
                    "out_of_scope": [],
                    "contradictions": [],
                    "user_wants_to_stop": false
                }),
            })
        })
        .collect::<Vec<_>>();

    Ok(serde_json::json!({ "items": response_items }).to_string())
}

fn extract_batch_payload(content: &str) -> Option<&str> {
    let start = content.find(ANSWER_MARKER)? + ANSWER_MARKER.len();
    let suffix = &content[start..];
    let end = suffix.find("\n\nReturn JSON now.")?;
    Some(&suffix[..end])
}

fn parse_dimension_key(raw: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(raw).ok()?;
    match parsed {
        Value::String(value) => Some(value),
        Value::Object(map) => map
            .get("custom")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase()),
        _ => None,
    }
}

fn request_contains_marker(request: &CompletionRequest, marker: &str) -> bool {
    let marker = marker.to_ascii_lowercase();
    request
        .system
        .iter()
        .chain(request.messages.iter().map(|message| &message.content))
        .any(|content| content.to_ascii_lowercase().contains(&marker))
}

async fn maybe_delay_startup(request: &CompletionRequest) {
    if request_contains_marker(request, PHASE28_SLOW_STARTUP_MARKER) {
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

fn maybe_fail_startup_once(request: &CompletionRequest) -> Result<(), LlmError> {
    if request_contains_marker(request, PHASE28_FAIL_ONCE_MARKER)
        && !PHASE28_FAIL_ONCE_TRIGGERED.swap(true, Ordering::SeqCst)
    {
        return Err(LlmError::Other(
            "phase 28 simulated startup failure before first reveal".into(),
        ));
    }

    Ok(())
}
