# Planner v2 — Deep Integrity Sweep

**Date**: 2026-03-05  
**Scope**: Every `.rs`, `.ts`, `.tsx` file in production code (excluding tests).  
**Method**: Automated pattern scanning + manual reading of every handler, hook, and module entry point.

---

## Verdict: 2 Real Gaps Found, Everything Else Is Genuine

The codebase is substantially implemented. This is not a mockup project. But there
are two concrete gaps that the previous audit glossed over.

---

## ❌ GAP 1: CXDB Not Wired Into Server (SIGNIFICANT)

**What**: CXDB (`planner-core/src/cxdb/`) is a fully implemented MessagePack-on-disk
storage engine (694 lines in `durable.rs`). The pipeline calls `config.persist()`
at 12 points. **But the server passes `store: None`** at `api.rs:650`.

**Consequence**: Every pipeline run from the web UI discards all Turn data after
the session ends. The `/sessions/{id}/turns` and `/sessions/{id}/runs` endpoints
exist but return empty arrays with comments saying "returns empty until durable
CXDB store is wired."

**Same issue in TUI**: `pipeline.rs:339` uses `PipelineConfig::minimal()` which
sets `store: None`. `spawn_socratic_interview` passes `None::<&CxdbEngine>` at
line 279.

**What's needed**: Construct a `CxdbEngine` instance at server startup (pointed at
a data directory), store it in `AppState`, pass `store: Some(&engine)` to
`PipelineConfig`. Same pattern for TUI. Estimated: ~30 lines changed across 3 files.

**Severity**: Significant. The storage layer works but nothing uses it in production.

---

## ❌ GAP 2: TUI Pipeline Doesn't Pass CXDB Store Either

Same as Gap 1 but for the TUI path. `spawn_pipeline()` in `planner-tui/src/pipeline.rs`
constructs a `PipelineConfig::minimal()` with no store. `spawn_socratic_interview()`
passes `None` for the TurnStore.

This is the same fix — construct a `CxdbEngine` and thread it through. Grouped
with Gap 1 since it's the same root cause.

---

## ✅ Everything Else Is Real

### Rust Pipeline (13,692 lines across 22 files)
- All 12 pipeline stages implemented: Intake, Chunk, Compile, Lint, AR Review,
  AR Refinement, Scenarios, Ralph, Graph, Factory, Validate, Git
- No `todo!()`, `unimplemented!()`, or empty function bodies in production code
- 593 tests passing, zero warnings on `cargo check`

### LLM Clients (1,930 lines)
- `run_cli()` spawns actual CLI processes (`claude`, `gemini`, `codex`) via
  `tokio::process::Command`
- Sandboxed execution with `env_clear()`, isolated CWD, stdin piping
- Real timeout handling, retry logic, JSON repair
- `cli_available()` checks `which` for binary presence

### Frontend (33 production files, 168 tests)
- API client: 100% real `fetch()` calls to server endpoints, zero mock data
- `useSocraticWebSocket` (600 lines): real `new WebSocket()` connections with
  reconnection, state management, message parsing
- All pages consume live API data: SessionPage, Dashboard, AdminPage, BlueprintPage
- No hardcoded demo data, no fake responses anywhere

### Server API (2,130 lines in api.rs)
- 24 route handlers, all backed by real state management
- Auth middleware with Auth0 JWT validation (or dev mode bypass)
- Full Blueprint CRUD + impact analysis
- Real observability event collection and filtering
- Exception: `list_turns` and `list_runs` return empty (see Gap 1)

### WebSocket (ws.rs + ws_socratic.rs = 1,367 lines total)
- Pipeline WS: 500ms polling loop, sends stage updates, chat messages, completion events
- Socratic WS: Full bidirectional protocol — classification, belief state updates,
  questions, speculative drafts, draft reactions, contradiction detection, dimension edits
- Both handle client disconnect, session ownership, expiry touching

### CXDB Storage (1,952 lines)
- Content-addressable blob storage with BLAKE3 hashing
- MessagePack serialization via `rmp_serde`
- Run indexing, project-level run tracking, turn metadata
- Query layer with ancestor traversal and filtering
- Protocol layer for Turn lifecycle
- **Fully implemented, just not connected in server/TUI** (see Gaps)

### TUI (3,234 lines)
- Real ratatui terminal UI with input handling, stage tracking, event log
- Spawns actual Socratic interview via channels (`TuiSocraticIO` implements `SocraticIO`)
- Spawns actual pipeline execution via `spawn_pipeline()`
- Real event drain on tick, progress tracking, state transitions

### Socratic Engine (socratic/ — 2,494 lines)
- Real LLM calls: `classify_domain` → `router.complete()`, `generate_question` →
  `router.complete()`, `generate_draft` → `router.complete()`
- Belief state tracking with convergence calculation
- Constitution-based question validation
- Contradiction detection between dimensions
- Speculative draft generation with section-level review

### Verification (359 lines)
- Generates Lean4 theorem templates — the `sorry` keywords are Lean4's standard
  "proof not yet provided" marker. This is by-design, not a stub.

---

## Things That Looked Suspicious But Are Fine

| Pattern | Location | Why it's OK |
|---------|----------|-------------|
| `"placeholder"` in validate.rs | Lines 563, 568 | BDD trigger word list — detecting placeholders in *user* input |
| `sorry -- proof stub` in verification.rs | Multiple | Lean4 convention — theorem templates for formal methods team |
| `models()` returns static list | api.rs:428 | Model catalog is configuration, not dynamic data |
| `LOGIN_BANNER` is ASCII art | LoginPage.tsx:36 | Decorative, renders via `<pre>` with `aria-label` |

---

## Summary

| Area | Status | Lines |
|------|--------|-------|
| Pipeline stages | ✅ All 12 real | 13,692 |
| LLM clients | ✅ Real CLI invocation | 1,930 |
| CXDB storage | ⚠️ Implemented but disconnected | 1,952 |
| Server API | ⚠️ 22/24 endpoints real, 2 return empty | 2,130 |
| WebSocket | ✅ Fully bidirectional | 1,367 |
| TUI | ✅ Real functionality | 3,234 |
| Frontend | ✅ All live data | ~4,200 |
| Socratic engine | ✅ Real LLM calls | 2,494 |
| Tests | ✅ 761 total (593 Rust + 168 frontend) | — |

**Total production code**: ~31,000 lines  
**Gaps to fix**: 1 root cause (CXDB not wired), affects 2 endpoints + both TUI/server paths  
**Estimated fix**: ~30 lines across 3 files
