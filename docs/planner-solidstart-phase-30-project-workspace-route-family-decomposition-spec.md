# Planner SolidStart Phase 30 Project Workspace Route Family Decomposition Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md)  
**Related Planning:** [Planner SolidStart Phase 20 Project Surfaces Local-App And Primitive Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-20-project-surfaces-local-app-and-primitive-hardening-spec.md), [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-26 direct inspection of `planner-solid/src/routes/projects/[projectSlug].tsx`, `planner-solid/src/components/projects/*`, `planner-solid/src/lib/advanced.ts`, `planner-solid/src/lib/projects.ts`, and the Phase 29 verification surfaces

## 1. Executive Judgment

Phase 29 aligned workflow truth and entry continuity, but it intentionally did
not solve the structural problem inside the project workspace route family.

The current project workspace route remains a 334-line controller that owns:

- base project and session fetches
- attached-surface URL state
- advanced-surface resource loading
- derived project/build/review summaries
- project-local mutation handlers
- composition of the hero, session list, and advanced panel

That is now the highest-value next refactor because it directly affects the
repo's safest place to keep extending project workflow without reopening the
route-state drift that Phase 20 and Phase 29 just reduced.

This slice should decompose the project workspace route family while preserving
the current product behavior, URLs, and bank-first/session-startup contracts.

## 2. User Outcome

After Phase 30:

- `/projects/:projectSlug` still feels the same to users
- project loading, attached-surface state, and project-local actions are owned
  by clearer route-local modules instead of one expanding controller
- future work on project review/build/activity surfaces becomes cheaper and
  safer
- the route family stops accumulating behavior in one file before another
  product-flow slice lands

## 3. Problems To Solve

### 3.1 Route controller breadth

`planner-solid/src/routes/projects/[projectSlug].tsx` currently mixes route
loader behavior, summary derivation, tab-state control, project-session
selection, and mutation handlers in one route file.

### 3.2 Attached-surface resource coupling

The same route owns when to fetch blueprint, import, prompt-bank, run, event,
and export-history data for the advanced panel. That keeps the fetch policy and
the surface composition tightly coupled.

### 3.3 Mutation ownership drift

Starting analysis, review selection, apply, and reimport are all wired in the
same file that also renders the workspace. That makes behavior changes harder
to isolate and test.

### 3.4 Next-slice risk

If another project-workspace feature lands before this decomposition, the repo
will keep rebuilding local architecture inside the same route instead of
stabilizing the route family first.

## 4. Scope

### In Scope

- decomposing `planner-solid/src/routes/projects/[projectSlug].tsx` into
  clearer route-local modules, hooks, or components
- separating:
  - project workspace shell composition
  - attached-surface controller state
  - advanced-surface data loading and derived summaries
  - project-local mutation ownership
- keeping the existing URL-backed `tab` search-param behavior
- keeping the existing project-to-session and project-to-import workflow
  behavior
- keeping the existing rendered route family and user-visible labels unless a
  decomposition requires a tiny non-behavioral cleanup

### Out Of Scope

- changing the project workspace IA
- changing the selected project-first operating model
- redesigning the advanced panel content
- changing the bank-first runtime or saved-brief startup contract
- deleting routes or moving attached surfaces into new top-level routes
- session-route decomposition work

## 5. Product And Technical Contract

### 5.1 Behavior-preserving route split

Phase 30 is a structural slice, not a workflow redesign.

Required behavior:

- `/projects/:projectSlug`
  remains the primary project work surface
- the `tab` search param continues to open, close, and select the attached
  surface exactly as it does now
- the existing hero, session list, and advanced panel remain the visible route
  structure
- project-local actions keep the same route targets and backend calls

### 5.2 Stable data-ownership boundary

The refactor must create one clearer ownership boundary for:

- project/session selection and summary derivation
- advanced-surface lazy data loading
- project-local mutations and their pending/error state

The repo does not need a new framework abstraction. It does need fewer implicit
dependencies crossing one route body.

### 5.3 Future-spec handoff

This slice should leave the route family ready for:

- a later session-workspace route decomposition
- later project-workspace behavior changes without reopening structural churn

## 6. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/projects/[projectSlug].tsx`
- new route-local modules under `planner-solid/src/routes/projects/` or
  `planner-solid/src/components/projects/`
- `planner-solid/src/lib/advanced.ts`
- `planner-solid/src/lib/projects.ts`
- route/browser tests covering project-workspace continuity

## 7. Acceptance Criteria

This slice is complete only when:

1. the project workspace route is materially smaller and no longer owns every
   fetch, summary, and mutation inline
2. attached-surface URL state still works without regression
3. starting analysis, import review apply, and reimport still behave the same
4. no user-visible project-workspace product flow is silently redesigned
5. verification proves the decomposed route still covers the current project ->
   session -> import continuity

## 8. Verification Plan

- targeted tests for any extracted controller/helper logic
- browser proof for:
  - opening `/projects/:projectSlug`
  - changing attached surfaces with `tab`
  - starting a project analysis
  - re-entering project-local import/review flow
- standard `planner-solid` lint/test/build verification

## 9. Rollback / Fallback

If the full decomposition is too large in one pass:

- keep the route URLs and visible composition unchanged
- land the controller/data split first
- defer deeper component extraction rather than widening into IA work

## 10. Open Questions

None block readiness for this bounded structural slice.

## 11. Implementation Outcome

Implemented on 2026-03-26.

Phase 30 landed as a behavior-preserving project-route decomposition:

- `planner-solid/src/routes/projects/[projectSlug].tsx` is now a thin route
  wrapper
- project-workspace data loading, URL-backed surface state, and mutation
  ownership now live in extracted route-local controller/helper modules
- visible project workspace composition remains the same through the extracted
  screen component

Verification included targeted controller/helper tests, `planner-solid`
lint/build, and browser proof across the current project-workflow continuity
surfaces.
