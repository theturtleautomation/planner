# Planner UI Reset Phase 04 Sessions Queue Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner UI Reset Phase 03 Project Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-03-project-workspace-spec.md), [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md), [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)  
**Source Research:** [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx), session routing in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), and external research on work queues, priority scanning, and attention management from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Turn `/sessions` into a cross-project attention queue that makes session urgency
and next action legible at a glance.

This route should not feel like a secondary dashboard.
It should feel like a queue the user can clear, resume, or inspect.

## User Outcome

After this slice:

- users can see which sessions need attention first
- resumable, active, blocked, failed, and complete work are easier to separate
- project context stays visible without crowding the queue
- route summaries support scanning instead of competing with rows

## Design Research Synthesis

- queue and task-list research consistently favors the list as the dominant
  surface and discourages summary-card competition above it
- visibility-of-status guidance supports exposing urgency, failure, and next
  action inline with the work object
- recognition-over-recall guidance supports preserving project and workflow
  context inside the row so the user does not need to remember where each
  session came from

Planner implication:

- the row is the product object on this route
- summary metrics should be quiet framing at most
- visible row semantics must do most of the explanatory work

## Locked Decisions

- `/sessions` remains a global, cross-project route
- this page is an attention queue, not the product home
- project identity must remain visible on every row
- backend workflow semantics are unchanged
- no session-detail redesign is included here

## Scope

### In scope

- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
- queue row hierarchy and route framing in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any small queue-support extraction required to make row states clearer

### Out of scope

- changes to session backend capabilities
- shell hierarchy beyond `UIR-00`
- Socratic lobby redesign beyond its contract with the queue

## Current-State Evidence

- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  already contains useful status logic:
  `getPrimaryActionLabel`, `getPrimaryActionTone`, `isActionable`, and
  `needsAttention`
- the page already knows how to distinguish active phases, retry conditions,
  reconnect conditions, and resumability, which means the route has the raw
  semantics required for a real attention queue
- the route still risks summary and support content competing with the rows
  instead of rows carrying most of the meaning

## Proposed UI Model

## Route role

`/sessions` is the cross-project work queue.

It answers:

- what requires action now
- what can be resumed
- what is building
- what failed
- what can be ignored for now

## Dominant surface

The session list must dominate.

Each row should show:

- session title
- project context
- current phase or blockage state
- last activity
- primary action

Rows should make urgency obvious without requiring the user to inspect a
separate summary panel first.

## Supporting surfaces

Supporting surfaces should be compact:

- a quiet queue summary band if needed
- compact filters or grouping controls
- an empty-state explanation when there are no sessions

No peer dashboard-card region should compete with the list.

## Queue semantics

Visible row semantics should be explicit and stable:

- needs attention
- resumable
- in progress
- build running
- failed
- stale
- complete

Those do not require new backend states.
They are presentation groupings over existing route truth.

## Reveal model

- lower-priority metadata such as longer descriptions or workflow step detail
  should appear as secondary text or attached expansion, not as row clutter
- any route-level grouping or filters should remain visible but compact

## State model

The route should explicitly support:

- mixed active queue
- only complete sessions
- no sessions
- load error
- rows with retry, reconnect, resume, and start actions

## Design-System-Patterns Lens

- semantic surfaces:
  primary queue surface, secondary queue framing, dormant detail metadata
- component-state modeling:
  row states must remain legible in normal, hover, loading, and disabled action
  conditions
- theming discipline:
  no fake KPI cards, no traffic-control theater, no status overload outside the
  row object

## Contracts And Touched Surfaces

- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  remains the route owner
- existing queue data contract remains unchanged
- navigation to `/session/:id` or `/session/new` remains unchanged
- touched surfaces:
  [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- `/sessions` reads as an attention queue rather than a secondary dashboard
- session rows are the dominant route object
- urgency and next action are readable from the row without extra hunting
- project context remains visible and helpful
- summary framing, if present, no longer competes with the queue

## Verification Plan

- targeted frontend tests for
  [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  covering:
  - actionable rows
  - failed rows
  - in-progress rows
  - empty state
  - load error
- `npx tsc --noEmit`
- manual verification with mixed session statuses and multiple projects

## Rollback And Fallback

- if grouping rows by urgency is too disruptive in the first pass, preserve the
  flat list but strengthen inline row semantics and demote summary content
- if one small summary band remains necessary, keep it compact and directly tied
  to queue interpretation

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the route reset in
  [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  by replacing the old split hero-summary layout with a quieter queue header and
  explicit grouped queue sections: attention, live/building, ready, quiet,
  complete, and archived.
- Preserved the existing session row semantics and action logic while making
  the grouped list, rather than the summary chrome, carry the route's primary
  explanatory weight.
- Verification completed with:
  `npm test -- src/pages/__tests__/Dashboard.test.tsx`
  and `npx tsc --noEmit`.
