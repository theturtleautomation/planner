# Import Existing Project Phase 12 Historical Applied Restore For Review Spec

**Status:** Ready for implementation  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 11 Selective Apply Merge Controls Spec](/home/thetu/planner/docs/import-existing-project-phase-11-selective-apply-merge-controls-spec.md)

## Objective

Advance `Import Existing Project` from current-draft merge controls into
broader historical reconciliation by restoring a historical applied import into
the current review slot instead of directly into canonical blueprint state.

This slice should let a user take an older historical `applied` import from
project history, reopen it as a fresh latest `review_pending` draft, and then
use the existing Phase 11 merge controls before applying it. It should reuse
the existing append-only history model rather than inventing a second mutable
history lane.

It does **not** yet introduce multi-history compare, cross-history cherry-
picking, or a generalized rollback dashboard.

## User Outcome

After this slice:

- a project can reopen a historical `applied` import into the current review
  slot instead of immediately restoring it into canonical blueprint state
- the reopened historical applied import becomes a fresh latest
  `review_pending` job for auditability
- the reopened historical draft defaults to all nodes included and can use the
  existing Phase 11 merge controls before apply
- canonical blueprint state remains unchanged until the user explicitly applies
  the reopened historical draft
- users can safely revisit an older known-good applied import and selectively
  reintroduce only part of it

The user still does **not** get free-form time travel, compare-any-two history
entries, or per-edge historical editing.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- the restore target is only a historical import job for the **same project**
  with status `applied` and a durable draft payload
- restore-for-review creates a **new append-only latest `review_pending` job**
  instead of mutating the historical applied job in place
- restore-for-review does **not** mutate canonical blueprint state directly
- the reopened historical draft defaults to all nodes included, using the
  existing Phase 11 review selection model
- if the project already has a current latest `review_pending` draft, restore-
  for-review is blocked until that review is resolved
- the action stays on the existing `ProjectSessionsPage` history surface
- this slice does not remove or replace the existing direct historical applied
  restore path from Phase 9

## Scope

### In scope

- add a project-level action to reopen a historical `applied` import as a new
  latest `review_pending` draft
- validate eligibility against project ownership, applied status, and draft
  presence
- clone the historical draft payload into a fresh latest review job with
  restore lineage
- reset Phase 11 review selection state to all included for the fresh latest
  review job
- expose the new restore-for-review affordance on eligible historical applied
  entries
- keep the existing direct restore action available so product can support both
  “restore immediately” and “restore for review” paths
- add focused tests for gating, lineage, reset selection state, and non-
  destructive workflow behavior

### Out of scope

- removing the direct historical applied restore path
- restoring while a current `review_pending` draft exists
- compare-any-two history entries
- multi-draft merge tooling
- edge-level history editing
- automatic blueprint mutation during restore-for-review

## Current-State Evidence

- Phase 9 already restores a historical `applied` import directly into
  canonical blueprint state
- Phase 10 already restores a historical `review_pending` draft into the
  current review slot without touching canonical blueprint state
- Phase 11 now gives the current latest reviewable draft per-node exclude-from-
  apply controls
- what is still missing is the bridge between historical applied imports and
  the new review-time merge controls

## Requirements

### Restore-for-review contract

For an eligible historical `applied` import:

- the server must expose a restore-for-review action bound to that historical
  job
- restore-for-review must create a new latest `review_pending` job with
  lineage back to the historical applied job
- the restored latest draft must reuse the historical draft payload
- canonical blueprint state must remain unchanged until the user later presses
  `Apply Import Draft`

### Selection contract

The reopened historical applied draft must integrate cleanly with Phase 11:

- the new latest review job starts with all nodes included
- the project review surface can immediately use the existing merge controls
- stale selection state from the historical applied job must not leak into the
  new latest review job

### Workflow contract

The project workflow should remain single-threaded:

- if a current latest `review_pending` draft already exists, restore-for-review
  must be blocked with an explicit error
- restore-for-review should reuse the existing latest review card and apply
  flow rather than introducing a second review page
- history should remain append-only and explain that the new latest review
  draft originated from a historical applied import

### UI contract

`ProjectSessionsPage` should gain a second lightweight action on eligible
historical applied history rows:

- `Restore This Import` keeps the existing direct-restore-to-canonical path
- `Restore For Review` creates a fresh latest `review_pending` draft instead
- the page should make the difference between those actions clear

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server tests around history restore behavior

Implementation should stay bounded to reopening historical applied imports into
the review flow. If the work starts requiring generalized history comparison or
arbitrary multi-version merge tooling, stop and split that into a later spec.

## Acceptance Criteria

- an eligible historical `applied` import can be reopened into a fresh latest
  `review_pending` job
- restore-for-review does not mutate canonical blueprint state directly
- the reopened historical applied draft defaults to all nodes included under
  the existing review selection model
- restore-for-review is blocked while a current `review_pending` draft exists
- the UI clearly distinguishes direct restore vs restore-for-review on
  historical applied entries
- no generalized history dashboard or multi-draft merge tooling is introduced
  in this slice

## Verification Plan

### Server

- tests proving restore-for-review rejects invalid job ids, non-applied jobs,
  and missing draft payloads
- tests proving restore-for-review is blocked by an existing latest
  `review_pending` draft
- tests proving restore-for-review creates a new latest `review_pending` job
  with lineage back to the historical applied job
- tests proving the reopened draft starts with all nodes included
- tests proving canonical blueprint state remains unchanged until later apply

### Web

- `ProjectSessionsPage` tests for rendering both restore actions on eligible
  historical applied entries
- tests proving restore-for-review refreshes the latest review card instead of
  jumping straight to applied state
- tests proving merge controls on the reopened historical applied draft start
  with all nodes included

## Rollback And Fallback

- if representing both direct restore and restore-for-review on the same
  history row becomes confusing in one slice, fall back to a single explicit
  overflow or secondary action treatment rather than broadening into a new
  history management page
- if applied-history lineage needs a shared refactor with earlier restore
  routes, keep that refactor internal and stop before broadening the user-
  facing scope

## Open Questions

None. The slice is ready for bounded implementation.
