# Blueprint Implementation — Status Tracker

**Started:** March 5, 2026
**Last Updated:** March 5, 2026 (Phase C complete)

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
### Phase C: Detail Drawer Editing + Edge Creation [COMPLETE]
- [x] C.1 — `EditNodeForm.tsx` component (387 lines)
  - Type-specific inline edit forms for all 6 node types
  - Decision: title, status, context, options (add/remove with pros/cons/chosen), consequences, assumptions, tags
  - Technology: name, category, ring, version, rationale, license, tags
  - Component: name, type, status, description, provides/consumes, tags
  - Constraint: title, type, description, source, tags
  - Pattern: name, description, rationale, tags
  - QualityRequirement: attribute, scenario, priority, tags
  - Auto-updates `updated_at` timestamp on every field change
  - Error handling with display, save/cancel actions
- [x] C.2 — Edit mode toggle in DetailDrawer
  - View ↔ Edit toggle: "Edit" button switches to EditNodeForm
  - Footer buttons (Edit/Delete/Impact) hidden during edit mode
  - Save calls `PATCH /blueprint/nodes/:id` (full replacement)
  - Cancel returns to view mode without changes
  - Edit mode resets when navigating to a different node
  - `onNodeUpdated` callback triggers BlueprintPage re-fetch
- [x] C.3 — `AddEdgeModal.tsx` component (175 lines)
  - Source/target node dropdowns sorted alphabetically with type prefix badges
  - Edge type selector with all 8 types + descriptions
  - Optional metadata field
  - Visual edge preview: "Source —[type]→ Target"
  - Validation: source ≠ target, both required
  - "Add Edge" button in BlueprintPage topbar
  - Pre-fills source from selected node
- [x] C.4 — CSS: `.edit-node-form`, `.edit-node-form-body`, `.edit-node-form-actions`
- [x] C.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
  - Cargo check/test deferred to CI (no Rust toolchain in sandbox)
### Phase C.5: Knowledge & Library Pages [COMPLETE]
- [x] C.5.1 — `NodeListPanel.tsx` reusable component (293 lines)
  - Filterable, searchable, sortable node list with completeness scoring
  - Configurable columns, search bar, summary stats
  - Shared between KnowledgeLibraryPage (and future pages)
- [x] C.5.2 — `KnowledgeLibraryPage.tsx` (196 lines)
  - Tabbed page: All / Decisions / Technologies / Components / Constraints / Patterns / Quality
  - Count badges per tab, summary statistics bar
  - Integrates DetailDrawer for node inspection
- [x] C.5.3 — Routing + Navigation
  - Added lazy-loaded `/knowledge` route in App.tsx with ProtectedRoute wrapper
  - Added 'Knowledge' item with book icon to sidebar in Layout.tsx
- [x] C.5.4 — CSS: `.knowledge-page`, `.knowledge-header`, `.knowledge-tabs`, `.knowledge-tab`,
  `.node-list-panel`, `.node-list-toolbar`, `.node-list-search`
  - Also added preemptive reconvergence CSS (`.recon-panel`, `.recon-step`, `.recon-summary`)
- [x] C.5.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
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
5. ✅ WebUI primary, TUI table-only — full CRUD (create/edit/delete nodes + edges) in Phase C

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

### Phase C — TypeScript (frontend)
- `planner-web/src/components/EditNodeForm.tsx` — NEW (387 lines) type-specific inline edit forms
- `planner-web/src/components/AddEdgeModal.tsx` — NEW (175 lines) edge creation modal
- `planner-web/src/components/DetailDrawer.tsx` — Added edit mode toggle, EditNodeForm integration,
  `onNodeUpdated` prop, edit/view state management
- `planner-web/src/pages/BlueprintPage.tsx` — Added `handleCreateEdge`, `addEdgeModalOpen` state,
  "Add Edge" topbar button, `onNodeUpdated={loadBlueprint}` callback, AddEdgeModal render
- `planner-web/src/index.css` — Added `.edit-node-form`, `.edit-node-form-body`, `.edit-node-form-actions`

### Phase C.5 — TypeScript (frontend)
- `planner-web/src/components/NodeListPanel.tsx` — NEW (293 lines) reusable filterable/sortable list
- `planner-web/src/pages/KnowledgeLibraryPage.tsx` — NEW (196 lines) tabbed knowledge page
- `planner-web/src/App.tsx` — Added lazy-loaded `/knowledge` route
- `planner-web/src/components/Layout.tsx` — Added 'Knowledge' sidebar item + book icon
- `planner-web/src/index.css` — Added knowledge + reconvergence CSS classes

## Test Results
- Frontend: 166/166 tests passing (11 test files)
- TypeScript: compiles clean (`tsc --noEmit`)
- Vite build: succeeds (production bundle built)
- Rust: cargo check/test deferred to CI (no Rust toolchain in sandbox)
