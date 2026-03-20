# Import Existing Project Phase 15 Selection-Aware History Comparison Spec

**Status:** Ready for implementation  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md)

## Objective

Advance `Import Existing Project` from raw draft history comparison into
selection-aware history comparison.

This slice should make history comparison truthful to Phase 11 merge controls
by respecting persisted review-selection state when a compared history entry
has excluded nodes. It should continue using the existing node-level diff
model and stay on `ProjectSessionsPage`.

It does **not** yet introduce edge-level diffing, graph visualization, or
automatic restore recommendations.

## User Outcome

After this slice:

- history comparison reflects the effective apply footprint for entries with
  saved include/exclude state instead of always diffing the full raw draft
- the comparison panel can explain when excluded nodes changed the result
- users can compare older review drafts or restored review jobs without
  forgetting that merge controls changed what would actually be applied
- current restore and apply actions remain explicit and unchanged

The user still does **not** get graph visual diffing, cross-project compare,
or compare-driven editing of review selection.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- comparison remains limited to existing same-project compare flows:
  - selected historical entry vs current state
  - arbitrary two-entry history comparison
- if an import job has persisted review selection state, comparison must use
  the effective included-node subset for that job
- jobs without persisted review selection continue to compare using the full
  draft payload
- selection-aware comparison remains informational only and does not mutate
  review selection, import jobs, or blueprint state
- the comparison UI stays on `ProjectSessionsPage`
- this slice does not add new merge controls or change apply semantics

## Scope

### In scope

- apply persisted review-selection filtering when building comparison drafts
- make current-state comparison truthful when the latest review draft has
  excluded nodes
- make arbitrary two-entry comparison truthful when either compared entry has
  persisted review-selection state
- render lightweight UI copy indicating when comparison is using selected nodes
- add focused tests for filtered diff results and unchanged non-mutating
  behavior

### Out of scope

- editing review selection from the comparison panel
- edge-level diffing or graph visualization
- compare more than two entries at once
- restore recommendations or merge simulation
- rewriting old stored drafts to materialize selection state

## Current-State Evidence

- Phase 11 introduced persisted per-job include/exclude state for
  `review_pending` import drafts
- Phase 13 and Phase 14 compare raw draft payloads only
- that means current comparison can drift from the actual apply footprint when
  the user has excluded nodes from the latest review job or a restored review
  draft

## Requirements

### Comparison contract

For any compare flow involving a job with durable draft payload:

- comparison must use the persisted selected-node subset when review selection
  exists for that job
- comparison must continue returning the same lightweight added/removed node
  summary shape
- comparison must identify which jobs are being compared exactly as today
- comparison must remain non-mutating

### Workflow contract

The comparison workflow should remain explicit:

- selection-aware comparison must not change any include/exclude state
- restore and apply actions remain separate from compare actions
- switching review selection through the existing merge controls should clear
  stale compare panels so the UI stays truthful

### UI contract

`ProjectSessionsPage` should stay compact:

- comparison panels should indicate when a compared entry is using selected
  nodes rather than its full raw draft
- no second comparison page or graph viewer should be introduced
- existing compare actions remain in place

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server tests around import history comparison

Implementation should stay bounded to truthful selection-aware comparison. If
the work starts requiring new diff models or graph rendering, stop and split
that into a later spec.

## Acceptance Criteria

- history comparison reflects persisted review selection when it exists
- current-state comparison reflects the effective selected subset of the latest
  review draft
- arbitrary two-entry comparison reflects the effective selected subset for
  either compared entry
- comparison remains informational only and non-mutating
- the UI indicates when selection-aware filtering affected the comparison

## Verification Plan

### Server

- tests proving current-state comparison uses persisted selection filtering
- tests proving arbitrary two-entry comparison uses persisted selection
  filtering on either side
- tests proving comparison remains non-mutating

### Web

- `ProjectSessionsPage` tests proving comparison copy updates when selected-node
  filtering is active
- tests proving selection changes clear stale compare panels

## Rollback And Fallback

- if historical selection-aware comparison proves too confusing to explain in
  the current panel, limit the slice to current-state comparison first instead
  of widening the UI
- if selection metadata is missing for older jobs, fall back cleanly to raw
  draft comparison without inventing synthetic filtering

## Open Questions

None. The slice is ready for bounded implementation.
