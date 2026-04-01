# Planner SolidStart Phase 37.5 Session Header Signal Consolidation Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md)  
**Related Planning:** [Planner SolidStart Phase 37.1 Session Command Rail Narrow-Width And Focus Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-1-session-command-rail-narrow-width-and-focus-continuity-spec.md), [Planner SolidStart Phase 37.2 Session Command Rail Canonical Runtime Proof Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-2-session-command-rail-canonical-runtime-proof-spec.md), [Planner SolidStart Phase 37.4 Session Question Chrome Reduction Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-4-session-question-chrome-reduction-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-01 direct inspection of `planner-solid/src/routes/sessions/session-workspace-screen.tsx` and the Phase 37 route header/inline status surfaces after Phase 37.4 closeout

## 1. Purpose

Reduce the remaining top-of-page density in the implemented Phase 37 session
workspace by consolidating header signals and transient feedback.

Phase 37 fixed the route hierarchy. Phase 37.4 quieted the question cards.
The remaining noise now sits above the active workspace: the header still
competes between route identity, status, aggregate progress, queued-work
counts, and session actions, and the route can stack multiple inline status
messages before the user reaches the work area.

## 2. Problem

The current route still over-explains session state at the top:

- the header shows both a state badge/detail row and a separate aggregate
  progress line
- queued-later work appears in the header even though it already lives in the
  rail disclosure
- session actions sit in a dedicated top-level block with similar visual weight
  to the route status
- action notice, action error, and submit error can stack as multiple sibling
  banners beneath the header

This is a signal hierarchy problem, not a backend truth problem.

## 3. User Outcome

After this phase:

- the session route starts with one calm identity/status block instead of a
  layered preamble
- the user reaches the active thread faster
- queued-work and secondary actions stay available without competing with the
  answering surface
- feedback remains truthful but is presented in one predictable place

## 4. Scope

### In Scope

- the Phase 37 session header structure
- header-level progress and status presentation
- top-level session action trigger/menu presentation
- inline route feedback placement for action/submit notice and error states
- session-route proof updates only if visible assertions change

### Out Of Scope

- changing question-card behavior or commit flow
- changing command-rail layout or narrow-width selector behavior
- changing backend session status contracts or action capability logic
- redesigning the active thread workspace again
- adding new session actions

## 5. Contract

- the route must still expose truthful session state from the current
  controller/runtime contract
- aggregate progress may be reduced or relocated, but not removed if it is the
  only truthful route-level summary left
- queued-later work must not regain peer visual weight with the active thread
- session actions must remain available in both frontend-mock and canonical
  `planner-server` runtimes because they target the same shared route surface
- error feedback must remain visible and cannot be silently collapsed away

## 6. Product Decision

### 6.1 Header structure

Required direction:

- keep the back link, session title, and one concise route-status line in the
  header
- demote or remove the separate aggregate progress line if the same signal is
  already carried more usefully by the rail and active-thread workspace
- stop showing queued-later counts as peer header metadata when queued work is
  already reachable from the rail disclosure

### 6.2 Session actions

Required direction:

- keep actions behind one compact overflow/disclosure trigger
- reduce the visual weight of the actions trigger so it reads as secondary to
  route identity and active work
- keep capability-driven action truth unchanged

### 6.3 Inline feedback

Required direction:

- use one predictable feedback slot near the top of the work area rather than
  stacking multiple sibling banners
- preserve truthful success/error semantics
- prioritize the most important current feedback if multiple messages compete

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/app.css`
- optional helper shaping in `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts` only if visible route
  assertions change
- `planner-solid/e2e/phase-37-canonical-static-runtime.spec.ts` only if
  canonical route assertions change

## 8. Acceptance Criteria

1. the session header reads as one compact identity/status block instead of a
   layered dashboard preamble
2. queued-later work no longer appears as peer header metadata when it is
   already represented in the rail
3. the actions trigger remains available but visually secondary
4. top-of-route feedback uses one predictable slot instead of stacking multiple
   banners
5. session status, action capability truth, and backend behavior remain
   unchanged

## 9. Implementation Update

Implemented in:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/app.css`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts`
- `planner-solid/e2e/phase-37-canonical-static-runtime.spec.ts`

Delivered behavior:

- removes the redundant header aggregate progress strip from the session route
  so the header now resolves to back link, title, one concise status line, and
  a secondary actions trigger
- stops surfacing queued-later work as peer header metadata because queued work
  already lives in the rail disclosure
- demotes session actions behind a lighter-weight `Actions` trigger without
  changing capability-driven action truth
- consolidates top-of-route notice/error messaging into one prioritized
  feedback slot instead of stacking multiple sibling banners

## 10. Verification Evidence

- `npm --prefix planner-solid run lint`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run test:e2e:canonical-static`

## 11. Rollback / Fallback

If the full consolidation proves too subjective in one pass:

- keep the header compaction
- keep the quieter actions trigger
- leave deeper feedback-slot consolidation for a follow-on

Do not reopen the command rail or restore a larger summary/dashboard header as
fallback.

## 12. Open Questions

None block readiness.

The main remaining implementation choice is presentation shape, not behavior:
whether the aggregate progress survives as a quieter single sentence or is
fully removed from the header in favor of the existing rail/workspace signals.
