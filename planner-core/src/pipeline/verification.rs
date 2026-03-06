//! # Formal Verification — Lean4 Proposition Generation
//!
//! Generates Lean4 propositions for critical invariants identified in the NLSpec.
//! These propositions are templates with `sorry` placeholders that can be filled in
//! with actual proofs by a formal methods team or verified interactively in the
//! Lean4 prover.
//!
//! ## Focus Areas
//! 1. **State machine invariants**: Payment state transitions are valid
//! 2. **Uniqueness constraints**: No duplicate IDs in generated artifacts
//! 3. **Completeness**: Every Sacred Anchor traces to at least one requirement
//! 4. **Coverage**: Every satisfaction criterion maps to at least one scenario
//! 5. **DAG integrity**: Turn parent-child relationships form a valid DAG

use uuid::Uuid;

use planner_schemas::*;

// ---------------------------------------------------------------------------
// Lean4 Proposition Types
// ---------------------------------------------------------------------------

/// A generated Lean4 proposition template.
#[derive(Debug, Clone)]
pub struct Lean4Proposition {
    /// Unique proposition ID (e.g. "PROP-SA-TRACE-1").
    pub id: String,

    /// Category of the proposition.
    pub category: PropositionCategory,

    /// Human-readable description of what this proves.
    pub description: String,

    /// The Lean4 source code (theorem statement, no proof body).
    pub lean4_source: String,

    /// Which spec element(s) this relates to.
    pub traces_to: Vec<String>,
}

/// Categories of formal propositions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionCategory {
    /// Sacred anchor traceability.
    AnchorTraceability,
    /// State machine validity.
    StateMachineInvariant,
    /// Uniqueness constraints.
    Uniqueness,
    /// Coverage completeness.
    Coverage,
    /// DAG structural integrity.
    DagIntegrity,
}

// ---------------------------------------------------------------------------
// Proposition Generation
// ---------------------------------------------------------------------------

/// Generate Lean4 proposition templates from an NLSpec.
pub fn generate_propositions(spec: &NLSpecV1) -> Vec<Lean4Proposition> {
    let mut props = Vec::new();
    let mut idx = 0u32;

    // 1. Anchor traceability: every sacred anchor traces to ≥1 requirement
    if let Some(ref anchors) = spec.sacred_anchors {
        for anchor in anchors {
            idx += 1;
            props.push(Lean4Proposition {
                id: format!("PROP-SA-TRACE-{}", idx),
                category: PropositionCategory::AnchorTraceability,
                description: format!(
                    "Sacred anchor '{}' traces to at least one requirement",
                    anchor.id
                ),
                lean4_source: format!(
                    r#"-- Sacred anchor traceability: {}
theorem anchor_{}_has_trace :
  ∃ (r : Requirement), r.traces_to.contains "{}" := by
  sorry -- proof stub"#,
                    anchor.statement,
                    anchor.id.replace('-', "_").to_lowercase(),
                    anchor.id
                ),
                traces_to: vec![anchor.id.clone()],
            });
        }
    }

    // 2. Requirement ID uniqueness
    idx += 1;
    let req_ids: Vec<&str> = spec.requirements.iter().map(|r| r.id.as_str()).collect();
    props.push(Lean4Proposition {
        id: format!("PROP-UNIQUE-REQ-{}", idx),
        category: PropositionCategory::Uniqueness,
        description: "All requirement IDs are unique".into(),
        lean4_source: format!(
            r#"-- Requirement ID uniqueness
-- Requirements: {:?}
theorem req_ids_unique :
  List.Nodup [{ids}] := by
  sorry -- proof stub"#,
            req_ids,
            ids = req_ids
                .iter()
                .map(|id| format!("\"{}\"", id))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        traces_to: req_ids.iter().map(|id| id.to_string()).collect(),
    });

    // 3. Satisfaction criteria coverage: each criterion maps to ≥1 scenario (stub)
    for sc in &spec.satisfaction_criteria {
        idx += 1;
        props.push(Lean4Proposition {
            id: format!("PROP-COVERAGE-{}", idx),
            category: PropositionCategory::Coverage,
            description: format!(
                "Satisfaction criterion '{}' is covered by at least one scenario",
                sc.id
            ),
            lean4_source: format!(
                r#"-- Coverage: satisfaction criterion {}
-- Description: {}
theorem criterion_{}_covered :
  ∃ (s : Scenario), s.source_criterion = some "{}" := by
  sorry -- proof stub"#,
                sc.id,
                sc.description,
                sc.id.replace('-', "_").to_lowercase(),
                sc.id,
            ),
            traces_to: vec![sc.id.clone()],
        });
    }

    // 4. DTU state machine invariants for high-priority dependencies
    for dep in &spec.external_dependencies {
        if dep.dtu_priority == DtuPriority::High {
            idx += 1;
            props.push(Lean4Proposition {
                id: format!("PROP-FSM-{}", idx),
                category: PropositionCategory::StateMachineInvariant,
                description: format!(
                    "DTU '{}' state transitions are valid (no invalid transitions)",
                    dep.name
                ),
                lean4_source: format!(
                    r#"-- State machine invariant: {}
-- Usage: {}
theorem dtu_{}_valid_transitions :
  ∀ (s : State) (t : Transition),
    valid_transition s t →
    valid_state (apply_transition s t) := by
  sorry -- proof stub"#,
                    dep.name,
                    dep.usage_description,
                    dep.name.to_lowercase().replace(' ', "_"),
                ),
                traces_to: vec![dep.name.clone()],
            });
        }
    }

    // 5. DAG integrity (always generated)
    idx += 1;
    props.push(Lean4Proposition {
        id: format!("PROP-DAG-{}", idx),
        category: PropositionCategory::DagIntegrity,
        description: "Turn parent-child relationships form a valid DAG (no cycles)".into(),
        lean4_source: r#"-- DAG integrity: no cycles in turn parent chain
theorem turn_dag_acyclic :
  ∀ (t : Turn),
    ¬ (t ∈ ancestors t) := by
  sorry -- proof stub"#
            .into(),
        traces_to: vec!["CXDB".into()],
    });

    props
}

/// Render all propositions into a single Lean4 file.
pub fn render_lean4_file(
    props: &[Lean4Proposition],
    project_name: &str,
    _project_id: Uuid,
) -> String {
    let mut lines = Vec::new();

    lines.push(format!("-- Planner v2 Formal Verification Stubs"));
    lines.push(format!("-- Project: {}", project_name));
    lines.push(format!("-- Generated by Ralph formal verification module"));
    lines.push(format!("-- {} propositions total", props.len()));
    lines.push(String::new());
    lines.push("-- These are theorem STUBS. Replace `sorry` with actual proofs.".into());
    lines.push("-- Run with: `lean --run Planner.lean`".into());
    lines.push(String::new());

    // Group by category
    let categories = [
        (
            PropositionCategory::AnchorTraceability,
            "Sacred Anchor Traceability",
        ),
        (PropositionCategory::Uniqueness, "Uniqueness Constraints"),
        (PropositionCategory::Coverage, "Coverage Completeness"),
        (
            PropositionCategory::StateMachineInvariant,
            "State Machine Invariants",
        ),
        (PropositionCategory::DagIntegrity, "DAG Integrity"),
    ];

    for (cat, label) in &categories {
        let cat_props: Vec<&Lean4Proposition> =
            props.iter().filter(|p| &p.category == cat).collect();

        if cat_props.is_empty() {
            continue;
        }

        lines.push(format!(
            "-- ========================================================================="
        ));
        lines.push(format!("-- {}", label));
        lines.push(format!(
            "-- ========================================================================="
        ));
        lines.push(String::new());

        for prop in cat_props {
            lines.push(format!("-- [{}] {}", prop.id, prop.description));
            lines.push(prop.lean4_source.clone());
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 50,
            created_from: "test".into(),
            intent_summary: Some("Test project".into()),
            sacred_anchors: Some(vec![
                NLSpecAnchor {
                    id: "SA-1".into(),
                    statement: "Data must be encrypted at rest".into(),
                },
                NLSpecAnchor {
                    id: "SA-2".into(),
                    statement: "All actions must be auditable".into(),
                },
            ]),
            requirements: vec![
                Requirement {
                    id: "FR-1".into(),
                    statement: "Encrypt data".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-1".into()],
                },
                Requirement {
                    id: "FR-2".into(),
                    statement: "Audit trail".into(),
                    priority: Priority::Must,
                    traces_to: vec!["SA-2".into()],
                },
            ],
            architectural_constraints: vec![],
            phase1_contracts: None,
            external_dependencies: vec![ExternalDependency {
                name: "Stripe".into(),
                usage_description: "Payment processing".into(),
                dtu_priority: DtuPriority::High,
            }],
            definition_of_done: vec![DoDItem {
                criterion: "All data encrypted".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Payments succeed".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec![],
            amendment_log: vec![],
        }
    }

    #[test]
    fn generates_anchor_traceability_props() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);

        let anchor_props: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::AnchorTraceability)
            .collect();
        assert_eq!(anchor_props.len(), 2); // SA-1 and SA-2
        assert!(anchor_props[0].lean4_source.contains("sorry"));
    }

    #[test]
    fn generates_uniqueness_prop() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);

        let unique_props: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::Uniqueness)
            .collect();
        assert_eq!(unique_props.len(), 1);
        assert!(unique_props[0].lean4_source.contains("FR-1"));
        assert!(unique_props[0].lean4_source.contains("FR-2"));
    }

    #[test]
    fn generates_coverage_props() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);

        let coverage: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::Coverage)
            .collect();
        assert_eq!(coverage.len(), 1); // SC-1
    }

    #[test]
    fn generates_fsm_prop_for_high_priority_dep() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);

        let fsm: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::StateMachineInvariant)
            .collect();
        assert_eq!(fsm.len(), 1); // Stripe (High priority)
        assert!(fsm[0].lean4_source.contains("Stripe"));
    }

    #[test]
    fn generates_dag_integrity_prop() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);

        let dag: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::DagIntegrity)
            .collect();
        assert_eq!(dag.len(), 1);
        assert!(dag[0].lean4_source.contains("acyclic"));
    }

    #[test]
    fn no_fsm_prop_for_low_priority_dep() {
        let mut spec = make_test_spec();
        spec.external_dependencies = vec![ExternalDependency {
            name: "Redis".into(),
            usage_description: "Caching".into(),
            dtu_priority: DtuPriority::Low,
        }];

        let props = generate_propositions(&spec);
        let fsm: Vec<_> = props
            .iter()
            .filter(|p| p.category == PropositionCategory::StateMachineInvariant)
            .collect();
        assert!(fsm.is_empty());
    }

    #[test]
    fn render_lean4_file_contains_all_sections() {
        let spec = make_test_spec();
        let props = generate_propositions(&spec);
        let rendered = render_lean4_file(&props, "Test Project", Uuid::new_v4());

        assert!(rendered.contains("Sacred Anchor Traceability"));
        assert!(rendered.contains("Uniqueness Constraints"));
        assert!(rendered.contains("Coverage Completeness"));
        assert!(rendered.contains("State Machine Invariants"));
        assert!(rendered.contains("DAG Integrity"));
        assert!(rendered.contains("sorry"));
    }

    #[test]
    fn empty_spec_still_generates_dag_and_uniqueness() {
        let mut spec = make_test_spec();
        spec.sacred_anchors = None;
        spec.external_dependencies = vec![];
        spec.satisfaction_criteria = vec![];

        let props = generate_propositions(&spec);
        // Should still have uniqueness + DAG
        assert!(props
            .iter()
            .any(|p| p.category == PropositionCategory::Uniqueness));
        assert!(props
            .iter()
            .any(|p| p.category == PropositionCategory::DagIntegrity));
    }
}
