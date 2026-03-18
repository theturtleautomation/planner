# Phase 6B Pipeline Stop And Runtime Ownership Execution Checklist

**Status:** Implemented  
**Date:** 2026-03-08

## Objective

Add explicit ownership and shutdown control for live work so delete can stop
sessions honestly.

## Scope Guardrails

### In scope

- pipeline runtime registry design and implementation
- storing and removing per-session pipeline task ownership
- cancellation signaling for active pipeline work
- registering every pipeline spawn path
- tests for runtime registration and stop behavior

### Explicitly out of scope

- `DELETE /projects/{projectRef}`
- deleting sessions, projects, or event files
- CXDB purge
- blueprint local delete or shared unlink
- UI delete confirmation

## Success Criteria

- every active pipeline task is discoverable by `session_id`
- the server can stop active pipeline work for one session
- normal completion removes the task from the registry
- forced stop removes the task from the registry
- existing Socratic runtime shutdown remains intact
- later delete code can call one internal stop helper for both interview and
  pipeline work

## Current Code Anchors

- `planner-server/src/runtime.rs`
- `planner-server/src/lib.rs`
- `planner-server/src/api.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/main.rs`

## Test-First Execution Order

## Step 1: Add runtime registry tests first

### Goal

Lock the new pipeline-registry behavior before changing spawn sites.

### Files

- `planner-server/src/runtime.rs`

### Tests to add first

1. `pipeline_registry_insert_and_get`
2. `pipeline_registry_remove_returns_handle`
3. `pipeline_registry_stop_signals_shutdown`
4. `pipeline_registry_rejects_duplicate_session_registration`

### Assertions to include

- one active runtime can be registered for one session
- duplicate registration fails deterministically
- `stop(session_id)` closes the entry and signals shutdown
- `remove(session_id)` clears ownership cleanly

## Step 2: Add API-path tests for pipeline tracking

### Goal

Ensure the server entry points that start pipelines actually register them.

### Files

- `planner-server/src/api.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`

### Tests to add first

1. `send_message_registers_pipeline_runtime_when_pipeline_starts`
2. `retry_pipeline_registers_pipeline_runtime`
3. `ws_pipeline_start_registers_pipeline_runtime`
4. `socratic_pipeline_transition_registers_pipeline_runtime`

### Assertions to include

- registry entry exists after a pipeline start path transitions into running
- registry entry is removed on completion
- the same session does not double-register parallel pipeline tasks

## Step 3: Implement the pipeline runtime registry

### Goal

Add a server-owned registry parallel to the Socratic runtime registry.

### Files

- `planner-server/src/runtime.rs`
- `planner-server/src/lib.rs`
- `planner-server/src/main.rs`

### Task order

1. Add a `SessionPipelineRuntime` or similarly named runtime record.
2. Store a cancellation signal and join handle ownership.
3. Add `SessionPipelineRegistry`.
4. Add the registry to `AppState`.
5. Initialize it in server startup.

### Concrete checklist

- `planner-server/src/runtime.rs`
  - add runtime struct for pipeline tasks
  - add `insert`, `get`, `remove`, and `stop` helpers
  - define duplicate-registration behavior explicitly
- `planner-server/src/lib.rs`
  - add pipeline registry to `AppState`
- `planner-server/src/main.rs`
  - initialize the registry at server startup

## Step 4: Register all pipeline spawn sites

### Goal

Remove the current gap where pipeline work is launched by detached tasks with
no retained ownership.

### Files

- `planner-server/src/api.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`

### Task order

1. Wrap pipeline spawns in helper(s) that register before spawning.
2. Ensure completion paths deregister.
3. Ensure failure and cancellation paths deregister.

### Concrete checklist

- `planner-server/src/api.rs`
  - replace direct `tokio::spawn(...)` pipeline launches with registry-backed
    spawn helper(s)
  - add internal helper for stopping pipeline work by session ID
- `planner-server/src/ws.rs`
  - route websocket-triggered pipeline starts through the same helper
- `planner-server/src/ws_socratic.rs`
  - route post-interview pipeline starts through the same helper

### Guardrail

- keep the existing interview-runtime stop path unchanged except where the new
  unified stop helper composes both systems

## Step 5: Add unified session-stop helper

### Goal

Prepare a single internal surface for later delete cascade code.

### Files

- `planner-server/src/api.rs`

### Checklist

- add helper like `stop_active_session_work(state, session_id)`
- ensure it:
  - stops live Socratic interview runtime if present
  - stops pipeline runtime if present
- keep it internal in this phase

## Step 6: Regression pass

### Verification checklist

1. Run focused runtime tests.
2. Run focused API/websocket tests covering pipeline start paths.
3. Verify no delete endpoint has been introduced yet.
4. Verify no project lifecycle UI work was mixed into this phase.

## Recommended Command Order

```bash
# 1. Add failing runtime tests
cargo test -p planner-server pipeline_registry -- --nocapture

# 2. Add failing API/runtime integration tests
cargo test -p planner-server send_message_registers_pipeline_runtime -- --nocapture

# 3. Implement runtime registry and spawn wrappers

# 4. Run focused regression checks
cargo test -p planner-server pipeline_runtime -- --nocapture
```

## Exit Criteria For Phase 6B

- pipeline runtime ownership is explicit
- the server can stop active pipeline work by session
- all known pipeline spawn paths are registry-backed
- no delete cascade work has started yet
