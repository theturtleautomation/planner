# Planner SolidStart Phase 17 Workflow Closeout And React Retirement Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 00 Shell, Sessions, And Socratic Anchor Spec](/home/thetu/planner/docs/planner-solidstart-phase-00-shell-sessions-and-socratic-anchor-spec.md), [Planner SolidStart Phase 16 Project Import Comparison And Selection Summary Spec](/home/thetu/planner/docs/planner-solidstart-phase-16-project-import-comparison-and-selection-summary-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)

> Planning note (2026-03-24): after Phase 16, SolidStart covers the primary
> route family and the project-local import workflow. The next widening slice
> should not invent another major route. It should close the remaining workflow
> holes, complete the highest-value session actions in Solid, and retire
> `planner-web` as an active product surface.
>
> Implementation sync (2026-03-24): the Solid session workspace now exposes
> duplicate, export, restart-from-description, and retry-pipeline actions
> directly in the active session route. The project workspace and import desk
> now expose project-local reimport entry points, and the root Makefile,
> installer, server docs, and README now point at `planner-solid` as the active
> frontend target. Browser proof covers the full project -> session -> import
> workflow loop inside Solid.

## 1. Executive Judgment

The next SolidStart slice should be a **workflow closeout and platform
retirement** pass.

The main route family is already in place. The remaining product risk is no
longer missing navigation breadth. It is that a few important workflow actions
still feel scattered, and the repo still carries a live React surface that can
confuse deployment, scripts, and product ownership.

This slice should make SolidStart the only active frontend surface for normal
Planner use while closing the last high-value workflow actions needed for daily
operation.

## 2. User Outcome

After Phase 17:

- the normal Planner workflow is complete inside SolidStart
- high-value session actions no longer require falling back to old React-era
  assumptions or ad hoc backend calls
- project-to-session-to-import work reads as one coherent product flow
- repo, build, and deployment expectations no longer treat `planner-web` as an
  active product frontend
- the user-facing platform story is unambiguous: Planner runs through the Solid
  app

## 3. Problems To Solve

### 3.1 Remaining workflow gaps

Solid now covers the main route family, but the product still lacks a final
closeout pass over the most important operational actions:

- session duplication
- session export
- restart-from-description
- retry pipeline
- project-local reimport entry clarity
- project/session event drill-ins where they materially complete the workflow

These are not large new route families. They are the remaining pieces that make
the product feel finished.

### 3.2 Platform ambiguity

Even after the Solid route family landed, `planner-web` still exists in the
repo as a substantial prior frontend. That is acceptable historically, but it
must stop reading like a parallel active product surface.

The future state should be:

- SolidStart is the active frontend
- React is retained only as historical baseline and migration reference

## 4. Scope

### In Scope

- close the highest-value session workflow actions inside Solid
- close the remaining project-local workflow links needed to keep project,
  session, Socratic, and import work coherent
- make Solid the sole active frontend in repo-level scripts, docs, and
  deployment assumptions where feasible
- add browser proof for the full workflow loop, not just isolated route visits
- downgrade React product surfaces and docs from active to historical where the
  Solid replacement now exists

### Out Of Scope

- deleting `planner-web` from the repository entirely
- non-local auth redesign
- backend rewrites unrelated to the workflow closeout
- a brand-new top-level route family invented only to continue phase widening

## 5. Product Contract

### 5.1 Session workflow completion

The Solid session surfaces must expose the highest-value lifecycle controls in a
clear, low-noise way:

- duplicate session
- export session
- restart from description
- retry pipeline

These controls must be:

- present where users naturally operate on sessions
- visually secondary to the main workflow
- available without forcing users into buried utility-only views

### 5.2 Project workflow coherence

The project workspace must preserve the project-first operating model while
making these transitions clear:

- project to session
- session back to project context
- project to import review/history/reimport
- project to build/readiness/outputs/activity

This slice should remove any remaining “where do I do this now?” ambiguity.

### 5.3 React retirement posture

After this phase:

- `planner-web` must not be presented as an active frontend target in planning
  or routine developer workflow
- root scripts, docs, and deployment notes should point at the Solid app as the
  active frontend where the replacement now exists
- React remains only as historical implementation evidence and migration source

## 6. Technical Contract

### 6.1 Solid surface completion

The Solid app should add or tighten the remaining workflow actions by extending
existing routes and views, not by broadening the route tree unless a route is
strictly necessary.

Preferred pattern:

- keep project and session work inside the existing project/session route
  family
- use attached panels, concise action groups, and explicit status affordances
  rather than utility sprawl

### 6.2 Deployment and script clarity

Repo-level scripts and docs should align around:

- `planner-solid` as the active frontend
- `planner-server` serving the Solid artifact
- no routine expectation that developers boot or verify `planner-web` for the
  current product path

### 6.3 Browser proof

This slice must verify the real user loop:

1. enter via projects or sessions
2. open or create active work
3. move into Socratic/session work
4. exercise at least one lifecycle action
5. return to project context
6. move into import work without route confusion

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/sessions/*`
- `planner-solid/src/routes/projects/*`
- `planner-solid/src/lib/*` shared session/project view-model helpers
- `planner-solid/e2e/*` workflow proof
- repo root scripts and docs that still imply React is active
- planning docs that still describe React-era product surfaces as live targets

## 8. Acceptance Criteria

This phase is complete only when:

1. the key remaining session lifecycle actions exist in Solid and are reachable
   from the natural session workflow
2. the project workspace keeps project, session, Socratic, and import work
   readable as one coherent loop
3. repo docs and routine scripts no longer present `planner-web` as an active
   frontend target for current product work
4. browser verification proves the end-to-end project -> session -> Socratic
   -> import loop works in Solid without route confusion
5. `project-plan.md` truthfully marks the Solid route-family migration as
   closed out after this slice unless a new materially different platform gap is
   discovered

## 9. Verification Plan

- Solid unit tests for any new workflow helpers
- Solid browser coverage for:
  - session duplicate/export/restart/retry where implemented
  - project-to-session handoff
  - return from session work into project context
  - project-local import re-entry
- build/lint/test verification for `planner-solid`
- one explicit plan/docs review proving the platform story now points at Solid
  as the active frontend

## 10. Rollback / Fallback

If this slice proves too large in one pass:

- keep the route family unchanged
- land the session lifecycle actions first
- defer repo-level React retirement wording to a narrow follow-on closeout note

But the target remains the same: do not open another arbitrary feature route
just to continue phase expansion.

## 11. Open Questions

None that block implementation.

The remaining work is bounded and should now be executed as a closeout slice,
not treated as another broad planning branch.

## 12. Readiness Judgment

This spec is **implemented**.
