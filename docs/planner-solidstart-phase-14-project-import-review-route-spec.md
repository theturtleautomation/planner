# Planner SolidStart Phase 14 Project Import Review Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md), [Planner SolidStart Phase 13 Route Family Closeout Spec](/home/thetu/planner/docs/planner-solidstart-phase-13-route-family-closeout-spec.md)

> Planning note (2026-03-24): once the primary Solid route family reads as the
> active frontend, the next practical widening move is project-local import
> review. The current project workspace already surfaces import state
> summaries, but the actual include/exclude/apply workflow deserves a dedicated
> decision desk.
>
> Implementation sync (2026-03-24): the Solid app now includes
> `/projects/:projectSlug/import` as a project-local import decision route.
> The route keeps pending import review primary, supports include/exclude and
> apply actions in place, and remains attached to the surrounding project
> workspace through direct links and seeded-session jump paths.

## 1. Executive Judgment

The next SolidStart widening slice should add a **project-local import review**
route.

This route should answer:

- whether a project import draft is waiting for decisions
- which imported nodes are included or excluded
- what should happen next before the imported structure is applied

## 2. User Outcome

After Phase 14:

- `/projects/:projectSlug/import` exists in SolidStart
- import review decisions are local to the project workspace
- include/exclude actions are immediate and explicit
- apply, seeded-session jump, and current import posture remain attached

## 3. Locked Decisions

- this is a project-local route, not a top-level utility page
- pending review state outranks historical applied state
- backend import semantics remain unchanged in this slice
- the route should stay dense and decision-oriented rather than reading like a
  log or a generic import dashboard

## 4. Acceptance Criteria

This slice is complete only when:

1. `/projects/:projectSlug/import` exists in the Solid app
2. pending import review dominates when present
3. include/exclude and apply actions are available in-route
4. browser verification proves the route keeps import decisions primary

## 5. Readiness Judgment

This spec is **implemented**.
