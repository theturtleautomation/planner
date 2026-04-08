# Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 30 Project Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-30-project-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 22 Session Workspace Master-Detail Density And Autosave Spec](/home/thetu/planner/docs/planner-solidstart-phase-22-session-workspace-master-detail-density-and-autosave-spec.md), [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-26 direct inspection of `planner-solid/src/routes/sessions/[sessionId].tsx`, `planner-solid/src/lib/prompt-bank.ts`, `planner-solid/src/lib/session-status.ts`, and the live-proof/Playwright coverage landed through Phase 29

## 1. Executive Judgment

The session workspace route is still the repo's largest single controller.

`planner-solid/src/routes/sessions/[sessionId].tsx` is 1099 lines and currently
owns:

- session and prompt-bank loading
- websocket lifecycle and startup handshake behavior
- prompt-bank graph state and first-reveal logic
- draft autosave timing
- session lifecycle actions
- responsive interview/artifact surface state
- topbar action chrome and return navigation

That makes the route the next major structural refactor after the project
workspace split. But unlike the project route, the exact decomposition seam is
still coupled to the current interview/artifact workspace shape, so this spec
is drafted as the next architecture target rather than promoted to ready yet.

## 2. User Outcome

After this phase:

- `/sessions/:sessionId` keeps the same runtime truth and visible behavior
- websocket/runtime logic, prompt-bank state control, and pane rendering are no
  longer fused into one route file
- later session-workspace redesign work can happen on stable boundaries instead
  of inside a giant controller

## 3. Problems To Solve

- one route mixes runtime state, product state, and view composition
- startup/retry/live-attach behavior is too close to rendering details
- prompt-bank graph ownership and autosave ownership are hard to review safely
- responsive pane state and action chrome are tightly coupled to the runtime
  controller

## 4. Scope

### In Scope

- structural decomposition of `planner-solid/src/routes/sessions/[sessionId].tsx`
- separating runtime/session controller logic from pane rendering
- separating prompt-bank graph ownership, autosave ownership, and lifecycle
  action ownership
- preserving the current session URL, runtime truth, and return-navigation
  behavior

### Out Of Scope

- broad redesign of the active session UI
- changing the bank-first runtime contract
- changing the saved-brief startup contract
- route-topology or IA changes

## 5. Contract

- Phase 26 and Phase 28 runtime/startup truth remain locked
- the current session route family remains intact while its internals are split
- extracted boundaries should be stable enough to support a later workspace
  redesign without reworking runtime truth again

## 6. Touched Surfaces

- `planner-solid/src/routes/sessions/[sessionId].tsx`
- route-local session workspace modules/hooks
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/session-status.ts`
- browser proof for startup truth and active-session continuity

## 7. Acceptance Criteria

1. the session route is materially smaller and no longer owns all runtime and
   pane logic inline
2. Phase 26 and Phase 28 browser proof still passes
3. draft autosave, startup retry, duplicate/export/restart/retry, and return
   navigation still behave the same
4. no workspace redesign is silently bundled into the structural split

## 8. Verification Plan

- extracted-helper tests where useful
- the Phase 26, Phase 28, and Phase 29 browser proof surfaces
- focused browser checks around autosave and active-session actions

## 9. Open Questions

- what exact seam should own websocket/runtime lifecycle versus prompt-bank
  graph lifecycle
- whether the route should end as one slim coordinator or a small route family
- whether any extraction should wait for the later workspace-redesign decision

## 10. Implementation Outcome

Implemented on 2026-03-26 after Phase 30 established the route-family split
pattern.

Phase 31 landed as a structural session-route decomposition without reopening
runtime truth:

- `planner-solid/src/routes/sessions/[sessionId].tsx` is now a thin route
  wrapper
- websocket/runtime state, prompt-bank graph ownership, autosave/lifecycle
  actions, and surface-state control now live in
  `session-workspace-controller.ts`
- route composition now lives in `session-workspace-screen.tsx`
- shared view helpers for save-state copy, viewport buckets, and return target
  selection now live in `session-workspace-view.ts`

Verification included targeted helper tests, `planner-solid` lint/build, and
browser proof across the active-session, live-artifact, runtime-truth, startup,
and workflow-continuity surfaces.
