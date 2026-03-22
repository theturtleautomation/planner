# Planner UI Reset Phase 07 Blueprint Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner Design System Phase 8 Blueprint Command Chrome And Inspector Spec](/home/thetu/planner/docs/planner-design-system-phase-8-blueprint-command-chrome-and-inspector-spec.md), [Planner UI Reset Phase 06 Knowledge Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md)  
**Source Research:** [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx), [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx), and external research on canvas workspaces, inspector hierarchy, and command disclosure from Nielsen Norman Group, Fluent, Carbon, and Material

## Objective

Reset Blueprint into a clearer canvas workspace in a first bounded pass where
the graph or active view is obviously primary when the route opens.

This delivered slice does not claim a full command-band or inspector rewrite.
It establishes the graph-first opening posture and leaves broader chrome
consolidation as later follow-on work if still needed.

## User Outcome

After this slice:

- the user can tell what the primary canvas is immediately when the route opens
- graph mode becomes the default opening posture instead of overview mode
- existing alternate views remain available without changing the route contract
- current selection detail and tool surfaces remain intact while the opening
  posture becomes more decisive

## Design Research Synthesis

- canvas-workspace research supports giving the canvas the most space and the
  least ambiguity
- inspector patterns work best when the inspector is stable and clearly tied to
  current selection rather than functioning as a competing content page
- disclosure guidance supports compact command bands with lower-priority tools
  moved into menus, panels, or secondary controls

Planner implication:

- the active blueprint canvas or mode must dominate
- command chrome should guide action without stealing focus
- the inspector should read as selection authority

## Locked Decisions

- graph performance remains first-class
- the graph stays primary when graph mode is active
- overview, traceability, dependencies, and table remain supported views
- the route does not become a flashy graph theater surface
- backend blueprint semantics and layout algorithms are unchanged

## Scope

### In scope

- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- blueprint-specific hierarchy and control styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

### Out of scope

- graph layout algorithm changes
- node or edge schema changes
- discovery or knowledge route redesign

## Current-State Evidence

- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  already manages multiple true modes:
  `overview`, `traceability`, `dependencies`, `graph`, and `table`
- the route also owns layout mode, selected node, hovered node, filtering,
  global search, impact preview, reconvergence, add-edge, create-node, and
  delete flows
- that breadth makes a strong command hierarchy mandatory; otherwise the route
  can easily read as a general tool shelf wrapped around a graph

## Proposed UI Model

## Route role

Blueprint is Planner's structural reasoning workspace.

Its job is to let the user inspect and manipulate structure without wondering
which surface is in control.

## Dominant surface

The dominant surface is the active blueprint canvas.

That means:

- in `graph` mode, the graph owns the route
- in `table`, `traceability`, `dependencies`, or `overview` mode, the chosen
  view owns the route with the same compositional authority the graph would get

The mode switch decides the canvas.
It should not create a sense of several simultaneous primary regions.

## Supporting surfaces

Supporting surfaces should include:

- one compact command band for mode switching, search, filters, and create
  actions
- one stable inspector or detail surface tied to selection
- modals or overlays for exceptional flows such as impact preview, create node,
  add edge, delete, and reconvergence

For the implemented first pass, the existing command band and inspector
structure remain largely intact.
The posture reset is the graph-first default, not a full chrome consolidation.

## Reveal model

- lower-priority controls should move into secondary menus or condensed control
  clusters
- the inspector should remain visible when helpful, but not visually rival the
  active canvas
- optional tools such as reconvergence should appear as explicit revealed
  workflows, not ambient clutter

## State model

The route should explicitly support:

- no selection
- node selected
- graph mode
- non-graph mode with the same inspector contract
- search or filter active
- modal workflow open
- loading and fetch failure

## Design-System-Patterns Lens

- semantic surfaces:
  one primary canvas surface, one secondary command band, one secondary
  selection inspector
- reveal discipline:
  exceptional workflows belong in overlays, not persistent chrome
- theming discipline:
  preserve Planner's restrained command-center tone and avoid neon graph
  spectacle

## Contracts And Touched Surfaces

- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  remains the route owner
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  remains the basis for attached node detail unless a blueprint-specific
  inspector abstraction is extracted
- existing blueprint APIs remain unchanged
- touched surfaces:
  [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the active blueprint canvas is visually unambiguous when the route opens
- graph mode is the default starting posture
- existing multi-view, selection-detail, and modal workflows remain intact
- the route feels more like a working canvas than an overview-first utility
  surface

## Verification Plan

- targeted frontend tests for the blueprint page and touched inspector behavior,
  including graph-first default and preserved multi-view switching
- `npx tsc --noEmit`
- manual verification for:
  - graph mode with no selection
  - graph mode with selection
  - non-graph mode with selection
  - impact preview and reconvergence flows
  - dense search and filter use

## Rollback And Fallback

- if command-band consolidation becomes too risky for one pass, preserve the
  active-canvas and inspector hierarchy first and defer smaller control cleanup
- if the inspector cannot remain persistently useful across modes, keep it as
  an attached reveal rather than letting multiple detail surfaces emerge

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the first bounded posture reset in
  [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  by making the route default to `graph` mode instead of `overview`, so the
  active blueprint canvas is the immediate primary surface when the workspace
  opens.
- Preserved the existing multi-view and inspector contracts while making the
  graph workspace the default command posture and updating route tests to align
  with the new graph-first model.
- Residual tranche-correction follow-up work then added direct automated
  evidence that the route keeps the graph-first opening posture while still
  switching cleanly across overview, traceability, dependencies, and inventory
  modes.
- Verification completed with:
  `npm test -- src/pages/__tests__/BlueprintPage.test.tsx`
  and `npx tsc --noEmit`.
