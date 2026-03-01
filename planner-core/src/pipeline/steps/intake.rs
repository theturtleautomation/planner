//! # Intake Gateway — Socratic Interview → IntakeV1
//!
//! Phase 0 implementation: Opus-driven multi-turn Socratic interview
//! that produces a structured IntakeV1 artifact. The interview:
//!
//! 1. Asks the user what they want to build
//! 2. Clarifies intent via targeted follow-up questions
//! 3. Validates the output domain (micro-tool only in Phase 0)
//! 4. Extracts Sacred Anchors, satisfaction criteria, and out-of-scope
//! 5. Produces a complete IntakeV1
//!
//! The LLM drives the conversation and extracts structured data.
//! In Phase 0, the interview is non-interactive (single prompt → structured output).
//! Phase 1+ adds true multi-turn with user IO.

use chrono::Utc;
use uuid::Uuid;

use crate::llm::{CompletionRequest, CompletionResponse, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// System prompt for the Intake Gateway
// ---------------------------------------------------------------------------

const INTAKE_SYSTEM_PROMPT: &str = r#"You are the Intake Gateway for Planner v2, a Socratic AI development partner.

Your job: take a user's plain-English description of what they want to build and produce a structured JSON intake document.

## Phase 0 Constraints
- Output domain MUST be a micro-tool: either a single-view React+Tailwind widget OR a single-file Python FastAPI backend
- Generated code should be ~200 lines max
- If the user's request is too complex, decompose it and pick the most valuable micro-tool piece

## Your Output
Respond with ONLY a JSON object (no markdown fences, no explanation) matching this schema:

{
  "project_name": "Human-readable name",
  "feature_slug": "kebab-case-slug",
  "intent_summary": "Plain-English description of what will be built",
  "output_domain": {
    "type": "micro_tool",
    "variant": "react_widget" | "fastapi_backend"
  },
  "environment": {
    "language": "TypeScript" | "Python",
    "framework": "React" | "FastAPI",
    "package_manager": "npm" | "pip" | null,
    "build_tool": "vite" | "uvicorn" | null
  },
  "sacred_anchors": [
    {
      "id": "SA-1",
      "statement": "Imperative constraint from user intent",
      "rationale": "Why this matters"
    }
  ],
  "satisfaction_criteria_seeds": [
    "Plain-English description of what 'working' means"
  ],
  "out_of_scope": [
    "Things explicitly excluded"
  ]
}

## Rules
1. Extract AT LEAST one Sacred Anchor from the user's description — the core thing that must not break
2. Extract AT LEAST one satisfaction criterion — what does "done" look like to the user?
3. Always include out_of_scope items to set clear boundaries
4. Sacred Anchor statements MUST use imperative language (must/must not/always/never)
5. If the request is ambiguous, make reasonable choices and document them in the intent_summary
6. project_name should be 2-4 words, human-friendly
7. feature_slug should be kebab-case, descriptive"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the Intake Gateway: user description → IntakeV1.
///
/// Phase 0: Single-shot LLM call (no multi-turn conversation).
/// The user_description is the raw text of what they want to build.
pub async fn execute_intake(
    router: &LlmRouter,
    project_id: Uuid,
    user_description: &str,
) -> StepResult<IntakeV1> {
    let request = CompletionRequest {
        system: Some(INTAKE_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_description.to_string(),
        }],
        max_tokens: 4096,
        temperature: 0.3, // Low temperature for structured output
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    parse_intake_response(project_id, user_description, &response)
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

/// Intermediate JSON structure for parsing LLM output.
#[derive(Debug, serde::Deserialize)]
struct IntakeJson {
    project_name: String,
    feature_slug: String,
    intent_summary: String,
    output_domain: OutputDomainJson,
    environment: EnvironmentJson,
    sacred_anchors: Vec<SacredAnchorJson>,
    satisfaction_criteria_seeds: Vec<String>,
    out_of_scope: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct OutputDomainJson {
    #[serde(rename = "type")]
    domain_type: String,
    variant: String,
}

#[derive(Debug, serde::Deserialize)]
struct EnvironmentJson {
    language: String,
    framework: String,
    package_manager: Option<String>,
    #[serde(default)]
    existing_dependencies: Option<Vec<String>>,
    build_tool: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SacredAnchorJson {
    id: String,
    statement: String,
    rationale: Option<String>,
}

fn parse_intake_response(
    project_id: Uuid,
    user_description: &str,
    response: &CompletionResponse,
) -> StepResult<IntakeV1> {
    // Strip markdown code fences if present, then try JSON repair
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| strip_code_fences(&response.content));

    let json: IntakeJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse Intake Gateway response: {}. Raw content: {}",
            e,
            &response.content[..response.content.len().min(500)]
        )))?;

    // Map output domain
    let output_domain = match (json.output_domain.domain_type.as_str(), json.output_domain.variant.as_str()) {
        ("micro_tool", "react_widget") => OutputDomain::MicroTool {
            variant: MicroToolVariant::ReactWidget,
        },
        ("micro_tool", "fastapi_backend") => OutputDomain::MicroTool {
            variant: MicroToolVariant::FastApiBackend,
        },
        _ => {
            // Phase 0 fallback: default to React widget
            OutputDomain::MicroTool {
                variant: MicroToolVariant::ReactWidget,
            }
        }
    };

    // Map environment
    let environment = EnvironmentInfo {
        language: json.environment.language,
        framework: json.environment.framework,
        package_manager: json.environment.package_manager,
        existing_dependencies: json.environment.existing_dependencies.unwrap_or_default(),
        build_tool: json.environment.build_tool,
    };

    // Map sacred anchors
    let sacred_anchors: Vec<SacredAnchor> = json.sacred_anchors.into_iter()
        .map(|a| SacredAnchor {
            id: a.id,
            statement: a.statement,
            rationale: a.rationale,
        })
        .collect();

    // Build conversation log (Phase 0: just the initial exchange)
    let now = Utc::now().to_rfc3339();
    let conversation_log = vec![
        ConversationTurn {
            role: "user".into(),
            content: user_description.to_string(),
            timestamp: now.clone(),
        },
        ConversationTurn {
            role: "system".into(),
            content: format!("[Intake Gateway produced structured output using {}]", response.model),
            timestamp: now,
        },
    ];

    Ok(IntakeV1 {
        project_id,
        project_name: json.project_name,
        feature_slug: json.feature_slug,
        intent_summary: json.intent_summary,
        output_domain,
        environment,
        sacred_anchors,
        satisfaction_criteria_seeds: json.satisfaction_criteria_seeds,
        out_of_scope: json.out_of_scope,
        conversation_log,
    })
}

/// Strip markdown code fences from LLM output.
/// Handles ```json ... ``` and ``` ... ``` patterns.
pub(crate) fn strip_code_fences(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("```") {
        let without_opening = if let Some(rest) = trimmed.strip_prefix("```json") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("```") {
            rest
        } else {
            trimmed
        };
        if let Some(content) = without_opening.strip_suffix("```") {
            return content.trim().to_string();
        }
        return without_opening.trim().to_string();
    }
    trimmed.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fences_json() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_code_fences_bare() {
        let input = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn strip_code_fences_none() {
        let input = "{\"key\": \"value\"}";
        assert_eq!(strip_code_fences(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn parse_valid_intake_json() {
        let response = CompletionResponse {
            content: r#"{
                "project_name": "Task Tracker",
                "feature_slug": "task-tracker-widget",
                "intent_summary": "A simple task tracking widget with add, complete, and delete",
                "output_domain": { "type": "micro_tool", "variant": "react_widget" },
                "environment": {
                    "language": "TypeScript",
                    "framework": "React",
                    "package_manager": "npm",
                    "build_tool": "vite"
                },
                "sacred_anchors": [
                    {
                        "id": "SA-1",
                        "statement": "User data must never be lost on page refresh",
                        "rationale": "Core user expectation for a task tracker"
                    }
                ],
                "satisfaction_criteria_seeds": [
                    "Adding a task and refreshing the page shows the task still present"
                ],
                "out_of_scope": ["Multi-user collaboration", "Cloud sync"]
            }"#.to_string(),
            model: "claude-opus-4-6".into(),
            input_tokens: 100,
            output_tokens: 200,
            estimated_cost_usd: 0.0,
        };

        let result = parse_intake_response(Uuid::new_v4(), "build me a task tracker", &response);
        assert!(result.is_ok());

        let intake = result.unwrap();
        assert_eq!(intake.project_name, "Task Tracker");
        assert_eq!(intake.feature_slug, "task-tracker-widget");
        assert_eq!(intake.sacred_anchors.len(), 1);
        assert_eq!(intake.sacred_anchors[0].id, "SA-1");
        assert!(matches!(intake.output_domain, OutputDomain::MicroTool { variant: MicroToolVariant::ReactWidget }));
        assert_eq!(intake.satisfaction_criteria_seeds.len(), 1);
        assert_eq!(intake.out_of_scope.len(), 2);
        assert_eq!(intake.conversation_log.len(), 2);
    }

    #[test]
    fn parse_intake_with_code_fences() {
        let response = CompletionResponse {
            content: "```json\n{\"project_name\":\"Timer\",\"feature_slug\":\"timer-widget\",\"intent_summary\":\"A countdown timer\",\"output_domain\":{\"type\":\"micro_tool\",\"variant\":\"react_widget\"},\"environment\":{\"language\":\"TypeScript\",\"framework\":\"React\",\"package_manager\":\"npm\",\"build_tool\":\"vite\"},\"sacred_anchors\":[{\"id\":\"SA-1\",\"statement\":\"Timer must never lose count while running\",\"rationale\":\"Core function\"}],\"satisfaction_criteria_seeds\":[\"Start a timer and it counts down\"],\"out_of_scope\":[\"Sound alerts\"]}\n```".into(),
            model: "claude-opus-4-6".into(),
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
        };

        let result = parse_intake_response(Uuid::new_v4(), "build a timer", &response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().project_name, "Timer");
    }

    #[test]
    fn parse_invalid_json_gives_useful_error() {
        let response = CompletionResponse {
            content: "I'd be happy to help! Here's what I think...".into(),
            model: "claude-opus-4-6".into(),
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
        };

        let result = parse_intake_response(Uuid::new_v4(), "test", &response);
        assert!(result.is_err());
        match result.unwrap_err() {
            StepError::JsonError(msg) => {
                assert!(msg.contains("Failed to parse Intake Gateway response"));
            }
            other => panic!("Expected JsonError, got: {:?}", other),
        }
    }
}
