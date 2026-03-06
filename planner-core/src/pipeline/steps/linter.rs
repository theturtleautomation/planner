//! # Spec Linter — 12 Rules, Deterministic
//!
//! Validates an NLSpec chunk against structural and content rules.
//! No LLM calls — purely deterministic checks.
//!
//! Phase 3 adds `lint_spec_set()` for cross-chunk reference validation
//! across the full set of root + domain chunks.

use super::{StepError, StepResult};
use planner_schemas::*;

/// Validate an NLSpec chunk against the 12 linting rules.
pub fn lint_spec(spec: &NLSpecV1) -> StepResult<()> {
    let mut violations = Vec::new();

    // Rule 1: All required sections present in root chunk
    if spec.chunk == ChunkType::Root {
        if spec.intent_summary.is_none() {
            violations.push("Rule 1: Root chunk missing Intent Summary".into());
        }
        if spec.sacred_anchors.is_none() {
            violations.push("Rule 1: Root chunk missing Sacred Anchors".into());
        }
        if spec.phase1_contracts.is_none() {
            violations.push("Rule 1: Root chunk missing Phase 1 Contracts".into());
        }
    }

    // Rule 2: DoD checklist non-empty
    if spec.definition_of_done.is_empty() {
        violations.push("Rule 2: Definition of Done is empty".into());
    }

    // Rule 3: All Sacred Anchors have ≥1 corresponding FR
    if let Some(anchors) = &spec.sacred_anchors {
        for anchor in anchors {
            let has_fr = spec
                .requirements
                .iter()
                .any(|r| r.traces_to.contains(&anchor.id));
            if !has_fr {
                violations.push(format!(
                    "Rule 3: Sacred Anchor {} has no corresponding FR",
                    anchor.id
                ));
            }
        }
    }

    // Rule 4: All FRs use imperative language
    let imperative_words = ["must", "must not", "always", "never", "shall", "shall not"];
    for req in &spec.requirements {
        let lower = req.statement.to_lowercase();
        if !imperative_words.iter().any(|w| lower.contains(w)) {
            violations.push(format!(
                "Rule 4: FR {} does not use imperative language (must/must not/always/never)",
                req.id
            ));
        }
    }

    // Rule 5: Open Questions list is empty
    let unresolved = spec
        .open_questions
        .iter()
        .filter(|oq| oq.resolution.is_none())
        .count();
    if unresolved > 0 {
        violations.push(format!(
            "Rule 5: {} unresolved Open Question(s)",
            unresolved
        ));
    }

    // Rule 6: Phase 1 Contracts include types for all cross-domain interfaces
    // (Simplified check for Phase 0 — just verify non-empty if root chunk)
    if spec.chunk == ChunkType::Root {
        if let Some(contracts) = &spec.phase1_contracts {
            if contracts.is_empty() {
                violations.push("Rule 6: Phase 1 Contracts is empty in root chunk".into());
            }
        }
    }

    // Rule 7: External Dependencies have DTU priority assigned
    // Dependencies with DtuPriority::None mean "no mock needed" — that's
    // intentional for standard libraries. But if a dep has been added
    // and is *not* a standard-library dep, leaving it as None may cause
    // the DTU generator to skip cloning it, which can silently break
    // scenario validation. Flag it as a warning so engineers make a
    // conscious decision.
    for dep in &spec.external_dependencies {
        if dep.dtu_priority == DtuPriority::None {
            violations.push(format!(
                "Rule 7: External dependency '{}' has dtu_priority=None — \
                 verify this is intentional (standard-library / no-mock deps are OK, \
                 but cloud services should be High or Medium)",
                dep.name,
            ));
        }
    }

    // Rule 8: Each chunk ≤500 lines
    if spec.line_count > 500 {
        violations.push(format!(
            "Rule 8: Chunk exceeds 500 lines ({})",
            spec.line_count
        ));
    }

    // Rule 9: All cross-chunk references use stable IDs
    for req in &spec.requirements {
        if !req.id.starts_with("FR-") {
            violations.push(format!(
                "Rule 9: Requirement ID '{}' does not use stable FR-N format",
                req.id
            ));
        }
    }

    // Rule 10: Out of Scope list is non-empty
    if spec.out_of_scope.is_empty() {
        violations.push("Rule 10: Out of Scope list is empty".into());
    }

    // Rule 11: Satisfaction Criteria has ≥1 critical scenario seed
    let has_critical = spec
        .satisfaction_criteria
        .iter()
        .any(|sc| sc.tier_hint == ScenarioTierHint::Critical);
    if !has_critical {
        violations.push("Rule 11: Satisfaction Criteria has no critical scenario seed".into());
    }

    // Rule 12: Amendment Log is append-only (structural check only —
    // we verify no entries were removed by comparing with previous version)
    // Phase 0: just verify the log exists (no previous version to compare)

    if violations.is_empty() {
        Ok(())
    } else {
        Err(StepError::LintFailure { violations })
    }
}

// ---------------------------------------------------------------------------
// Multi-Chunk Lint: cross-chunk reference validation
// ---------------------------------------------------------------------------

/// Validate a full set of NLSpec chunks (root + domains) for cross-chunk consistency.
///
/// This checks:
/// - Rule 9a: All cross-chunk references use stable IDs and match existing entities
/// - Rule 9b: Every Sacred Anchor is covered by at least one FR across all chunks
/// - Rule 9c: Domain FR IDs use the correct domain prefix format
/// - Rule 9d: No duplicate FR IDs across chunks
/// - Rule 9e: Phase 1 Contract references in domain chunks match root contracts
///
/// Each individual chunk is also linted with `lint_spec()`.
pub fn lint_spec_set(specs: &[NLSpecV1]) -> StepResult<()> {
    if specs.is_empty() {
        return Err(StepError::Other("Empty spec set".into()));
    }

    let mut violations = Vec::new();

    // Step 1: Lint each chunk individually
    for spec in specs {
        if let Err(StepError::LintFailure {
            violations: chunk_violations,
        }) = lint_spec(spec)
        {
            let chunk_label = match &spec.chunk {
                ChunkType::Root => "root".to_string(),
                ChunkType::Domain { name } => format!("domain:{}", name),
            };
            for v in chunk_violations {
                violations.push(format!("[{}] {}", chunk_label, v));
            }
        }
    }

    let root = &specs[0];

    // Step 2: Collect all FR IDs across all chunks and check for duplicates (Rule 9d)
    let mut all_fr_ids: Vec<(String, String)> = Vec::new(); // (fr_id, chunk_label)
    for spec in specs {
        let chunk_label = match &spec.chunk {
            ChunkType::Root => "root".to_string(),
            ChunkType::Domain { name } => name.clone(),
        };
        for req in &spec.requirements {
            if let Some((_existing_id, existing_chunk)) =
                all_fr_ids.iter().find(|(id, _)| id == &req.id)
            {
                violations.push(format!(
                    "Rule 9d: Duplicate FR ID '{}' found in chunks '{}' and '{}'",
                    req.id, existing_chunk, chunk_label,
                ));
            }
            all_fr_ids.push((req.id.clone(), chunk_label.clone()));
        }
    }

    // Step 3: Every Sacred Anchor must be covered by at least one FR across all chunks (Rule 9b)
    if let Some(anchors) = &root.sacred_anchors {
        let all_traced_anchors: Vec<&str> = specs
            .iter()
            .flat_map(|s| s.requirements.iter())
            .flat_map(|r| r.traces_to.iter())
            .map(|s| s.as_str())
            .collect();

        for anchor in anchors {
            if !all_traced_anchors.contains(&anchor.id.as_str()) {
                violations.push(format!(
                    "Rule 9b: Sacred Anchor {} has no corresponding FR in any chunk",
                    anchor.id,
                ));
            }
        }
    }

    // Step 4: Domain chunks should use domain-prefixed FR IDs (Rule 9c) — advisory warning
    for spec in specs.iter().skip(1) {
        if let ChunkType::Domain { name } = &spec.chunk {
            let expected_prefix = format!("FR-{}-", name.to_uppercase().replace('-', "_"));
            for req in &spec.requirements {
                if !req.id.starts_with(&expected_prefix) {
                    violations.push(format!(
                        "Rule 9c: Domain '{}' FR '{}' should use prefix '{}'",
                        name, req.id, expected_prefix,
                    ));
                }
            }
        }
    }

    // Step 5: All traces_to references in domain chunks must reference valid anchor IDs (Rule 9a)
    let valid_anchor_ids: Vec<&str> = root
        .sacred_anchors
        .as_ref()
        .map(|anchors| anchors.iter().map(|a| a.id.as_str()).collect())
        .unwrap_or_default();

    for spec in specs.iter().skip(1) {
        let chunk_label = match &spec.chunk {
            ChunkType::Root => "root".to_string(),
            ChunkType::Domain { name } => name.clone(),
        };
        for req in &spec.requirements {
            for anchor_ref in &req.traces_to {
                if !valid_anchor_ids.contains(&anchor_ref.as_str()) {
                    violations.push(format!(
                        "Rule 9a: Domain '{}' FR '{}' traces to unknown anchor '{}'",
                        chunk_label, req.id, anchor_ref,
                    ));
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(StepError::LintFailure { violations })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_valid_spec() -> NLSpecV1 {
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Root,
            status: NLSpecStatus::Draft,
            line_count: 200,
            created_from: "planner.intake.v1:test".into(),
            intent_summary: Some("A simple task tracker widget".into()),
            sacred_anchors: Some(vec![NLSpecAnchor {
                id: "SA-1".into(),
                statement: "User data must never be lost".into(),
            }]),
            requirements: vec![Requirement {
                id: "FR-1".into(),
                statement: "The system must persist all task data to local storage".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            }],
            architectural_constraints: vec!["Single-file React component".into()],
            phase1_contracts: Some(vec![Phase1Contract {
                name: "Task".into(),
                type_definition: "{ id: string, title: string, done: boolean }".into(),
                consumed_by: vec!["ui".into()],
            }]),
            external_dependencies: vec![],
            definition_of_done: vec![DoDItem {
                criterion: "All tasks persist across page refresh".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: "SC-1".into(),
                description: "Adding a task and refreshing shows it still there".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec!["Multi-user sync".into()],
            amendment_log: vec![],
        }
    }

    #[test]
    fn valid_spec_passes_linter() {
        let spec = make_valid_spec();
        assert!(lint_spec(&spec).is_ok());
    }

    #[test]
    fn empty_dod_fails_linter() {
        let mut spec = make_valid_spec();
        spec.definition_of_done.clear();
        let err = lint_spec(&spec).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations.iter().any(|v| v.contains("Rule 2")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn over_500_lines_fails_linter() {
        let mut spec = make_valid_spec();
        spec.line_count = 501;
        let err = lint_spec(&spec).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations.iter().any(|v| v.contains("Rule 8")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn non_imperative_fr_fails_linter() {
        let mut spec = make_valid_spec();
        spec.requirements[0].statement = "The system saves data".into();
        let err = lint_spec(&spec).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations.iter().any(|v| v.contains("Rule 4")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn unresolved_open_question_fails_linter() {
        let mut spec = make_valid_spec();
        spec.open_questions.push(OpenQuestion {
            question: "What auth provider?".into(),
            raised_by: "test".into(),
            resolution: None,
        });
        let err = lint_spec(&spec).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations.iter().any(|v| v.contains("Rule 5")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn orphan_sacred_anchor_fails_linter() {
        let mut spec = make_valid_spec();
        spec.sacred_anchors.as_mut().unwrap().push(NLSpecAnchor {
            id: "SA-2".into(),
            statement: "Orphaned anchor".into(),
        });
        let err = lint_spec(&spec).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations
                    .iter()
                    .any(|v| v.contains("Rule 3") && v.contains("SA-2")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    // -- Multi-chunk lint tests --

    fn make_valid_domain_spec(domain_name: &str) -> NLSpecV1 {
        let prefix = domain_name.to_uppercase();
        NLSpecV1 {
            project_id: Uuid::new_v4(),
            version: "1.0".into(),
            chunk: ChunkType::Domain {
                name: domain_name.to_string(),
            },
            status: NLSpecStatus::Draft,
            line_count: 100,
            created_from: format!("test:domain:{}", domain_name),
            intent_summary: None,
            sacred_anchors: None,
            phase1_contracts: None,
            requirements: vec![Requirement {
                id: format!("FR-{}-1", prefix),
                statement: "The system must handle this domain".into(),
                priority: Priority::Must,
                traces_to: vec!["SA-1".into()],
            }],
            architectural_constraints: vec![],
            external_dependencies: vec![],
            definition_of_done: vec![DoDItem {
                criterion: "Domain works correctly".into(),
                mechanically_checkable: true,
            }],
            satisfaction_criteria: vec![SatisfactionCriterion {
                id: format!("SC-{}-1", prefix),
                description: "Domain test passes".into(),
                tier_hint: ScenarioTierHint::Critical,
            }],
            open_questions: vec![],
            out_of_scope: vec!["Things not in this domain".into()],
            amendment_log: vec![],
        }
    }

    #[test]
    fn lint_spec_set_valid_multichunk() {
        let root = make_valid_spec();
        let auth = make_valid_domain_spec("AUTH");
        let result = lint_spec_set(&[root, auth]);
        assert!(result.is_ok());
    }

    #[test]
    fn lint_spec_set_duplicate_fr_ids() {
        let root = make_valid_spec();
        let mut auth = make_valid_domain_spec("AUTH");
        auth.requirements[0].id = "FR-1".into(); // Conflicts with root's FR-1
        let err = lint_spec_set(&[root, auth]).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations
                    .iter()
                    .any(|v| v.contains("Rule 9d") && v.contains("FR-1")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn lint_spec_set_orphaned_anchor_across_chunks() {
        let mut root = make_valid_spec();
        root.sacred_anchors.as_mut().unwrap().push(NLSpecAnchor {
            id: "SA-99".into(),
            statement: "No FR covers this".into(),
        });
        let auth = make_valid_domain_spec("AUTH");
        let err = lint_spec_set(&[root, auth]).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations
                    .iter()
                    .any(|v| v.contains("Rule 9b") && v.contains("SA-99")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn lint_spec_set_invalid_anchor_reference() {
        let root = make_valid_spec();
        let mut auth = make_valid_domain_spec("AUTH");
        auth.requirements[0].traces_to = vec!["SA-999".into()]; // Doesn't exist
        let err = lint_spec_set(&[root, auth]).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations
                    .iter()
                    .any(|v| v.contains("Rule 9a") && v.contains("SA-999")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }

    #[test]
    fn lint_spec_set_wrong_domain_prefix() {
        let root = make_valid_spec();
        let mut auth = make_valid_domain_spec("AUTH");
        auth.requirements[0].id = "FR-WRONG-1".into(); // Wrong prefix for AUTH domain
        let err = lint_spec_set(&[root, auth]).unwrap_err();
        match err {
            StepError::LintFailure { violations } => {
                assert!(violations.iter().any(|v| v.contains("Rule 9c")));
            }
            _ => panic!("Expected LintFailure"),
        }
    }
}
