# Planner SolidStart Phase 09 Admin Operations Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner UI Reset Phase 10 Admin Operations Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md), [Planner SolidStart Phase 08 Events Timeline And Snapshots Spec](/home/thetu/planner/docs/planner-solidstart-phase-08-events-timeline-and-snapshots-spec.md)

> Planning note (2026-03-24): after `/events`, the next bounded operational
> route migration should be `/admin`, keeping one dominant health desk with a
> quieter event stream below it.
>
> Implementation sync (2026-03-24): the Solid app now includes `/admin` backed
> by `GET /admin/status` and `GET /admin/events`. The route anchors on runtime
> posture, provider availability, and session/event health first, then exposes a
> compact operator-visible event stream with lightweight filtering. Verification
> completed with Solid lint/build and Playwright route proof.

## 1. Executive Judgment

The next SolidStart widening slice should migrate the **admin operations route**
into the new shell.

This route should remain serious and dense, but it must clearly answer:

- is the system healthy enough
- what requires operator attention
- what changed recently

## 2. User Outcome

After Phase 09:

- `/admin` exists in SolidStart
- runtime posture is obvious before low-level event detail
- provider availability and session counts are easy to scan
- the event stream supports the top posture instead of competing with it

## 3. Locked Decisions

- one dominant health desk anchors the route
- events remain secondary but dense
- no decorative operations drama or fake infrastructure dashboard styling
- backend admin semantics remain unchanged

## 4. Acceptance Criteria

This slice is complete only when:

1. `/admin` exists in the Solid app
2. runtime posture is the obvious first reading target
3. the event stream remains secondary and filterable
4. browser verification proves the intended hierarchy

## 5. Readiness Judgment

This spec is **implemented**.
