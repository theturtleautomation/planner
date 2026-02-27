//! # Compiler — IntakeV1 → NLSpecV1 → GraphDotV1 → ScenarioSetV1 → AgentsManifestV1
//!
//! The Compiler transforms raw intake data through a series of LLM-driven
//! compilation steps. Each step takes a structured artifact and produces
//! the next one in the pipeline.
//!
//! Phase 0: Single root NLSpec chunk, simplified graph.dot, critical-tier
//! scenarios only, single AGENTS.md (no domain docs).

use uuid::Uuid;

use crate::llm::{CompletionRequest, CompletionResponse, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};

// ===========================================================================
// IntakeV1 → NLSpecV1
// ===========================================================================

const SPEC_SYSTEM_PROMPT: &str = r#"You are the Spec Compiler for Planner v2. Your job: transform an IntakeV1 document into a structured NLSpecV1 (Progressive Specification).

## Output Format
Respond with ONLY a JSON object (no markdown fences, no explanation):

{
  "intent_summary": "Plain-English project description (from intake)",
  "sacred_anchors": [
    { "id": "SA-1", "statement": "Imperative constraint (must/must not/always/never)" }
  ],
  "requirements": [
    {
      "id": "FR-1",
      "statement": "The system must ... (imperative language required)",
      "priority": "Must" | "Should" | "Could",
      "traces_to": ["SA-1"]
    }
  ],
  "architectural_constraints": ["Single-file React component", ...],
  "phase1_contracts": [
    {
      "name": "TypeName",
      "type_definition": "{ field: type, ... }",
      "consumed_by": ["ui", "api"]
    }
  ],
  "external_dependencies": [],
  "definition_of_done": [
    { "criterion": "What must be true", "mechanically_checkable": true|false }
  ],
  "satisfaction_criteria": [
    {
      "id": "SC-1",
      "description": "Plain-English expected behavior",
      "tier_hint": "Critical" | "High" | "Medium"
    }
  ],
  "out_of_scope": ["Excluded items"],
  "open_questions": []
}

## Rules
1. Every Sacred Anchor from the intake MUST have at least one FR tracing to it
2. ALL FR statements MUST use imperative language: must, must not, always, never, shall
3. At least one Satisfaction Criterion MUST have tier_hint "Critical"
4. Phase 1 Contracts MUST define concrete types (TypeScript-style for React, Python-style for FastAPI)
5. Definition of Done items should be mechanically checkable where possible
6. Keep total content under 500 lines (this is a micro-tool spec)
7. Open Questions MUST be empty — resolve all ambiguities by making reasonable choices
8. Out of Scope MUST be non-empty — always set boundaries
9. For micro-tools: 3-8 FRs is typical, more signals over-scoping"#;

/// IntakeV1 → NLSpecV1 (single root chunk in Phase 0).
pub async fn compile_spec(
    router: &LlmRouter,
    intake: &IntakeV1,
) -> StepResult<NLSpecV1> {
    let intake_json = serde_json::to_string_pretty(intake)
        .map_err(|e| StepError::JsonError(e.to_string()))?;

    let request = CompletionRequest {
        system: Some(SPEC_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Compile this IntakeV1 into an NLSpecV1:\n\n{}",
                intake_json
            ),
        }],
        max_tokens: 8192,
        temperature: 0.2,
        model: DefaultModels::COMPILER_SPEC.to_string(),
    };

    let response = router.complete(request).await?;
    parse_spec_response(intake.project_id, &response)
}

#[derive(Debug, serde::Deserialize)]
struct SpecJson {
    intent_summary: String,
    sacred_anchors: Vec<SpecAnchorJson>,
    requirements: Vec<RequirementJson>,
    architectural_constraints: Vec<String>,
    phase1_contracts: Vec<Phase1ContractJson>,
    #[serde(default)]
    external_dependencies: Vec<ExtDepJson>,
    definition_of_done: Vec<DoDJson>,
    satisfaction_criteria: Vec<SatCritJson>,
    #[serde(default)]
    open_questions: Vec<OpenQuestionJson>,
    out_of_scope: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SpecAnchorJson { id: String, statement: String }

#[derive(Debug, serde::Deserialize)]
struct RequirementJson {
    id: String,
    statement: String,
    priority: String,
    traces_to: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Phase1ContractJson {
    name: String,
    type_definition: String,
    consumed_by: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ExtDepJson {
    name: String,
    #[serde(default = "default_dtu")]
    dtu_priority: String,
    #[serde(default)]
    usage_description: String,
}

fn default_dtu() -> String { "None".into() }

#[derive(Debug, serde::Deserialize)]
struct DoDJson { criterion: String, mechanically_checkable: bool }

#[derive(Debug, serde::Deserialize)]
struct SatCritJson { id: String, description: String, tier_hint: String }

#[derive(Debug, serde::Deserialize)]
struct OpenQuestionJson {
    question: String,
    #[serde(default = "default_raised_by")]
    raised_by: String,
    resolution: Option<String>,
}

fn default_raised_by() -> String { "compiler".into() }

fn parse_spec_response(
    project_id: Uuid,
    response: &CompletionResponse,
) -> StepResult<NLSpecV1> {
    let content = super::intake::strip_code_fences(&response.content);

    let json: SpecJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse Spec Compiler response: {}. Raw: {}",
            e, &response.content[..response.content.len().min(500)]
        )))?;

    // Estimate line count from content length
    let line_count = (content.len() / 60).min(500) as u32;

    let sacred_anchors: Vec<NLSpecAnchor> = json.sacred_anchors.into_iter()
        .map(|a| NLSpecAnchor { id: a.id, statement: a.statement })
        .collect();

    let requirements: Vec<Requirement> = json.requirements.into_iter()
        .map(|r| Requirement {
            id: r.id,
            statement: r.statement,
            priority: match r.priority.to_lowercase().as_str() {
                "must" => Priority::Must,
                "should" => Priority::Should,
                "could" => Priority::Could,
                _ => Priority::Must,
            },
            traces_to: r.traces_to,
        })
        .collect();

    let phase1_contracts: Vec<Phase1Contract> = json.phase1_contracts.into_iter()
        .map(|c| Phase1Contract {
            name: c.name,
            type_definition: c.type_definition,
            consumed_by: c.consumed_by,
        })
        .collect();

    let external_dependencies: Vec<ExternalDependency> = json.external_dependencies.into_iter()
        .map(|d| ExternalDependency {
            name: d.name,
            dtu_priority: match d.dtu_priority.to_lowercase().as_str() {
                "high" => DtuPriority::High,
                "medium" => DtuPriority::Medium,
                "low" => DtuPriority::Low,
                _ => DtuPriority::None,
            },
            usage_description: d.usage_description,
        })
        .collect();

    let definition_of_done: Vec<DoDItem> = json.definition_of_done.into_iter()
        .map(|d| DoDItem {
            criterion: d.criterion,
            mechanically_checkable: d.mechanically_checkable,
        })
        .collect();

    let satisfaction_criteria: Vec<SatisfactionCriterion> = json.satisfaction_criteria.into_iter()
        .map(|s| SatisfactionCriterion {
            id: s.id,
            description: s.description,
            tier_hint: match s.tier_hint.to_lowercase().as_str() {
                "critical" => ScenarioTierHint::Critical,
                "high" => ScenarioTierHint::High,
                _ => ScenarioTierHint::Medium,
            },
        })
        .collect();

    let open_questions: Vec<OpenQuestion> = json.open_questions.into_iter()
        .map(|q| OpenQuestion {
            question: q.question,
            raised_by: q.raised_by,
            resolution: q.resolution,
        })
        .collect();

    Ok(NLSpecV1 {
        project_id,
        version: "1.0".into(),
        chunk: ChunkType::Root,
        status: NLSpecStatus::Draft,
        line_count,
        created_from: format!("{}:{}", IntakeV1::TYPE_ID, project_id),
        intent_summary: Some(json.intent_summary),
        sacred_anchors: Some(sacred_anchors),
        requirements,
        architectural_constraints: json.architectural_constraints,
        phase1_contracts: Some(phase1_contracts),
        external_dependencies,
        definition_of_done,
        satisfaction_criteria,
        open_questions,
        out_of_scope: json.out_of_scope,
        amendment_log: vec![],
    })
}

// ===========================================================================
// NLSpecV1 → GraphDotV1
// ===========================================================================

const GRAPH_DOT_SYSTEM_PROMPT: &str = r#"You are the Graph.dot Compiler for Planner v2. Your job: transform an NLSpecV1 into a Kilroy Attractor-compatible DOT pipeline.

## Phase 0 Micro-Tool Pipeline
For micro-tools (~200 lines), generate a SIMPLIFIED pipeline (not the full reference template). The topology:

1. start → check_toolchain → expand_spec → implement → verify_build → verify_test → review → exit

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "dot_content": "digraph pipeline { ... }",
  "node_count": 6,
  "estimated_cost_usd": 0.50,
  "run_budget_usd": 2.00,
  "model_routing": [
    {
      "node_name": "implement",
      "node_class": "hard",
      "model": "claude-sonnet-4-6",
      "fidelity": "truncate",
      "goal_gate": false,
      "max_retries": 2
    }
  ]
}

## DOT Format Rules (Kilroy Attractor-compatible)
- `shape=Mdiamond` for start node
- `shape=Msquare` for exit node
- `shape=box` for LLM codergen nodes (need prompts)
- `shape=parallelogram` for tool/shell command nodes (need tool_command)
- `shape=diamond` for routing/decision nodes
- `model_stylesheet` in graph attributes for model assignment
- Edge conditions: `condition="outcome=success"`, `condition="outcome=fail"`
- Anthropic model IDs use DOTS in the stylesheet: `claude-opus-4.6`, `claude-sonnet-4.6`
- `goal_gate=true` on review consensus node
- `auto_status=true` on nodes that don't write explicit status
- `max_retries=N` for retry budget per node
- Default retry target: implement node
- Use `class="hard"` for the implementation node

## Key Constraints
- The DOT must be valid Graphviz
- Every shape=box node needs a prompt (composed by the ingestor, not by you)
- Every shape=parallelogram node needs a tool_command
- Use project-specific tool_commands derived from the NLSpec environment info
- Keep it simple for Phase 0 — no fan-out branches, just a linear pipeline with retry"#;

/// NLSpecV1 → GraphDotV1 (Attractor-compatible DOT pipeline).
pub async fn compile_graph_dot(
    router: &LlmRouter,
    spec: &NLSpecV1,
) -> StepResult<GraphDotV1> {
    let spec_json = serde_json::to_string_pretty(spec)
        .map_err(|e| StepError::JsonError(e.to_string()))?;

    let request = CompletionRequest {
        system: Some(GRAPH_DOT_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate a Kilroy Attractor DOT pipeline for this NLSpec:\n\n{}",
                spec_json
            ),
        }],
        max_tokens: 8192,
        temperature: 0.2,
        model: DefaultModels::COMPILER_GRAPH_DOT.to_string(),
    };

    let response = router.complete(request).await?;
    parse_graph_dot_response(spec.project_id, &spec.version, &response)
}

#[derive(Debug, serde::Deserialize)]
struct GraphDotJson {
    dot_content: String,
    node_count: u32,
    estimated_cost_usd: f32,
    run_budget_usd: f32,
    model_routing: Vec<NodeModelJson>,
}

#[derive(Debug, serde::Deserialize)]
struct NodeModelJson {
    node_name: String,
    node_class: String,
    model: String,
    fidelity: String,
    goal_gate: bool,
    max_retries: u32,
}

fn parse_graph_dot_response(
    project_id: Uuid,
    nlspec_version: &str,
    response: &CompletionResponse,
) -> StepResult<GraphDotV1> {
    let content = super::intake::strip_code_fences(&response.content);

    let json: GraphDotJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse graph.dot Compiler response: {}. Raw: {}",
            e, &response.content[..response.content.len().min(500)]
        )))?;

    let model_routing: Vec<NodeModelAssignment> = json.model_routing.into_iter()
        .map(|n| NodeModelAssignment {
            node_name: n.node_name,
            node_class: n.node_class,
            model: n.model,
            fidelity: n.fidelity,
            goal_gate: n.goal_gate,
            max_retries: n.max_retries,
        })
        .collect();

    Ok(GraphDotV1 {
        project_id,
        nlspec_version: nlspec_version.to_string(),
        dot_content: json.dot_content,
        node_count: json.node_count,
        estimated_cost_usd: json.estimated_cost_usd,
        run_budget_usd: json.run_budget_usd,
        model_routing,
    })
}

// ===========================================================================
// NLSpecV1 → ScenarioSetV1
// ===========================================================================

const SCENARIO_SYSTEM_PROMPT: &str = r#"You are the Scenario Generator for Planner v2. Your job: transform NLSpec Sacred Anchors and Satisfaction Criteria into BDD scenarios.

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "scenarios": [
    {
      "id": "SC-CRIT-1",
      "tier": "Critical" | "High" | "Medium",
      "title": "Human-readable scenario title",
      "bdd_text": "Given ...\nWhen ...\nThen ...",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": "SC-1"
    }
  ]
}

## Rules
1. Every Sacred Anchor MUST have at least one Critical scenario
2. Every Satisfaction Criterion seed MUST produce at least one scenario
3. Phase 0: Generate ONLY Critical tier scenarios (simplify for micro-tools)
4. BDD text must use Given/When/Then format
5. Scenarios must be testable against a running instance
6. ID format: SC-CRIT-N for Critical, SC-HIGH-N for High, SC-MED-N for Medium
7. For micro-tools: 2-5 Critical scenarios is typical"#;

/// NLSpecV1 → ScenarioSetV1 (BDD scenarios, critical tier only in Phase 0).
pub async fn generate_scenarios(
    router: &LlmRouter,
    spec: &NLSpecV1,
) -> StepResult<ScenarioSetV1> {
    // Extract only the relevant parts for scenario generation
    let context = serde_json::json!({
        "project_id": spec.project_id,
        "intent_summary": spec.intent_summary,
        "sacred_anchors": spec.sacred_anchors,
        "satisfaction_criteria": spec.satisfaction_criteria,
        "requirements": spec.requirements,
        "definition_of_done": spec.definition_of_done,
    });

    let request = CompletionRequest {
        system: Some(SCENARIO_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate BDD scenarios for this NLSpec:\n\n{}",
                serde_json::to_string_pretty(&context).unwrap_or_default()
            ),
        }],
        max_tokens: 4096,
        temperature: 0.3,
        model: DefaultModels::COMPILER_SPEC.to_string(), // Opus for scenario generation
    };

    let response = router.complete(request).await?;
    parse_scenario_response(spec.project_id, &spec.version, &response)
}

#[derive(Debug, serde::Deserialize)]
struct ScenarioSetJson {
    scenarios: Vec<ScenarioJson>,
}

#[derive(Debug, serde::Deserialize)]
struct ScenarioJson {
    id: String,
    tier: String,
    title: String,
    bdd_text: String,
    #[serde(default)]
    dtu_deps: Vec<String>,
    #[serde(default)]
    traces_to_anchors: Vec<String>,
    source_criterion: Option<String>,
}

fn parse_scenario_response(
    project_id: Uuid,
    nlspec_version: &str,
    response: &CompletionResponse,
) -> StepResult<ScenarioSetV1> {
    let content = super::intake::strip_code_fences(&response.content);

    let json: ScenarioSetJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse Scenario Generator response: {}. Raw: {}",
            e, &response.content[..response.content.len().min(500)]
        )))?;

    let scenarios: Vec<Scenario> = json.scenarios.into_iter()
        .map(|s| Scenario {
            id: s.id,
            tier: match s.tier.to_lowercase().as_str() {
                "critical" => ScenarioTier::Critical,
                "high" => ScenarioTier::High,
                _ => ScenarioTier::Medium,
            },
            title: s.title,
            bdd_text: s.bdd_text,
            dtu_deps: s.dtu_deps,
            traces_to_anchors: s.traces_to_anchors,
            source_criterion: s.source_criterion,
        })
        .collect();

    Ok(ScenarioSetV1 {
        project_id,
        nlspec_version: nlspec_version.to_string(),
        scenarios,
        isolation_context_id: Uuid::new_v4(),
        ralph_augmented: false,
    })
}

// ===========================================================================
// NLSpecV1 → AgentsManifestV1
// ===========================================================================

const AGENTS_SYSTEM_PROMPT: &str = r#"You are the AGENTS.md Compiler for Planner v2. Your job: transform an NLSpecV1 into an AGENTS.md file that Kilroy's factory agents will read.

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "root_agents_md": "<full markdown content of the AGENTS.md file, with newlines as literal newline characters in the JSON string>"
}

## AGENTS.md Structure (Kilroy Convention)
The root AGENTS.md must include:
1. **Goal** — What the factory agent is building (from intent_summary)
2. **Sacred Anchors** — Constraints the agent must never violate
3. **Requirements** — Functional requirements (FR-1, FR-2, etc.)
4. **Constraints** — Architectural constraints
5. **Phase 1 Contracts** — Type definitions the agent must implement
6. **Definition of Done** — What "complete" means
7. **Out of Scope** — What the agent must NOT build

## Rules
1. Keep under 500 lines
2. Use markdown formatting for readability
3. Sacred Anchors should be prominent (the agent must never violate these)
4. Requirements should include their IDs for traceability
5. Phase 0: Single root AGENTS.md, no domain docs"#;

/// NLSpecV1 → AgentsManifestV1 (AGENTS.md hierarchy).
pub async fn compile_agents_manifest(
    router: &LlmRouter,
    spec: &NLSpecV1,
) -> StepResult<AgentsManifestV1> {
    let spec_json = serde_json::to_string_pretty(spec)
        .map_err(|e| StepError::JsonError(e.to_string()))?;

    let request = CompletionRequest {
        system: Some(AGENTS_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate an AGENTS.md from this NLSpec:\n\n{}",
                spec_json
            ),
        }],
        max_tokens: 4096,
        temperature: 0.2,
        model: DefaultModels::COMPILER_SPEC.to_string(),
    };

    let response = router.complete(request).await?;
    parse_agents_response(spec.project_id, &spec.version, &response)
}

#[derive(Debug, serde::Deserialize)]
struct AgentsJson {
    root_agents_md: String,
}

fn parse_agents_response(
    project_id: Uuid,
    nlspec_version: &str,
    response: &CompletionResponse,
) -> StepResult<AgentsManifestV1> {
    let content = super::intake::strip_code_fences(&response.content);

    let json: AgentsJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse AGENTS.md Compiler response: {}. Raw: {}",
            e, &response.content[..response.content.len().min(500)]
        )))?;

    Ok(AgentsManifestV1 {
        project_id,
        nlspec_version: nlspec_version.to_string(),
        root_agents_md: json.root_agents_md,
        domain_docs: vec![],  // Phase 0: no domain docs
        skill_refs: vec![],   // Phase 0: no skill refs
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response(content: &str) -> CompletionResponse {
        CompletionResponse {
            content: content.to_string(),
            model: "claude-opus-4-6".into(),
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost_usd: 0.0,
        }
    }

    #[test]
    fn parse_valid_spec_json() {
        let response = sample_response(r#"{
            "intent_summary": "A task tracker widget",
            "sacred_anchors": [
                { "id": "SA-1", "statement": "User data must persist" }
            ],
            "requirements": [
                { "id": "FR-1", "statement": "The system must save tasks to localStorage", "priority": "Must", "traces_to": ["SA-1"] },
                { "id": "FR-2", "statement": "The system must allow marking tasks complete", "priority": "Must", "traces_to": ["SA-1"] }
            ],
            "architectural_constraints": ["Single-file React component"],
            "phase1_contracts": [
                { "name": "Task", "type_definition": "{ id: string, title: string, done: boolean }", "consumed_by": ["ui"] }
            ],
            "external_dependencies": [],
            "definition_of_done": [
                { "criterion": "Tasks persist across refresh", "mechanically_checkable": true }
            ],
            "satisfaction_criteria": [
                { "id": "SC-1", "description": "Add task and refresh shows it", "tier_hint": "Critical" }
            ],
            "out_of_scope": ["Cloud sync"],
            "open_questions": []
        }"#);

        let result = parse_spec_response(Uuid::new_v4(), &response);
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.requirements.len(), 2);
        assert!(spec.sacred_anchors.unwrap().len() == 1);
        assert_eq!(spec.chunk, ChunkType::Root);
        assert_eq!(spec.status, NLSpecStatus::Draft);
    }

    #[test]
    fn parse_valid_graph_dot_json() {
        let response = sample_response(r#"{
            "dot_content": "digraph pipeline { start [shape=Mdiamond]; exit [shape=Msquare]; start -> exit; }",
            "node_count": 2,
            "estimated_cost_usd": 0.50,
            "run_budget_usd": 2.00,
            "model_routing": [
                { "node_name": "implement", "node_class": "hard", "model": "claude-sonnet-4-6", "fidelity": "truncate", "goal_gate": false, "max_retries": 2 }
            ]
        }"#);

        let result = parse_graph_dot_response(Uuid::new_v4(), "1.0", &response);
        assert!(result.is_ok());
        let graph = result.unwrap();
        assert!(graph.dot_content.contains("digraph"));
        assert_eq!(graph.model_routing.len(), 1);
        assert_eq!(graph.model_routing[0].model, "claude-sonnet-4-6");
    }

    #[test]
    fn parse_valid_scenario_json() {
        let response = sample_response(r#"{
            "scenarios": [
                {
                    "id": "SC-CRIT-1",
                    "tier": "Critical",
                    "title": "Task persistence across refresh",
                    "bdd_text": "Given the user adds a task 'Buy milk'\nWhen the page is refreshed\nThen the task 'Buy milk' is visible",
                    "dtu_deps": [],
                    "traces_to_anchors": ["SA-1"],
                    "source_criterion": "SC-1"
                }
            ]
        }"#);

        let result = parse_scenario_response(Uuid::new_v4(), "1.0", &response);
        assert!(result.is_ok());
        let set = result.unwrap();
        assert_eq!(set.scenarios.len(), 1);
        assert_eq!(set.scenarios[0].tier, ScenarioTier::Critical);
        assert!(set.scenarios[0].bdd_text.contains("Given"));
    }

    #[test]
    fn parse_valid_agents_json() {
        let json_str = r##"{ "root_agents_md": "# AGENTS.md" }"##;
        let response = sample_response(json_str);

        let result = parse_agents_response(Uuid::new_v4(), "1.0", &response);
        assert!(result.is_ok());
        let agents = result.unwrap();
        assert!(agents.root_agents_md.contains("AGENTS.md"));
        assert!(agents.domain_docs.is_empty());
    }
}
