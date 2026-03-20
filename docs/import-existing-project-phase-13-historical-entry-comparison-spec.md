# Import Existing Project Phase 13 Historical Entry Comparison Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 12 Historical Applied Restore For Review Spec](/home/thetu/planner/docs/import-existing-project-phase-12-historical-applied-restore-for-review-spec.md)

## Objective

Advance `Import Existing Project` from restore-first historical workflows into
bounded history comparison.

This slice should let a user compare one selected historical import entry
against the current project import state before choosing whether to restore it
directly or reopen it for review. It should stay on the existing
`ProjectSessionsPage` history surface and reuse the current lightweight diff
model instead of inventing a graph viewer.

It does **not** yet introduce compare-any-two arbitrary entries, multi-entry
merge planning, or graph-level visual reconciliation.

## User Outcome

After this slice:

- a project can select a historical import entry and see a focused diff against
  the current import state before taking restore actions
- comparison works for the current latest reviewable or applied state against a
  selected historical entry
- the history surface can explain what would be added or removed if the user
  restores that historical entry
- restore actions remain explicit and unchanged after the comparison is shown

The user still does **not** get visual graph diffing, compare-any-two history
entries, or per-edge history merge tooling.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 13 delivery slice.

Execution landed in:

- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/api/__tests__/client.test.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- `GET /projects/{projectRef}/import-history/{jobId}/compare` now compares one
  selected historical import entry against the current import state
- the comparison reuses the lightweight node-level summary shape from Phase 7
  instead of introducing a new graph diff model
- `ProjectSessionsPage` now exposes `Compare To Current` on eligible
  historical import entries and renders a compact comparison panel
- restore actions clear the selected comparison so the panel does not linger on
  stale current-state assumptions

Verification completed:

- `cargo test -p planner-server compare_project_import_history_entry -- --nocapture`
- `cargo test -p planner-server get_project_import_history_returns_descending_entries_and_diff_summary -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- comparison is always **selected historical entry vs current project import
  state**, not arbitrary history-entry vs history-entry
- the comparison surface is informational only; it does not mutate review
  selection or blueprint state
- only entries with a durable draft payload participate in comparison
- the comparison stays on `ProjectSessionsPage`; no detached history dashboard
- the diff model remains node-level and summary-oriented, following the Phase 7
  lightweight comparison style
- this slice does not change the semantics of `Restore This Import`,
  `Restore For Review`, or `Restore Draft For Review`

## Scope

### In scope

- add a project-level compare action on eligible historical import entries
- compute a lightweight node-level diff between the selected historical draft
  and the current latest import draft or latest applied import state
- surface summary counts plus added/removed node names and types for the
  selected comparison
- refresh or clear the comparison when a restore action changes the current
  import state
- add focused tests for comparison eligibility, payload shape, and UI behavior

### Out of scope

- compare-any-two arbitrary historical entries
- graph visualization or edge-level comparison
- editing review selection directly from the comparison panel
- automatic restore recommendations
- compare across different projects
- merge simulation beyond added/removed node summaries

## Current-State Evidence

- Phase 7 already provides a lightweight diff between the current latest draft
  and the last applied import
- Phase 9 through Phase 12 now give multiple restore-first historical flows,
  but the user still lacks a targeted comparison against a chosen historical
  entry before taking those actions
- the history surface already has the right place for a compact comparison
  affordance next to restore actions

## Requirements

### Comparison contract

For a selected historical import entry with a durable draft payload:

- the server must expose a comparison payload against the current project
  import state
- the comparison must describe added/removed nodes and node-type counts using
  the same lightweight summary shape already familiar from Phase 7
- if the current project import state or the selected historical entry lacks a
  durable draft payload, comparison must fail explicitly rather than guessing

### Workflow contract

The project history workflow should remain explicit:

- comparing a historical entry must not change the current latest review or
  applied state
- restore actions stay separate from comparison
- if the current latest import state changes, stale comparison UI should be
  cleared or refreshed to stay truthful

### UI contract

`ProjectSessionsPage` should gain a compact comparison panel:

- eligible historical entries show a `Compare To Current` action
- the resulting panel names the selected historical entry and the current state
- the panel shows lightweight summary counts and added/removed node lists
- the panel should help the user decide between `Restore This Import` and
  `Restore For Review` without replacing those actions

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server tests around import history comparison

Implementation should stay bounded to selected-entry comparison against the
current import state. If the work starts requiring arbitrary history-to-history
compare or visual graph tooling, stop and split that into a later spec.

## Acceptance Criteria

- an eligible historical import entry can be compared to the current project
  import state
- comparison is informational only and does not mutate review or blueprint
  state
- the comparison panel shows lightweight added/removed node summaries and type
  counts
- comparison clears or refreshes when restore actions change the current state
- restore actions remain explicit and unchanged
- no detached history dashboard or graph visualizer is introduced in this slice

## Verification Plan

### Server

- tests proving comparison rejects invalid job ids and entries without a durable
  draft payload
- tests proving comparison returns the expected added/removed node summary for
  a selected historical entry vs current state
- tests proving comparison does not mutate blueprint or import job state

### Web

- `ProjectSessionsPage` tests for rendering `Compare To Current` on eligible
  history entries
- tests proving the comparison panel renders the selected-entry summary
- tests proving restore actions clear or refresh the comparison panel

## Rollback And Fallback

- if a dedicated compare endpoint proves broader than necessary, fall back to a
  server-built comparison embedded in the existing history payload for one
  selected entry at a time rather than widening into arbitrary history
  navigation
- if the UI becomes crowded on one history row, move comparison behind a single
  lightweight details toggle rather than creating a new page

## Open Questions

None. The slice is implemented and verified.
