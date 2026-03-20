# Import Existing Project Phase 14 Arbitrary History Comparison Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md)

## Objective

Advance `Import Existing Project` from selected-entry-vs-current comparison
into bounded arbitrary history comparison.

This slice should let a user compare any two eligible import history entries
for the same project, instead of only comparing one historical entry against
the current import state. It should stay on `ProjectSessionsPage`, reuse the
existing lightweight node-level diff model, and remain informational only.

It does **not** yet introduce graph visualization, merge simulation, or
cross-project comparison.

## User Outcome

After this slice:

- a project can select one historical import entry as the comparison baseline
- the user can compare another history entry against that selected baseline
- the history surface can explain what changed between two concrete import
  attempts before the user decides whether either entry should be restored
- comparison remains lightweight, readable, and explicitly separate from
  restore actions

The user still does **not** get visual graph diffing, multi-entry merge plans,
or compare/restore operations across different projects.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 14 delivery slice.

Execution landed in:

- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/api/__tests__/client.test.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- `GET /projects/{projectRef}/import-history/{baseJobId}/compare/{jobId}` now
  compares any two eligible same-project history entries without mutating
  import or blueprint state
- `ProjectSessionsPage` now lets the user mark one history entry as a baseline
  and compare a second entry against it while keeping restore actions separate
- the arbitrary two-entry comparison reuses the existing lightweight node-level
  added/removed summary shape instead of inventing a second diff model
- changing the selected baseline clears the prior pair comparison panel so the
  history surface stays truthful

Verification completed:

- `cargo test -p planner-server compare_project_import_history_entry -- --nocapture`
- `cargo test -p planner-server compare_project_import_history_entries -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- comparison is limited to **two import history entries for the same project**
- both comparison targets must have durable draft payloads
- comparison remains informational only and must not mutate import jobs,
  review state, or blueprint state
- the existing selected-entry-vs-current comparison may remain as a shortcut,
  but this slice adds explicit history-entry-vs-history-entry comparison
- the diff model remains node-level and summary-oriented, using the existing
  lightweight added/removed node summary shape
- the comparison UI stays on `ProjectSessionsPage`; no detached history page
- this slice does not alter the semantics of `Restore This Import`,
  `Restore For Review`, or `Restore Draft For Review`

## Scope

### In scope

- add a project-level way to choose one history entry as a comparison baseline
- compare a second eligible history entry against that chosen baseline
- expose a server payload for arbitrary same-project history-entry comparison
- render a compact comparison panel naming both compared entries
- keep restore actions explicit and separate from the comparison controls
- add focused tests for eligibility, payload shape, and panel behavior

### Out of scope

- compare more than two entries at once
- graph visualization or edge-level diffing
- compare across different projects
- merge simulation or restore recommendations
- editing review selection from the comparison panel
- timeline visualizations or historical playback

## Current-State Evidence

- Phase 13 now supports one selected historical entry compared against the
  current import state
- users still cannot compare two non-current historical entries directly, which
  limits usefulness once a project has multiple prior imports worth reviewing
- the lightweight diff machinery already exists and can be reused for another
  compare mode without inventing a second model

## Requirements

### Comparison contract

For two selected history entries on the same project:

- the server must return a lightweight diff payload describing what is added or
  removed between the two selected entries
- the response must identify both compared entries explicitly
- comparison must fail explicitly if either selected entry lacks a durable
  draft payload
- comparison must not mutate import state or blueprint state

### Workflow contract

The project history workflow should remain explicit:

- selecting a comparison baseline must not trigger restore behavior
- restore actions remain available and separate from comparison
- switching the selected baseline or compared entry should replace the current
  comparison panel instead of accumulating multiple comparison views

### UI contract

`ProjectSessionsPage` should support an explicit two-entry compare flow:

- one history entry can be marked as the comparison baseline
- other eligible entries can then show `Compare To Selected`
- the comparison panel must name both selected entries and render the existing
  lightweight added/removed summary
- the surface should remain compact and readable even when history contains
  both `applied` and `review_pending` entries

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server tests around import history comparison

Implementation should stay bounded to same-project, two-entry comparison. If
the work starts requiring graph visualization or multi-entry selection, stop
and split that into a later spec.

## Acceptance Criteria

- a user can choose one history entry as a comparison baseline
- a second eligible history entry can be compared against that baseline
- the comparison panel names both compared entries and renders the lightweight
  added/removed node summary
- comparison is informational only and does not mutate import or blueprint
  state
- restore actions remain explicit and unchanged
- no detached history dashboard or graph visualizer is introduced in this slice

## Verification Plan

### Server

- tests proving arbitrary same-project history-entry comparison returns the
  expected diff summary
- tests proving comparison rejects entries without a durable draft payload
- tests proving comparison does not mutate import jobs or blueprint state

### Web

- `ProjectSessionsPage` tests for selecting a baseline history entry
- tests for comparing a second entry against that selected baseline
- tests proving the comparison panel updates cleanly when the baseline changes

## Rollback And Fallback

- if arbitrary two-entry comparison adds too much control density to the
  history rows, fall back to a simple “set baseline / compare against baseline”
  sequence rather than broadening into a separate page
- if the server route shape becomes too broad, keep the API limited to one
  explicit `base_job_id` and one explicit `compared_job_id` rather than adding
  generic query grammar

## Open Questions

None. The slice is ready for bounded implementation.
