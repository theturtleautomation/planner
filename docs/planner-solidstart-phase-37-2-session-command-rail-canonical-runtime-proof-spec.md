# Planner SolidStart Phase 37.2 Session Command Rail Canonical Runtime Proof Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md)  
**Related Planning:** [Planner SolidStart Phase 35.3 Session Workspace Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-3-session-workspace-frontend-mock-spec.md), [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md), [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Delivery Attempt Note (2026-04-01):** an earlier delivery attempt found a broader canonical-runtime blocker before the session route could be proven. That bootstrap failure is now repaired by [Planner SolidStart Phase 37.3 Canonical Static Runtime Parity Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-3-canonical-static-runtime-parity-remediation-spec.md), so this slice is unblocked and returns to its intended proof-only scope.

## 1. Purpose

Prove that the Phase 37 command-rail session workspace is truthful outside the
frontend-mock runtime.

Phase 37 already has strong Builder-facing proof on the `3000`
frontend-mock path. This slice adds the missing complementary proof that the
same session surface behaves correctly under the canonical server-backed route
contract instead of only under mock-only browsing conditions.

## 2. Problem

The current implementation evidence for Phase 37 is heavily weighted toward the
frontend-mock runtime.

That is necessary for Builder UI review, but it leaves one verification gap:

- the session hierarchy should also be proven on the canonical route surface
  served through `planner-server`
- any SSR, hydration, transport, or saved-brief differences in the canonical
  runtime should be caught explicitly instead of assumed away

Without this slice, the session route is implemented and browser-proven in mock
mode, but canonical-runtime parity remains an inference.

## 3. User Outcome

After this phase:

- the same command-rail hierarchy is explicitly proven in the canonical
  server-backed session route
- the repo can say with evidence that Builder edits to the session route are
  still landing on the same real session surface later served by
  `planner-server`
- regressions specific to SSR, hydration, or session transport are more likely
  to be caught before the route drifts

## 4. Scope

### In Scope

- canonical-runtime browser proof for `/sessions/:sessionId`
- route-level selectors or test hooks only if needed to make proof stable
- parity checks for header compaction, rail switching, active-thread updates,
  and queued-work subordination
- proof that the command rail is not a frontend-mock-only behavior

### Out Of Scope

- redesigning the session route again
- broad backend changes
- replacing the existing frontend-mock proof
- inventing new runtime APIs purely for testing convenience

## 5. Contract

- the session route served by `planner-server` must expose the same truthful
  command-rail hierarchy already implemented in `planner-solid`
- canonical proof must exercise the real route behavior, not a per-test route
  interception that bypasses the actual session surface
- any stabilization helpers introduced for proof must remain truthful to the
  existing runtime contract
- this slice may fix route-surface issues discovered during proof, but it must
  stay bounded to parity and verification rather than reopening Phase 37

## 6. Required Proof Surface

Canonical proof should establish all of the following on the server-backed
runtime:

- the compact session header renders correctly
- there is one command rail and one dominant active-thread work area
- selecting a different thread updates the active work area without a route
  transition
- queued-later work remains subordinate
- draft-save and commit flow continue to work or remain truthfully represented
  under the canonical runtime

The exact test harness may vary, but the route evidence must be against the
real server-backed session experience.

## 7. Touched Surfaces

- canonical-runtime Playwright config or route-proof harness in `planner-solid`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts` only if shared helpers are
  extracted
- `planner-solid/src/routes/sessions/session-workspace-screen.tsx` only if
  proof surfaces uncover truthful parity fixes
- optional helper docs in `docs/builder-local-workflow.md` if the runtime proof
  requires a documented local launch path

## 8. Acceptance Criteria

1. the repo has explicit browser-proof coverage for the Phase 37 session route
   on the canonical server-backed runtime
2. that proof validates the command rail, active-thread work area, and queued
   work hierarchy rather than only route reachability
3. the proof does not depend on fake per-test session markup unrelated to the
   real route surface
4. any parity issues found during proof are either fixed in the slice or
   called out explicitly as blockers
5. the repo can now point to complementary proof for both:
   - the Builder-facing frontend-mock runtime
   - the canonical server-backed runtime

## 9. Verification Plan

- run the canonical runtime needed for proof
- execute the new canonical route proof
- re-run `npm --prefix planner-solid run test:e2e:frontend-mock` if shared
  helpers change
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

## 10. Rollback / Fallback

If the canonical runtime cannot yet support stable automated proof:

- reduce this slice to a truthful manual-verification harness and documented
  local procedure
- do not claim canonical-runtime parity is proven by the frontend-mock suite
  alone

## 11. Open Questions

1. Is the cleanest proof path a dedicated `planner-server` Playwright config,
   or a documented launch harness reused by an existing spec file?
2. Does the canonical runtime already expose enough deterministic state for one
   stable session proof, or should a minimal seeded-session contract be added
   first?

## 12. Implementation Update

Implemented on 2026-04-01.

What landed:

- extended the canonical server-backed proof harness in
  [phase-37-canonical-static-runtime.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-37-canonical-static-runtime.spec.ts)
  so it now proves the actual Phase 37 command-rail route contract instead of
  only route reachability
- reused the dedicated `planner-server` Playwright config added in
  [Planner SolidStart Phase 37.3 Canonical Static Runtime Parity Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-3-canonical-static-runtime-parity-remediation-spec.md)
  rather than inventing a fake route or markup harness

Verification completed:

- `npm --prefix planner-solid run test:e2e:canonical-static`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Verification result:

- the canonical `planner-server` runtime now has explicit browser proof for the
  compact session header, one command rail, and one dominant active-thread work
  area
- the proof shows thread switching remains local to the route without a URL
  change
- the proof stays truthful to the server-backed runtime by adapting to the
  route's real session progression:
  - if the deterministic `phase26_live` profile exposes multiple live threads
    immediately, the proof switches threads directly
  - if the same runtime first exposes one live interview thread with queued
    work subordinate in the rail, the proof advances that real thread until the
    live bank widens, then performs the same rail-switch assertion
- the active answer composer and commit affordance are explicitly present after
  a canonical-runtime thread switch
- Phase 37 now has complementary proof for both the Builder-facing frontend
  mock runtime and the canonical server-backed runtime
