# Server Security & Pipeline Integrity Fixes

**Date:** 2026-02-28  
**Branch:** server-security-pipeline-integrity  
**Tests:** 347 passed, 0 failed  
**`cargo check`:** Clean (no warnings promoted to errors)

---

## Summary

This change set addresses 11 numbered fixes spanning `planner-server` and `planner-core`, covering:
- Authentication fail-closed behaviour and JWT secret warnings
- Session store poisoning safety via `parking_lot::RwLock`
- Session expiry / cleanup background task
- WebSocket deduplication of stage-update messages
- Safe WebSocket pipeline dispatch (no duplicate spawns)
- Versioned API route (`/api/v1`)
- Robust JSON repair utility used across all LLM parse sites
- `cargo check` / `tsc --noEmit` post-generation compilation check in the factory worker
- OpenAI CLI stdin fix (prompt was being passed as a positional arg instead of stdin)
- Persist calls added throughout the pipeline so artefacts survive crashes
- AR refinement lint failures now produce structured `ArFinding` objects instead of silently clearing findings

A pre-existing compilation bug in `planner-core/src/cxdb/durable.rs` (`TurnMetadata` missing `project_id` field) was also fixed as it blocked `cargo check`.

---

## Fix 1 — Auth: Fail-Closed JWT Validation

**File:** `planner-server/src/auth.rs`

**Problem:** When no JWT secret was configured, `validate_token()` called
`insecure_disable_signature_validation()` and returned `Ok(claims)`, silently
accepting *any* token regardless of signature.

**Fix:**
- Replaced the `None` branch with an explicit `Err(AuthError::MissingSecret)`.
- Added a startup `CRITICAL` log warning in `JwtConfig::from_env()` when a
  domain is set but `JWT_SECRET` is absent.
- Added test `test_validate_token_fails_without_secret` confirming the error
  path is exercised.

---

## Fix 2 — Session Store: Replace `std::sync::RwLock` with `parking_lot::RwLock`

**Files:** `planner-server/src/session.rs`, `planner-server/Cargo.toml`

**Problem:** `std::sync::RwLock` poisons on a writer-thread panic, causing all
subsequent `.read()` / `.write()` calls to return `Err(PoisonError)`. The
existing code called `.unwrap()` on every lock operation, meaning a single
panic would permanently wedge the session store.

**Fix:**
- Added `parking_lot = "0.12"` to `planner-server/Cargo.toml`.
- Changed `use std::sync::RwLock` → `use parking_lot::RwLock`.
- Removed all `.unwrap()` calls on lock guards (`parking_lot` guards are
  infallible — no `Result` wrapper).
- Added `last_accessed: String` field to `Session`; `get()` and `update()`
  both refresh it on every call.
- Added `SessionStore::cleanup_expired(max_age_secs: u64)` method that
  removes sessions whose `last_accessed` timestamp is older than the given
  threshold.
- Made `sessions` field `pub(crate)` to allow the cleanup test to inspect state.
- Added test `cleanup_expired_removes_old_sessions`.

---

## Fix 3 — Session Expiry: Background Cleanup Task

**File:** `planner-server/src/main.rs`

**Problem:** Sessions accumulated indefinitely; there was no mechanism to
evict stale entries.

**Fix:**
- Added a `tokio::spawn` background task in `main()` that calls
  `state.sessions.cleanup_expired(3600)` (1-hour TTL) every 5 minutes via
  `tokio::time::interval`.

---

## Fix 4 — WebSocket: Deduplicate Stage-Update Messages

**File:** `planner-server/src/ws.rs`

**Problem:** Stage-update messages were emitted every time the pipeline loop
ran, flooding the client with identical status frames even when nothing had
changed.

**Fix:**
- Added `last_sent_stages: Vec<(String, String)>` field to the WS handler
  state to track the last-sent `(stage_name, status)` pairs.
- Stage-update messages are now only sent when the status of at least one
  stage has changed since the previous send.

---

## Fix 5 — WebSocket: Safe Pipeline Dispatch (No Duplicate Spawns)

**File:** `planner-server/src/ws.rs`

**Problem:** The `StartPipeline` handler could spawn a second pipeline task
if the client sent the message while a pipeline was already running.

**Fix:**
- On `StartPipeline`, the handler checks a `pipeline_running` boolean
  *before* spawning; if already `true` the message is ignored.
- Sets `pipeline_running = true`, stores `project_description`, advances the
  first stage to `"running"`, then spawns `crate::api::run_pipeline_for_session`.
- `run_pipeline_for_session` is now `pub async fn` in `api.rs` so `ws.rs`
  can call it directly.

---

## Fix 6 — API: Safe Slice Indexing for Message History

**File:** `planner-server/src/api.rs`

**Problem:** The pipeline message-history logic used `msgs[msgs.len()-2]` and
`msgs[msgs.len()-1]` which panic on short slices (fewer than 2 messages).

**Fix:**
- Replaced with `msgs.last()` and `msgs.iter().rev().nth(1)` — both return
  `Option` and are handled gracefully with early returns / defaults.
- Replaced a fragile content-equality duplicate-spawn check with a
  `was_running` boolean captured *before* the state mutation.

---

## Fix 7 — API: Versioned Route Prefix `/api/v1`

**File:** `planner-server/src/main.rs`

**Problem:** All API routes were registered under `/api` only. No `/api/v1`
prefix existed, making future versioning impossible without breaking existing
clients.

**Fix:**
- Added `.nest("/api/v1", api::routes(state.clone()))` *before* the existing
  `/api` nest so both prefixes are served simultaneously during the migration
  window.

---

## Fix 8 — JSON Repair Utility

**Files:**  
- `planner-core/src/llm/json_repair.rs` *(new)*  
- `planner-core/src/llm/mod.rs`  
- `planner-core/src/pipeline/steps/intake.rs`  
- `planner-core/src/pipeline/steps/ar.rs`  
- `planner-core/src/pipeline/steps/ar_refinement.rs`  
- `planner-core/src/pipeline/steps/compile.rs`  
- `planner-core/src/pipeline/steps/validate.rs`

**Problem:** LLM responses frequently wrap JSON in markdown code fences or
include leading/trailing prose. All parse sites used ad-hoc stripping that
failed on variations not anticipated at write-time.

**Fix:**
- Created `planner-core/src/llm/json_repair.rs` exporting:
  - `try_repair_json(raw: &str) -> Option<String>` — applies 4 strategies in
    order: (1) strip markdown fences then parse, (2) parse as-is, (3) find
    `{}`/`[]` boundaries via character scan, (4) strip common LLM preamble
    patterns. Returns `Some(valid_json)` on first success, `None` if all
    strategies fail.
  - `strip_code_fences(raw: &str) -> &str` — public helper for callers that
    only need fence stripping.
- Registered `pub mod json_repair` in `planner-core/src/llm/mod.rs`.
- Integrated `try_repair_json` as the primary parse path in: `intake.rs`,
  `ar.rs`, `ar_refinement.rs`, all 5 parse functions in `compile.rs`, and
  `evaluate_scenario_once` in `validate.rs`.
- Added 9 unit tests covering all repair strategies and edge cases.

---

## Fix 9 — Factory Worker: Post-Generation Compilation Check

**File:** `planner-core/src/pipeline/steps/factory_worker.rs`

**Problem:** The factory worker generated code but never verified it compiled,
meaning broken output could be persisted and passed downstream without any
diagnostic information.

**Fix:**
- Added `run_compilation_check(worktree: &Path, timeout: Duration) -> (bool, Option<String>)`:
  - Detects project type: `Cargo.toml` → runs `cargo check --manifest-path …`;
    `package.json` → runs `npx tsc --noEmit`; otherwise logs a warning and
    returns success (unknown project type is not a hard failure).
  - Handles `NotFound` OS errors (binary not installed) gracefully.
- Called after `scan_worktree_files` in `CodexFactoryWorker::generate`.
- The result sets `success` and `error` fields on `WorkerResult` so callers
  can react appropriately.

---

## Fix 10 — OpenAI CLI Client: Prompt Passed via Stdin, Not Positional Arg

**File:** `planner-core/src/llm/providers.rs`

**Problem:** `OpenAiCliClient::complete` appended `&prompt` to the CLI
argument vector. This caused the prompt to be treated as a positional argument
(filename or subcommand) rather than being streamed via stdin, producing
errors or silently empty responses for any prompt containing spaces or special
characters.

**Fix:**
- Removed `&prompt` from the `args` vec.
- Changed the `run_cli` call from `None` (no stdin) to `Some(&prompt)` so
  the prompt is written to the process's stdin as expected by the CLI tool.

---

## Fix 11 — Pipeline Persist Calls

**File:** `planner-core/src/pipeline/mod.rs`

**Problem:** Intermediate pipeline artefacts (NL specs, AR reports, graph
definitions, scenario sets, agent manifests, factory outputs, satisfaction
results, git commits, run budgets) were computed in memory but never persisted
to the durable store. A crash at any point meant all prior work was lost and
the pipeline had to restart from scratch.

**Fix:** Added `config.persist()` calls immediately after each artefact is
produced:

- `run_phase0_front_office_with_config`:
  - After each `NlSpecV1` compile attempt
  - After each `ArReportV1` generation
  - After `GraphDotV1` generation
  - After `ScenarioSetV1` generation
  - After `AgentsManifestV1` generation

- `run_full_pipeline`:
  - After each `FactoryOutputV1` attempt (per-worker)
  - After `SatisfactionResultV1` (both pass and fail paths)
  - After `GitCommitV1`
  - After `RunBudgetV1`

A `run_id` (`Uuid::new_v4()`) is now extracted once at the top of
`run_phase0_front_office_with_config` and threaded through all persist calls
within that invocation.

---

## Pre-existing Bug Fixed (Compilation Blocker)

**File:** `planner-core/src/cxdb/durable.rs`

**Problem:** `TurnMetadata { … }` struct initializer was missing the
`project_id: None` field added in a prior refactor. This caused a compilation
error that blocked `cargo check` for the entire workspace.

**Fix:** Added `project_id: None` to the `TurnMetadata` struct literal.

---

## Test Results

```
running 245 tests in planner-core (unit)   ... ok
running  45 tests in planner-core (integration) ... ok
running   4 tests in planner-schemas       ... ok
running  33 tests in planner-server        ... ok
running  20 tests in planner-tui           ... ok
─────────────────────────────────────────
total: 347 tests, 0 failures
```

### New Tests Added

| Test | File |
|---|---|
| `test_validate_token_fails_without_secret` | `planner-server/src/auth.rs` |
| `cleanup_expired_removes_old_sessions` | `planner-server/src/session.rs` |
| `test_try_repair_json_strips_fences` | `planner-core/src/llm/json_repair.rs` |
| `test_try_repair_json_valid_passthrough` | `planner-core/src/llm/json_repair.rs` |
| `test_try_repair_json_finds_object_boundary` | `planner-core/src/llm/json_repair.rs` |
| `test_try_repair_json_strips_preamble` | `planner-core/src/llm/json_repair.rs` |
| `test_try_repair_json_returns_none_on_garbage` | `planner-core/src/llm/json_repair.rs` |
| `test_strip_code_fences_backtick` | `planner-core/src/llm/json_repair.rs` |
| `test_strip_code_fences_tilde` | `planner-core/src/llm/json_repair.rs` |
| `test_strip_code_fences_no_fence` | `planner-core/src/llm/json_repair.rs` |
| `test_strip_code_fences_language_tag` | `planner-core/src/llm/json_repair.rs` |

---

## Files Changed

### planner-server
| File | Change Type |
|---|---|
| `planner-server/Cargo.toml` | Modified — added `parking_lot = "0.12"` |
| `planner-server/src/auth.rs` | Modified — fail-closed JWT, startup warning, new test |
| `planner-server/src/session.rs` | Modified — parking_lot RwLock, last_accessed, cleanup_expired, new test |
| `planner-server/src/ws.rs` | Modified — stage dedup, safe pipeline dispatch |
| `planner-server/src/api.rs` | Modified — safe slice indexing, was_running check, pub async fn |
| `planner-server/src/main.rs` | Modified — /api/v1 route, cleanup background task |

### planner-core
| File | Change Type |
|---|---|
| `planner-core/src/llm/json_repair.rs` | **New** — try_repair_json utility + 9 tests |
| `planner-core/src/llm/mod.rs` | Modified — added `pub mod json_repair` |
| `planner-core/src/llm/providers.rs` | Modified — OpenAI CLI stdin fix |
| `planner-core/src/pipeline/mod.rs` | Modified — persist calls, run_id extraction |
| `planner-core/src/pipeline/steps/intake.rs` | Modified — try_repair_json integration |
| `planner-core/src/pipeline/steps/ar.rs` | Modified — try_repair_json integration |
| `planner-core/src/pipeline/steps/ar_refinement.rs` | Modified — owned ArFinding, lint→findings, try_repair_json |
| `planner-core/src/pipeline/steps/compile.rs` | Modified — try_repair_json in all 5 parse fns |
| `planner-core/src/pipeline/steps/validate.rs` | Modified — read_factory_files helper, source_files context, try_repair_json |
| `planner-core/src/pipeline/steps/factory_worker.rs` | Modified — run_compilation_check post-generation |
| `planner-core/src/cxdb/durable.rs` | Modified — pre-existing TurnMetadata project_id bug fix |
