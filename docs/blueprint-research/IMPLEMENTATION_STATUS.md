# Blueprint Implementation — Status Tracker

**Started:** March 5, 2026
**Last Updated:** March 5, 2026 (Phase B complete)

## Research Documents (committed to repo)
- `docs/blueprint-research/BLUEPRINT_DEEP_DIVE.md` — Decision audit, spec vs. code gap analysis
- `docs/blueprint-research/BLUEPRINT_MOCKUP_VS_IMPLEMENTATION.md` — Mockup v2 vs. current code, feature roadmap
- `docs/blueprint-research/architecture_tools_research.md` — Industry tool research (Backstage, Structurizr, etc.)

## Implementation Phases

### Phase A: Type Alignment & Cleanup [COMPLETE]
- [x] A.1 — Sync TypeScript types with Rust structs (CRITICAL)
  - Rewrote `planner-web/src/types/blueprint.ts` (226 lines)
  - Fixed: DecisionOption.chosen, Consequence.positive, Assumption.description+confidence,
    ComponentNode.provides/consumes, ConstraintNode.constraint_type+source,
    PatternNode.rationale, QualityRequirementNode without measure/target,
    TechnologyCategory runtime/protocol variants
- [x] A.2 — Fix Rust doc comment shapes (Decision=rounded rect, Constraint=diamond)
  - Fixed lines 11-16 in `planner-schemas/src/artifacts/blueprint.rs`
- [x] A.3 — Add edge DELETE endpoint
  - Added `DELETE /blueprint/edges` handler in `planner-server/src/api.rs`
  - Uses `remove_edges_where` matching source+target+edge_type
  - Added 2 tests: `test_delete_blueprint_edge` + `test_delete_blueprint_edge_not_found`
  - Added `deleteBlueprintEdge()` to frontend API client
- [x] A.4 — Add history GET endpoint
  - Added `GET /blueprint/history` handler + `SnapshotEntry` / `HistoryListResponse` types
  - Added `list_history()` method to `BlueprintStore` in `planner-core/src/blueprint.rs`
  - Returns sorted list of snapshot timestamps (newest first)
  - Added test: `test_list_blueprint_history_empty`
  - Added `listBlueprintHistory()` to frontend API client
- [x] A.5 — Add "Create Node" button + form modal to BlueprintPage
  - Created `CreateNodeModal.tsx` (569 lines) with type-specific form fields
  - All 6 node types supported: Decision, Technology, Component, Constraint, Pattern, Quality Requirement
  - Generates human-readable IDs with UUID8 suffix (per decision #1)
  - Wired into BlueprintPage topbar as "New Node" button
- [x] A.6 — Add node deletion UI with confirmation dialog
  - Created `DeleteNodeDialog.tsx` (103 lines)
  - Delete button in topbar (red trash icon, enabled when node selected)
  - Delete button in DetailDrawer footer (via `onRequestDelete` prop)
  - Confirmation dialog warns about permanent deletion + edge removal
- [x] A.7 — Frontend verified: TypeScript compiles clean, Vite build succeeds, 166/166 tests pass
  - CSS added: `.modal-backdrop`, `.modal-close`, `.field-label`, `.field-input` styles
  - Cargo check/test deferred (no Rust toolchain in sandbox; CI will verify)

### Phase B: Event Sourcing [COMPLETE]
- [x] B.1 — `BlueprintEvent` enum in `planner-schemas/src/artifacts/blueprint.rs`
  - 5 variants: NodeCreated, NodeUpdated, NodeDeleted, EdgeCreated, EdgesDeleted
  - Tagged serde serialization (`#[serde(tag = "event_type", rename_all = "snake_case")]`)
  - `timestamp()` and `summary()` methods on all variants
  - NodeUpdated captures `before` + `after` state for diffing
  - NodeDeleted captures `removed_edges` for full audit trail
- [x] B.2 — EventLog in `planner-core/src/blueprint.rs`
  - Added `events: RwLock<Vec<BlueprintEvent>>` to BlueprintStore
  - Read methods: `events()`, `events_for_node()`, `event_count()`
  - Helpers: `now_iso()`, `append_event()` (with tracing debug)
  - Event emission in all 5 mutation methods:
    - `upsert_node()` → NodeCreated (new) or NodeUpdated (existing)
    - `remove_node()` → NodeDeleted (with incident edges)
    - `update_node()` → NodeUpdated (before/after diff)
    - `add_edge()` → EdgeCreated
    - `remove_edges_where()` → EdgesDeleted (captures removed edges before deletion)
  - Persistence: `load_events()` from `events.msgpack`, `flush_events()` atomic write
  - `flush()` now calls `flush_events()` after nodes+edges flush
- [x] B.3 — API endpoint `GET /blueprint/events`
  - Handler: `list_blueprint_events()` in `planner-server/src/api.rs`
  - Query params: `node_id` (optional filter), `limit` (optional cap)
  - Returns events newest-first with `BlueprintEventsResponse { events, total }`
  - `BlueprintEventPayload` shape: `{ event_type, summary, timestamp, data }`
  - 3 new tests: empty log, events after CRUD, filtered by node_id
- [x] B.4 — Frontend types + API client
  - Added `BlueprintEventType`, `BlueprintEventPayload`, `BlueprintEventsResponse` to `blueprint.ts`
  - Added `listBlueprintEvents({ nodeId?, limit? })` to `client.ts`
  - URLSearchParams-based query string construction
- [x] B.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, code reviewed
  - Cargo check/test deferred to CI (no Rust toolchain in sandbox)
### Phase C: Detail Drawer Editing — PENDING
### Phase C.5: Knowledge & Library Pages — PENDING
### Phase D: Reconvergence Engine — PENDING
### Phase E: Graph UX Polish — PENDING
### Phase F: Lifecycle & History — PENDING
### Phase G: Automated Discovery — PENDING
### Phase H: TUI Blueprint Table — PENDING

## Key Decisions (from GitHub conversation)
1. ✅ NodeId: human-readable slug + UUID8 — implemented in CreateNodeModal.generateId()
2. ✅ Event sourced: full event log with append-only persistence — Phase B complete
3. ⚠️ Reconvergence autonomy: types defined, no execution, needs Phase D
4. ⚠️ One per project: global singleton, OK for now
5. ✅ WebUI primary, TUI table-only

## Files Modified

### Phase A — Rust (backend)
- `planner-schemas/src/artifacts/blueprint.rs` — Doc comment shape fixes (lines 11-16)
- `planner-core/src/blueprint.rs` — Added `list_history()` method to BlueprintStore
- `planner-server/src/api.rs` — Added DELETE /blueprint/edges, GET /blueprint/history,
  SnapshotEntry + HistoryListResponse types, 3 new tests

### Phase A — TypeScript (frontend)
- `planner-web/src/types/blueprint.ts` — FULLY REWRITTEN (226 lines) to match Rust structs
- `planner-web/src/api/client.ts` — Added `deleteBlueprintEdge()` + `listBlueprintHistory()`
- `planner-web/src/components/DetailDrawer.tsx` — REWRITTEN (465 lines) with typed node details,
  Edit/Delete/Impact buttons, `onRequestDelete` prop
- `planner-web/src/components/CreateNodeModal.tsx` — NEW (569 lines) type-specific create form
- `planner-web/src/components/DeleteNodeDialog.tsx` — NEW (103 lines) confirmation dialog
- `planner-web/src/pages/BlueprintPage.tsx` — Added Create/Delete UI, integrated new components
- `planner-web/src/index.css` — Added modal-backdrop, modal-close, field-label, field-input styles

### Phase B — Rust (backend)
- `planner-schemas/src/artifacts/blueprint.rs` — Added `BlueprintEvent` enum (5 variants),
  `timestamp()` + `summary()` methods, tagged serde serialization
- `planner-core/src/blueprint.rs` — Added `events: RwLock<Vec<BlueprintEvent>>` to BlueprintStore;
  read methods (`events()`, `events_for_node()`, `event_count()`); helpers (`now_iso()`, `append_event()`);
  event emission in all 5 mutation methods; persistence (`load_events()`, `flush_events()`)
- `planner-server/src/api.rs` — Added `GET /blueprint/events` route + handler,
  `BlueprintEventsQuery`, `BlueprintEventsResponse`, `BlueprintEventPayload` types,
  3 new tests

### Phase B — TypeScript (frontend)
- `planner-web/src/types/blueprint.ts` — Added `BlueprintEventType`, `BlueprintEventPayload`,
  `BlueprintEventsResponse` (247 lines total)
- `planner-web/src/api/client.ts` — Added `listBlueprintEvents()` method

## Test Results
- Frontend: 166/166 tests passing (11 test files)
- TypeScript: compiles clean (`tsc --noEmit`)
- Vite build: succeeds (production bundle built)
- Rust: cargo check/test deferred to CI (no Rust toolchain in sandbox)
