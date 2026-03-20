# Project Plan

**Status:** Active  
**Date:** 2026-03-19

## Purpose

This is a lightweight project planning index for the Planner repo.

It is not meant to replace feature-specific planning docs. It exists to give a
simple top-level view of:

- the main planning documents already in this repo
- the current active planning thread
- the next expected move before implementation

## Current Planning Spine

These are the main planning documents currently shaping the repo:

- [Project-First UI Research Sessions](/home/thetu/planner/docs/project-first-ui-research-sessions.md)
- [Phase 00 Project Ownership Implementation](/home/thetu/planner/docs/phase-00-project-ownership-implementation.md)
- [Phase 01 Root Landing And Navigation Implementation](/home/thetu/planner/docs/phase-01-root-landing-implementation.md)
- [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)
- [Knowledge Library Project Scope Plan](/home/thetu/planner/docs/knowledge-library-project-scope-plan.md)
- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)
- [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)
- [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md)
- [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)
- [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)
- [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md)
- [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md)
- [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md)
- [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md)
- [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md)
- [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md)
- [Import Existing Project Phase 11 Selective Apply Merge Controls Spec](/home/thetu/planner/docs/import-existing-project-phase-11-selective-apply-merge-controls-spec.md)
- [Import Existing Project Phase 12 Historical Applied Restore For Review Spec](/home/thetu/planner/docs/import-existing-project-phase-12-historical-applied-restore-for-review-spec.md)
- [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md)
- [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md)
- [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md)
- [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md)
- [Planning Status Audit Remediation Spec](/home/thetu/planner/docs/planning-status-audit-remediation-spec.md)

## Current Active Thread

### Import Existing Project

Goal:

- allow a user to import an existing codebase
- analyze it
- review imported structure before it becomes canonical blueprint knowledge
- then enter the Socratic lobby against that imported project

Canonical planning doc:

- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)

Supporting research doc:

- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)

Current completed slice:

- queued import domain skeleton is implemented and verified in
  [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)
- public GitHub acquisition and managed checkout are implemented and verified in
  [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md)
- project-scoped import draft generation and seeded-session handoff are
  implemented and verified in
  [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)
- explicit review/apply promotion into canonical project blueprint knowledge is
  implemented and verified in
  [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)
- local repo provider parity is implemented and verified in
  [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md)
- re-import, duplicate-source protection, and import-owned lifecycle cleanup
  are implemented and verified in
  [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md)
- project-scoped import history and lightweight draft-vs-last-applied diffing
  are implemented and verified in
  [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md)
- apply-time canonical reconciliation for import-owned project-local blueprint
  state is implemented and verified in
  [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md)
- project-scoped restore to a historical applied import is implemented and
  verified in
  [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md)
- reopening an older historical `review_pending` draft into the current review
  slot is implemented and verified in
  [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md)
- selective apply merge controls on the current review draft are implemented
  and verified in
  [Import Existing Project Phase 11 Selective Apply Merge Controls Spec](/home/thetu/planner/docs/import-existing-project-phase-11-selective-apply-merge-controls-spec.md)
- historical applied import restore-for-review is implemented and verified in
  [Import Existing Project Phase 12 Historical Applied Restore For Review Spec](/home/thetu/planner/docs/import-existing-project-phase-12-historical-applied-restore-for-review-spec.md)
- selected-entry historical comparison is implemented and verified in
  [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md)
- arbitrary two-entry historical comparison is implemented and verified in
  [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md)
- selection-aware history comparison is implemented and verified in
  [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md)

Current agreed product constraints:

- product framing: `Import Existing Project`
- providers in v1:
  - public GitHub repo import
  - local repo import
- clone/storage policy:
  - managed clone under Planner data for GitHub
  - validated local path for local import
- import merge policy:
  - auto-analyze
  - do not auto-merge into canonical blueprint
  - require import review before canonical blueprint commit

Current next bounded slice:

- broader history truthfulness is now bounded to exposing effective selection
  summaries directly on history rows
  in
  [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md)
- later history work should stay phased behind follow-on specs after this slice
  is implemented and verified

## Immediate Bounded Closeout Slice

The remaining planning-status drift surfaced by the 2026-03-19 audit is now
closed in:

- [Planning Status Audit Remediation Spec](/home/thetu/planner/docs/planning-status-audit-remediation-spec.md)

This was a cross-cutting documentation and verification slice. It did not
replace the active product thread.

## Working Rule

Keep planning and implementation aligned to artifact state:

1. Define or update the feature plan.
2. Turn the feature into a concrete spec/backlog item.
3. Only implement once the relevant spec is ready.
4. After implementation, review against the spec and sync docs.

## Next Expected Move

For the import feature, the next move is the next bounded spec, not more
unplanned implementation.

The next move is:

- use `delivery-cycle` to implement
  [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md)

After that, keep later import work phased behind follow-on specs for:

- broader historical reconciliation and history comparison behavior beyond
  current restore flows, selected-entry comparison, arbitrary two-entry
  comparison, selection-aware comparison, and history selection summaries

Use
[Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)
as the source research document for later import phases, not as the direct
execution artifact.
