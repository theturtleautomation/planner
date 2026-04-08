# Planner SolidStart Phase 37.4 Session Question Chrome Reduction Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md)  
**Related Planning:** [Planner SolidStart Phase 33 Session Workspace Interaction And Artifact Refinement Spec](/home/thetu/planner/docs/planner-solidstart-phase-33-session-workspace-interaction-and-artifact-refinement-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 37.1 Session Command Rail Narrow-Width And Focus Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-1-session-command-rail-narrow-width-and-focus-continuity-spec.md), [Planner SolidStart Phase 37.2 Session Command Rail Canonical Runtime Proof Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-2-session-command-rail-canonical-runtime-proof-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-01 direct inspection of `planner-solid/src/routes/sessions/session-workspace-screen.tsx` and the current question-card styles in `planner-solid/src/app.css` after Phase 37 closeout

## 1. Purpose

Reduce the remaining per-question chrome noise inside the implemented Phase 37
command-rail session workspace.

Phase 37 corrected the macro hierarchy. The next bounded cleanup is local to
the active-thread work area: question cards still repeat several status and
autosave signals that are no longer earning their space now that only one
thread is dominant at a time.

## 2. Problem

The route is structurally correct, but the answering surface still carries
redundant local chrome:

- inactive cards repeat `Committed` and `Draft` badges even though preview text
  already communicates the same state
- the active card shows a separate save-state label plus a persistent autosave
  hint, which competes with the actual question and commit action
- `Current` remains a badge even though only one expanded card is visible and
  the active card already has dominant styling
- the user still reads several micro-status labels before getting to the
  question itself

This is a density problem, not a missing-feature problem.

## 3. User Outcome

After this phase:

- the active question feels calmer and more decisive
- inactive cards read as compact previews instead of status-heavy mini panels
- autosave remains truthful without repeating instructional copy on every
  active card state
- the route keeps the Phase 37 command-rail hierarchy while reducing local
  answer-surface noise

## 4. Scope

### In Scope

- question-card chrome and helper copy inside the Phase 37 session workspace
- active-question save-state presentation
- inactive-question preview-state presentation
- restrained relocation or reduction of autosave/help text
- browser-proof updates only if existing session-route assertions need to adapt

### Out Of Scope

- changing the command-rail layout
- changing backend draft-save or commit behavior
- changing thread switching or route topology
- adding new session actions
- reopening broader session-page redesign decisions outside the question area

## 5. Contract

- active-question editing, autosave, and commit behavior remain exactly
  truthful to the current controller/runtime contract
- the route may reduce or relocate status chrome, but it must not hide error
  state when draft save actually fails
- inactive previews must remain legible enough to scan answered vs unanswered
  work without restoring a heavy badge system
- the cleanup must work the same way in frontend-mock and canonical
  `planner-server` runtimes because it is the same shared session surface

## 6. Product Decision

### 6.1 Active question chrome

Required direction:

- treat the expanded card styling itself as the primary active-state signal
- remove the dedicated `Current` badge unless implementation proves it is still
  needed for accessibility
- keep the commit action visually dominant
- collapse passive save-state language so it is quieter than the question and
  action

### 6.2 Inactive preview chrome

Required direction:

- use the preview answer text as the main state signal for processed cards
- remove redundant `Committed` and `Draft` badges if the preview treatment
  already communicates the same truth
- keep question order and text visible, but reduce extra labels around them

### 6.3 Autosave guidance

Required direction:

- keep autosave truthful, but stop letting helper copy dominate the question
  block
- move the keyboard shortcut hint into a calmer treatment, such as a quieter
  footer line, helper text only on focus, or one thread-level note
- keep explicit error state visible when save fails

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/app.css`
- optional helper extraction in `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts` only if the visible
  session assertions need to adapt
- `planner-solid/e2e/phase-37-canonical-static-runtime.spec.ts` only if
  canonical assertions depend on exact helper text that is intentionally
  removed

## 8. Acceptance Criteria

1. the active question no longer depends on a redundant `Current` badge to
   read as active
2. inactive preview cards no longer repeat `Committed` and `Draft` badges when
   preview content already conveys state
3. autosave guidance is still truthful but visually quieter than the question
   and commit action
4. draft-save error state remains visible when the underlying save actually
   fails
5. the route feels lower-noise without changing thread switching, commit, or
   backend truth

## 9. Implementation Update

Implemented in:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts`
- `planner-solid/e2e/phase-37-canonical-static-runtime.spec.ts`

Delivered behavior:

- removes the redundant `Current`, `Committed`, and `Draft` micro-badges from
  question cards
- keeps autosave and keyboard guidance as one calmer thread-level note instead
  of repeating it on every active card
- preserves visible draft-save state when the controller reports a non-idle
  save state, including error styling for failure truth
- keeps the Phase 37 command rail, thread switching, and canonical/frontend
  parity contracts unchanged

## 10. Verification Evidence

- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run lint`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `npm --prefix planner-solid run test:e2e:canonical-static`

## 11. Rollback / Fallback

If the full chrome cleanup proves too subjective in one pass:

- keep the active-card save-state reduction
- keep the inactive-card badge reduction
- avoid broad CSS churn outside the question-card surfaces

Do not reopen the command-rail hierarchy or restore the old stacked session
layout as fallback.

## 12. Open Questions

None block readiness.

The implementation latitude is visual, not architectural: the route contract
is already settled, and this slice is only about making the active question
surface quieter and clearer.
