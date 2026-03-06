# Blueprint Implementation — Status Tracker

**Started:** March 5, 2026
**Last Updated:** March 5, 2026 (Phase E complete)

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
### Phase D: Reconvergence Engine [COMPLETE]
- [x] D.1 — Reconvergence types in `blueprint.ts`
  - `ReconvergenceStepStatus` ('pending' | 'running' | 'done' | 'skipped' | 'error')
  - `ReconvergenceStep` interface (step_id, node_id, action, severity, status, error)
  - `ReconvergenceRequest` (source_node_id, impact_report, auto_apply)
  - `ReconvergenceResult` (steps, summary with total/applied/skipped/errors/needs_review, timestamp)
- [x] D.2 — `POST /blueprint/reconverge` endpoint in `api.rs`
  - `ReconvergeRequest`, `ReconvergeStepResponse`, `ReconvergeSummary`, `ReconvergeResponse` types
  - Policy: auto_apply=true → shallow/medium auto-accepted ("done"), deep requires review ("pending")
  - Verifies source node exists, iterates impact entries, applies severity-based policy
  - Added `reconvergeBlueprint()` method to frontend API client
- [x] D.3 — `ReconvergencePanel.tsx` component (297 lines)
  - Status icons: animated spinner (running), checkmark (done), warning (pending), X (skipped/error)
  - Severity badges with color coding (shallow=faint, medium=warning, deep=error)
  - Step rows with action tags, descriptions, and approve/skip controls for deep-severity pending steps
  - Summary bar with total/applied/review/skipped/errors counts
  - Optimistic local state overrides for approve/skip actions
  - Uses preemptive CSS from Phase C.5 (`.recon-panel`, `.recon-step`, `.recon-summary`)
- [x] D.4 — Wired "Apply & Reconverge" button in BlueprintPage
  - `handleImpactApply` captures impact report, closes impact modal, opens reconvergence panel
  - Calls `api.reconvergeBlueprint()` with auto_apply=true (per decision #3)
  - `handleReconClose` refreshes blueprint data after reconvergence completes
  - State: `reconResult`, `reconLoading`, `reconVisible`
- [x] D.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
  - Cargo check/test deferred to CI (no Rust toolchain in sandbox)
### Phase E: Graph & Visualization Polish [COMPLETE]
- [x] E.1 — Pre-bake simulation
  - Run `sim.tick(N)` before first render (N = min(300, 100 + nodeCount*8))
  - Positions nodes + edges at pre-baked coordinates immediately
  - Restart with low alpha (0.1) for interactive settling + drag
  - Result: graph appears stable on first paint instead of animating from origin
- [x] E.2 — Adaptive charge strength
  - Scale repulsion based on node count: ≤8 nodes → -1200, ≤20 → -1400, ≤50 → -1800, >50 → -2400
  - Link distance scales: ≤8 → 180px, ≤20 → 220px, else 260px
  - Type-force strength scales: ≤8 → 0.12, else 0.15
- [x] E.3 — Minimap
  - 160×110px overview in top-right corner with semi-transparent background
  - Color-coded dots for each node (matches node type colors)
  - Viewport rectangle showing current zoom/pan extent
  - Updates on every simulation tick and zoom event
  - Scale-to-fit with 60px padding, centered within minimap bounds
- [x] E.4 — Neighborhood focus mode
  - Double-click node to show only 1-hop neighbors (all others fade to 8% opacity)
  - Connected edges highlighted at 85% opacity, others at 3%
  - Double-click again or click background to clear focus
  - Disabled d3 dblclick-to-zoom to prevent conflict
  - Coexists with type filter (focus takes priority when active)
- [x] E.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
### Phase F: Lifecycle & History — PENDING
### Phase G: Automated Discovery — PENDING
### Phase H: TUI Blueprint Table — PENDING

## Key Decisions (from GitHub conversation)
1. ✅ NodeId: human-readable slug + UUID8 — implemented in CreateNodeModal.generateId()
2. ✅ Event sourced: full event log with append-only persistence — Phase B complete
3. ✅ Reconvergence autonomy: auto-accept shallow/medium, review deep — Phase D complete
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

### Phase D — Rust (backend)
- `planner-server/src/api.rs` — Added `POST /blueprint/reconverge` route + handler,
  `ReconvergeRequest`, `ReconvergeStepResponse`, `ReconvergeSummary`, `ReconvergeResponse` types,
  severity-based auto-apply policy (shallow/medium=done, deep=pending)

### Phase D — TypeScript (frontend)
- `planner-web/src/types/blueprint.ts` — Added `ReconvergenceStepStatus`, `ReconvergenceStep`,
  `ReconvergenceRequest`, `ReconvergenceResult` (283 lines total)
- `planner-web/src/api/client.ts` — Added `reconvergeBlueprint()` method
- `planner-web/src/components/ReconvergencePanel.tsx` — NEW (297 lines) reconvergence progress panel
  with status icons, severity badges, approve/skip controls, optimistic state
- `planner-web/src/pages/BlueprintPage.tsx` — Wired reconvergence: `handleImpactApply` calls API,
  `reconResult`/`reconLoading`/`reconVisible` state, renders ReconvergencePanel

### Phase E — TypeScript (frontend)
- `planner-web/src/components/BlueprintGraph.tsx` — Pre-bake simulation (E.1), adaptive charge (E.2),
  minimap with viewport rect (E.3), neighborhood focus dblclick mode (E.4)

## Test Results
- Frontend: 166/166 tests passing (11 test files)
- TypeScript: compiles clean (`tsc --noEmit`)
- Vite build: succeeds (production bundle built)
- Rust: cargo check/test deferred to CI (no Rust toolchain in sandbox)
