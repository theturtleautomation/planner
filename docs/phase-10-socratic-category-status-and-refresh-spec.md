# Phase 10 Socratic Category Status And Refresh Spec

**Status:** Implemented  
**Date:** 2026-03-21  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)  
**Prior Slice:** [Phase 09 Socratic Recursive Category Synthesis Spec](/home/thetu/planner/docs/phase-09-socratic-recursive-category-synthesis-spec.md)

## Objective

Make the main Socratic category screen trustworthy after repeated drill-down by
turning the category snapshot into a clear progress and refresh surface rather
than just a list of titles.

This slice should let users return from a category and immediately understand:

- what changed
- which categories are complete or still active
- whether new categories appeared because of their recent answers
- why build/start is or is not available from the main screen

It does **not** redesign the prompt-answering UI or replace server-owned
category synthesis with client-side heuristics.

## User Outcome

After this slice:

- categories can show meaningful status transitions such as pending, active,
  ready, complete, or blocked based on current interview state
- the refreshed main category screen can surface newly introduced categories
  when answers open new lines of questioning
- users can see whether a category is still worth visiting before they click
  into it
- the main screen can explain build gating instead of only exposing a boolean
  `build_ready`

The user still does **not** get predictive completion percentages, timeline
analytics, or automatic category prioritization based on inferred preference.

## Implementation Notes

Implemented on 2026-03-21 in the bounded Phase 10 delivery slice.

Execution landed in:

- `planner-schemas/src/artifacts/socratic.rs`
- `planner-core/src/pipeline/steps/socratic/category_planner.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-web/src/types.ts`
- `planner-web/src/components/CategoryNavigator.tsx`
- `planner-web/src/components/__tests__/CategoryNavigator.test.tsx`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`
- `planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `planner-tui/src/app.rs`

Delivered behavior:

- category snapshots now carry `newly_available_category_ids` and a
  `build_readiness_message`
- visible categories are marked with durable server-authored statuses:
  `ready`, `blocked`, `active`, and `complete` as appropriate for the current
  screen state
- the planner now preserves recently disappeared visible categories as
  `complete` placeholders instead of silently dropping them from the refreshed
  screen
- lower-priority main-screen roots are surfaced as `blocked` when a higher
  priority group still exists
- web and TUI category surfaces now render build guidance and newly opened
  categories directly from the snapshot rather than inferring from hidden state

Verification completed:

- `cargo test -p planner-core category_planner -- --nocapture`
- `cargo test -p planner-tui tick_socratic_category_state_shows_active_branch_children -- --nocapture`
- `cargo test -p planner-server ws_socratic_io_ -- --nocapture`
- `npm test -- --run src/components/__tests__/CategoryNavigator.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx` in `planner-web/`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- category statuses remain server-derived from the current interview state
- clients render status semantics and refresh cues but do not infer them
- `build_ready` remains the final build gate, but the snapshot should include
  enough supporting detail to explain why it is true or false
- newly emerged categories should be visible when the user returns to the main
  category screen after answering prompts
- status semantics should work for both the current bounded hierarchy and the
  recursive tree introduced by Phase 09

## Scope

### In scope

- define durable semantics for `pending`, `active`, `ready`, `complete`, and
  `blocked`
- add snapshot fields needed to explain refreshes and build gating
- mark newly surfaced categories after recent prompt adjudication
- surface category status and refresh cues in web and TUI
- focused tests proving status transitions and refreshed main-screen behavior

### Out of scope

- score-based prioritization or recommendation engines
- historical trend charts for interview coverage
- automatic category collapsing or hiding based on user behavior
- non-Socratic pipeline execution changes

## Current-State Evidence

- Phase 08 introduced category snapshots and a category-status enum
- the current implementation uses only a minimal subset of status behavior and
  does not clearly explain refresh semantics or build gating after returning to
  the main screen
- the project plan already identifies richer category-status semantics as a
  likely follow-on investment

## Requirements

### Status semantics

The server must assign category status consistently:

- `pending` means the category exists but is not the most immediate actionable
  path yet
- `active` means the category is currently selected or otherwise directly in
  focus
- `ready` means the category has prompt-ready work available now
- `complete` means the category subtree no longer has unresolved interview work
  worth surfacing
- `blocked` means the category cannot progress until another higher-priority or
  prerequisite path changes

### Refresh semantics

The main category snapshot must explain change over time:

- the snapshot should indicate when new categories appeared after recent
  answers
- the snapshot should support plain-language UI copy for refreshed categories
  without requiring the client to diff arbitrary trees heuristically
- returning to the main screen after prompt submission must expose the newest
  status truth before the user chooses the next category

### Build-gating explanation

The snapshot must support user-facing gating copy:

- if build is not ready, the main screen should be able to explain the blocking
  reason at a high level such as unresolved contradiction, missing required
  dimension coverage, or still-available verification work
- if build is ready, the main screen should make clear that further category
  exploration is optional rather than still required

### Client behavior

Web and TUI should stay thin:

- clients render status and refresh metadata supplied by the server
- clients do not compute build gating reasons from raw belief-state internals
- the status surface must remain compact enough that the main category screen
  still feels like navigation rather than an analytics dashboard

## Dependencies And Touched Surfaces

This slice should build on the category-snapshot work in Phase 08 and the
recursive hierarchy work in Phase 09.

Likely touched surfaces:

- [planner-schemas/src/artifacts/socratic.rs](/home/thetu/planner/planner-schemas/src/artifacts/socratic.rs)
- [planner-core/src/pipeline/steps/socratic/category_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/category_planner.rs)
- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-server/src/ws.rs](/home/thetu/planner/planner-server/src/ws.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)
- [planner-tui/src/app.rs](/home/thetu/planner/planner-tui/src/app.rs)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/components/CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx)
- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

Implementation should stay bounded to status truth and refresh explanation. If
the work starts expanding into analytics, prioritization, or recommendation UI,
stop and split that into a later spec.

## Acceptance Criteria

- category snapshots expose durable semantics for all supported category status
  values
- the main category screen can identify newly emerged categories after prompt
  answers
- users can see why build/start is unavailable without inspecting hidden
  belief-state internals
- users can see when build/start is available and further exploration is
  optional
- web and TUI surfaces remain server-driven and compact

## Verification Plan

### Shared and core

- tests proving status transitions across prompt adjudication and category
  recomputation
- tests proving newly emerged categories are marked distinctly on refreshed main
  snapshots
- tests proving build-gating explanation data is present when `build_ready` is
  false

### Server

- websocket tests proving refreshed snapshots include status and gating detail
  after prompt submission and `back_to_categories`

### Web and TUI

- web tests proving status badges and build-gating copy render correctly
- TUI tests proving refreshed category lists surface new and blocked states
  without ambiguity

## Rollback And Fallback

- if status semantics prove too noisy, keep the server-owned truth but reduce
  UI rendering to a smaller subset of stable labels rather than dropping the
  underlying metadata
- if precise gating reasons are difficult to expose initially, fall back to a
  compact high-level reason string rather than surfacing raw belief-state
  details

## Open Questions

None. The slice is ready for bounded implementation.
