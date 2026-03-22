# Planner Design System Phase 4 Utility Route Consistency Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 3 Overlay Depth And Restrained Glass Spec](/home/thetu/planner/docs/planner-design-system-phase-3-overlay-depth-and-restrained-glass-spec.md)  
**Source Research:** user-provided visual audit dated 2026-03-22 plus local Chromium-based manual sweep on 2026-03-22

> Implementation update (2026-03-22): the utility-route command-center cleanup
> is now shipped across the global sessions queue, admin route, shared
> `EventLogPanel`, and non-graph Blueprint chrome. Verification passed via
> targeted Vitest coverage, `npx tsc --noEmit`, and a Vite-served Chromium
> visual sweep of `/sessions`, `/admin`, and `/blueprint` in both themes.

## Objective

Close the most visible command-center rollout gap by migrating the legacy
utility routes and their shared chrome away from border-heavy dashboard
conventions and into the tonal, editorial visual system already proven on Home,
Projects, Session entry, and overlays.

This slice exists because the earlier three phases established a successful
token system, but manual visual verification showed fragmented route-level
adoption on older utility pages.

## User Outcome

After this slice:

- `/sessions` reads like part of the same command-center system as Home and
  Projects instead of a separate legacy dashboard
- `/admin` uses the same calm page zoning, editorial heading hierarchy, and
  tonal containers as the rest of the app while keeping dense operational data
  legible
- utility-route empty states no longer fall back to dashed placeholder boxes
- shared event-log and blueprint chrome no longer rely on structural lines as
  their primary separation device

## In Scope

- the global sessions queue route implemented in
  [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  (`/sessions`; there is no separate `SessionsPage.tsx` in this repo)
- the admin route implemented in
  [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
- the shared legacy event surface in
  [EventLogPanel.tsx](/home/thetu/planner/planner-web/src/components/EventLogPanel.tsx)
- only the non-graph header/sidebar chrome and hygiene badges in
  [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  where visible structural dividers still persist
- token-driven support in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
  if shared utility-route primitives need refinement

## Out Of Scope

- blueprint node visuals, edge rendering, graph layout density, or graph
  interaction redesign
- backend contract changes, route changes, or workflow changes
- a broad restyle of every admin sub-widget if a smaller token or chrome change
  can carry the page
- deep event semantics, filtering behavior, or admin observability product
  behavior
- reopening the earlier modal/drawer blur work beyond incidental inheritance

## Current-State Summary

The design-system foundation works where it has already been applied, but the
manual sweep and code inspection show specific legacy holdouts:

- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  still uses a hard page-header divider, explicit bordered summary cards,
  dashed empty states, and square CTA treatments on the `/sessions` route
- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  still uses monospace-first page framing, repeated `1px solid` card borders,
  and border-bottom row separators as the primary structure language
- [EventLogPanel.tsx](/home/thetu/planner/planner-web/src/components/EventLogPanel.tsx)
  still reads as a boxed log viewer with repeated horizontal rules
- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  still carries a hard internal sidebar divider and top-rule separators in the
  non-graph chrome

## Proposed Behavior

### Sessions route consistency

- convert the `/sessions` route header to the same command-center hierarchy used
  on Home and Projects:
  - kicker
  - display heading
  - supportive copy
- replace the dashed empty state with the shared
  `.empty-state-card` treatment
- migrate route-level CTA styling to shared button classes instead of local
  square buttons
- reduce bordered stat cards and tip boxes to tonal surfaces or ghost-border
  exceptions only where emphasis is still needed

### Admin route consistency

- preserve the dense operational information, but move the page shell away from
  monospace-only framing
- use display hierarchy for the page title while keeping labels, timestamps,
  and row metadata compact and utilitarian
- replace card borders and page-header divider lines with tonal zones, ambient
  elevation, and spacing
- replace row-level structural separators with softer tonal grouping where
  feasible

### Shared event log cleanup

- refactor `EventLogPanel` so its container, filter strip, and expanded rows do
  not depend on repeated hard borders
- preserve explicit severity signaling through tone and badges rather than
  card-boxing every row

### Blueprint chrome cleanup

- remove the hard internal sidebar divider in the non-graph content chrome
- remove simple top-rule separators that can be handled by spacing and tone
- preserve graph readability and performance by keeping the actual graph canvas
  and node styles out of scope

## Implementation Constraints

- keep the slice frontend-only
- do not broaden this into a full admin IA redesign
- preserve light and dark theme parity
- preserve operational readability for dense admin and event data
- prefer shared tokens and classes over large one-off inline restyles when
  feasible
- if a utility surface truly needs a visible outline for comprehension, use a
  ghost-border fallback rather than a return to hard dashboard boxing

## Touched Surfaces

Expected primary files:

- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
- [EventLogPanel.tsx](/home/thetu/planner/planner-web/src/components/EventLogPanel.tsx)
- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
- [index.css](/home/thetu/planner/planner-web/src/index.css)

Expected supporting tests:

- [Dashboard.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/Dashboard.test.tsx)
- [AdminPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/AdminPage.test.tsx)
- [BlueprintPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/BlueprintPage.test.tsx)
- add a targeted `EventLogPanel` test if the shared component structure changes
  materially

## Acceptance Criteria

- `/sessions` uses command-center header hierarchy and no longer uses a dashed
  empty state box
- `/admin` no longer uses a hard horizontal page-header divider as its primary
  sectioning device
- the dominant route-level cards on `/sessions` and `/admin` rely on tonal
  surfaces and spacing more than `1px solid` boxes
- the shared `EventLogPanel` no longer depends on boxed row borders for normal
  readability
- blueprint top chrome and internal non-graph sidebar chrome no longer rely on
  visible structural divider lines where tonal zoning is sufficient
- dense data remains legible in both themes after the border cleanup

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - `/sessions` empty-state and action rendering
  - `/admin` page header and major health/event containers
  - blueprint header/sidebar chrome if visible structure changes materially
  - `EventLogPanel` if row/container structure changes materially
- run:
  - `npm test -- --run src/pages/__tests__/Dashboard.test.tsx src/pages/__tests__/AdminPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx`
  - add an `EventLogPanel` test target if a new test file is created
  - `npx tsc --noEmit`

### Manual

- verify `/sessions` in light and dark theme, including the empty state and at
  least one populated session queue state
- verify `/admin` in light and dark theme with status cards and event rows
- verify `/blueprint` top chrome and sidebar in both themes without reopening a
  graph-density redesign
- verify the resulting utility routes feel aligned with Home and Projects
  without losing dense operational readability

## Rollback And Fallback

- if one dense admin block becomes ambiguous without borders, reintroduce a
  ghost-border or slightly stronger tonal contrast only for that block
- if blueprint chrome cleanup risks graph usability, keep the route-level
  header cleanup and defer the sidebar chrome adjustment
- if the shared `EventLogPanel` cannot adopt the new treatment cleanly, keep
  route-level changes local and document the component as a bounded follow-on

## Open Questions

None blocking readiness.

The route targets, legacy border patterns, and verification surface are all
concrete enough to support bounded frontend implementation.
