//! # Domain Classifier — First-Message Classification
//!
//! Classifies the project type and complexity from the user's first message.
//! Single LLM call with structured output. Determines interview depth
//! and which question templates load.

use planner_schemas::*;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use super::super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// System Prompt
// ---------------------------------------------------------------------------

const CLASSIFY_SYSTEM_PROMPT: &str = r#"You are a project classifier for a Socratic requirements elicitation system.

Given the user's initial description of what they want to build, classify it into:

1. **project_type**: One of: cli_tool, web_app, api_backend, data_pipeline, mobile_app, library_crate, hybrid
2. **complexity**: One of: light, standard, deep
   - light: CLI, script, prototype, single-file tool (~200 lines)
   - standard: Web app, API, multi-user system
   - deep: Distributed, regulated, multi-tenant, real-time
3. **detected_signals**: List of specific words/phrases from the description that drove your classification

Respond with ONLY a JSON object (no markdown fences):
{
  "project_type": "web_app",
  "complexity": "standard",
  "detected_signals": ["web", "users", "dashboard"]
}

Signals to look for:
- Role mentions (admin, user, customer) → multi-user → standard+
- Integration mentions (Stripe, Auth0, S3) → standard+
- Regulatory terms (HIPAA, GDPR, SOX) → deep
- Real-time/distributed terms (websocket, streaming, queue) → deep
- Simplicity indicators (simple, quick, script, prototype) → light
- CLI indicators (command-line, terminal, flags, args) → cli_tool"#;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Classify a user's project description.
///
/// Returns a DomainClassification with project type, complexity tier,
/// question budget, and required dimensions.
pub async fn classify_domain(
    router: &LlmRouter,
    user_description: &str,
) -> StepResult<DomainClassification> {
    let request = CompletionRequest {
        system: Some(CLASSIFY_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: user_description.to_string(),
        }],
        max_tokens: 1024,
        temperature: 0.2,
        model: DefaultModels::INTAKE_GATEWAY.to_string(),
    };

    let response = router.complete(request).await?;
    parse_classification(&response.content)
}

// ---------------------------------------------------------------------------
// Response Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ClassifyJson {
    project_type: String,
    complexity: String,
    detected_signals: Vec<String>,
}

fn parse_classification(content: &str) -> StepResult<DomainClassification> {
    let cleaned = crate::pipeline::steps::intake::strip_code_fences(content);
    let json: ClassifyJson = serde_json::from_str(&cleaned)
        .or_else(|_| {
            // Try JSON repair
            let repaired = crate::llm::json_repair::try_repair_json(content)
                .unwrap_or_else(|| cleaned.clone());
            serde_json::from_str(&repaired)
        })
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse domain classification: {}. Raw: {}",
            e, &content[..content.len().min(300)]
        )))?;

    let project_type = match json.project_type.as_str() {
        "cli_tool" => ProjectType::CliTool,
        "web_app" => ProjectType::WebApp,
        "api_backend" => ProjectType::ApiBackend,
        "data_pipeline" => ProjectType::DataPipeline,
        "mobile_app" => ProjectType::MobileApp,
        "library_crate" => ProjectType::LibraryCrate,
        _ => ProjectType::Hybrid,
    };

    let complexity = match json.complexity.as_str() {
        "light" => ComplexityTier::Light,
        "standard" => ComplexityTier::Standard,
        "deep" => ComplexityTier::Deep,
        _ => ComplexityTier::Standard,
    };

    let required_dimensions = Dimension::required_for(&project_type);

    Ok(DomainClassification {
        project_type,
        complexity,
        detected_signals: json.detected_signals,
        required_dimensions,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_web_app_classification() {
        let json = r#"{"project_type":"web_app","complexity":"standard","detected_signals":["dashboard","users","login"]}"#;
        let result = parse_classification(json).unwrap();
        assert_eq!(result.project_type, ProjectType::WebApp);
        assert_eq!(result.complexity, ComplexityTier::Standard);
        assert!(result.required_dimensions.contains(&Dimension::Auth));
    }

    #[test]
    fn parse_cli_tool_classification() {
        let json = r#"{"project_type":"cli_tool","complexity":"light","detected_signals":["command-line","csv","parse"]}"#;
        let result = parse_classification(json).unwrap();
        assert_eq!(result.project_type, ProjectType::CliTool);
        assert_eq!(result.complexity, ComplexityTier::Light);
    }

    #[test]
    fn parse_with_code_fences() {
        let json = "```json\n{\"project_type\":\"api_backend\",\"complexity\":\"deep\",\"detected_signals\":[\"HIPAA\",\"microservices\"]}\n```";
        let result = parse_classification(json).unwrap();
        assert_eq!(result.project_type, ProjectType::ApiBackend);
        assert_eq!(result.complexity, ComplexityTier::Deep);
    }

    #[test]
    fn parse_unknown_type_defaults_to_hybrid() {
        let json = r#"{"project_type":"unknown_thing","complexity":"standard","detected_signals":[]}"#;
        let result = parse_classification(json).unwrap();
        assert_eq!(result.project_type, ProjectType::Hybrid);
    }
}
