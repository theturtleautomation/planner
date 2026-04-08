# Import Existing Project Phase 2 GitHub Acquisition Spec

**Status:** Implemented  
**Date:** 2026-03-19  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)

> Delivery update (2026-03-19): public GitHub acquisition is now implemented
> across server background acquisition, durable checkout metadata persistence,
> and truthful `ProjectsPage` polling/status feedback. The verification snapshot
> for this slice is recorded below.

## Objective

Advance `Import Existing Project` from a truthful queued request into a real
source-acquisition feature for public GitHub repositories.

This slice begins actual import work by cloning the default branch of a public
GitHub repository into Planner-managed storage, persisting acquisition
metadata, and exposing truthful progress through the existing import-job API.

It does **not** yet analyze the checkout, merge findings into blueprint, or
seed a Socratic session.

## User Outcome

A user can paste a public GitHub repository URL into the import flow and
Planner will:

- create the project and import job
- clone the repository default branch into managed storage
- persist the checkout metadata on the import records
- report whether acquisition is queued, cloning, ready, or failed

When the job reaches `ready`, the user has a prepared checkout owned by the
project, but not yet an analyzed import draft or seeded Socratic session.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- this slice supports **public GitHub repos only**
- acquisition uses Git CLI against the repository URL, not the GitHub API
- only the remote default branch is acquired
- GitHub checkouts are cloned into Planner-managed storage under the data dir
- import status remains truthful to the work actually completed:
  - `queued`
  - `cloning`
  - `ready`
  - `failed`
- local-path provider acquisition is deferred to a later spec
- analysis, import-review, blueprint merge, and Socratic handoff stay deferred

## Scope

### In scope

- extend the import job/status model to represent acquisition progress
- start background acquisition work for newly created GitHub import jobs
- normalize GitHub URLs into a canonical persisted ref
- resolve the remote default branch
- clone the repo into a deterministic managed checkout path under Planner data
- persist `default_branch`, `head_revision`, and `local_root` on the source
  binding after a successful clone
- persist truthful progress and failure messages on the import job
- expose updated status through `GET /projects/imports/{jobId}`
- add minimal projects-surface progress polling for the most recent import job
- update UI copy so GitHub acquisition readiness is clear and no later phases
  are implied

### Out of scope

- private GitHub auth
- branch picker support
- local-path provider acquisition
- project-scoped analysis or import draft generation
- blueprint merge or review/apply flow
- seeded session creation or import-review-first lobby behavior
- re-import, idempotency, or duplicate-source detection
- managed checkout cleanup on project delete beyond current lifecycle behavior

## Current-State Evidence

- Phase 1 already created durable import records, the import CTA, and the
  import API surface in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs),
  [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts),
  and [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx).
- The current GitHub path only normalizes the URL and creates a queued job in
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs).
- `ProjectImportStore` currently persists only queued jobs plus source bindings;
  it has no acquisition worker, checkout location, or status-transition helper
  in [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs).
- The current projects UI truthfully says clone and analysis are not part of
  the current slice in
  [planner-web/src/components/ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx)
  and [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx).

## Requirements

### Acquisition contract

For `provider = "github"`:

- `POST /projects/imports` still creates the canonical project, import job, and
  source binding synchronously
- after persistence succeeds, the server starts a background acquisition task
  for that job
- the job transitions:
  - `queued` immediately after request acceptance
  - `cloning` once acquisition begins
  - `ready` after managed checkout metadata is persisted
  - `failed` if acquisition cannot complete

For `provider = "local"`:

- do not broaden this slice into local acquisition behavior
- the existing queued-only behavior remains unchanged until a later spec

### GitHub source normalization

The canonical GitHub ref contract for this slice is:

- accept `https://github.com/org/repo`
- accept trailing `/`
- accept trailing `.git`
- upgrade `http://github.com/...` to `https://github.com/...`
- reject non-GitHub URLs
- reject malformed or incomplete GitHub repo paths

The stored canonical ref should be stable enough to support later duplicate
source detection.

### Managed checkout contract

Successful GitHub acquisition must persist:

- `source_binding.managed_checkout = true`
- `source_binding.local_root = <managed checkout path>`
- `source_binding.default_branch = <resolved remote default branch>`
- `source_binding.head_revision = <checked-out commit sha>`

The managed checkout path must:

- live under the Planner data dir
- be deterministic per project or import job
- avoid colliding with unrelated imports
- be safe to clean up in a later lifecycle spec

### Job progress contract

`GET /projects/imports/{jobId}` must become the source of truth for acquisition
progress:

- `queued` means the request exists but acquisition has not started
- `cloning` means Git acquisition is actively running
- `ready` means the managed checkout is present and the source binding metadata
  has been persisted
- `failed` means acquisition stopped without a usable checkout

The job must also expose a truthful `progress_message` or `error_message`
describing the current terminal or in-flight state.

### Failure behavior

GitHub acquisition failure must be explicit and durable:

- invalid GitHub URLs are rejected synchronously by the request path
- clone failures produce `failed` jobs with a user-visible error message
- a failed GitHub acquisition must not pretend the import is ready
- a partially created checkout directory should be removed or left in a clearly
  non-ready state so later phases are not confused

### UI contract

The projects surface should remain minimal but truthful:

- keep `Import Existing Project` on `ProjectsPage`
- after creating a GitHub import, poll the job status while the latest-import
  banner is visible
- show `queued`, `cloning`, `ready`, or `failed` language without implying
  analysis or Socratic readiness
- when acquisition reaches `ready`, offer the existing project navigation path
- when acquisition fails, surface the server error clearly and stop polling

This slice does **not** require:

- a dedicated import dashboard
- project-page import history
- progress rendering anywhere outside the latest-import feedback surface

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/lib.rs](/home/thetu/planner/planner-server/src/lib.rs)
- [planner-server/src/main.rs](/home/thetu/planner/planner-server/src/main.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectsPage.test.tsx)

Implementation may add helper modules, but execution should stay bounded to
source acquisition and truthful progress.

## Acceptance Criteria

- GitHub import requests progress beyond `queued` without requiring a manual
  second step
- successful GitHub acquisition persists `default_branch`, `head_revision`, and
  a managed `local_root`
- successful GitHub imports return `ready` from
  `GET /projects/imports/{jobId}`
- failed GitHub acquisition returns durable `failed` state with an error
  message
- local-provider behavior is not silently broadened in this slice
- the projects UI reflects acquisition progress truthfully and stops short of
  implying analysis or Socratic readiness
- the slice introduces no blueprint, discovery, or session-seeding behavior

## Verification Plan

### Server

- unit tests for GitHub URL normalization
- import-store tests for status and metadata persistence updates
- API tests for GitHub acquisition success and failure states
- integration tests against a temp bare git repo used as a simulated remote
- tests confirming `GET /projects/imports/{jobId}` returns updated acquisition
  metadata for the owner

### Web

- client typing tests for any updated import status payload shape
- `ProjectsPage` tests for GitHub import progress polling
- `ProjectsPage` tests for `ready` and `failed` terminal banner states

### Manual

- import a small public GitHub repo and verify the job reaches `ready`
- inspect the managed checkout location under the Planner data dir
- reload the server and confirm the ready job metadata still reads correctly

## Verification Snapshot (2026-03-19)

Passed:

- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server github_import -- --nocapture`
- `cargo test -p planner-server import_store -- --nocapture`
- `cargo test -p planner-server git_cli_acquirer_clones_temp_bare_repo_and_reads_metadata -- --nocapture`
- `cargo test -p planner-server tier2_archive_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectsPage.test.tsx`

## Rollback / Fallback

- if background acquisition proves unstable, keep the Phase 1 queued-state
  contract and defer acquisition rather than shipping misleading progress
- if checkout persistence is unreliable, do not claim `ready`; fail the job and
  preserve an actionable error message

## Open Questions

These do not block this slice because they are explicitly deferred:

- when local-path provider acquisition should become active
- whether later re-import should reuse the same managed checkout path
- whether project delete should remove managed checkout data in the same phase
