# Deferred Rust-Only Features — Implementation Specifications

**Created:** March 5, 2026
**Status:** Pending — requires Rust toolchain
**Prerequisite:** All frontend phases (A–H) complete, 166/166 tests passing, codebase clean

These 5 features were identified during phases C, D, G, and H but could not be implemented
in the sandbox environment (no Rust toolchain). Each specification below includes the exact
files to modify, the current state of the code, the implementation approach, acceptance
criteria, and test plan.

---

## Summary Table

| ID    | Feature                              | Phase | Files to Modify                                            | Complexity |
|-------|--------------------------------------|-------|------------------------------------------------------------|------------|
| C.4   | JSON Merge Patch for partial updates | C     | `planner-server/src/api.rs`                                | Low        |
| C.5.7 | Attach documentation to nodes        | C.5   | `planner-schemas/.../blueprint.rs`, `api.rs`, frontend TS  | Medium     |
| D.3   | WebSocket streaming for reconvergence| D     | `planner-server/src/api.rs`                                | Medium     |
| G.6   | Rust backend discovery scanners      | G     | `planner-server/src/api.rs`, new `discovery.rs` module     | High       |
| H.2   | TUI Blueprint Table                  | H     | `planner-tui/src/{app,ui,blueprint_table}.rs`              | Medium     |

---

## C.4 — JSON Merge Patch for Partial Node Updates

### Current State

The PATCH endpoint (`update_blueprint_node` at line 1175 of `api.rs`) performs **full replacement**:

```rust
/// PATCH /blueprint/nodes/{nodeId} — Replace a node (full replacement under its ID).
///
/// For v1, PATCH does a full replacement of the node at the given ID.
/// The incoming payload must be a complete BlueprintNode with the same ID.
async fn update_blueprint_node(
    State(state): State<Arc<AppState>>,
    _claims: Claims,
    Path(node_id): Path<String>,
    Json(node): Json<planner_schemas::artifacts::blueprint::BlueprintNode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
```

The frontend `EditNodeForm.tsx` currently sends the **entire node** on every save. This works
but is inefficient and prone to race conditions if two users edit different fields concurrently.

### Implementation Plan

1. **Add dependency:** `json-patch = "2"` or implement RFC 7396 JSON Merge Patch manually
   (merge patch is simpler than JSON Patch — just deep-merge a partial JSON object).

2. **Modify `update_blueprint_node`:**
   - Accept `Json(patch): Json<serde_json::Value>` instead of a typed `BlueprintNode`
   - Load the existing node: `state.blueprints.get_node(&node_id)`
   - Serialize existing node to `serde_json::Value`
   - Apply merge patch: for each key in the patch object, overwrite the corresponding key
     in the existing node (nulls remove fields per RFC 7396)
   - Deserialize the merged result back to `BlueprintNode` (validates the shape)
   - If deserialization fails, return `400 Bad Request` with the serde error
   - Call `state.blueprints.update_node()` with the merged node
   - Emit `NodeUpdated` event with before/after

3. **Preserve backward compatibility:**
   - Sending a complete node still works (it's a valid merge patch)
   - The frontend can incrementally adopt partial sends

### Acceptance Criteria

- `PATCH /blueprint/nodes/:id` with `{"tags": ["new-tag"]}` merges into the existing node
- `PATCH` with a complete node still works identically to current behavior
- Invalid partial payloads (e.g., `{"status": "bogus"}`) return 400
- `NodeUpdated` event captures correct before/after diff
- Existing tests continue to pass (they send full nodes)

### New Tests

```rust
#[tokio::test]
async fn test_partial_patch_merges_tags() { /* send only tags, verify other fields unchanged */ }

#[tokio::test]
async fn test_partial_patch_invalid_field_returns_400() { /* send invalid enum value */ }

#[tokio::test]
async fn test_full_replacement_still_works() { /* existing behavior preserved */ }
```

---

## C.5.7 — Attach Documentation to Any Node

### Current State

No `documentation` field exists on any node type. The 6 node structs in
`planner-schemas/src/artifacts/blueprint.rs` (lines 282–377) have no markdown body field.
The frontend `DetailDrawer.tsx` has no documentation tab or markdown rendering.
The `EditNodeForm.tsx` has no documentation textarea.

### Implementation Plan

#### Step 1: Schema extension (`planner-schemas/src/artifacts/blueprint.rs`)

Add to **all 6 node structs** (Decision, Technology, Component, Constraint, Pattern,
QualityRequirement):

```rust
/// Freeform markdown documentation attached to this node.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub documentation: Option<String>,
```

This field is optional with `skip_serializing_if` so existing serialized data (MessagePack
files) will deserialize correctly — missing fields default to `None`.

#### Step 2: NodeSummary enrichment (`planner-core/src/blueprint.rs`)

Add `has_documentation: bool` to `NodeSummary` so the frontend can show a docs icon
without fetching the full node:

```rust
pub struct NodeSummary {
    // ... existing fields ...
    pub has_documentation: bool,
}
```

Update the `summary()` method on `BlueprintNode` to populate this from
`self.documentation().is_some()`.

#### Step 3: Frontend types (`planner-web/src/types/blueprint.ts`)

Add `documentation?: string` to all 6 node type interfaces and `has_documentation: boolean`
to `NodeSummary`.

#### Step 4: DetailDrawer UI (`planner-web/src/components/DetailDrawer.tsx`)

Add a third tab: **Details | History | Docs**
- Docs tab renders the markdown body (use a simple markdown renderer or `dangerouslySetInnerHTML`
  with a sanitizer)
- Show "No documentation yet" placeholder when empty

#### Step 5: EditNodeForm UI (`planner-web/src/components/EditNodeForm.tsx`)

Add a `<textarea>` for the documentation field at the bottom of every node type's edit form.
Label: "Documentation (markdown)". Optional field.

#### Step 6: NodeListPanel indicator

Show a small "docs" icon/badge in `NodeListPanel.tsx` when `has_documentation` is true.

### Acceptance Criteria

- All 6 node types accept and persist a `documentation` field
- Existing nodes without documentation load correctly (field defaults to `None`)
- Documentation renders as formatted markdown in the drawer
- Documentation is editable via the edit form
- NodeListPanel shows which nodes have docs attached
- MessagePack backward compatibility maintained (new optional field)

### New Tests

```rust
#[test]
fn node_with_documentation_roundtrip() { /* serialize/deserialize with docs field */ }

#[test]
fn node_without_documentation_defaults_to_none() { /* backward compat */ }

#[tokio::test]
async fn test_create_node_with_documentation() { /* POST with docs field, verify GET returns it */ }

#[tokio::test]
async fn test_patch_documentation_only() { /* partial PATCH to add/update docs (depends on C.4) */ }
```

### Frontend Tests

```ts
it('renders documentation tab in DetailDrawer', () => { /* ... */ })
it('shows documentation textarea in EditNodeForm', () => { /* ... */ })
it('shows docs badge in NodeListPanel when has_documentation is true', () => { /* ... */ })
```

---

## D.3 — WebSocket Streaming for Reconvergence Progress

### Current State

The reconvergence endpoint (`POST /blueprint/reconverge` at line 1385 of `api.rs`) is
**synchronous** — it processes all impact entries in a single request/response cycle and
returns the full `ReconvergeResponse` at the end. The frontend `ReconvergencePanel.tsx`
receives the complete result in one shot.

The server already has WebSocket infrastructure for Socratic interviews
(`ws_handler` at line 979, `socratic_ws_handler` at line 1050). The axum `WebSocketUpgrade`
extractor is already imported (line 7).

### Implementation Plan

#### Step 1: New WebSocket endpoint

Add route: `.route("/blueprint/reconverge/ws", get(reconverge_ws_handler))`

```rust
async fn reconverge_ws_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_reconverge_ws(socket, state))
}
```

#### Step 2: WebSocket message protocol

```rust
// Client → Server (first message)
#[derive(Deserialize)]
struct ReconvergeWsRequest {
    source_node_id: String,
    impact_report: ImpactReport,
    auto_apply: bool,
}

// Server → Client (per-step progress)
#[derive(Serialize)]
#[serde(tag = "type")]
enum ReconvergeWsMessage {
    #[serde(rename = "step")]
    Step(ReconvergeStepResponse),
    #[serde(rename = "summary")]
    Summary(ReconvergeSummary),
    #[serde(rename = "error")]
    Error { message: String },
}
```

#### Step 3: Handler implementation

```rust
async fn handle_reconverge_ws(mut socket: WebSocket, state: Arc<AppState>) {
    // 1. Receive the request message
    // 2. Validate source node exists
    // 3. For each impact entry:
    //    a. Process the step (apply severity policy)
    //    b. Send a Step message immediately
    //    c. Small delay (50ms) for UI animation
    // 4. Send Summary message
    // 5. Close the socket
}
```

#### Step 4: Frontend integration

Update `ReconvergencePanel.tsx` to optionally use WebSocket:
- Open WebSocket to `/api/blueprint/reconverge/ws`
- Send the request as the first message
- Receive step messages one-by-one, updating the UI progressively
- On summary message, finalize the display
- Fallback to the existing REST endpoint if WebSocket fails

Update `client.ts`:
```ts
export function reconvergeBlueprintWs(
  req: ReconvergenceRequest,
  onStep: (step: ReconvergenceStep) => void,
  onComplete: (summary: ReconvergenceSummary) => void,
  onError: (error: string) => void,
): WebSocket { /* ... */ }
```

### Acceptance Criteria

- WebSocket endpoint accepts reconvergence requests
- Steps stream to the client one at a time (not batched)
- Frontend shows each step appearing progressively with animation
- Summary message closes the stream cleanly
- REST `POST /blueprint/reconverge` endpoint remains functional (backward compat)
- Error conditions (invalid node, malformed request) send error message and close socket

### New Tests

```rust
#[tokio::test]
async fn test_reconverge_ws_streams_steps() { /* connect, send request, verify step-by-step messages */ }

#[tokio::test]
async fn test_reconverge_ws_invalid_node_sends_error() { /* verify error message for missing node */ }
```

---

## G.6 — Rust Backend Discovery Scanners

### Current State

The frontend is complete (G.1–G.5): types, API client methods, `DiscoveryPage.tsx`, routing.
The API client calls these endpoints which **do not yet exist** on the server:

- `POST /blueprint/discovery/scan` — trigger discovery scan
- `GET /blueprint/discovery/proposals` — list proposed nodes
- `POST /blueprint/discovery/proposals/:id/accept` — accept a proposal
- `POST /blueprint/discovery/proposals/:id/reject` — reject a proposal

No discovery-related routes exist in `api.rs` (confirmed by grep). No proposal storage exists.

### Implementation Plan

#### Step 1: Proposal storage (`planner-core/src/blueprint.rs` or new `discovery.rs`)

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedNode {
    pub id: String,                    // UUID for the proposal itself
    pub node: BlueprintNode,           // The proposed node data
    pub source: DiscoverySource,
    pub reason: String,
    pub status: ProposalStatus,
    pub confidence: f32,               // 0.0–1.0
    pub source_artifact: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoverySource {
    CargoToml,
    DirectoryScan,
    PipelineRun,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
    Merged,
}

pub struct ProposalStore {
    dir: PathBuf,  // data_dir/blueprint/proposals/
    proposals: RwLock<Vec<ProposedNode>>,
}
```

Persistence: append-only MessagePack file at `data_dir/blueprint/proposals.msgpack`,
following the same atomic write-then-rename pattern as `BlueprintStore`.

#### Step 2: Cargo.toml scanner

```rust
pub fn scan_cargo_toml(project_root: &Path) -> Vec<ProposedNode> {
    // 1. Find all Cargo.toml files under project_root
    // 2. Parse [dependencies] and [dev-dependencies] sections
    // 3. For each dependency:
    //    a. Create a proposed TechnologyNode with:
    //       - name: crate name
    //       - version: from version spec
    //       - category: infer from well-known crates (serde→library, tokio→runtime, etc.)
    //       - ring: AdoptionRing::Adopt (it's in use)
    //       - confidence: 0.9 (high — directly from manifest)
    //    b. Deduplicate against existing blueprint nodes by name
    // 4. Return proposals
}
```

#### Step 3: Directory scanner

```rust
pub fn scan_directory_structure(project_root: &Path) -> Vec<ProposedNode> {
    // 1. Scan for conventional project structures:
    //    - `src/` subdirectories → potential Component nodes
    //    - `crates/` or workspace members → Component nodes
    //    - Common patterns: `api/`, `core/`, `web/`, `cli/`, `db/`
    // 2. For each detected module:
    //    a. Create a proposed ComponentNode
    //    b. Infer component_type from directory name
    //    c. confidence: 0.6 (medium — heuristic-based)
    // 3. Return proposals
}
```

#### Step 4: API handlers (`planner-server/src/api.rs`)

Add routes:
```rust
.route("/blueprint/discovery/scan", post(run_discovery_scan))
.route("/blueprint/discovery/proposals", get(list_proposals))
.route("/blueprint/discovery/proposals/{id}/accept", post(accept_proposal))
.route("/blueprint/discovery/proposals/{id}/reject", post(reject_proposal))
```

Request/response types:
```rust
#[derive(Deserialize)]
struct DiscoveryScanRequest {
    sources: Vec<String>,  // ["cargo_toml", "directory_scan"]
    project_path: Option<String>,
}

#[derive(Serialize)]
struct DiscoveryScanResponse {
    run_id: String,
    proposals_found: usize,
    sources_scanned: Vec<String>,
    timestamp: String,
}

#[derive(Serialize)]
struct ProposalListResponse {
    proposals: Vec<ProposedNode>,
    total: usize,
}
```

#### Step 5: Accept flow

When a proposal is accepted:
1. Change proposal status to `Accepted`
2. Call `state.blueprints.upsert_node(proposal.node)` to create the real node
3. Emit a `NodeCreated` event with source metadata
4. Change proposal status to `Merged`
5. Persist updated proposals

#### Step 6: AppState extension

Add `proposals: ProposalStore` to `AppState` in `main.rs`.

### Acceptance Criteria

- `POST /blueprint/discovery/scan` with `sources: ["cargo_toml"]` scans the project's
  Cargo.toml files and creates proposals
- `GET /blueprint/discovery/proposals` returns all proposals, filterable by `?status=pending`
- Accepting a proposal creates a real blueprint node + event
- Rejecting a proposal marks it rejected with an optional reason
- Proposals persist across server restarts
- Existing frontend (DiscoveryPage.tsx) works without modification

### New Tests

```rust
#[test]
fn scan_cargo_toml_finds_dependencies() { /* parse a test Cargo.toml, verify proposals */ }

#[test]
fn scan_directory_finds_components() { /* create temp dir structure, verify proposals */ }

#[tokio::test]
async fn test_discovery_scan_endpoint() { /* POST scan, verify response */ }

#[tokio::test]
async fn test_accept_proposal_creates_node() { /* accept proposal, verify node exists */ }

#[tokio::test]
async fn test_reject_proposal() { /* reject, verify status change */ }

#[tokio::test]
async fn test_list_proposals_filter_by_status() { /* filter pending vs all */ }
```

---

## H.2 — TUI Blueprint Table Implementation

### Current State

A detailed implementation plan exists in `planner-tui/src/blueprint_table.rs` (181 lines of
doc comments with architecture diagram, state structs, key bindings, render functions, data
loading, and testing plan). No executable Rust code exists yet.

The TUI crate (`planner-tui`) already uses `ratatui` and `crossterm` — no new dependencies needed.

### Implementation Plan

The full plan is documented in `planner-tui/src/blueprint_table.rs`. Summary of work:

#### Step 1: App state extension (`planner-tui/src/app.rs`)

```rust
pub enum AppView {
    Socratic,    // existing default
    Blueprint,   // new view
}

pub struct BlueprintTableState {
    pub nodes: Vec<NodeSummary>,
    pub edges: Vec<EdgePayload>,
    pub selected: usize,
    pub filter: String,
    pub type_filter: Option<String>,
    pub table_state: ratatui::widgets::TableState,
    pub detail_expanded: bool,
    pub detail_node: Option<BlueprintNode>,
}

impl BlueprintTableState {
    pub fn filtered_nodes(&self) -> Vec<&NodeSummary> { /* filter by search + type */ }
    pub fn move_up(&mut self) { /* clamp to 0 */ }
    pub fn move_down(&mut self) { /* clamp to len-1 */ }
    pub fn cycle_type_filter(&mut self) { /* None → decision → technology → ... → None */ }
}
```

#### Step 2: Key handling (`app.rs::handle_key`)

| Key       | Action                                    |
|-----------|-------------------------------------------|
| `j`/`↓`   | Move selection down                      |
| `k`/`↑`   | Move selection up                        |
| `Enter`   | Toggle detail pane for selected node     |
| `/`       | Enter search mode (filter by name/id)    |
| `Esc`     | Clear search / exit blueprint view       |
| `t`       | Cycle type filter (all → decision → …)   |
| `Tab`     | Toggle focus: table ↔ detail pane        |
| `q`       | Return to Socratic view                  |
| `g`/`Home`| Jump to top                              |
| `G`/`End` | Jump to bottom                           |

#### Step 3: Render function (`planner-tui/src/ui.rs`)

New function `render_blueprint_table(f: &mut Frame, app: &App, area: Rect)`:
- Split-pane layout: left 55% (table) / right 45% (detail) when expanded
- Header row: ID, Type, Name, Status
- Type-colored cells matching web UI palette
- Highlight style with `►` marker
- Status bar: node count, edge count, active type filter

#### Step 4: Node detail pane

New function `render_node_detail(f: &mut Frame, app: &App, area: Rect)`:
- Full typed field display (all fields per node type)
- Upstream/downstream edge listing
- Tags display
- Timestamps

#### Step 5: View switching

- `Ctrl+B` toggles between `AppView::Socratic` and `AppView::Blueprint`
- `draw()` in `ui.rs` routes to the appropriate renderer based on `app.view`
- Entering Blueprint view loads data from `BlueprintStore::snapshot()`

#### Step 6: Data loading

```rust
impl App {
    pub fn load_blueprint(&mut self, store: &BlueprintStore) {
        let snapshot = store.snapshot();
        self.blueprint.nodes = snapshot.nodes.iter().map(|n| n.summary()).collect();
        self.blueprint.edges = snapshot.edges.clone();
        self.blueprint.selected = 0;
        self.blueprint.table_state.select(Some(0));
    }
}
```

### Acceptance Criteria

- `Ctrl+B` switches to blueprint table view
- Node table displays all blueprint nodes with correct columns
- `j`/`k` navigation works with visual highlight
- `Enter` expands the detail pane with full node information
- `/` activates search filter
- `t` cycles through type filters
- `q` returns to Socratic view
- Empty state handled gracefully (no nodes in blueprint)

### New Tests

```rust
#[test]
fn filtered_nodes_returns_all_when_no_filter() { /* ... */ }

#[test]
fn filtered_nodes_filters_by_search_string() { /* ... */ }

#[test]
fn filtered_nodes_filters_by_type() { /* ... */ }

#[test]
fn navigation_wraps_at_boundaries() { /* move_up at 0 stays at 0, etc. */ }

#[test]
fn type_filter_cycles_through_all_types() { /* None → decision → ... → None */ }

#[test]
fn load_blueprint_populates_state() { /* create test store, verify nodes loaded */ }
```

---

## Implementation Order (Recommended)

1. **C.5.7** (Attach docs) — Schema change first, since C.4 depends on the updated schema
2. **C.4** (JSON Merge Patch) — Enables partial updates for all features including docs
3. **D.3** (WebSocket streaming) — Builds on existing WS infrastructure
4. **H.2** (TUI Blueprint Table) — Self-contained, uses existing `BlueprintStore` API
5. **G.6** (Discovery scanners) — Most complex, benefits from having all other features stable

Features 3, 4, and 5 are independent and can be parallelized.

---

## Dependencies

### Cargo.toml additions needed

**planner-server/Cargo.toml:**
- None required (axum WebSocket already available, serde_json already present)
- Optional: `json-patch = "2"` for RFC 6902 compliance (C.4 can use manual merge instead)

**planner-tui/Cargo.toml:**
- None required (ratatui and crossterm already present)

**planner-schemas/Cargo.toml:**
- None required

**planner-core/Cargo.toml:**
- None required

### Frontend changes needed (alongside Rust work)

| Feature | Frontend Change |
|---------|----------------|
| C.4     | Update `EditNodeForm.tsx` to send partial payloads (optional optimization) |
| C.5.7   | Add `documentation?: string` to TS types, docs tab in DetailDrawer, textarea in EditNodeForm |
| D.3     | Add WebSocket client in `client.ts`, update `ReconvergencePanel.tsx` for streaming |
| G.6     | None — frontend already complete (G.1–G.5) |
| H.2     | None — TUI only |
