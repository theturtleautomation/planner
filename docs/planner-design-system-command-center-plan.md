# Planner Design System Command Center Plan

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)

## Purpose

Turn the supplied design-system analysis into a phased visual refresh plan for
the Planner React SPA.

This plan defines the transferable visual language, the rollout boundaries, and
the bounded implementation slices that should carry the redesign without
turning it into an unscoped full-app restyle.

## Design Direction

Planner should move toward a "Surgical Command Center" visual system:

- no-line layout sectioning that relies on tonal separation instead of constant
  structural borders
- a four-tier surface stack that defines recessed canvases, base layouts,
  elevated cards, and floating layers
- editorial hierarchy for headings and section anchors, while preserving dense,
  legible data presentation
- restrained depth through soft ambient shadows rather than harsh black drop
  shadows
- crisp, saturated accents reserved for primary actions and active states
- macro-loose, micro-dense spacing that keeps modules calm without making the
  product feel sparse

## Transferable Principles

These principles are in scope for Planner and should guide later slices:

- tonal sectioning over border-heavy chrome
- display-versus-data typography hierarchy
- ambient elevation for cards, drawers, and floating context
- restrained glass only for transient overlays
- strong whitespace between page zones with compact data density inside zones

## Explicit Non-Goals

These findings are intentionally not treated as default requirements:

- heavy glassmorphism across graph-heavy or DOM-heavy base surfaces
- direct reuse of neon tertiary hues from the analyzed archive
- asymmetry that reduces spatial predictability on productivity screens

## Original Product Anchors

This section captures the pre-refresh baseline that motivated the phased
command-center rollout. It is historical context, not the current rendered
state after the implemented four-phase refresh.

At planning time, the existing visual system still carried the older
dark-first dashboard shell:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
  still defines the shared token system, border-heavy shell, and current shadow
  treatment
- [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
  still uses a right-divided sidebar and compact utility chrome
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  still leans on bordered cards and default CTA styling
- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx),
  and [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  still use many explicit container borders and legacy card conventions
- graph-heavy and admin surfaces exist, but they should not be restyled in the
  same slice as the global shell foundation

## Rollout Model

The initial redesign shipped in four bounded phases.

### Phase 1: Tonal Foundation And Border Removal

Goal:

- establish the four-tier surface language and no-line shell treatment in the
  highest-value shared surfaces

Primary surfaces:

- global token layer in [index.css](/home/thetu/planner/planner-web/src/index.css)
- shell in [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- home and project/session entry surfaces

Output:

- [Planner Design System Phase 1 Tonal Foundation Spec](/home/thetu/planner/docs/planner-design-system-phase-1-tonal-foundation-spec.md)

Status:

- implemented and verified on 2026-03-21

### Phase 2: Editorial Typography And CTA Hierarchy

Goal:

- introduce a dual-font heading strategy, stronger page-level hierarchy, and
  premium CTA emphasis after the shell foundation is stable

Primary surfaces:

- top-level page headers
- empty states
- primary and secondary action components

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md)

### Phase 3: Floating Depth And Glass Restraint

Goal:

- replace legacy shadows with ambient elevation and add restrained blur/glass to
  transient overlays only after performance-sensitive surfaces are profiled

Primary surfaces:

- modals
- context menus
- small floating drawers and popovers

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 3 Overlay Depth And Restrained Glass Spec](/home/thetu/planner/docs/planner-design-system-phase-3-overlay-depth-and-restrained-glass-spec.md)

### Phase 4: Utility Route Consistency

Goal:

- migrate the remaining legacy utility routes and shared operational chrome into
  the command-center system without reopening graph-density or backend work

Primary surfaces:

- sessions queue route
- admin route
- shared event log chrome
- non-graph blueprint header and sidebar chrome

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md)

## Follow-On Spec Queue

The Stitch-to-Planner translation work is intentionally queued as follow-on
specs instead of reopening the initial four-phase rollout as one unbounded
redesign.

### Phase 5: Route Hierarchy And Operational Density

Goal:

- strengthen dominant-module hierarchy and operational row density across Home,
  Projects, `/sessions`, and project-local session management

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)

### Phase 6: Operational Surfaces And Event Density

Goal:

- refine Admin, Events, Discovery, import review/history, and shared event
  surfaces into denser command-center review tools

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)

### Phase 7: Knowledge Inventory And Context

Goal:

- turn Knowledge into a clearer inventory-and-context workspace without copying
  the Stitch source's featured-content framing

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 7 Knowledge Inventory And Context Spec](/home/thetu/planner/docs/planner-design-system-phase-7-knowledge-inventory-and-context-spec.md)

### Phase 8: Blueprint Command Chrome And Inspector

Goal:

- improve Blueprint command chrome and inspector clarity while preserving graph
  performance and avoiding source-like graph theatrics

Status:

- implemented and verified on 2026-03-22 in
  [Planner Design System Phase 8 Blueprint Command Chrome And Inspector Spec](/home/thetu/planner/docs/planner-design-system-phase-8-blueprint-command-chrome-and-inspector-spec.md)

## Working Constraints

- redesign slices must stay frontend-only unless a product contract forces
  backend touch
- graph-heavy views should keep performance as a first-class constraint
- each phase must preserve light and dark theme parity
- each phase must name concrete files and concrete verification paths
- do not broaden a visual slice into route, IA, or product-behavior changes

## Next Move

The initial four-phase command-center refresh and the Stitch-translation
follow-on queue are now implemented and verified through Phase 8.

Any further visual work should be treated as a new bounded slice instead of
reopening the completed design-system queue.

Later context-menu or popover work should remain a separate future slice rather
than being bundled into the completed Blueprint command-chrome work.
