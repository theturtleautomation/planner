# Blueprint Mockup v2 vs. Current Implementation — Full Gap Analysis & Feature Roadmap

**Date:** March 5, 2026
**Repo:** github.com/theturtleautomation/planner
**Context:** Comparing `Planner-Blueprint-Mockup-2.html` against the current React implementation + Rust backend, then expanding with industry research on software architecture BOM tools.

---

## 1. Feature-by-Feature Comparison: Mockup vs. Current Code

### Sidebar / Navigation

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **App-level sidebar** | Full sidebar with "Registry" section (Tools, Plans, Research, Analyses, Scripts) + "Blueprint" section + "Session" section | Sidebar in `Layout.tsx` with only 3 items: Sessions, Blueprint, Admin | ⚠️ **Much simpler** — no Registry concept, no Knowledge/Library sections |
| **Blueprint node-type filter in sidebar** | Blueprint section shows All Nodes (27), Decisions (6), Technologies (6), Components (7), Constraints (3), Patterns (3), Quality (2) with counts | Same filter exists but as an inner sidebar panel within `BlueprintPage.tsx`, not in the app-level sidebar | ✅ **Functionally equivalent** but different placement |
| **Knowledge/Library section** | Not in mockup (but user is asking about this) | Does not exist | ❌ **Missing entirely** |
| **Registry sections** | Tools, Plans, Research, Analyses, Scripts — these represent the full project workspace | Only Sessions and Blueprint pages exist | ⚠️ **Significantly less** |

### Top Bar

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **Title "Living System Blueprint"** | Shows "Living System Blueprint" with subtitle "Planner v2 — 27 nodes, 28 edges — Last converged 2m ago" | Shows "Blueprint" with "X nodes · Y edges" | ⚠️ **Missing "Living System Blueprint" name, missing "Last converged" timestamp** |
| **View tabs (Graph/Table/Radar)** | Tabs in the topbar-left area | Tabs in topbar-right area with icons | ✅ **Implemented** (different placement) |
| **Impact Preview button** | Ghost button in topbar-right, always accessible | In topbar-right, disabled when no node selected | ✅ **Implemented** (slightly different behavior) |
| **Theme toggle** | In topbar-right | In sidebar bottom | ✅ **Implemented** (different placement) |

### Graph View

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **D3 force-directed graph** | Full implementation with type-specific shapes (rounded rect, hexagon, sharp rect, diamond, ellipse, shield) | Full implementation with same shapes | ✅ **Implemented** |
| **Node type prefixes** (DEC, TECH, COMP, etc.) | Monospace prefix before label | Same | ✅ **Implemented** |
| **Edge styling** (solid, dashed, dotted per type) | 7 edge types with distinct colors/dashes | Same 7 edge types | ✅ **Implemented** |
| **Graph legend (bottom-left)** | Color-coded dot legend for node types | Exists in graph component | ✅ **Implemented** |
| **Edge legend (bottom-right)** | Line-style legend for edge types | In sidebar panel when graph view active | ✅ **Implemented** (different placement) |
| **Force parameters** | charge: -1400, collision: dynamic+20, type-based x/y positioning | Same values | ✅ **Matches** |
| **Zoom + drag** | D3 zoom with scale 0.2-4, node drag | Same | ✅ **Implemented** |
| **Hover tooltip** | Fixed-position tooltip following mouse with name, type badge, status | Bottom-left info panel on hover | ⚠️ **Different UX** — mockup uses follow-cursor tooltip, implementation uses corner info box |
| **Hover edge highlighting** | Connected edges brighten to 0.95, others dim to 0.06 | Same behavior | ✅ **Implemented** |
| **Loading skeleton** | Pulse animation during init | Same | ✅ **Implemented** |

### Table View

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **Sortable columns** (Name, Type, Status, ID, Connections) | Click headers to sort asc/desc | Exists in `TableView.tsx` | ✅ **Implemented** |
| **Filter chips** above table | All/Decision/Technology/etc. chips | Same filter chips | ✅ **Implemented** |
| **Click row → open drawer** | Row click opens detail drawer | Same | ✅ **Implemented** |
| **Type badges in cells** | Colored badge per node type | Same | ✅ **Implemented** |
| **Status badges in cells** | Colored status badge | Same | ✅ **Implemented** |
| **Connection count column** | Shows total upstream+downstream count | Same | ✅ **Implemented** |

### Radar View

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **4 rings** (Adopt, Trial, Assess, Hold) | SVG-rendered concentric circles | Same | ✅ **Implemented** |
| **4 quadrants** (Languages, Frameworks, Libraries, Tools) | Angular labels | Same | ✅ **Implemented** |
| **Deterministic placement** | GroupBy quadrant+ring, spread by index | Same algorithm | ✅ **Implemented** |
| **Click dot → open drawer** | Click tech dot opens drawer | Same | ✅ **Implemented** |

### Detail Drawer

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **Slide-out panel** (420px from right, overlay behind) | Animated slide with overlay | Same | ✅ **Implemented** |
| **Header** (title + type badge + status badge) | Shows both badges | Same | ✅ **Implemented** |
| **Node ID** (monospace, faint) | Shows human-readable ID | Same | ✅ **Implemented** |
| **Description section** | Full description text | Same | ✅ **Implemented** |
| **Decision options** (with "CHOSEN" label) | Lists options with chosen highlight | Same | ✅ **Implemented** |
| **Technology category** | Shows subtype category | Same | ✅ **Implemented** |
| **Quality attribute** | Shows attribute name | Same | ✅ **Implemented** |
| **Component responsibilities** | Not in mockup (mockup shows description only) | Shows responsibilities array from TS type | ✅ **Has more** than mockup |
| **Upstream/downstream connections** | Navigable relation rows with badges | Same | ✅ **Implemented** |
| **Tags** | Not shown in mockup | Shows tags section | ✅ **Has more** than mockup |
| **Timestamps** | Not shown in mockup | Shows created_at / updated_at | ✅ **Has more** than mockup |
| **"Edit" button** | Footer has "Edit", "Propose Change", "Impact Preview" | Footer has only "Close" and "Impact Preview" | ❌ **Missing "Edit" and "Propose Change" buttons** |
| **Inline editing** | Not yet — mockup shows display-only too, but has Edit button stub | Not implemented | ❌ **Missing** |
| **"View History" button** | Not in mockup but in spec | Not implemented | ❌ **Missing** |

### Impact Preview Modal

| Feature | Mockup | Current Implementation | Status |
|---|---|---|---|
| **Modal overlay** with centered panel | 680px modal with scale-in animation | Same | ✅ **Implemented** |
| **Terminal-style impact output** | Monospace font, dark bg (#0d0d0c), Terraform-style symbols | Same styling | ✅ **Implemented** |
| **Summary line** (counts by action) | "2 reconverge, 2 update, 1 invalidate, 1 new" | Same format | ✅ **Implemented** |
| **Impact entries** with symbols (+, ~, ✗, ⛔) | Color-coded per action type | Same | ✅ **Implemented** |
| **Explanatory descriptions** | Indented description per entry | Same | ✅ **Implemented** |
| **⚠ warning markers** | On reconverge/deep entries | Same | ✅ **Implemented** |
| **Footer buttons** | "Cancel" + "Apply & Reconverge" | Same | ✅ **Implemented** (but Apply is a no-op) |

---

## 2. Summary: What the Current Implementation HAS from the Mockup

The current React implementation has **~90% of the mockup's visual and interactive features**:

- ✅ All 3 view modes (Graph, Table, Radar)
- ✅ D3 force graph with correct shapes, colors, edges, zoom, drag
- ✅ Table with sorting, filtering, badges
- ✅ Radar with deterministic placement
- ✅ Detail drawer with navigation between connected nodes
- ✅ Impact preview modal with Terraform-style output
- ✅ Theme toggle (dark/light)
- ✅ Sidebar node-type filtering with counts
- ✅ Loading/error states
- ✅ Keyboard navigation (Escape to close)

## 3. What's MISSING from the Mockup

| Gap | Priority | Effort |
|---|---|---|
| **Edit button + Propose Change button** in drawer footer | High | Small — add buttons, wire to backend PATCH |
| **"Last converged" timestamp** in topbar subtitle | Medium | Small — add to API response |
| **Follow-cursor tooltip** on graph (vs. corner info box) | Low | Small — UX preference |
| **App-level sidebar** with Registry sections (Tools, Plans, Research, etc.) | N/A | These aren't Blueprint features |

---

## 4. What's MISSING Beyond the Mockup — The Bigger Picture

This is where your question about "knowledge, library, viewable/editable components, technology of the projects, viewable editable library of decisions" comes in. The mockup is a read-only visualization. A complete software architecture BOM needs much more.

Based on research across Backstage, Structurizr, IcePanel, LeanIX, Port, Cortex, OpsLevel, CycloneDX, ADR tools, and tech radar implementations:

### 4.1 Decision Library (ADR Registry)

**Current state:** Decisions are nodes in the graph. You can view them in the drawer. No editing, no history, no supersession chain navigation, no decision lifecycle management.

**What a complete Decision Library needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Decision list view** | Dedicated page/panel listing all decisions with status filter (Proposed/Accepted/Superseded/Deprecated) | Log4brains, Backstage ADR plugin |
| **Decision timeline** | Chronological view showing when decisions were made, with full history | Log4brains timeline view |
| **Supersession chain** | Visual chain: Decision A → superseded by B → superseded by C. Navigate the lineage. | adr-tools DOT graph, CycloneDX pedigree |
| **Decision detail page** | Full page (not just drawer) with context, options with pros/cons, consequences, assumptions, related components, discussion thread | Log4brains preview page |
| **Inline editing** | Edit decision fields directly in the UI — title, context, options, status transitions | Port entity editing, LeanIX fact sheets |
| **Decision proposal workflow** | "Propose Change" → creates a draft → shows Impact Preview → approve/reject flow | Arachne decision gates |
| **Decision search** | Full-text search across all decision contexts, options, consequences | Log4brains full-text search, Backstage catalog search |
| **Contextual attachment** | Each decision shows which components/technologies/patterns it affects (already partially done via edges) | Backstage ADR plugin — ADRs live next to the service |

### 4.2 Technology Registry (Tech Catalog)

**Current state:** Technologies are nodes with name, category, ring, rationale. Viewable in drawer and radar view. No editing.

**What a complete Technology Registry needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Technology detail page** | Full page with: version, category, ring, rationale, adoption date, deprecation date, alternatives, migration guide | LeanIX fact sheets, Backstage TechDocs |
| **Ring lifecycle tracking** | Show when a technology moved between rings (Assess → Trial → Adopt). Movement history with timestamps and reasons. | Thoughtworks Tech Radar "moved in/out" indicators |
| **Versioned entries** | Track which version of a technology the project uses vs. latest available | CycloneDX version tracking |
| **Technology comparison** | Side-by-side comparison of two technologies (the options from a Decision) | Custom — no standard tool does this well |
| **Dependency impact** | "Which components use this technology?" — already partially visible via graph edges, but needs a dedicated panel | OpsLevel upstream/downstream mapping |
| **Alternative technologies** | For each technology, show what was considered and rejected (links back to the Decision that chose it) | Log4brains options considered |
| **Migration guidance** | When a technology moves to "Hold", attach a migration guide to its replacement | Zalando Compendium |

### 4.3 Component Registry

**Current state:** Components have name, description, provides/consumes (Rust) or responsibilities/interfaces (TS — mismatched). Viewable in drawer. No editing.

**What a complete Component Registry needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Component detail page** | Full page with: description, owner, status, provides, consumes, dependencies, health status, documentation links | Backstage catalog entity page, Port entity page |
| **Component status lifecycle** | Planned → In Progress → Shipped → Deprecated with transition dates | Backstage lifecycle, OpsLevel service tiers |
| **Provides/Consumes** | Explicit interface declarations — what does this component expose, what does it consume | CycloneDX services element |
| **Link to code** | Direct link from component to its source code directory/module | IcePanel link-to-reality |
| **API surface** | If the component exposes an API, show the routes/endpoints | Backstage API plugin |
| **Quality scorecard** | Per-component quality scores: test coverage, documentation completeness, security posture | Cortex scorecards, OpsLevel rubrics |
| **Dependency graph (scoped)** | Show this component's immediate dependency neighborhood, not the whole graph | Port dependency view |

### 4.4 Pattern Library

**Current state:** Patterns are nodes with name, description, rationale (Rust) or scope (TS — mismatched). Viewable in drawer. No editing.

**What a complete Pattern Library needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Pattern catalog page** | Browsable library of all patterns with search and category filter | Zalando Compendium |
| **Pattern detail** | Full page with: description, rationale, when to use, when not to use, examples, related decisions | Zalando Compendium pattern pages |
| **Implementation map** | "Which components implement this pattern?" — bidirectional link from pattern → components | Already exists as `implements` edges |
| **Pattern lifecycle** | Proposed → Accepted → Deprecated, similar to decisions | Custom |
| **Code examples** | Embedded code snippets showing how the pattern is implemented in this project | Backstage TechDocs |
| **Anti-patterns** | Document known anti-patterns alongside the accepted patterns | Zalando Compendium |

### 4.5 Constraint Registry

**Current state:** Constraints have title, description, constraint_type (Rust) or source enum (TS — mismatched). No editing.

**What a complete Constraint Registry needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Constraint list** | Filterable by type (technical, organizational, philosophical, regulatory) | Custom |
| **Constraint → Decision chain** | Which decisions were driven by this constraint? (already partially via `constrains` edges) | ADR context |
| **Negotiability flag** | Is this constraint negotiable? Under what conditions? | TS type has `negotiable: boolean` but Rust doesn't |
| **Expiry/review date** | Some constraints are time-bound (e.g., budget constraints). When should they be reviewed? | LeanIX lifecycle |
| **Validation status** | Is this constraint still active? Has it been relaxed? | Custom |

### 4.6 Quality Requirement Tracking

**Current state:** Quality requirements have attribute, scenario, priority. No measure/target (Rust), but TS type has measure/target (mismatched). No editing.

**What complete Quality tracking needs:**

| Feature | Description | Inspiration |
|---|---|---|
| **Quality dashboard** | Overview of all quality requirements with satisfaction status | Cortex maturity report |
| **Measure + Target** | Explicit metrics: "Response time < 100ms at p99 under 1000 RPS" | Cortex CQL scorecards |
| **Evidence** | Link to test results, benchmarks, monitoring dashboards that prove satisfaction | Cortex integration evidence |
| **Trend tracking** | Quality satisfaction over time — are we improving or regressing? | OpsLevel historical reporting |
| **Campaigns** | Time-bound quality improvement initiatives: "Reduce cold start time to <500ms by Q2" | Cortex initiatives, OpsLevel campaigns |

---

## 5. Feature Suggestions — New Capabilities Inspired by Industry Tools

### 5.1 Knowledge Base / Documentation Layer

**Not in mockup or current implementation at all.**

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Attach docs to any node** | Every node (decision, component, pattern, etc.) can have attached markdown documentation. Renders inline in the drawer or a dedicated docs tab. | Structurizr `!docs`, Backstage TechDocs | **High** |
| **Architecture Principles page** | Separate page listing overarching architecture principles (distinct from decisions — principles are durable, decisions are specific) | LeanIX principles, ArchiMate | Medium |
| **Glossary** | Project-specific terminology with definitions, linked to relevant nodes | Custom | Low |
| **"Why" annotations on edges** | Each relationship (e.g., "Session Store → Event Store: depends_on") can carry an explanation of why this dependency exists | Custom — extremely high value | **High** |

### 5.2 Completeness & Data Quality

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Completeness percentage** | Each node type shows what fields are filled vs. empty. "This decision is 60% complete — missing: consequences, assumptions" | LeanIX completion % | **High** |
| **Orphan detection** | Highlight nodes with zero edges — they're likely missing relationships | CycloneDX compositions | Medium |
| **Stale node detection** | Flag nodes not updated in >90 days | OpsLevel campaigns | Medium |
| **Unknown inventory marker** | Explicit "This area of the system is not yet documented" placeholders | CycloneDX compositions (known vs. unknown) | Medium |

### 5.3 Lifecycle & History

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Event timeline** | Global timeline showing all changes across all node types: "Mar 3: Decision 'Use MessagePack' accepted", "Mar 4: Component 'CXDB' moved to Shipped" | Log4brains timeline | **High** |
| **Node history** | Per-node changelog: every edit, status transition, edge change with timestamps and diffs | Event sourcing (your Decision #2) | **High** — blocked on Phase B |
| **Diff view** | Before/after comparison for any node change | Custom | Medium |
| **Blueprint snapshots** | Named snapshots: "v1.0 Architecture", "Post-migration Architecture" — compare how the graph changed | Structurizr workspace snapshots | Medium |

### 5.4 Automated Discovery & Population

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Pipeline-driven population** | Decisions/technologies/components are auto-extracted from pipeline runs and proposed as new nodes | Already exists via `blueprint_emitter.rs` — **partially implemented** | N/A |
| **Cargo.toml / package.json scanning** | Auto-detect technologies from dependency manifests and propose them as Technology nodes | Syft, OpsLevel automated detection | **High** |
| **Git history analysis** | Detect components from directory structure; detect patterns from commit patterns | Custom | Low |
| **LLM-assisted extraction** | During sessions, the LLM identifies implicit decisions/constraints and proposes Blueprint nodes | Already the convergence extraction model | N/A |

### 5.5 Search & Filtering

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Global search** | Search across all node types by name, description, tags, ID | Log4brains full-text, Backstage catalog search | **High** |
| **Tag-based filtering** | Filter the entire graph/table by tags (cross-cutting concerns like "security", "performance") | IcePanel tags-as-selectors | **High** |
| **Saved filters / views** | Save named filter configurations: "Security-relevant nodes", "In-progress components" | LeanIX saved views, IcePanel | Medium |
| **Multi-select filter** | Select multiple node types simultaneously (e.g., show Decisions + Components together) | Custom | Medium |

### 5.6 Editing & Collaboration

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Inline editing in drawer** | Edit any field directly in the detail drawer; save with PATCH | Port, LeanIX inline editing | **Critical** — Phase C |
| **Create node from UI** | "Add Decision", "Add Technology" buttons with form wizard | Port self-service actions, LeanIX | **High** |
| **Delete node from UI** | With confirmation and impact preview before deletion | Standard CRUD | **High** |
| **Edge management** | Add/remove/edit edges between nodes from the UI (not just API) | IcePanel canvas editing | Medium |
| **Propose + Review workflow** | Proposed changes go through impact preview → approval (your Decision #3) | Arachne decision gates | Phase D |
| **Batch operations** | Select multiple nodes and apply status changes, tag additions, or deletions | LeanIX bulk edit | Low |

### 5.7 Graph Visualization Enhancements

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Hierarchical layout toggle** | Switch between force-directed and dagre/hierarchical layout | Structurizr multiple views | Medium |
| **Flow overlays** | Show data/request flows as animated paths over the static graph | IcePanel flows as overlays | Medium |
| **Minimap** | Small overview in corner for large graphs | Standard graph UX | Medium |
| **Cluster grouping** | Group nodes by type or tag into visual clusters with bounding boxes | Backstage catalog graph | Low |
| **Pre-bake simulation** | Run `sim.tick(300)` before rendering for instant stable layout | Backstage catalog graph | **High** — Phase E |
| **Neighborhood focus** | Click a node to show only its N-hop neighborhood | Port dependency view | Medium |

### 5.8 Export & Integration

| Feature | Description | Inspiration | Priority |
|---|---|---|---|
| **Export to CycloneDX SBOM** | Generate a standards-compliant SBOM from the Blueprint data | CycloneDX | Medium |
| **Export to Markdown ADRs** | Export decisions as standard ADR markdown files | adr-tools format | Medium |
| **Export graph as SVG/PNG** | Download the current graph view as an image | Standard | Medium |
| **API for external tools** | Full REST API for CRUD on all node types (partially exists) | Port GraphQL API | Already partial |

---

## 6. Recommended Implementation Roadmap (Updated)

Building on the previous Phase A–F plan, incorporating the new feature suggestions:

### Phase A: Foundation (Critical)
1. Sync TypeScript types with Rust structs (**blocking**)
2. Fix Rust doc comment shapes
3. Add missing API endpoints (history GET, edge DELETE)
4. Add "Edit" and "Propose Change" buttons to drawer footer (even as stubs)

### Phase B: Event Sourcing
1. `BlueprintEvent` enum + `EventLog`
2. Event persistence
3. `GET /blueprint/history/events` endpoint
4. Per-node history in drawer ("View History" button)

### Phase C: Editing & CRUD
1. Inline editing in detail drawer
2. "Create Node" wizard (form-based creation for each node type)
3. "Delete Node" with confirmation + impact preview
4. Partial PATCH (JSON Merge Patch)
5. Edge creation/deletion from UI

### Phase C.5: Knowledge & Library (NEW)
1. **Decision library page** — dedicated list view with timeline, search, status filter
2. **Technology catalog page** — grid/list with ring badges, category grouping, version info
3. **Component registry page** — list with status lifecycle, owner, dependency count
4. **Pattern library page** — browsable catalog with "which components implement this?"
5. **Global search** across all node types
6. **Completeness indicators** — show % complete per node, highlight missing fields
7. **Attach documentation** to any node (markdown body rendered in drawer)

### Phase D: Reconvergence Engine
1. Reconvergence execution with autonomy rules
2. Wire "Apply & Reconverge" button
3. WebSocket progress streaming
4. Result report

### Phase E: Graph & Visualization Polish
1. Pre-bake simulation
2. Adaptive charge strength
3. Minimap
4. Hierarchical layout toggle
5. Neighborhood focus mode

### Phase F: Lifecycle & History
1. Event timeline page (global changes across all nodes)
2. Per-node changelog with diffs
3. Blueprint snapshots (named versions)
4. Stale node detection + orphan detection

### Phase G: Automated Discovery
1. Cargo.toml scanning for technologies
2. Directory-structure scanning for components
3. "Proposed nodes" queue from pipeline runs

### Phase H: TUI Blueprint Table
1. `ratatui::Table` with keyboard navigation
2. Node detail in split pane

---

## 7. The "Bill of Materials" Vision

Looking at this through the lens of a **complete software architecture Bill of Materials**, here's what the Blueprint should ultimately provide:

```
┌───────────────────────────────────────────────────────────┐
│  DECISIONS          "Why we chose this path"               │
│  - Full ADR registry with lifecycle                        │
│  - Supersession chains                                     │
│  - Impact analysis on proposed changes                     │
│  - Linked to affected components/technologies              │
├───────────────────────────────────────────────────────────┤
│  TECHNOLOGIES       "What we build with"                   │
│  - Tech radar with ring lifecycle tracking                 │
│  - Version tracking against latest                         │
│  - Adoption/deprecation dates                              │
│  - Migration guides when moving to Hold                    │
├───────────────────────────────────────────────────────────┤
│  COMPONENTS         "What we build"                        │
│  - Full component registry with owner + status             │
│  - Provides/Consumes interface declarations                │
│  - Link to source code, API definitions                    │
│  - Quality scorecards per component                        │
├───────────────────────────────────────────────────────────┤
│  PATTERNS           "How we build"                         │
│  - Pattern library with rationale + examples               │
│  - Implementation map → which components use them          │
│  - Anti-patterns documented alongside                      │
├───────────────────────────────────────────────────────────┤
│  CONSTRAINTS        "What limits us"                       │
│  - Technical, organizational, philosophical, regulatory    │
│  - Negotiability + review dates                            │
│  - Linked to decisions they drove                          │
├───────────────────────────────────────────────────────────┤
│  QUALITY            "How well we build"                    │
│  - Requirements with measurable targets                    │
│  - Evidence of satisfaction (test results, benchmarks)     │
│  - Trend tracking over time                                │
│  - Improvement campaigns with deadlines                    │
├───────────────────────────────────────────────────────────┤
│  KNOWLEDGE          "What we know" (NEW LAYER)             │
│  - Documentation attached to every node                    │
│  - Architecture principles (durable guidelines)            │
│  - Glossary of project-specific terms                      │
│  - "Why" annotations on relationships                      │
│  - Global search across everything                         │
└───────────────────────────────────────────────────────────┘
```

The current implementation has the **graph-level structure** right — it models all 6 node types with typed relationships. What it's missing is the **depth** within each node type (editing, lifecycle management, documentation) and the **cross-cutting concerns** (search, completeness tracking, history, knowledge base).

---

## 8. Quick Wins — High Impact, Low Effort

These can be done without major architecture changes:

1. **Add Edit + Propose Change buttons to drawer** (~1 hour) — even as "coming soon" toast, it signals intent
2. **Add "Last converged" timestamp to topbar** (~30 min) — API already has timestamps
3. **Add global search to topbar** (~2-3 hours) — filter nodes by name/description across all types
4. **Add completeness indicator per node** (~2 hours) — count filled vs. total fields, show as % badge
5. **Add "Create Node" button** (~3-4 hours) — form modal for creating new Decision/Technology/etc.
6. **Add node deletion** (~1-2 hours) — backend DELETE exists, just needs UI confirm dialog
7. **Show "why" on edges** (~1 hour) — `metadata` field on EdgePayload already exists, just render it

---

*Research based on: Backstage, Structurizr, IcePanel, LeanIX, Port, Cortex, OpsLevel, CycloneDX, SPDX, Syft, FOSSA, adr-tools, Log4brains, Arachne, Thoughtworks Tech Radar, Zalando Tech Radar.*
