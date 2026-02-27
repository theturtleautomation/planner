//! # Ralph Loops — Scenario Augmentation + Gene Transfusion
//!
//! Ralph is the adversarial advisor that runs AFTER scenario generation
//! but BEFORE factory handoff. It operates in two modes:
//!
//! ## ScenarioAugmentation
//! Reviews the generated ScenarioSet and adds edge-case scenarios that
//! the primary generator missed. Focuses on:
//! - Failure modes for external dependencies
//! - Concurrency edge cases
//! - State corruption scenarios
//! - Boundary conditions (empty inputs, max limits, unicode, etc.)
//!
//! ## GeneTransfusion
//! Pattern-matches known component types (auth, payments, file upload, etc.)
//! and injects advisory findings based on common pitfalls for those patterns.
//! Does NOT modify the spec — only surfaces findings as ConsequenceCards.
//!
//! High-severity findings are surfaced as ConsequenceCards in the Impact Inbox.

use uuid::Uuid;

use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use crate::llm::providers::LlmRouter;
use planner_schemas::*;
use super::{StepResult, StepError};

// ---------------------------------------------------------------------------
// Ralph Modes
// ---------------------------------------------------------------------------

/// Which Ralph mode to execute.
#[derive(Debug, Clone)]
pub enum RalphMode {
    /// Post-scenario-gen: adds edge-case scenarios.
    ScenarioAugmentation,
    /// Pattern-matching: surfaces advisory findings for known component types.
    GeneTransfusion,
}

// ---------------------------------------------------------------------------
// Ralph findings
// ---------------------------------------------------------------------------

/// A finding surfaced by Ralph.
#[derive(Debug, Clone)]
pub struct RalphFinding {
    /// Unique ID (e.g. "RALPH-SA-1" for ScenarioAugmentation, "RALPH-GT-1" for GeneTransfusion).
    pub id: String,

    /// Which mode produced this finding.
    pub mode: RalphMode,

    /// Severity: "high" triggers a ConsequenceCard, "medium" is advisory, "low" is informational.
    pub severity: RalphSeverity,

    /// What Ralph found.
    pub description: String,

    /// Affected component/pattern (e.g. "auth", "payments", "file-upload").
    pub affected_pattern: String,

    /// Suggested action.
    pub suggestion: Option<String>,
}

/// Ralph finding severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RalphSeverity {
    /// Triggers a ConsequenceCard in the Impact Inbox.
    High,
    /// Advisory — should be addressed but doesn't block.
    Medium,
    /// Informational — for awareness.
    Low,
}

// ---------------------------------------------------------------------------
// Ralph output
// ---------------------------------------------------------------------------

/// Complete Ralph output after both modes run.
#[derive(Debug, Clone)]
pub struct RalphOutput {
    /// Additional scenarios from ScenarioAugmentation.
    pub augmented_scenarios: Vec<Scenario>,

    /// Advisory findings from GeneTransfusion.
    pub findings: Vec<RalphFinding>,

    /// ConsequenceCards for high-severity findings.
    pub consequence_cards: Vec<ConsequenceCardV1>,
}

// ---------------------------------------------------------------------------
// ScenarioAugmentation
// ---------------------------------------------------------------------------

const SCENARIO_AUGMENTATION_PROMPT: &str = r#"You are Ralph, the adversarial advisor for Planner v2. Your job: review a ScenarioSet and suggest ADDITIONAL edge-case scenarios that the primary generator missed.

## Focus Areas
1. **Failure modes**: What happens when external services are down? Network timeouts?
2. **Concurrency**: Race conditions, stale data, double-submit
3. **State corruption**: What if localStorage is corrupted? What if the DB has inconsistent data?
4. **Boundary conditions**: Empty inputs, max-length strings, unicode, special characters
5. **Security edge cases**: XSS payloads, SQL injection attempts, token expiration mid-operation
6. **Recovery**: What happens after a crash? Can the user resume?

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "scenarios": [
    {
      "id": "SC-RALPH-1",
      "tier": "High" | "Medium",
      "title": "Edge case title",
      "bdd_text": "Given ...\nWhen ...\nThen ...",
      "dtu_deps": [],
      "traces_to_anchors": ["SA-1"],
      "source_criterion": null
    }
  ]
}

## Rules
1. Only generate HIGH and MEDIUM tier scenarios (Critical is set by the primary generator)
2. Focus on realistic edge cases, not contrived scenarios
3. 2-5 additional scenarios is typical
4. Reference existing Sacred Anchors where applicable
5. BDD text must use Given/When/Then format
6. ID format: SC-RALPH-N"#;

/// Run ScenarioAugmentation: review existing scenarios and add edge cases.
pub async fn augment_scenarios(
    router: &LlmRouter,
    spec: &NLSpecV1,
    scenarios: &ScenarioSetV1,
) -> StepResult<Vec<Scenario>> {
    let context = serde_json::json!({
        "intent_summary": spec.intent_summary,
        "sacred_anchors": spec.sacred_anchors,
        "requirements": spec.requirements,
        "existing_scenarios": scenarios.scenarios.iter().map(|s| {
            serde_json::json!({
                "id": s.id,
                "tier": format!("{:?}", s.tier),
                "title": s.title,
            })
        }).collect::<Vec<_>>(),
        "external_dependencies": spec.external_dependencies,
    });

    let request = CompletionRequest {
        system: Some(SCENARIO_AUGMENTATION_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Review these scenarios and suggest additional edge cases:\n\n{}",
                serde_json::to_string_pretty(&context).unwrap_or_default(),
            ),
        }],
        max_tokens: 2048,
        temperature: 0.4, // Slightly higher for creative edge cases
        model: DefaultModels::COMPILER_SPEC.to_string(),
    };

    let response = router.complete(request).await?;
    parse_augmented_scenarios(&response.content)
}

#[derive(Debug, serde::Deserialize)]
struct AugmentedScenariosJson {
    scenarios: Vec<AugScenarioJson>,
}

#[derive(Debug, serde::Deserialize)]
struct AugScenarioJson {
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

fn parse_augmented_scenarios(content: &str) -> StepResult<Vec<Scenario>> {
    let cleaned = super::intake::strip_code_fences(content);

    let json: AugmentedScenariosJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse Ralph ScenarioAugmentation response: {}. Raw: {}",
            e, &content[..content.len().min(300)],
        ))
    })?;

    Ok(json.scenarios.into_iter().map(|s| Scenario {
        id: s.id,
        tier: match s.tier.to_lowercase().as_str() {
            "high" => ScenarioTier::High,
            "medium" => ScenarioTier::Medium,
            _ => ScenarioTier::Medium,
        },
        title: s.title,
        bdd_text: s.bdd_text,
        dtu_deps: s.dtu_deps,
        traces_to_anchors: s.traces_to_anchors,
        source_criterion: s.source_criterion,
    }).collect())
}

// ---------------------------------------------------------------------------
// GeneTransfusion
// ---------------------------------------------------------------------------

/// Known component patterns and their common pitfalls.
const KNOWN_PATTERNS: &[(&str, &[&str])] = &[
    ("auth", &[
        "Session fixation: regenerate session ID after login",
        "Password reset tokens must expire and be single-use",
        "Rate-limit login attempts to prevent brute force",
        "Store passwords with bcrypt/argon2, never plain text",
    ]),
    ("payment", &[
        "Idempotency keys required for all payment operations",
        "Never store raw card numbers — use tokenization",
        "Handle webhook retries gracefully (dedup by event ID)",
        "Implement timeout + reconciliation for async payment flows",
    ]),
    ("file-upload", &[
        "Validate file type on server side (don't trust Content-Type header)",
        "Enforce max file size before reading entire body",
        "Scan uploaded files for malware in async pipeline",
        "Use pre-signed URLs for direct upload to avoid proxy bottleneck",
    ]),
    ("api", &[
        "Validate and sanitize all input — never trust client data",
        "Return consistent error format across all endpoints",
        "Implement request timeout to prevent hanging connections",
        "Use pagination for list endpoints — never return unbounded results",
    ]),
    ("database", &[
        "Use database transactions for multi-step operations",
        "Add indexes for commonly queried columns",
        "Implement connection pooling — don't open/close per request",
        "Use soft deletes for user-facing data to allow recovery",
    ]),
    ("realtime", &[
        "Handle WebSocket reconnection gracefully with backoff",
        "Implement heartbeat/ping to detect stale connections",
        "Buffer messages during disconnection for replay on reconnect",
        "Rate-limit incoming messages per connection",
    ]),
];

/// Run GeneTransfusion: pattern-match known component types in the spec
/// and surface advisory findings based on common pitfalls.
pub fn gene_transfusion(spec: &NLSpecV1) -> Vec<RalphFinding> {
    let mut findings = Vec::new();
    let mut idx = 0u32;

    // Collect all text from the spec for pattern matching
    let spec_text = build_spec_search_text(spec);
    let spec_lower = spec_text.to_lowercase();

    for (pattern_name, pitfalls) in KNOWN_PATTERNS {
        // Check if the spec mentions this pattern
        let pattern_present = spec_lower.contains(pattern_name)
            || spec.external_dependencies.iter().any(|d| d.name.to_lowercase().contains(pattern_name))
            || spec.requirements.iter().any(|r| r.statement.to_lowercase().contains(pattern_name));

        if !pattern_present {
            continue;
        }

        // Check which pitfalls are NOT already addressed in the spec
        for pitfall in *pitfalls {
            let pitfall_lower = pitfall.to_lowercase();
            let key_words: Vec<&str> = pitfall_lower.split_whitespace()
                .filter(|w| w.len() > 4)
                .take(3)
                .collect();

            let already_addressed = key_words.iter()
                .filter(|w| spec_lower.contains(**w))
                .count() >= 2;

            if !already_addressed {
                idx += 1;
                findings.push(RalphFinding {
                    id: format!("RALPH-GT-{}", idx),
                    mode: RalphMode::GeneTransfusion,
                    severity: if is_high_severity_pitfall(pitfall) {
                        RalphSeverity::High
                    } else {
                        RalphSeverity::Medium
                    },
                    description: pitfall.to_string(),
                    affected_pattern: pattern_name.to_string(),
                    suggestion: Some(format!(
                        "Consider adding a requirement or DoD item addressing: {}",
                        pitfall,
                    )),
                });
            }
        }
    }

    findings
}

fn build_spec_search_text(spec: &NLSpecV1) -> String {
    let mut parts = Vec::new();

    if let Some(ref intent) = spec.intent_summary {
        parts.push(intent.as_str());
    }

    for req in &spec.requirements {
        parts.push(req.statement.as_str());
    }

    for constraint in &spec.architectural_constraints {
        parts.push(constraint.as_str());
    }

    for dod in &spec.definition_of_done {
        parts.push(dod.criterion.as_str());
    }

    for dep in &spec.external_dependencies {
        parts.push(dep.name.as_str());
        parts.push(dep.usage_description.as_str());
    }

    parts.join(" ")
}

fn is_high_severity_pitfall(pitfall: &str) -> bool {
    let lower = pitfall.to_lowercase();
    lower.contains("never") || lower.contains("security")
        || lower.contains("password") || lower.contains("token")
        || lower.contains("malware") || lower.contains("brute force")
        || lower.contains("idempoten")
}

// ---------------------------------------------------------------------------
// ConsequenceCard generation
// ---------------------------------------------------------------------------

/// Surface high-severity Ralph findings as ConsequenceCards.
pub fn surface_consequence_cards(
    findings: &[RalphFinding],
    project_id: Uuid,
) -> Vec<ConsequenceCardV1> {
    findings.iter()
        .filter(|f| f.severity == RalphSeverity::High)
        .map(|f| {
            ConsequenceCardV1 {
                card_id: Uuid::new_v4(),
                project_id,
                trigger: CardTrigger::RalphFinding,
                problem: f.description.clone(),
                proposed_solution: f.suggestion.clone().unwrap_or_else(|| "Review and address".into()),
                impact: format!(
                    "Pattern '{}' has a known pitfall that is not addressed in the current spec. \
                     If not addressed, this could lead to production issues.",
                    f.affected_pattern,
                ),
                actions: vec![
                    CardAction {
                        label: "Add Requirement".into(),
                        description: "Add a new FR to address this pitfall".into(),
                    },
                    CardAction {
                        label: "Add to DoD".into(),
                        description: "Add a DoD item to verify this is handled".into(),
                    },
                    CardAction {
                        label: "Dismiss".into(),
                        description: "Acknowledge but take no action".into(),
                    },
                ],
                status: CardStatus::Pending,
                resolution: None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Full Ralph execution
// ---------------------------------------------------------------------------

/// Run the full Ralph loop: ScenarioAugmentation + GeneTransfusion.
///
/// Returns augmented scenarios, advisory findings, and ConsequenceCards
/// for the Impact Inbox.
pub async fn execute_ralph(
    router: &LlmRouter,
    spec: &NLSpecV1,
    scenarios: &ScenarioSetV1,
    project_id: Uuid,
) -> StepResult<RalphOutput> {
    tracing::info!("Ralph: starting adversarial advisory loop");

    // Mode 1: ScenarioAugmentation
    tracing::info!("  Ralph ScenarioAugmentation...");
    let augmented = augment_scenarios(router, spec, scenarios).await?;
    tracing::info!("    → {} additional edge-case scenarios", augmented.len());

    // Mode 2: GeneTransfusion (deterministic, no LLM)
    tracing::info!("  Ralph GeneTransfusion...");
    let findings = gene_transfusion(spec);
    tracing::info!("    → {} advisory findings", findings.len());

    // Surface high-severity findings as ConsequenceCards
    let consequence_cards = surface_consequence_cards(&findings, project_id);
    if !consequence_cards.is_empty() {
        tracing::warn!(
            "  Ralph: {} high-severity finding(s) → ConsequenceCards",
            consequence_cards.len(),
        );
    }

    Ok(RalphOutput {
        augmented_scenarios: augmented,
        findings,
        consequence_cards,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 100,
            created_from: "test".into(),
            intent_summary: Some("User authentication system".into()),
            sacred_anchors: Some(vec![
                NLSpecAnchor { id: "SA-1".into(), statement: "Credentials must be securely stored".into() },
            ]),
            requirements: vec![
                Requirement {
                    id: "FR-1".into(),
                    statement: "The system must hash passwords with bcrypt".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-1".into()],
                },
                Requirement {
                    id: "FR-2".into(),
                    statement: "The system must issue JWT tokens on login".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-1".into()],
                },
            ],
            architectural_constraints: vec!["Node.js backend".into()],
            phase1_contracts: Some(vec![]),
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem { criterion: "User can sign up and login".into(), mechanically_checkable: true },
            ],
            satisfaction_criteria: vec![
                SatisfactionCriterion {
                    id: "SC-1".into(),
                    description: "Login with valid credentials succeeds".into(),
                    tier_hint: ScenarioTierHint::Critical,
                },
            ],
            open_questions: vec![],
            out_of_scope: vec!["OAuth".into()],
            amendment_log: vec![],
        }
    }

    #[test]
    fn gene_transfusion_detects_auth_pattern() {
        let spec = make_auth_spec();
        let findings = gene_transfusion(&spec);

        // Should find auth-related pitfalls
        assert!(!findings.is_empty());
        assert!(findings.iter().all(|f| f.affected_pattern == "auth"));
        assert!(findings.iter().any(|f| f.severity == RalphSeverity::High));
    }

    #[test]
    fn gene_transfusion_skips_unrelated_patterns() {
        let spec = make_auth_spec();
        let findings = gene_transfusion(&spec);

        // Should NOT find file-upload or payment patterns
        assert!(!findings.iter().any(|f| f.affected_pattern == "file-upload"));
        assert!(!findings.iter().any(|f| f.affected_pattern == "payment"));
    }

    #[test]
    fn gene_transfusion_skips_addressed_pitfalls() {
        let mut spec = make_auth_spec();
        // Add a requirement that addresses the bcrypt pitfall
        spec.requirements.push(Requirement {
            id: "FR-3".into(),
            statement: "The system must rate-limit login attempts to prevent brute force attacks".into(),
            priority: Priority::Must,
            traces_to: vec!["SA-1".into()],
        });

        let findings = gene_transfusion(&spec);

        // "rate-limit login attempts" pitfall should NOT appear
        assert!(!findings.iter().any(|f|
            f.description.to_lowercase().contains("rate-limit login")
        ));
    }

    #[test]
    fn consequence_cards_only_for_high_severity() {
        let findings = vec![
            RalphFinding {
                id: "RALPH-GT-1".into(),
                mode: RalphMode::GeneTransfusion,
                severity: RalphSeverity::High,
                description: "Serious issue".into(),
                affected_pattern: "auth".into(),
                suggestion: Some("Fix it".into()),
            },
            RalphFinding {
                id: "RALPH-GT-2".into(),
                mode: RalphMode::GeneTransfusion,
                severity: RalphSeverity::Medium,
                description: "Minor issue".into(),
                affected_pattern: "auth".into(),
                suggestion: None,
            },
        ];

        let cards = surface_consequence_cards(&findings, Uuid::new_v4());
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].problem, "Serious issue");
        assert_eq!(cards[0].trigger, CardTrigger::RalphFinding);
        assert_eq!(cards[0].status, CardStatus::Pending);
    }

    #[test]
    fn parse_augmented_scenarios_valid() {
        let content = r#"{
            "scenarios": [
                {
                    "id": "SC-RALPH-1",
                    "tier": "High",
                    "title": "Login with expired JWT",
                    "bdd_text": "Given a user with an expired token\nWhen they make an API request\nThen they receive a 401 response",
                    "dtu_deps": [],
                    "traces_to_anchors": ["SA-1"],
                    "source_criterion": null
                }
            ]
        }"#;

        let result = parse_augmented_scenarios(content);
        assert!(result.is_ok());
        let scenarios = result.unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].tier, ScenarioTier::High);
        assert_eq!(scenarios[0].id, "SC-RALPH-1");
    }

    #[test]
    fn parse_augmented_scenarios_with_code_fences() {
        let content = "```json\n{\"scenarios\": [{\"id\": \"SC-RALPH-1\", \"tier\": \"Medium\", \"title\": \"Edge case\", \"bdd_text\": \"Given...\\nWhen...\\nThen...\"}]}\n```";
        let result = parse_augmented_scenarios(content);
        assert!(result.is_ok());
    }
}
