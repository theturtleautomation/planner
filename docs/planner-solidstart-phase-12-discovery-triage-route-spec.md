# Planner SolidStart Phase 12 Discovery Triage Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner UI Reset Phase 08 Discovery Review Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-08-discovery-review-workspace-spec.md), [Planner SolidStart Phase 11 Blueprint Graph Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-11-blueprint-graph-route-spec.md)

> Planning note (2026-03-24): after the blueprint graph route, the next
> practical widening move is discovery review. The Solid route should stay
> proposal-triage-first, with pending review work clearly outranking reviewed
> history and utility controls.
>
> Implementation sync (2026-03-24): the Solid app now includes `/discovery` as
> a proposal-triage route. Node and edge proposal modes, pending-versus-
> reviewed grouping, scan refresh, accept/reject actions, and attached proposal
> context are all live in the new shell. Verification completed with Solid
> lint/build/tests and a dedicated Playwright route proof.

## 1. Executive Judgment

The next SolidStart widening slice should migrate the **discovery triage**
workspace into the new shell.

This route should answer:

- what structural proposals still need review
- which proposal mode is active now
- what supporting context helps the next accept or reject decision

## 2. User Outcome

After Phase 12:

- `/discovery` exists in SolidStart
- pending proposals dominate the route
- node and edge proposal modes remain local and immediate
- scan and related-context actions stay available without becoming the route's
  main job

## 3. Locked Decisions

- discovery remains a proposal-triage workspace, not a utility page
- pending proposals visually outrank reviewed items
- node and edge proposal modes both remain supported
- backend discovery semantics remain unchanged in this slice
- knowledge context and scan actions stay attached and secondary

## 4. Acceptance Criteria

This slice is complete only when:

1. `/discovery` exists in the Solid app
2. pending proposals are the obvious dominant route object
3. node and edge proposal modes both work without route churn
4. scan actions and related context remain secondary
5. browser verification proves the triage-first hierarchy

## 5. Readiness Judgment

This spec is **implemented**.
