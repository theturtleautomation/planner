# Planner SolidStart Phase 35.6 Knowledge And Blueprint Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md)  
**Related Planning:** [Planner SolidStart Phase 10 Knowledge Inventory Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-10-knowledge-inventory-route-spec.md), [Planner SolidStart Phase 11 Blueprint Graph Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-11-blueprint-graph-route-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/knowledge/index.tsx` and `planner-solid/src/routes/blueprint/index.tsx`

## 1. Executive Judgment

These two routes are a natural pair because both read from the same underlying
blueprint/project graph contract:

- `/knowledge`
- `/blueprint`

They should share the same mock blueprint scenarios rather than inventing
separate route-specific fake graph data.

## 2. User Outcome

After this phase:

- Builder can browse both the knowledge inventory and blueprint graph routes in
  frontend mock mode
- project switching, filtering, and node selection all work from shared mock
  graph state

## 3. Scope

### In Scope

- frontend mock support for `/knowledge` and `/blueprint`
- shared mock project list and blueprint payloads
- browse-only selection/filter interactions already owned by the routes

### Out Of Scope

- graph editing or mutation
- project workspace/import/session route behavior
- backend search or graph-generation semantics

## 4. Contract

### 4.1 Required scenarios

This slice should support at minimum:

- `default`
  - one populated project graph
- `empty`
  - empty or nearly empty graph state
- `multi-project-graph`
  - at least two projects with distinct graph shapes for project switching

### 4.2 Shared blueprint source

The same mock blueprint payload family must drive both routes:

- `/knowledge` consumes it as inventory/detail
- `/blueprint` consumes it as graph canvas/detail

This keeps the two routes visually and semantically aligned.

## 5. Product Decisions

### 5.1 Keep these routes read-heavy

Mock mode should reinforce that these are browse/exploration surfaces.

Required behavior:

- project switching
- filter changes
- node selection

No fake editing workflows are needed in this slice.

### 5.2 Prefer graph coherence over graph size

Use smaller but believable blueprint graphs.

The point is to support Builder/UI design review, not to maximize fake node
count.

## 6. Touched Surfaces

- [knowledge route](/home/thetu/planner/planner-solid/src/routes/knowledge/index.tsx)
- [blueprint route](/home/thetu/planner/planner-solid/src/routes/blueprint/index.tsx)
- shared mock blueprint scenario modules

## 7. Acceptance Criteria

1. `/knowledge` and `/blueprint` both render in frontend mock mode without a
   backend
2. both routes draw from the same mock project/blueprint state family
3. project switching works coherently across the scenarios
4. filtering and node selection remain usable
5. empty and populated graph states are both covered

## 8. Verification Plan

- targeted browser proof in frontend mock mode for:
  - knowledge route filtering and node selection
  - blueprint route project switching and node selection
  - empty graph scenario
- targeted tests for shared scenario consumption across both routes

## 9. Rollback / Fallback

If the multi-project graph scenario is too broad in one pass:

- ship one populated project and one empty-state project first
- add richer multi-project distinctions after the shared graph source is
  working

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice moved the graph and inventory routes onto one shared frontend mock
blueprint source:

- the scenario registry now includes empty, default, and multi-project graph
  variants
- `/knowledge` and `/blueprint` now consume the same project list and blueprint
  payload family under frontend mock mode
- project switching, filtering, and selection remain route-owned interactions
  over coherent in-memory graph state

Primary implementation surfaces:

- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/knowledge/index.tsx)
- [index.tsx](/home/thetu/planner/planner-solid/src/routes/blueprint/index.tsx)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/knowledge?mockScenario=multi-project-graph`
  - `/blueprint?mockScenario=multi-project-graph`
  - project switching on the knowledge route
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
