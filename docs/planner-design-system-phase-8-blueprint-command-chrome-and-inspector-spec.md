# Planner Design System Phase 8 Blueprint Command Chrome And Inspector Spec

**Status:** Implemented and verified on 2026-03-22  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 7 Knowledge Inventory And Context Spec](/home/thetu/planner/docs/planner-design-system-phase-7-knowledge-inventory-and-context-spec.md)  
**Source Research:** Stitch-to-Planner design translation report dated 2026-03-22

## Objective

Finish the Stitch-translation queue with a bounded Blueprint-specific visual
pass that improves command chrome, control hierarchy, and inspector clarity
while keeping the actual graph rendering practical and performance-safe.

## User Outcome

After this slice:

- Blueprint feels more integrated with the rest of the command-center system
- top controls, search, and view-mode tools are easier to scan and use
- the detail inspector reads as the current context authority without relying
  on graph theatrics

## In Scope

- route-level chrome, control-group hierarchy, and non-graph framing in
  [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
- inspector or detail-chrome refinement in
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- token support in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- directly adjacent blueprint support panels only if needed to maintain
  coherence, such as
  [ReconvergencePanel.tsx](/home/thetu/planner/planner-web/src/components/ReconvergencePanel.tsx)

## Out Of Scope

- graph node restyling, edge rendering changes, layout algorithm changes, or
  broad canvas animation work
- decorative dotted backdrops, neon line work, or source-like cyber effects
- heavy blur or glass on the base Blueprint page
- backend Blueprint behavior changes

## Current-State Summary

Earlier phases intentionally kept Blueprint graph density out of scope except
for route chrome cleanup and overlay normalization. The remaining gap is the
Blueprint-specific command experience:

- top-of-page controls, view modes, and search are truthful but still more
  utilitarian than the desired command-center direction
- the selected-node context is useful, but the route can do more to clarify
  what is primary canvas, what is command chrome, and what is active inspector
  context
- the Stitch archive demonstrates a useful shell discipline for graph-adjacent
  routes, but much of its graph styling is too theatrical for Planner

## Proposed Behavior

### Blueprint command chrome

- make search, view mode, and layout controls read as one coherent command
  cluster
- prioritize the current working mode and selection state more clearly
- reduce any remaining sense of miscellaneous toolbar clutter

### Canvas framing

- the graph canvas should feel like the primary workspace, but not through
  decorative effects
- tonal framing and spacing should separate command chrome from the canvas
  without hard dashboard borders

### Inspector and detail context

- the inspector should read as the active local authority for the current
  selection
- selected-node metadata, related actions, and navigation between related nodes
  should be easier to scan
- keep the inspector calmer than the Stitch archive's graph panels

## Implementation Constraints

- performance remains first-class for Blueprint
- no graph-cosplay visuals, no fake infrastructure drama, and no glowing node
  theater
- preserve the overlay-only glass rule from Phase 3
- keep the route aligned with the existing Planner palette and typographic
  system
- if a visual change risks graph clarity or D3 interaction stability, prefer
  command-chrome work and defer the risky canvas change

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)

Expected supporting files, only if needed:

- [ReconvergencePanel.tsx](/home/thetu/planner/planner-web/src/components/ReconvergencePanel.tsx)
- other blueprint-local control components referenced directly by the page

## Acceptance Criteria

- Blueprint top chrome reads as a coherent command surface
- selected-node context and inspector hierarchy are clearer than before
- the page feels more aligned with the rest of the command-center system
  without changing the graph into a theatrical showcase
- graph interaction and performance are not materially harmed

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - [BlueprintPage](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  - [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  - any directly touched Blueprint support panel tests
- run `npx tsc --noEmit`

### Manual

- verify Blueprint in both themes with no selection, with a selected node, and
  with related inspector interactions
- verify search, layout-mode, and view-mode controls remain clear and stable
- verify D3 graph interaction remains responsive
- verify the route feels calmer than the Stitch source and does not drift into
  graph spectacle

## Rollback And Fallback

- if one chrome change harms graph usability, keep the shared token and
  inspector improvements and revert the risky local control treatment
- if inspector density becomes too heavy, reduce metadata surface count before
  reintroducing visible lines
- if performance regresses, keep the non-canvas command work and defer any
  canvas-adjacent styling

## Open Questions

None blocking readiness.

The route and performance boundaries are concrete enough to support bounded
implementation.
