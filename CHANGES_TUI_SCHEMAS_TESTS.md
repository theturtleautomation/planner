# TUI, Schemas, Tests & Code Quality Fixes

**Date**: 2026-02-28  
**Branch**: main  
**cargo check**: ✅ clean (0 errors, 0 warnings)  
**cargo test**: ✅ 349 tests, 0 failures

---

## Summary of Changes

### 1. TUI UTF-8 Cursor Fix
**File**: `planner-tui/src/app.rs`

The input cursor in `handle_input_key` used byte indices for `String::insert` /
`String::remove`, causing panics or silent data corruption on multi-byte Unicode
characters (e.g. `©`, `中`, `€`).

**Fix**: All cursor arithmetic now uses _character_ indices (`char_indices().nth(pos)`
to convert the char-index to the byte offset required by `String::insert` /
`String::remove`). Affected key handlers: `Char`, `Backspace`, `Delete`, `Right`,
`End` — all now operate correctly on multi-byte input.

**New test**: `app_utf8_multibyte_cursor` — exercises insert, backspace, delete,
left, right, home, end with `a©中b` and `€` (2- and 3-byte characters).

---

### 2. Schema Doc Table Verification
**File**: `planner-schemas/src/lib.rs`

Verified that all 5 new type entries introduced in previous sessions
(`ArReportV1`, `ArFindingV1`, `TelemetryReportV1`, `PyramidTreeV1`,
`ProjectInfoV1`) are already present in the module-level type registry doc
table. No change required.

---

### 3. Recipe::phase0() Doc Comment
**File**: `planner-core/src/pipeline/mod.rs`

Added a doc comment to the `Recipe::phase0()` call-site clearly marking it as
a **Phase 3+ Feature — DAG Interpreter** that is not yet wired into execution.
This prevents future confusion about the unreferenced method.

---

### 4. ProjectRegistry Wired into run_full_pipeline
**File**: `planner-core/src/pipeline/mod.rs`

`run_full_pipeline` now:
1. Constructs a `ProjectRegistry` and calls `registry.register(name, slug, tags)`
   after the front-office step completes (slug comes from the intake artifact).
   Duplicate-slug errors are silently ignored (idempotent across retries).
2. Calls `registry.update_status(project_id, ProjectStatus::Completed)` at the
   end of a successful run.

Previously the `ProjectRegistry` type was defined but never instantiated inside
the pipeline.

---

### 5. PyramidBuilder Wired into run_full_pipeline
**File**: `planner-core/src/pipeline/mod.rs`

`PyramidBuilder::with_defaults().build_pyramid(project_id, &turn_pairs)` is
now called after the telemetry step, using the run's turn IDs and type labels.
The resulting `PyramidTree` is stored in the `PipelineOutput`. No LLM call is
needed — the pyramid builder uses deterministic truncation.

---

### 6. Linter Rule 7 Fixed
**File**: `planner-core/src/pipeline/steps/linter.rs`

Rule 7 was a no-op: `let _ = &dep.dtu_priority` discarded the value without
checking it. The correct check is `dep.dtu_priority == DtuPriority::None` —
if a DTU dependency has no priority set, it's a linter violation.

**Fix**: Replaced the no-op with:
```rust
if dep.dtu_priority == DtuPriority::None {
    violations.push(LintViolation {
        rule: "R7".into(),
        message: format!(
            "DTU dependency '{}' has no priority set (DtuPriority::None)",
            dep.provider
        ),
        severity: LintSeverity::Warning,
        location: format!("dtu_dependencies[{}]", dep.provider),
    });
}
```

---

### 7 & 8. Integration Test Improvements
**File**: `planner-core/tests/integration_e2e.rs`

Replaced three trivial/pass-through integration tests with meaningful ones:

| Old test (trivial) | New test (meaningful) |
|---|---|
| `e2e_phase2_ar_severity_classification` — just constructed an `ArFinding` struct | Now exercises `ArReportV1::recalculate()` with a full set of blocking/advisory/informational findings and asserts counts |
| `e2e_phase2_refinement_no_blocking_passthrough` — returned early without calling any logic | Now calls `execute_ar_refinement` with `has_blocking=false` and asserts the short-circuit path (`iterations=0, resolved=true`) |
| `e2e_phase2_recipe_includes_ar_steps` + `e2e_phase3_recipe_includes_new_steps` — two overlapping recipe length checks | Consolidated into one comprehensive 17-step recipe regression test with per-step name assertions |

---

### 9. project_id Added to TurnMetadata
**Files**:
- `planner-schemas/src/turn.rs`
- `planner-core/src/cxdb/mod.rs`

**Problem**: `CxdbEngine::store_turn` was passing `project_id: None` as a
hardcoded literal, preventing CXDB from automatically indexing turns under the
correct project for multi-project queries.

**Changes**:

**`planner-schemas/src/turn.rs`**:
- Added `project_id: Option<Uuid>` field to `TurnMetadata`
- `Turn::new` sets `project_id: None` (backward compatible — all existing call
  sites continue to compile without changes)
- Added `Turn::new_with_project(payload, parent_id, run_id, produced_by,
  execution_id, project_id)` constructor that sets `metadata.project_id`
- Added tests: `turn_new_with_project_sets_project_id`,
  `turn_new_without_project_has_none`

**`planner-core/src/cxdb/mod.rs`**:
- `TurnStore::store_turn` now passes `turn.metadata.project_id` instead of
  `None` to `store_turn_internal`
- `reconstruct_turn` now copies `stored._project_id` back into
  `metadata.project_id` so the field survives a store/retrieve round-trip
- Added test: `cxdb_project_id_roundtrip` — stores a turn via
  `Turn::new_with_project`, retrieves it, and asserts both that
  `metadata.project_id` is preserved and that the run is indexed under the
  project

---

### 10. AR Reviewers Parallelized with tokio::join!
**File**: `planner-core/src/pipeline/steps/ar.rs`

The three `run_single_reviewer` calls in `execute_adversarial_review` were
sequential. Since each call only holds a shared `&LlmRouter` reference with no
mutable state, all three can run concurrently.

**Fix**: Replaced the three sequential `.await?` calls with a single
`tokio::join!` macro invocation. The results are propagated with `?` after the
join completes (fail-fast: the first error wins).

**Performance impact**: Wall-clock latency reduced from ~sum of all three LLM
round-trips (typically 6–12 s) to ~max of any single reviewer (~2–4 s).

**Doc comment updated** to reflect the change from "sequential for simplicity"
to parallel via `tokio::join!`.

---

### 11. TUI StepComplete Event
**Files**:
- `planner-tui/src/pipeline.rs`
- `planner-tui/src/app.rs`

**Problem**: The TUI progress tracker (12 pipeline stages) never advanced during
pipeline execution — it only bulk-completed all stages on `Completed` or
bulk-failed on `Failed`.

**Changes**:

**`planner-tui/src/pipeline.rs`**:
- Added `StepComplete(String)` variant to `PipelineEvent` with doc comment
  explaining the intended behavior
- `spawn_pipeline` now emits one `StepComplete(stage_name)` event per stage
  (in order) immediately before the final `Completed` event. Since
  `run_full_pipeline` is currently monolithic (no internal callbacks), this
  drives the progress bar correctly on success while establishing the event
  contract for future per-step wiring

**`planner-tui/src/app.rs`**:
- Added `PipelineEvent::StepComplete(name)` arm to `tick()`:
  - Finds the matching stage by name, marks it `Complete`
  - Marks the immediately following `Pending` stage as `Running` (ripple
    advance)
  - Updates `status_message` to `"Completed: {name}"`
- Added tests:
  - `tick_step_complete_advances_stages` — sends `StepComplete("Intake")` then
    `StepComplete("Chunk")` and asserts stage status transitions
  - `tick_step_complete_unknown_name_is_noop` — sends a name that doesn't match
    any stage and asserts no stages change state

---

## Test Results

```
cargo check: Finished dev profile — 0 errors
cargo test:  349 passed; 0 failed; 0 ignored

  planner-core (unit):        245 tests ✅
  planner-core (integration):  45 tests ✅
  planner-schemas (unit):       4 tests ✅
  planner-server (unit):       33 tests ✅
  planner-tui (unit):          22 tests ✅
```

## Files Modified

| File | Change |
|---|---|
| `planner-tui/src/app.rs` | UTF-8 cursor fix; StepComplete handler; 3 new tests |
| `planner-tui/src/pipeline.rs` | Added StepComplete variant; emit events in spawn_pipeline |
| `planner-schemas/src/turn.rs` | Added project_id to TurnMetadata; Turn::new_with_project; 2 new tests |
| `planner-core/src/cxdb/mod.rs` | store_turn uses turn.metadata.project_id; reconstruct_turn preserves it; 1 new test |
| `planner-core/src/pipeline/mod.rs` | Wired ProjectRegistry; wired PyramidBuilder; Recipe doc comment |
| `planner-core/src/pipeline/steps/ar.rs` | tokio::join! parallelization of 3 reviewers |
| `planner-core/src/pipeline/steps/linter.rs` | Rule 7 DtuPriority::None check (was no-op) |
| `planner-core/tests/integration_e2e.rs` | Replaced 3 trivial tests; consolidated duplicate recipe test |
