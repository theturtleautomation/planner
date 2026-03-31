# Planner SolidStart Phase 35.3 Session Workspace Frontend Mock Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md), [Planner SolidStart Phase 35.2 Work-Entry And Queue Routes Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-2-work-entry-and-queue-routes-frontend-mock-spec.md)  
**Related Planning:** [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 28 Session Entry And Startup Product Flow Spec](/home/thetu/planner/docs/planner-solidstart-phase-28-session-entry-and-startup-product-flow-spec.md), [Planner SolidStart Phase 26 Socratic Runtime Truth Completion Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-26-socratic-runtime-truth-completion-remediation-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/routes/sessions/session-workspace-controller.ts`, `planner-solid/src/routes/sessions/session-workspace-screen.tsx`, and the existing session Playwright fixtures

## 1. Executive Judgment

`/sessions/:sessionId` is the hardest route in the Phase 35 family.

Unlike the list and detail routes, it currently assumes:

- live session detail fetches
- prompt-bank fetches
- draft-save mutations
- websocket-style progression events

So this slice must explicitly make the session workspace interactable in
frontend mock mode instead of reducing it to a static rendered shell.

## 2. User Outcome

After this phase:

- Builder can open a mock session route and see the question-bank workspace
- drafts can be edited and saved in the frontend mock store
- commit-and-advance interactions work deterministically
- the route can simulate the startup and prompt-bank progression needed for UI
  design review

## 3. Scope

### In Scope

- frontend mock support for `/sessions/:sessionId`
- mock session detail, prompt bank, and prompt drafts
- mock transport events for startup and prompt progression
- in-memory save and processed-state continuity within the running app

### Out Of Scope

- proving backend/runtime truth
- full persistence across browser reload
- unrelated workspace redesign

## 4. Contract

### 4.1 Required scenarios

This slice should support at minimum:

- `session-workspace`
  - ready question-bank state with answerable prompts
- `session-startup`
  - saved brief, startup in progress, prompt bank arrives through mock
    transport
- `session-complete`
  - completed session with non-editing state
- `session-attention`
  - startup/connection issue state for visual coverage

### 4.2 Mock transport behavior

The mock session transport must be able to:

- open deterministically
- optionally emit a prompt-bank payload after startup
- receive prompt responses
- mark thread completion in the mock store
- emit convergence/pipeline-complete style refresh triggers when needed

### 4.3 Draft behavior

Required behavior:

- draft save uses the same route affordance as live mode
- saved drafts update the in-memory store
- navigation within the session route preserves those drafts during the running
  frontend session

## 5. Product Decisions

### 5.1 Keep the bank-first contract shape

The mock route must preserve the current product shape:

- question-bank workspace
- local thread jumping
- per-question answer blocks
- visible queued work

Mock mode should not invent a simplified session UI just to make the route
easier to fake.

### 5.2 Support design-relevant progression only

The mock transport only needs enough fidelity for browsing and design review.

Required direction:

- startup open
- prompt-bank reveal
- draft save
- commit-and-advance
- end-of-thread progression

It does not need to simulate the full real backend lifecycle.

## 6. Touched Surfaces

- [session-workspace-controller.ts](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-controller.ts)
- [session-workspace-screen.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-screen.tsx)
- route-local session helpers as needed
- shared mock provider/transport foundation from Phase 35.1

## 7. Acceptance Criteria

1. `/sessions/:sessionId` renders and remains usable in frontend mock mode
2. the mock route supports at least one startup path and one fully banked path
3. draft save mutates local mock state
4. commit-and-advance works deterministically without a backend
5. the route still visually matches the current question-bank workspace
   contract

## 8. Verification Plan

- targeted unit tests for mock session transport behavior
- targeted route tests for draft save and commit progression
- browser proof in frontend mock mode for:
  - startup reveal
  - banked question editing
  - commit-and-advance
  - completed/attention state coverage

## 9. Rollback / Fallback

If full commit progression is too broad in one pass:

- keep startup reveal and banked-question browsing as the first milestone
- then add local draft save
- then add deterministic commit progression

Do not fallback to a static screenshot-only session mock.

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice completed the backendless session workspace contract on top of the
Phase 35.1 and 35.2 foundation:

- the frontend mock scenario registry now covers startup, active, completed,
  and attention-needed session workspace states
- the session workspace transport remains deterministic in frontend mock mode
  while preserving the bank-first route contract
- draft save and commit-and-advance now mutate in-memory mock state without a
  backend
- session-local and return-path navigation preserves the active mock scenario

Primary implementation surfaces:

- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts)
- [api-provider.ts](/home/thetu/planner/planner-solid/src/lib/api-provider.ts)
- [session-workspace-controller.ts](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-controller.ts)
- [session-workspace-screen.tsx](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-screen.tsx)
- [session-workspace-view.ts](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-view.ts)

Verification evidence:

- targeted browser proof in frontend mock mode covered:
  - `/sessions/session-11?mockScenario=session-workspace`
  - `/sessions/session-12?mockScenario=session-startup`
  - `/sessions/session-13?mockScenario=session-complete`
  - `/sessions/session-14?mockScenario=session-attention`
  - banked-question editing plus deterministic `Commit and next`
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
