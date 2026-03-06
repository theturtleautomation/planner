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

use super::{StepError, StepResult};
use crate::llm::providers::LlmRouter;
use crate::llm::{CompletionRequest, DefaultModels, Message, Role};
use planner_schemas::*;

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
    /// DTU Configuration: generates behavioral clone specs for high-priority dependencies.
    DtuConfiguration,
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

/// Complete Ralph output after all modes run.
#[derive(Debug, Clone)]
pub struct RalphOutput {
    /// Additional scenarios from ScenarioAugmentation.
    pub augmented_scenarios: Vec<Scenario>,

    /// Advisory findings from GeneTransfusion.
    pub findings: Vec<RalphFinding>,

    /// ConsequenceCards for high-severity findings.
    pub consequence_cards: Vec<ConsequenceCardV1>,

    /// DTU configurations generated for high-priority dependencies.
    pub dtu_configs: Vec<DtuConfigV1>,
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
            e,
            &content[..content.len().min(300)],
        ))
    })?;

    Ok(json
        .scenarios
        .into_iter()
        .map(|s| Scenario {
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
        })
        .collect())
}

// ---------------------------------------------------------------------------
// GeneTransfusion
// ---------------------------------------------------------------------------

/// Known component patterns and their common pitfalls.
const KNOWN_PATTERNS: &[(&str, &[&str])] = &[
    (
        "auth",
        &[
            "Session fixation: regenerate session ID after login",
            "Password reset tokens must expire and be single-use",
            "Rate-limit login attempts to prevent brute force",
            "Store passwords with bcrypt/argon2, never plain text",
        ],
    ),
    (
        "payment",
        &[
            "Idempotency keys required for all payment operations",
            "Never store raw card numbers — use tokenization",
            "Handle webhook retries gracefully (dedup by event ID)",
            "Implement timeout + reconciliation for async payment flows",
        ],
    ),
    (
        "file-upload",
        &[
            "Validate file type on server side (don't trust Content-Type header)",
            "Enforce max file size before reading entire body",
            "Scan uploaded files for malware in async pipeline",
            "Use pre-signed URLs for direct upload to avoid proxy bottleneck",
        ],
    ),
    (
        "api",
        &[
            "Validate and sanitize all input — never trust client data",
            "Return consistent error format across all endpoints",
            "Implement request timeout to prevent hanging connections",
            "Use pagination for list endpoints — never return unbounded results",
        ],
    ),
    (
        "database",
        &[
            "Use database transactions for multi-step operations",
            "Add indexes for commonly queried columns",
            "Implement connection pooling — don't open/close per request",
            "Use soft deletes for user-facing data to allow recovery",
        ],
    ),
    (
        "realtime",
        &[
            "Handle WebSocket reconnection gracefully with backoff",
            "Implement heartbeat/ping to detect stale connections",
            "Buffer messages during disconnection for replay on reconnect",
            "Rate-limit incoming messages per connection",
        ],
    ),
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
            || spec
                .external_dependencies
                .iter()
                .any(|d| d.name.to_lowercase().contains(pattern_name))
            || spec
                .requirements
                .iter()
                .any(|r| r.statement.to_lowercase().contains(pattern_name));

        if !pattern_present {
            continue;
        }

        // Check which pitfalls are NOT already addressed in the spec
        for pitfall in *pitfalls {
            let pitfall_lower = pitfall.to_lowercase();
            let key_words: Vec<&str> = pitfall_lower
                .split_whitespace()
                .filter(|w| w.len() > 4)
                .take(3)
                .collect();

            let already_addressed = key_words
                .iter()
                .filter(|w| spec_lower.contains(**w))
                .count()
                >= 2;

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
    lower.contains("never")
        || lower.contains("security")
        || lower.contains("password")
        || lower.contains("token")
        || lower.contains("malware")
        || lower.contains("brute force")
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
    findings
        .iter()
        .filter(|f| f.severity == RalphSeverity::High)
        .map(|f| ConsequenceCardV1 {
            card_id: Uuid::new_v4(),
            project_id,
            trigger: CardTrigger::RalphFinding,
            problem: f.description.clone(),
            proposed_solution: f
                .suggestion
                .clone()
                .unwrap_or_else(|| "Review and address".into()),
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
        })
        .collect()
}

// ---------------------------------------------------------------------------
// DTU Configuration Generation (Mode 3)
// ---------------------------------------------------------------------------

/// Known DTU provider mappings — maps dependency names to DTU provider IDs.
const DTU_PROVIDER_MAP: &[(&str, &str)] = &[
    ("stripe", "stripe"),
    ("auth0", "auth0"),
    ("sendgrid", "sendgrid"),
    ("supabase", "supabase"),
    ("twilio", "twilio"),
];

/// DTU Configuration generation prompt.
const DTU_CONFIG_PROMPT: &str = r#"You are Ralph, the DTU Configuration generator for Planner v2.
Your job: analyze how an external dependency is used in the NLSpec and generate
a behavioral clone configuration.

## Output Format
Respond with ONLY a JSON object (no markdown fences):

{
  "behavioral_rules": [
    {
      "id": "RULE-1",
      "endpoint": "/v1/...",
      "method": "POST",
      "behavior": "Plain-English description",
      "state_transitions": [
        {
          "entity_type": "...",
          "from_state": null,
          "to_state": "..."
        }
      ]
    }
  ],
  "seed_state": [
    {
      "entity_type": "...",
      "entity_id": "...",
      "initial_state": {}
    }
  ],
  "failure_modes": [
    {
      "id": "FAIL-1",
      "endpoint": "/v1/...",
      "trigger": "always | nth_request:N | condition",
      "status_code": 500,
      "error_body": {},
      "description": "What this tests"
    }
  ]
}

## Rules
1. Only include endpoints actually referenced in the NLSpec
2. Seed state should support the scenarios in the NLSpec
3. Include 1-3 failure modes for common edge cases
4. behavioral_rules should cover the happy path AND error handling"#;

/// Generate DTU configurations for all high-priority external dependencies.
pub fn generate_dtu_configs_deterministic(spec: &NLSpecV1, project_id: Uuid) -> Vec<DtuConfigV1> {
    spec.external_dependencies
        .iter()
        .filter(|dep| dep.dtu_priority == DtuPriority::High)
        .filter_map(|dep| {
            let dep_lower = dep.name.to_lowercase();
            let provider_id = DTU_PROVIDER_MAP
                .iter()
                .find(|(name, _)| dep_lower.contains(name))
                .map(|(_, id)| id.to_string());

            provider_id.map(|pid| {
                generate_default_dtu_config(project_id, &dep.name, &pid, &dep.usage_description)
            })
        })
        .collect()
}

/// Generate a sensible default DTU config based on the provider type.
fn generate_default_dtu_config(
    project_id: Uuid,
    dependency_name: &str,
    provider_id: &str,
    usage_description: &str,
) -> DtuConfigV1 {
    match provider_id {
        "stripe" => generate_stripe_dtu_config(project_id, dependency_name, usage_description),
        "auth0" => generate_auth0_dtu_config(project_id, dependency_name, usage_description),
        _ => DtuConfigV1 {
            project_id,
            dependency_name: dependency_name.to_string(),
            provider_id: provider_id.to_string(),
            behavioral_rules: vec![],
            seed_state: vec![],
            failure_modes: vec![],
            validated: false,
        },
    }
}

fn generate_stripe_dtu_config(
    project_id: Uuid,
    dependency_name: &str,
    usage_description: &str,
) -> DtuConfigV1 {
    use planner_schemas::{DtuBehavioralRule, DtuFailureMode, DtuSeedEntry, DtuStateTransition};

    let usage_lower = usage_description.to_lowercase();

    let mut rules = vec![DtuBehavioralRule {
        id: "STRIPE-RULE-1".into(),
        endpoint: "/v1/customers".into(),
        method: "POST".into(),
        behavior: "Create a new customer with email and metadata".into(),
        state_transitions: vec![DtuStateTransition {
            entity_type: "customer".into(),
            from_state: None,
            to_state: "active".into(),
        }],
    }];

    // Add payment-specific rules if usage mentions payments
    if usage_lower.contains("payment")
        || usage_lower.contains("charge")
        || usage_lower.contains("checkout")
    {
        rules.push(DtuBehavioralRule {
            id: "STRIPE-RULE-2".into(),
            endpoint: "/v1/payment_intents".into(),
            method: "POST".into(),
            behavior: "Create payment intent with amount and currency. Transitions to requires_payment_method.".into(),
            state_transitions: vec![
                DtuStateTransition {
                    entity_type: "payment_intent".into(),
                    from_state: None,
                    to_state: "requires_payment_method".into(),
                },
            ],
        });
        rules.push(DtuBehavioralRule {
            id: "STRIPE-RULE-3".into(),
            endpoint: "/v1/payment_intents/{id}/confirm".into(),
            method: "POST".into(),
            behavior: "Confirm payment. Auto-capture transitions to succeeded. Manual-capture to requires_capture.".into(),
            state_transitions: vec![
                DtuStateTransition {
                    entity_type: "payment_intent".into(),
                    from_state: Some("requires_payment_method".into()),
                    to_state: "succeeded".into(),
                },
            ],
        });
    }

    let seed_state = vec![DtuSeedEntry {
        entity_type: "customer".into(),
        entity_id: "cus_test_1".into(),
        initial_state: serde_json::json!({
            "id": "cus_test_1",
            "object": "customer",
            "email": "test@example.com",
            "name": "Test Customer"
        }),
    }];

    let failure_modes = vec![
        DtuFailureMode {
            id: "STRIPE-FAIL-1".into(),
            endpoint: "/v1/payment_intents".into(),
            trigger: "amount > 999999".into(),
            status_code: 400,
            error_body: serde_json::json!({
                "error": {
                    "type": "invalid_request_error",
                    "message": "Amount must be no more than $9,999.99"
                }
            }),
            description: "Reject excessively large payment amounts".into(),
        },
        DtuFailureMode {
            id: "STRIPE-FAIL-2".into(),
            endpoint: "/v1/payment_intents".into(),
            trigger: "nth_request:5".into(),
            status_code: 429,
            error_body: serde_json::json!({
                "error": {
                    "type": "rate_limit_error",
                    "message": "Rate limit exceeded"
                }
            }),
            description: "Simulate rate limiting on high-frequency calls".into(),
        },
    ];

    DtuConfigV1 {
        project_id,
        dependency_name: dependency_name.to_string(),
        provider_id: "stripe".to_string(),
        behavioral_rules: rules,
        seed_state,
        failure_modes,
        validated: false,
    }
}

fn generate_auth0_dtu_config(
    project_id: Uuid,
    dependency_name: &str,
    usage_description: &str,
) -> DtuConfigV1 {
    use planner_schemas::{DtuBehavioralRule, DtuFailureMode, DtuSeedEntry, DtuStateTransition};

    let usage_lower = usage_description.to_lowercase();

    let mut rules = vec![
        DtuBehavioralRule {
            id: "AUTH0-RULE-1".into(),
            endpoint: "/oauth/token".into(),
            method: "POST".into(),
            behavior: "Password grant: validate credentials, return access + refresh + id tokens"
                .into(),
            state_transitions: vec![DtuStateTransition {
                entity_type: "token".into(),
                from_state: None,
                to_state: "active".into(),
            }],
        },
        DtuBehavioralRule {
            id: "AUTH0-RULE-2".into(),
            endpoint: "/api/v2/users".into(),
            method: "POST".into(),
            behavior: "Create user with email + password. Reject duplicate emails.".into(),
            state_transitions: vec![DtuStateTransition {
                entity_type: "user".into(),
                from_state: None,
                to_state: "active".into(),
            }],
        },
    ];

    // Add role-based rules if RBAC is mentioned
    if usage_lower.contains("role")
        || usage_lower.contains("permission")
        || usage_lower.contains("rbac")
    {
        rules.push(DtuBehavioralRule {
            id: "AUTH0-RULE-3".into(),
            endpoint: "/api/v2/users/{id}/roles".into(),
            method: "POST".into(),
            behavior: "Assign roles to user. Validate role IDs exist.".into(),
            state_transitions: vec![DtuStateTransition {
                entity_type: "user_role".into(),
                from_state: None,
                to_state: "assigned".into(),
            }],
        });
    }

    let seed_state = vec![DtuSeedEntry {
        entity_type: "user".into(),
        entity_id: "auth0|test_1".into(),
        initial_state: serde_json::json!({
            "user_id": "auth0|test_1",
            "email": "test@example.com",
            "email_verified": true,
            "name": "Test User",
            "_password": "TestPassword123"
        }),
    }];

    let failure_modes = vec![DtuFailureMode {
        id: "AUTH0-FAIL-1".into(),
        endpoint: "/oauth/token".into(),
        trigger: "nth_request:10".into(),
        status_code: 429,
        error_body: serde_json::json!({
            "error": "too_many_requests",
            "error_description": "Rate limit exceeded"
        }),
        description: "Simulate rate limiting on login attempts".into(),
    }];

    DtuConfigV1 {
        project_id,
        dependency_name: dependency_name.to_string(),
        provider_id: "auth0".to_string(),
        behavioral_rules: rules,
        seed_state,
        failure_modes,
        validated: false,
    }
}

/// LLM-enhanced DTU configuration generation (for complex/unknown dependencies).
pub async fn generate_dtu_config_with_llm(
    router: &LlmRouter,
    spec: &NLSpecV1,
    dependency: &ExternalDependency,
    project_id: Uuid,
) -> StepResult<DtuConfigV1> {
    let context = serde_json::json!({
        "dependency_name": dependency.name,
        "usage_description": dependency.usage_description,
        "requirements": spec.requirements.iter().map(|r| &r.statement).collect::<Vec<_>>(),
        "constraints": spec.architectural_constraints,
    });

    let request = CompletionRequest {
        system: Some(DTU_CONFIG_PROMPT.to_string()),
        messages: vec![Message {
            role: Role::User,
            content: format!(
                "Generate a DTU configuration for this dependency:\n\n{}",
                serde_json::to_string_pretty(&context).unwrap_or_default(),
            ),
        }],
        max_tokens: 2048,
        temperature: 0.2,
        model: DefaultModels::RALPH_LOOPS.to_string(),
    };

    let response = router.complete(request).await?;
    parse_dtu_config(&response.content, project_id, &dependency.name)
}

#[derive(Debug, serde::Deserialize)]
struct DtuConfigJson {
    #[serde(default)]
    behavioral_rules: Vec<DtuRuleJson>,
    #[serde(default)]
    seed_state: Vec<DtuSeedJson>,
    #[serde(default)]
    failure_modes: Vec<DtuFailureJson>,
}

#[derive(Debug, serde::Deserialize)]
struct DtuRuleJson {
    id: String,
    endpoint: String,
    method: String,
    behavior: String,
    #[serde(default)]
    state_transitions: Vec<DtuTransitionJson>,
}

#[derive(Debug, serde::Deserialize)]
struct DtuTransitionJson {
    entity_type: String,
    from_state: Option<String>,
    to_state: String,
}

#[derive(Debug, serde::Deserialize)]
struct DtuSeedJson {
    entity_type: String,
    entity_id: String,
    initial_state: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct DtuFailureJson {
    id: String,
    endpoint: String,
    trigger: String,
    status_code: u16,
    error_body: serde_json::Value,
    description: String,
}

fn parse_dtu_config(
    content: &str,
    project_id: Uuid,
    dependency_name: &str,
) -> StepResult<DtuConfigV1> {
    use planner_schemas::{DtuBehavioralRule, DtuFailureMode, DtuSeedEntry, DtuStateTransition};

    let cleaned = super::intake::strip_code_fences(content);
    let json: DtuConfigJson = serde_json::from_str(&cleaned).map_err(|e| {
        StepError::JsonError(format!(
            "Failed to parse DTU config response: {}. Raw: {}",
            e,
            &content[..content.len().min(300)],
        ))
    })?;

    let dep_lower = dependency_name.to_lowercase();
    let provider_id = DTU_PROVIDER_MAP
        .iter()
        .find(|(name, _)| dep_lower.contains(name))
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| dep_lower.replace(' ', "_"));

    Ok(DtuConfigV1 {
        project_id,
        dependency_name: dependency_name.to_string(),
        provider_id,
        behavioral_rules: json
            .behavioral_rules
            .into_iter()
            .map(|r| DtuBehavioralRule {
                id: r.id,
                endpoint: r.endpoint,
                method: r.method,
                behavior: r.behavior,
                state_transitions: r
                    .state_transitions
                    .into_iter()
                    .map(|t| DtuStateTransition {
                        entity_type: t.entity_type,
                        from_state: t.from_state,
                        to_state: t.to_state,
                    })
                    .collect(),
            })
            .collect(),
        seed_state: json
            .seed_state
            .into_iter()
            .map(|s| DtuSeedEntry {
                entity_type: s.entity_type,
                entity_id: s.entity_id,
                initial_state: s.initial_state,
            })
            .collect(),
        failure_modes: json
            .failure_modes
            .into_iter()
            .map(|f| DtuFailureMode {
                id: f.id,
                endpoint: f.endpoint,
                trigger: f.trigger,
                status_code: f.status_code,
                error_body: f.error_body,
                description: f.description,
            })
            .collect(),
        validated: false,
    })
}

// ---------------------------------------------------------------------------
// Full Ralph execution
// ---------------------------------------------------------------------------

/// Run the full Ralph loop: ScenarioAugmentation + GeneTransfusion + DTU Configuration.
///
/// Returns augmented scenarios, advisory findings, ConsequenceCards,
/// and DTU configurations for the Impact Inbox.
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

    // Mode 3: DTU Configuration (deterministic for known providers)
    tracing::info!("  Ralph DTU Configuration...");
    let dtu_configs = generate_dtu_configs_deterministic(spec, project_id);
    tracing::info!("    → {} DTU configuration(s) generated", dtu_configs.len());

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
        dtu_configs,
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
            sacred_anchors: Some(vec![NLSpecAnchor {
                id: "SA-1".into(),
                statement: "Credentials must be securely stored".into(),
            }]),
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
            definition_of_done: vec![DoDItem {
                criterion: "User can sign up and login".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Login with valid credentials succeeds".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
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
            statement: "The system must rate-limit login attempts to prevent brute force attacks"
                .into(),
            priority: Priority::Must,
            traces_to: vec!["SA-1".into()],
        });

        let findings = gene_transfusion(&spec);

        // "rate-limit login attempts" pitfall should NOT appear
        assert!(!findings
            .iter()
            .any(|f| f.description.to_lowercase().contains("rate-limit login")));
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

    // -----------------------------------------------------------------------
    // DTU Configuration tests
    // -----------------------------------------------------------------------

    fn make_spec_with_stripe_dep() -> NLSpecV1 {
        let mut spec = make_auth_spec();
        spec.external_dependencies = vec![ExternalDependency {
            name: "Stripe".into(),
            usage_description: "Process payment intents and charge customers".into(),
            dtu_priority: DtuPriority::High,
        }];
        spec
    }

    fn make_spec_with_auth0_dep() -> NLSpecV1 {
        let mut spec = make_auth_spec();
        spec.external_dependencies = vec![ExternalDependency {
            name: "Auth0".into(),
            usage_description:
                "User authentication with role-based access control and RBAC permissions".into(),
            dtu_priority: DtuPriority::High,
        }];
        spec
    }

    fn make_spec_with_mixed_deps() -> NLSpecV1 {
        let mut spec = make_auth_spec();
        spec.external_dependencies = vec![
            ExternalDependency {
                name: "Stripe Payments".into(),
                usage_description: "Checkout flow for subscriptions".into(),
                dtu_priority: DtuPriority::High,
            },
            ExternalDependency {
                name: "Auth0".into(),
                usage_description: "Login and signup with permissions".into(),
                dtu_priority: DtuPriority::Low,
            },
            ExternalDependency {
                name: "Redis".into(),
                usage_description: "Caching layer".into(),
                dtu_priority: DtuPriority::None,
            },
        ];
        spec
    }

    #[test]
    fn dtu_config_stripe_generates_rules() {
        let spec = make_spec_with_stripe_dep();
        let project_id = Uuid::new_v4();
        let configs = generate_dtu_configs_deterministic(&spec, project_id);

        assert_eq!(configs.len(), 1);
        let cfg = &configs[0];
        assert_eq!(cfg.provider_id, "stripe");
        assert_eq!(cfg.dependency_name, "Stripe");
        assert_eq!(cfg.project_id, project_id);
        assert!(!cfg.validated);

        // Should have customer rule + payment rules (usage mentions "payment")
        assert!(cfg.behavioral_rules.len() >= 2);
        assert!(cfg
            .behavioral_rules
            .iter()
            .any(|r| r.endpoint.contains("customers")));
        assert!(cfg
            .behavioral_rules
            .iter()
            .any(|r| r.endpoint.contains("payment_intents")));

        // Seed state
        assert!(!cfg.seed_state.is_empty());
        assert_eq!(cfg.seed_state[0].entity_type, "customer");

        // Failure modes
        assert!(cfg.failure_modes.len() >= 2);
        assert!(cfg.failure_modes.iter().any(|f| f.status_code == 400));
        assert!(cfg.failure_modes.iter().any(|f| f.status_code == 429));
    }

    #[test]
    fn dtu_config_auth0_generates_rules() {
        let spec = make_spec_with_auth0_dep();
        let project_id = Uuid::new_v4();
        let configs = generate_dtu_configs_deterministic(&spec, project_id);

        assert_eq!(configs.len(), 1);
        let cfg = &configs[0];
        assert_eq!(cfg.provider_id, "auth0");
        assert_eq!(cfg.dependency_name, "Auth0");

        // Should have token + user rules + RBAC rule (usage mentions "role")
        assert!(cfg.behavioral_rules.len() >= 3);
        assert!(cfg
            .behavioral_rules
            .iter()
            .any(|r| r.endpoint.contains("oauth/token")));
        assert!(cfg
            .behavioral_rules
            .iter()
            .any(|r| r.endpoint.contains("users")));
        assert!(cfg
            .behavioral_rules
            .iter()
            .any(|r| r.endpoint.contains("roles")));

        // Seed state
        assert!(!cfg.seed_state.is_empty());
        assert_eq!(cfg.seed_state[0].entity_type, "user");
    }

    #[test]
    fn dtu_config_filters_by_priority() {
        let spec = make_spec_with_mixed_deps();
        let project_id = Uuid::new_v4();
        let configs = generate_dtu_configs_deterministic(&spec, project_id);

        // Only Stripe is High priority — Auth0 is Low, Redis is None
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].provider_id, "stripe");
    }

    #[test]
    fn dtu_config_no_deps_returns_empty() {
        let spec = make_auth_spec(); // no external_dependencies
        let project_id = Uuid::new_v4();
        let configs = generate_dtu_configs_deterministic(&spec, project_id);
        assert!(configs.is_empty());
    }

    #[test]
    fn dtu_config_unknown_provider_skipped() {
        let mut spec = make_auth_spec();
        spec.external_dependencies = vec![ExternalDependency {
            name: "SomeCustomAPI".into(),
            usage_description: "Internal microservice".into(),
            dtu_priority: DtuPriority::High,
        }];
        let configs = generate_dtu_configs_deterministic(&spec, Uuid::new_v4());
        // Unknown provider not in DTU_PROVIDER_MAP → skipped
        assert!(configs.is_empty());
    }

    #[test]
    fn dtu_config_stripe_no_payment_usage_fewer_rules() {
        let mut spec = make_auth_spec();
        spec.external_dependencies = vec![ExternalDependency {
            name: "Stripe".into(),
            usage_description: "Customer management only".into(),
            dtu_priority: DtuPriority::High,
        }];
        let configs = generate_dtu_configs_deterministic(&spec, Uuid::new_v4());
        assert_eq!(configs.len(), 1);
        // Only customer rule (no payment_intents rules since usage doesn't mention payments)
        assert_eq!(configs[0].behavioral_rules.len(), 1);
        assert!(configs[0].behavioral_rules[0]
            .endpoint
            .contains("customers"));
    }

    #[test]
    fn parse_dtu_config_valid_json() {
        let content = r#"{
            "behavioral_rules": [
                {
                    "id": "RULE-1",
                    "endpoint": "/v1/things",
                    "method": "POST",
                    "behavior": "Create a thing",
                    "state_transitions": [
                        {
                            "entity_type": "thing",
                            "from_state": null,
                            "to_state": "created"
                        }
                    ]
                }
            ],
            "seed_state": [
                {
                    "entity_type": "thing",
                    "entity_id": "thing_1",
                    "initial_state": {"id": "thing_1", "status": "active"}
                }
            ],
            "failure_modes": [
                {
                    "id": "FAIL-1",
                    "endpoint": "/v1/things",
                    "trigger": "always",
                    "status_code": 500,
                    "error_body": {"error": "server_error"},
                    "description": "Simulated server error"
                }
            ]
        }"#;

        let result = parse_dtu_config(content, Uuid::new_v4(), "TestAPI");
        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert_eq!(cfg.behavioral_rules.len(), 1);
        assert_eq!(cfg.seed_state.len(), 1);
        assert_eq!(cfg.failure_modes.len(), 1);
        assert_eq!(cfg.provider_id, "testapi"); // lowercased + no spaces
        assert!(!cfg.validated);
    }

    #[test]
    fn parse_dtu_config_with_code_fences() {
        let content =
            "```json\n{\"behavioral_rules\": [], \"seed_state\": [], \"failure_modes\": []}\n```";
        let result = parse_dtu_config(content, Uuid::new_v4(), "Stripe");
        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert!(cfg.behavioral_rules.is_empty());
        assert_eq!(cfg.provider_id, "stripe");
    }

    #[test]
    fn parse_dtu_config_invalid_json_errors() {
        let content = "this is not json at all";
        let result = parse_dtu_config(content, Uuid::new_v4(), "Stripe");
        assert!(result.is_err());
    }

    #[test]
    fn dtu_config_state_transitions_are_populated() {
        let spec = make_spec_with_stripe_dep();
        let configs = generate_dtu_configs_deterministic(&spec, Uuid::new_v4());
        let cfg = &configs[0];

        for rule in &cfg.behavioral_rules {
            assert!(
                !rule.state_transitions.is_empty(),
                "Rule {} has no state transitions",
                rule.id
            );
            for transition in &rule.state_transitions {
                assert!(!transition.entity_type.is_empty());
                assert!(!transition.to_state.is_empty());
            }
        }
    }
}
