# Planner UI Reset Phase 09 Events Timeline Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md), [Planner UI Reset Phase 10 Admin Operations Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md)  
**Source Research:** [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx) and external research on timeline readability, chronological streams, and secondary disclosure from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset Events into a chronological workspace where the event stream is the clear
anchor and secondary controls or snapshot history support it without competing
for first attention.

## User Outcome

After this slice:

- users can read the route as a timeline immediately
- the main event flow is easier to scan for recency and relevance
- snapshots remain available without splitting the page into rival sections
- filters and limits stay useful but quieter

## Design Research Synthesis

- chronological-workspace guidance favors one dominant stream with compact
  filtering and attached detail
- visibility-of-status guidance supports exposing event type, recency, and
  related object identity directly in the stream
- disclosure guidance supports moving secondary historical artifacts into tabs,
  panels, or reveals rather than giving them equal space

Planner implication:

- the stream is the route's truth
- filters are support, not the story
- snapshots should remain reachable without displacing the event flow

## Locked Decisions

- the main event stream remains the primary route object
- snapshot history remains supported on the route
- the route does not become an analytics dashboard or calendar view
- backend event semantics and snapshot APIs remain unchanged

## Scope

### In scope

- [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
- route hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any small extracted event item structure needed for stronger hierarchy

### Out of scope

- backend event generation changes
- admin route redesign beyond coordination with this route
- global observability architecture changes

## Current-State Evidence

- [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  already distinguishes the main stream from `snapshots` using `activeSection`
- the route also owns filtering, result limit, snapshot creation, and deep-link
  logic to Knowledge
- this means the route already has the right ingredients, but it still needs a
  more decisive composition so the stream always reads as primary

## Proposed UI Model

## Route role

Events is the chronological audit-and-activity workspace for blueprint-related
changes.

Its main question is:
"what happened, and what should I inspect next?"

## Dominant surface

The event stream must dominate.

Each event item should make these points easy to scan:

- event type
- timestamp or recency
- affected object
- relevant project or knowledge context when available

## Supporting surfaces

Supporting surfaces should be compact:

- a filter and limit bar
- an explicit snapshots switch or reveal
- attached event detail or contextual deep link affordances

Snapshots should not visually rival the live chronological stream by default.

## Reveal model

- snapshot history should live behind an explicit route mode, tab, or reveal
- richer event payload detail should appear through expansion or attached
  context, not by inflating every event card
- if snapshot creation remains visible, it should stay attached to snapshot
  context rather than the main event reading path

## State model

The route should explicitly support:

- populated event stream
- filtered stream
- snapshots mode
- empty event stream
- loading and fetch failure
- snapshot creation in progress

## Design-System-Patterns Lens

- semantic surfaces:
  one primary stream surface, one secondary control band, one conditional
  snapshots surface
- component-state modeling:
  filtered, loading, and empty states should preserve the stream-first posture
- theming discipline:
  use Planner's tonal hierarchy and avoid activity-feed card clutter

## Contracts And Touched Surfaces

- [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  remains the route owner
- existing APIs remain unchanged:
  `listBlueprintEvents`, `listBlueprintHistory`, and `createBlueprintSnapshot`
- knowledge deep-link behavior remains unchanged
- touched surfaces:
  [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the route reads clearly as a timeline workspace
- the event stream is more visually dominant than filters and snapshots
- event identity and recency are easier to scan inline
- snapshots remain available without becoming an equal-weight peer surface
- loading, empty, and filtered states preserve the same stream-first hierarchy

## Verification Plan

- targeted frontend tests for
  [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  covering:
  - event stream loaded
  - filtered stream
  - snapshots mode
  - empty stream
  - fetch failure
  - snapshot creation state
- `npx tsc --noEmit`
- manual verification with populated mixed event types and knowledge deep links

## Rollback And Fallback

- if snapshots still need stronger presence, keep them as a compact route mode
  rather than restoring a split primary surface
- if event items become too dense, reduce payload detail before adding larger
  multi-column framing

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the bounded stream-readability reset in
  [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  by grouping the filtered event feed into day-based timeline sections instead
  of presenting the whole stream as one uninterrupted list.
- Preserved the existing filter, snapshots, and knowledge deep-link flows while
  making recency and chronology easier to scan from the main stream itself.
- Verification completed with:
  `npm test -- src/pages/__tests__/EventTimelinePage.test.tsx`
  and `npx tsc --noEmit`.
- Verification was refreshed in the tranche audit remediation slice with
  route-specific assertions for grouped timeline sections, filters, snapshots,
  and snapshot creation.
- Residual tranche-correction follow-up work then added direct automated
  coverage for the fetch-failure state named in the route spec.
