# Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md)

## Objective

Advance `Import Existing Project` from first-run provider parity into a durable
project-owned lifecycle.

This slice should let a project with an existing source binding perform an
explicit re-import through the same import pipeline, prevent accidental
duplicate-project creation for already-bound sources, and clean up import-owned
artifacts when a project is deleted.

It does **not** yet introduce import history views, diff UIs, branch selection,
or automatic reconciliation of previously applied canonical blueprint state.

## User Outcome

After this slice:

- a project with an existing GitHub or local source binding can be explicitly
  refreshed through a project-level re-import action
- re-import reuses the existing project instead of minting a second project for
  the same source by accident
- the refreshed import runs through the same analysis, seeded-session, and
  review/apply path already used for first-run imports
- deleting a project removes import-owned records and managed GitHub checkout
  state
- deleting a project with a local source binding never deletes the user-owned
  local repo path

The user still does **not** get import history browsing, diff visualization
between applied imports, or automatic rollback of already-applied canonical
blueprint state.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 6 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`
- `planner-web/src/api/__tests__/client.test.ts`

Delivered behavior:

- `POST /projects/imports` now rejects duplicate visible source bindings with an
  explicit conflict payload instead of minting a second project silently
- `GET /projects/{projectRef}/import-state` exposes the latest import state for
  an existing imported project
- `POST /projects/{projectRef}/reimport` creates a fresh job against the
  existing project binding and reuses the shared import worker for both GitHub
  and local sources
- `ProjectSessionsPage` now exposes a project-level `Re-import` action and
  latest-import status feedback without replacing the existing review/apply
  surface
- project delete now purges import jobs, import drafts, and managed GitHub
  checkout directories while preserving external local roots
- delete summaries and client types now report import-owned cleanup counts

Verification completed:

- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server reimport -- --nocapture`
- `cargo test -p planner-server delete_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- re-import is an explicit **project-level action**, not a hidden side effect
  of repeating `POST /projects/imports`
- re-import reuses the existing `ProjectSourceBinding` and canonical project
  rather than creating a second project
- each re-import creates a new import job and draft payload so the current
  refresh attempt is durable and reviewable
- the latest successful re-import becomes the current review/apply handoff for
  the project
- `POST /projects/imports` should refuse accidental duplicate project creation
  when the normalized source is already bound to a visible project and direct
  the caller toward re-import instead
- project delete must remove import jobs, import drafts, and managed GitHub
  checkout state owned by the deleted project
- project delete must **not** delete externally owned local source roots
- this slice does **not** auto-prune or auto-revert canonical blueprint nodes
  previously applied from older import drafts

## Scope

### In scope

- add an explicit project-level re-import endpoint for the existing source
  binding
- rerun GitHub or local import through the shared background processing path
  against the existing project
- persist a new import job for each re-import request
- keep the latest re-import draft/review state available through the existing
  project-level review surface
- prevent duplicate project creation for an already-bound canonical source in
  the create-import path
- extend project delete/import lifecycle handling to remove:
  - import job records
  - import draft records
  - managed GitHub checkout directories
- preserve external local paths during cleanup
- add minimal UI affordance for re-import on an existing imported project

### Out of scope

- import history timeline or job browser
- re-import diff visualization against prior draft or applied blueprint
- rollback of previously applied canonical blueprint state
- branch selection or private GitHub auth
- provider expansion beyond the existing GitHub + local paths
- background scheduling or automatic refresh policies

## Current-State Evidence

- The import feature now supports first-run GitHub and local sources through
  acquisition/validation, analysis, seeded-session handoff, and explicit
  review/apply in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs),
  [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  and [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx).
- `POST /projects/imports` still creates a brand-new project every time, so the
  repo does not yet guard against accidental duplicate imports of the same
  source.
- Import artifacts are now durable on disk, but delete-time cleanup for
  `imports/` and managed checkout state has not been explicitly scoped into the
  project lifecycle yet.
- The source research already identifies re-import and cleanup as the remaining
  durability gap after provider parity.

## Requirements

### Re-import contract

For a project with an existing `ProjectSourceBinding`:

- the server must expose an explicit re-import endpoint bound to the project
- re-import must create a fresh import job for the existing source binding
- the shared worker should handle provider-specific preparation exactly as it
  already does for first-run imports:
  - GitHub: refresh managed checkout and continue into analysis
  - local: revalidate external path and continue into analysis
- re-import must preserve the existing canonical project identity

This slice does not require a separate re-import pipeline.

### Duplicate-source protection contract

For `POST /projects/imports`:

- if the normalized source ref is already bound to an existing visible project,
  the server should not create a second project silently
- the response should be explicit enough for the client to route the user back
  to the existing project and use re-import instead

Implementation may choose the exact status code and response shape, but the
user-visible outcome must prevent accidental duplicate project creation.

### Review handoff contract

Re-import should converge on the existing review model:

- a fresh re-import that succeeds should produce a new draft payload and seeded
  session
- the project-level review surface should show the latest pending/applied import
  handoff for the project
- review/apply remains explicit; re-import must not auto-apply into canonical
  blueprint

### Cleanup contract

When a project is deleted:

- import jobs owned by the project must be removed
- import drafts owned by the project must be removed
- managed GitHub checkout directories owned by the project must be removed
- local external roots must not be deleted or mutated

This slice does not require cleanup of previously applied canonical blueprint
nodes beyond the existing project deletion behavior already handled elsewhere.

### UI contract

The UI should stay lightweight and project-first:

- surface a `Re-import` action on the existing project workflow surface
- if the user attempts to import a source already bound to a project, route them
  toward the existing project instead of creating a hidden duplicate
- keep review/apply and seeded-session navigation on the current project pages

This slice does **not** require an import management dashboard.

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/project.rs](/home/thetu/planner/planner-server/src/project.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectsPage.test.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- the Phase 06 project lifecycle implementation docs where import-owned cleanup
  behavior becomes part of the current lifecycle truth

Implementation may add a small import cleanup helper, but execution should stay
bounded to re-import plus import-owned lifecycle cleanup.

## Acceptance Criteria

- a project with an existing source binding can trigger an explicit re-import
  without creating a new project
- re-import creates a fresh import job and reaches the existing review/apply
  path through the shared import pipeline
- first-run import creation no longer silently creates duplicate projects for an
  already-bound source
- project delete removes import-owned records and managed GitHub checkout state
- project delete preserves external local source roots
- re-import, duplicate-source protection, and lifecycle cleanup remain bounded
  to the existing project-first workflow without introducing a second import UI

## Verification Plan

### Server

- re-import API tests for GitHub and local source bindings
- tests proving duplicate create-import attempts for an existing source do not
  mint a new project
- project delete tests proving import jobs/drafts are removed
- managed-checkout cleanup tests for GitHub imports
- local-path delete tests proving external roots are preserved

### Web

- `ProjectSessionsPage` tests for re-import action visibility and happy-path
  routing
- `ProjectsPage` tests for duplicate-source conflict handling
- client tests for any new re-import or conflict payload shapes

### Manual

- re-import an existing imported project and confirm the same project reaches a
  fresh review state
- attempt to import the same source twice and confirm Planner routes to the
  existing project
- delete a GitHub-imported project and confirm managed checkout cleanup
- delete a local-imported project and confirm the original local repo remains
  untouched

## Rollback / Fallback

- if duplicate-source protection is too ambiguous to resolve in create-import,
  fail explicitly rather than minting a second project
- if cleanup broadens into applied-blueprint rollback semantics, stop and split
  that into a later spec
- if re-import UI becomes too large, keep it on the project sessions surface
  rather than inventing a dedicated management page

## Open Questions

These are explicitly deferred and do not block this slice:

- whether re-import should eventually offer draft-vs-canonical diffing
- whether import job history should become user-visible later
- whether duplicate-source protection should later include archived projects or
  team-shared ownership cases
- whether import refresh should eventually support branch selection
