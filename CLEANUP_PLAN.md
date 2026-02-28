# Planner v2 — Stub & Dead Code Cleanup — COMPLETED

All 10 phases completed and merged to `main`. Summary below.

---

## Completed Phases

| Phase | Description | Commit | Tests | Net Lines |
|---|---|---|---|---|
| H | StepResult doc comment fix | `4935775` | 325 | ~2 |
| I | Verification module naming | `4935775` | 325 | ~6 |
| J | Multi-chunk test + heuristic bugfix | `4935775` | 325 | ~30 |
| E | Remove phantom reqwest dependency | `4935775` | 325 | -5 |
| A | Remove `#![allow(dead_code)]` blankets, fix all warnings | `7a32677` | 325, 0 warnings | ~50 |
| B | Kill old Kilroy simulation path | `fb1dea0` | 318, 0 warnings | -800+ |
| C | Kill git simulation path | `fb1dea0` | 318, 0 warnings | -100+ |
| D | Remove legacy SQLite, migrate TurnStore to CXDB | `fb1dea0` | 318, 0 warnings | -300+ |
| F | Wire TUI to real pipeline execution | `38df41d` | 321, 0 warnings | +250 |
| G | Wire Server to real pipeline + WebSocket | `38df41d` | 321, 0 warnings | +150 |

## Final Stats

- **321 tests** (235 unit + 46 integration + 2 schema + 19 TUI + 19 server)
- **0 compiler warnings**
- **~1,139 lines net deleted** from dead code removal (Phases B+C+D)
- **~400 lines added** for real pipeline wiring (Phases F+G)

## What Changed

### Removed
- Blanket `#![allow(dead_code)]` suppressions (Phase A)
- Old Kilroy CLI simulation path — `invoke_kilroy()`, `run_kilroy_simulation()`, `execute_factory_handoff()`, `KilroyCheckpoint` (Phase B)
- Git simulation — `simulate_git_projection()` (Phase C)
- SQLite storage — `storage/mod.rs`, `rusqlite` dependency (Phase D)
- Phantom `reqwest` dependency (Phase E)

### Implemented
- `run_full_pipeline()` as the single pipeline entry point with `FactoryWorker` trait (Phase B)
- `StepError::GitNotAvailable` for missing git (Phase C)
- `TurnStore` trait + `StorageError` relocated to `cxdb/mod.rs` (Phase D)
- TUI → real pipeline via `mpsc::unbounded_channel` + background tokio task (Phase F)
- Server → real pipeline via `tokio::spawn` + WebSocket handler with 500ms polling (Phase G)
- Multi-chunk integration test using `build_multi_chunk_intake()` (Phase J)
- Heuristic bugfix for `chunk_planner` empty-description edge case (Phase J)

### Renamed/Clarified
- `StepResult<T>` doc: "Placeholder" → "Result type for pipeline step execution" (Phase H)
- Verification module: "Stubs" → "Lean4 Proposition Generation" (Phase I)
- `StepError::KilroyError` → `StepError::FactoryError` (Phase B)
