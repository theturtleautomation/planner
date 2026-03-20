# Import Existing Project Phase 10 Historical Review Draft Restore Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md)

## Objective

Advance `Import Existing Project` from historical applied-import restore into
bounded historical draft restore for review.

This slice should let a user reopen an older historical `review_pending` import
draft as the current review target for the project without restoring it
directly into canonical blueprint state. It should reuse the existing draft
payloads and project review workflow rather than inventing a parallel draft
management system.

It does **not** yet introduce per-node merge controls, arbitrary draft editing,
or direct restore from failed history.

## User Outcome

After this slice:

- a project can reopen an older historical import draft that previously reached
  `review_pending`
- reopening creates a fresh latest `review_pending` job so the workflow remains
  append-only and auditable
- the reopened draft becomes the current review/apply target on
  `ProjectSessionsPage`
- reopening does not mutate canonical blueprint state until the user explicitly
  applies the restored draft
- users can recover a previously reviewable import draft without rerunning
  acquisition or analysis

The user still does **not** get partial merge editing, direct restore from
failed history, or any automatic blueprint mutation during draft reopen.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 10 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/api/__tests__/client.test.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- `POST /projects/{projectRef}/import-history/{jobId}/restore-review-draft` now
  reopens an older historical `review_pending` draft into a fresh latest
  `review_pending` job
- restore-review-draft reuses the persisted draft payload and does not mutate
  canonical blueprint state
- restored draft jobs now carry lineage back to the historical reviewable job
- `ProjectSessionsPage` now exposes `Restore Draft For Review` on eligible
  historical `review_pending` rows and refreshes the latest review card after
  success

Verification completed:

- `cargo test -p planner-server restore_project_import_review_draft -- --nocapture`
- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server import_history -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/ProjectSessionsPage.test.tsx src/api/__tests__/client.test.ts`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- restore targets only a historical import job for the **same project** with:
  - persisted draft payload present
  - status `review_pending`
- reopening a historical draft creates a **new append-only import job record**
  with status `review_pending`; it does not mutate the historical job in place
- reopening a historical draft does **not** touch canonical blueprint state by
  itself
- reopened drafts reuse the existing review/apply path, including Phase 8
  reconciliation only when the user later presses `Apply Import Draft`
- reopening is blocked while the project already has a current latest
  `review_pending` draft so the review workflow stays single-threaded
- this slice does not add a second draft-review page; the restored draft stays
  on the existing `ProjectSessionsPage` review surface
- this slice does not add merge controls, per-node accept/reject, or manual
  editing of restored draft contents

## Scope

### In scope

- add a project-level restore endpoint for a historical `review_pending` import
  job
- validate restore eligibility against project ownership, job status, and draft
  presence
- create a fresh append-only latest `review_pending` job that carries lineage
  back to the historical draft being reopened
- clone the historical draft payload onto the new latest job
- expose eligible restore actions in the project import history surface
- refresh the latest import state and review card to point at the reopened
  draft after success
- add focused tests for reopen safety, lineage, and gating rules

### Out of scope

- applying the restored draft automatically
- any canonical blueprint mutation during reopen
- restore from `failed`, `queued`, `cloning`, `analyzing`, or `applied` jobs
- per-node merge controls
- editing the draft contents before review/apply
- a separate draft-restore management page

## Current-State Evidence

- Phase 7 already exposes project-scoped import history plus the current review
  surface on `ProjectSessionsPage`.
- Phase 9 now restores historical applied imports directly into canonical
  blueprint state, but there is still no way to recover an older draft that was
  reviewable but never applied.
- Historical draft payloads are already durable in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  which makes reopening an older reviewable draft possible without reacquiring
  the repo.
- The current product still treats the latest reviewable draft as ephemeral
  from the user’s perspective once a different import becomes current.

## Requirements

### Historical draft restore contract

For a project with import history:

- the server must expose an action bound to a specific historical import job
- the action must reject targets that are not historical `review_pending` jobs
  or do not have a durable draft payload
- a successful restore must create a new latest `review_pending` job and draft
  payload that become the current review target for the project
- the restored latest draft must preserve lineage back to the historical job

Implementation may choose the exact lineage field shape, but the outcome must
remain append-only and auditable.

### Workflow contract

The project review workflow should remain single-threaded:

- if the project already has a current latest `review_pending` draft, restoring
  another historical draft must be blocked with an explicit error
- reopening a historical draft should make that new latest draft appear on the
  existing review/apply card
- after reopen, the user should still explicitly choose whether to apply the
  draft through the existing apply path

### Safety contract

Historical draft restore must be non-destructive:

- restoring a historical `review_pending` draft must not mutate canonical
  blueprint state
- restoring must not touch shared/global records
- restoring must not delete or archive existing import-owned blueprint records
- if lineage is ambiguous or the draft payload is missing, the restore must
  fail explicitly instead of guessing

### UI contract

`ProjectSessionsPage` should gain a lightweight restore affordance on eligible
historical `review_pending` entries:

- only eligible historical `review_pending` entries show the restore action
- the page should explain that reopening a draft restores it for review, not
  for direct blueprint apply
- after restore, the latest import state and review card should point at the
  reopened draft

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)

Implementation should stay bounded to reopening historical reviewable drafts. If
the work starts requiring merge tooling or draft editing, stop and split that
into a later spec.

## Acceptance Criteria

- an eligible historical `review_pending` import can be reopened from project
  history
- reopening creates a new latest `review_pending` job instead of mutating the
  old history row
- reopening makes the restored draft the current review/apply target
- canonical blueprint state remains unchanged until the user explicitly applies
  the reopened draft
- restore is blocked when the project already has a current latest
  `review_pending` draft
- no merge controls, auto-apply behavior, or direct blueprint mutation are
  introduced in this slice

## Verification Plan

### Server

- tests proving restore rejects invalid job ids, non-`review_pending` jobs, and
  missing draft payloads
- tests proving restore is blocked while a project already has a current latest
  `review_pending` draft
- tests proving restore creates a new latest `review_pending` job with restore
  lineage
- tests proving canonical blueprint state is unchanged by the reopen action

### Web

- `ProjectSessionsPage` tests for restore action visibility on eligible
  historical `review_pending` entries
- tests proving restore refreshes latest import state and the review/apply card
  after success
- tests proving the UI copy makes clear that the action restores a draft for
  review, not direct blueprint apply

## Rollback And Fallback

- if a clean append-only restored-draft record cannot be represented without
  broadening the import job schema too far, fall back to a minimal lineage
  field on the new latest job that still preserves the source historical job id
- if the restore path starts coupling too tightly to Phase 9 applied-import
  restore semantics, stop and split that shared lineage refactor into a separate
  bounded spec rather than broadening this slice

## Open Questions

- whether the new latest restored-draft job should reuse
  `restored_from_job_id` or add a more draft-specific lineage field
- whether reopened historical drafts should remain labeled `review_pending` in
  history or get a more explicit user-facing status label later
- whether a later slice should allow promoting a historical `applied` import
  back into a reviewable draft instead of only direct restore
