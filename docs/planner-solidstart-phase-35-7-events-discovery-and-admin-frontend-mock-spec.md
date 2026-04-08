# Planner SolidStart Phase 35.7 Events, Discovery, And Admin Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md)  
**Related Planning:** [Planner SolidStart Phase 08 Events Timeline And Snapshots Spec](/home/thetu/planner/docs/planner-solidstart-phase-08-events-timeline-and-snapshots-spec.md), [Planner SolidStart Phase 09 Admin Operations Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-09-admin-operations-route-spec.md), [Planner SolidStart Phase 12 Discovery Triage Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-12-discovery-triage-route-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/events/index.tsx`, `planner-solid/src/routes/discovery/index.tsx`, and `planner-solid/src/routes/admin/index.tsx`

## 1. Executive Judgment

These routes are operational surfaces with related mock needs:

- `/events`
- `/discovery`
- `/admin`

They can share the same deterministic operational scenarios:

- quiet system
- attention-needed system

That makes them a good final route-family slice under Phase 35.

## 2. User Outcome

After this phase:

- Builder can browse timeline, discovery, and admin routes in frontend mock
  mode
- quiet and degraded operational states are both available for design review
- local scan/snapshot/review actions can update the in-memory mock state

## 3. Scope

### In Scope

- frontend mock support for `/events`, `/discovery`, and `/admin`
- mock blueprint events and snapshot history
- mock discovery proposals and scan results
- mock admin status and admin events
- local mutation for snapshot creation, discovery triage, and scan refresh

### Out Of Scope

- backend operational truth
- route families outside these three surfaces
- deep blueprint/project/session behavior except where referenced in event copy

## 4. Contract

### 4.1 Required scenarios

This slice should support at minimum:

- `ops-quiet`
  - healthy posture, low event volume
- `ops-attention`
  - warnings/errors, pending discovery proposals, populated events

### 4.2 Local mutation behavior

Required behavior:

- `/events`
  - snapshot creation appends to in-memory snapshot history
- `/discovery`
  - accept/reject updates local proposal status
  - run scan refreshes proposal/event counts deterministically
- `/admin`
  - filter changes work over local admin event data

## 5. Product Decisions

### 5.1 Keep chronology and triage primary

Mock mode must preserve each route’s current product role:

- events stays chronology-first
- discovery stays pending-triage-first
- admin stays health-desk-first

### 5.2 Use scenario coherence across routes

If `ops-attention` is active:

- admin should show degraded posture
- discovery should show pending/reviewable work
- events should show a corresponding denser stream

These routes should feel like one coherent system state.

## 6. Touched Surfaces

- [events route](/home/thetu/planner/planner-solid/src/routes/events/index.tsx)
- [discovery route](/home/thetu/planner/planner-solid/src/routes/discovery/index.tsx)
- [admin route](/home/thetu/planner/planner-solid/src/routes/admin/index.tsx)
- shared operational scenario modules

## 7. Acceptance Criteria

1. `/events`, `/discovery`, and `/admin` all render in frontend mock mode
   without a backend
2. quiet and attention-needed scenarios are both browseable
3. snapshot creation, proposal triage, and scan refresh are locally usable
4. admin filters work over mock operational data
5. the three routes present one coherent operational state per scenario

## 8. Verification Plan

- targeted browser proof in frontend mock mode for:
  - events route timeline plus snapshot creation
  - discovery triage actions
  - admin posture plus event filtering
- targeted tests for operational scenario mutation behavior

## 9. Rollback / Fallback

If full local mutation breadth is too large in one pass:

- ship browse-only quiet/attention scenarios first
- add discovery triage mutation next
- then add snapshot creation and scan refresh

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice completed the operational route family under frontend mock mode:

- the scenario registry now includes quiet and attention-needed operational
  states shared across events, discovery, and admin
- snapshot creation appends to in-memory blueprint history
- discovery triage and scan refresh mutate one shared proposal state family
- admin status and event filtering now browse against deterministic mock
  operational data

Primary implementation surfaces:

- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/events/index.tsx)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/discovery/index.tsx)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/admin/index.tsx)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/events?mockScenario=ops-attention` plus `Create snapshot`
  - `/discovery?mockScenario=ops-attention` plus proposal acceptance
  - `/admin?mockScenario=ops-attention` rendered with degraded posture and
    local filtering controls
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Residual closeout note:

- implementation review after delivery found one bounded follow-on gap in this
  slice:
  - discovery scan refresh should reseed pending proposals after prior review,
    not only recount the existing in-memory lists
- that remediation is now captured in
  [Planner SolidStart Phase 35.8 Backendless Mock Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-8-backendless-mock-closeout-remediation-spec.md)
- that follow-on is now implemented, so this slice remains closed as
  implemented with its residual discovery reseeding gap remediated
