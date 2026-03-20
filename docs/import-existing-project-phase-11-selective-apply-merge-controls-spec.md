# Import Existing Project Phase 11 Selective Apply Merge Controls Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md)

## Objective

Advance `Import Existing Project` from bounded historical restore into bounded
merge controls on the current review draft.

This slice should let a user exclude selected discovered import nodes from the
current `review_pending` draft before applying it, so import apply is no longer
all-or-nothing. It should stay on the existing project review surface and
should reuse the current Phase 8 reconciliation path by changing which draft
nodes are considered approved for apply.

It does **not** yet introduce arbitrary graph editing, edge-level merge
controls, historical draft comparison dashboards, or free-form reconciliation
across multiple history entries.

## User Outcome

After this slice:

- a project with a current `review_pending` import draft can selectively
  exclude discovered nodes before apply
- the review surface can show which draft nodes are still included vs excluded
- applying the draft promotes only the included nodes into canonical import-
  owned project blueprint state
- excluded draft nodes remain in the persisted historical draft for auditability
  but are not merged into canonical blueprint state
- later restore and history behavior remain append-only and truthful about the
  original historical draft vs the current review selection

The user still does **not** get node editing, manual edge editing, arbitrary
draft mutation, or multi-version merge tooling.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 11 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/api/__tests__/client.test.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- the latest `review_pending` import draft now carries durable job-scoped
  review selection state
- `POST /projects/{projectRef}/import-review-selection` now toggles per-node
  include/exclude state for the current review draft
- `ProjectSessionsPage` now shows a compact merge-controls list with included
  vs excluded node state and per-node toggle actions
- `Apply Import Draft` now promotes only the selected-in nodes while treating
  excluded nodes as intentionally absent for Phase 8 reconciliation
- restoring a historical review draft now resets the fresh latest review job
  back to all nodes included instead of inheriting stale old exclusions

Verification completed:

- `cargo test -p planner-server project_import_review -- --nocapture`
- `cargo test -p planner-server restore_project_import_review_draft -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- merge controls apply only to the **current latest `review_pending` draft**
- the control surface is **exclude from apply**, not arbitrary draft editing
- the persisted historical draft payload remains immutable; user choices are
  stored as review selection state for the current review job
- apply continues to use the existing explicit `Apply Import Draft` action
- reconciliation operates on the selected-in subset of the current review
  draft, not the full raw discovered draft
- excluded nodes are treated as intentionally absent for apply-time
  reconciliation
- this slice does not add edge-level toggles, rename controls, or per-field
  merge semantics
- restoring a historical review draft or historical applied import resets the
  current review selection to the restored draft's full discovered-node set

## Scope

### In scope

- add project-owned review selection state for the latest `review_pending`
  import draft
- default all discovered draft nodes to included when a fresh reviewable draft
  is created, re-imported, or restored for review
- expose lightweight include/exclude controls for discovered nodes on
  `ProjectSessionsPage`
- update apply so only included nodes are promoted into canonical blueprint
  state
- make Phase 8 reconciliation operate against the included subset, so excluded
  nodes are not kept active merely because they are present in the raw draft
- surface truthful counts and copy for selected vs excluded draft nodes on the
  review card
- add focused tests for selection persistence, apply behavior, and restore
  reset semantics

### Out of scope

- editing node names, node types, or node metadata
- edge-level include/exclude controls
- merging across two different historical drafts at once
- historical diff visualizations beyond the existing Phase 7 summary
- applying only part of a historical applied restore
- manual archive/unarchive outside the import review/apply flow
- free-form blueprint editing from the import review surface

## Current-State Evidence

- the current review surface on
  [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  shows only summary metadata plus an all-or-nothing `Apply Import Draft`
  action
- the persisted import draft already stores discovered project-scoped
  blueprint nodes in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- Phase 8 already reconciles canonical import-owned blueprint state during
  apply, but it currently assumes the entire latest draft is approved
- Phase 9 and Phase 10 already made restore/history append-only and auditable,
  so the next safe expansion is controlling what the current review job
  approves rather than broadening historical restore again

## Requirements

### Review selection contract

For the latest `review_pending` import draft:

- the server must maintain a deterministic set of included vs excluded
  discovered nodes for that review job
- a new reviewable draft must default to all nodes included
- the user must be able to toggle selection for an individual discovered node
  from the project review surface
- selection changes must persist across reloads while the draft remains the
  latest `review_pending` review target

Implementation may choose whether the selection model is include-list or
exclude-list, but it must remain deterministic and job-scoped.

### Apply contract

When the user applies the current reviewable draft:

- only selected-in discovered nodes are eligible for canonical import-owned
  blueprint promotion
- excluded nodes must not be promoted
- previously import-owned nodes absent from the selected-in subset must be
  reconciled according to the existing Phase 8 rules
- non-import-owned project-local records must remain untouched

### History and restore contract

History should remain truthful and append-only:

- the raw historical draft payload remains the original discovered draft
- the current review selection state applies only to the latest reviewable job
- restoring a historical review draft creates a fresh latest review job with a
  fresh default selection state of all nodes included
- restoring an older applied import continues to restore the historical applied
  state; it does not retroactively inherit later review selections

### UI contract

`ProjectSessionsPage` should gain a lightweight review control surface:

- show the discovered nodes on the current review draft in a compact,
  reviewable list
- let the user exclude or re-include an individual node before apply
- show truthful counts for total, included, and excluded draft nodes
- keep the existing apply button and review flow rather than introducing a new
  dedicated import editor page

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server tests around apply and restore behavior

Implementation should stay bounded to selection-based merge controls on the
latest reviewable draft. If the work starts requiring draft editing, edge merge
tooling, or generalized historical reconciliation, stop and split that into a
later spec.

## Acceptance Criteria

- the latest `review_pending` import draft exposes per-node include/exclude
  controls
- review selection persists for the latest reviewable draft across reloads
- applying the draft promotes only the selected-in nodes
- excluded nodes are treated as absent for apply-time reconciliation
- restoring a historical review draft resets selection to all included on the
  new latest job
- restoring a historical applied import does not retroactively alter historical
  draft payloads or prior applied history
- no edge-level merge tooling, draft editing, or detached merge UI is
  introduced in this slice

## Verification Plan

### Server

- tests proving selection defaults to all included for new reviewable drafts
- tests proving selection toggles persist for the latest review job only
- tests proving apply promotes only selected nodes and excludes deselected ones
- tests proving previously import-owned nodes absent from the selected subset
  are reconciled under the existing safety rules
- tests proving historical restore-review creates a fresh latest selection
  state instead of reusing stale prior toggles

### Web

- `ProjectSessionsPage` tests for rendering discovered nodes with include/
  exclude affordances
- tests proving the included/excluded counts update after toggle actions
- tests proving apply uses the updated selection state
- tests proving restored review drafts render with all nodes selected by
  default

## Rollback And Fallback

- if persisting per-node selection proves too broad for one slice, fall back to
  a minimal exclude-list keyed by deterministic draft node identity rather than
  broadening into full draft editing
- if the UI starts requiring a large diff-management surface, stop and split
  the presentation problem into a later spec rather than expanding this slice

## Open Questions

None. The slice is implemented and verified.
