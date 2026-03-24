# Planner SolidStart Phase 11 Blueprint Graph Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner UI Reset Phase 07 Blueprint Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md), [Planner SolidStart Phase 10 Knowledge Inventory Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-10-knowledge-inventory-route-spec.md)

> Planning note (2026-03-24): after the simplified knowledge inventory route,
> the next bounded SolidStart widening slice should migrate the blueprint graph
> workspace. The route should stay graph-first and inspection-oriented without
> collapsing into a generic admin or database view.
>
> Implementation sync (2026-03-24): the Solid app now includes `/blueprint` as
> a graph-first structural workspace. Project selection, graph filtering, the
> primary SVG graph canvas, attached node inspection, and browser proof are all
> live in the new shell. Verification completed with Solid tests, lint/build,
> and a dedicated Playwright route proof.

## 1. Executive Judgment

The next SolidStart widening slice should add a dedicated **blueprint graph**
route to the new shell.

This route should answer:

- what the current graph shape looks like
- which nodes and edges dominate the current project state
- what graph-level structure deserves inspection next

## 2. User Outcome

After Phase 11:

- `/blueprint` exists in SolidStart
- graph structure is the obvious primary surface
- summary counts and structural highlights remain supporting context
- the route stays legible and tool-like rather than turning into graph theater

## 3. Locked Decisions

- the graph or graph-summary surface is the route anchor
- summary and filter context remain supportive
- this slice does not redesign blueprint backend semantics
- the route should stay dense, calm, and project-useful

## 4. Acceptance Criteria

This slice is complete only when:

1. `/blueprint` exists in the Solid app
2. graph structure is the primary reading target
3. summary context supports rather than competes with the graph
4. browser verification proves the intended graph-first hierarchy

## 5. Readiness Judgment

This spec is **implemented**.
