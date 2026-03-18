# Phase 06 Project Archive And Delete Implementation

**Status:** Implemented — focused validation green; manual regression/signoff still pending  
**Date:** 2026-03-08

> Update (2026-03-18): The Phase 06 lifecycle work is now implemented in code
> across server, core, and web. Focused validation passed via:
> `cargo test -p planner-server archive_project -- --nocapture`,
> `cargo test -p planner-server delete_project -- --nocapture`,
> `cargo test -p planner-core purge_project -- --nocapture`, and
> `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectsPage.test.tsx`.
> The phased plan below is retained as the implementation record; use
> `phase-06f-project-lifecycle-hardening-execution-checklist.md` for the
> current closure status and any remaining manual regression notes.

## Objective

Add explicit project lifecycle actions for `Archive` and `Delete`, where:

- `Archive` hides a project from the default working surface without removing
  any underlying data.
- `Delete` is a true destructive operation that stops active work, removes
  owned sessions and associated durable records, deletes project-local
  blueprint data, and unlinks shared knowledge from the deleted project.

This phase is complete when the backend owns a real lifecycle contract, the UI
can expose archive and delete safely, and destructive behavior is covered by
server and frontend tests rather than relying on best-effort manual cleanup.

## Non-Goals

- redesign the broader project-first route tree from Phases 0 through 5
- add project duplication, branching, or merge behavior
- redesign the knowledge library information architecture beyond lifecycle
  actions
- introduce a full trash-bin or delayed delete recovery workflow
- add multi-user approval flows or team moderation for destructive actions
- solve long-running pipeline cancellation for arbitrary external child
  processes beyond the server tasks Planner directly owns
- migrate blueprint storage into fully isolated per-project directories in this
  phase

## Decision Summary

- Add two explicit project lifecycle actions:
  - `Archive`: soft-hide only, reversible.
  - `Delete`: true hard delete.
- Delete confirmation must explicitly warn the user that deleting a project will
  stop and remove its sessions.
- Shared blueprint or library records linked to multiple projects must be
  unlinked from the deleted project, not deleted outright.
- Project-local blueprint records owned by the project must be deleted.
- The backend, not the UI, owns deletion semantics and cascade rules.
- Delete must remove durable session records, per-session event files, and CXDB
  project-run data rather than only deleting the top-level project row.
- Live Socratic runtimes can already be stopped. Active pipeline jobs require a
  new cancellable task registry before the delete contract can honestly claim
  that work is stopped.
- Project archive should use the existing `archived_at` field and be available
  before delete ships.

## Current-State Summary

The codebase already has a canonical `Project` entity and project-first pages,
but project lifecycle actions stop at create/read/update.

| Surface | Current behavior | Current gap |
| --- | --- | --- |
| Project API | `GET`, `POST`, `PATCH` only | no `DELETE`, no archive toggle, no lifecycle summary |
| Project storage | persisted `Project` record includes `archived_at` | no delete primitive and no archive behavior wired to listing |
| Projects UI | project cards expose `Open`, `Knowledge`, `Blueprint`, `Events` | no lifecycle actions or destructive confirmation |
| Session ownership | sessions already carry canonical `project_id` | delete cascade is not implemented at project scope |
| Session persistence | `SessionStore::delete()` removes session files | separate `EventStore` files are not deleted by project cascade today |
| Live interview runtime | runtime registry can remove and close a live Socratic session | pipeline jobs are spawned without stored cancellation handles |
| CXDB | project run indices can be registered and listed | no delete or compaction path for project-owned runs |
| Blueprint graph | node and edge deletion exists | project-wide purge and shared unlink behavior do not exist |
| Blueprint history | append-only `events.msgpack` and `history/` snapshots persist edits | true delete would still leave project traces on disk |

### Current code anchors

- `planner-server/src/api.rs`
- `planner-server/src/project.rs`
- `planner-server/src/session.rs`
- `planner-server/src/runtime.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-core/src/observability.rs`
- `planner-core/src/cxdb/durable.rs`
- `planner-core/src/blueprint.rs`
- `planner-schemas/src/artifacts/blueprint.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-server/tests/server_integration.rs`

### Code findings that make the current state unsafe to ship as-is

- `/projects/{projectRef}` only supports `GET` and `PATCH`; there is no delete
  route.
- `ProjectStore` persists and updates projects, but does not expose a delete
  operation.
- `SessionStore::delete()` removes a session record and its session file, but
  project-scoped delete would also need to remove `EventStore` files.
- `stop_session_runtime()` can terminate live interview runtimes, but pipeline
  execution is currently launched by detached `tokio::spawn(...)` calls whose
  handles are not retained.
- `DurableCxdbEngine` can register and list project runs, but has no project
  delete path.
- `BlueprintStore` can delete individual nodes and edges, but not a full
  project-local purge plus shared unlink pass.
- Blueprint event and history persistence is append-only and currently global,
  which means a true delete contract requires explicit pruning or compaction.

## Proposed Behavior

### Archive behavior

Archive is a reversible, non-destructive lifecycle state.

When a project is archived:

- `archived_at` is set to the current timestamp.
- the project is hidden from the default `/projects` list and default project
  chooser surfaces
- archived projects remain readable and restorable
- sessions, blueprint data, CXDB data, and event history remain intact
- archive does not stop active sessions automatically in the first cut
- archive should be reversible with an explicit `Unarchive` action

Recommended default list behavior:

- `/projects` excludes archived projects unless `include_archived=true`
- project-first chooser surfaces should default to active-only
- project detail routes should still resolve archived projects directly by slug
  or UUID

### Delete behavior

Delete is a true destructive operation.

When a project is deleted:

- the server resolves the canonical project record and verifies ownership
- the server stops live Socratic interview runtimes for sessions under that
  project
- the server stops active pipeline tasks for sessions under that project
- the server deletes owned sessions
- the server deletes per-session event files from `EventStore`
- the server deletes CXDB project-run index data and project-owned run
  metadata directories
- the server deletes project-local blueprint nodes and their incident edges
- the server removes the deleted project from `linked_project_ids` on shared
  blueprint records instead of deleting those shared records
- the server prunes blueprint event and history persistence so deleted project
  data is not retained in durable blueprint audit files
- the server deletes the canonical project record itself

### Confirmation behavior

The delete action must require explicit confirmation in the UI.

The confirmation should tell the user:

- the project name being deleted
- that deletion is permanent
- that active sessions will be stopped
- that sessions and associated project data will be removed
- that shared knowledge linked to other projects will be preserved and only
  unlinked from this project

Recommended confirmation copy shape:

> Delete "{Project Name}" permanently? This will stop any active sessions,
> remove this project's sessions and owned knowledge, and unlink shared records
> from this project. This action cannot be undone.

## API And Data Model Changes

### Project API

Add lifecycle endpoints under `/projects/{projectRef}`:

- `PATCH /projects/{projectRef}`
  - allow archive and unarchive through an explicit archive field or action
- `DELETE /projects/{projectRef}`
  - performs the true delete cascade
  - returns a deletion summary

Recommended response shape for delete:

```ts
interface DeleteProjectResponse {
  project_id: string;
  project_name: string;
  stopped_live_sessions: number;
  stopped_pipeline_sessions: number;
  deleted_sessions: number;
  deleted_session_event_files: number;
  deleted_cxdb_runs: number;
  deleted_blueprint_nodes: number;
  unlinked_shared_blueprint_nodes: number;
  deleted_project_record: boolean;
}
```

### Project model

The existing project shape already includes `archived_at`. No new core project
fields are strictly required.

Recommended request shape extension:

```ts
interface UpdateProjectRequest {
  name?: string;
  slug?: string;
  description?: string;
  team_label?: string;
  legacy_scope_keys?: string[];
  archived?: boolean;
}
```

### Pipeline runtime state

Add a cancellable runtime registry for active pipeline jobs, parallel to the
existing Socratic runtime registry.

Recommended server-owned state:

- `SessionPipelineRegistry`
- keyed by `session_id`
- stores a task handle and cancellation signal
- exposes `insert`, `remove`, and `stop` operations

This is required so delete can honestly stop active work instead of only
removing records after detached tasks continue running.

## UI And Routing Changes

### Projects page

Add lifecycle actions to each project card in `/projects`:

- `Archive` when active
- `Unarchive` when archived and visible via archive filter
- `Delete` as a danger action

Recommended card action layout:

- keep navigation actions grouped separately from lifecycle actions
- show `Delete` with explicit danger styling
- show loading and disabled states while archive or delete is in flight

### Project list filtering

Add a lightweight archive filter:

- default: active projects only
- optional toggle: `Show archived`

### Post-action navigation

Recommended behavior:

- after archive: remain on `/projects` and refresh list
- after delete from `/projects`: remain on `/projects` and refresh list
- if delete is later triggered from a project detail page: redirect to
  `/projects`

## Migration And Backfill Plan

No destructive data migration is required before implementation.

### Archive rollout

- existing projects already deserialize `archived_at`
- existing clients simply ignore archived state today
- adding archive support is backward-compatible as long as default list
  filtering is explicit and test-covered

### Delete rollout

Delete requires new purge primitives but does not require a one-time backfill.

The only migration-sensitive area is blueprint durability:

- current global blueprint event and history files may already contain project
  references
- once true delete ships, the implementation must either:
  - prune deleted project content from those files, or
  - replace them with a compacted snapshot and filtered event log

## Phased Implementation Plan

## Phase 6A Lifecycle Contract And Archive API

### Objective

Define the lifecycle contract in the API and make archive/unarchive functional
before destructive delete work begins.

### Impacted files

- `planner-server/src/api.rs`
- `planner-server/src/project.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`

### Changes

- extend `UpdateProjectRequest` to accept `archived`
- update project list behavior to optionally exclude archived projects by
  default
- add client methods or patch usage to archive and unarchive a project
- add UI affordances for `Archive`, `Unarchive`, and `Show archived`
- ensure archived projects remain resolvable by direct route lookup

### File-by-file checklist

- `planner-server/src/api.rs`
  - add `archived` to `UpdateProjectRequest`
  - wire archive/unarchive behavior into `update_project()`
  - add optional `include_archived` query behavior to `list_projects()`
- `planner-server/src/project.rs`
  - add helper(s) for archive and unarchive mutations
- `planner-web/src/types.ts`
  - confirm `Project` includes `archived_at`
  - add any missing request types for lifecycle actions
- `planner-web/src/api/client.ts`
  - add `includeArchived` support to `listProjects()`
  - add helper for project lifecycle patching if needed
- `planner-web/src/pages/ProjectsPage.tsx`
  - add archive filter UI
  - add archive and unarchive actions per card
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
  - cover archive filter and archive action success path

### Done when

- projects can be archived and restored without deleting data
- default project list hides archived records
- the UI exposes archive state safely

### Detailed execution checklist

- `docs/phase-06a-project-archive-execution-checklist.md`

## Phase 6B Pipeline Stop And Runtime Ownership

### Objective

Add explicit ownership and shutdown control for live work so delete can stop
sessions honestly.

### Impacted files

- `planner-server/src/lib.rs`
- `planner-server/src/runtime.rs`
- `planner-server/src/api.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/main.rs`

### Changes

- introduce a new pipeline runtime registry keyed by `session_id`
- store task handles and cancellation state for active pipeline jobs
- update every pipeline spawn site to register and deregister the running task
- expose an internal helper to stop pipeline work for a session
- keep `stop_session_runtime()` for live interview shutdown, but pair it with
  pipeline task cancellation for delete paths

### File-by-file checklist

- `planner-server/src/runtime.rs`
  - add a `SessionPipelineRegistry` abstraction or sibling runtime type
  - implement insert, remove, stop, and cleanup helpers
- `planner-server/src/lib.rs`
  - add the pipeline registry to `AppState`
- `planner-server/src/main.rs`
  - initialize the new registry in server startup
- `planner-server/src/api.rs`
  - wrap `tokio::spawn(...)` pipeline entry points with registry bookkeeping
  - add a helper to stop active pipeline work for one session
- `planner-server/src/ws_socratic.rs`
  - register pipeline tasks spawned after interview completion
- `planner-server/src/ws.rs`
  - register pipeline tasks spawned from websocket-driven pipeline starts

### Done when

- every active pipeline run is discoverable by session ID
- delete can request a stop for both interview and pipeline work
- task cleanup occurs on normal completion and on forced stop

### Detailed execution checklist

- `docs/phase-06b-pipeline-stop-runtime-execution-checklist.md`

## Phase 6C Project Delete Cascade In Core Server Stores

### Objective

Add the actual delete cascade across projects, sessions, and per-session event
files.

### Impacted files

- `planner-server/src/project.rs`
- `planner-server/src/session.rs`
- `planner-core/src/observability.rs`
- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

### Changes

- add `ProjectStore::delete(project_id)`
- enumerate sessions for a project and delete them
- delete each session's persisted event file through `EventStore`
- add `DELETE /projects/{projectRef}`
- return a delete summary payload with counts and stop stats

### File-by-file checklist

- `planner-server/src/project.rs`
  - add delete primitive that removes the project from memory, dirty state, and
    disk
- `planner-server/src/session.rs`
  - add any helper needed to enumerate or bulk-delete project sessions cleanly
- `planner-core/src/observability.rs`
  - reuse `delete_session_events()` from the project cascade
  - add any bulk helper only if it reduces duplication materially
- `planner-server/src/api.rs`
  - add `DELETE /projects/{projectRef}` route
  - resolve ownership, stop live work, delete sessions, delete session events,
    and delete the project record
  - return deletion summary counts
- `planner-server/tests/server_integration.rs`
  - add integration coverage for project delete removing project, sessions, and
    session event files

### Done when

- a project can be deleted via API
- owned sessions are removed
- persisted session event files are removed
- delete responses report real counts

### Detailed execution checklist

- `docs/phase-06c-project-delete-cascade-execution-checklist.md`

## Phase 6D CXDB Purge And Blueprint Purge/Unlink

### Objective

Remove durable project-owned planning artifacts and correctly preserve shared
knowledge by unlinking it.

### Impacted files

- `planner-core/src/cxdb/durable.rs`
- `planner-core/src/blueprint.rs`
- `planner-schemas/src/artifacts/blueprint.rs`
- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

### Changes

- add `DurableCxdbEngine::delete_project(project_id)`
- delete the project's run index file and run metadata directories
- add `BlueprintStore` helpers to:
  - delete project-local nodes
  - unlink shared nodes by removing the deleted project from
    `linked_project_ids`
  - prune or compact blueprint events/history so deleted project data is not
    retained in durable blueprint files

### File-by-file checklist

- `planner-core/src/cxdb/durable.rs`
  - add delete helper for project run index and owned run directories
  - decide whether orphaned blobs are tolerated or whether blob GC is deferred
- `planner-core/src/blueprint.rs`
  - add project purge helper that walks current node summaries
  - delete owned nodes where `project_id == deleted_project_id` and
    `scope_visibility == project_local`
  - unlink shared nodes where `linked_project_ids` contains the deleted project
  - add event log and history compaction/pruning logic for true delete
- `planner-schemas/src/artifacts/blueprint.rs`
  - add helper predicates only if they make ownership and shared-unlink logic
    less error-prone
- `planner-server/src/api.rs`
  - call CXDB and blueprint purge helpers from project delete
  - include blueprint and CXDB counts in the delete summary
- `planner-server/tests/server_integration.rs`
  - cover local node deletion, shared unlink preservation, and CXDB purge

### Done when

- project-local blueprint records are gone after delete
- shared blueprint records survive with the deleted project link removed
- CXDB project-run data is removed
- blueprint durable history no longer retains deleted project content

### Detailed execution checklist

- `docs/phase-06d-cxdb-blueprint-delete-execution-checklist.md`

## Phase 6E UI Delete Flow And Confirmation UX

### Objective

Expose archive and delete in the projects UI with explicit destructive
confirmation and clear result handling.

### Impacted files

- `planner-web/src/api/client.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- optionally `planner-web/src/components/Layout.tsx` if archive visibility needs
  shared nav treatment later

### Changes

- add client methods for delete and lifecycle updates
- add project card danger action for delete
- add confirmation prompt copy that explicitly says delete will stop and remove
  sessions
- disable actions while the request is in flight
- refresh the list after success and surface failures inline

### File-by-file checklist

- `planner-web/src/api/client.ts`
  - add `deleteProject(projectRef)`
  - add typed response for delete summary
- `planner-web/src/pages/ProjectsPage.tsx`
  - add `Delete` action per card
  - show confirmation prompt with the agreed warning text
  - refresh project list after delete
  - keep the page stable if delete fails
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
  - cover confirmation accepted and cancelled paths
  - cover delete success refresh behavior
  - cover delete failure rendering

### Done when

- the UI exposes delete safely
- users are warned that delete stops and removes sessions
- destructive flows are test-covered

### Detailed execution checklist

- `docs/phase-06e-project-delete-ui-execution-checklist.md`

## Phase 6F Hardening, Edge Cases, And Rollout

### Objective

Finish the lifecycle work with integration coverage, edge-case handling, and a
safe rollout order.

### Impacted files

- `planner-server/tests/server_integration.rs`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- any focused blueprint or CXDB test modules touched by new delete helpers
- `docs/phase-06-project-archive-delete-implementation.md`

### Changes

- test forbidden and not-found delete cases
- test deleting a project with active interview runtime
- test deleting a project with active pipeline work
- test deleting a project with no sessions and no blueprint data
- test archived project visibility and restoration behavior
- verify delete summary counts for mixed local and shared blueprint data
- verify direct-route behavior for archived projects

### File-by-file checklist

- `planner-server/tests/server_integration.rs`
  - add delete and archive integration coverage across success and failure paths
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
  - add archived filter behavior and destructive UI coverage
- focused unit tests near new helpers
  - add targeted tests for blueprint unlink/purge logic
  - add targeted tests for CXDB project delete logic
  - add targeted tests for pipeline registry stop behavior

### Done when

- the lifecycle contract is fully test-covered
- edge cases do not rely on manual verification
- rollout order is explicit and implementable by one engineer without guessing

### Detailed execution checklist

- `docs/phase-06f-project-lifecycle-hardening-execution-checklist.md`
