# Project-Picture Structural Concerns Plan

**Status:** Implemented  
**Date:** 2026-04-08  
**Parent:** [Project-Picture-Centered Planning Consolidation Plan](/home/thetu/planner/docs/project-picture-centered-planning-consolidation-plan.md)

## Purpose

Define the canonical conceptual model for the structural concern layer beneath the
project-picture center before choosing the first implementation-facing child
slice.

This plan does **not** choose implementation order. It defines the conceptual
order, boundaries, and follow-on placement needed to make later child planning
truthful.

## Canonical Structural Stack

### 1. Truth foundation — hidden truth-model / blueprint relationship

This layer defines what the product must remain faithful to even when the user
never sees a raw blueprint graph.

It owns questions like:
- what hidden structural truth must always remain projected into the project
  picture,
- what may remain implicit,
- and what the product must never fake or smooth over in order to stay
  trustworthy.

### 2. Recoverability contract — whole-project recoverability beyond same-route shell

This layer defines how the user regains orientation to that truth when deep in
area work.

It owns questions like:
- what whole-project context must remain visible or recallable,
- what re-entry / return affordances are necessary beyond today's same-route
  shell,
- and how the user regains the whole without falling back to dashboard-heavy
  navigation.

### 3. Interaction mechanisms — richer overlay / reorientation model

This layer defines optional support mechanisms that may help satisfy the truth
and recoverability contracts.

It owns questions like:
- which temporary surfaces, if any, help the user reorient,
- when an overlay is warranted instead of base-workspace structure,
- and how reorientation support stays humane instead of turning into a cockpit.

## Boundary Model

### Hidden truth-model owns
- the minimum visible truth contract between hidden structure and the visible
  project picture,
- trust boundaries about what must remain legible,
- and the rule that the visible picture must stay grounded in real underlying
  structure.

### Hidden truth-model does not own
- the full user recovery flow after deep rabbit-hole work,
- or the concrete overlay family used to assist reorientation.

### Whole-project recoverability owns
- whole-project re-entry and orientation,
- return-path clarity after deep work,
- and the minimum contract for regaining the whole without leaving the shaping
  flow.

### Whole-project recoverability does not own
- the underlying truth rules themselves,
- or every possible interaction surface that might support those rules.

### Overlay / reorientation owns
- supportive temporary interaction surfaces,
- contextual reorientation help,
- and the question of when extra surface area is justified.

### Overlay / reorientation does not own
- the foundational truth contract,
- or the core definition of whole-project recoverability.

## Follow-on Placement

### Provenance / change-inspection UX

This follows **after** the structural model as a specialized inspection concern.
It should build on the truth and recoverability model rather than define them.

The next provenance question is not "what is the structural stack?" but "what
kind of inspection question should the product answer first: why, where-from,
or compared-to-what?"

### Preview hierarchy refinement

This also follows **after** the structural model as a presentation-balance
concern.

It should tune the first-impression hierarchy once the structural model is
clear, rather than standing in for that deeper conceptual work.

## Outcome Of This Pass

This workstream is now modeled as:
- **truth foundation** first,
- **recoverability contract** second,
- **overlay / reorientation mechanisms** third.

That order is conceptual, not execution order.

The next unresolved planning question is which implementation-facing child slice
should be shaped first from within this model. Because that choice is still
open, the workstream remains active and still routes through further narrowing.

## Sync impact

Updated truth surfaces in this pass:
- `docs/project-picture-structural-concerns-plan.md`
- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/current-status.md`
- `.omx/ledger/project-plan.md`

Regenerated maintenance surfaces after final sync:
- `.omx/ledger/automation-trace.json`
- `.omx/ledger/automation-report.md`

Routing result after sync:
- `workstream:project-picture-structural-concerns` remains `active`
- `routing_state` remains `needs_deep_interview`

Reason:
- the conceptual model is now explicit,
- but the first implementation-facing child slice is still unresolved,
- and `npm run project:ledger:auto` refreshed maintenance metadata after the
  final canonical plan edit without changing routing truth.
