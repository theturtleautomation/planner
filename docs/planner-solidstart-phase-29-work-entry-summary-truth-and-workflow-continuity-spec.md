# Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 28 Session Entry And Startup Product Flow Spec](/home/thetu/planner/docs/planner-solidstart-phase-28-session-entry-and-startup-product-flow-spec.md)  
**Related Planning:** [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md), [Planner SolidStart Phase 17 Workflow Closeout And React Retirement Spec](/home/thetu/planner/docs/planner-solidstart-phase-17-workflow-closeout-and-react-retirement-spec.md), [Planner SolidStart Phase 20 Project Surfaces Local-App And Primitive Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-20-project-surfaces-local-app-and-primitive-hardening-spec.md), [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 repo-specific workflow follow-on analysis across `planner-solid/src/routes/index.tsx`, `planner-solid/src/routes/projects/index.tsx`, `planner-solid/src/routes/projects/new.tsx`, `planner-solid/src/routes/projects/[projectSlug].tsx`, `planner-solid/src/routes/sessions/index.tsx`, `planner-solid/src/routes/sessions/new.tsx`, `planner-solid/src/routes/sessions/[sessionId].tsx`, `planner-solid/src/lib/session-status.ts`, `planner-solid/src/lib/projects.ts`, `planner-solid/src/lib/advanced.ts`, and `planner-solid/src/components/projects/ProjectSessionList.tsx`

## 1. Executive Judgment

Phase 28 closed the saved-brief startup-truth thread, but the next highest-value
product issue is no longer another hidden runtime bug.

The active Solid app now has a narrower but still user-visible workflow drift:

- the truthful startup/status contract is strongest on `/sessions/:sessionId`
  but weakens on the surrounding work-entry, project, and queue surfaces
- summary rows and helper labels still fall back to raw `intake_phase` or thin
  local heuristics even though the repo now has a richer `workspace_status` and
  `resume_status` contract
- project-first remains the selected product direction, but the root empty
  state, session queue, and session return path still leave the flow feeling
  partially split between project-first and session-first mental models

The next slice should therefore be a bounded **workflow-continuity and summary-
truth** pass, not a silent route-family rewrite.

## 2. User Outcome

After Phase 29:

- `/`, `/projects`, `/projects/:projectSlug`, `/sessions`, and
  `/sessions/:sessionId` describe work through one backend-grounded status
  language instead of mixing truthful session projection with raw internal phase
  labels
- the root and project surfaces reinforce the selected project-first operating
  model without removing `/sessions/new` as an available secondary entry path
- active-session return navigation is explicit and stable whether the session is
  project-scoped or standalone
- the app feels more like one product flow from work entry through active
  session work and back out again
- the selected bank-first runtime and saved-brief startup contracts from Phase
  26 and Phase 28 remain unchanged

## 3. Problems To Solve

### 3.1 Status truth still stops at the session workspace

`planner-solid/src/lib/session-status.ts` already defines a richer
`workspace_status`-driven interpretation layer, but surrounding route surfaces
still expose raw `intake_phase` or a small local heuristic model:

- `/sessions` shows raw phase pills instead of truthful queue language
- the project session list shows raw `intake_phase`
- project activity summaries still serialize `intake_phase` into row copy
- project work summaries still choose "active/recent/attention" strictly from
  `interviewing` and `pipeline_running`

This produces avoidable product contradiction immediately after the Phase 28
truth hardening.

### 3.2 Project-first is selected, but route-level entry cues still drift

Phase 01 intentionally made the app project-first, but the current route family
still presents mixed signals:

- `/projects/new` creates the stable project container and lands in the project
  workspace
- `/sessions/new` still starts analysis directly
- the root empty state still advertises both paths at equal visual weight

The goal of this slice is not to delete a route. It is to reassert the already-
selected project-first hierarchy through clearer primary and secondary action
framing.

### 3.3 Return navigation is inconsistent during active work

The session workspace only offers "Back to project" when `project_slug` exists.
Standalone sessions have no equivalent explicit return path to the work queue or
work-entry surface.

That weakens continuity across:

- session creation
- active session work
- review/build handoff
- return navigation after a focused session visit

### 3.4 The repo has real route-family monoliths, but they are the wrong next slice

The project and session workspaces are structurally overgrown and should
eventually be refactored. But that is a larger architecture thread:

- `planner-solid/src/routes/projects/[projectSlug].tsx`
- `planner-solid/src/routes/sessions/[sessionId].tsx`

Folding that work into the next delivery slice would turn a truthful follow-on
into another broad redesign pass. This spec keeps those refactors explicitly out
of scope so the next implementation can stay bounded.

## 4. Product And Technical Decision

Phase 29 selects four strict decisions.

### 4.1 One route-family status language

The app should use one truthful status vocabulary across work-entry, queue,
project, and active-session surfaces.

Required behavior:

- prefer backend-grounded `workspace_status` and `resume_status` semantics
  wherever the route is summarizing or prioritizing session work
- do not expose raw `intake_phase` as the primary user-facing explanation on
  `/sessions`, project session lists, or project activity summaries
- keep row/status language low-noise and operational rather than verbose

### 4.2 Project-first remains primary, sessions-first remains secondary

This slice does not remove `/sessions/new`, but it does lock the route hierarchy
more clearly:

- project creation and project-local "start analysis" remain the dominant
  work-entry model
- `/sessions/new` remains available as a direct secondary path
- the root route should visually reinforce that projects are the main container
  for ongoing work

### 4.3 Every active session needs an explicit return path

Required behavior:

- project-scoped sessions keep a return path back to the owning project
- standalone sessions gain an explicit return path to `/sessions` or `/`
- return actions should feel like workflow continuity, not utility clutter

### 4.4 Major route-family refactors are deferred on purpose

This slice explicitly does **not** solve the larger structural problem by
rewriting:

- the project workspace route family
- the session workspace route family
- the overall information architecture around eliminating or collapsing routes

Those remain valid future specs, but they are not part of the next bounded
delivery cycle.

## 5. Scope

### In Scope

- status and next-action truth on:
  - `/`
  - `/projects`
  - `/projects/:projectSlug`
  - `/sessions`
  - `/sessions/:sessionId` topbar and return-navigation chrome
- shared helper changes required to drive summary truth from backend-facing
  session state
- project-first versus session-first action weighting on the root work-entry and
  nearby empty states
- a consistent explicit return-navigation contract from active session work
- the already-landed root typography correction only as baseline context, not
  as new scope

### Out Of Scope

- project or session route-family decomposition
- redesigning the active session interview/artifact workspace
- changing the selected bank-first runtime contract from Phase 26
- changing the saved-brief startup contract from Phase 28
- deleting `/sessions/new`
- broad IA removal of `/sessions` as a route family
- dependency changes

## 6. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/[projectSlug].tsx`
- `planner-solid/src/routes/sessions/index.tsx`
- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/components/projects/ProjectSessionList.tsx`
- `planner-solid/src/lib/session-status.ts`
- `planner-solid/src/lib/projects.ts`
- `planner-solid/src/lib/advanced.ts`
- route-level tests and browser proof covering summary truth and return
  navigation

## 7. Acceptance Criteria

This slice is complete only when:

1. `/sessions` no longer uses raw `intake_phase` as the primary visible row
   status
2. project session summaries and project activity summaries no longer describe
   active work through raw `intake_phase` strings
3. root and project surfaces still support both project-first and session-first
   entry, but project-first is visually and behaviorally primary
4. `/sessions/:sessionId` exposes one explicit return path for both
   project-scoped and standalone sessions
5. no Phase 26 or Phase 28 startup/runtime truth contract is weakened or
   replaced by new client heuristics
6. verification proves the updated route-family summary truth on real route
   objects rather than only helper-unit expectations

## 8. Verification Plan

- targeted helper tests for any new session-summary/status mapping logic in
  `planner-solid/src/lib/*`
- route tests for:
  - root/work-entry CTA hierarchy
  - session queue row status rendering
  - project session-list status rendering
  - session return-navigation behavior for project-scoped and standalone
    sessions
- browser proof for at least one realistic flow:
  - enter through `/`
  - open project or session work
  - land on `/sessions/:sessionId`
  - use the explicit return path back into the surrounding workflow
- standard `planner-solid` build/lint/test verification

## 9. Rollback / Fallback

If the full route-family summary convergence proves too large in one pass:

- land `/sessions` plus project session-list truth first
- keep the root CTA rebalance and session return-navigation in the same slice if
  they remain low-risk
- continue to defer the larger route-family refactors rather than broadening the
  implementation opportunistically

## 10. Open Questions

None block readiness for this bounded slice.

The larger unresolved questions are intentionally deferred:

- whether `/sessions/new` should eventually remain a first-class top-level entry
  or move behind project-first entry
- whether `/projects/:projectSlug` and `/sessions/:sessionId` should become
  route families instead of monolithic route controllers
