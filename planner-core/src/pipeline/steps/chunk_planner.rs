//! # Chunk Planner — IntakeV1 → ChunkPlan
//!
//! Analyzes an IntakeV1 to determine whether the project requires
//! multi-chunk Progressive Specification or a single root chunk.
//!
//! The decision is based on:
//! - Output domain complexity (MicroTool → single chunk, full app → multi)
//! - Requirement count signals (>8 FRs suggests multiple domains)
//! - Explicit domain boundaries in the user's description
//!
//! For multi-chunk projects, the Chunk Planner identifies:
//! - Which domain chunks are needed (e.g. auth, api, ui, payments)
//! - Which Sacred Anchors apply to each chunk
//! - Which Phase 1 Contracts bridge domains
//!
//! Phase 0-2: Always returned single root chunk.
//! Phase 3: LLM-driven domain decomposition for complex projects.

use uuid::Uuid;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// ChunkPlan — output of the chunk planner
// ---------------------------------------------------------------------------

/// The chunk plan — determines how many NLSpec chunks to generate.
#[derive(Debug, Clone)]
pub struct ChunkPlan {
    /// Project ID.
    pub project_id: Uuid,

    /// Whether this project needs multi-chunk compilation.
    pub is_multi_chunk: bool,

    /// The domain chunks to generate. Always includes "root".
    /// For multi-chunk: root + N domain chunks.
    /// For single-chunk: root only.
    pub chunks: Vec<PlannedChunk>,
}

/// A planned chunk within the ChunkPlan.
#[derive(Debug, Clone)]
pub struct PlannedChunk {
    /// Chunk identifier (e.g. "root", "auth", "api", "ui").
    pub chunk_id: String,

    /// Chunk type.
    pub chunk_type: ChunkType,

    /// Which Sacred Anchor IDs are relevant to this chunk.
    pub relevant_anchor_ids: Vec<String>,

    /// Domain-specific context extracted from the intake.
    pub domain_context: String,

    /// Estimated requirement count for this chunk (advisory).
    pub estimated_fr_count: u32,
}

// ---------------------------------------------------------------------------
// Heuristic: should we multi-chunk?
// ---------------------------------------------------------------------------

/// Thresholds for deciding whether to use multi-chunk compilation.
const MULTI_CHUNK_FR_THRESHOLD: usize = 8;
const MULTI_CHUNK_ANCHOR_THRESHOLD: usize = 4;

/// Quick heuristic check: does this intake warrant multi-chunk compilation?
///
/// This runs BEFORE the LLM call to avoid unnecessary spend on simple projects.
fn should_multi_chunk(intake: &IntakeV1) -> bool {
    // MicroTools are always single-chunk
    match &intake.output_domain {
        OutputDomain::MicroTool { .. } => return false,
        _ => {}
    }

    // If many sacred anchors, probably multi-chunk
    if intake.sacred_anchors.len() >= MULTI_CHUNK_ANCHOR_THRESHOLD {
        return true;
    }

    // If many satisfaction criteria seeds, probably multi-chunk
    if intake.satisfaction_criteria_seeds.len() >= MULTI_CHUNK_FR_THRESHOLD {
        return true;
    }

    // Default: let the LLM decide based on description complexity
    // For now, use a simple word count heuristic on the intent summary
    intake.intent_summary.split_whitespace().count() > 100
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze an IntakeV1 and produce a ChunkPlan.
///
/// For simple projects (micro-tools), returns a single root chunk
/// without an LLM call. For complex projects, asks an LLM to
/// decompose the project into domain chunks.
pub async fn plan_chunks(
    router: &LlmRouter,
    intake: &IntakeV1,
    project_id: Uuid,
) -> StepResult<ChunkPlan> {
    if !should_multi_chunk(intake) {
        tracing::info!("Chunk Planner: single root chunk (micro-tool or simple project)");
        return Ok(build_single_chunk_plan(intake, project_id));
    }

    tracing::info!("Chunk Planner: complex project detected — asking LLM for domain decomposition");
    plan_multi_chunk(router, intake, project_id).await
}

/// Build a single-chunk plan (Phase 0-2 behavior preserved).
fn build_single_chunk_plan(intake: &IntakeV1, project_id: Uuid) -> ChunkPlan {
    let anchor_ids: Vec<String> = intake.sacred_anchors.iter()
        .map(|a| a.id.clone())
        .collect();

    ChunkPlan {
        project_id,
        is_multi_chunk: false,
        chunks: vec![PlannedChunk {
            chunk_id: "root".into(),
            chunk_type: ChunkType::Root,
            relevant_anchor_ids: anchor_ids,
            domain_context: intake.intent_summary.clone(),
            estimated_fr_count: 5, // typical micro-tool
        }],
    }
}

// ---------------------------------------------------------------------------
// LLM-driven multi-chunk planning
// ---------------------------------------------------------------------------

const CHUNK_PLANNER_SYSTEM_PROMPT: &str = r#"You are the Chunk Planner for Planner v2. Your job: analyze a project intake and decompose it into domain chunks for parallel specification.

## Context
Complex projects are split into:
1. A **root** chunk: Intent Summary, Sacred Anchors, Phase 1 Contracts (shared types)
2. Multiple **domain** chunks: each has its own FRs, constraints, DoD, and satisfaction criteria

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "chunks": [
    {
      "chunk_id": "root",
      "relevant_anchor_ids": ["SA-1", "SA-2", "SA-3"],
      "domain_context": "Overall project: ...",
      "estimated_fr_count": 3
    },
    {
      "chunk_id": "auth",
      "relevant_anchor_ids": ["SA-1"],
      "domain_context": "Authentication domain: login, signup, session management",
      "estimated_fr_count": 5
    },
    {
      "chunk_id": "api",
      "relevant_anchor_ids": ["SA-2", "SA-3"],
      "domain_context": "API domain: REST endpoints, data validation, error handling",
      "estimated_fr_count": 6
    }
  ]
}

## Rules
1. The first chunk MUST be "root" — it contains cross-cutting concerns
2. Each domain chunk should have 3-8 FRs (aim for 5)
3. Every Sacred Anchor must appear in at least one chunk's relevant_anchor_ids
4. Domain names should be short, lowercase, kebab-friendly (auth, api, ui, payments, etc.)
5. Keep chunks ≤500 lines each when later compiled into NLSpecs
6. Prefer fewer chunks (2-4 domains) over many small ones
7. Each chunk should represent a cohesive, independently-reviewable domain"#;

async fn plan_multi_chunk(
    router: &LlmRouter,
    intake: &IntakeV1,
    project_id: Uuid,
) -> StepResult<ChunkPlan> {
    let intake_json = serde_json::to_string_pretty(intake)
        .map_err(|e| StepError::JsonError(e.to_string()))?;

    let request = CompletionRequest {
        system: Some(CHUNK_PLANNER_SYSTEM_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Analyze this project intake and determine the domain chunks:\n\n{}",
                intake_json,
            ),
        }],
        max_tokens: 1024,
        temperature: 0.2,
        model: DefaultModels::COMPILER_SPEC.to_string(),
    };

    let response = router.complete(request).await?;
    parse_chunk_plan_response(&response.content, intake, project_id)
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ChunkPlanJson {
    chunks: Vec<ChunkJson>,
}

#[derive(Debug, serde::Deserialize)]
struct ChunkJson {
    chunk_id: String,
    #[serde(default)]
    relevant_anchor_ids: Vec<String>,
    #[serde(default)]
    domain_context: String,
    #[serde(default = "default_fr_count")]
    estimated_fr_count: u32,
}

fn default_fr_count() -> u32 { 5 }

fn parse_chunk_plan_response(
    content: &str,
    intake: &IntakeV1,
    project_id: Uuid,
) -> StepResult<ChunkPlan> {
    let cleaned = super::intake::strip_code_fences(content);

    let json: ChunkPlanJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse chunk plan response: {}. Raw: {}",
            e, &content[..content.len().min(300)]
        ))
    })?;

    if json.chunks.is_empty() {
        return Err(StepError::Other("Chunk plan returned no chunks".into()));
    }

    // Ensure root is first
    if json.chunks[0].chunk_id != "root" {
        return Err(StepError::Other(
            "Chunk plan must start with 'root' chunk".into(),
        ));
    }

    // Validate: every Sacred Anchor must appear in at least one chunk
    let all_anchor_ids: Vec<&str> = intake.sacred_anchors.iter()
        .map(|a| a.id.as_str())
        .collect();
    let covered_anchors: Vec<&str> = json.chunks.iter()
        .flat_map(|c| c.relevant_anchor_ids.iter().map(|s| s.as_str()))
        .collect();
    for anchor_id in &all_anchor_ids {
        if !covered_anchors.contains(anchor_id) {
            tracing::warn!(
                "Chunk plan missing Sacred Anchor {} — adding to root chunk",
                anchor_id,
            );
        }
    }

    let chunks: Vec<PlannedChunk> = json.chunks.into_iter().map(|c| {
        let chunk_type = if c.chunk_id == "root" {
            ChunkType::Root
        } else {
            ChunkType::Domain { name: c.chunk_id.clone() }
        };

        PlannedChunk {
            chunk_id: c.chunk_id,
            chunk_type,
            relevant_anchor_ids: c.relevant_anchor_ids,
            domain_context: c.domain_context,
            estimated_fr_count: c.estimated_fr_count,
        }
    }).collect();

    let is_multi = chunks.len() > 1;
    tracing::info!(
        "Chunk plan: {} chunk(s) — {}",
        chunks.len(),
        chunks.iter().map(|c| c.chunk_id.as_str()).collect::<Vec<_>>().join(", "),
    );

    Ok(ChunkPlan {
        project_id,
        is_multi_chunk: is_multi,
        chunks,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chunk_plan_for_micro_tool() {
        let intake = build_micro_tool_intake();
        let project_id = Uuid::new_v4();
        let plan = build_single_chunk_plan(&intake, project_id);

        assert!(!plan.is_multi_chunk);
        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(plan.chunks[0].chunk_id, "root");
        assert_eq!(plan.chunks[0].chunk_type, ChunkType::Root);
        assert_eq!(plan.chunks[0].relevant_anchor_ids, vec!["SA-1", "SA-2"]);
    }

    #[test]
    fn should_multi_chunk_false_for_micro_tool() {
        let intake = build_micro_tool_intake();
        assert!(!should_multi_chunk(&intake));
    }

    #[test]
    fn parse_valid_chunk_plan() {
        let content = r#"{
            "chunks": [
                {
                    "chunk_id": "root",
                    "relevant_anchor_ids": ["SA-1", "SA-2", "SA-3"],
                    "domain_context": "E-commerce checkout system",
                    "estimated_fr_count": 3
                },
                {
                    "chunk_id": "auth",
                    "relevant_anchor_ids": ["SA-1"],
                    "domain_context": "Authentication: login, signup, sessions",
                    "estimated_fr_count": 5
                },
                {
                    "chunk_id": "payments",
                    "relevant_anchor_ids": ["SA-2", "SA-3"],
                    "domain_context": "Payment processing: Stripe integration",
                    "estimated_fr_count": 6
                }
            ]
        }"#;

        let intake = build_complex_intake();
        let project_id = Uuid::new_v4();
        let plan = parse_chunk_plan_response(content, &intake, project_id);

        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert!(plan.is_multi_chunk);
        assert_eq!(plan.chunks.len(), 3);
        assert_eq!(plan.chunks[0].chunk_id, "root");
        assert_eq!(plan.chunks[1].chunk_id, "auth");
        assert_eq!(plan.chunks[2].chunk_id, "payments");
        assert_eq!(plan.chunks[1].chunk_type, ChunkType::Domain { name: "auth".into() });
    }

    #[test]
    fn parse_chunk_plan_rejects_missing_root() {
        let content = r#"{
            "chunks": [
                {
                    "chunk_id": "auth",
                    "relevant_anchor_ids": ["SA-1"],
                    "domain_context": "Auth stuff",
                    "estimated_fr_count": 5
                }
            ]
        }"#;

        let intake = build_complex_intake();
        let result = parse_chunk_plan_response(content, &intake, Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn parse_chunk_plan_rejects_empty() {
        let content = r#"{"chunks": []}"#;
        let intake = build_complex_intake();
        let result = parse_chunk_plan_response(content, &intake, Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn parse_chunk_plan_with_code_fences() {
        let content = "```json\n{\"chunks\": [{\"chunk_id\": \"root\", \"relevant_anchor_ids\": [\"SA-1\"], \"domain_context\": \"Simple\", \"estimated_fr_count\": 3}]}\n```";
        let intake = build_micro_tool_intake();
        let result = parse_chunk_plan_response(content, &intake, Uuid::new_v4());
        assert!(result.is_ok());
    }

    // -- Test helpers --

    fn build_micro_tool_intake() -> IntakeV1 {
        IntakeV1 {
            project_id: Uuid::new_v4(),
            project_name: "Countdown Timer".into(),
            feature_slug: "countdown-timer".into(),
            intent_summary: "A simple countdown timer widget".into(),
            output_domain: OutputDomain::MicroTool {
                variant: MicroToolVariant::ReactWidget,
            },
            environment: EnvironmentInfo {
                language: "TypeScript".into(),
                framework: "React".into(),
                package_manager: Some("npm".into()),
                existing_dependencies: vec![],
                build_tool: Some("vite".into()),
            },
            sacred_anchors: vec![
                SacredAnchor { id: "SA-1".into(), statement: "Never negative".into(), rationale: None },
                SacredAnchor { id: "SA-2".into(), statement: "Pause preserves time".into(), rationale: None },
            ],
            satisfaction_criteria_seeds: vec!["Timer counts down".into()],
            out_of_scope: vec!["Sound alerts".into()],
            conversation_log: vec![],
        }
    }

    fn build_complex_intake() -> IntakeV1 {
        IntakeV1 {
            project_id: Uuid::new_v4(),
            project_name: "Checkout System".into(),
            feature_slug: "checkout-system".into(),
            intent_summary: "A full e-commerce checkout system with authentication, product catalog, shopping cart, payment processing via Stripe, order management, and email notifications. Users can browse products, add to cart, create accounts, login, checkout with saved payment methods, and receive order confirmation emails.".into(),
            output_domain: OutputDomain::FullApp {
                estimated_domains: 3,
            },
            environment: EnvironmentInfo {
                language: "Python+TypeScript".into(),
                framework: "FastAPI+React".into(),
                package_manager: Some("pip+npm".into()),
                existing_dependencies: vec![],
                build_tool: Some("vite".into()),
            },
            sacred_anchors: vec![
                SacredAnchor { id: "SA-1".into(), statement: "User credentials must be securely hashed".into(), rationale: None },
                SacredAnchor { id: "SA-2".into(), statement: "Payment data must never touch our servers".into(), rationale: None },
                SacredAnchor { id: "SA-3".into(), statement: "Orders must be idempotent".into(), rationale: None },
            ],
            satisfaction_criteria_seeds: vec![
                "User can create account and login".into(),
                "User can browse products".into(),
                "User can add items to cart".into(),
                "User can checkout with Stripe".into(),
                "User receives order confirmation email".into(),
                "Admin can view order list".into(),
                "Cart persists across sessions".into(),
                "Payment failure shows helpful error".into(),
            ],
            out_of_scope: vec!["Shipping calculation".into(), "Inventory management".into()],
            conversation_log: vec![],
        }
    }
}
