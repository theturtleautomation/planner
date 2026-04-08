# Project-Picture Experience Consolidation Plan

**Status:** Implemented  
**Date:** 2026-04-08  
**Parent:** [Project-Picture-Centered Planning Consolidation Plan](/home/thetu/planner/docs/project-picture-centered-planning-consolidation-plan.md)

## Purpose

Define the canonical conceptual model for the **experience layer** beneath the
project-picture center before choosing the first implementation-facing child
slice.

This plan does **not** choose implementation order. It defines the conceptual
order, ownership boundaries, and sibling-layer relationship needed to make
later child planning truthful.

## Canonical Experience Model

### 1. Shared experience grammar — design system

This family defines the reusable visual and interaction language that the rest
of the experience layer should speak.

It owns questions like:
- what semantic surface hierarchy the product should reuse across routes,
- what visual language should feel consistent from page to page,
- and what shared interaction grammar should remain stable as the product
  evolves.

### 2. Route posture and hierarchy — UI reset

This family defines how route families use that grammar to establish dominant
surfaces, reveal models, and product-first hierarchy.

It owns questions like:
- what each route should make primary,
- what supporting context should remain visible or move into reveal surfaces,
- and how routes should feel calmer and clearer without hiding real product
  state.

### 3. Domain workspace expression — knowledge library

This family defines how one specialized product domain expresses that grammar
and posture as a scoped knowledge workspace.

It owns questions like:
- how knowledge should feel project-scoped rather than globally diffuse,
- how inventory and rationale should be expressed within the broader route
  posture model,
- and how a specialized workspace can stay aligned with the product language
  without becoming the template for every other route family.

## Boundary Model

### Design system owns
- the shared experience grammar,
- reusable semantic-surface rules,
- and the common visual/interaction language that other experience families
  inherit.

### Design system does not own
- the primary job or reveal model of every individual route,
- or the domain-specific behavior of the knowledge workspace.

### UI reset owns
- route-level posture,
- dominant-surface and supporting-context hierarchy,
- and reveal-model discipline for route families.

### UI reset does not own
- the foundational shared grammar itself,
- or the specialized product thesis of the knowledge library domain.

### Knowledge library owns
- the specialized scoped-workspace model for knowledge,
- project-first knowledge navigation and context,
- and the way this domain expresses inventory, rationale, and scope within the
  broader experience language.

### Knowledge library does not own
- the shared cross-product grammar for every route,
- or the canonical route posture model for unrelated product families.

## Structural Relationship

Structural concerns remain a **separate sibling child layer** beneath the same
project-picture center.

That means this plan does **not** absorb:
- hidden truth-model / blueprint relationship,
- whole-project recoverability beyond same-route shell,
- or richer overlay / reorientation modeling.

Those concerns matter, but they define a deeper structural layer rather than
the experience-layer stack itself.

## Child Plans In This Layer

The current experience-layer child plans are:
- [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)
- [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
- [Knowledge Library Project Scope Plan](/home/thetu/planner/docs/knowledge-library-project-scope-plan.md)

This pass clarifies how those plans relate. It does **not** choose which of
those child families should become the first implementation-facing slice.

## Outcome Of This Pass

This experience layer is now modeled as:
- **shared experience grammar** first,
- **route posture and hierarchy** second,
- **domain workspace expression** third.

That order is conceptual, not execution order.

The next unresolved planning question is which child family should be tightened
next from within this model. Because that choice is still open, the experience
layer remains the current planning surface rather than a completed execution
queue.

## Sync impact

Updated truth surfaces in this pass:
- `docs/project-picture-experience-consolidation-plan.md`

Unchanged routing / summary truth surfaces:
- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/project-plan.md`

Regenerated maintenance surfaces after final sync:
- `.omx/ledger/current-status.md`
- `.omx/ledger/automation-trace.json`
- `.omx/ledger/automation-report.md`

Routing result after sync:
- `plan:project-picture-experience-consolidation` remains `draft`
- `routing_state` remains `ready_for_ralplan`

Reason:
- the experience-layer conceptual model is now explicit,
- but the first implementation-facing child slice is still unresolved,
- and `npm run project:ledger:auto` only needs to refresh maintenance metadata
  after the canonical plan edit unless later routing truth changes.
