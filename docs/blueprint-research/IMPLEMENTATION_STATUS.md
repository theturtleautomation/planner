# Blueprint Implementation — Status Tracker

**Started:** March 5, 2026
**Last Updated:** March 5, 2026 (Gap audit + Phase G + Phase H plan)

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
- [x] C.5.6 — Completeness indicators (approximate — scores based on NodeSummary generic fields;
  type-specific field depth requires server-side enrichment or full-node fetch)
- [—] C.5.7 — DEFERRED: Attach documentation to any node (markdown body rendered in drawer)
  - Requires new `documentation` field in node types + schema extension + storage support
  - Blocked on Rust schema + persistence changes (same category as C.4, D.3, G.6)
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
### Phase F: Lifecycle & History [COMPLETE]
- [x] F.1 — Per-node event timeline tab in DetailDrawer
  - Added "Details" / "History" tab bar below drawer header (only in view mode)
  - History tab fetches events via `api.listBlueprintEvents({ nodeId })` on activation
  - Vertical timeline with color-coded dots per event type (created=green, updated=blue, deleted=red, edge=warning)
  - Each event shows: type badge, timestamp, summary, expandable JSON details
  - Lazy loading — events only fetched when History tab is selected
  - Tab and event state reset when navigating to a different node
- [x] F.2 — Global `EventTimelinePage.tsx` (171 lines)
  - Full-page event log at `/events` route with sidebar navigation (activity icon)
  - Filter chips: All Events, Created, Updated, Deleted, Edge Created, Edges Deleted
  - Configurable limit selector: Last 50 / 100 / 250 / 500
  - Relative timestamps (just now, Xm ago, Xh ago, Xd ago)
  - Refresh button, error/empty states, loading skeleton
  - Lazy-loaded route with ProtectedRoute wrapper
- [x] F.3 — Stale node detection + orphan node detection
  - **NodeListPanel**: "stale" badge (yellow) on nodes not updated in 30+ days,
    "orphan" badge (red) on nodes with zero edges — both in Name column
  - **BlueprintGraph**: health indicator dots at top-right of node shapes
    (yellow dot = stale, red dot = orphan) with title tooltips
  - CSS: `.health-badge`, `.health-stale`, `.health-orphan` classes
- [x] F.4 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
### Phase G: Automated Discovery [FRONTEND COMPLETE — RUST BACKEND PENDING]
- [x] G.1 — Discovery types in `blueprint.ts`
  - `DiscoverySource` ('cargo_toml' | 'directory_scan' | 'pipeline_run' | 'manual')
  - `ProposalStatus` ('pending' | 'accepted' | 'rejected' | 'merged')
  - `ProposedNode` interface (id, node, source, reason, status, confidence, source_artifact)
  - `DiscoveryScanRequest`, `DiscoveryScanResult`, `DiscoveryRunResponse`, `ProposedNodesResponse`
- [x] G.2 — API client methods in `client.ts`
  - `runDiscoveryScan(req)` → `POST /blueprint/discovery/scan`
  - `listProposedNodes(status?)` → `GET /blueprint/discovery/proposals`
  - `acceptProposal(id)` → `POST /blueprint/discovery/proposals/:id/accept`
  - `rejectProposal(id, reason?)` → `POST /blueprint/discovery/proposals/:id/reject`
- [x] G.3 — `DiscoveryPage.tsx` (358 lines)
  - "Run Discovery Scan" button triggers all scanners
  - Scan feedback: success summary or error message
  - Status filter chips: All, Pending, Accepted, Rejected, Merged
  - Proposal cards: source icon, node name + type badge, confidence %, status, relative time
  - Expandable detail: source artifact path, full node data JSON, Accept/Reject buttons
  - Pending proposals highlighted with warning border
- [x] G.4 — Route + navigation
  - Added lazy-loaded `/discovery` route in App.tsx with ProtectedRoute wrapper
  - Added 'Discovery' item with search icon to sidebar in Layout.tsx
  - SidebarIcon: added 'search' SVG icon variant
- [x] G.5 — Verification: `tsc --noEmit` clean, 166/166 vitest passing, Vite build succeeds
- [ ] G.6 — DEFERRED: Rust backend implementation
  - Cargo.toml scanner: parse `[dependencies]` → proposed TechnologyNodes
  - Directory structure scanner: detect modules/services → proposed ComponentNodes
  - Pipeline run scanner: extract detected patterns/constraints from LLM pipeline
  - Proposed nodes persistence: append-only store in `data_dir/blueprint/proposals/`
  - REST handlers: scan trigger, proposal CRUD, accept (creates real node + event)

### Phase H: TUI Blueprint Table [PLAN DOCUMENTED — RUST IMPLEMENTATION PENDING]
- [x] H.1 — Detailed implementation plan in `planner-tui/src/blueprint_table.rs`
  - Architecture: split-pane layout (node table left, detail right)
  - App state extension: `AppView` enum, `BlueprintTableState` struct
  - Key bindings: j/k navigation, Enter toggle detail, / search, t type filter, q return
  - Render function: `ratatui::Table` with header, type-colored cells, highlight style
  - Node detail pane: full typed field display, edge listing
  - Data loading: from `BlueprintStore::snapshot()`
  - View switching: Ctrl+B to toggle between Socratic and Blueprint views
  - Testing plan: filtered_nodes, navigation bounds, type filter cycling
- [ ] H.2 — DEFERRED: Rust implementation (requires Rust toolchain)
  - Implement `BlueprintTableState` in `planner-tui/src/app.rs`
  - Implement `render_blueprint_table()` in `planner-tui/src/ui.rs`
  - Add key handling for `AppView::Blueprint` mode
  - Wire `Ctrl+B` view toggle
  - Add unit + integration tests

## Key Decisions (from GitHub conversation)
1. ✅ NodeId: human-readable slug + UUID8 — implemented in CreateNodeModal.generateId()
2. ✅ Event sourced: full event log with append-only persistence — Phase B complete
3. ✅ Reconvergence autonomy: auto-accept shallow/medium, review deep — Phase D complete
4. ⚠️ One per project: global singleton, OK for now
5. ✅ WebUI primary, TUI table-only — full CRUD (create/edit/delete nodes + edges) in Phase C

## Gap Audit Fixes [COMPLETE]

After Phase F, a thorough gap audit identified 7 items from the roadmap not yet
implemented. 5 were frontend-implementable; 2 were deferred as Rust-only.

- [x] Gap 1 — Global search bar on BlueprintPage
  - Search input in topbar filters all views (graph, table) via `filteredBlueprint` memo
  - Searches node names, IDs, descriptions, statuses
  - CSS: `.global-search-bar` + `.global-search-input`
- [x] Gap 2 — Edge annotations ("why" metadata)
  - Edge metadata displayed in DetailDrawer connections as `.edge-annotation` spans
  - AddEdgeModal metadata label renamed to "Why this relationship?"
- [x] Gap 3 — Hierarchical layout toggle
  - Installed `@dagrejs/dagre` dependency
  - Added dagre layout computation in BlueprintGraph (top-to-bottom layering)
  - Layout toggle button in BlueprintPage topbar (Force / Hierarchy modes)
  - `layoutMode` prop passed to BlueprintGraph
- [x] Gap 4 — Snapshots UI in EventTimelinePage
  - Events / Snapshots tab bar
  - Snapshot list with file icon, timestamp, filename, relative age
  - `createBlueprintSnapshot()` API client method
  - CSS: `.snapshot-list`, `.snapshot-item`, `.snapshot-icon`, `.snapshot-info`,
    `.snapshot-timestamp`, `.snapshot-filename`, `.snapshot-age`
- [x] Gap 5 — Diff view for node_updated events
  - Before/after side-by-side display in History tab (DetailDrawer)
  - Before/after side-by-side display in global EventTimelinePage
  - Extracts `before` and `after` from `evt.data`, computes changed keys
  - CSS: `.diff-view`, `.diff-panel`, `.diff-panel-before`, `.diff-panel-after`,
    `.diff-panel-header`, `.diff-row`, `.diff-key`, `.diff-value`,
    `.diff-changed`, `.diff-added`, `.diff-removed`
- [—] Gap 6 — Partial PATCH (JSON Merge Patch) — DEFERRED (Rust server change)
- [—] Gap 7 — WebSocket streaming for reconvergence — DEFERRED (Rust server change)

## Files Modified

### Gap Audit Fixes
- `planner-web/src/pages/BlueprintPage.tsx` — Global search bar, filteredBlueprint memo, layout toggle
- `planner-web/src/components/BlueprintGraph.tsx` — dagre import, layoutMode prop, hierarchical layout
- `planner-web/src/components/DetailDrawer.tsx` — Edge annotations, diff view in History tab
- `planner-web/src/components/AddEdgeModal.tsx` — Metadata label rename
- `planner-web/src/pages/EventTimelinePage.tsx` — Snapshots tab, diff view, JSX fragment fix
- `planner-web/src/api/client.ts` — `createBlueprintSnapshot()` method
- `planner-web/src/index.css` — Snapshot + diff CSS classes
- `planner-web/package.json` — `@dagrejs/dagre` dependency

### Phase G — TypeScript (frontend)
- `planner-web/src/types/blueprint.ts` — Discovery/proposed types (DiscoverySource, ProposalStatus, etc.)
- `planner-web/src/api/client.ts` — 4 discovery API methods
- `planner-web/src/pages/DiscoveryPage.tsx` — NEW (358 lines) discovery review queue
- `planner-web/src/App.tsx` — Added `/discovery` route
- `planner-web/src/components/Layout.tsx` — Added Discovery nav + search icon

### Phase H — Rust (plan/scaffold)
- `planner-tui/src/blueprint_table.rs` — NEW (181 lines) implementation plan + scaffold

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

### Phase F — TypeScript (frontend)
- `planner-web/src/components/DetailDrawer.tsx` — Added History tab (tab bar, event state, event fetch,
  timeline view with dots/badges/details), import BlueprintEventPayload type
- `planner-web/src/pages/EventTimelinePage.tsx` — NEW (171 lines) global event log page with
  filter chips, limit selector, relative timestamps, refresh, error/empty states
- `planner-web/src/components/NodeListPanel.tsx` — Stale/orphan detection badges in Name column
  (STALE_THRESHOLD_DAYS=30, edge-count check for orphans)
- `planner-web/src/components/BlueprintGraph.tsx` — Health indicator dots on graph nodes
  (stale=warning, orphan=error circles at top-right of node shapes)
- `planner-web/src/App.tsx` — Added lazy-loaded `/events` route with ProtectedRoute
- `planner-web/src/components/Layout.tsx` — Added 'Events' sidebar item + activity icon
- `planner-web/src/index.css` — Added drawer-tabs, event-timeline, global-event-timeline,
  event-filters, health-badge CSS classes

## Test Results
- Frontend: 166/166 tests passing (11 test files)
- TypeScript: compiles clean (`tsc --noEmit`)
- Vite build: succeeds (production bundle built)
- Rust: cargo check/test deferred to CI (no Rust toolchain in sandbox)

## Code Audit Fixes (Post-Phase H)
After a comprehensive deep-read audit of all frontend source files (A–H),
7 issues were identified and all fixable items resolved:

- [x] **BUG-1 (Critical):** `edgeData` → `edges` in BlueprintGraph.tsx line 352 — runtime crash fix
- [x] **ISSUE-2 (Medium):** AddEdgeModal `useState` patterns → proper `useEffect` for form reset
- [x] **ISSUE-3 (Medium):** Added "Create Snapshot" button to EventTimelinePage snapshots tab
- [x] **ISSUE-4 (Medium):** Improved completeness scoring + documented NodeSummary limitation
- [x] **ISSUE-5 (Medium):** Tracked C.5.7 as deferred in this document (was previously omitted)
- [x] **NOTE-6 (Low):** RadarView `inferRing()` now checks `node.status` before tags fallback
- [x] **NOTE-7 (Low):** Added `supersedes` to EDGE_STYLES legend + `edgeDash()` function
- Full audit report: `docs/blueprint-research/PHASE_AUDIT_REPORT.md`
