# Planner SolidStart Phase 32 Work Entry IA And Session Route Topology Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md), [Planner SolidStart Phase 17 Workflow Closeout And React Retirement Spec](/home/thetu/planner/docs/planner-solidstart-phase-17-workflow-closeout-and-react-retirement-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 follow-on repo review of the implemented `/`, `/projects`, `/projects/new`, `/sessions`, and `/sessions/new` routes after Phase 30 and Phase 31 route-family decomposition

## 1. Executive Judgment

Phase 29 reasserted project-first entry, and Phase 30/31 removed the route
structure as the main blocker to making the topology decision explicit.

The repo is now clear enough to decide the long-term route hierarchy:

- keep `/` as a distinct work-entry hub
- keep `/projects` and `/projects/new` as the primary project-first path
- keep `/sessions` as a secondary utility queue, not a co-equal primary entry
- keep `/sessions/new` as a retained advanced direct-session path, but demote
  its language and CTA role so it reads as an intentional detour rather than a
  second main onboarding model

This phase should make that topology explicit in the product copy, CTA
hierarchy, and planning docs without deleting routes or reopening runtime
truth.

## 2. User Outcome

After this phase:

- the app has one explicit route hierarchy instead of a half-resolved
  project-first/session-first split
- project creation and project return remain primary
- direct-session entry remains available for focused one-off work, but reads as
  secondary everywhere it appears
- any later route removal or redirect work stays optional and must happen
  through an explicit migration plan

## 3. Problems To Solve

- `/sessions/new` is still structurally present, but it currently mixes
  "direct session" intent with "new session" language
- `/sessions` already behaves like a useful reopen queue, but the repo still
  treats it in planning as if its long-term role were undecided
- `/` is now aligned as a work-entry hub, but the retained role of that hub
  versus `/projects` is not yet written down as a durable product choice

## 4. Scope

### In Scope

- deciding the long-term role of:
  - `/`
  - `/projects`
  - `/projects/new`
  - `/sessions`
  - `/sessions/new`
- deciding which routes are primary, which are secondary, and where direct
  session entry should remain visible
- normalizing route copy and CTA hierarchy so the retained topology is obvious
- defining migration constraints so no route is removed silently later

### Out Of Scope

- implementing route removal or redirects
- changing runtime or startup truth
- structural decomposition of project/session route internals
- redesigning the active session workspace

## 5. Contract

- project-first remains the current operating model unless this spec explicitly
  changes it
- Phase 26 and Phase 28 runtime/startup truth remain fixed inputs
- Phase 30 and Phase 31 structural decomposition are already implemented inputs
- this phase must preserve the bank-first runtime and truthful saved-brief
  startup contract while only changing topology language and route-role clarity

## 6. Product Decision

### 6.1 Retained route topology

The long-term route model is:

- `/`
  remains a distinct work-entry hub that curates the next best project action
- `/projects`
  remains the canonical project directory and the main place to reopen or start
  project-scoped work
- `/projects/new`
  remains the primary creation path for new ongoing work
- `/sessions`
  remains a utility queue for reopening active sessions, including standalone
  sessions that have no project container
- `/sessions/new`
  remains available, but only as an explicit direct-session detour for
  one-off/focused work

### 6.2 Language and CTA hierarchy

Required hierarchy:

- project creation stays primary on `/`, `/projects`, and `/sessions`
- direct session entry stays secondary/subtle wherever it appears
- `/sessions/new` should use "direct session" framing in visible route copy so
  it does not read like a second primary onboarding model
- `/sessions` should describe itself as a queue/reopen surface, not as a peer
  to the project-first path

### 6.3 Deferred route removal

This phase does not remove routes.

If the repo later chooses to demote or remove `/sessions/new` or collapse route
families, it must first:

1. preserve a truthful direct-session fallback for projectless work
2. ship redirects and navigation fallback explicitly
3. update browser proof so route removal is proven rather than assumed
## 7. Touched Surfaces

- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/new.tsx`
- `planner-solid/src/routes/sessions/index.tsx`
- `planner-solid/src/routes/sessions/new.tsx`
- route-level browser proof for entry hierarchy and direct-session continuity

## 8. Acceptance Criteria

1. the retained long-term role of `/`, `/projects`, `/projects/new`,
   `/sessions`, and `/sessions/new` is explicit
2. direct-session entry is consistently framed as secondary/advanced rather
   than co-primary
3. no routes are silently removed or redirected
4. the product-plan thread stops carrying route-topology ambiguity informally
5. verification proves the retained hierarchy still supports direct session
   entry and project-first return continuity

## 9. Verification Plan

- targeted browser proof for:
  - `/` promoting project-first entry while retaining the secondary direct
    session path
  - `/sessions` presenting queue-first copy with project creation primary and
    direct session secondary
  - `/sessions/new` using direct-session framing while still creating a truthful
    saved-brief startup path
- reuse of the Phase 28 and Phase 29 browser proof surfaces
- standard `planner-solid` lint/build verification

## 10. Rollback / Fallback

If the full topology language cleanup feels too broad in one pass:

- keep the retained route set unchanged
- land the copy/CTA hierarchy first
- defer any deeper nav/shell cleanup rather than slipping into route removal

## 11. Open Questions

None block readiness for this bounded topology-decision slice. The major
decision is now explicit: retain the route set, but normalize its hierarchy.

## 12. Implementation Outcome

Implemented on 2026-03-26.

Phase 32 landed as a bounded route-topology clarification slice:

- `/`, `/projects/new`, `/sessions`, and `/sessions/new` now use copy and CTA
  hierarchy that explicitly keeps project-first creation primary
- `/sessions/new` now frames itself as a retained direct-session detour instead
  of a second primary onboarding path
- the route set remains unchanged, and the Phase 28/29 continuity contracts
  remain intact

Verification included a dedicated Phase 32 browser spec plus the retained
Phase 28 and Phase 29 proof surfaces, along with standard `planner-solid`
lint/build verification.
