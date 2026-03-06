//! # Constitution — Rule Loading + Self-Critique Evaluation
//!
//! Loads the interviewer constitution and provides self-critique
//! evaluation for generated questions. The constitution is a configurable
//! text artifact that can be edited per project.

use planner_schemas::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load the constitution for a session.
///
/// In v1, this loads the default constitution. Future versions will
/// support file-based constitutions (.toml or .md in project root).
pub fn load_constitution() -> InterviewerConstitution {
    InterviewerConstitution::default_constitution()
}

/// Evaluate a generated question against the constitution.
///
/// Returns a list of violated rules (empty if the question passes).
/// This is used by the question planner for self-critique before
/// presenting a question to the user.
pub fn evaluate_question(
    question: &str,
    target_dimension: &Dimension,
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
) -> Vec<ConstitutionViolation> {
    let mut violations = Vec::new();

    for rule in constitution.all_rules() {
        if let Some(violation) = check_rule(rule, question, target_dimension, state) {
            violations.push(violation);
        }
    }

    violations
}

/// A detected constitution violation.
#[derive(Debug, Clone)]
pub struct ConstitutionViolation {
    /// Which rule was violated.
    pub rule_id: u32,
    /// Category of the rule.
    pub category: RuleCategory,
    /// What went wrong.
    pub explanation: String,
}

// ---------------------------------------------------------------------------
// Rule checking (heuristic — not LLM-based in v1)
// ---------------------------------------------------------------------------

fn check_rule(
    rule: &ConstitutionRule,
    question: &str,
    target_dimension: &Dimension,
    state: &RequirementsBeliefState,
) -> Option<ConstitutionViolation> {
    match rule.id {
        // Rule 2: Never ask more than one question per turn
        2 => {
            let question_marks = question.matches('?').count();
            if question_marks > 1 {
                return Some(ConstitutionViolation {
                    rule_id: 2,
                    category: RuleCategory::Behavioral,
                    explanation: format!(
                        "Question contains {} question marks — should be exactly 1",
                        question_marks
                    ),
                });
            }
        }

        // Rule 4: Never ask about implementation until functional scope is established
        4 => {
            let implementation_dims = [
                Dimension::TechStack,
                Dimension::Performance,
                Dimension::Scalability,
                Dimension::Platform,
            ];
            let functional_dims = [
                Dimension::Goal,
                Dimension::CoreFeatures,
                Dimension::UserFlows,
            ];

            if implementation_dims.contains(target_dimension) {
                let functional_covered =
                    functional_dims.iter().any(|d| state.filled.contains_key(d));
                if !functional_covered {
                    return Some(ConstitutionViolation {
                        rule_id: 4,
                        category: RuleCategory::Behavioral,
                        explanation: format!(
                            "Asking about {} before functional scope (Goal/CoreFeatures/UserFlows) is established",
                            target_dimension.label()
                        ),
                    });
                }
            }
        }

        // Rule 7: Session incomplete without security, error handling, success criteria
        7 => {
            // This is a coverage check — not a per-question violation
            // Checked at convergence time, not per-question
        }

        // Rule 10: After 3 filled dimensions, offer a speculative draft
        10 => {
            // This is handled by the speculative draft trigger logic, not here
        }

        // Rule 12: Surface contradictions immediately
        12 => {
            let unresolved = state.contradictions.iter().filter(|c| !c.resolved).count();
            if unresolved > 0 && !is_contradiction_question(question) {
                return Some(ConstitutionViolation {
                    rule_id: 12,
                    category: RuleCategory::Process,
                    explanation: format!(
                        "{} unresolved contradiction(s) exist — should address them before asking new questions",
                        unresolved
                    ),
                });
            }
        }

        _ => {}
    }

    None
}

/// Check if a question is about resolving a contradiction.
fn is_contradiction_question(question: &str) -> bool {
    let lower = question.to_lowercase();
    lower.contains("conflict")
        || lower.contains("contradict")
        || lower.contains("incompatible")
        || lower.contains("which is more important")
        || lower.contains("you mentioned") && lower.contains("but also")
}

/// Check coverage rules at convergence time.
///
/// Returns dimensions that the constitution requires but haven't been addressed.
pub fn check_coverage(
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
) -> Vec<Dimension> {
    let mut uncovered = Vec::new();

    // Rule 7: security, error handling, and success criteria are mandatory
    for rule in constitution.all_rules() {
        if rule.id == 7 {
            let mandatory = [
                Dimension::Security,
                Dimension::ErrorHandling,
                Dimension::SuccessCriteria,
            ];
            for dim in &mandatory {
                let is_covered = state.filled.contains_key(dim)
                    || state.uncertain.contains_key(dim)
                    || state.out_of_scope.contains(dim);
                if !is_covered {
                    uncovered.push(dim.clone());
                }
            }
        }

        // Rule 8: stakeholders required for multi-user systems
        if rule.id == 8 {
            let is_multi_user = state
                .classification
                .as_ref()
                .map(|c| {
                    matches!(
                        c.project_type,
                        ProjectType::WebApp | ProjectType::ApiBackend | ProjectType::Hybrid
                    )
                })
                .unwrap_or(false);

            if is_multi_user {
                let stakeholders_covered = state.filled.contains_key(&Dimension::Stakeholders)
                    || state.uncertain.contains_key(&Dimension::Stakeholders)
                    || state.out_of_scope.contains(&Dimension::Stakeholders);
                if !stakeholders_covered {
                    uncovered.push(Dimension::Stakeholders);
                }
            }
        }

        // Rule 9: out-of-scope must be addressed
        if rule.id == 9 {
            let oos_covered =
                state.filled.contains_key(&Dimension::OutOfScope) || state.out_of_scope.len() > 0;
            if !oos_covered {
                uncovered.push(Dimension::OutOfScope);
            }
        }
    }

    uncovered.dedup();
    uncovered
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_state() -> RequirementsBeliefState {
        RequirementsBeliefState {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: vec![Dimension::Goal, Dimension::CoreFeatures],
            out_of_scope: vec![],
            contradictions: vec![],
            required_dimensions: vec![Dimension::Goal, Dimension::CoreFeatures],
            turn_count: 0,
            classification: None,
        }
    }

    #[test]
    fn default_constitution_loads() {
        let c = load_constitution();
        assert_eq!(c.rules.len(), 12);
    }

    #[test]
    fn rule2_multiple_questions() {
        let state = make_state();
        let c = load_constitution();

        let violations = evaluate_question(
            "What's the goal? And who are the users?",
            &Dimension::Goal,
            &state,
            &c,
        );

        assert!(violations.iter().any(|v| v.rule_id == 2));
    }

    #[test]
    fn rule2_single_question_passes() {
        let state = make_state();
        let c = load_constitution();

        let violations = evaluate_question(
            "What's the primary goal of this project?",
            &Dimension::Goal,
            &state,
            &c,
        );

        assert!(!violations.iter().any(|v| v.rule_id == 2));
    }

    #[test]
    fn rule4_implementation_before_scope() {
        let state = make_state();
        let c = load_constitution();

        let violations = evaluate_question(
            "What tech stack do you prefer?",
            &Dimension::TechStack,
            &state,
            &c,
        );

        assert!(violations.iter().any(|v| v.rule_id == 4));
    }

    #[test]
    fn rule4_implementation_after_scope() {
        let mut state = make_state();
        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "Task tracker".into(),
                source_turn: 1,
                source_quote: None,
            },
        );

        let c = load_constitution();

        let violations = evaluate_question(
            "What tech stack do you prefer?",
            &Dimension::TechStack,
            &state,
            &c,
        );

        assert!(!violations.iter().any(|v| v.rule_id == 4));
    }

    #[test]
    fn coverage_check_mandatory_dims() {
        let state = make_state();
        let c = load_constitution();

        let uncovered = check_coverage(&state, &c);
        assert!(uncovered.contains(&Dimension::Security));
        assert!(uncovered.contains(&Dimension::ErrorHandling));
        assert!(uncovered.contains(&Dimension::SuccessCriteria));
    }
}
