# Planner Design System Command Center Plan

**Status:** Implemented  
**Date:** 2026-03-21  
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

## Current Product Anchors

The existing visual system still carries the older dark-first dashboard shell:

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

This redesign should ship in three bounded phases.

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

## Working Constraints

- redesign slices must stay frontend-only unless a product contract forces
  backend touch
- graph-heavy views should keep performance as a first-class constraint
- each phase must preserve light and dark theme parity
- each phase must name concrete files and concrete verification paths
- do not broaden a visual slice into route, IA, or product-behavior changes

## Next Move

The current three-phase command-center refresh is implemented. The next move,
if design work resumes, should be either:

- a manual visual confidence sweep across light and dark overlay states, or
- a new bounded spec for a later context-menu or popover primitive
