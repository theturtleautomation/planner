# Planner SolidStart Phase 35.2 Work-Entry And Queue Routes Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md)  
**Related Planning:** [Planner SolidStart Phase 32 Work Entry IA And Session Route Topology Spec](/home/thetu/planner/docs/planner-solidstart-phase-32-work-entry-ia-and-session-route-topology-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/index.tsx`, `planner-solid/src/routes/projects/index.tsx`, `planner-solid/src/routes/projects/new.tsx`, `planner-solid/src/routes/sessions/index.tsx`, and `planner-solid/src/routes/sessions/new.tsx`

## 1. Executive Judgment

This is the first high-value route slice after the foundation.

These routes define the click-through entry experience Builder needs first:

- `/`
- `/projects`
- `/projects/new`
- `/sessions`
- `/sessions/new`

If these routes are browsable with coherent mock navigation, the rest of the
mock route family becomes meaningfully usable.

## 2. User Outcome

After this phase:

- Builder can open the app in frontend mock mode and click through the main
  entry and queue surfaces
- project-first and direct-session entry both remain visible and navigable
- mock create flows can move the user into the next page without a backend

## 3. Scope

### In Scope

- mock browsing support for the five routes above
- project/session list scenarios
- create-project and create-session in-memory mutations
- navigation continuity from create flows into downstream route placeholders

### Out Of Scope

- deep session workspace behavior
- project workspace advanced summaries
- import/knowledge/blueprint/events/discovery/admin surfaces

## 4. Contract

### 4.1 Route scenarios

This slice should at minimum support:

- `default`
  - active project, active session, standard queue state
- `empty`
  - no projects, no sessions

### 4.2 Create behavior

Mock create actions should be truthfully navigable:

- `/projects/new`
  - creating a project writes to the in-memory mock store
  - navigation continues to `/projects/:projectSlug`
- `/sessions/new`
  - creating a direct session writes to the in-memory mock store
  - navigation continues to `/sessions/:sessionId`

The downstream pages may initially rely on later route slices, but the IDs and
slugs should be coherent.

## 5. Product Decisions

### 5.1 Preserve route hierarchy

This slice must preserve the already-chosen product hierarchy:

- project-first remains primary
- direct session remains secondary

Mock mode must not invent a different topology or CTA role.

### 5.2 Mock create flows should be simple, not theatrical

Required behavior:

- no fake progress spinners beyond the route’s existing submit affordances
- no invented backend jargon
- just enough local mutation for click-through continuity

## 6. Touched Surfaces

- [index.tsx](/home/thetu/planner/planner-solid/src/routes/index.tsx)
- [projects/index.tsx](/home/thetu/planner/planner-solid/src/routes/projects/index.tsx)
- [projects/new.tsx](/home/thetu/planner/planner-solid/src/routes/projects/new.tsx)
- [sessions/index.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/index.tsx)
- [sessions/new.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/new.tsx)
- shared frontend mock scenario modules from Phase 35.1

## 7. Acceptance Criteria

1. the five work-entry and queue routes render in frontend mock mode without a
   backend
2. `default` and `empty` scenarios both browse cleanly
3. creating a project navigates to a coherent mock project route
4. creating a direct session navigates to a coherent mock session route
5. project-first versus direct-session CTA hierarchy remains the same as the
   live product contract

## 8. Verification Plan

- targeted browser proof in frontend mock mode for:
  - `/`
  - `/projects`
  - `/projects/new`
  - `/sessions`
  - `/sessions/new`
- targeted tests proving mock create flows mutate the in-memory store

## 9. Rollback / Fallback

If create-flow mutation is too broad in one pass:

- make the list routes browsable first
- land create-route rendering next
- keep navigation targets coherent, but defer mutation to a follow-on within
  this slice rather than blocking the entire route family

## 10. Open Questions

- whether mock create actions should assign deterministic IDs/slugs from a
  seeded counter or from stable fixture builders

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice landed the first route-family browsing surface on top of the Phase
35.1 mock foundation:

- the five work-entry and queue routes now preserve the active frontend mock
  scenario during normal link navigation
- create-project and create-session flows now preserve scenario continuity when
  navigating into downstream project/session routes
- direct-session creation now invalidates the cached session list so queue
  views stay truthful after local mutation
- the mock provider now supports project deletion, allowing the projects queue
  to remain coherent under frontend-only browsing

Primary implementation surfaces:

- [app.tsx](/home/thetu/planner/planner-solid/src/app.tsx)
- [runtime.ts](/home/thetu/planner/planner-solid/src/lib/mock/runtime.ts)
- [api.ts](/home/thetu/planner/planner-solid/src/lib/api.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/index.tsx)
- [projects/index.tsx](/home/thetu/planner/planner-solid/src/routes/projects/index.tsx)
- [projects/new.tsx](/home/thetu/planner/planner-solid/src/routes/projects/new.tsx)
- [sessions/index.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/index.tsx)
- [sessions/new.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/new.tsx)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/`, `/projects`, `/projects/new`, `/sessions`, and `/sessions/new`
  - both `default` and `empty` scenarios
  - project creation from `empty` into `/projects/mock-travel-planner?mockScenario=empty`
  - direct session creation from `empty` into `/sessions/session-1?mockScenario=empty`
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Residual verification note:

- the build still emits the pre-existing Nitro warning about `"send"` from
  `h3/dist/_entries/node.mjs`, but the command exits successfully and this
  slice did not introduce a new build failure.
