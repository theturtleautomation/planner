# Living System Blueprint — Specification

**Version:** 1.0-draft  
**Date:** March 5, 2026  
**Status:** Awaiting sign-off  
**Context:** Planner v2 — an AI system builder, knowledge system, and artifact library

---

## 1. Problem Statement

Planner's convergence process generates design decisions, technology selections, component definitions, architecture patterns, and constraints. Today these exist as ephemeral outputs of convergence sessions — invisible after the session ends, uneditable, and disconnected from the artifacts they produced.

**The user's requirement:** every parameter that influences a system's design and architecture must be visible, editable, and reactive. Editing any parameter triggers AI reconvergence of affected downstream artifacts.

No existing tool does this. The closest analogues:
- **Structurizr** links ADRs to C4 model elements, but decisions aren't editable parameters that trigger re-computation
- **Terraform** has the edit-plan-apply pattern with impact preview, but for infrastructure, not system design
- **Design token systems** cascade parameter changes through a dependency graph, but for visual properties, not architectural decisions
- **Product configurators** (Tacton, SAP) resolve constraint-based configurations reactively, but for physical products

The Living System Blueprint is the novel synthesis: a parameterized, reactive system anatomy where **the specification IS the system**.

---

## 2. Core Concept

The Blueprint is a **typed dependency graph** where every node is an editable parameter that influences system design. Editing any node triggers:

1. **Impact preview** — what will change (Terraform plan pattern)
2. **User confirmation** — approve/reject the cascade
3. **AI reconvergence** — affected nodes are re-evaluated and downstream artifacts rebuilt

The graph is not a static diagram. It's a live, queryable, editable model of *why the system is the way it is*.

---

## 3. Node Taxonomy

Every node in the Blueprint is one of six types. These map to the research findings across C4, arc42, Backstage, ThoughtWorks Radar, and ADR tooling.

### 3.1 Decision

An architectural choice with rationale. Derived from ADR format (MADR variant).

| Field | Type | Description |
|---|---|---|
| id | string | Unique identifier (auto-generated, e.g., DEC-017) |
| title | string | Imperative statement: "Use MessagePack for disk serialization" |
| status | enum | proposed → accepted → superseded / deprecated |
| context | text | Problem statement and constraints driving this decision |
| options | Vec\<Option\> | Considered alternatives, each with pros/cons |
| chosen | ref → Option | The selected option with justification |
| consequences | Vec\<Consequence\> | Positive and negative outcomes |
| assumptions | Vec\<Assumption\> | Unstated beliefs embedded in this decision (with confidence rating) |
| supersedes | ref → Decision? | Previous decision this replaces |
| tags | Vec\<string\> | Freeform classification |

**Examples in Planner:** "Use Rust for the core engine", "MessagePack over SQLite for CXDB", "Gemini CLI over HTTP API", "Event sourcing for session history"

### 3.2 Technology

A specific technology, framework, library, or tool used in the system. Inspired by ThoughtWorks Radar blips and Backstage Component metadata.

| Field | Type | Description |
|---|---|---|
| id | string | e.g., TECH-005 |
| name | string | "Rust", "Tokio", "MessagePack", "Gemini CLI" |
| version | string? | Current version in use |
| category | enum | language / framework / library / runtime / tool / platform / protocol |
| ring | enum | adopt / trial / assess / hold — adoption posture (ThoughtWorks pattern) |
| rationale | text | Why this technology was chosen |
| alternatives | Vec\<ref → Technology\> | Known alternatives (for quick comparison) |
| license | string? | FOSS license type |
| decided_by | ref → Decision | The decision that selected this technology |

**Examples in Planner:** Rust (language), Tokio (async runtime), Ratatui (TUI framework), Axum (web framework), MessagePack (serialization), Gemini CLI (LLM client)

### 3.3 Component

A logical building block of the system — a module, service, subsystem, or significant code boundary. Maps to C4 Container/Component and Backstage Component.

| Field | Type | Description |
|---|---|---|
| id | string | e.g., COMP-003 |
| name | string | "Convergence Engine", "CXDB", "TUI", "Web UI" |
| type | enum | module / service / library / store / interface / pipeline |
| description | text | What this component does |
| technologies | Vec\<ref → Technology\> | Technologies this component uses |
| depends_on | Vec\<ref → Component\> | Runtime/build dependencies |
| provides | Vec\<string\> | APIs/interfaces this component exposes |
| consumes | Vec\<string\> | APIs/interfaces this component uses |
| decided_by | Vec\<ref → Decision\> | Decisions that created or shaped this component |
| status | enum | planned / in_progress / shipped / deprecated |

**Examples in Planner:** ConvergenceEngine, SessionStore, EventStore, CXDB, PlanExecutor, ToolRegistry, TUI, WebUI, LLMRouter

### 3.4 Constraint

An external force that narrows the solution space — technical, organizational, or philosophical. Maps to arc42 Section 2.

| Field | Type | Description |
|---|---|---|
| id | string | e.g., CON-002 |
| title | string | "LLM clients must use native CLIs, not HTTP API keys" |
| type | enum | technical / organizational / philosophical / regulatory |
| description | text | Full explanation of the constraint and its origin |
| source | string | Who/what imposed this constraint ("user directive", "platform limitation", etc.) |
| affects | Vec\<ref → Decision \| Component \| Technology\> | What this constraint restricts |

**Examples in Planner:** "No HTTP API keys for LLM access", "File-system storage over SQLite", "No stubs or unimplemented code", "Sandboxed execution over yolo flags", "Both TUI and Web required"

### 3.5 Pattern

An architectural pattern, design principle, or structural approach used across the system. Maps to arc42 Section 8 (Crosscutting Concepts).

| Field | Type | Description |
|---|---|---|
| id | string | e.g., PAT-004 |
| name | string | "Event Sourcing", "Memory-first disk-backed", "Factory + Convergence" |
| description | text | What this pattern is and how it applies |
| rationale | text | Why this pattern was chosen |
| applies_to | Vec\<ref → Component\> | Components implementing this pattern |
| decided_by | ref → Decision? | The decision that selected this pattern |

**Examples in Planner:** Event sourcing for session history, Factory pattern for convergence, RunBudget circuit breaker, Optimistic UI with server ack, Draft confirmation state

### 3.6 Quality Requirement

A measurable quality attribute the system must satisfy. Maps to arc42 Section 10.

| Field | Type | Description |
|---|---|---|
| id | string | e.g., QR-001 |
| attribute | enum | performance / reliability / security / usability / maintainability |
| scenario | text | Specific, testable scenario: "Session recovery on restart completes within 2 seconds for 1000 sessions" |
| priority | enum | critical / high / medium / low |
| satisfied_by | Vec\<ref → Decision \| Pattern\> | What achieves this requirement |

---

## 4. Edge Taxonomy

Edges in the graph are typed and directional. Every edge has a semantic meaning.

| Edge Type | Source → Target | Meaning |
|---|---|---|
| decided_by | Technology, Component, Pattern → Decision | "This exists because of that decision" |
| supersedes | Decision → Decision | "This decision replaces that one" |
| depends_on | Component → Component | "This component needs that component at runtime/build" |
| uses | Component → Technology | "This component is built with that technology" |
| constrains | Constraint → Decision / Component / Technology | "This constraint limits those choices" |
| implements | Component → Pattern | "This component follows that pattern" |
| satisfies | Decision / Pattern → Quality Requirement | "This decision/pattern achieves that quality goal" |
| affects | Decision → Component / Technology | "Changing this decision impacts those things" |

---

## 5. The Reactive Loop

When a user edits a node, the system follows the **edit-plan-apply** pattern borrowed from Terraform, adapted for AI reconvergence.

### Step 1: Edit
User modifies a node. Examples:
- Changes a Decision's chosen option (e.g., "Switch from MessagePack to SQLite for CXDB")
- Changes a Technology's version or replaces it entirely
- Adds, removes, or modifies a Constraint
- Changes a Component's structure or dependencies

### Step 2: Impact Preview (Plan)
The system traverses the dependency graph downstream from the edited node and generates an impact report:

```
Impact Plan for: DEC-008 "Use MessagePack for disk serialization" → "Use SQLite"
═══════════════════════════════════════════════════════════════════════════════

Summary: 2 reconverge, 3 update, 1 invalidate, 1 new required

~ COMP-004: CXDB                            [RECONVERGE]  ⚠
  Storage implementation must be rewritten for SQL schema
  
~ COMP-005: EventStore                      [RECONVERGE]  ⚠
  Event persistence format changes from msgpack to SQL rows

~ TECH-011: rmp-serde                       [REMOVE]      ✗
  No longer needed — replaced by SQLite driver

+ TECH-new: rusqlite                        [ADD]         ℹ
  Required: SQLite driver for Rust

~ PAT-002: Memory-first disk-backed         [UPDATE]      ~
  Pattern unchanged but implementation strategy differs

~ DEC-012: Atomic writes via tempfile       [INVALIDATE]  ⛔
  SQLite handles atomicity internally — decision no longer applies

~ QR-003: Crash-safe persistence            [UPDATE]      ~
  SQLite provides ACID; verification approach changes
```

Each affected node shows:
- **Action symbol:** `~` update, `+` add, `✗` remove, `⛔` invalidate (borrowed from Terraform)
- **Severity classification:** RECONVERGE (AI work required), UPDATE (metadata change), INVALIDATE (downstream decision broken), ADD/REMOVE (graph topology change)
- **Human-readable explanation:** Why this node is affected

### Step 3: Confirm
User reviews the plan. Options:
- **Apply & Reconverge** — commit the change, AI begins reconvergence work on affected nodes
- **Modify** — adjust the proposed change before applying
- **Cancel** — discard the change entirely

### Step 4: Reconverge
For each node marked RECONVERGE:
1. The AI re-evaluates the node given the new upstream state
2. Research is conducted if needed (new technology evaluation, pattern alternatives)
3. The node is updated with new values
4. Downstream nodes of THIS node are checked for cascading impact
5. The process repeats until no more nodes need reconvergence (convergence)

This is the same topological-order cascade as Excel's dirty-cell recalculation or Terraform's apply-in-dependency-order.

### Step 5: Report
After reconvergence completes, the user sees:
- Summary of all changes made
- Any new decisions the AI made during reconvergence (requiring user review)
- Updated Blueprint graph showing the new state
- Diff of artifacts that were regenerated

---

## 6. Storage Model

The Blueprint is stored as part of CXDB (Planner's existing persistence layer), using MessagePack on disk.

### Schema

```
{data_dir}/blueprint/
  ├── nodes/
  │   ├── decisions/
  │   │   ├── DEC-001.msgpack
  │   │   └── DEC-002.msgpack
  │   ├── technologies/
  │   │   ├── TECH-001.msgpack
  │   │   └── ...
  │   ├── components/
  │   ├── constraints/
  │   ├── patterns/
  │   └── quality_requirements/
  ├── edges.msgpack          # All edges in a single file (small enough)
  ├── graph_index.msgpack    # Precomputed adjacency lists for traversal
  └── history/
      ├── 2026-03-05T10-30-00Z.msgpack  # Snapshot before edit
      └── ...
```

### In-Memory Model

```rust
struct Blueprint {
    nodes: HashMap<NodeId, BlueprintNode>,
    edges: Vec<Edge>,
    // Precomputed indexes
    forward_adj: HashMap<NodeId, Vec<(EdgeType, NodeId)>>,  // outgoing
    reverse_adj: HashMap<NodeId, Vec<(EdgeType, NodeId)>>,  // incoming
}

enum BlueprintNode {
    Decision(Decision),
    Technology(Technology),
    Component(Component),
    Constraint(Constraint),
    Pattern(Pattern),
    QualityRequirement(QualityRequirement),
}

struct Edge {
    source: NodeId,
    target: NodeId,
    edge_type: EdgeType,
    metadata: Option<String>,  // e.g., "technology choice" for a decided_by edge
}
```

### Persistence Strategy
Same as the session persistence architecture: memory-first, periodic flush (5s), crash-safe atomic writes via tempfile, startup load from disk.

---

## 7. UI Integration — The Blueprint Tab

The Blueprint integrates into the existing Registry layout (approved in mockups v2) as a primary navigation destination.

### 7.1 Navigation

The sidebar gains a "Blueprint" entry alongside existing registry categories (Tools, Plans, Research, etc.). The Blueprint is not a separate app — it's a first-class view within the Registry.

```
┌─────────────────────┬──────────────────────────────────────────────┐
│  PLANNER             │                                              │
│                      │                                              │
│  📋 Registry        │   [Blueprint content area]                   │
│    🔧 Tools         │                                              │
│    📝 Plans         │                                              │
│    🔬 Research      │                                              │
│    📊 Analyses      │                                              │
│    ⚡ Scripts       │                                              │
│                      │                                              │
│  🏗️ Blueprint       │                                              │
│    Decisions (12)   │                                              │
│    Technologies (8) │                                              │
│    Components (9)   │                                              │
│    Constraints (5)  │                                              │
│    Patterns (6)     │                                              │
│    Quality (4)      │                                              │
│                      │                                              │
│  ⚙️ Sessions        │                                              │
└─────────────────────┴──────────────────────────────────────────────┘
```

### 7.2 Blueprint Views

Three views, switchable via tabs at the top of the content area:

#### Graph View (default)

An interactive, zoomable force-directed graph showing all Blueprint nodes and edges.

- **Node rendering:** Shape encodes type (rounded rect = Decision, hexagon = Technology, square = Component, diamond = Constraint, oval = Pattern, shield = Quality Requirement)
- **Node color:** Status-driven (green = active/accepted, amber = in_progress/trial, gray = deprecated/superseded, blue = planned)
- **Edge rendering:** Styled by type (solid = depends_on, dashed = decided_by, dotted = constrains)
- **Interaction:** Click node → slide-out detail drawer. Hover → show node summary tooltip. Click edge → highlight the relationship with explanation.
- **Filtering:** Toggle node types on/off. Filter by status. Search by name.
- **Layout:** Force-directed by default, with option for hierarchical (top-down by dependency depth)
- **Clustering:** Components are visually grouped if they share a parent system/module

#### Table View

A filterable, sortable table of all Blueprint nodes — similar to Backstage catalog or Cortex entity list.

| Name | Type | Status | Depends On | Decided By | Tags |
|---|---|---|---|---|---|
| CXDB | Component | shipped | EventStore, SessionStore | DEC-008 | storage, core |
| MessagePack | Technology | adopt | — | DEC-008 | serialization |
| ... | ... | ... | ... | ... | ... |

Columns are sortable, filterable. Click a row → same detail drawer as graph view.

#### Radar View

A ThoughtWorks-style technology radar showing all Technology nodes, positioned by ring (adopt/trial/assess/hold) and grouped by category (language/framework/library/tool/platform).

This view is read-only — it's a snapshot of technology posture. Clicking a blip opens the Technology detail drawer where edits can be made.

### 7.3 Detail Drawer

When a node is selected (from any view), a drawer slides in from the right showing the full node data. All fields are inline-editable.

Decision drawer example:

```
┌──────────────────────────────────────────────────┐
│ DEC-008: Use MessagePack for disk serialization  │
│ Status: [Accepted ▼]    Created: 2026-03-01      │
│──────────────────────────────────────────────────│
│                                                    │
│ Context                                            │
│ ┌────────────────────────────────────────────────┐│
│ │ CXDB needs a fast, compact disk format for     ││
│ │ session and event persistence. Must support    ││
│ │ Rust's serde ecosystem. ...                    ││
│ └────────────────────────────────────────────────┘│
│                                                    │
│ Options Considered                                 │
│ ● MessagePack ← chosen                            │
│   + Fast binary serialization                      │
│   + Compact on disk                                │
│   + Native serde support                           │
│   - Not human-readable                             │
│ ○ SQLite                                           │
│   + ACID transactions                              │
│   + Query capability                               │
│   - Heavier runtime                                │
│ ○ JSON                                             │
│   + Human-readable                                 │
│   - Slow, large files                              │
│                                                    │
│ Consequences                                       │
│ + Minimal deserialization overhead                  │
│ - Cannot query data without loading entire file     │
│                                                    │
│ ──── Relationships ────                            │
│ Affects:                                           │
│   COMP-004: CXDB                                   │
│   COMP-005: EventStore                             │
│   TECH-011: rmp-serde                              │
│ Constrained by:                                    │
│   CON-002: File-system storage over SQLite         │
│                                                    │
│ ──── Assumptions ────                              │
│ ⚠ Data volumes stay small enough for full-file     │
│   reads (confidence: medium)                       │
│ ✓ Rust serde ecosystem remains stable              │
│   (confidence: high)                               │
│                                                    │
│ [Edit Decision]  [Propose Change]  [View History]  │
└──────────────────────────────────────────────────┘
```

"Propose Change" triggers the Impact Preview workflow (Section 5).

### 7.4 Impact Preview Modal

When a change is proposed, a modal overlays the screen showing the impact plan (Section 5, Step 2). The modal includes:

- Summary line (counts by severity)
- Scrollable list of affected nodes with action symbols and explanations
- Before/after diff for nodes with value changes
- Color-coded severity (green = safe update, amber = reconverge needed, red = invalidation)
- Action buttons: Cancel / Modify / Apply & Reconverge

---

## 8. Population Strategy

The Blueprint doesn't start empty. It's populated from two sources:

### 8.1 Convergence Extraction
During convergence, every question asked and answered becomes a Decision node. Every technology selected becomes a Technology node. Components are extracted from the system design. Constraints come from user directives.

The convergence engine will be extended to emit Blueprint nodes as a side effect of convergence.

### 8.2 Retroactive Import
For the existing Planner codebase, we do a one-time import:
1. Walk the existing convergence artifacts and spec documents
2. Extract decisions, technologies, components, constraints, and patterns
3. Build the initial graph with edges
4. Present to the user for review and correction

### 8.3 Ongoing Maintenance
- **During convergence:** New nodes and edges are added automatically
- **During development:** When code changes, the AI can detect if a Blueprint node's implementation has drifted from its specification
- **Manual editing:** Users can always add, edit, or remove nodes directly through the UI

---

## 9. Reconvergence Scope

When a node changes, the reconvergence engine must determine what to re-evaluate. The scope depends on the node type and the nature of the change.

| Node Type | Change Type | Reconvergence Scope |
|---|---|---|
| Decision | Chosen option changed | All downstream: Components using this decision, Technologies selected by it, Patterns it implemented |
| Decision | New option added (no selection change) | None — information only |
| Technology | Replaced with alternative | Components using this tech, Decisions referencing it |
| Technology | Version change | Components using this tech (compatibility check) |
| Component | Restructured / split / merged | Dependent components, Decisions scoping to it |
| Constraint | Added | All decisions in scope — re-validate compliance |
| Constraint | Removed | Decisions it constrained — may open new options |
| Pattern | Changed | Components implementing it |
| Quality Req | Scenario tightened | Decisions/patterns satisfying it — re-validate |

The depth of reconvergence is bounded:
- **Shallow (metadata only):** Status change, description update → update node, no cascade
- **Medium (local reconverge):** Technology version bump → check compatibility of using components
- **Deep (full cascade):** Decision reversal → full downstream traversal, potential artifact rebuilds

---

## 10. What This Is NOT

To prevent scope creep and maintain clarity:

- **Not a monitoring dashboard.** The Blueprint shows design intent, not runtime health.
- **Not a deployment topology.** C4 deployment diagrams are orthogonal. The Blueprint can link to one but doesn't replace it.
- **Not a project management tool.** It doesn't track sprints, tickets, or velocity. It tracks *why the system is the way it is*.
- **Not a CMDB.** It doesn't inventory every server and license. It tracks the decisions and technologies that shaped the architecture.
- **Not a static document.** It's a live, reactive, queryable graph that drives AI behavior.

---

## 11. Implementation Phases

### Phase 1: Data Model & Storage
- Define Rust structs for all node types and edges
- Implement Blueprint store (CXDB pattern: memory-first, disk-backed)
- Implement graph traversal (forward/reverse adjacency, topological sort)
- Implement impact analysis (given a node change, compute affected set)

### Phase 2: Convergence Integration
- Extend convergence engine to emit Blueprint nodes
- Map convergence questions → Decision nodes
- Map technology selections → Technology nodes
- Map component architecture → Component nodes
- Map user directives → Constraint nodes

### Phase 3: UI — Graph & Table Views
- Implement Blueprint sidebar navigation in Registry
- Build interactive graph view (force-directed, filterable, clickable)
- Build table view (sortable, filterable)
- Build detail drawer (inline-editable)
- Build radar view for technologies

### Phase 4: Reactive Loop
- Implement "Propose Change" workflow
- Build impact preview engine
- Build impact preview modal UI
- Implement reconvergence trigger (AI loop that re-evaluates affected nodes)
- Implement reconvergence result reporting

### Phase 5: Retroactive Import
- Build extraction pipeline for existing Planner codebase
- Generate initial Blueprint graph
- User review and correction workflow

---

## 12. Open Questions for Sign-Off

1. **Node ID format:** Auto-generated sequential (DEC-001) vs. slug-based (dec-use-messagepack) vs. UUID? Sequential is most readable in impact reports.
2. **History granularity:** Full snapshot on every edit, or event-sourced (store the edits, reconstruct state)? Event-sourced is more storage-efficient and preserves the full edit chain. Aligns with existing EventStore pattern.
3. **Reconvergence autonomy:** When the AI reconverges a downstream node, should it auto-accept its own decisions, or always surface them for user review? Recommendation: auto-accept for shallow/medium changes, require review for deep changes.
4. **Blueprint scope:** One Blueprint per project, or one global Blueprint? Recommendation: one per project (each Planner project has its own system anatomy).
5. **Graph rendering in TUI:** The TUI gets a simplified version (ASCII graph? Table-only?) while the Web UI gets the full interactive force-directed graph. How much parity is required?

---

## Sources

- **Architecture documentation:** [C4 Model](https://c4model.com/), [Structurizr DSL](https://structurizr.com/), [arc42](https://arc42.org/), [Backstage System Model](https://backstage.io/docs/features/software-catalog/system-model)
- **Tech catalogs:** [ThoughtWorks Radar](https://www.thoughtworks.com/en-us/radar), [Backstage Catalog](https://backstage.io/docs/features/software-catalog/), [Port Blueprints](https://docs.getport.io/), [Cortex Scorecards](https://www.cortex.io/)
- **Reactive specs:** [Terraform Dependency Graph](https://developer.hashicorp.com/terraform/internals/graph), [Style Dictionary Architecture](https://styledictionary.com/info/architecture/), [CUE Language](https://cuelang.org/docs/introduction/), [Excel Recalculation](https://learn.microsoft.com/en-us/office/client-developer/excel/excel-recalculation)
- **Decision tooling:** [ADR ecosystem](https://adr.github.io/), [MADR Template](https://ozimmer.ch/practices/2022/11/22/MADRTemplatePrimer.html), [Log4brains](https://github.com/thomvaill/log4brains)
- **Impact analysis:** [Terraform Plan](https://developer.hashicorp.com/terraform/internals/json-format), [ControlMonkey](https://controlmonkey.io/resource/terraform-plan-made-simple/)
