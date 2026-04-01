# Planner SolidStart Phase 37.3 Canonical Static Runtime Parity Remediation Spec

**Status:** ready for implementation  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md)  
**Related Planning:** [Planner SolidStart Phase 37.2 Session Command Rail Canonical Runtime Proof Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-2-session-command-rail-canonical-runtime-proof-spec.md), [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md), [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  

## 1. Purpose

Repair the broader canonical static-runtime failure that blocks server-backed
browser proof for the session route.

Phase 37.2 tried to prove the new command rail under `planner-server`, but the
attempt exposed a more fundamental problem: the built frontend crashes before
the route mounts. This slice isolates that runtime parity defect so Phase 37.2
can become a normal proof/verification pass again instead of carrying a hidden
runtime-remediation burden.

## 2. Problem

The server-backed runtime currently fails before the session route can be
proved.

Observed during the attempted 37.2 delivery:

- the dedicated `planner-server` proof harness could create real sessions under
  deterministic `phase26_live`
- the built static runtime crashed on load for both `/` and `/sessions/:id`
- the bundled client runtime threw:
  `TypeError: Cannot read properties of undefined (reading 'done')`
- the failure occurred before the Phase 37 route selectors or session transport
  behavior could be evaluated

This means the missing parity proof is currently blocked by a runtime bootstrap
problem, not by insufficient test intent.

## 3. User Outcome

After this phase:

- the built frontend served by `planner-server` loads successfully again on at
  least `/` and `/sessions/:sessionId`
- canonical server-backed browser proof is unblocked for Phase 37.2
- Planner can treat frontend-mock and server-backed session verification as
  complementary evidence instead of living with a known static-runtime gap

## 4. Scope

### In Scope

- diagnosing and fixing the built static-runtime crash in the server-backed
  Planner frontend
- the minimal build/runtime changes required to restore truthful client
  bootstrap under `planner-server`
- a bounded verification harness proving that `/` and `/sessions/:sessionId`
  mount instead of crashing
- documentation or plan sync needed to make the blocker status explicit

### Out Of Scope

- reopening the Phase 37 session layout design
- broad Builder workflow redesign
- replacing the frontend-mock runtime
- claiming full canonical route parity proof for Phase 37.2 inside this slice

## 5. Contract

- the fix must restore a truthful built frontend runtime under `planner-server`
  rather than masking the crash with mock-only behavior
- the remediation should stay at the runtime/bootstrap layer unless route-level
  defects are conclusively part of the root cause
- this slice exists to unblock canonical proof; it does not replace the need
  for the actual Phase 37.2 parity proof afterward
- any build/export script changes must remain compatible with the current
  Builder/server-backed workflow documented for Planner

## 6. Root-Cause Surface To Inspect

The initial delivery attempt narrowed the likely fault area to the built
frontend bootstrap path rather than session-specific controller logic.

Primary surfaces to inspect:

- `planner-solid/scripts/export-static.mjs`
- `planner-solid/src/entry-client.tsx`
- the built `planner-solid/dist/static` output and generated manifest/runtime
  assumptions
- `planner-server` static serving path and any expectations about manifest or
  bootstrap globals

The route-level session files should only be revisited if the runtime crash is
demonstrably caused by session hydration rather than app bootstrap.

## 7. Touched Surfaces

- `planner-solid/scripts/export-static.mjs`
- `planner-solid/src/entry-client.tsx` if required
- optional build/runtime helpers or verification harness files in
  `planner-solid`
- `docs/project-plan.md` for blocker closeout and re-promotion of 37.2 if
  fixed

## 8. Acceptance Criteria

1. the built frontend no longer crashes on initial load under the canonical
   server-backed runtime
2. `/` and `/sessions/:sessionId` can be loaded in a browser against
   `planner-server` without the `reading 'done'` client crash
3. the fix is validated against the real built runtime rather than only Vite
   dev or frontend-mock mode
4. Phase 37.2 is now truthfully unblocked, or any remaining blocker is
   narrowed further and documented

## 9. Verification Plan

- `npm --prefix planner-solid run build`
- launch `planner-server` against `./planner-solid/dist/static`
- verify `/` loads without client bootstrap crash
- verify `/sessions/:sessionId` loads without client bootstrap crash
- `npm --prefix planner-solid run lint`

If an automated canonical-runtime harness naturally falls out of the fix, it
may be added here as long as the slice stays focused on runtime parity rather
than full Phase 37.2 route proof.

## 10. Rollback / Fallback

If the root cause proves larger than one bounded slice:

- keep 37.2 explicitly blocked
- record the narrowed bootstrap findings truthfully
- do not claim the server-backed route is healthy based on frontend-mock proof

## 11. Open Questions

1. Is the crash caused by the export/build path, the client bootstrap path, or
   an assumption mismatch between generated assets and `planner-server` static
   serving?
2. Does the runtime need a permanent export-path correction, or is the defect a
   smaller regression in the current built asset contract?
