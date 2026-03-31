use async_trait::async_trait;
use planner_core::llm::providers::LlmRouter;
use planner_core::llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
use planner_core::pipeline::steps::factory_worker::{
    CodexFactoryWorker, FactoryWorker, MockFactoryWorker,
};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const RUNTIME_MOCK_ENV: &str = "PLANNER_LLM_MOCK";
const LEGACY_RUNTIME_MOCK_ENV: &str = "PLANNER_E2E_LLM_MOCK";
const PHASE26_LIVE_MODE: &str = "phase26_live";
const FULL_PIPELINE_MODE: &str = "full_pipeline";
const ANSWER_MARKER: &str = "## Prompt Answers To Adjudicate:\n";
const PHASE28_FAIL_ONCE_MARKER: &str = "[phase28-fail-once]";
const PHASE28_SLOW_STARTUP_MARKER: &str = "[phase28-slow-startup]";

static PHASE28_FAIL_ONCE_TRIGGERED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerMockMode {
    Disabled,
    SocraticOnly,
    FullPipeline,
}

impl PlannerMockMode {
    fn from_raw(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "0" | "false" | "off" | "disabled" | "none" => Self::Disabled,
            PHASE26_LIVE_MODE => Self::SocraticOnly,
            FULL_PIPELINE_MODE | "full" | "builder" => Self::FullPipeline,
            _ => Self::Disabled,
        }
    }
}

pub fn mock_mode_from_env() -> PlannerMockMode {
    std::env::var(RUNTIME_MOCK_ENV)
        .ok()
        .or_else(|| std::env::var(LEGACY_RUNTIME_MOCK_ENV).ok())
        .map(|value| PlannerMockMode::from_raw(&value))
        .unwrap_or(PlannerMockMode::Disabled)
}

pub fn router_from_env_or_default() -> LlmRouter {
    match mock_mode_from_env() {
        PlannerMockMode::SocraticOnly => {
            tracing::info!("Using Planner runtime mock LLM mode: {}", PHASE26_LIVE_MODE);
            LlmRouter::with_mock(Box::new(PlannerRuntimeMockLlm {
                mode: PlannerMockMode::SocraticOnly,
            }))
        }
        PlannerMockMode::FullPipeline => {
            tracing::info!(
                "Using Planner runtime mock LLM mode: {}",
                FULL_PIPELINE_MODE
            );
            LlmRouter::with_mock(Box::new(PlannerRuntimeMockLlm {
                mode: PlannerMockMode::FullPipeline,
            }))
        }
        PlannerMockMode::Disabled => LlmRouter::from_env(),
    }
}

pub fn factory_worker_from_env_or_default() -> Result<Box<dyn FactoryWorker>, String> {
    match mock_mode_from_env() {
        PlannerMockMode::FullPipeline => Ok(Box::new(MockFactoryWorker::success(
            "Mock factory worker completed deterministic implementation output.",
            vec![
                "src/mock-countdown-timer.tsx".into(),
                "src/mock-app.tsx".into(),
            ],
        ))),
        PlannerMockMode::Disabled | PlannerMockMode::SocraticOnly => CodexFactoryWorker::new()
            .map(|worker| Box::new(worker) as Box<dyn FactoryWorker>)
            .map_err(|error| error.to_string()),
    }
}

struct PlannerRuntimeMockLlm {
    mode: PlannerMockMode,
}

#[async_trait]
impl LlmClient for PlannerRuntimeMockLlm {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let system = request.system.as_deref().unwrap_or("");
        maybe_delay_startup(&request).await;
        maybe_fail_startup_once(&request)?;
        let content = if let Some(content) = try_socratic_mock_response(system, &request)? {
            content
        } else if self.mode == PlannerMockMode::FullPipeline {
            full_pipeline_mock_response(system)
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
        match self.mode {
            PlannerMockMode::SocraticOnly => "phase26-live-mock",
            PlannerMockMode::FullPipeline => "planner-full-pipeline-mock",
            PlannerMockMode::Disabled => "live",
        }
    }
}

fn try_socratic_mock_response(
    system: &str,
    request: &CompletionRequest,
) -> Result<Option<String>, LlmError> {
    if system.contains("project classifier for a Socratic requirements elicitation system") {
        return Ok(Some(
            serde_json::json!({
                "project_type": "cli_tool",
                "complexity": "light",
                "detected_signals": ["cli", "timer", "workout"]
            })
            .to_string(),
        ));
    }

    if system.contains("Belief State Verifier") {
        return Ok(Some(
            serde_json::json!({
                "filled_updates": [],
                "uncertain_updates": [],
                "out_of_scope": [],
                "contradictions": [],
                "expertise_level": "intermediate",
                "user_wants_to_stop": false
            })
            .to_string(),
        ));
    }

    if system.contains("Belief State Adjudicator") {
        return Ok(Some(mock_adjudication_response(request)?));
    }

    if system.contains("Generate ONE focused question about the target dimension") {
        let dimension = request
            .messages
            .last()
            .and_then(|message| extract_target_dimension_label(&message.content))
            .unwrap_or_else(|| "core features".into());
        return Ok(Some(mock_question_response_for_dimension(&dimension)));
    }

    Ok(None)
}

fn full_pipeline_mock_response(system: &str) -> String {
    if system.contains("Intake Gateway") {
        mock_intake_json()
    } else if system.contains("Spec Compiler") && !system.contains("Domain") {
        mock_spec_json()
    } else if system.contains("Domain Spec Compiler") {
        mock_domain_spec_json()
    } else if system.contains("Graph.dot Compiler") {
        mock_graph_dot_json()
    } else if system.contains("Scenario Generator") {
        mock_scenario_json()
    } else if system.contains("AGENTS.md Compiler") {
        mock_agents_json()
    } else if system.contains("Adversarial Reviewer") {
        mock_ar_review_json()
    } else if system.contains("NLSpec Refiner") {
        mock_ar_refinement_json()
    } else if system.contains("Scenario Augmentation") || system.contains("Ralph") {
        mock_ralph_augment_json()
    } else if system.contains("Scenario Validator") || system.contains("evaluate whether") {
        mock_validate_json()
    } else if system.contains("Telemetry Presenter") {
        mock_telemetry_json()
    } else if system.contains("Chunk Planner") {
        mock_chunk_planner_json()
    } else if system.contains("DTU Configuration") || system.contains("dtu") {
        mock_dtu_config_json()
    } else {
        format!(
            r#"{{"fallback": true, "prompt_hint": "{}"}}"#,
            &system[..system.len().min(80)]
        )
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

fn mock_intake_json() -> String {
    r#"{
  "project_name": "Mock Timer",
  "feature_slug": "mock-timer-widget",
  "intent_summary": "A countdown timer widget",
  "output_domain": { "type": "micro_tool", "variant": "react_widget" },
  "environment": {
    "language": "TypeScript",
    "framework": "React",
    "package_manager": "npm",
    "build_tool": "vite"
  },
  "sacred_anchors": [
    { "id": "SA-1", "statement": "Timer must never display negative time", "rationale": "Core invariant" },
    { "id": "SA-2", "statement": "Pausing must preserve remaining time exactly", "rationale": "User expectation" }
  ],
  "satisfaction_criteria_seeds": [
    "Starting a 10-second timer shows it counting down to 0",
    "Pausing at 5 seconds and resuming continues from 5"
  ],
  "out_of_scope": ["Sound alerts", "Multiple timers"]
}"#
        .into()
}

fn mock_spec_json() -> String {
    r#"{
  "intent_summary": "A countdown timer widget with start, pause, and reset controls.",
  "sacred_anchors": [
    { "id": "SA-1", "statement": "Timer must never display negative time" },
    { "id": "SA-2", "statement": "Pausing must preserve remaining time exactly" }
  ],
  "requirements": [
    { "id": "FR-1", "statement": "The timer must accept a positive integer duration in seconds", "priority": "must", "traces_to": ["SA-1"] },
    { "id": "FR-2", "statement": "The display must update remaining time every second", "priority": "must", "traces_to": ["SA-1"] },
    { "id": "FR-3", "statement": "The widget must provide start, pause, and reset controls", "priority": "must", "traces_to": ["SA-2"] },
    { "id": "FR-4", "statement": "The timer must never display a negative value", "priority": "must", "traces_to": ["SA-1"] }
  ],
  "architectural_constraints": ["Single React component with hooks", "Tailwind CSS"],
  "phase1_contracts": [
    { "name": "TimerState", "type_definition": "{ duration: number, remaining: number, running: boolean }", "consumed_by": ["ui"] }
  ],
  "external_dependencies": [],
  "definition_of_done": [
    { "criterion": "Timer counts down from specified duration to zero", "mechanically_checkable": true },
    { "criterion": "Start/pause/reset buttons function correctly", "mechanically_checkable": true },
    { "criterion": "Timer never displays negative values", "mechanically_checkable": true }
  ],
  "satisfaction_criteria": [
    { "id": "SC-1", "description": "Starting a 10-second timer shows it counting down to 0", "tier_hint": "critical" },
    { "id": "SC-2", "description": "Pausing at 5 seconds and resuming continues from 5", "tier_hint": "critical" },
    { "id": "SC-3", "description": "Resetting returns to the original duration", "tier_hint": "high" }
  ],
  "open_questions": [],
  "out_of_scope": ["Sound alerts", "Multiple concurrent timers", "Persistent state"]
}"#
        .into()
}

fn mock_domain_spec_json() -> String {
    mock_spec_json()
}

fn mock_graph_dot_json() -> String {
    r#"{
  "dot_content": "digraph timer {\n  start [shape=Mdiamond];\n  exit [shape=Msquare];\n  implement [shape=box];\n  verify [shape=box];\n  start -> implement;\n  implement -> verify;\n  verify -> exit;\n}",
  "node_count": 4,
  "estimated_cost_usd": 0.50,
  "run_budget_usd": 2.00,
  "model_routing": [
    { "node_name": "implement", "node_class": "implementation", "model": "claude-sonnet-4-6", "fidelity": "truncate", "goal_gate": false, "max_retries": 2 }
  ]
}"#
        .into()
}

fn mock_scenario_json() -> String {
    r#"{
  "scenarios": [
    {
      "id": "SC-CRIT-1",
      "tier": "critical",
      "title": "Timer counts down to zero",
      "bdd_text": "Given a countdown timer set to 10 seconds\nWhen the user presses Start\nThen the display should count down from 10 to 0",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": "SC-1"
    },
    {
      "id": "SC-CRIT-2",
      "tier": "critical",
      "title": "Pause preserves remaining time",
      "bdd_text": "Given a running timer showing 5 seconds\nWhen the user presses Pause\nThen the display shows exactly 5 seconds",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-2"],
      "source_criterion": "SC-2"
    }
  ]
}"#
        .into()
}

fn mock_agents_json() -> String {
    serde_json::json!({
        "root_agents_md": "# AGENTS.md\n\n## Goal\nBuild a countdown timer widget.\n\n## Sacred Anchors\n- Timer must never display negative time\n- Pausing must preserve remaining time\n\n## Key Files\n- src/CountdownTimer.tsx\n- src/App.tsx\n\n## Constraints\n- Single React component\n- Tailwind CSS\n"
    })
    .to_string()
}

fn mock_ar_review_json() -> String {
    r#"{
  "findings": [
    {
      "severity": "advisory",
      "affected_section": "Definition of Done",
      "affected_requirements": [],
      "description": "DoD item 2 could be more specific about button behavior",
      "suggested_resolution": "Specify expected state after each button press"
    }
  ],
  "summary": "Spec is well-structured with minor advisory suggestions. No blocking issues found."
}"#
    .into()
}

fn mock_ar_refinement_json() -> String {
    r#"{
  "amendments": [],
  "open_questions": [],
  "amendment_log_entry": "No amendments needed — no blocking findings."
}"#
    .into()
}

fn mock_ralph_augment_json() -> String {
    r#"{
  "scenarios": [
    {
      "id": "SC-RALPH-1",
      "tier": "high",
      "title": "Edge case: zero-length timer",
      "bdd_text": "Given a countdown timer set to 0 seconds\nWhen the user presses Start\nThen the display should show 0 immediately",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": null
    }
  ]
}"#
        .into()
}

fn mock_validate_json() -> String {
    r#"{
  "score": 0.92,
  "passed": true,
  "reasoning": "The implementation satisfies the scenario requirements.",
  "error_category": null,
  "error_severity": null
}"#
    .into()
}

fn mock_telemetry_json() -> String {
    r#"{
  "headline": "Everything works as described.",
  "summary": "The countdown timer widget was built successfully. All scenarios pass validation.",
  "needs_user_action": false,
  "action_description": null
}"#
    .into()
}

fn mock_chunk_planner_json() -> String {
    r#"{
  "chunks": [
    {
      "chunk_id": "root",
      "relevant_anchor_ids": ["SA-1", "SA-2"],
      "domain_context": "Single-chunk micro-tool",
      "estimated_fr_count": 4
    }
  ]
}"#
    .into()
}

fn mock_dtu_config_json() -> String {
    r#"{
  "rules": [],
  "transitions": [],
  "seeds": [],
  "failure_modes": []
}"#
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap()
    }

    #[test]
    fn mock_mode_prefers_primary_env() {
        let _guard = env_guard();
        std::env::set_var(RUNTIME_MOCK_ENV, FULL_PIPELINE_MODE);
        std::env::set_var(LEGACY_RUNTIME_MOCK_ENV, PHASE26_LIVE_MODE);
        assert_eq!(mock_mode_from_env(), PlannerMockMode::FullPipeline);
        std::env::remove_var(RUNTIME_MOCK_ENV);
        std::env::remove_var(LEGACY_RUNTIME_MOCK_ENV);
    }

    #[test]
    fn full_pipeline_router_reports_mock_provider() {
        let _guard = env_guard();
        std::env::set_var(RUNTIME_MOCK_ENV, FULL_PIPELINE_MODE);
        let router = router_from_env_or_default();
        assert_eq!(router.available_providers(), vec!["mock"]);
        std::env::remove_var(RUNTIME_MOCK_ENV);
    }

    #[test]
    fn full_pipeline_worker_uses_mock_factory_worker() {
        let _guard = env_guard();
        std::env::set_var(RUNTIME_MOCK_ENV, FULL_PIPELINE_MODE);
        let worker = factory_worker_from_env_or_default().expect("mock worker");
        assert_eq!(worker.worker_name(), "mock-factory-worker");
        std::env::remove_var(RUNTIME_MOCK_ENV);
    }
}
