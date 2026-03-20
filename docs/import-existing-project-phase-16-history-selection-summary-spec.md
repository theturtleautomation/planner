# Import Existing Project Phase 16 History Selection Summary Spec

**Status:** Ready for implementation  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md)

## Objective

Advance `Import Existing Project` from selection-aware comparison into
selection-aware history summaries.

This slice should expose the effective included/excluded footprint for import
history entries directly in the history list and history summary payloads, so a
user can see at a glance when a draft or restored review job has saved
merge-control exclusions without opening a comparison panel first.

It does **not** yet introduce graph-level summaries, automatic restore
recommendations, or bulk history management.

## User Outcome

After this slice:

- history rows can show whether a job uses the full raw draft or a filtered
  selected subset
- project history can show included and excluded node counts for entries with
  saved review selection state
- users can distinguish “raw discovered nodes” from “effective apply
  footprint” directly in the history surface
- compare and restore actions remain unchanged

The user still does **not** get graph diff visualizations, timeline playback,
or compare-driven editing from the history list.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- history summary remains project-scoped and stays on `ProjectSessionsPage`
- the server should expose effective included/excluded counts per eligible
  history entry rather than inventing a second aggregate history endpoint
- jobs without saved review selection state continue to show only raw draft
  counts
- this slice is informational only and must not mutate import jobs, review
  selection, or blueprint state
- existing compare, restore, and apply actions remain unchanged

## Scope

### In scope

- add selection-summary metadata to project import history entries
- expose effective included/excluded counts for jobs with saved review
  selection state
- render lightweight UI copy on history rows when saved exclusions exist
- keep the existing compare and restore affordances intact
- add focused tests for payload shape and row rendering

### Out of scope

- editing history-row merge controls in place
- graph visualization or edge-level summaries
- bulk restore or bulk compare actions
- timeline charts or historical analytics

## Current-State Evidence

- Phase 15 made compare results truthful to saved merge-control exclusions
- history rows still primarily expose raw draft node counts
- users still need to open a compare panel to notice that a reviewable history
  entry has saved exclusions affecting its effective apply footprint

## Requirements

### History payload contract

For history entries with a durable draft payload:

- the server must expose raw discovered node count exactly as today
- if saved review selection exists, the server must also expose effective
  included and excluded counts
- history entries without saved selection state must continue to work without
  synthetic values

### UI contract

`ProjectSessionsPage` should remain compact:

- history rows should show effective selected-node counts when available
- rows with saved exclusions should make that explicit in plain language
- compare, restore, and restore-for-review actions remain separate from the
  selection summary display

### Workflow contract

The feature remains informational only:

- rendering history selection summaries must not change saved review selection
- compare panels and restore actions continue to behave exactly as they did in
  Phase 15

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)

Implementation should stay bounded to exposing effective selection summaries on
history rows. If the work starts requiring historical graph summaries or bulk
edit tooling, stop and split that into a later spec.

## Acceptance Criteria

- project import history exposes effective included/excluded counts when saved
  review selection exists
- history rows explain when exclusions affect the effective apply footprint
- raw draft node counts remain available
- compare and restore behavior remain unchanged

## Verification Plan

### Server

- tests proving history payloads include effective included/excluded counts for
  jobs with saved review selection
- tests proving jobs without saved selection state continue to return cleanly

### Web

- `ProjectSessionsPage` tests proving history rows render selection-summary
  copy when exclusions exist
- tests proving rows without selection state remain compact and unchanged

## Rollback And Fallback

- if row density becomes too high, move the selection summary into a compact
  secondary line rather than broadening into expandable history cards
- if older jobs lack saved review selection state, fall back cleanly to raw
  draft counts without inventing synthetic summaries

## Open Questions

None. The slice is ready for bounded implementation.
