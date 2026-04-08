# Planner SolidStart Phase 35.5 Import Review Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md), [Planner SolidStart Phase 35.4 Project Workspace Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-4-project-workspace-frontend-mock-spec.md)  
**Related Planning:** [Planner SolidStart Phase 14 Project Import Review Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-14-project-import-review-route-spec.md), [Planner SolidStart Phase 15 Project Import History And Restore Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-15-project-import-history-and-restore-route-spec.md), [Planner SolidStart Phase 16 Project Import Comparison And Selection Summary Spec](/home/thetu/planner/docs/planner-solidstart-phase-16-project-import-comparison-and-selection-summary-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/projects/[projectSlug]/import.tsx`

## 1. Executive Judgment

The import review route has its own distinct payload family and workflow
semantics. It should be mocked as its own bounded slice rather than hidden
inside the broader project workspace spec.

Builder needs to browse:

- pending review state
- history state
- comparison state
- restore/apply affordances

That is enough UI-design value without any need for real import jobs.

## 2. User Outcome

After this phase:

- Builder can open the import review route in frontend mock mode
- the route can show pending review, applied state, and history scenarios
- comparison and restore actions can update the in-memory route state

## 3. Scope

### In Scope

- frontend mock support for `/projects/:projectSlug/import`
- mock project detail, import review, import state, and history payloads
- local mutation for include/exclude, apply, compare, and restore affordances

### Out Of Scope

- real import acquisition or analysis
- project workspace summary logic outside what is needed for route continuity
- backend-truth claims for import lifecycle actions

## 4. Contract

### 4.1 Required scenarios

This slice should support at minimum:

- `import-review`
  - pending review with selectable nodes
- `import-applied`
  - stable applied import state and history
- `import-empty`
  - no current import posture attached to the project

### 4.2 Local mutation behavior

Required behavior:

- include/exclude toggles mutate the current in-memory review selection
- apply transitions the route into a coherent applied state
- compare actions reveal deterministic comparison payloads
- restore/reopen actions update the in-memory current state and notices

The route only needs enough mutation fidelity for browsing and design review.

## 5. Product Decisions

### 5.1 Preserve project-local framing

Mock mode must keep import review attached to the project context:

- back-to-project navigation remains present
- history and compare are attached import tools, not separate workflows

### 5.2 Prefer a few coherent history entries over broad fake history

This slice should use a small number of believable history entries with
consistent comparison outcomes instead of a large synthetic audit archive.

## 6. Touched Surfaces

- [project import route](/home/thetu/planner/planner-solid/src/routes/projects/%5BprojectSlug%5D/import.tsx)
- import-history helpers under `planner-solid/src/lib/` as needed
- shared mock scenario modules

## 7. Acceptance Criteria

1. `/projects/:projectSlug/import` renders in frontend mock mode without a
   backend
2. pending review, applied, and empty states are all browsable
3. include/exclude and apply actions mutate local mock state coherently
4. compare and restore affordances are visually and behaviorally browseable
5. route copy and navigation remain project-local

## 8. Verification Plan

- targeted browser proof in frontend mock mode for:
  - pending review
  - history comparison
  - apply flow
  - restore/reopen flow
- targeted tests for import-review mock state transitions

## 9. Rollback / Fallback

If local mutation breadth is too large in one pass:

- ship browse-only pending/applied/empty scenarios first
- then add compare and restore transitions
- then add include/exclude and apply mutation

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice made the import review desk fully browseable in frontend mock mode:

- the scenario registry now includes pending-review, applied, and empty import
  route states
- include/exclude, apply, compare, restore, and reopen actions resolve against
  one in-memory import history graph rather than static one-off payloads
- import review state stays attached to the owning project and seeded session
  links remain navigable under the active mock scenario

Primary implementation surfaces:

- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [import.tsx](/home/thetu/planner/planner-solid/src/routes/projects/%5BprojectSlug%5D/import.tsx)
- [import-history.ts](/home/thetu/planner/planner-solid/src/lib/import-history.ts)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/projects/personal-calendar/import?mockScenario=import-review`
  - `/projects/personal-calendar/import?mockScenario=import-applied`
  - `/projects/personal-calendar/import?mockScenario=import-empty`
  - include/exclude plus `Apply import review`
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
