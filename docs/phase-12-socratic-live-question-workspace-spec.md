# Phase 12 Socratic Live Question Workspace Spec

**Status:** Implemented and verified on 2026-03-22  
**Date:** 2026-03-22  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md), [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md), and external UX research on feedback, loading, progressive disclosure, and realtime updates  
**Prior Slice:** [Phase 11 Socratic Category Replay And Validation Spec](/home/thetu/planner/docs/phase-11-socratic-category-replay-and-validation-spec.md)

## Objective

Replace the current category-first drill-down as the primary Socratic work
surface with a live question workspace that makes the real intake work visible,
streamed, and understandable.

The current interaction asks users to click into categories and trust that
questions will appear. In practice, category entry can feel like a dead click,
can silently return the user to the main category list, and hides too much of
the active question landscape behind one-branch-at-a-time navigation. This
slice should turn the experience into a realtime planning workspace rather than
an opaque menu tree.

It does **not** replace server-authored Socratic reasoning, move category
generation client-side, or broaden into generic provider-speed work outside the
question-workspace flow.

This slice shipped as a websocket snapshot-first workspace. Full fine-grained
workspace delta messages and warm-question reuse remain optional future
follow-on work if responsiveness or branch-update fidelity still needs another
bounded pass.

## Visual Thesis

Planner’s Socratic intake should feel like a live command floor, not a wizard:
the category map becomes a tense but calm navigation rail, the question library
becomes the dominant working surface, and streamed updates give the page a
sense of active synthesis rather than loading dead air.

## User Outcome

After this slice:

- users can see the active question landscape in one place instead of hunting
  through categories that may or may not open into work
- categories act as filters, grouping, and status cues rather than the only way
  to reach a question
- when Planner is preparing, invalidating, or refreshing question sets, the UI
  says so explicitly
- newly synthesized questions can appear live in the workspace over websocket
  updates without forcing users back through a static tree
- if a category branch collapses or moves because of recent answers, the system
  explains what changed instead of silently ejecting the user to the root list

The user still does **not** get client-authored question content, manual
question-library editing, or collaborative multi-user intake.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- the category tree remains server-authored, but it is demoted from primary
  workspace to navigation-and-status rail
- the primary intake surface becomes a live question workspace that can show
  multiple category groups at once
- the server remains authoritative for all question content, category status,
  build readiness, and workspace deltas
- the client may hold optimistic focus state, but it must not invent prompts,
  categories, or branch outcomes
- websocket push is the default update model for category and question
  workspace changes; do not redesign this around polling
- category clicks should focus or filter the workspace, not feel like route
  transitions into hidden content
- a category that no longer resolves to active question work must produce an
  explicit branch-collapse or branch-moved explanation
- bounded warm question reuse is still allowed, but it is subordinate to the
  workspace model rather than the headline feature
- build/start is allowed from the workspace header when the server marks the
  workspace build-ready; it should no longer depend on returning to a special
  main category screen

## Scope

### In scope

- replacing the current category-drilldown primary UI with a split workspace:
  - category map / status rail
  - live question library / main working pane
  - retained right-side context where appropriate
- websocket-backed workspace snapshots and deltas for categories, question
  groups, focus state, branch preparation, and branch invalidation
- visibility of all currently active question groups, grouped by category and
  searchable or filterable from the category rail
- explicit states for:
  - preparing a question set
  - question set ready
  - question set updated
  - branch collapsed
  - branch moved
  - build ready
- bounded server-side warm preparation of likely next question groups where it
  improves responsiveness
- user-facing explanation when recent answers change where remaining work lives
- focused tests for realtime workspace behavior and no-silent-bounce safety

### Out of scope

- replacing the belief-state model or prompt-adjudication logic
- turning Socratic intake into a generic editable form builder
- cross-session shared question libraries
- collaborative multi-user editing
- broad pipeline UX redesign after intake completion

## Current-State Evidence

- in
  [socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs),
  category entry can resolve to an active leaf, attempt prompt planning, and if
  no prompt is produced, silently pop the active category and rebuild the
  category snapshot instead of explaining what happened
- in
  [category_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/category_planner.rs),
  a node can be treated as `has_prompt_ready` because it is a leaf even though
  the runtime may still fail to produce a displayable scoped prompt
- in
  [useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts),
  `enterCategory()` currently sends a websocket message with no explicit local
  pending or focused-branch state
- in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx),
  the user sees either a category navigator or a prompt panel; there is no
  durable question workspace that reveals the active question landscape
- in
  [CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx),
  the UI can promise “Questions ready” while the overall flow still behaves as
  a menu tree rather than a truthful work surface

## Workspace Model

### Category rail

The left-side category surface should become a live rail, not a destination
screen:

- each category entry should show:
  - title
  - short summary
  - current status
  - question count or readiness state
  - freshness cue such as updated, new, warm, or blocked
- selecting a category should focus the main workspace on that category’s
  question groups
- users should still be able to view all active questions across categories
  without re-entering the tree one branch at a time

### Question library

The main pane should show the real work:

- render all active question groups in a scrollable, dense, grouped workspace
- group by category with clear headers and compact status copy
- allow the user to answer multiple groups without navigating away from the
  broader question landscape
- newly added groups should enter live over websocket updates with explicit
  “new” or “updated” cues

This should feel closer to a live queue of planning work than to a wizard page.

### Context and build readiness

The workspace header should become the authority for session state:

- show current preparation or sync state
- show whether Planner is waiting on synthesis, has active questions, or is
  build-ready
- expose build/start when the server marks the workspace ready
- if a branch collapses after adjudication, show where the remaining work moved

## Realtime Contract

The current singular prompt/category alternation should evolve into a workspace
truth model:

- the server should emit an initial workspace snapshot after connection or
  resume
- the server should emit delta-style updates when:
  - categories change
  - question groups are prepared
  - question groups are invalidated or moved
  - build readiness changes
- websocket updates should be sufficient for the client to keep the visible
  workspace current without a secondary polling loop
- reconnect and checkpoint resume must restore the workspace state rather than
  forcing the user back through a hidden category-transition path

The exact message names may differ in implementation, but the snapshot-plus-
delta model is required.

## Requirements

### No silent bounce-back

The user must never click into a category and quietly land back at the root
with no explanation:

- if question preparation succeeds, the focused workspace must update to show
  the relevant group or groups
- if the branch no longer contains active work, the user must see an explicit
  explanation such as:
  - branch resolved
  - work moved to another category
  - branch is refreshing because the latest answers changed the map

### Live visibility of questions

Users must be able to see the active question field without serial hunting:

- the workspace must support showing all active question groups at once
- category focus may narrow the workspace, but “all active questions” must
  remain available as a first-class mode
- category counts and readiness cues must reflect server truth, not client
  heuristics

### Workspace preparation states

When question content is not yet ready, the UI must still feel alive:

- show preparation state immediately after focus or category selection
- use skeleton or structured placeholders that resemble the eventual question
  group layout
- if preparation is long-running, show meaningful progress copy derived from
  server events rather than a generic spinner
- allow users to continue reviewing other visible question groups while one
  branch is still warming

### Warm reuse and invalidation

Prepared question groups may be reused when safe:

- the server may warm likely next groups for visible or recently focused
  categories
- warm groups must be invalidated on relevant revision, turn, or UI-capability
  change
- reuse should improve responsiveness, but correctness and clarity outrank cache
  hit rate

### Draft and belief-state continuity

The existing right-side context model should remain coherent:

- draft-review content may still appear in the right-side draft context, but
  its actionable review items should participate in the live workspace model
- belief-state and events context should continue to update alongside workspace
  deltas
- the workspace should not hide system-state truth behind ornamental motion or
  ambiguous transitions

## Design Guidance

### Composition

- make the question library the dominant module
- keep the category rail narrow, dense, and always visible
- avoid returning to a full-page category-selector state after the workspace
  has been established
- avoid equal-weight card grids; question groups should read as one operational
  feed with internal sectioning

### Motion

- use immediate state changes on category focus so the rail feels responsive
- use fast skeleton-to-content transitions for prepared question groups
- use streamed insertion for new groups or group updates so synthesis feels
  active rather than page-reload-like

### Copy

- use product-operational language, not chatty assistant language
- name what is happening: preparing, updated, moved, blocked, ready
- never force the user to infer whether the system is loading, recomputing, or
  empty

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-schemas/src/artifacts/socratic.rs](/home/thetu/planner/planner-schemas/src/artifacts/socratic.rs)
- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs)
- [planner-core/src/pipeline/steps/socratic/category_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/category_planner.rs)
- [planner-server/src/ws.rs](/home/thetu/planner/planner-server/src/ws.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)
- [planner-server/src/session.rs](/home/thetu/planner/planner-server/src/session.rs)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/hooks/useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)
- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [planner-web/src/components/CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx)
- [planner-web/src/components/PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
- any new dedicated workspace or category-rail components required to replace
  the current singular prompt-or-category rendering split

Implementation should stay bounded to the Socratic live workspace. If the work
starts broadening into pipeline redesign, collaborative editing, or a wholly
new planning product, stop and split that into a later spec.

## Acceptance Criteria

- users can see all active question groups in one workspace without repeated
  branch-entry hunting
- selecting a category focuses or filters the workspace instead of acting like
  a hidden subpage transition
- category clicks can no longer silently bounce the user back to the main list
  without explanation
- the workspace shows explicit preparing, updated, moved, and ready states
- websocket deltas can add or update visible question groups without requiring
  a manual refresh or full route reset
- build/start is exposed from the workspace header when the server marks the
  workspace build-ready
- warm question reuse improves responsiveness where valid, but stale groups are
  never served as truth

## Verification Plan

### Core and server

- tests proving a category marked as question-bearing cannot silently fall back
  to root without emitting an explicit branch outcome
- tests proving workspace snapshots and deltas stay revision-safe across resume
  and reconnect
- tests proving warm group reuse and invalidation remain correct under belief-
  state and capability changes
- websocket tests proving branch-preparing, branch-ready, branch-moved, and
  workspace-build-ready states are emitted truthfully

### Web

- hook tests proving category focus enters an explicit pending or focused state
- session-page tests proving multiple active question groups can render at once
- tests proving a branch-collapse explanation renders instead of a silent return
  to the root category rail
- tests proving websocket deltas insert or update visible groups without losing
  focus or answer drafts unnecessarily

### Manual

- open the live workspace and confirm active questions are visible without
  category hunting
- select a category and confirm the main pane focuses that group rather than
  blanking into dead air
- trigger a branch refresh and confirm the UI explains whether questions were
  prepared, moved, or removed
- answer a question that causes new work to appear and confirm the new question
  group streams into the workspace live
- verify build/start becomes available from the workspace header when the
  server marks the session ready

## Rollback And Fallback

- if the full multi-group workspace is too large for one delivery slice, keep
  the category rail plus a focused live question pane, but preserve the no-
  silent-bounce rule and websocket-driven branch-state model
- if streaming deltas prove too complex initially, ship a full workspace
  snapshot refresh over websocket first, then add finer-grained deltas later
- if warm reuse introduces correctness risk, fall back to cold preparation with
  explicit preparing states rather than reverting to hidden branch transitions

## Open Questions

None. The slice is ready for bounded implementation.
