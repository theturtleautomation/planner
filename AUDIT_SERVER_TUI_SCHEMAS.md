# Audit: planner-server, planner-tui, planner-schemas

**Auditor:** automated code review  
**Date:** 2026-02-28  
**Files reviewed:** 19 schema files, 5 server files, 5 TUI files  

---

## Executive Summary

All three crates are **real implementations**, not scaffolding. The schemas are well-formed and comprehensive. The server is a working Axum HTTP/WebSocket server with genuine auth and session isolation logic. The TUI is a functional Ratatui application with real key handling, pipeline plumbing, and meaningful tests. However, there are **several serious security and correctness issues** detailed below.

---

## 1. planner-schemas

### 1.1 Type Registry Coverage

The `lib.rs` docstring claims 18 artifact types. The source tree contains **16 concrete `ArtifactPayload` implementations** in artifact modules, plus the 3 runtime types in `runtime.rs`. Count reconciliation:

| Module | Type | `ArtifactPayload` impl | TYPE_ID |
|---|---|---|---|
| `intake.rs` | `IntakeV1` | ✅ | `planner.intake.v1` |
| `nlspec.rs` | `NLSpecV1` | ✅ | `planner.nlspec.v1` |
| `graph_dot.rs` | `GraphDotV1` | ✅ | `planner.graph_dot.v1` |
| `scenario_set.rs` | `ScenarioSetV1` | ✅ | `planner.scenario_set.v1` |
| `factory_output.rs` | `FactoryOutputV1` | ✅ | `planner.factory_output.v1` |
| `satisfaction_result.rs` | `SatisfactionResultV1` | ✅ | `planner.satisfaction_result.v1` |
| `run_budget.rs` | `RunBudgetV1` | ✅ | `planner.run_budget.v1` |
| `agents_manifest.rs` | `AgentsManifestV1` | ✅ | `planner.agents_manifest.v1` |
| `ar_report.rs` | `ArReportV1` | ✅ | `planner.ar_report.v1` |
| `consequence_card.rs` | `ConsequenceCardV1` | ✅ | `planner.consequence_card.v1` |
| `preview_snapshot.rs` | `PreviewSnapshotV1` | ✅ | `planner.preview_snapshot.v1` |
| `ralph_finding.rs` | `RalphFindingV1` | ✅ | `planner.ralph_finding.v1` |
| `git_commit.rs` | `GitCommitV1` | ✅ | `planner.git_commit.v1` |
| `runtime.rs` | `GateResultV1` | ✅ | `planner.gate_result.v1` |
| `runtime.rs` | `DecisionV1` | ✅ | `planner.decision.v1` |
| `runtime.rs` | `ContextPackV1` | ✅ | `planner.context_pack.v1` |
| `dtu.rs` | `DtuConfigV1` | ✅ | `planner.dtu_config.v1` |
| `pyramid_summary.rs` | `PyramidSummaryV1` | ✅ | `planner.pyramid_summary.v1` |

**All 18 types are defined and implement `ArtifactPayload`.** The type registry table in `lib.rs` lists only 18 entries but omits `planner.gate_result.v1`, `planner.decision.v1`, `planner.context_pack.v1`, `planner.dtu_config.v1`, and `planner.pyramid_summary.v1` — the doc table is **missing 5 entries** and should list 18 not 13.

### 1.2 Which Types Are Actually Used by Pipeline Steps

Cross-referencing `planner-core/src/pipeline/mod.rs` and the step files:

| Type | Constructed by pipeline | Notes |
|---|---|---|
| `IntakeV1` | ✅ `intake::execute_intake()` | Persisted via `Turn::new()` |
| `NLSpecV1` | ✅ `compile::compile_spec()` / multi-chunk | |
| `ArReportV1` | ✅ `ar::execute_adversarial_review()` | |
| `GraphDotV1` | ✅ `compile::compile_graph_dot()` | |
| `ScenarioSetV1` | ✅ `compile::generate_scenarios()` | |
| `AgentsManifestV1` | ✅ `compile::compile_agents_manifest()` | |
| `FactoryOutputV1` | ✅ `factory::execute_factory_with_worker()` | |
| `SatisfactionResultV1` | ✅ `validate::execute_scenario_validation()` | |
| `RunBudgetV1` | ✅ `RunBudgetV1::new_phase0()` in `run_full_pipeline` | |
| `GitCommitV1` | ✅ `git::execute_git_projection()` | |
| `RalphFindingV1` | ✅ `ralph::execute_ralph()` | |
| `GateResultV1` | ✅ referenced in `run_full_pipeline` retry logic | |
| `DecisionV1` | ⚠️ Type defined; no pipeline step constructs it yet | Sacred Anchor amendment flow is not wired |
| `ContextPackV1` | ⚠️ Type defined; no pipeline call site found in `pipeline/mod.rs` | Exists in `steps/context_pack.rs` but not called from the main runner |
| `DtuConfigV1` | ⚠️ Schema defined; `ralph::DtuConfiguration` variant exists but DTU config wiring is Phase 4 | Not constructed in Phase 0 runs |
| `PreviewSnapshotV1` | ⚠️ Type defined; `DeploySandbox` step listed in recipe but `run_full_pipeline` does not call it | Never constructed in current pipeline |
| `ConsequenceCardV1` | ⚠️ Ralph generates `consequence_cards` vec but they are only logged via `tracing::warn!`, not stored as `Turn<ConsequenceCardV1>` | Written to CXDB only if Ralph wiring calls `config.persist` — not confirmed |
| `PyramidSummaryV1` | ⚠️ `pyramid.rs` module exists; no call in `run_full_pipeline` | Phase 6+ feature, not called |

**Summary:** 10 of 18 artifact types are actively constructed and used in the current pipeline runner. The 8 remaining types (`DecisionV1`, `ContextPackV1`, `DtuConfigV1`, `PreviewSnapshotV1`, `ConsequenceCardV1`, `PyramidSummaryV1`, and supporting infrastructure) are defined and well-formed but not yet wired into the Phase 0 runner.

### 1.3 Turn<T> Hashing — Is It Real?

**Yes, it is real.** `Turn::new()` calls:

```rust
let blob_bytes = rmp_serde::to_vec(&payload).expect("payload must be serializable to msgpack");
let blob_hash = blake3::hash(&blob_bytes).to_hex().to_string();
```

- BLAKE3 of msgpack-encoded payload is computed on every `Turn::new()` call.
- `verify_integrity()` re-encodes and re-hashes, returning `false` on tamper.
- The tests (`turn_roundtrip_and_integrity` and `tampered_payload_fails_integrity`) are **meaningful** — they exercise the actual hash path.
- **Issue:** `expect()` on the msgpack serialization will **panic** if any payload type becomes non-serializable. This is an `expect` in a hot path.

### 1.4 ArtifactPayload Trait

The trait is a real interface, not a marker-only stub:

```rust
pub trait ArtifactPayload: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    const TYPE_ID: &'static str;
}
```

Every struct implements it with a distinct `TYPE_ID`. The `const` approach correctly prevents runtime registry collisions.

### 1.5 No `unsafe` Blocks

None found in any schema file. All types derive `Serialize`/`Deserialize` via macros.

---

## 2. planner-server

### 2.1 auth.rs — JWT Validation Security Audit

**CRITICAL SECURITY ISSUE: Insecure-by-default signature validation.**

When `AUTH0_DOMAIN` is set but `AUTH0_SECRET` is **not** set (the expected production Auth0 configuration — RS256 with JWKS), the code falls into:

```rust
None => {
    // Without a decoding key, do insecure decode (dev/testing only).
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    decode::<Claims>(token, &DecodingKey::from_secret(b""), &validation)
        ...
}
```

This means: if you set `AUTH0_DOMAIN` and `AUTH0_AUDIENCE` but forget to also set `AUTH0_SECRET`, **the server accepts any JWT regardless of signature and ignores expiration**. An attacker can forge an arbitrary `sub` claim with no cryptographic material and gain authenticated access.

The comment correctly says "dev/testing only" but the code path is reached in a production deployment whenever the operator sets domain/audience without a secret — which is the normal Auth0 RS256 configuration (keys come from JWKS, not a shared secret).

**Correct fix:** Without a `decoding_key`, the server should **fetch the JWKS** from `https://{domain}/.well-known/jwks.json` and validate using RS256 public keys. The current fallback should either panic at startup or return `401 Unauthorized` on every request until a key source is configured.

Additional observations:
- `AuthConfig::from_env()` uses `unwrap_or_default()` for `AUTH0_AUDIENCE`, so an unset audience silently disables audience validation (`validation.validate_aud = false`). This is documented but risky.
- Dev mode (no `AUTH0_DOMAIN`) inserts `exp: u64::MAX` synthetic claims — intentional and safe for local dev.
- Tests for token extraction are meaningful and correct.

### 2.2 session.rs — User Isolation

**Session isolation is correctly enforced.** Every access path checks ownership:

- `get_session`: checks `session.user_id != claims.sub` → 403
- `send_message`: checks ownership before mutation
- `ws_handler`: checks ownership before upgrading

`SessionStore` uses `RwLock<HashMap<Uuid, Session>>`:
- `RwLock::unwrap()` on read/write will **panic** on lock poisoning. If any thread panics while holding the write lock, all subsequent requests crash. This is an `unwrap()` in a shared-state hot path.
- Sessions are in-memory only — no persistence across restarts. This is documented behavior but means all active sessions are lost on server restart.
- No session expiry or cleanup mechanism. Memory will grow unboundedly with stale sessions.

### 2.3 api.rs — Pipeline Background Task

**The pipeline background task is wired correctly but has a critical logic bug and a potential panic.**

**Bug 1: Double-spawn condition**

```rust
if session.pipeline_running && session.project_description.as_deref() == Some(&content) {
    tokio::spawn(async move {
        run_pipeline_for_session(state_clone, session_id, description).await;
    });
}
```

The check uses `project_description == content`, which is true after the update sets it. However, on any *subsequent* call to `send_message` while the pipeline is running, `pipeline_running` is true but `project_description` is the *first* message's content — so the spawn check would only fail if the user sends the exact same text again. If a user sends a *different* message while the pipeline is running, `pipeline_running` is true but the `project_description` won't match the new content, so no spawn occurs (correct). However, the condition is fragile and should be a dedicated boolean flag like `pipeline_spawned`.

**Bug 2: Index assumption panic**

```rust
let msgs = &session.messages;
let user_msg = msgs[msgs.len() - 2].clone();
let planner_msg = msgs[msgs.len() - 1].clone();
```

This assumes at least 2 messages were added. Because `add_message` for user and planner runs inside `sessions.update()` before this code executes, there will always be at least a system message + user message + planner message (3 total) on first call. But on subsequent calls (pipeline already running), only one message is added (the "Pipeline is currently running" reply), leaving `msgs.len() - 2` potentially pointing at the previous planner message, not the current user message. The response object's `user_message` field will be the wrong message.

**What happens when LLM CLIs aren't available (server path):**

`run_pipeline_for_session` calls `CodexFactoryWorker::new()` which always succeeds (it only checks for the CLI and logs a warning). The actual failure occurs inside `generate()` when `cli_available == false`:

```rust
return Err(StepError::FactoryError("codex CLI not found..."));
```

This returns an `Err` that propagates to `run_full_pipeline`, which returns `Err(e)`. The error handler in `run_pipeline_for_session` correctly catches this and calls `s.add_message("planner", &format!("Pipeline failed: {}", e))`. **The server will not crash** — it gracefully posts an error message to the session. The pipeline background task will silently fail and leave the session in `pipeline_running: false` with an error message.

**Downstream, the LLM intake step** (`planner_core::pipeline::steps::intake`) will fail long before the factory worker if no LLM CLI (`claude`) is available. The same graceful error path applies.

### 2.4 ws.rs — WebSocket Handler

**The WebSocket handler is functionally correct but has a design limitation and one potential issue.**

**What works:**
- Polling loop every 500ms: reads current session state, sends new messages, sends all stage statuses.
- New messages are forwarded by comparing `last_msg_count` to `session.messages.len()`.
- Pipeline completion closes the WebSocket (`return` after sending `PipelineComplete`).
- Client disconnection detected via `None` or `Close` from `socket.recv()`.
- Ownership already verified in `ws_handler` before reaching `handle_ws`.

**Design limitation — stage updates are noisy:**
Every 500ms tick sends all 12 stage status messages regardless of whether anything changed. A client receiving 12 `stage_update` messages per tick (24/second) for a multi-minute pipeline run will receive thousands of redundant messages. Should diff against last-sent state.

**Potential issue — pipeline never started:**
The completion condition is:
```rust
if !session.pipeline_running && session.project_description.is_some() {
```
If a client connects to a session where the WebSocket connection is opened before any message is sent (so `project_description` is `None` and `pipeline_running` is false), the loop runs indefinitely, sending stage updates every 500ms until the client disconnects. This is not a crash but is a resource leak for idle connections.

**`StartPipeline` client message does not actually start the pipeline** — it adds the user message to the session but does not call `run_pipeline_for_session`. This is effectively dead functionality; pipeline start only works via `POST /api/sessions/:id/message`.

### 2.5 main.rs

No issues. CORS is tightened when auth is enabled (restricts to `localhost:5173` and `localhost:3100`). Static file serving is conditional on directory existence with graceful fallback. `unwrap()` on TCP bind is acceptable for a startup-phase operation.

### 2.6 Server Tests

Tests are meaningful:
- `test_health`, `test_health_no_auth_required`: real HTTP round-trips via `tower::ServiceExt::oneshot`.
- `test_get_session_wrong_user`: verifies cross-user isolation returns 403.
- `test_send_message_wrong_user`: same.
- `test_protected_endpoint_requires_token_when_auth_enabled`: verifies 401 without token.
- `test_send_empty_message`: whitespace-only message rejected with 400.

The `test_send_message` test verifies `pipeline_running` becomes true and the planner message contains "pipeline". It does **not** test the actual pipeline execution (which would require LLM CLIs), which is appropriate for unit tests.

---

## 3. planner-tui

### 3.1 Is the TUI Actually Functional?

**Yes.** The TUI is a fully wired Ratatui application with:
- Real terminal setup/teardown using crossterm (raw mode, alternate screen, mouse capture).
- Async event loop with `250ms` tick rate.
- Cursor position tracking for mid-line editing (insert, delete, left, right, home, end).
- Focus switching between input and chat scroll modes.
- Pipeline channel integration.

### 3.2 spawn_pipeline() — What Happens Without LLM CLIs

`spawn_pipeline()` in `pipeline.rs`:

1. Immediately sends `PipelineEvent::Started` — the TUI updates status to "Pipeline running..."
2. Calls `CodexFactoryWorker::new()` — **always succeeds** (only logs a warning if `codex` is absent).
3. Calls `run_full_pipeline()` — which calls `intake::execute_intake()` first, invoking the `claude` CLI.
4. If `claude` is not on PATH, the intake step returns `Err(StepError::CliNotFound(...))` or similar.
5. `run_full_pipeline` propagates the error.
6. `spawn_pipeline` catches it and sends `PipelineEvent::Failed("...error message...")`.
7. `App::tick()` receives `PipelineEvent::Failed`, sets `pipeline_running = false`, marks the first Running stage as Failed, and posts the error as a planner message.

**The TUI will not crash** when LLM CLIs are absent. The user sees a "Pipeline failed: ..." message in the chat window. This is the correct UX for a missing-dependency failure.

### 3.3 app.rs — tick() → Pipeline Event Processing

`tick()` is **real and correct**:

```rust
pub fn tick(&mut self) {
    let events: Vec<PipelineEvent> = {
        if let Some(ref mut rx) = self.pipeline_rx {
            let mut buf = Vec::new();
            while let Ok(ev) = rx.try_recv() {
                buf.push(ev);
            }
            buf
        } else {
            return;
        }
    };
    for event in events { ... }
}
```

The double-buffer pattern (collect into `Vec` then process) is the correct way to drain a `tokio::sync::mpsc::UnboundedReceiver` without holding a mutable borrow on `self` while calling `&mut self` methods. This is well-implemented.

Event handling:
- `Started` → updates status bar label only.
- `Completed(summary)` → marks all 12 stages Complete, adds planner message with full summary (project name, feature slug, spec count, factory status, git hash).
- `Failed(err)` → marks first Running stage as Failed, adds planner error message, clears running flag.

**Gap:** There is no intermediate progress — the pipeline is all-or-nothing from the TUI's perspective. The stages all flip to Complete at the end rather than advancing one-by-one as each pipeline step finishes. This is because `PipelineEvent` only has `Started/Completed/Failed` — there is no `StepComplete(step_name)` variant. This is a known limitation (commented as "Phase F pipeline wiring").

### 3.4 Key Handling Edge Cases

**Potential panic:** In `handle_input_key`:

```rust
KeyCode::Char(c) => {
    self.input.insert(self.cursor_position, c);
    self.cursor_position += 1;
}
```

`String::insert` takes a **byte index**, not a character index. If `cursor_position` ever points into the middle of a multi-byte UTF-8 character, `insert` will **panic with a char boundary error**. The cursor is maintained as a usize that increments by 1 per `KeyCode::Char`, but `KeyCode::Char(c)` delivers a full Unicode scalar value — so `cursor_position` advances by 1 for emoji (which may be 4 bytes). Moving left/right also adjusts by 1, meaning after typing an emoji the cursor can point to a non-boundary position.

This is a standard Ratatui/crossterm bug surface for non-ASCII input. For ASCII-only input (likely in practice for project descriptions) this will not trigger.

### 3.5 events.rs — EventHandler

`event::poll()` is called with the tick rate duration. The return value is `unwrap_or(false)` — if `poll()` returns an error (e.g., terminal closed) it silently falls back to `Tick`. This is reasonable for robustness but masks terminal errors.

The `EventHandler::next()` loop returns `Tick` if `poll` times out without an event. For key and resize events, it returns immediately. The `async fn next()` is called with `.await` in `run_app`, which is correct for async context usage.

### 3.6 ui.rs — Rendering

The renderer is real Ratatui code. Layout is `[3, Min(8), 3, 3]` — header, chat, pipeline status bar, input. All four panels draw in `draw()`.

**Scrollbar heuristic**: scrollbar is only shown when `messages.len() > 5`. The `ScrollbarState::new(messages.len() * 3)` is an approximation of total scrollable rows. This may not match actual rendered line count for messages with newlines, causing scroll thumb position to be inaccurate for long multi-line messages.

**Cursor position calculation**:
```rust
frame.set_cursor_position(Position::new(
    area.x + app.cursor_position as u16 + 3, // +3 for border + "> "
    area.y + 1, // +1 for border
));
```
Same multi-byte issue as above: `cursor_position` is a byte index used as a column offset. For ASCII this is correct; for multi-byte characters the cursor visual position will be wrong.

### 3.7 TUI Tests

Tests are meaningful:
- `tick_processes_pipeline_events` and `tick_handles_pipeline_failure`: directly inject `PipelineEvent` through a channel and verify state transitions — this is exactly the right way to test async event handling without spawning a real pipeline.
- `pending_pipeline_description_is_set_and_taken`: verifies the one-shot description handoff.
- `draw_does_not_panic` / `draw_with_messages_does_not_panic` / `draw_with_pipeline_progress`: render to `TestBackend` — real regression guards.
- `app_get_session_wrong_user`, `app_esc_clears_or_quits`, `app_ctrl_c_quits`: all test real behavior.

---

## 4. Cross-Cutting Issues

### 4.1 `unwrap()` in Production Paths

| Location | Call | Risk |
|---|---|---|
| `turn.rs:91` | `rmp_serde::to_vec(&payload).expect(...)` | Panics if any payload has a non-serializable field (unlikely but possible with `serde_json::Value` fields in `DtuSeedEntry`) |
| `turn.rs:113` | same in `verify_integrity()` | Same |
| `session.rs:111` | `self.sessions.write().unwrap()` | Panics on lock poison across all write paths |
| `session.rs:117` | `self.sessions.read().unwrap()` | Panics on lock poison across all read paths |
| `session.rs:125` | same in `update()` | |
| `session.rs:136` | same in `list_for_user()` | |
| `session.rs:147` | same in `list_ids()` | |
| `session.rs:152` | same in `count()` | |
| `main.rs:128` | `TcpListener::bind().await.unwrap()` | Panics if port is already in use; acceptable for startup |
| `main.rs:129` | `axum::serve().await.unwrap()` | Panics on server error |

The `RwLock::unwrap()` pattern in `SessionStore` is the most dangerous in production. Any panic in a write-lock-holding context (e.g., inside `sessions.update()`) poisons the lock and crashes all subsequent requests. Should use `.map_err(|_| ...)` with `expect()` replaced by proper error handling, or use a `parking_lot::RwLock` which does not poison.

### 4.2 No `unsafe` Blocks

No `unsafe` found in any of the three crates.

### 4.3 Missing Types in Schema doc-table

`lib.rs` lists 13 types in its CXDB Type Registry table but 18 are implemented. Missing from the table:
- `planner.gate_result.v1`
- `planner.decision.v1`
- `planner.context_pack.v1`
- `planner.dtu_config.v1`
- `planner.pyramid_summary.v1`

### 4.4 Model Names in api.rs Are Speculative

The `models` endpoint in `api.rs` hardcodes model IDs like `gpt-5.3-codex`, `gpt-5.2`, `gemini-3.1-pro`, `claude-opus-4-6`. These are future/fictional model names not available at current API endpoints. This is acceptable as forward-looking configuration but will cause confusion if the frontend uses these IDs for actual API calls.

---

## 5. Critical Issues Summary

| Severity | Location | Issue |
|---|---|---|
| 🔴 CRITICAL | `auth.rs:173-181` | Signature validation silently disabled when `AUTH0_SECRET` is unset; any JWT accepted |
| 🔴 CRITICAL | `auth.rs:177` | Expiry validation disabled in same path; expired tokens accepted |
| 🟠 HIGH | `session.rs` (all lock sites) | `RwLock::unwrap()` on every read/write — lock poisoning crashes the entire server |
| 🟠 HIGH | `app.rs:213` | `String::insert(cursor_position, c)` panics on multi-byte UTF-8 input |
| 🟡 MEDIUM | `api.rs:271` | Spawn condition is fragile — uses content equality instead of a dedicated flag |
| 🟡 MEDIUM | `api.rs:281-283` | Index-based message access (`msgs.len() - 2`) returns wrong message on second+ send |
| 🟡 MEDIUM | `ws.rs:166-171` | `StartPipeline` client message accepted but does not actually start the pipeline |
| 🟡 MEDIUM | `ws.rs` | Stage updates sent every tick regardless of change — noisy for long runs |
| 🟡 MEDIUM | `turn.rs:91` | `expect()` on msgpack serialization — panics if payload is non-serializable |
| 🟢 LOW | `lib.rs` (schemas) | Doc table missing 5 of 18 artifact types |
| 🟢 LOW | `events.rs:37` | `poll()` errors silently mapped to `false` |
| 🟢 LOW | Session store | No expiry/cleanup — unbounded memory growth |
| 🟢 LOW | 8 artifact types | Defined but not yet constructed by pipeline runner (expected for Phase 0) |

---

## 6. Answers to Critical Questions

**Q1: Are all 18 artifact types populated by pipeline steps?**  
No. 10 are actively constructed. 8 (`DecisionV1`, `ContextPackV1`, `DtuConfigV1`, `PreviewSnapshotV1`, `ConsequenceCardV1`, `PyramidSummaryV1`, and the `context_pack` / `dtu` subtypes) are defined and well-formed but not wired into the Phase 0 pipeline runner. This is consistent with the codebase's phased rollout design.

**Q2: Is JWT validation production-grade or insecure-by-default?**  
Insecure-by-default. When `AUTH0_DOMAIN` is set without `AUTH0_SECRET`, signature validation and expiry checks are disabled. A real production deployment requires either fetching JWKS (unimplemented) or setting `AUTH0_SECRET`. The code should fail closed, not open.

**Q3: Does the pipeline background task work? Would `send_message()` crash?**  
The background task is correctly spawned in a `tokio::spawn`. `send_message()` will not crash — errors propagate gracefully as planner messages. The spawn condition logic is fragile but functionally correct for the common case (one message per session).

**Q4: Is the WebSocket handler functional?**  
Yes. Polling-based (500ms), sends new messages and stage updates, closes on completion. Main limitations: noisy (sends all 12 stage statuses every tick), `StartPipeline` message does nothing, idle connections (pre-pipeline) run forever.

**Q5: Is `user_id` enforced in session.rs?**  
Yes. Both `get_session` and `send_message` check `session.user_id != claims.sub` and return 403. Cross-user access is blocked. Tests cover this explicitly.

**Q6: Does `spawn_pipeline()` work? What happens without LLM CLIs?**  
`spawn_pipeline()` works correctly. Without LLM CLIs, the pipeline fails gracefully at the intake step and sends `PipelineEvent::Failed(error_message)` through the channel. The TUI displays the error in the chat window. No crash.

**Q7: Is `tick()` → pipeline event processing real?**  
Yes. It is real, correct, and well-tested. The double-buffer drain pattern is idiomatic for Tokio mpsc in non-async contexts. Events update TUI state correctly for all three event types.
