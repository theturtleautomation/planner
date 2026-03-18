# Phase 6D CXDB And Blueprint Delete Execution Checklist

**Status:** Implemented  
**Date:** 2026-03-08

## Objective

Remove durable project-owned planning artifacts and correctly preserve shared
knowledge by unlinking it.

## Scope Guardrails

### In scope

- CXDB project-run deletion
- project-local blueprint node deletion
- shared blueprint unlink behavior
- blueprint delete summary counts
- blueprint event/history pruning or compaction for true delete

### Explicitly out of scope

- project list archive behavior
- UI delete confirmation
- final rollout signoff and regression matrix beyond focused new tests

## Success Criteria

- CXDB project-run data is removed for deleted projects
- project-local blueprint nodes are removed
- shared blueprint nodes remain but lose the deleted project link
- delete summary reports CXDB and blueprint counts accurately
- blueprint durable files no longer retain deleted project content

## Current Code Anchors

- `planner-core/src/cxdb/durable.rs`
- `planner-core/src/blueprint.rs`
- `planner-schemas/src/artifacts/blueprint.rs`
- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

## Test-First Execution Order

## Step 1: Add CXDB delete tests first

### Files

- `planner-core/src/cxdb/durable.rs`

### Tests to add first

1. `delete_project_removes_project_run_index`
2. `delete_project_removes_run_metadata_directories`
3. `delete_project_preserves_other_project_runs`

## Step 2: Add blueprint purge/unlink tests first

### Files

- `planner-core/src/blueprint.rs`
- optionally `planner-schemas/src/artifacts/blueprint.rs`

### Tests to add first

1. `purge_project_deletes_project_local_nodes`
2. `purge_project_unlinks_shared_nodes`
3. `purge_project_preserves_other_project_links`
4. `purge_project_compacts_event_log_for_deleted_project`

### Assertions to include

- local nodes owned by the deleted project are gone
- shared nodes remain
- `linked_project_ids` no longer contains the deleted project
- unrelated shared/local nodes remain untouched
- durable blueprint event/history output no longer contains deleted project data

## Step 3: Implement CXDB project deletion

### Files

- `planner-core/src/cxdb/durable.rs`

### Task order

1. Add `delete_project(project_id)`.
2. Remove the project run-index file.
3. Remove run metadata directories for each associated run.
4. Decide whether blob garbage collection is deferred.

### Guardrail

- if blob GC is deferred, document that explicitly in code comments or doc
  notes; do not silently imply full blob-level cleanup

## Step 4: Implement blueprint local-delete and shared-unlink helpers

### Files

- `planner-core/src/blueprint.rs`
- `planner-schemas/src/artifacts/blueprint.rs`

### Task order

1. Add helper(s) to classify node ownership for one project.
2. Delete project-local nodes.
3. Unlink shared nodes.
4. Recompute affected edges and summaries.
5. Prune or compact event/history persistence.

### Concrete checklist

- `planner-core/src/blueprint.rs`
  - walk current node summaries by project
  - delete local nodes by project ownership
  - mutate shared nodes to remove the deleted project link
  - rewrite durable event/history files so true delete is honored
- `planner-schemas/src/artifacts/blueprint.rs`
  - add helper predicates only if they reduce repeated ownership logic

## Step 5: Wire CXDB and blueprint purge into project delete

### Files

- `planner-server/src/api.rs`
- `planner-server/tests/server_integration.rs`

### Task order

1. Call CXDB delete from the project delete route.
2. Call blueprint purge/unlink from the project delete route.
3. Extend delete summary counts.
4. Add integration tests for mixed local/shared blueprint data.

## Step 6: Regression pass

### Verification checklist

1. Run focused CXDB delete tests.
2. Run focused blueprint purge/unlink tests.
3. Run project delete integration tests.
4. Verify delete still preserves shared records correctly.

## Recommended Command Order

```bash
# 1. Add failing CXDB tests
cargo test -p planner-core delete_project_removes_project_run_index -- --nocapture

# 2. Add failing blueprint purge tests
cargo test -p planner-core purge_project_deletes_project_local_nodes -- --nocapture

# 3. Implement CXDB and blueprint helpers

# 4. Wire into planner-server delete flow

# 5. Run focused regression checks
cargo test -p planner-core purge_project -- --nocapture
cargo test -p planner-server delete_project -- --nocapture
```

## Exit Criteria For Phase 6D

- CXDB project-run data is removed
- local blueprint nodes are removed
- shared blueprint records are unlinked and preserved
- durable blueprint history honors true delete
