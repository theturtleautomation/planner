# Import Existing Project Phase 5 Local Provider Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)

## Objective

Advance `Import Existing Project` from GitHub-only source acquisition into the
second locked v1 provider: local repo import.

This slice should let a user point Planner at an existing absolute local path,
validate it as an importable source root, reuse the already-shipped analysis,
seeded-session, and review/apply pipeline, and reach the same project-scoped
review state that GitHub imports already support.

It does **not** yet solve re-import semantics, duplicate-source detection, or
managed/external-root cleanup on project delete.

## User Outcome

A user can choose the local provider in the existing import flow, enter an
absolute local repo path, and Planner will:

- create the canonical project and import job
- validate the local source root without cloning it
- persist source metadata with `managed_checkout = false`
- analyze the local root through the same import-draft pipeline already used
  for GitHub
- create the same seeded project session
- reach the same `review_pending` state and project-level review/apply path

The user still does **not** get re-import, duplicate-source reuse, branch
selection, or delete-time external-root cleanup in this slice.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 5 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/components/ImportProjectModal.tsx`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- local provider imports now enter the shared background import worker instead
  of stalling at queued-only status
- local absolute paths are validated in place and persisted with
  `managed_checkout = false`
- local imports reuse the existing analysis, seeded-session, and review/apply
  pipeline
- best-effort local Git metadata is persisted when available
- import UI copy is now truthful about local-path behavior

Verification completed:

- `cargo test -p planner-server local_import -- --nocapture`
- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server github_import -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- local provider support is part of the v1 import product
- local import uses an existing absolute filesystem path directly
- local import must **not** copy, move, or delete the user-owned source root
- `source_binding.managed_checkout = false` for local provider imports
- the shared post-acquisition pipeline remains the product shape:
  - analysis
  - import draft persistence
  - seeded session creation
  - explicit project-level review/apply
- local provider imports should reuse the current shared import worker instead
  of forking a second product flow
- branch picker support, re-import, duplicate-source detection, and lifecycle
  cleanup remain deferred to later specs

## Scope

### In scope

- extend the import request path so `provider = "local"` progresses beyond the
  current queued-only stub behavior
- validate local import paths as absolute, existing, readable directories
- persist `local_root` immediately from the validated external path
- best-effort capture local Git metadata when available:
  - `default_branch`
  - `head_revision`
- move successful local imports into the existing analysis pipeline without
  cloning
- reuse the current import-draft, seeded-session, and review/apply behavior
  for local imports
- update UI copy so local provider behavior is truthful and no managed clone is
  implied
- add focused tests for local-path success and failure behavior

### Out of scope

- re-import or refresh of an existing local source binding
- duplicate-source detection across projects
- branch selection or non-default branch import
- local path browsing UI beyond the existing text input
- cleanup of external local roots on project delete
- private GitHub auth or GitHub re-import changes
- import history UX across multiple jobs

## Current-State Evidence

- The import product now has end-to-end GitHub support through acquisition,
  analysis, seeded-session handoff, and explicit review/apply in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs),
  [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  and [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx).
- The current request path already accepts `provider = "local"` only when the
  source path is absolute, but local imports stop at queued state and do not
  enter analysis or review in
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs).
- The source research already locks the intended local-provider model:
  validated absolute local path with the same downstream pipeline as GitHub in
  [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md).
- The current UI still tells the user that local acquisition lands later, so
  the product copy is now stale after Phase 4.

## Requirements

### Local acquisition contract

For `provider = "local"`:

- `POST /projects/imports` still creates the canonical project, import job, and
  source binding synchronously
- the server validates that the provided source ref is:
  - an absolute path
  - an existing directory
  - readable enough for the existing analysis pipeline to traverse
- after validation succeeds, the server starts the same shared background import
  processing path used by GitHub, but without clone work

The local import status progression must stay truthful to the work completed:

- `queued` immediately after request acceptance
- `analyzing` once the validated local root enters shared analysis work
- `review_pending` once draft persistence and seeded-session handoff succeed
- `failed` if validation or analysis cannot complete

This slice does **not** require a local-only intermediate status beyond the
existing model.

### Local source metadata contract

Successful local imports must persist:

- `source_binding.managed_checkout = false`
- `source_binding.local_root = <validated absolute path>`
- `source_binding.canonical_ref = <validated absolute path>`

When the local path is a Git checkout, the system should also persist
best-effort metadata:

- `source_binding.default_branch`
- `source_binding.head_revision`

If Git metadata cannot be resolved but the directory is otherwise importable,
implementation may leave those fields empty rather than failing the import.

### Shared pipeline contract

After a local source root is validated, all later behavior should match the
existing GitHub path:

- analysis runs against `PreparedImportSource.local_root`
- import draft state is persisted with explicit project scope
- seeded session creation uses the same project-description brief path
- project-level review/apply remains the approval surface

This slice should not introduce a second local-only review workflow.

### Failure behavior

Failure handling must be explicit and safe:

- non-absolute local paths remain rejected synchronously
- missing or unreadable local directories produce durable `failed` jobs with a
  clear error message
- failed local imports must not create review-ready draft state
- local provider failure must never delete or mutate the user-owned source root

### UI contract

The existing import surfaces should stay lightweight but truthful:

- keep local provider choice in `ImportProjectModal`
- remove stale copy that says local import lands later
- on success, local imports should reach the same latest-import review feedback
  path and project sessions review surface as GitHub imports
- on failure, local import errors should surface through the same latest-import
  feedback path without implying clone cleanup or managed storage

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/lib.rs](/home/thetu/planner/planner-server/src/lib.rs)
- [planner-server/src/main.rs](/home/thetu/planner/planner-server/src/main.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-web/src/components/ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx)
- [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/__tests__/ProjectsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectsPage.test.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)

Implementation may add a small local-source metadata helper, but execution
should stay bounded to local-provider parity on top of the existing import
pipeline.

## Acceptance Criteria

- local import requests no longer stall at queued-only behavior
- successful local imports validate the external path, persist
  `managed_checkout = false`, and set `local_root`
- successful local imports reach `review_pending` and create the same seeded
  session/import draft handoff as GitHub imports
- local imports can use the existing project-level review/apply path without a
  local-only workflow
- invalid or unreadable local paths produce truthful failure behavior and do
  not create review-ready draft state
- local imports never copy, move, or delete the external source root
- re-import, duplicate-source detection, and lifecycle cleanup remain deferred

## Verification Plan

### Server

- import API tests for local-path validation success and failure
- tests proving successful local imports reach `review_pending` with seeded
  session and draft state
- tests proving local imports persist `managed_checkout = false` and the
  validated `local_root`
- tests proving local failure does not create review-ready draft state

### Web

- `ProjectsPage` tests for local import copy and latest-import feedback
- `ProjectSessionsPage` tests proving local imports surface through the same
  review banner contract
- client tests for any changed local import status payloads

### Manual

- import a small local repo and confirm it reaches project review without
  creating a managed clone
- validate against a real local path such as `~/recipes` once implementation
  lands
- confirm a broken local path fails cleanly and leaves the source directory
  untouched

## Rollback / Fallback

- if local-path validation is noisier than expected, fail fast with a durable
  `failed` job rather than inventing partial review-ready state
- if local Git metadata resolution proves unreliable, keep metadata optional and
  preserve the shared analysis path
- if local-provider parity broadens unexpectedly into re-import or cleanup
  semantics, stop and split those into later specs

## Open Questions

These are explicitly deferred and do not block this slice:

- whether repeated imports of the same local path should reuse an existing
  project
- whether local imports should require a detectable Git checkout versus any
  readable project directory
- whether branch selection for local worktrees belongs in the same provider
  family later
- how delete-time cleanup should record or prune local import job history
