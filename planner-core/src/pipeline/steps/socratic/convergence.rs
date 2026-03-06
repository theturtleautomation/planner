//! # Convergence Decider — Multi-Criteria Stopping Logic
//!
//! Determines when to stop asking and start building.
//! Uses 4 overlapping criteria (any triggers "done"):
//!
//! 1. Completeness gate — all required dimensions resolved
//! 2. Confidence threshold — no uncertain dimension below threshold
//! 3. Diminishing returns — last N questions produced no new info
//! 4. User signal — user explicitly says "just build it"

use planner_schemas::*;

use super::constitution::check_coverage;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check all convergence criteria and decide whether to stop.
///
/// `stale_turns` is the count of consecutive turns that produced no new
/// filled entries and no confidence improvements above 0.1.
pub fn check_convergence(
    state: &RequirementsBeliefState,
    constitution: &InterviewerConstitution,
    user_wants_to_stop: bool,
    stale_turns: u32,
) -> ConvergenceResult {
    let convergence_pct = state.convergence_pct();

    // Criterion 4: User signal — always respected
    if user_wants_to_stop {
        return ConvergenceResult {
            is_done: true,
            reason: StoppingReason::UserSignal,
            convergence_pct,
        };
    }

    // Criterion 1: Completeness gate
    // All required dimensions must be in filled or out_of_scope
    let all_required_resolved = state
        .required_dimensions
        .iter()
        .all(|d| state.filled.contains_key(d) || state.out_of_scope.contains(d));

    if all_required_resolved {
        // Also check constitution coverage rules
        let uncovered = check_coverage(state, constitution);
        if uncovered.is_empty() {
            return ConvergenceResult {
                is_done: true,
                reason: StoppingReason::CompletenessGate,
                convergence_pct,
            };
        }
        // If constitution says we need more, continue
    }

    // Criterion 2: Confidence threshold
    // All uncertain dimensions must be above the tier threshold
    let threshold = state
        .classification
        .as_ref()
        .map(|c| c.complexity.confidence_threshold())
        .unwrap_or(0.6);

    let all_above_threshold = state
        .uncertain
        .values()
        .all(|(_val, confidence)| *confidence >= threshold);

    if all_required_resolved && all_above_threshold && state.missing.is_empty() {
        return ConvergenceResult {
            is_done: true,
            reason: StoppingReason::ConfidenceThreshold,
            convergence_pct,
        };
    }

    // Criterion 3: Diminishing returns
    // If last 3+ turns produced nothing new, user has saturated
    if stale_turns >= 3 && state.turn_count >= 5 {
        return ConvergenceResult {
            is_done: true,
            reason: StoppingReason::DiminishingReturns { stale_turns },
            convergence_pct,
        };
    }

    // Not done — identify next priorities
    let mut next_priorities: Vec<Dimension> = state.missing.iter().take(3).cloned().collect();

    // Also include low-confidence uncertain dims
    for (dim, (_val, confidence)) in &state.uncertain {
        if *confidence < threshold && next_priorities.len() < 5 {
            next_priorities.push(dim.clone());
        }
    }

    ConvergenceResult {
        is_done: false,
        reason: StoppingReason::Continue { next_priorities },
        convergence_pct,
    }
}

/// Count stale turns — consecutive turns with no new filled entries
/// and no confidence improvements above 0.1.
///
/// This is tracked by comparing belief state snapshots before/after
/// each verifier pass.
pub fn is_stale_turn(
    before_filled: usize,
    after_filled: usize,
    before_uncertain_confs: &[f32],
    after_uncertain_confs: &[f32],
) -> bool {
    // No new filled entries
    if after_filled > before_filled {
        return false;
    }

    // Check if any uncertain confidence improved by >0.1
    for (before, after) in before_uncertain_confs
        .iter()
        .zip(after_uncertain_confs.iter())
    {
        if after - before > 0.1 {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_constitution() -> InterviewerConstitution {
        InterviewerConstitution::default_constitution()
    }

    fn make_test_state() -> RequirementsBeliefState {
        RequirementsBeliefState {
            filled: HashMap::new(),
            uncertain: HashMap::new(),
            missing: vec![Dimension::Goal, Dimension::CoreFeatures],
            out_of_scope: vec![],
            contradictions: vec![],
            required_dimensions: vec![Dimension::Goal, Dimension::CoreFeatures],
            turn_count: 0,
            classification: Some(DomainClassification {
                project_type: ProjectType::CliTool,
                complexity: ComplexityTier::Light,
                detected_signals: vec![],
                required_dimensions: vec![Dimension::Goal, Dimension::CoreFeatures],
            }),
        }
    }

    #[test]
    fn user_signal_always_stops() {
        let state = make_test_state();
        let c = make_constitution();

        let result = check_convergence(&state, &c, true, 0);
        assert!(result.is_done);
        assert!(matches!(result.reason, StoppingReason::UserSignal));
    }

    #[test]
    fn completeness_gate_when_all_filled() {
        let mut state = make_test_state();
        // Fill all required dimensions
        state.fill(
            Dimension::Goal,
            SlotValue {
                value: "test".into(),
                source_turn: 1,
                source_quote: None,
            },
        );
        state.fill(
            Dimension::CoreFeatures,
            SlotValue {
                value: "test".into(),
                source_turn: 2,
                source_quote: None,
            },
        );
        // Constitution requires Security, ErrorHandling, SuccessCriteria, OutOfScope
        state.fill(
            Dimension::Security,
            SlotValue {
                value: "test".into(),
                source_turn: 3,
                source_quote: None,
            },
        );
        state.fill(
            Dimension::ErrorHandling,
            SlotValue {
                value: "test".into(),
                source_turn: 4,
                source_quote: None,
            },
        );
        state.fill(
            Dimension::SuccessCriteria,
            SlotValue {
                value: "test".into(),
                source_turn: 5,
                source_quote: None,
            },
        );
        state.mark_out_of_scope(Dimension::OutOfScope);

        let c = make_constitution();
        let result = check_convergence(&state, &c, false, 0);
        assert!(result.is_done);
        assert!(matches!(result.reason, StoppingReason::CompletenessGate));
    }

    #[test]
    fn diminishing_returns_stops() {
        let mut state = make_test_state();
        state.turn_count = 6;
        let c = make_constitution();

        let result = check_convergence(&state, &c, false, 3);
        assert!(result.is_done);
        assert!(matches!(
            result.reason,
            StoppingReason::DiminishingReturns { stale_turns: 3 }
        ));
    }

    #[test]
    fn continue_when_nothing_triggers() {
        let state = make_test_state();
        let c = make_constitution();

        let result = check_convergence(&state, &c, false, 0);
        assert!(!result.is_done);
        assert!(matches!(result.reason, StoppingReason::Continue { .. }));
    }

    #[test]
    fn stale_turn_detection() {
        // No new fills, no confidence improvement
        assert!(is_stale_turn(3, 3, &[0.5, 0.6], &[0.5, 0.65]));

        // New fill
        assert!(!is_stale_turn(3, 4, &[0.5], &[0.5]));

        // Confidence improvement > 0.1
        assert!(!is_stale_turn(3, 3, &[0.5], &[0.7]));
    }
}
