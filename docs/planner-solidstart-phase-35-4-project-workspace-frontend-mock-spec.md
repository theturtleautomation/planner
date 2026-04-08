# Planner SolidStart Phase 35.4 Project Workspace Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md), [Planner SolidStart Phase 35.2 Work-Entry And Queue Routes Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-2-work-entry-and-queue-routes-frontend-mock-spec.md), [Planner SolidStart Phase 35.3 Session Workspace Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-3-session-workspace-frontend-mock-spec.md)  
**Related Planning:** [Planner SolidStart Phase 30 Project Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-30-project-workspace-route-family-decomposition-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/projects/project-workspace-controller.ts` and `planner-solid/src/routes/projects/[projectSlug].tsx`

## 1. Executive Judgment

`/projects/:projectSlug` is the broadest aggregate workspace in the route
family.

It does not need live transport like the session route, but it does require a
coherent project-local summary model across:

- project detail
- session summaries
- prompt-bank summary
- review/readiness/build/activity summaries
- attached advanced surfaces

So this slice should make the project workspace browsable as a coherent
project-local desk, not as disconnected placeholder cards.

## 2. User Outcome

After this phase:

- Builder can open a mock project route and browse the main project workspace
- attached advanced surfaces can be opened and switched without a backend
- the route can navigate coherently to related session and import pages
- mock "start analysis" can create or select a mock session and move into the
  session route

## 3. Scope

### In Scope

- frontend mock support for `/projects/:projectSlug`
- mock project detail and project session summary data
- mock advanced-surface summary data already consumed by the route
- in-memory start-analysis flow

### Out Of Scope

- the dedicated import review route itself
- blueprint/knowledge routes outside the project workspace summary use
- backend-truth verification for project lifecycle actions

## 4. Contract

### 4.1 Required scenarios

This slice should support at minimum:

- `project-active`
  - active project with a current working session
- `project-ready`
  - project summary that reads as build/review ready
- `project-empty`
  - project exists but no active session yet

### 4.2 Start-analysis behavior

Required behavior:

- mock start-analysis writes a session into the mock store
- navigation continues to `/sessions/:sessionId`
- the project route remains consistent with the newly created mock session

### 4.3 Advanced surfaces

The route must support browsing its attached advanced tabs from mock data:

- review
- readiness
- build
- activity
- execution
- outputs

The slice only needs the currently consumed summary shapes, not deeper live
backend semantics.

## 5. Product Decisions

### 5.1 Keep the project workspace project-first

Mock mode must preserve the existing product decision:

- the project route remains the primary home for ongoing work
- advanced surfaces stay attached, not promoted into separate primary pages

### 5.2 Prefer coherent summaries over exhaustive fake detail

The mock route should prioritize truthful route composition:

- one coherent project state per scenario
- summary cards and tabs agree with one another
- avoid inventing excessive fake runs/events just to fill space

## 6. Touched Surfaces

- [project-workspace-controller.ts](/home/thetu/planner/planner-solid/src/routes/projects/project-workspace-controller.ts)
- [project route](/home/thetu/planner/planner-solid/src/routes/projects/%5BprojectSlug%5D.tsx)
- any project summary helpers touched by mock data assumptions
- shared mock provider/scenario modules

## 7. Acceptance Criteria

1. `/projects/:projectSlug` renders in frontend mock mode without a backend
2. the main project workspace and attached advanced tabs all browse from
   coherent mock state
3. mock start-analysis can navigate into a valid mock session route
4. project/session/import summary state agrees within each scenario
5. the route preserves the existing project-first product hierarchy

## 8. Verification Plan

- targeted browser proof in frontend mock mode for:
  - active project scenario
  - empty project scenario
  - advanced-tab switching
  - start-analysis navigation continuity
- targeted tests for project-local mock store updates

## 9. Rollback / Fallback

If start-analysis mutation is too broad in one pass:

- land coherent browse-only project scenarios first
- then add mock start-analysis as the route action closeout

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice made the project workspace and its attached advanced summaries
browsable from coherent frontend-only mock state:

- the scenario registry now includes active, ready, and empty project
  workspace variants
- mock project detail, project sessions, import posture, blueprint summary, and
  operational summary data now resolve through the frontend provider
- mock start-analysis continues into a valid mock session route without
  dropping the active scenario
- project-local navigation into import and session surfaces preserves route
  continuity during frontend-only browsing

Primary implementation surfaces:

- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [project-workspace-controller.ts](/home/thetu/planner/planner-solid/src/routes/projects/project-workspace-controller.ts)
- [ProjectWorkspaceHero.tsx](/home/thetu/planner/planner-solid/src/components/projects/ProjectWorkspaceHero.tsx)
- [ProjectSessionList.tsx](/home/thetu/planner/planner-solid/src/components/projects/ProjectSessionList.tsx)
- [ProjectAdvancedPanel.tsx](/home/thetu/planner/planner-solid/src/components/projects/ProjectAdvancedPanel.tsx)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/projects/personal-calendar?mockScenario=project-active`
  - `/projects/personal-calendar?mockScenario=project-ready&tab=review`
  - `/projects/personal-calendar?mockScenario=project-empty`
  - advanced-tab switching and start-analysis navigation continuity
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
