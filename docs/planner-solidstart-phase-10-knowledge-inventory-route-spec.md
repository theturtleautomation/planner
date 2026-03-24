# Planner SolidStart Phase 10 Knowledge Inventory Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner UI Reset Phase 06 Knowledge Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md), [Planner SolidStart Phase 09 Admin Operations Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-09-admin-operations-route-spec.md)

> Planning note (2026-03-24): after migrating events and admin, the next
> practical route widening move is a simplified knowledge inventory route. This
> first Solid slice should stay inventory-first and project-scoped rather than
> attempt to recreate every historical React mode at once.
>
> Implementation sync (2026-03-24): the Solid app now includes `/knowledge` as
> a project-scoped inventory workspace. Project selection, search, type
> filtering, inventory browsing, and attached selected-node detail are all live
> in the new shell. Verification completed with Solid lint/build and Playwright
> route proof.

## 1. Executive Judgment

The next SolidStart widening slice should migrate the **knowledge inventory**
into the new app shell.

The route should answer:

- what knowledge exists in the current project
- what filtered slice is visible
- what the selected node means

## 2. User Outcome

After Phase 10:

- `/knowledge` exists in SolidStart
- inventory browsing is the dominant route behavior
- scope and filters stay visible but disciplined
- selected-node detail remains attached, not a rival page

## 3. Locked Decisions

- the first Solid slice is inventory-first and project-scoped
- attached detail remains in the same workspace
- search and type filtering are local and immediate
- the route does not attempt the full historical React mode matrix yet

## 4. Acceptance Criteria

This slice is complete only when:

1. `/knowledge` exists in the Solid app
2. inventory remains the primary route object
3. selected-node detail is attached and secondary
4. browser verification proves the inventory-first hierarchy

## 5. Readiness Judgment

This spec is **implemented**.
