# Planner SolidStart Phase 16 Project Import Comparison And Selection Summary Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md), [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md), [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md), [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md), [Planner SolidStart Phase 15 Project Import History And Restore Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-15-project-import-history-and-restore-route-spec.md)

> Planning note (2026-03-24): once import history is project-local in Solid,
> the follow-on move should make that history truly usable. Users need to
> compare one entry to the current state, compare two arbitrary historical
> entries, and understand when saved merge-control exclusions changed the
> effective comparison footprint.
>
> Implementation sync (2026-03-24): the Solid import workspace now includes
> current-vs-history comparison, baseline-vs-selected arbitrary history
> comparison, truthful selection-aware notes, and effective included/excluded
> counts directly on history rows. This closes the remaining React-era import
> history parity for bounded historical comparison without inventing a new
> graph-diff model.

## 1. Executive Judgment

The final import-history widening slice should make the project-local history
surface **comparison-ready and selection-aware**.

This slice should answer:

- how a selected historical import differs from the current import state
- how any two historical entries differ when chosen as baseline and compared
- whether saved exclusions changed the effective comparison or apply footprint

## 2. User Outcome

After Phase 16:

- current-state and arbitrary history comparison are available in SolidStart
- saved merge-control exclusions are surfaced truthfully in history summaries
- comparison notes explain when selection-aware filtering changed the result
- import restore, review recovery, and comparison all remain attached to the
  project-local import desk

## 3. Locked Decisions

- this work stays attached to `/projects/:projectSlug/import`
- comparisons remain informational only and do not mutate history
- saved selection summaries are read-only metadata, not editable state
- no new graph viewer or generalized time-travel surface is introduced

## 4. Acceptance Criteria

This slice is complete only when:

1. a user can compare a historical import entry to the current import state in
   the Solid import desk
2. a user can select a baseline entry and compare another historical entry to
   it
3. saved selection-aware filtering is reflected in comparison notes and
   effective included/excluded counts
4. browser verification proves the import desk keeps project context primary
   while exposing comparison and restore controls together

## 5. Readiness Judgment

This spec is **implemented**.
