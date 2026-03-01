# Backend Sprint 5 — Changes Log

**Date:** 2026-02-28  
**Status:** All 6 changes applied. `cargo check` ✅  `cargo test` ✅  
**Tests:** 377 passed, 0 failed (245 planner-core + 45 integration + 4 schemas + 61 server + 22 tui)

---

## Change 1 — Wire ConsequenceCards as persisted Turns

**File:** `planner-core/src/pipeline/mod.rs`

After the Ralph loop warning log (previously only logging the count), each
`ConsequenceCardV1` is now persisted as an immutable `Turn` via `config.persist`.

```rust
for card in &ralph_output.consequence_cards {
    let turn = Turn::new(card.clone(), None, run_id, "ralph", "consequence-card");
    config.persist(&turn);
}
```

`ConsequenceCardV1` already implements `ArtifactPayload` (TYPE_ID =
`"planner.consequence_card.v1"`), so this compiles without any wrapping.
The turns land in the CXDB store under `type_id = "planner.consequence_card.v1"`
and are queryable via the new `/sessions/{id}/turns` endpoint.

---

## Change 2 — Wire ContextPack into pipeline spec compilation

**File:** `planner-core/src/pipeline/mod.rs`

Immediately after persisting NLSpecV1 chunks (Step 3), a `ContextPackV1` is
built and logged using `build_spec_context_pack` with `ContextTarget::SpecCompiler`
and a token budget of 8 000 tokens:

```rust
let pack = steps::context_pack::build_spec_context_pack(
    &specs[0],
    steps::context_pack::ContextTarget::SpecCompiler,
    8000,
);
tracing::info!(
    "  → ContextPack: {} sections, ~{} tokens, truncated={}",
    pack.sections.len(), pack.estimated_tokens, pack.was_truncated,
);
```

`ContextPackV1` is a **local struct** (not from `planner-schemas`) and does not
implement `ArtifactPayload`, so it is logged only — it cannot be persisted as a
Turn without a schema change. A TODO comment explains this constraint.

---

## Change 3 — Wire DTU clones into scenario validation

**File:** `planner-core/src/pipeline/steps/validate.rs`

`execute_scenario_validation` now accepts an additional parameter:

```rust
pub async fn execute_scenario_validation(
    router: &LlmRouter,
    scenarios: &ScenarioSetV1,
    factory_output: &FactoryOutputV1,
    dtu_registry: Option<&DtuRegistry>,
) -> StepResult<SatisfactionResultV1>
```

When `dtu_registry` is `Some` and has registered providers, their names, IDs,
and supported endpoints are logged at `INFO` level so the evaluation context
documents which DTU clones were active:

```
Scenario Validator: 2 DTU clone(s) available:
  - Stripe (stripe): endpoints=["/v1/customers", ...]
  - Auth0 (auth0): endpoints=["/oauth/token", ...]
```

The call site in `pipeline/mod.rs` is updated to pass `config.dtu_registry`.
This is the natural extension point for Phase 5+ work that routes actual
scenario evaluation HTTP calls through the DTU clones instead of mock paths.

---

## Change 4 — CXDB HTTP read API endpoints

**File:** `planner-server/src/api.rs`

### New response types

```rust
pub struct TurnResponse      { turn_id, type_id, timestamp, produced_by }
pub struct ListTurnsResponse { turns: Vec<TurnResponse>, count: usize }
pub struct RunListResponse   { runs: Vec<String> }
```

### New routes (added to protected router)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/sessions/{id}/turns` | List Turn metadata for a session |
| `GET` | `/sessions/{id}/runs`  | List run UUIDs for a session |

Both handlers enforce ownership (403 for wrong user, 404 for missing session).
Both currently return **empty lists** because `PipelineConfig::minimal` (used
by the server) does not wire durable storage.

TODO comments in the handlers note the wiring path once storage is attached:
> `store.list_turns_for_session(session_id)` once wired.

### Tests added (6 new tests)

- `test_list_turns_empty` — 200 + empty list
- `test_list_turns_not_found` — 404
- `test_list_turns_wrong_user` — 403
- `test_list_runs_empty` — 200 + empty list
- `test_list_runs_not_found` — 404
- `test_list_runs_wrong_user` — 403

---

## Change 5 — Rate limiting middleware

**Files:** `planner-server/src/rate_limit.rs` (new), `planner-server/src/main.rs`

### rate_limit.rs

| Symbol | Description |
|--------|-------------|
| `RateLimiter` | `parking_lot::Mutex<HashMap<String, Vec<Instant>>>` |
| `RateLimiter::check_and_record` | Returns `true` if allowed; stale timestamps pruned on each call |
| `RateLimiter::evict_stale` | Removes expired entries (called by background task) |
| `extract_key` | `X-Forwarded-For` → `X-Real-IP` → `"unknown"` priority chain |
| `rate_limit_middleware` | Axum `from_fn_with_state` middleware; returns 429 + `Retry-After: 60` |
| `spawn_eviction_task` | Tokio background task; runs every 300 s |

**Limits:** 100 requests per 60-second sliding window per key.

### main.rs wiring

```rust
let rate_limiter = Arc::new(rate_limit::RateLimiter::new());
rate_limit::spawn_eviction_task(rate_limiter.clone());
// ...
.layer(axum::middleware::from_fn_with_state(
    rate_limiter,
    rate_limit::rate_limit_middleware,
))
```

Applied across all `/api` and `/api/v1` routes via the outer `Router`.

### Tests (7 new)

- `allows_requests_under_limit`
- `blocks_request_over_limit`
- `different_keys_are_independent`
- `evict_stale_removes_all_entries_after_window`
- `extract_key_xff_first_ip`
- `extract_key_real_ip_fallback`
- `extract_key_unknown_fallback`

---

## Change 6 — RBAC type definitions

**Files:** `planner-server/src/rbac.rs` (new), `planner-server/src/main.rs`

### Roles and permissions matrix

| Role     | Create | Read | Delete | Run | Cancel | TurnsR | TurnsX | SetR | SetW |
|----------|--------|------|--------|-----|--------|--------|--------|------|------|
| Admin    | ✓      | ✓    | ✓      | ✓   | ✓      | ✓      | ✓      | ✓    | ✓    |
| Operator | ✓      | ✓    | —      | ✓   | —      | ✓      | ✓      | ✓    | —    |
| Viewer   | —      | ✓    | —      | —   | —      | ✓      | —      | ✓    | —    |
| Service  | —      | —    | —      | ✓   | —      | ✓      | ✓      | —    | —    |

### Key types

- `Role` — enum with `Admin | Operator | Viewer | Service`
- `Permission` — 9-variant enum covering all CRUD + pipeline + settings ops
- `Role::permissions()` — returns owned `Vec<Permission>`
- `Role::has_permission(&Permission)` — O(n) linear scan (small n ≤ 9)
- `TeamMember { user_id, role, team_id }`
- `Team { team_id, name, members }` + `role_for(user_id)` + `has_permission(user_id, perm)`

Registered in `main.rs` as `pub mod rbac`. Phase 2 will add JWT claim
injection and enforcement middleware.

### Tests (15 new)

Full coverage of all Role/Permission combinations plus `Team` delegation helpers.

---

## Test summary

```
planner-core    245 passed (unit)
integration_e2e  45 passed
planner-schemas   4 passed
planner-server   61 passed  ← +13 new (6 CXDB + 7 rate_limit)
planner-tui      22 passed
────────────────────────
Total           377 passed  0 failed
```

RBAC tests (15) are counted inside the 61 planner-server tests above.
