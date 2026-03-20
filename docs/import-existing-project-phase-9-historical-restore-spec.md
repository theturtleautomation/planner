# Import Existing Project Phase 9 Historical Restore Spec

**Status:** Ready for implementation  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md)

## Objective

Advance `Import Existing Project` from latest-draft reconciliation into bounded
historical restore.

This slice should let a user restore the project’s import-owned canonical
blueprint state to match an earlier applied import draft that already exists in
project history. It should reuse the Phase 8 reconciliation engine rather than
inventing a second rollback pipeline.

It does **not** yet introduce free-form time travel, per-node merge editing, or
destructive deletion of archived import-owned history.

## User Outcome

After this slice:

- a project can restore its import-owned canonical blueprint state to an older
  applied import from project history
- restore uses the persisted historical draft payload, so no source reacquire
  or re-analysis is required
- previously archived import-owned nodes that exist in the selected historical
  draft can become active again
- import history remains append-only and truthful about the restore action
- manually created or otherwise non-import-owned project-local knowledge
  remains untouched

The user still does **not** get arbitrary draft editing, partial rollback,
cross-project restore, or hard-delete cleanup of archived import-owned nodes.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- restore targets only a historical import job for the **same project** with:
  - persisted draft payload present
  - status `applied`
- restore creates a **new append-only import job record** rather than mutating
  the old historical job in place
- the new restore job reuses the historical draft payload as the restore target
  and becomes the latest applied import for the project
- restore reuses the Phase 8 reconciliation path so import-owned project-local
  blueprint state ends up matching the selected historical draft
- restore is blocked while the project has a current `review_pending` draft so
  the workflow stays single-threaded
- restore affects only import-owned project-local blueprint state and must
  never touch shared, global, or non-import-owned project-local records
- this slice does not introduce a new import provider, new source acquisition,
  or a second rollback UI outside `ProjectSessionsPage`

## Scope

### In scope

- add a project-level restore endpoint for a historical applied import job
- validate restore eligibility against project ownership, job status, and draft
  presence
- create a fresh append-only restore job record for auditability
- restore canonical import-owned project-local blueprint state by reconciling
  against the selected historical draft
- reactivate previously archived import-owned nodes when they exist in the
  selected historical draft
- archive import-owned nodes absent from the selected historical draft
- expose eligible restore actions in the existing project import history
  surface
- make history and latest-import messaging truthful that the latest applied
  state was restored from a historical import
- add focused tests for restore safety, lineage, and workflow gating

### Out of scope

- restore from failed or review-pending jobs
- restore while a pending review draft exists
- per-node cherry-picking or merge conflict resolution
- editing a historical draft before restore
- re-running acquisition or analysis during restore
- hard deletion of archived import-owned nodes
- a separate rollback management page

## Current-State Evidence

- Phase 7 already exposes project-scoped import history plus lightweight draft
  comparison, so the user can see older applied imports on
  `ProjectSessionsPage`.
- Phase 8 already reconciles import-owned project-local blueprint state against
  a chosen draft during apply, including archival of stale imported nodes.
- Historical draft payloads are already durable in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  which makes restore possible without reacquiring the source repo.
- The import workflow is still latest-focused today: users can re-import and
  apply the newest draft, but they cannot intentionally restore an older known-
  good import state.

## Requirements

### Restore contract

For a project with import history:

- the server must expose a restore action bound to a specific historical import
  job for that project
- restore must reject targets that are not `applied` or do not have a durable
  draft payload
- restore must create a new latest job entry that clearly records the restore
  lineage from the selected historical job
- the restored canonical import-owned blueprint state must match the selected
  historical draft after reconciliation completes

Implementation may choose the exact lineage field shape, but the user-visible
outcome must stay append-only and auditable.

### Reconciliation contract

Restore must reuse the same import-owned reconciliation safety rules as Phase 8:

- nodes present in the restored historical draft become active project-local
  import-owned records
- previously import-owned nodes absent from the restored draft become archived
- non-import-owned project-local records remain untouched
- shared and global records remain untouched

### Workflow contract

The project workflow should remain single-threaded:

- if the project has a latest `review_pending` draft, restore must be blocked
  with an explicit error telling the user to resolve the pending review first
- restore actions live on the existing import history surface
- the latest import status and history list should explain when the current
  applied state came from a historical restore

### UI contract

`ProjectSessionsPage` should gain a lightweight restore affordance on eligible
historical applied entries:

- only eligible applied history entries show the restore action
- restore updates the latest import status card and history after success
- the page should not imply that restore reran analysis or reacquired source
- the page should continue to use the existing history and latest-state
  surfaces rather than a detached rollback dashboard

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)

Implementation should stay bounded to historical restore against persisted
drafts. If the work starts requiring generalized versioning or arbitrary graph
editing, stop and split that into a later spec.

## Acceptance Criteria

- an eligible historical applied import can be restored from project history
- restore creates a new append-only latest import job instead of mutating the
  old history row
- canonical import-owned blueprint state matches the selected historical draft
  after restore reconciliation
- archived import-owned nodes can be reactivated when present in the restored
  draft
- non-import-owned project-local records remain untouched
- restore is blocked when a project has a current `review_pending` import draft
- no source reacquire, re-analysis, or hard-delete behavior is introduced in
  this slice

## Verification Plan

### Server

- tests proving restore rejects invalid job ids, non-applied jobs, and missing
  draft payloads
- tests proving restore is blocked while a pending review draft exists
- tests proving restore creates a new latest applied job with restore lineage
- tests proving restore reactivates archived import-owned nodes present in the
  target historical draft
- tests proving restore archives import-owned nodes absent from the restored
  draft and preserves non-import-owned records

### Web

- `ProjectSessionsPage` tests for restore action visibility on eligible history
  entries
- tests proving restore refreshes latest import state and history after
  success
- tests proving restore is not shown or is disabled when the project currently
  has a pending review draft

## Rollback And Fallback

- if a clean append-only restore record cannot be represented without
  broadening the import job schema too far, fall back to an explicit latest-job
  metadata field that still preserves the source historical job id
- if reactivating archived import-owned nodes proves unsafe without broader
  provenance refactoring, stop and split that deeper provenance work into a
  later bounded spec rather than guessing

## Open Questions

- whether restore lineage should live on `ProjectImportJob` as a dedicated
  `restored_from_job_id` field or equivalent structured metadata
- whether the UI should surface restore provenance only on the latest status
  card or on both the latest card and the history row
- whether a later slice should allow restore directly from a `review_pending`
  historical draft instead of only previously applied imports
