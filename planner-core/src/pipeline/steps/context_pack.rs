//! # Context Packs — Token-Budgeted LLM Context Assembly
//!
//! Instead of raw-serializing entire artifacts into LLM prompts, Context Packs
//! build a priority-ordered slice of relevant information that fits within
//! a given token budget.
//!
//! Each pipeline step declares what context it needs (requirements, anchors,
//! contracts, scenarios, etc.) and the Context Pack builder selects the
//! highest-priority items that fit within the budget.
//!
//! This prevents token overflow for large multi-chunk projects while ensuring
//! the most important context is always included.

use planner_schemas::*;

// ---------------------------------------------------------------------------
// ContextPackV1 — the assembled context for one LLM call
// ---------------------------------------------------------------------------

/// A token-budgeted context pack assembled for a specific LLM call.
#[derive(Debug, Clone)]
pub struct ContextPackV1 {
    /// Human-readable label for this context pack (e.g. "spec-compiler:auth").
    pub label: String,

    /// Estimated token count of the assembled content.
    pub estimated_tokens: u32,

    /// Maximum token budget this pack was built against.
    pub token_budget: u32,

    /// Whether the pack was truncated to fit the budget.
    pub was_truncated: bool,

    /// The assembled sections, in priority order.
    pub sections: Vec<ContextSection>,
}

/// A single section within a context pack.
#[derive(Debug, Clone)]
pub struct ContextSection {
    /// Section name (e.g. "sacred_anchors", "requirements", "phase1_contracts").
    pub name: String,

    /// Priority tier: 0 = must include, 1 = high priority, 2 = nice to have.
    pub priority: u8,

    /// The serialized content for this section.
    pub content: String,

    /// Estimated token count for this section.
    pub estimated_tokens: u32,

    /// Whether this section was truncated.
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Rough token estimation: ~4 characters per token for English text.
/// Conservative to avoid overflow — better to under-estimate and leave room.
const CHARS_PER_TOKEN: u32 = 4;

/// Estimate token count from a string.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32) / CHARS_PER_TOKEN + 1
}

// ---------------------------------------------------------------------------
// Context Pack Builder
// ---------------------------------------------------------------------------

/// What type of context this pack is for.
#[derive(Debug, Clone)]
pub enum ContextTarget {
    /// Compiling a spec (needs intake, anchors).
    SpecCompiler,
    /// Compiling a domain chunk (needs root contracts + anchors + domain context).
    DomainCompiler { domain_name: String },
    /// Adversarial Review (needs full spec).
    AdversarialReview,
    /// Scenario Generation (needs anchors, requirements, satisfaction criteria).
    ScenarioGenerator,
    /// Graph.dot compilation (needs spec + requirements).
    GraphDotCompiler,
}

/// Build a context pack for an NLSpec, prioritizing sections by the target.
pub fn build_spec_context_pack(
    spec: &NLSpecV1,
    target: ContextTarget,
    token_budget: u32,
) -> ContextPackV1 {
    let label = match &target {
        ContextTarget::SpecCompiler => "spec-compiler".to_string(),
        ContextTarget::DomainCompiler { domain_name } => format!("domain-compiler:{}", domain_name),
        ContextTarget::AdversarialReview => "ar-review".to_string(),
        ContextTarget::ScenarioGenerator => "scenario-gen".to_string(),
        ContextTarget::GraphDotCompiler => "graph-dot".to_string(),
    };

    // Build sections in priority order based on target
    let mut sections = Vec::new();

    // Priority 0: Always include Sacred Anchors (for any target)
    if let Some(ref anchors) = spec.sacred_anchors {
        let content = anchors.iter()
            .map(|a| format!("{}: {}", a.id, a.statement))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(ContextSection {
            name: "sacred_anchors".into(),
            priority: 0,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 0: Intent Summary
    if let Some(ref intent) = spec.intent_summary {
        sections.push(ContextSection {
            name: "intent_summary".into(),
            priority: 0,
            estimated_tokens: estimate_tokens(intent),
            content: intent.clone(),
            truncated: false,
        });
    }

    // Priority 1: Requirements
    {
        let content = spec.requirements.iter()
            .map(|r| format!("{} [{}]: {} (traces: {:?})",
                r.id, format!("{:?}", r.priority), r.statement, r.traces_to))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(ContextSection {
            name: "requirements".into(),
            priority: 1,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 1: Phase 1 Contracts (for domain/spec compilation)
    if let Some(ref contracts) = spec.phase1_contracts {
        let content = contracts.iter()
            .map(|c| format!("{} = {} (consumed by: {:?})", c.name, c.type_definition, c.consumed_by))
            .collect::<Vec<_>>()
            .join("\n");
        let priority = match &target {
            ContextTarget::DomainCompiler { .. } => 0, // Critical for domain compilation
            _ => 1,
        };
        sections.push(ContextSection {
            name: "phase1_contracts".into(),
            priority,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 1: Satisfaction Criteria
    {
        let content = spec.satisfaction_criteria.iter()
            .map(|sc| format!("{} [{:?}]: {}", sc.id, sc.tier_hint, sc.description))
            .collect::<Vec<_>>()
            .join("\n");
        let priority = match &target {
            ContextTarget::ScenarioGenerator => 0, // Critical for scenario gen
            _ => 1,
        };
        sections.push(ContextSection {
            name: "satisfaction_criteria".into(),
            priority,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 2: Architectural Constraints
    if !spec.architectural_constraints.is_empty() {
        let content = spec.architectural_constraints.join("\n");
        sections.push(ContextSection {
            name: "architectural_constraints".into(),
            priority: 2,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 2: Definition of Done
    if !spec.definition_of_done.is_empty() {
        let content = spec.definition_of_done.iter()
            .map(|d| format!("[{}] {}",
                if d.mechanically_checkable { "mechanical" } else { "manual" },
                d.criterion))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(ContextSection {
            name: "definition_of_done".into(),
            priority: 2,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 2: Out of Scope
    if !spec.out_of_scope.is_empty() {
        let content = spec.out_of_scope.join("\n");
        sections.push(ContextSection {
            name: "out_of_scope".into(),
            priority: 2,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Priority 2: External Dependencies
    if !spec.external_dependencies.is_empty() {
        let content = spec.external_dependencies.iter()
            .map(|d| format!("{} [{:?}]: {}", d.name, d.dtu_priority, d.usage_description))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(ContextSection {
            name: "external_dependencies".into(),
            priority: 2,
            estimated_tokens: estimate_tokens(&content),
            content,
            truncated: false,
        });
    }

    // Sort by priority
    sections.sort_by_key(|s| s.priority);

    // Slice to fit budget
    let mut total_tokens: u32 = 0;
    let mut was_truncated = false;
    let mut included_sections = Vec::new();

    for section in sections {
        if total_tokens + section.estimated_tokens <= token_budget {
            total_tokens += section.estimated_tokens;
            included_sections.push(section);
        } else if section.priority == 0 {
            // Must-include sections: truncate content to fit
            let remaining_budget = token_budget.saturating_sub(total_tokens);
            let max_chars = (remaining_budget * CHARS_PER_TOKEN) as usize;
            let truncated_content = if section.content.len() > max_chars {
                format!("{}... [truncated]", &section.content[..max_chars])
            } else {
                section.content.clone()
            };
            let truncated_tokens = estimate_tokens(&truncated_content);
            total_tokens += truncated_tokens;
            included_sections.push(ContextSection {
                content: truncated_content,
                estimated_tokens: truncated_tokens,
                truncated: true,
                ..section
            });
            was_truncated = true;
        } else {
            was_truncated = true;
            // Skip lower-priority sections that don't fit
        }
    }

    ContextPackV1 {
        label,
        estimated_tokens: total_tokens,
        token_budget,
        was_truncated,
        sections: included_sections,
    }
}

/// Render a context pack into a single string for inclusion in an LLM prompt.
pub fn render_context_pack(pack: &ContextPackV1) -> String {
    let mut output = Vec::new();

    for section in &pack.sections {
        output.push(format!("## {}", section.name.replace('_', " ").to_uppercase()));
        output.push(section.content.clone());
        output.push(String::new());
    }

    if pack.was_truncated {
        output.push(format!(
            "[Context truncated to fit {} token budget — some lower-priority sections omitted]",
            pack.token_budget,
        ));
    }

    output.join("\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_test_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 100,
            created_from: "test".into(),
            intent_summary: Some("Build a task tracker".into()),
            sacred_anchors: Some(vec![
                NLSpecAnchor { id: "SA-1".into(), statement: "Data must persist".into() },
            ]),
            requirements: vec![
                Requirement {
                    id: "FR-1".into(),
                    statement: "Must save to localStorage".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-1".into()],
                },
            ],
            architectural_constraints: vec!["React single file".into()],
            phase1_contracts: Some(vec![
                Phase1Contract {
                    name: "Task".into(),
                    type_definition: "{ id: string, done: boolean }".into(),
                    consumed_by: vec!["ui".into()],
                },
            ]),
            external_dependencies: vec![],
            definition_of_done: vec![
                DoDItem { criterion: "Tasks persist".into(), mechanically_checkable: true },
            ],
            satisfaction_criteria: vec![
                SatisfactionCriterion {
                    id: "SC-1".into(),
                    description: "Add task survives refresh".into(),
                    tier_hint: ScenarioTierHint::Critical,
                },
            ],
            open_questions: vec![],
            out_of_scope: vec!["Cloud sync".into()],
            amendment_log: vec![],
        }
    }

    #[test]
    fn estimate_tokens_basic() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars / 4 + 1
        assert_eq!(estimate_tokens(""), 1); // minimum 1
    }

    #[test]
    fn build_spec_context_pack_includes_all_sections() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 10000);

        assert!(!pack.was_truncated);
        assert!(pack.sections.len() >= 5); // anchors, intent, reqs, contracts, sat criteria, etc.
        assert!(pack.estimated_tokens > 0);
        assert!(pack.estimated_tokens <= 10000);
    }

    #[test]
    fn build_spec_context_pack_truncates_on_small_budget() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 20);

        // With only 20 tokens, can't fit everything
        assert!(pack.was_truncated);
        // Priority 0 sections should still be included (possibly truncated)
        assert!(!pack.sections.is_empty());
    }

    #[test]
    fn domain_compiler_prioritizes_contracts() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(
            &spec,
            ContextTarget::DomainCompiler { domain_name: "auth".into() },
            10000,
        );

        // Phase 1 contracts should be priority 0 for domain compilation
        let contracts_section = pack.sections.iter().find(|s| s.name == "phase1_contracts");
        assert!(contracts_section.is_some());
        assert_eq!(contracts_section.unwrap().priority, 0);
    }

    #[test]
    fn scenario_gen_prioritizes_sat_criteria() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(&spec, ContextTarget::ScenarioGenerator, 10000);

        let sc_section = pack.sections.iter().find(|s| s.name == "satisfaction_criteria");
        assert!(sc_section.is_some());
        assert_eq!(sc_section.unwrap().priority, 0);
    }

    #[test]
    fn render_context_pack_produces_readable_output() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 10000);
        let rendered = render_context_pack(&pack);

        assert!(rendered.contains("SACRED ANCHORS"));
        assert!(rendered.contains("INTENT SUMMARY"));
        assert!(rendered.contains("REQUIREMENTS"));
        assert!(!rendered.contains("[Context truncated")); // Not truncated
    }

    #[test]
    fn render_truncated_pack_shows_notice() {
        let spec = make_test_spec();
        let pack = build_spec_context_pack(&spec, ContextTarget::SpecCompiler, 20);
        let rendered = render_context_pack(&pack);

        assert!(rendered.contains("[Context truncated"));
    }
}
