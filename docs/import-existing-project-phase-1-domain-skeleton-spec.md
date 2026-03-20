# Import Existing Project Phase 1 Domain Skeleton Spec

**Status:** Implemented  
**Date:** 2026-03-19  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)

> Delivery update (2026-03-19): the queued import domain skeleton is now
> implemented across server persistence, API routes, and the projects UI. The
> verification snapshot for this slice is recorded below.

## Objective

Create the canonical first implementation slice for `Import Existing Project`:
introduce a project-owned import domain model, persistence layer, and truthful
API surface without yet shipping clone, analysis, or review automation.

This slice exists to replace the current planning gap where the repo has an
active import thread and detailed research, but no bounded execution artifact
for the first implementation pass.

## User Outcome

A user can start an import request for a supported source type and the server
creates durable import records that truthfully represent the work as queued.

The product does **not** yet promise a completed GitHub/local import, project
analysis, or seeded Socratic handoff in this slice.

## Locked Decisions For This Slice

These decisions are treated as settled for Phase 1 so implementation can stay
bounded:

- product framing remains `Import Existing Project`
- provider model is import-provider based, not GitHub-only product modeling
- initial providers are `github` and `local`
- GitHub uses a managed clone policy later; local uses validated absolute paths
  later
- import findings do **not** auto-merge into canonical blueprint
- import review remains required before canonical blueprint commit

## Scope

### In scope

- add a server-owned import job record
- add a project source binding record
- persist both records durably alongside existing project persistence
- add provider and status enums shared across API surfaces
- add `POST /projects/imports`
- add `GET /projects/imports/{jobId}`
- create the canonical `Project` when an import request is accepted
- normalize and persist the requested source reference as import metadata
- return truthful queued-state responses to the client
- add minimal web client typing and submission support for the new endpoints
- add a minimal import CTA and pending-state UI on the projects surface
- add tests for creation, persistence, serialization, and basic state fetch

### Out of scope

- cloning any repository
- validating remote GitHub reachability
- validating local path contents beyond basic shape checks required to store the
  request safely
- background workers that advance jobs into cloning or analyzing
- project-scoped import draft generation
- blueprint merge or review/apply endpoints for imported findings
- seeded draft session creation
- import-review-first lobby behavior
- re-import, branch selection, private GitHub auth, or provider expansion

## Current-State Evidence

- The server already has canonical project persistence in
  `planner-server/src/project.rs`.
- The project/session product shell already exists in
  `planner-server/src/api.rs`,
  `planner-web/src/pages/ProjectsPage.tsx`, and
  `planner-web/src/pages/ProjectSessionsPage.tsx`.
- There is currently **no** product import module, import API route, import UI,
  or import persistence model in the codebase.
- The only existing `import` behavior is blueprint discovery edge-proposal
  import, which is not the same feature.

## Requirements

### Domain model

Add durable server-side records for:

```rust
enum ImportProvider {
    GitHub,
    Local,
}

enum ImportStatus {
    Queued,
    Failed,
}

struct ProjectSourceBinding {
    project_id: Uuid,
    provider: ImportProvider,
    canonical_ref: String,
    default_branch: Option<String>,
    head_revision: Option<String>,
    local_root: Option<String>,
    managed_checkout: bool,
    created_at: String,
    updated_at: String,
}

struct ProjectImportJob {
    id: Uuid,
    project_id: Uuid,
    provider: ImportProvider,
    requested_ref: String,
    status: ImportStatus,
    progress_message: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
}
```

Implementation may refine field names, but the shape above is the contract to
preserve:

- the `Project` remains the canonical product container
- import-specific retryable state lives outside the `Project`
- source binding and import-job history are distinct concepts

### Request contract

`POST /projects/imports` accepts:

```json
{
  "provider": "github" | "local",
  "source_ref": "https://github.com/org/repo" | "/absolute/path/to/repo"
}
```

Rules:

- reject unknown providers
- reject empty or whitespace-only `source_ref`
- normalize provider casing and reference formatting before persistence
- create the `Project` and the import records in one request path
- return `201 Created` with `{ project, import_job, source_binding }`

For this slice, queued means:

- request accepted
- project created
- source binding stored
- import job stored
- no acquisition/analysis work has started yet

### Read contract

`GET /projects/imports/{jobId}` returns the stored job, its owning project
reference, and the persisted source binding data needed for later phases.

The read path must be truthful:

- if no worker exists yet, the job stays `queued`
- do not imply clone/analyze progress that the server cannot actually perform

### UI contract

Add a minimal import entry point on the projects surface:

- a visible `Import Existing Project` action on `ProjectsPage`
- a modal or lightweight form that captures provider and source reference
- a pending/queued confirmation state after successful submission

This slice does **not** require:

- a dedicated import dashboard
- progress polling UI beyond the initial queued confirmation
- Home hub import routing

### Ownership and security

- imported projects are owned by the requesting user
- import job fetch must respect the same ownership checks as projects
- local-path requests must require absolute paths
- do not store secrets or auth material in import records

## Acceptance Criteria

- `POST /projects/imports` creates a canonical project plus durable import job
  and source binding records
- `GET /projects/imports/{jobId}` returns the stored queued job for the owner
- the new records survive persistence reloads
- the provider/reference contract is type-safe in server and web code
- the projects UI exposes a truthful import action and shows queued-state
  feedback after submission
- the new slice introduces no fake progress language around clone/analyze work
- no blueprint, discovery, or Socratic behavior changes are required to ship
  this slice

## Verification Plan

### Server

- API tests for request validation and response shape
- persistence tests for import jobs and source bindings
- ownership tests for job fetch
- serialization tests for provider/status enums

### Web

- client typing tests for the new endpoints
- projects-page tests for import CTA, submit success, and queued-state feedback
- failure-path UI tests for invalid request and API error rendering

### Manual

- create a GitHub import request and verify queued response
- create a local-path import request and verify queued response
- reload the server and confirm the queued job can still be fetched

## Verification Snapshot (2026-03-19)

Passed:

- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server import_store_round_trips_records_from_disk -- --nocapture`
- `cargo test -p planner-server tier2_archive_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectsPage.test.tsx`

## Rollback / Fallback

- if import-job persistence proves unstable, hide the import CTA and keep the
  server routes behind the same bounded record model until fixed
- do not ship any UI affordance that implies import analysis is already active

## Remaining Follow-On Work

After this slice lands, later specs should cover:

- GitHub acquisition and managed checkout
- local-path validation beyond basic request safety
- analysis and project-scoped import draft generation
- review/apply flow into canonical blueprint
- seeded session creation and Socratic handoff
- re-import and idempotency semantics
