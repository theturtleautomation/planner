# Planner v2 — Stub & Dead Code Removal Plan

Comprehensive plan to remove all stubs, simulation code, dead code, and phantom
dependencies. Organized into phases that can each be committed independently
while keeping `cargo test --workspace` green at every step.

---

## Phase A: Remove `#![allow(dead_code)]` and Fix Warnings

**Goal:** Remove the blanket suppressions and either fix or surgically annotate only the items that genuinely need it.

| File | Current State | Action |
|---|---|---|
| `planner-core/src/lib.rs:8` | `#![allow(dead_code)]` | Remove. Run `cargo check` and fix every warning by either wiring, removing, or adding narrow `#[allow(dead_code)]` with a `// REASON:` comment. |
| `planner-core/src/main.rs:15` | `#![allow(dead_code)]` | Remove. Same approach. |
| `planner-server/src/main.rs:17` | `#![allow(dead_code)]` | Remove. |
| `planner-server/src/main.rs:32` | `#[allow(dead_code)]` on `AppState` | Remove — make `AppState` actually used or `pub`. |
| `planner-tui/src/main.rs:21` | `#![allow(dead_code)]` | Remove. |
| Several struct fields (`providers.rs:189`, `ar_refinement.rs:283`, `factory.rs:395`, `telemetry.rs:363`, `validate.rs:233,322`) | Field-level `#[allow(dead_code)]` | Evaluate each: either read the field somewhere or remove the field. If it's deserialized-but-not-read, use `_` prefix instead. |

**Estimated touches:** 10 files, ~50 warning fixes
**Risk:** Low — purely compiler-driven

---

## Phase B: Kill the Old Kilroy Simulation Path

**Goal:** The Phase 7 `FactoryWorker` trait + `CodexFactoryWorker` replaced the old Kilroy CLI path. Remove the dead Kilroy code entirely.

### What to remove from `factory.rs`:
1. `invoke_kilroy()` (line ~291) — the old Kilroy CLI invocation function
2. `run_kilroy_simulation()` (line ~354) — the entire simulation mode function
3. `execute_factory_handoff()` (line ~572) — the old orchestrator that calls `invoke_kilroy`
4. `poll_checkpoint()` — only used by the old path
5. `KilroyCheckpoint` struct — deserialization struct for old checkpoint.json
6. Test `kilroy_simulation_creates_checkpoint` — tests removed code

### What to remove from `pipeline/mod.rs`:
1. `run_phase0_full_with_config()` (line ~520) — the old orchestrator that calls `execute_factory_handoff`
2. `run_phase0_full()` (line ~652) — backward-compat wrapper around the old path
3. Update `run_phase0_full_with_worker()` to become the only full-pipeline entry point, renamed to `run_full_pipeline()`

### What to update in `main.rs`:
1. Switch from `pipeline::run_phase0_full()` → `pipeline::run_full_pipeline()` with a `CodexFactoryWorker`
2. This makes the CLI binary use the real worker path

### What to update in integration tests:
1. `e2e_phase0_pipeline_simulation` — rewrite to use `MockFactoryWorker` + `execute_factory_with_worker()` instead of `execute_factory_handoff()` simulation
2. Remove references to "simulation mode" in test comments

**Estimated touches:** 3 files (factory.rs, pipeline/mod.rs, main.rs) + 1 test file
**Risk:** Medium — need to verify all callers are rewired

---

## Phase C: Kill the Git Simulation Path

**Goal:** Remove `simulate_git_projection()` from `git.rs`. If git isn't available, that should be an error, not a silent fake.

### What to remove from `git.rs`:
1. `simulate_git_projection()` function (line ~206)
2. Test `simulated_projection_produces_valid_result`

### What to replace it with:
1. If `git` is not found on PATH, return `StepError::GitNotAvailable` (new error variant)
2. Add `GitNotAvailable` variant to `StepError` enum in `steps/mod.rs`

**Estimated touches:** 2 files (git.rs, steps/mod.rs)
**Risk:** Low

---

## Phase D: Remove Legacy SQLite Storage

**Goal:** The `storage/mod.rs` module is the "Phase 0 SQLite sidecar" that was meant to be replaced by CXDB. CXDB (`cxdb/` module with `DurableCxdbEngine`) is now built. Remove the SQLite layer.

### Analysis:
- `TurnStore` trait is defined in `storage/mod.rs` — it's used by `pipeline/mod.rs`, `cxdb/mod.rs`, `cxdb/durable.rs`
- `CxdbEngine` and `DurableCxdbEngine` both implement `TurnStore`
- `SqliteTurnStore` is the concrete SQLite impl in `storage/mod.rs`

### What to do:
1. **Move `TurnStore` trait + `StorageError` enum** out of `storage/mod.rs` into `cxdb/mod.rs` (or a new `cxdb/traits.rs`). This is the trait that all code depends on.
2. **Delete `storage/mod.rs`** and `pub mod storage` from `lib.rs` + `main.rs`
3. **Remove `rusqlite`** from `planner-core/Cargo.toml` workspace dependencies
4. **Update all `use crate::storage::*` → `use crate::cxdb::*`** imports across pipeline and tests

### What to keep:
- `TurnStore` trait (relocated to cxdb)
- `StorageError` enum (relocated, but remove `Sqlite` variant)
- Both `CxdbEngine` (in-memory) and `DurableCxdbEngine` (filesystem)

**Estimated touches:** ~8 files
**Risk:** Medium — many import paths change

---

## Phase E: Remove Phantom `reqwest` Dependency

**Goal:** `reqwest` is in `Cargo.toml` workspace deps but never imported or used in any `.rs` file. Remove it.

### What to do:
1. Remove `reqwest = { ... }` from workspace `Cargo.toml` `[workspace.dependencies]`
2. Remove from any crate-level `Cargo.toml` if present
3. Run `cargo build --workspace` to confirm nothing breaks

**Estimated touches:** 1-2 Cargo.toml files
**Risk:** None

---

## Phase F: Wire TUI to Real Pipeline

**Goal:** Replace the canned responses in `planner-tui/src/app.rs` with actual pipeline calls.

### Current state (canned):
- `submit_input()` (line ~264): On first message, prints a hardcoded "Starting Socratic planning" message. On subsequent messages, prints "Thank you for that clarification."
- `tick()` (line ~314): No-op comment "Future: check for async pipeline results here"

### What to build:
1. Add `LlmRouter` and `PipelineConfig` to `App` struct (or an `Arc<Mutex<PipelineHandle>>`)
2. On first `submit_input()`:
   - Spawn a tokio task running `pipeline::run_full_pipeline()` on a background thread
   - Pipeline emits progress events through an `mpsc::channel` → `App` reads in `tick()`
   - Stage status updates flow through the channel
3. On subsequent messages:
   - Queue them for the Socratic interview stage (intake step needs back-and-forth)
4. `tick()`:
   - Poll the `mpsc::Receiver` for pipeline progress events
   - Update `stages[]` status
   - Append planner messages to chat history
5. Define a `PipelineEvent` enum: `StageStarted(String)`, `StageCompleted(String)`, `Message(String)`, `PipelineComplete(Phase0FullOutput)`, `PipelineError(String)`

### Dependencies:
- Requires Phase B (single pipeline entry point)
- Requires the pipeline to support a "streaming" or "event callback" mode — currently it's a monolithic `async fn` that runs start-to-finish

### Intermediate approach (simpler):
- Keep synchronous pipeline execution
- Run entire pipeline in a background tokio task
- Post completion event back to TUI
- The Socratic interview becomes: collect description → run pipeline → show results
- Real interactive Socratic back-and-forth deferred to a later phase

**Estimated touches:** 3 files (app.rs, main.rs, possibly a new `pipeline_bridge.rs`)
**Risk:** Medium-High — async pipeline + TUI event loop coordination

---

## Phase G: Wire Server to Real Pipeline + WebSocket

**Goal:** Replace the canned responses in `planner-server/src/api.rs` and implement the WebSocket handler.

### Current state (canned):
- `send_message()` in `api.rs` (line ~164): Hardcoded "Starting Socratic planning" / "Thank you for that clarification"
- `ws.rs`: Only defines message types (`ServerMessage`, `ClientMessage`), no actual handler
- `api.rs:235`: Comment "WebSocket stub"

### What to build:
1. **REST `send_message()` → real pipeline**:
   - On first message, spawn pipeline as background task
   - Return planner's first Socratic response
   - Store pipeline task handle in `Session`
2. **WebSocket handler**:
   - Register `GET /api/sessions/:id/ws` route in `api.rs` using `axum::extract::ws::WebSocket`
   - On connect, subscribe to session's pipeline event stream
   - Forward `PipelineEvent` → `ServerMessage` over the socket
   - Accept `ClientMessage::UserMessage` and `ClientMessage::StartPipeline` from client
3. **Session store upgrade**:
   - Add `tokio::sync::broadcast::Sender<ServerMessage>` to `Session`
   - Pipeline events get broadcast to both REST polling and WebSocket subscribers
4. **Web frontend update**:
   - `planner-web/dist/index.html`: Connect to WebSocket, display real-time stage updates

### Dependencies:
- Requires Phase B (single pipeline entry point)
- Shares the same `PipelineEvent` design as Phase F
- `tokio-tungstenite` already in workspace deps (but Axum has built-in WS support, may not need it)

**Estimated touches:** 4-5 files (api.rs, ws.rs, session.rs, main.rs, index.html)
**Risk:** Medium-High

---

## Phase H: Clean Up `StepResult<T>` "Placeholder" Comment

**Goal:** `steps/mod.rs:36` says "Placeholder result type for step execution." It's not a placeholder — it's the real type. Fix the doc comment.

### What to do:
1. Change `/// Placeholder result type for step execution.` → `/// Result type for pipeline step execution.`

**Estimated touches:** 1 file, 1 line
**Risk:** None

---

## Phase I: Clean Up Verification Module Naming

**Goal:** The verification module is titled "Formal Verification Stubs" and generates Lean4 `sorry` proof stubs. This is architecturally intentional (generates propositions for humans to prove), but the naming implies it's unfinished code.

### What to do:
1. Rename module doc from "Formal Verification Stubs" → "Formal Verification — Lean4 Proposition Generation"
2. Rename `Lean4Proposition` doc from "A generated Lean4 proposition stub" → "A generated Lean4 proposition template"
3. Keep the `sorry -- proof stub` comments in the generated Lean4 code — that's correct Lean4 convention
4. Rename `generate_propositions` doc from "Generate Lean4 proposition stubs" → "Generate Lean4 proposition templates from an NLSpec"

**Estimated touches:** 1 file (verification.rs), ~6 doc comment changes
**Risk:** None

---

## Phase J: Remove `#[cfg(test)]` Dead Helper in Integration Tests

**Goal:** `integration_e2e.rs:1169` has `build_multi_chunk_intake()` marked `#[allow(dead_code)]` with comment "Available for future Phase 3+ integration tests". Either write the test or delete the helper.

### What to do:
1. If multi-chunk pipeline path is tested elsewhere → delete the helper
2. If not tested → write a multi-chunk e2e test using it, then remove the `#[allow(dead_code)]`

**Estimated touches:** 1 file
**Risk:** Low

---

## Execution Order

```
Phase H (1 min)  ─┐
Phase I (5 min)  ─┤
Phase E (2 min)  ─┤── Independent, do first (trivial)
Phase J (15 min) ─┘

Phase A (1-2 hr)  ── Do next (compiler-guided, reveals true dead code)

Phase C (30 min)  ─┐
Phase D (1-2 hr)  ─┤── Structural cleanup (no behavior change)
Phase B (2-3 hr)  ─┘

Phase F (3-4 hr)  ─┐── Real wiring (behavior change)
Phase G (3-4 hr)  ─┘
```

**Total estimated effort:** 12-16 hours of focused work

---

## Test Strategy

Every phase must:
1. `cargo test --workspace` — all 323+ tests pass
2. `cargo check --workspace` — 0 warnings (after Phase A)
3. `cargo build --workspace` — clean build

For Phases B/C/D, existing tests get migrated (not deleted) to use the new paths.
For Phases F/G, new tests are added for the real pipeline wiring.
