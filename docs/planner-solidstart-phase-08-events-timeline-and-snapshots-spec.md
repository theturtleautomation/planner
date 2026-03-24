# Planner SolidStart Phase 08 Events Timeline And Snapshots Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md), [Planner SolidStart Phase 07 Project Outputs And Artifacts Spec](/home/thetu/planner/docs/planner-solidstart-phase-07-project-outputs-and-artifacts-spec.md)

> Planning note (2026-03-24): after the project workspace absorbed outputs and
> artifacts, the next bounded route migration should be the top-level events
> timeline. The Solid route should keep chronology primary and snapshots
> secondary.
>
> Implementation sync (2026-03-24): the Solid app now includes `/events` as a
> dedicated chronological workspace backed by `GET /blueprint/events` and
> `GET/POST /blueprint/history`. The stream stays primary, snapshots stay
> secondary, and the route preserves dense filtering without dashboard clutter.
> Verification completed with Solid lint/build and Playwright route proof.

## 1. Executive Judgment

The next SolidStart widening slice should migrate the **events timeline** into
the new app shell.

The route must answer one question first:

- what changed, and what should be inspected next

Snapshots remain important, but they must remain attached to the route rather
than compete with the chronological stream.

## 2. User Outcome

After Phase 08:

- `/events` exists in SolidStart
- the main event stream remains the clear primary surface
- snapshots remain available nearby without stealing first attention
- refresh, filtering, and snapshot creation stay local and direct

## 3. Locked Decisions

- the event stream is the route truth
- snapshots remain secondary
- route density should come from rhythm and chronology, not widget sprawl
- backend event and snapshot semantics remain unchanged

## 4. Acceptance Criteria

This slice is complete only when:

1. `/events` exists in the Solid app
2. the event stream is the clear dominant surface
3. snapshots remain available without becoming a peer hero module
4. route-level browser verification proves the hierarchy and interaction model

## 5. Readiness Judgment

This spec is **implemented**.
