# Planner SolidStart Phase 36.2 Home Route Canonicality Remediation Spec

**Status:** implemented  
**Date:** 2026-03-31  
**Parent:** [Planner SolidStart Phase 36 Home Project Directory Consolidation Spec](/home/thetu/planner/docs/planner-solidstart-phase-36-home-project-directory-consolidation-spec.md)  
**Related Planning:** [Planner SolidStart Phase 36.1 Frontend Mock Vite Shell Duplication Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-36-1-frontend-mock-vite-shell-duplication-remediation-spec.md), [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-31 code review of the implemented Phase 36 surfaces found two remaining contract gaps: the home composer still degrades through `/projects/new`, and `/projects` is only a client-side redirect page rather than a true route-level redirect or thin alias

## 1. Executive Judgment

Phase 36 materially improved the information architecture:

- `/` now carries the project directory
- the top navigation no longer advertises `Projects`
- the home surface is the visible starting point for project work

But the implementation still leaves two escape hatches that keep the old route
split alive under degraded or pre-hydration conditions:

- the home composer still uses `/projects/new` as its fallback path
- `/projects` still renders a distinct client-only redirect page before
  navigation takes over

Those are not new product questions. They are residual route-truth issues inside
the already-selected consolidation outcome.

The next bounded slice should therefore:

- make `/` the true canonical create surface, not just the hydrated one
- make `/projects` resolve to the canonical home surface without a distinct
  intermediate page
- preserve the already-implemented home directory layout and Phase 36.1 single-
  shell runtime behavior

## 2. User Outcome

After this remediation:

- `/` is the canonical project-entry surface in both hydrated and degraded
  runtime conditions
- creating a project from home no longer depends on bouncing through
  `/projects/new`
- `/projects` no longer behaves like a separate page that happens to redirect
  later
- Builder review on the frontend-mock runtime edits the same canonical work-
  entry route contract that the real app presents

## 3. Problem

The current Phase 36 implementation still violates its own canonical-route
contract in two places.

### 3.1 Home create falls back to the old secondary page

The home composer is visually correct, but its progressive-enhancement fallback
still points to `/projects/new`.

That keeps `/projects/new` in the critical path for route truth whenever the
home page submits before the client handler takes over.

### 3.2 `/projects` is still a distinct page until hydration

The current `/projects` route renders its own title and redirect copy, then
navigates to `/` in `onMount`.

That is not the same as:

- a route-level redirect, or
- a true thin alias of the home surface

It means the route remains a separate user-visible destination under
non-hydrated or partially hydrated conditions.

### 3.3 Builder-facing truth remains slightly split

Phase 35.10 and 36.1 locked Builder onto the frontend-mock runtime as the
canonical UI-review path. If route truth still differs before hydration, the
Builder-edited home surface is not quite the same contract as the broader app
surface.

## 4. Scope

### In Scope

- removing the home composer’s dependency on `/projects/new` as the fallback
  create path
- making `/projects` resolve to `/` through a truthful bounded mechanism
- preserving the current home directory layout and controls
- preserving Phase 36.1 single-shell runtime behavior
- targeted verification for canonical create flow and `/projects` compatibility

### Out Of Scope

- redesigning the home layout again
- removing `/projects/new` as a route if it still serves direct/manual access
- changing project workspace, session queue, or backend project semantics
- reopening the broader Phase 35 or Phase 36 tranches
- unrelated Builder workflow changes

## 5. Contracts

### 5.1 Canonical home create contract

`/` must own the primary create-project experience in a way that remains
truthful before hydration.

Required result:

- the home composer does not rely on redirecting the user into `/projects/new`
  as its fallback create path
- if a fallback path still exists for technical reasons, it must preserve the
  user’s experience as a canonical home-owned create action rather than a
  visible secondary-page handoff

Not acceptable:

- a form contract where home visually owns creation but degraded behavior still
  routes through the old secondary destination

### 5.2 `/projects` compatibility contract

`/projects` must remain compatible, but it must no longer present as a distinct
page.

Acceptable bounded outcomes:

- a route-level redirect that resolves to `/` without showing a separate page,
  or
- a thin alias that renders the same canonical home surface directly

Not acceptable:

- a dedicated `Projects` page shell with its own title or “Redirecting to
  home…” copy

### 5.3 Shared-surface contract

This remediation must preserve the existing shared-surface rule:

- the canonical work-entry UI remains the real `planner-solid` home route
- Builder/frontend-mock mode and the real server-backed app must still point at
  the same route/component surface
- the fix must not create a mock-only home behavior

### 5.4 Bounded fallback cleanup contract

Phase 36.1 explicitly allowed the temporary fallback bridge to remain if it was
still the smallest truthful solution.

This slice should now reevaluate that bridge narrowly:

- remove or tighten the fallback if it only exists to preserve the old route
  split
- keep any remaining seed/sync behavior only if it is still required for
  frontend-mock continuity after canonical route truth is restored

## 6. Candidate Touched Surfaces

- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/components/projects/ProjectCreateForm.tsx`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/new.tsx`
- `planner-solid/src/lib/api-provider.ts`
- route/runtime verification around `/` and `/projects`

## 7. Acceptance Criteria

This remediation is complete only when:

1. the home composer no longer depends on `/projects/new` as its visible or
   progressive fallback create path
2. `/projects` resolves to the canonical home surface through a route-level
   redirect or true thin alias
3. direct visits to `/projects` do not show a distinct intermediate “Projects”
   page
4. creating a project from `/` still lands in the created project workspace
5. returning to `/` still shows the created project in the directory
6. frontend-mock Builder runtime behavior remains aligned with the same
   canonical home route contract

## 8. Verification Plan

- targeted route proof for `/` create flow under the canonical home surface
- direct visit proof for `/projects` confirming route-level redirect or thin
  alias behavior
- frontend-mock browser proof confirming:
  - home create still works
  - returning home still shows the created project
  - `/projects` does not present a distinct intermediate page
- standard `planner-solid` lint/build verification

## 9. Rollback / Fallback

If the full canonical-route cleanup is larger than expected:

- prefer landing truthful `/projects` compatibility behavior first
- keep `/projects/new` available as an internal support route if needed
- do not widen the slice into a larger home redesign or project-route refactor

## 10. Open Questions

No open product questions block readiness.

The remaining decisions are implementation choices, not feature-definition
questions:

- whether `/projects` should become a true alias render or an earlier redirect
- whether the home create fallback can be removed entirely or should be reduced
  to a less user-visible technical bridge
