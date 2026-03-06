# Living System Blueprint — Deep Dive Audit & Implementation Plan

**Date:** March 5, 2026  
**Status:** Research Complete — Ready for Implementation  
**Repo:** github.com/theturtleautomation/planner

---

## 1. Decision Audit — Conversation Sign-Off vs. Current Code

### Decision 1: "Human-readable NodeId with UUID attached"

**Sign-off:** Node IDs should be human-readable slugs with UUID suffix for uniqueness.

**Current Implementation:** ✅ **FULLY ALIGNED**

```rust
// planner-schemas/src/artifacts/blueprint.rs
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(slug: &str) -> Self {
        let uuid_prefix = &Uuid::new_v4().to_string()[..8];
        // Produces: "use-messagepack-a1b2c3d4"
    }
    pub fn with_prefix(prefix: &str, slug: &str) -> Self {
        // Produces: "dec-use-messagepack-a1b2c3d4"
    }
}
```

The spec document (Section 12, Q1) originally showed `DEC-001` sequential IDs, but the implementation correctly followed your override to use `{slug}-{uuid8}`. The `with_prefix` variant adds a type prefix (dec-, tech-, comp-, con-, pat-, qr-) which is a good middle ground — you get readability AND uniqueness.

**No changes needed.**

---

### Decision 2: "Event sourced is fine"

**Sign-off:** Event-sourced history for Blueprint changes (store edits, reconstruct state).

**Current Implementation:** ⚠️ **PARTIALLY IMPLEMENTED — Snapshot-only, not event-sourced**

The spec (Section 6) defines a `history/` directory with timestamped snapshots:
```
{data_dir}/blueprint/
  └── history/
      ├── 2026-03-05T10-30-00Z.msgpack  # Snapshot before edit
      └── ...
```

The `BlueprintStore` has `save_snapshot()` which writes full Blueprint snapshots to `history/`. This is **snapshot-based**, not event-sourced. True event sourcing would:

1. Store each edit as an event (e.g., `NodeCreated`, `NodeUpdated`, `EdgeAdded`, `NodeDeleted`)
2. Reconstruct state by replaying events
3. Enable undo/redo by stepping through the event log
4. Support diffing between any two points in time

**What exists today:**
- `save_snapshot()` — full graph dump before edits (implemented)
- No event log structure
- No undo/redo capability
- No diff between snapshots

**Gap assessment:** The snapshot approach works for MVP but doesn't deliver the full power of event sourcing that the spec envisions. The reactive loop (Section 5) requires understanding *what changed*, not just *what the state was* — event sourcing gives you that natively.

**Recommendation:** Implement a `BlueprintEvent` enum and `EventLog` alongside the existing snapshot approach. Events feed the Impact Preview and Reconvergence Report features.

```rust
enum BlueprintEvent {
    NodeCreated { node: BlueprintNode, timestamp: String },
    NodeUpdated { node_id: NodeId, before: BlueprintNode, after: BlueprintNode, timestamp: String },
    NodeDeleted { node_id: NodeId, node: BlueprintNode, timestamp: String },
    EdgeCreated { edge: Edge, timestamp: String },
    EdgeDeleted { edge: Edge, timestamp: String },
    ReconvergenceStarted { trigger_node: NodeId, affected: Vec<NodeId>, timestamp: String },
    ReconvergenceCompleted { changed_nodes: Vec<NodeId>, timestamp: String },
}
```

---

### Decision 3: "Your suggestion is fine" (Reconvergence autonomy)

**Sign-off:** Auto-accept shallow/medium changes, require user review for deep changes.

**Current Implementation:** ⚠️ **TYPES DEFINED, LOGIC NOT WIRED**

The `ImpactSeverity` enum exists with all three levels:
```rust
pub enum ImpactSeverity {
    Shallow,  // Metadata only
    Medium,   // Local reconverge
    Deep,     // Full cascade
}
```

The `classify_impact()` function in `blueprint.rs` correctly classifies impact by severity. However:

- The "Apply & Reconverge" button in `ImpactPreviewModal.tsx` calls `handleImpactApply()` which just closes the modal (`handleImpactClose()`)
- No actual reconvergence logic exists — the comment says `// Future: apply reconvergence`
- The autonomy decision (auto-accept shallow/medium, review deep) is not implemented anywhere

**Gap:** The front-end and back-end types are ready, but the reconvergence execution pipeline doesn't exist. This is Phase 4 in the spec (Section 11).

---

### Decision 4: "One per project"

**Sign-off:** One Blueprint per project.

**Current Implementation:** ⚠️ **SINGLE GLOBAL BLUEPRINT — no project scoping**

The server creates one `BlueprintStore` at startup:
```rust
// main.rs line 90
let blueprint_store = BlueprintStore::open(Path::new(&data_dir))
```

`AppState` holds a single `blueprints: BlueprintStore`. There's no project ID association. The `BlueprintStore` doesn't have a `project_id` field, and the API routes (`/blueprint/*`) don't take a project ID parameter.

**Gap:** When multi-project support is added, the Blueprint store needs to be scoped per project. The `data_dir/blueprint/` path should become `data_dir/projects/{project_id}/blueprint/`.

**Recommendation for now:** This is fine for single-project use. Tag this as a known debt — when multi-project lands, refactor `BlueprintStore::open()` to accept a project-scoped path, and add project_id to the API routes.

---

### Decision 5: "WebUI is primary, TUI is table-only"

**Sign-off:** Can't do complex relationship graphs in the TUI, web UI is primary. TUI gets simplified views.

**Current Implementation:** ✅ **CORRECTLY SCOPED**

- **Web UI:** Full Blueprint page at `/blueprint` with Graph (D3 force-directed), Table, and Radar views, Detail Drawer, Impact Preview Modal
- **TUI:** No Blueprint view implemented (Ratatui app focuses on chat + pipeline stage tracking + event logs)

The web UI is treated as the primary surface for Blueprint interaction. The TUI does not attempt to render the graph — which is the correct call per this decision.

**Future TUI parity (when needed):**
- Table view of nodes (sortable, filterable) — maps to `ratatui::widgets::Table`
- Node detail panel on selection — maps to `ratatui::widgets::Paragraph` in a split pane
- No graph rendering in terminal — confirmed by your decision

---

## 2. Spec vs. Implementation — Detailed Gap Analysis

### Node Type Shapes

**Spec says (Section 7.2):**
| Type | Shape |
|---|---|
| Decision | Rounded rectangle |
| Technology | Hexagon |
| Component | Square |
| Constraint | Diamond |
| Pattern | Oval/Ellipse |
| Quality Requirement | Shield |

**Schema docs header says (blueprint.rs line 11-16):**
| Type | Shape |
|---|---|
| Decision | Diamond ⚠️ |
| Technology | Hexagon ✅ |
| Component | Rectangle ✅ |
| Constraint | Pentagon ⚠️ |
| Pattern | Oval ✅ |
| Quality Requirement | Shield ✅ |

**Frontend implementation (BlueprintGraph.tsx):**
| Type | Shape |
|---|---|
| Decision | Rounded rectangle (`rx=6, ry=6`) ✅ |
| Technology | Hexagon (path) ✅ |
| Component | Sharp rect (`rx=2, ry=2`) ✅ |
| Constraint | Diamond (path) ✅ |
| Pattern | Ellipse ✅ |
| Quality Requirement | Shield (path) ✅ |

**Verdict:** The frontend correctly implements the spec shapes. The Rust doc comment in `blueprint.rs` has stale/incorrect shape mapping (says Decision=diamond, Constraint=pentagon). The doc should be updated to match the spec and implementation.

### Edge Visual Styling

**Spec says (Section 7.2):**
- Solid = depends_on
- Dashed = decided_by
- Dotted = constrains

**Conversation says:**
- Purple solid = depends_on
- Blue dashed = decided_by
- Amber dotted = constrains

**Frontend implementation (BlueprintGraph.tsx):**
| Edge | Color | Dash | Spec Match |
|---|---|---|---|
| depends_on | purple | solid (none) | ✅ |
| decided_by | blue | `8,4` dashed | ✅ |
| constrains | amber/warning | `3,3` dotted | ✅ |
| uses | blue | solid | N/A (not in spec) |
| implements | green | `2,4` | N/A |
| satisfies | gold | `8,3,2,3` | N/A |
| affects | purple-hover | `6,4` | N/A |

**Verdict:** The three explicitly specced edge styles are correct. The additional edge types have reasonable styling. No issues.

### Force Simulation Parameters

**Conversation says:** "Increased force charge from -800 to -1400, collision radius now dynamically sized."

**Current code (BlueprintGraph.tsx line 400-407):**
```javascript
.force('charge', d3.forceManyBody().strength(-1400))          // ✅ Matches
.force('collision', d3.forceCollide().radius(d => {            // ✅ Dynamic
    const s = NODE_SIZES[d.node_type];
    return Math.max(s.w, s.h) / 2 + 20;
}))
.force('x', d3.forceX(d => typeX[d.node_type]).strength(0.15)) // ✅ Type clustering
.force('y', d3.forceY(d => typeY[d.node_type]).strength(0.15))
```

**Verdict:** Matches the conversation. The type-based positional forces spread clusters apart as described.

### Radar View

**Conversation says:** "Replaced random positioning with deterministic angular/radial spacing. Rust and Tokio no longer overlap."

**Current code (RadarView.tsx):**
- Groups techs by quadrant + ring
- Uses `indexInGroup` / `countInGroup` for deterministic `rFraction` and `angFraction`
- No random positioning

**Verdict:** ✅ Matches. Deterministic placement is implemented.

### Detail Drawer

**Spec says (Section 7.3):** Full node data, inline-editable, with "Edit Decision", "Propose Change", "View History" buttons.

**Current implementation (DetailDrawer.tsx):**
- ✅ Fetches full node via `api.getBlueprintNode(nodeId)`
- ✅ Shows description, options, category, quality attribute, responsibilities
- ✅ Shows upstream/downstream connections with navigation
- ✅ Shows tags and timestamps
- ⚠️ **NOT inline-editable** — display only
- ⚠️ **No "Edit Decision" button** — only "Close" and "Impact Preview"
- ⚠️ **No "View History" button**

**Gap:** The drawer is read-only. The spec envisions it as the primary editing surface. This is intentional for now (Phase 3 = UI views, Phase 4 = reactive loop including editing).

### Impact Preview Modal

**Spec says (Section 7.4):** Summary line, scrollable affected nodes with action symbols, before/after diff, color-coded severity, Cancel/Modify/Apply buttons.

**Current implementation (ImpactPreviewModal.tsx):**
- ✅ Summary counts by action type
- ✅ Terraform-style symbols (`+`, `~`, `✗`, `⛔`)
- ✅ Color-coded by action (green=add, gold=update/reconverge, red=remove/invalidate)
- ✅ ⚠ warning for reconverge/deep entries
- ⚠️ **No before/after diff** — just the explanation text
- ⚠️ **No "Modify" button** — only Cancel and "Apply & Reconverge"
- ⚠️ **"Apply & Reconverge" is a no-op** (just closes the modal)

**Gap:** The modal renders impact data correctly but can't execute anything. The "Apply & Reconverge" button needs backend support for the reconvergence engine.

### Blueprint API Completeness

**Server routes (api.rs):**
| Route | Method | Status |
|---|---|---|
| `/blueprint` | GET | ✅ Returns full graph summary |
| `/blueprint/nodes` | GET | ✅ List with optional `?type=` filter |
| `/blueprint/nodes` | POST | ✅ Create node |
| `/blueprint/nodes/{nodeId}` | GET | ✅ Get full node |
| `/blueprint/nodes/{nodeId}` | PATCH | ✅ Update (full replacement) |
| `/blueprint/nodes/{nodeId}` | DELETE | ✅ Delete node + cleanup edges |
| `/blueprint/edges` | POST | ✅ Create edge |
| `/blueprint/impact-preview` | POST | ✅ Impact analysis |
| `/blueprint/history` | GET | ❌ **Missing** |
| `/blueprint/reconverge` | POST | ❌ **Missing** (Phase 4) |
| `/blueprint/edges` | DELETE | ❌ **Missing** (can't delete individual edges via API) |
| `/blueprint/nodes/{nodeId}` | PATCH (partial) | ⚠️ Current PATCH is full replacement, not partial update |

### TypeScript Type Mismatches

**Frontend types (types/blueprint.ts) vs. Rust structs:**

| Field | Rust | TypeScript | Match |
|---|---|---|---|
| Decision.options | `Vec<DecisionOption>` with `{name, pros, cons, chosen}` | `{name, description, pros, cons}[]` — missing `chosen`, has `description` | ⚠️ **MISMATCH** |
| Decision.consequences | `Vec<Consequence>` with `{description, positive: bool}` | `{description, type: 'positive'\|'negative'\|'neutral'}` | ⚠️ **MISMATCH** |
| Decision.assumptions | `Vec<Assumption>` with `{description, confidence}` | `{statement, risk, validation_approach}` | ⚠️ **MISMATCH** |
| Constraint.source | `String` (free text) | `ConstraintSource` enum (business/technical/regulatory/resource) | ⚠️ **MISMATCH** |
| Constraint.constraint_type | exists in Rust | missing in TS (has `source` enum instead) | ⚠️ **MISMATCH** |
| Component.provides/consumes | `Vec<String>` | `responsibilities: string[]`, `interfaces: {...}[]` | ⚠️ **MISMATCH** |
| Pattern.rationale | exists in Rust | `scope: PatternScope` instead | ⚠️ **MISMATCH** |
| QualityRequirement | `{attribute, scenario, priority}` | `{attribute, scenario, measure, target, priority}` | ⚠️ **Extra fields in TS** |
| TechnologyCategory | 7 variants | 7 variants but different set | ⚠️ runtime/protocol vs database/infrastructure |

**This is a significant gap.** The TypeScript types in `types/blueprint.ts` appear to have been written from a different version of the spec or independently designed. They need to be synchronized with the Rust structs to avoid runtime deserialization failures.

### Blueprint Population Pipeline

**Spec says (Section 8):** Blueprint is populated from convergence extraction (during pipeline) and retroactive import.

**Current implementation (blueprint_emitter.rs):**
- ✅ `emit_from_intake()` — creates project scope Decision
- ✅ `emit_from_spec()` — extracts Constraints, Technologies, Components, QualityRequirements from NLSpec
- ✅ `emit_from_ar()` — adds Constraint nodes from blocking AR findings
- ✅ `emit_from_factory()` — adds Pattern + Component nodes from factory output
- ⚠️ Pipeline wiring exists (`PipelineConfig.blueprints`) but only in server path — CLI pipeline doesn't emit Blueprint nodes

**Verdict:** Population pipeline is well-implemented for server-side runs. The 4 emission stages cover the key pipeline outputs.

---

## 3. Research Findings & Best Practices

### Force-Directed Graph — Recommended Tuning

Based on research of production systems (Backstage, Structurizr, Terraform graph):

| Parameter | Current | Recommendation | Why |
|---|---|---|---|
| `alphaDecay` | default (0.0228) | 0.015-0.02 | Slower decay = more settling time = fewer jitter artifacts |
| `charge.strength` | -1400 | Keep -1400 for <50 nodes; scale to -800 for 50-100 | Avoids nodes flying off-screen with many nodes |
| `forceLink.distance` | 220 | 180-220 (current is fine) | Good range for readable labels |
| `collision.radius` | dynamic + 20px | dynamic + 30px | 20px padding is tight when labels are long |
| Initial positions | random (D3 default) | Pre-seed by type cluster | Reduces initial chaos and settling time |

**Key insight from Backstage:** Their catalog graph plugin uses a pre-bake pattern — run the simulation for ~300 ticks with `sim.tick(300)` before rendering, so users see a stable layout immediately rather than watching nodes bounce around.

### Event Sourcing — Recommended Pattern

For Blueprint history, the hybrid approach is best:

1. **Append-only event log** for the last N edits (N=1000 or 30 days)
2. **Periodic snapshots** at configurable intervals (every 50 events or every hour)
3. **Reconstruct:** Load latest snapshot, replay events after it
4. **Undo/redo:** Walk the event log backward/forward, reconstruct at each point

This matches the existing SessionStore pattern (memory-first, disk-backed) but adds the event dimension.

### TUI Parity — Recommended Scope

Per decision #5 and research into lazygit/k9s patterns:

| Feature | Web UI | TUI |
|---|---|---|
| Graph view | D3 force-directed | ❌ Not applicable |
| Table view | Sortable HTML table | `ratatui::Table` with keyboard nav |
| Radar view | SVG radar chart | ❌ Not applicable |
| Detail drawer | Slide-out panel | `ratatui::Paragraph` in split pane |
| Node filtering | Sidebar buttons | Keybinding (f) cycle + filter bar |
| Impact preview | Modal overlay | Scrollable pane with Terraform-style symbols |
| Node editing | Inline-editable drawer | ❌ Not applicable (web-only per decision) |

---

## 4. Priority Implementation Plan

### Phase A: Type Alignment & Cleanup (Critical — do first)

1. **Sync TypeScript types with Rust structs** — The `types/blueprint.ts` full-node interfaces (`DecisionNode`, `ComponentNode`, etc.) must match the actual Rust serde output. This blocks any real data rendering.
2. **Fix Rust doc comment** in `planner-schemas/src/artifacts/blueprint.rs` lines 11-16 — update Decision shape from "diamond" to "rounded rect" and Constraint from "pentagon" to "diamond"
3. **Add edge DELETE endpoint** — `DELETE /blueprint/edges` with source/target/type filter
4. **Add history GET endpoint** — `GET /blueprint/history` to list snapshots

### Phase B: Event Sourcing Layer

1. Define `BlueprintEvent` enum in `planner-schemas`
2. Add `EventLog` to `BlueprintStore` (append on every write operation)
3. Implement event persistence (append to `history/events.msgpack`, periodic compaction)
4. Wire `save_snapshot()` to trigger automatically every N events
5. Add `GET /blueprint/history/events` endpoint

### Phase C: Detail Drawer Editing

1. Make drawer fields inline-editable (contentEditable or input fields)
2. Add "Save Changes" button that PATCHes the node
3. Add "Propose Change" that triggers Impact Preview
4. Add "View History" button linking to event log for that node
5. Implement partial PATCH (JSON Merge Patch) instead of full replacement

### Phase D: Reconvergence Engine

1. Implement reconvergence execution in `planner-core`:
   - Topological sort affected nodes (already exists)
   - For each affected node: re-evaluate via LLM call or deterministic update
   - Auto-accept shallow/medium changes
   - Queue deep changes for user review
2. Wire "Apply & Reconverge" button to backend
3. Add real-time progress via WebSocket (reuse existing event streaming)
4. Implement reconvergence result report

### Phase E: Graph UX Polish

1. Pre-bake simulation (`sim.tick(300)`) for instant stable layout
2. Adaptive charge strength based on node count
3. Increase collision padding to +30px
4. Add hierarchical layout option (toggle between force and dagre/hierarchical)
5. Add search/filter bar within graph view
6. Minimap for large graphs

### Phase F: TUI Blueprint Table (if needed)

1. Add a "Blueprint" tab to TUI
2. Implement `ratatui::Table` with columns: Name, Type, Status, Connections
3. Add keybinding filtering by node type
4. Add split-pane detail view on Enter

---

## 5. TypeScript/Rust Type Sync — Detailed Fix List

These are the specific fields that need to change in `planner-web/src/types/blueprint.ts` to match Rust:

```typescript
// DecisionNode — align with Rust Decision struct
interface DecisionNode {
  node_type: 'decision';
  id: string;
  title: string;
  status: 'proposed' | 'accepted' | 'superseded' | 'deprecated';
  context: string;
  options: { name: string; pros: string[]; cons: string[]; chosen: boolean }[];  // FIX: chosen bool, remove description
  consequences: { description: string; positive: boolean }[];  // FIX: positive bool not type enum
  assumptions: { description: string; confidence: string }[];  // FIX: description+confidence, not statement+risk
  supersedes?: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ComponentNode — align with Rust Component struct
interface ComponentNode {
  node_type: 'component';
  id: string;
  name: string;
  component_type: 'module' | 'service' | 'library' | 'store' | 'interface' | 'pipeline';  // FIX: add
  description: string;
  provides: string[];   // FIX: was 'responsibilities'
  consumes: string[];   // FIX: was 'interfaces'
  status: 'planned' | 'in_progress' | 'shipped' | 'deprecated';
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ConstraintNode — align with Rust Constraint struct
interface ConstraintNode {
  node_type: 'constraint';
  id: string;
  title: string;
  constraint_type: 'technical' | 'organizational' | 'philosophical' | 'regulatory';  // FIX: was source enum
  description: string;
  source: string;  // FIX: free text, not enum
  tags: string[];
  created_at: string;
  updated_at: string;
}

// PatternNode — align with Rust Pattern struct
interface PatternNode {
  node_type: 'pattern';
  id: string;
  name: string;
  description: string;
  rationale: string;  // FIX: was scope enum
  tags: string[];
  created_at: string;
  updated_at: string;
}

// QualityRequirementNode — align with Rust QualityRequirement struct
interface QualityRequirementNode {
  node_type: 'quality_requirement';
  id: string;
  attribute: 'performance' | 'reliability' | 'security' | 'usability' | 'maintainability';
  scenario: string;
  priority: 'critical' | 'high' | 'medium' | 'low';
  tags: string[];
  created_at: string;
  updated_at: string;
  // REMOVE: measure, target (don't exist in Rust)
}

// TechnologyCategory — align with Rust enum
type TechnologyCategory = 'language' | 'framework' | 'library' | 'runtime' | 'tool' | 'platform' | 'protocol';
// FIX: was database/infrastructure, should be runtime/protocol
```

---

## 6. Summary

| Decision | Status | Action |
|---|---|---|
| 1. Human-readable NodeId + UUID | ✅ Implemented correctly | None |
| 2. Event sourced history | ⚠️ Snapshot-only, not event-sourced | Implement Phase B |
| 3. Reconvergence autonomy (auto shallow/medium, review deep) | ⚠️ Types defined, no execution logic | Implement Phase D |
| 4. One Blueprint per project | ⚠️ Global singleton, not project-scoped | Future debt — OK for now |
| 5. WebUI primary, TUI table-only | ✅ Correctly scoped | Phase F when needed |

| Area | Status | Priority |
|---|---|---|
| TypeScript/Rust type sync | ❌ **Mismatched** — will cause runtime failures | **CRITICAL — Phase A** |
| Rust doc comment shapes | ⚠️ Stale | Phase A (5 min fix) |
| Missing API endpoints (history, edge delete) | ⚠️ Gap | Phase A |
| Blueprint population (pipeline emitter) | ✅ Working | None |
| Graph view (D3 force-directed) | ✅ Working, tuned | Phase E for polish |
| Table view | ✅ Working | None |
| Radar view | ✅ Working, deterministic | None |
| Detail drawer | ⚠️ Read-only, needs editing | Phase C |
| Impact preview modal | ⚠️ Renders but can't execute | Phase D |
| Event sourcing | ⚠️ Not implemented | Phase B |
| Reconvergence engine | ❌ Not implemented | Phase D |
| TUI Blueprint | ❌ Not started | Phase F |
