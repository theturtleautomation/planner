//! # Compiler — IntakeV1 → NLSpecV1 → GraphDotV1 → ScenarioSetV1 → AgentsManifestV1
//!
//! The Compiler transforms raw intake data through a series of LLM-driven
//! compilation steps. Each step takes a structured artifact and produces
//! the next one in the pipeline.
//!
//! Phase 0: Single root NLSpec chunk, simplified graph.dot, single AGENTS.md.
//! Phase 1: All scenario tiers (Critical + High + Medium).
//! Phase 3: Multi-chunk compilation — ChunkPlan + IntakeV1 → root + N domain NLSpecV1s.

use uuid::Uuid;

use crate::llm::{CompletionRequest, CompletionResponse, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};
use super::chunk_planner::{ChunkPlan, PlannedChunk};

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
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| super::intake::strip_code_fences(&response.content));

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
// Multi-Chunk Compilation: ChunkPlan + IntakeV1 → Vec<NLSpecV1>
// ===========================================================================

const DOMAIN_SPEC_SYSTEM_PROMPT: &str = r#"You are the Domain Spec Compiler for Planner v2. Your job: generate a domain-scoped NLSpecV1 chunk for ONE domain of a multi-chunk project.

## Context
You are compiling the **{domain_name}** domain chunk. The root chunk has already defined the shared Sacred Anchors and Phase 1 Contracts. Your job is to produce domain-specific FRs, constraints, DoD, and satisfaction criteria.

## Cross-Chunk References
- Reference Phase 1 Contracts by name (e.g. "UserSession", "PaymentIntent") — the root chunk owns them.
- Reference Sacred Anchors by ID (e.g. "SA-1") — the root chunk owns them.
- FR IDs in this domain MUST use the format: FR-{domain_prefix}-N (e.g. FR-AUTH-1, FR-API-2, FR-UI-3)
- Satisfaction Criteria IDs: SC-{domain_prefix}-N (e.g. SC-AUTH-1)

## Output Format
Respond with ONLY a JSON object (no markdown fences, no explanation):

{
  "requirements": [
    {
      "id": "FR-{domain_prefix}-1",
      "statement": "The system must ... (imperative language required)",
      "priority": "Must" | "Should" | "Could",
      "traces_to": ["SA-1"]
    }
  ],
  "architectural_constraints": ["Domain-specific constraint", ...],
  "external_dependencies": [
    { "name": "Stripe", "dtu_priority": "High", "usage_description": "Payment processing" }
  ],
  "definition_of_done": [
    { "criterion": "What must be true", "mechanically_checkable": true|false }
  ],
  "satisfaction_criteria": [
    {
      "id": "SC-{domain_prefix}-1",
      "description": "Plain-English expected behavior",
      "tier_hint": "Critical" | "High" | "Medium"
    }
  ],
  "out_of_scope": ["Items this domain does NOT handle"]
}

## Rules
1. Only include FRs relevant to the {domain_name} domain — do NOT duplicate FRs from other domains
2. ALL FR statements MUST use imperative language: must, must not, always, never, shall
3. Every FR MUST trace to at least one Sacred Anchor from the root chunk
4. At least one Satisfaction Criterion MUST have tier_hint "Critical"
5. Keep this domain chunk ≤500 lines
6. Reference shared Phase 1 Contracts by name when your domain consumes them
7. 3-8 FRs is typical for a domain chunk"#;

/// Multi-chunk compilation: ChunkPlan + IntakeV1 → Vec<NLSpecV1>.
///
/// Generates a root NLSpecV1 (via the standard compiler) and then N domain
/// NLSpecV1s (one per non-root chunk in the plan). Domain chunks reference
/// the root's Sacred Anchors and Phase 1 Contracts by stable ID.
pub async fn compile_spec_multichunk(
    router: &LlmRouter,
    intake: &IntakeV1,
    plan: &ChunkPlan,
) -> StepResult<Vec<NLSpecV1>> {
    // Step 1: Compile the root chunk (same as single-chunk compilation)
    let root_spec = compile_spec(router, intake).await?;
    tracing::info!(
        "Multi-chunk root compiled: {} FRs, {} contracts",
        root_spec.requirements.len(),
        root_spec.phase1_contracts.as_ref().map(|c| c.len()).unwrap_or(0),
    );

    let mut specs = vec![root_spec];

    // Step 2: Compile each domain chunk
    for planned in &plan.chunks {
        if planned.chunk_id == "root" {
            continue; // Already compiled
        }

        let domain_spec = compile_domain_chunk(
            router, intake, &specs[0], planned,
        ).await?;

        tracing::info!(
            "Domain chunk '{}' compiled: {} FRs",
            planned.chunk_id,
            domain_spec.requirements.len(),
        );

        specs.push(domain_spec);
    }

    tracing::info!(
        "Multi-chunk compilation complete: {} total chunk(s)",
        specs.len(),
    );

    Ok(specs)
}

/// Compile a single domain chunk using the root spec as context.
async fn compile_domain_chunk(
    router: &LlmRouter,
    intake: &IntakeV1,
    root_spec: &NLSpecV1,
    planned: &PlannedChunk,
) -> StepResult<NLSpecV1> {
    let domain_name = &planned.chunk_id;
    let domain_prefix = domain_name.to_uppercase().replace('-', "_");

    // Build the system prompt with domain-specific substitutions
    let system_prompt = DOMAIN_SPEC_SYSTEM_PROMPT
        .replace("{domain_name}", domain_name)
        .replace("{domain_prefix}", &domain_prefix);

    // Build context for the LLM: root contracts + anchors + domain-specific info
    let context = serde_json::json!({
        "project_name": intake.project_name,
        "intent_summary": intake.intent_summary,
        "domain_name": domain_name,
        "domain_context": planned.domain_context,
        "relevant_sacred_anchors": root_spec.sacred_anchors.as_ref()
            .map(|anchors| anchors.iter()
                .filter(|a| planned.relevant_anchor_ids.contains(&a.id))
                .collect::<Vec<_>>()
            ).unwrap_or_default(),
        "phase1_contracts": root_spec.phase1_contracts,
        "estimated_fr_count": planned.estimated_fr_count,
        "environment": intake.environment,
        "out_of_scope": intake.out_of_scope,
    });

    let request = CompletionRequest {
        system: Some(system_prompt),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Compile the '{}' domain chunk for this project:\n\n{}",
                domain_name,
                serde_json::to_string_pretty(&context).unwrap_or_default(),
            ),
        }],
        max_tokens: 4096,
        temperature: 0.2,
        model: DefaultModels::COMPILER_SPEC.to_string(),
    };

    let response = router.complete(request).await?;
    parse_domain_chunk_response(
        intake.project_id,
        &root_spec.version,
        domain_name,
        &response,
    )
}

// ---------------------------------------------------------------------------
// Domain chunk response parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct DomainSpecJson {
    requirements: Vec<RequirementJson>,
    #[serde(default)]
    architectural_constraints: Vec<String>,
    #[serde(default)]
    external_dependencies: Vec<ExtDepJson>,
    definition_of_done: Vec<DoDJson>,
    satisfaction_criteria: Vec<SatCritJson>,
    #[serde(default)]
    out_of_scope: Vec<String>,
}

fn parse_domain_chunk_response(
    project_id: Uuid,
    root_version: &str,
    domain_name: &str,
    response: &CompletionResponse,
) -> StepResult<NLSpecV1> {
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| super::intake::strip_code_fences(&response.content));

    let json: DomainSpecJson = serde_json::from_str(&content)
        .map_err(|e| StepError::JsonError(format!(
            "Failed to parse domain '{}' Spec Compiler response: {}. Raw: {}",
            domain_name, e, &response.content[..response.content.len().min(500)]
        )))?;

    let line_count = (content.len() / 60).min(500) as u32;

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

    Ok(NLSpecV1 {
        project_id,
        version: root_version.to_string(),
        chunk: ChunkType::Domain { name: domain_name.to_string() },
        status: NLSpecStatus::Draft,
        line_count,
        created_from: format!("{}:{}:domain:{}", IntakeV1::TYPE_ID, project_id, domain_name),
        // Domain chunks don't have these root-only fields
        intent_summary: None,
        sacred_anchors: None,
        phase1_contracts: None,
        // Domain-specific fields
        requirements,
        architectural_constraints: json.architectural_constraints,
        external_dependencies,
        definition_of_done,
        satisfaction_criteria,
        open_questions: vec![],
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

const MULTI_CHUNK_GRAPH_DOT_SYSTEM_PROMPT: &str = r#"You are the Graph.dot Compiler for Planner v2 (multi-chunk mode). Your job: transform a set of NLSpec chunks into a Kilroy Attractor-compatible DOT pipeline with PARALLEL domain branches.

## Multi-Chunk Pipeline Topology
For multi-domain projects, generate a pipeline with parallel branches:

1. start → check_toolchain → lock_contracts
2. lock_contracts → [parallel domain branches]
   - Each domain: expand_spec_{domain} → implement_{domain} → verify_build_{domain} → verify_test_{domain}
3. All domain branches → merge_integration → integration_test → review → exit

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "dot_content": "digraph pipeline { ... }",
  "node_count": N,
  "estimated_cost_usd": X.XX,
  "run_budget_usd": Y.YY,
  "model_routing": [
    {
      "node_name": "implement_auth",
      "node_class": "hard",
      "model": "claude-sonnet-4-6",
      "fidelity": "truncate",
      "goal_gate": false,
      "max_retries": 2
    }
  ]
}

## DOT Format Rules (same as single-chunk, plus):
- Domain branches run in parallel (use `rank=same` for parallel nodes)
- `lock_contracts` node ensures Phase 1 Contracts are finalized before domain work
- `merge_integration` is a join node that waits for all domain branches
- Each domain's implement node gets `class="hard"`
- Cost estimation should sum across all parallel branches
- Budget should account for per-domain retries

## Key Constraints
- Valid Graphviz DOT
- Parallel branches must converge at merge_integration
- Each domain gets its own implement + verify cycle
- Integration test runs AFTER all domains merge"#;

/// NLSpecV1 → GraphDotV1 (Attractor-compatible DOT pipeline).
///
/// For single-chunk specs, produces a linear pipeline.
/// For multi-chunk specs (passed as a full set), produces parallel domain branches.
pub async fn compile_graph_dot(
    router: &LlmRouter,
    spec: &NLSpecV1,
) -> StepResult<GraphDotV1> {
    use super::context_pack::{build_spec_context_pack, render_context_pack, ContextTarget};

    let pack = build_spec_context_pack(spec, ContextTarget::GraphDotCompiler, 8000);
    let context_text = render_context_pack(&pack);
    tracing::debug!(
        "graph.dot context pack: {} tokens (truncated: {})",
        pack.estimated_tokens, pack.was_truncated,
    );

    let request = CompletionRequest {
        system: Some(GRAPH_DOT_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate a Kilroy Attractor DOT pipeline for this NLSpec:\n\n{}",
                context_text
            ),
        }],
        max_tokens: 8192,
        temperature: 0.2,
        model: DefaultModels::COMPILER_GRAPH_DOT.to_string(),
    };

    let response = router.complete(request).await?;
    parse_graph_dot_response(spec.project_id, &spec.version, &response)
}

/// Multi-chunk graph.dot: generates parallel DOT branches for each domain.
///
/// Takes the full set of NLSpec chunks (root + domains) and produces a
/// single GraphDotV1 with a contracts-first node, then parallel domain
/// implementation branches converging at integration.
pub async fn compile_graph_dot_multichunk(
    router: &LlmRouter,
    specs: &[NLSpecV1],
) -> StepResult<GraphDotV1> {
    if specs.is_empty() {
        return Err(StepError::Other("No NLSpec chunks provided for multi-chunk graph.dot".into()));
    }

    let root = &specs[0];
    let domains: Vec<&NLSpecV1> = specs.iter().skip(1).collect();

    let domain_names: Vec<String> = domains.iter().map(|d| {
        match &d.chunk {
            ChunkType::Domain { name } => name.clone(),
            ChunkType::Root => "root".into(),
        }
    }).collect();

    let context = serde_json::json!({
        "root_spec": {
            "intent_summary": root.intent_summary,
            "sacred_anchors": root.sacred_anchors,
            "phase1_contracts": root.phase1_contracts,
            "architectural_constraints": root.architectural_constraints,
        },
        "domain_chunks": domains.iter().map(|d| serde_json::json!({
            "chunk": d.chunk,
            "requirements": d.requirements,
            "external_dependencies": d.external_dependencies,
        })).collect::<Vec<_>>(),
        "domain_names": domain_names,
    });

    let request = CompletionRequest {
        system: Some(MULTI_CHUNK_GRAPH_DOT_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate a multi-chunk Kilroy Attractor DOT pipeline with parallel domain branches:\n\n{}",
                serde_json::to_string_pretty(&context).unwrap_or_default(),
            ),
        }],
        max_tokens: 8192,
        temperature: 0.2,
        model: DefaultModels::COMPILER_GRAPH_DOT.to_string(),
    };

    let response = router.complete(request).await?;
    parse_graph_dot_response(root.project_id, &root.version, &response)
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
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| super::intake::strip_code_fences(&response.content));

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

const SCENARIO_SYSTEM_PROMPT: &str = r#"You are the Scenario Generator for Planner v2. Your job: transform NLSpec Sacred Anchors and Satisfaction Criteria into BDD scenarios across ALL tiers.

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
3. Generate scenarios for ALL THREE tiers: Critical, High, and Medium
4. BDD text must use Given/When/Then format
5. Scenarios must be testable against a running instance
6. ID format: SC-CRIT-N for Critical, SC-HIGH-N for High, SC-MED-N for Medium
7. Tier assignment rules:
   - Critical: Core functionality, data integrity, security invariants. Must NEVER fail.
   - High: Important UX flows, performance expectations, edge cases. Expect ≥95% pass.
   - Medium: Minor UX polish, cosmetic behaviors, graceful degradation. Expect ≥90% pass.
8. For micro-tools: 2-4 Critical, 2-3 High, 1-3 Medium scenarios is typical
9. Respect the tier_hint on each Satisfaction Criterion — use it as the starting tier"#;

/// NLSpecV1 → ScenarioSetV1 (BDD scenarios, all tiers).
pub async fn generate_scenarios(
    router: &LlmRouter,
    spec: &NLSpecV1,
) -> StepResult<ScenarioSetV1> {
    use super::context_pack::{build_spec_context_pack, render_context_pack, ContextTarget};

    let pack = build_spec_context_pack(spec, ContextTarget::ScenarioGenerator, 6000);
    let context_text = render_context_pack(&pack);
    tracing::debug!(
        "Scenario gen context pack: {} tokens (truncated: {})",
        pack.estimated_tokens, pack.was_truncated,
    );

    let request = CompletionRequest {
        system: Some(SCENARIO_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate BDD scenarios for this NLSpec:\n\n{}",
                context_text
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
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| super::intake::strip_code_fences(&response.content));

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
    use super::context_pack::{build_spec_context_pack, render_context_pack, ContextTarget};

    let pack = build_spec_context_pack(spec, ContextTarget::SpecCompiler, 6000);
    let context_text = render_context_pack(&pack);
    tracing::debug!(
        "AGENTS.md context pack: {} tokens (truncated: {})",
        pack.estimated_tokens, pack.was_truncated,
    );

    let request = CompletionRequest {
        system: Some(AGENTS_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate an AGENTS.md from this NLSpec:\n\n{}",
                context_text
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
    let content = crate::llm::json_repair::try_repair_json(&response.content)
        .unwrap_or_else(|| super::intake::strip_code_fences(&response.content));

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

    #[test]
    fn parse_valid_domain_chunk_json() {
        let response = sample_response(r#"{
            "requirements": [
                {
                    "id": "FR-AUTH-1",
                    "statement": "The system must hash all user passwords with bcrypt",
                    "priority": "Must",
                    "traces_to": ["SA-1"]
                },
                {
                    "id": "FR-AUTH-2",
                    "statement": "The system must issue JWT tokens on successful login",
                    "priority": "Must",
                    "traces_to": ["SA-1"]
                }
            ],
            "architectural_constraints": ["Use bcrypt with cost factor 12"],
            "external_dependencies": [],
            "definition_of_done": [
                { "criterion": "User can sign up and login", "mechanically_checkable": true }
            ],
            "satisfaction_criteria": [
                { "id": "SC-AUTH-1", "description": "User creates account and logs in", "tier_hint": "Critical" }
            ],
            "out_of_scope": ["OAuth social login"]
        }"#);

        let result = parse_domain_chunk_response(
            Uuid::new_v4(), "1.0", "auth", &response,
        );
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.chunk, ChunkType::Domain { name: "auth".into() });
        assert_eq!(spec.requirements.len(), 2);
        assert_eq!(spec.requirements[0].id, "FR-AUTH-1");
        assert!(spec.intent_summary.is_none()); // Domain chunks don't have intent_summary
        assert!(spec.sacred_anchors.is_none()); // Domain chunks don't have sacred_anchors
        assert!(spec.phase1_contracts.is_none()); // Domain chunks don't have contracts
        assert!(!spec.definition_of_done.is_empty());
        assert!(!spec.out_of_scope.is_empty());
    }

    #[test]
    fn parse_domain_chunk_with_code_fences() {
        let response = sample_response(
            "```json\n{\"requirements\": [{\"id\": \"FR-API-1\", \"statement\": \"The system must validate all inputs\", \"priority\": \"Must\", \"traces_to\": [\"SA-2\"]}], \"definition_of_done\": [{\"criterion\": \"API returns 400 for invalid input\", \"mechanically_checkable\": true}], \"satisfaction_criteria\": [{\"id\": \"SC-API-1\", \"description\": \"Invalid input is rejected\", \"tier_hint\": \"Critical\"}], \"out_of_scope\": [\"Rate limiting\"]}\n```"
        );

        let result = parse_domain_chunk_response(
            Uuid::new_v4(), "1.0", "api", &response,
        );
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.chunk, ChunkType::Domain { name: "api".into() });
        assert_eq!(spec.requirements[0].id, "FR-API-1");
    }
}
