# Planner SolidStart Phase 35.8 Backendless Mock Closeout Remediation Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.7 Events, Discovery, And Admin Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-7-events-discovery-and-admin-frontend-mock-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 implementation review of the completed Phase 35 route-family slices, including `planner-solid/src/lib/mock/store.ts`, `planner-solid/e2e`, `README.md`, and `docs/builder-local-workflow.md`

## 1. Executive Judgment

Phase 35 is broadly implemented, but the review found three concrete closeout
gaps:

- discovery scan refresh does not deterministically repopulate pending work
  after review
- the shared frontend mock scenario registry is not yet reused by the route
  E2E coverage that Phase 35 said it would replace
- Builder-facing documentation still carries a split message about when bare
  `planner-solid` dev mode is acceptable

These are bounded remediation issues, not reasons to reopen the whole Phase 35
tranche.

So the correct planning move is:

- keep Phase 35 and Phase 35.1 through 35.7 closed as implemented
- add one explicit closeout remediation slice for the remaining contract and
  documentation drift

## 2. User Outcome

After this phase:

- discovery scan refresh in frontend mock mode can repopulate a believable
  pending proposal batch after prior proposals have already been reviewed
- at least one thin route-family E2E path proves the frontend mock contract
  directly instead of rebuilding route payloads through scattered
  `page.route(...)` fixtures
- Builder-facing docs state one coherent rule:
  - `planner-server` remains the canonical runtime for integration and Fusion
  - frontend-only mock mode is an explicit, allowed path for UI design and
    click-through browsing

## 3. Scope

### In Scope

- deterministic discovery scan refresh behavior in frontend mock mode
- a bounded migration of route-family browser proof onto the shared frontend
  mock contract
- documentation reconciliation for Builder-facing local workflow guidance
- honest planning closeout for the already-implemented Phase 35 slices

### Out Of Scope

- reopening the full Phase 35 implementation surface
- replacing every existing Playwright route fixture in one pass
- changing the canonical `planner-server` integration runtime
- widening frontend mock mode into backend-truth or pipeline-truth claims

## 4. Contract

### 4.1 Discovery scan refresh

Required behavior:

- `Run scan` must be able to repopulate pending discovery work even when the
  existing proposal lists are non-empty but already reviewed
- the reseeded batch must remain deterministic and scenario-coherent
- node and edge proposal counts should refresh together rather than only
  restamping the current counts

The route does not need an infinite proposal generator. It needs a small,
repeatable refresh cycle that keeps the UI browseable after accept/reject
actions.

### 4.2 Shared-scenario E2E proof

Required behavior:

- at least one route-family E2E proof should run with
  `VITE_PLANNER_FRONTEND_MOCK=1`
- that proof should browse multiple Phase 35 surfaces without relying on local
  `page.route(...)` fixture setup
- the proof should demonstrate that the runtime scenario registry, not the
  test fixture layer, now owns the default route payload family

This does not require migrating the entire E2E suite in one pass.

### 4.3 Builder documentation contract

Required documentation truth:

- `planner-server` is still the canonical runtime for Builder Fusion and
  backend-integrated work
- frontend-only mock browsing through bare `planner-solid` dev mode is an
  explicit exception for UI design and click-through review
- repo docs should no longer read as though bare `npm run dev` is categorically
  forbidden for every Builder-related use case

## 5. Product Decisions

### 5.1 Keep the remediation bounded

This phase should not reopen route coverage breadth.

It exists to close the residual gaps in:

- operational mock fidelity
- proof ownership
- documentation honesty

### 5.2 Prefer one strong proof over broad test churn

The goal is not to migrate every old E2E file immediately.

The goal is to add one truthful frontend-mock browser proof that demonstrates
the shared scenario registry is now a real app-owned test surface.

## 6. Touched Surfaces

- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [runtime.ts](/home/thetu/planner/planner-solid/src/lib/mock/runtime.ts) if
  scenario cycling needs a small contract addition
- one new or updated `planner-solid/e2e` proof that runs through frontend mock
  mode directly
- [README.md](/home/thetu/planner/README.md)
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
- Phase 35 parent and child planning artifacts for closeout honesty

## 7. Acceptance Criteria

1. discovery scan refresh can repopulate pending proposals after prior review
   actions in frontend mock mode
2. the refreshed discovery batch remains deterministic and coherent with the
   active operational scenario
3. at least one route-family E2E/browser proof uses the frontend mock runtime
   directly instead of `page.route(...)` setup
4. Builder-facing docs present a coherent split between canonical
   `planner-server` runtime and frontend-only mock browsing
5. Phase 35 planning artifacts explicitly record this remediation as the only
   residual follow-on instead of implying the tranche is both fully closed and
   fully proven already

## 8. Verification Plan

- targeted unit tests for discovery scan reseeding behavior
- one frontend-mock E2E/browser proof that covers multiple Phase 35 surfaces
  without route interception
- doc review confirming README and Builder workflow guidance are no longer in
  conflict
- standard frontend verification:
  - `npm --prefix planner-solid run test`
  - `npm --prefix planner-solid run lint`
  - `npm --prefix planner-solid run build`

## 9. Rollback / Fallback

If the E2E migration breadth is too large in one pass:

- keep the remediation to one new frontend-mock route-family proof
- do not block the phase on broad legacy test cleanup
- still fix discovery refresh and documentation truth in the same slice

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice closed the remaining bounded Phase 35 gaps without reopening the
broader route-family work:

- discovery scan refresh now reseeds deterministic pending node and edge
  proposals after prior review has exhausted the current pending queue
- the repo now carries one direct frontend-mock Playwright proof through
  [playwright.frontend-mock.config.ts](/home/thetu/planner/planner-solid/playwright.frontend-mock.config.ts)
  and
  [phase-35-frontend-mock.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-35-frontend-mock.spec.ts)
  instead of route interception
- Builder-facing docs now state one coherent split between canonical
  `planner-server` integration runtime and frontend-only mock browsing

Primary implementation surfaces:

- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [store.test.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.test.ts)
- [playwright.frontend-mock.config.ts](/home/thetu/planner/planner-solid/playwright.frontend-mock.config.ts)
- [phase-35-frontend-mock.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-35-frontend-mock.spec.ts)
- [package.json](/home/thetu/planner/planner-solid/package.json)
- [README.md](/home/thetu/planner/README.md)
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)

Verification evidence:

- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Residual verification note:

- the pre-existing Nitro `"send"` warning during build remains, but the build
  exits successfully and this slice did not widen that issue
