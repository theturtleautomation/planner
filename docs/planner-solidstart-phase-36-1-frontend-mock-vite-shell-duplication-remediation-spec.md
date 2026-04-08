# Planner SolidStart Phase 36.1 Frontend Mock Vite Shell Duplication Remediation Spec

**Status:** implemented  
**Date:** 2026-03-31  
**Parent:** [Planner SolidStart Phase 36 Home Project Directory Consolidation Spec](/home/thetu/planner/docs/planner-solidstart-phase-36-home-project-directory-consolidation-spec.md)  
**Related Planning:** [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md), [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-31 live frontend-mock runtime inspection during Phase 36 delivery showed that `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid` renders two `.app-shell` trees in the Vite dev runtime, leaving a second duplicated shell below the fold and destabilizing the top-of-home create flow

## 1. Executive Judgment

Phase 35.10 made the frontend mock Vite runtime the canonical Builder UI-review
path, and Phase 36 successfully consolidated home and projects onto `/`.

But the delivery proof also exposed one remaining runtime-quality defect in the
Builder-facing dev path:

- the Vite frontend-mock runtime renders two `.app-shell` trees inside `#app`
- the duplicate shell is visible below the fold
- the duplication destabilizes the new home composer because the first visible
  shell can behave differently from the later hydrated shell

This is not a new product-direction question. It is a bounded runtime
remediation.

The next slice should therefore:

- restore one rendered app shell in the frontend-mock Vite runtime
- keep the Builder-facing `3000` workflow unchanged
- preserve the Phase 36 home-project consolidation outcome
- remove any temporary create-flow workaround that is only needed because of the
  duplicated shell, if that cleanup falls naturally out of the root-cause fix

## 2. User Outcome

After this remediation:

- the frontend-mock Vite runtime used by Builder shows one app shell only
- the page no longer renders a second empty shell below the fold
- the home create-project flow behaves consistently in the same visible route
  tree that Builder is editing
- top-level shell navigation remains truthful in frontend mock mode
- Builder/UI review on `http://127.0.0.1:3000` feels like one real app instead
  of one visible app plus one hidden duplicate render

## 3. Problem

The Builder-facing frontend mock contract currently says:

- one frontend-only mock runtime
- one shared route/component surface
- truthful click-through route browsing

But the actual dev runtime still violates that in one important way:

- two `.app-shell` trees are present under `#app`
- the second shell repeats header and nav chrome
- the duplicate render creates ambiguous form and interaction ownership

That creates three concrete risks:

### 3.1 Builder review quality is degraded

A UI-review runtime with duplicated shell chrome is not a truthful editing
surface, even if navigation mostly works.

### 3.2 The new home composer needs defensive fallback behavior

Phase 36 had to preserve create continuity even when the visible home form was
not the only rendered app tree. That is acceptable as a bounded workaround, but
it should not be treated as the desired steady state.

### 3.3 Future route-level design work will be harder to trust

If the dev runtime renders duplicated app trees, future Builder edits on shell,
layout, or top-level route composition can produce misleading visual or
interaction results.

## 4. Scope

### In Scope

- identifying and fixing the duplicated `.app-shell` render in the frontend
  mock Vite runtime
- preserving the `3000` Builder/frontend-mock workflow contract
- preserving the Phase 36 home-project consolidation behavior while the runtime
  duplication is removed
- simplifying any temporary create-flow workaround introduced solely for the
  duplication defect, if safe to do within the same bounded pass
- browser proof and route assertions that explicitly verify a single app shell
  in the frontend-mock dev runtime

### Out Of Scope

- changing the chosen Builder runtime away from Vite mock mode on `3000`
- redesigning the home/project layout again
- changing project/session/backend data contracts
- reopening the broader Phase 35 route-coverage tranche
- unrelated shell or route IA work

## 5. Contracts

### 5.1 Single-shell runtime contract

In the Builder-facing frontend-mock Vite runtime, the app must render exactly
one root shell.

Required result:

- `#app` contains one `.app-shell`
- one primary `main` region is rendered
- the shell header and primary nav appear once

Not acceptable:

- one visible shell plus a second duplicate shell lower on the page
- duplicate nav landmarks that differ only by hydration state

### 5.2 Shared-surface contract remains locked

The fix must preserve the existing shared-surface truth from Phase 35.10:

- Builder still edits the real `planner-solid` route and shell surfaces
- the frontend mock runtime still swaps data/runtime seams only
- the fix must not introduce a mock-only alternate app bootstrap or shell tree

### 5.3 Home create-flow continuity contract

After the duplication fix:

- creating a project from the home composer must still land in the created
  project workspace
- returning home must still show that project in the directory
- if the temporary fallback bridge from Phase 36 becomes unnecessary, it may be
  removed

Acceptable outcomes:

- keep the fallback bridge if it is still the smallest truthful solution
- remove it if the single-shell runtime fix makes direct route handling fully
  sufficient

### 5.4 Builder runtime contract remains singular

This remediation must not reopen the already-closed Builder runtime choice.

The canonical UI-review workflow remains:

- `http://127.0.0.1:3000`
- `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid`

## 6. Candidate Touched Surfaces

- `planner-solid/src/app.tsx`
- `planner-solid/src/entry-client.*` or equivalent SolidStart/Vite bootstrap
  surface if the duplicate mount originates there
- frontend-mock runtime/provider surfaces if they are participating in the
  duplicate render
- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/routes/projects/new.tsx`
- `planner-solid/src/lib/api-provider.ts`
- frontend-mock Playwright proof and any helper assertions needed to verify one
  shell only

## 7. Acceptance Criteria

This remediation is complete only when:

1. the frontend-mock Vite runtime renders one `.app-shell` on `/`
2. the duplicated below-the-fold shell/header/nav no longer appears
3. the home create-project flow still succeeds and lands in the created project
   workspace
4. returning to `/` still shows the created project in the directory
5. frontend-mock top-level route browsing still works after the runtime fix
6. Builder-facing runtime guidance stays unchanged at `3000` and does not
   regress to the server-backed path

## 8. Verification Plan

- targeted browser proof in the frontend-mock Vite runtime for:
  - shell count assertion on `/`
  - create-project flow from the home composer
  - return to `/` with created project visible
  - top-nav route continuity after the duplication fix
- `page.evaluate(...)` or equivalent browser assertion that
  `document.querySelectorAll('.app-shell').length === 1`
- standard `planner-solid` lint/build verification

## 9. Rollback / Fallback

If the exact root-cause fix is larger than expected:

- keep the runtime choice and route behavior unchanged
- land the smallest correction that removes the visible duplicate shell first
- defer cleanup of the Phase 36 fallback bridge rather than broadening the
  remediation into a larger bootstrap rewrite

## 10. Open Questions

No open product questions block readiness.

The remaining uncertainty is technical root cause, not feature scope:

- whether the duplicate shell comes from app bootstrap, dev SSR/hydration, or a
  route-level render path

That does not block readiness for a bounded runtime remediation slice.
