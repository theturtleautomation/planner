# Phase 6C Project Delete Cascade Execution Checklist

**Status:** Implemented  
**Date:** 2026-03-08

## Objective

Add the actual delete cascade across projects, sessions, and per-session event
files.

## Scope Guardrails

### In scope

- `DELETE /projects/{projectRef}`
- project-store delete primitive
- enumerating project-owned sessions
- stopping active work through the new runtime helper
- deleting sessions
- deleting per-session persisted event files
- delete summary response

### Explicitly out of scope

- CXDB project-run deletion
- blueprint project-local delete or shared unlink
- delete UI
- blueprint event/history compaction

## Success Criteria

- project delete API exists
- delete stops active session work first
- owned sessions are removed
- per-session event files are removed
- project record is removed
- response includes real cascade counts

## Current Code Anchors

- `planner-server/src/project.rs`
- `planner-server/src/session.rs`
- `planner-core/src/observability.rs`
- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

## Test-First Execution Order

## Step 1: Add delete-route integration tests first

### Files

- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

### Tests to add first

1. `test_delete_project_removes_project_record`
2. `test_delete_project_removes_owned_sessions`
3. `test_delete_project_removes_session_event_files`
4. `test_delete_project_stops_active_session_work`
5. `test_delete_project_forbidden_for_non_owner`
6. `test_delete_project_not_found`

### Assertions to include

- project no longer resolves after delete
- sessions under the project are removed
- persisted session event files are removed
- delete summary reports stop and deletion counts
- ownership and not-found rules match existing project API behavior

## Step 2: Add store-level delete tests

### Files

- `planner-server/src/project.rs`
- `planner-server/src/session.rs`
- `planner-core/src/observability.rs`

### Tests to add first

1. `project_store_delete_removes_project`
2. `session_store_delete_project_session_set`
3. `event_store_delete_session_events_removes_file`

## Step 3: Implement store and cascade helpers

### Files

- `planner-server/src/project.rs`
- `planner-server/src/session.rs`
- `planner-core/src/observability.rs`

### Task order

1. Add `ProjectStore::delete(project_id)`.
2. Add any project-session enumeration or bulk-delete helper needed.
3. Reuse `EventStore::delete_session_events(session_id)` in the cascade path.

### Concrete checklist

- `planner-server/src/project.rs`
  - remove project from memory
  - remove from dirty set
  - remove on-disk msgpack file if persistent
- `planner-server/src/session.rs`
  - add project-scoped helper(s) only if they simplify the API handler
- `planner-core/src/observability.rs`
  - keep delete behavior session-based; avoid inventing a project-specific event
    store abstraction unless necessary

## Step 4: Implement `DELETE /projects/{projectRef}`

### Files

- `planner-server/src/api.rs`

### Task order

1. Add the route.
2. Resolve the project through the existing ownership helper.
3. Collect project-owned sessions.
4. Stop active work for each session.
5. Delete session event files.
6. Delete session records.
7. Delete the project record.
8. Return the summary payload.

### Guardrail

- do not add CXDB or blueprint deletion in this phase; leave placeholders only
  if needed for the next phase

## Step 5: Regression pass

### Verification checklist

1. Run delete-route server integration tests.
2. Verify archive behavior from `6A` still works unchanged.
3. Verify delete does not yet claim blueprint or CXDB counts it has not removed.

## Recommended Command Order

```bash
# 1. Add failing server integration tests
cargo test -p planner-server delete_project -- --nocapture

# 2. Add failing store-level tests
cargo test -p planner-server project_store_delete -- --nocapture

# 3. Implement cascade helpers and route

# 4. Run focused regression checks
cargo test -p planner-server delete_project -- --nocapture
```

## Exit Criteria For Phase 6C

- delete route exists and is test-covered
- project, sessions, and session event files are removed
- active work is stopped before record deletion
- CXDB and blueprint deletion are intentionally deferred to `6D`
