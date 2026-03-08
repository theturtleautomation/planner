# Project-First UI Research Phases

This document turns the requested product and UI changes into a phased research
program.

- Phase 0 is the starting phase.
- Work proceeds in order from `Phase 0 -> Phase 1 -> Phase 2 -> ...`.
- Each phase is one focused research session.
- Each phase ends with its own unique implementation document.
- No two phases share the same implementation document.

The output of this document is not code. The output of this document is a clear
phase tracker that tells us which research pass to run, what code to inspect,
what questions to ask, and which implementation document must be created at the
end of each phase.

## Phase Contract

Every phase should:

- perform code analysis first
- confirm assumptions with the user through Q&A when product intent is unclear
- identify backend, frontend, routing, migration, and test impact
- produce exactly one unique implementation document
- be considered incomplete until that implementation document exists

## Standard Output For Every Phase

Each phase should produce a dedicated implementation document with:

- objective and non-goals
- current-state summary
- proposed behavior
- impacted files and modules
- API and data model changes
- UI and routing changes
- migration or backfill plan
- tests to add or update
- risks, dependencies, and rollout order
- unresolved questions

Suggested output naming:

- `docs/phase-00-...-implementation.md`
- `docs/phase-01-...-implementation.md`
- `docs/phase-02-...-implementation.md`

## Current Code Signals

These findings explain why the current UX feels disconnected:

- Root navigation is still session-first. `/` renders `Dashboard`, and the
  left nav labels `/` as `Sessions`.
  References:
  `planner-web/src/App.tsx`
  `planner-web/src/components/Layout.tsx`
- Knowledge Library is already partially project-first. `/knowledge` shows
  project cards first, and `/knowledge/projects/:projectId` is a scoped view.
  References:
  `planner-web/src/pages/KnowledgeLibraryPage.tsx`
  `planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx`
  `docs/knowledge-library-project-scope-plan.md`
- Sessions do not have a first-class product project relationship. `Session`
  carries `project_description` and a pipeline-facing `cxdb_project_id`, but
  no durable user-facing project container.
  References:
  `planner-server/src/session.rs`
  `planner-web/src/types.ts`
  `planner-server/src/api.rs`
- The pipeline has an internal `ProjectRegistry`, but it is not the same thing
  as a web product project model and is not exposed as the main UI container.
  Reference:
  `planner-core/src/pipeline/project.rs`
- Session events are currently duplicated into the chat stream. The server
  forwards raw events as chat messages with role `event`, the chat panel renders
  them inline, and the session page also renders a separate event panel.
  References:
  `planner-server/src/ws_socratic.rs`
  `planner-web/src/hooks/useSocraticWebSocket.ts`
  `planner-web/src/components/ChatPanel.tsx`
  `planner-web/src/pages/SessionPage.tsx`
- Knowledge filters already exist, but the UI is a chip grid of buttons rather
  than a single horizontal filter bar with dropdowns.
  Reference:
  `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- Generic component naming is still present in generator paths. Unknown groups
  fall back to `X Module`, and component type terminology is still broad and
  technical.
  References:
  `planner-core/src/pipeline/blueprint_emitter.rs`
  `planner-core/src/discovery.rs`
  `planner-web/src/types/blueprint.ts`
  `planner-web/src/components/CreateNodeModal.tsx`
  `planner-web/src/components/EditNodeForm.tsx`

## Phase Sequence

Recommended execution order:

0. Phase 0: Project Ownership Model
1. Phase 1: Root Landing Page And Navigation
2. Phase 2: Naming And Taxonomy Audit
3. Phase 3: Component Naming Strategy
4. Phase 4: Knowledge Filter Bar Redesign
5. Phase 5: Session Lobby And Events Table

Rationale:

- project ownership affects routes, navigation, and session placement
- naming taxonomy should stabilize before deeper UI relabeling
- component naming should follow the taxonomy decisions
- filter and event work can then align to the new project-first shell

## Phase Tracker

| Phase | Research Focus | User Ask Covered | Primary Areas | Unique Output |
| --- | --- | --- | --- | --- |
| 0 | Project Ownership Model | Sessions should attach to a project; project should encapsulate sessions, blueprint, and knowledge | server session model, pipeline project model, routes, shared types | `docs/phase-00-project-ownership-implementation.md` |
| 1 | Root Landing Page And Navigation | Main landing page should be a prompt-like hub with links to projects, knowledge library, events, admin, etc. | app routing, nav shell, homepage IA | `docs/phase-01-root-landing-implementation.md` |
| 2 | Naming And Taxonomy Audit | Analyze names of value types and refactor them | shared types, labels, enums, UI copy, API compatibility | `docs/phase-02-naming-taxonomy-implementation.md` |
| 3 | Component Naming Strategy | Component names like `1 module`, `2 module` are not useful | blueprint emitter, discovery, node naming and rename strategy | `docs/phase-03-component-naming-implementation.md` |
| 4 | Knowledge Filter Bar Redesign | Knowledge filters should become a single horizontal list with dropdowns | knowledge library filters, persistence, responsive behavior | `docs/phase-04-knowledge-filter-bar-implementation.md` |
| 5 | Session Lobby And Events Table | Events should not render in the main display box; use an events table inside the lobby | websocket event flow, session layout, event log rendering | `docs/phase-05-session-lobby-events-implementation.md` |

## Phase 0: Project Ownership Model

### Goal

Define a first-class product project model that becomes the parent container for
sessions, blueprint, knowledge, and project-level events.

### Why This Needs Its Own Research Phase

The current codebase has three different project ideas:

- knowledge scope project IDs such as `proj-alpha`
- pipeline and CXDB UUIDs stored in `cxdb_project_id`
- an internal `ProjectRegistry` in core pipeline code

These are not currently aligned into one product concept.

### Current Code Anchors

- `planner-server/src/session.rs`
- `planner-web/src/types.ts`
- `planner-server/src/api.rs`
- `planner-core/src/pipeline/project.rs`
- `planner-web/src/types/blueprint.ts`
- `docs/knowledge-library-project-scope-plan.md`

### Research Tasks

- inventory every place where `project_id`, `project_name`, or project-like
  scope is created
- decide the canonical project entity for the web product
- decide whether existing knowledge `project_id` strings become the product
  source of truth or need a mapping layer
- define how sessions attach to projects at create time and during retrieval
- define project routes and whether project pages become the new top-level
  working surface
- define project lifecycle rules for archive, duplicate, branch, and ownership
- define migration handling for existing sessions with no project assignment

### User Q&A To Run During Research

- Must every session belong to exactly one project?
- Can a session move between projects after creation?
- Should projects use human-readable slugs, UUIDs, or both?
- Should the existing knowledge project IDs become the canonical project IDs?
- Should project pages expose tabs for `Sessions`, `Blueprint`, `Knowledge`,
  and `Events`?

### Unique Output

- `docs/phase-00-project-ownership-implementation.md`

### Done When

- the project model is defined clearly enough that route, API, and migration
  work can be estimated without guessing

## Phase 1: Root Landing Page And Navigation

### Goal

Replace the current session-first root page with a project-first landing
experience that behaves like a simple prompt hub with clear links into the main
surfaces.

### Why This Needs Its Own Research Phase

This is more than a visual restyle. It changes the product entry point, the nav
model, and likely the meaning of `/`.

### Current Code Anchors

- `planner-web/src/App.tsx`
- `planner-web/src/components/Layout.tsx`
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`

### Research Tasks

- define the new route map for `/`, `/projects`, `/projects/:id`, `/sessions`,
  `/knowledge`, `/events`, and `/admin`
- decide whether `Dashboard` remains a separate page or becomes a project-local
  sessions view
- define what the root prompt actually does
- define which links or quick actions appear on the landing page
- define empty, loading, authenticated, and local-dev states
- define whether the sidebar should become project-first as well

### User Q&A To Run During Research

- Should the root prompt submit a command, a search, or just act as a visual
  entry affordance?
- Should `Projects` replace `Sessions` as the first nav item?
- Should the root page show recent projects or stay minimal?
- Should creating work from the root start a project first or a session first?

### Unique Output

- `docs/phase-01-root-landing-implementation.md`

### Done When

- the final route and navigation model is explicit enough to redesign the shell
  without rework

## Phase 2: Naming And Taxonomy Audit

### Goal

Audit the user-facing names of value types, scopes, statuses, and labels, then
define a cleaner terminology system.

### Why This Needs Its Own Research Phase

This touches shared types, display labels, docs, tests, and possibly wire
contracts. It should be designed once and then applied consistently.

### Current Code Anchors

- `planner-web/src/types.ts`
- `planner-web/src/types/blueprint.ts`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/components/CreateNodeModal.tsx`
- `planner-web/src/components/EditNodeForm.tsx`

### Research Tasks

- inventory all user-visible value types and labels in the web app
- separate internal wire terms from display terms
- flag unclear or overly technical labels such as `quality_requirement`,
  `project_contextual`, `scope visibility`, `module`, and `store`
- define canonical product terminology and a display-label map
- define where aliases are enough and where schema changes are justified
- identify copy updates, tests, and migration implications

### User Q&A To Run During Research

- Which existing labels feel most wrong or least useful besides `module`?
- Do you want API terms cleaned up too, or only UI labels first?
- Do you prefer domain language like `Workstream`, `Feature Area`, and
  `Artifact`, or more literal engineering terms?

### Unique Output

- `docs/phase-02-naming-taxonomy-implementation.md`

### Done When

- there is a complete rename matrix for display terms, internal terms, and
  migration strategy

## Phase 3: Component Naming Strategy

### Goal

Replace weak autogenerated component names with deterministic, useful names that
help users distinguish system parts from the label alone.

### Why This Needs Its Own Research Phase

This needs naming logic, compatibility rules, and likely a backfill plan for
existing blueprint nodes.

### Current Code Anchors

- `planner-core/src/pipeline/blueprint_emitter.rs`
- `planner-core/src/discovery.rs`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/types/blueprint.ts`

### Research Tasks

- inventory every path that creates or mutates component names
- analyze why requirement-group naming falls back to labels like `X Module`
- define naming heuristics using project, feature, artifact, behavior, or file
  context
- separate component display name concerns from component type taxonomy
- define whether existing generic component names should be backfilled
- define how manual edits interact with future regeneration
- identify test fixtures that need stronger naming expectations

### User Q&A To Run During Research

- Should existing generic names be renamed automatically or only new nodes?
- Should manual component names always win over generated names?
- Is there a preferred naming pattern such as `Task Sync Service` or
  `Task Board API`?

### Unique Output

- `docs/phase-03-component-naming-implementation.md`

### Done When

- generator and migration rules are specific enough to prevent another round of
  generic naming

## Phase 4: Knowledge Filter Bar Redesign

### Goal

Convert the current filter chip grid into a single horizontal filter bar with
dropdown controls while preserving scoped filtering behavior.

### Why This Needs Its Own Research Phase

The filters already have persistence and counts. This phase is about changing
interaction design without breaking scoped behavior.

### Current Code Anchors

- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx`
- `planner-web/src/index.css`

### Research Tasks

- inventory current filters, counts, persistence behavior, and deep-link usage
- decide which filters are always visible and which can overflow
- decide whether filters stay single-select or become multi-select
- design the horizontal bar behavior on smaller screens
- define whether counts remain visible inside dropdown options
- define how active filters appear once selected
- identify test updates needed for route persistence and deep-link handling

### User Q&A To Run During Research

- Which filters must stay visible at all times?
- Should dropdowns support multi-select or only single-select?
- Should active filters still render as chips after selection?
- Is there a preferred order for the filter controls?

### Unique Output

- `docs/phase-04-knowledge-filter-bar-implementation.md`

### Done When

- the new control model is specified clearly enough to rebuild the filter UI
  without changing filter semantics accidentally

## Phase 5: Session Lobby And Events Table

### Goal

Separate operational events from the conversational workspace and move event
review into an explicit events table inside the session lobby.

### Why This Needs Its Own Research Phase

The current issue is not just layout. Events are duplicated into the transport
and display layers, so this may require both API-contract and rendering changes.

### Current Code Anchors

- `planner-server/src/ws_socratic.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/components/ChatPanel.tsx`
- `planner-web/src/components/EventLogPanel.tsx`
- `planner-web/src/pages/SessionPage.tsx`

### Research Tasks

- trace how events enter `messages` and `events`
- decide whether `event` chat messages should stop being emitted entirely or
  just stop being rendered in the main conversation pane
- define the session lobby information architecture
- define the events table columns, sort order, filters, and expansion behavior
- decide whether the current `EventLogPanel` is replaced, promoted to a tab, or
  rebuilt as a table component
- define how planner messages, system notices, and event logs differ in the UI
- identify required backend and frontend tests

### User Q&A To Run During Research

- Should event records disappear from the conversation entirely?
- Should the events table be a tab, a split-pane section, or a separate subview?
- Which columns matter most: time, level, source, step, message, duration,
  metadata?
- Should live streaming remain visible while the table is open?

### Unique Output

- `docs/phase-05-session-lobby-events-implementation.md`

### Done When

- the event source of truth and the final session layout are both explicit

## Immediate Next Phase

Start with Phase 0: Project Ownership Model.

Reason:

- it resolves the core mismatch behind "two projects but one session"
- it determines whether the rest of the UI changes are mostly routing work or a
  deeper product model refactor
