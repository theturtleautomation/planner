# Planner SolidStart Phase 36 Home Project Directory Consolidation Spec

**Status:** implemented  
**Date:** 2026-03-31  
**Parent:** [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md)  
**Related Planning:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md), [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Planner SolidStart Phase 32 Work Entry IA And Session Route Topology Spec](/home/thetu/planner/docs/planner-solidstart-phase-32-work-entry-ia-and-session-route-topology-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-31 direct design review of the implemented `planner-solid` work-entry surfaces, including `planner-solid/src/routes/index.tsx`, `planner-solid/src/routes/projects/index.tsx`, and `planner-solid/src/app.tsx`

## 1. Executive Judgment

The current Solid app now has a cleaner shell and a working frontend mock
runtime, but the core work-entry information architecture still carries one
avoidable product contradiction:

- `/` is a guided work-entry surface
- `/projects` is the full operating directory
- the shell advertises both as top-level peers

That split made sense while the route family was still being widened, but it no
longer matches the preferred product feel. The current home route shows a
featured-project spotlight plus a partial recent list, while the actual project
directory truth lives one click away on `/projects`.

The next bounded correction is to consolidate them:

- `/` becomes the canonical home for project work entry and project browsing
- the topmost home surface becomes a lightweight inline project composer with:
  - blank title input
  - blank description input
  - primary `Create project` action
- the full project-directory data currently shown on `/projects` moves onto `/`
- `/projects` stops being a distinct destination and becomes a compatibility
  redirect or alias to `/`
- the shell stops presenting `Home` and `Projects` as separate top-level
  destinations

This intentionally revises the retained-topology decision from Phase 32. The
user has now explicitly chosen consolidation over preserving two project-entry
surfaces.

## 2. User Outcome

After this phase:

- the home route feels like the real start of work, not a teaser for the page
  that actually matters
- a user can land on `/`, type a project title and description immediately, and
  create a project without first visiting `/projects/new`
- the project list, statuses, next actions, and delete controls that currently
  exist on `/projects` are visible directly on `/`
- the shell no longer presents redundant `Home` and `Projects` navigation
- bookmarked or manually visited `/projects` URLs still land on the same shared
  surface through a redirect or alias instead of breaking
- Builder-driven design work against the frontend mock runtime edits the same
  real route surface that the server-backed app will later serve

## 3. Problem

The current split weakens the route story in four ways.

### 3.1 The most useful project information is one click too far away

`/` currently shows:

- a single featured project spotlight
- a compact recent-project list capped to a few rows

`/projects` is where the actual operating directory lives:

- the full project list
- truthful project status summaries
- next-action labels
- delete controls

That makes the homepage feel like a staging page for the real work-entry route.

### 3.2 The shell still advertises redundant destinations

The top navigation currently includes both:

- `Home`
- `Projects`

If the real difference between them is only "partial project summary" versus
"full project summary," the navigation is doing product harm rather than adding
clarity.

### 3.3 The current home hero is not the preferred first action

The featured-project spotlight was a reasonable way to reinforce project-first
work, but the preferred top-of-home interaction is now more direct:

- blank title
- blank description
- create project

The page should start by inviting new work, then show the directory of existing
work underneath.

### 3.4 Phase 32 preserved a distinction the product no longer wants

Phase 32 explicitly kept `/` and `/projects` as distinct roles. That decision
was truthful at the time, but the user has now chosen the simpler outcome:

- one canonical project-entry surface
- no redundant top-level project directory peer

This needs a new explicit spec so the repo does not keep treating the old route
split as a locked product truth.

## 4. Scope

### In Scope

- consolidating the current `/` and `/projects` route responsibilities
- redesigning the topmost home panel around an inline project composer
- moving the existing projects-directory data and controls onto `/`
- deciding the shell navigation contract once `Home` and `Projects` are no
  longer distinct destinations
- deciding the compatibility behavior for `/projects`
- targeted route verification and planning sync for the consolidation

### Out Of Scope

- redesigning the project workspace at `/projects/:projectSlug`
- redesigning the session workspace or session queue
- changing the frontend mock runtime contract from Phase 35
- changing backend truth or project/session status semantics from Phase 29
- broad shell redesign beyond the required `Home`/`Projects` consolidation
- removing `/projects/new` unless the implementation can prove it is safely
  replaced by the inline composer without widening scope

## 5. Contracts

### 5.1 Canonical work-entry contract

After this phase, `/` is the canonical project work-entry and project directory
surface.

Required behavior:

- `/` owns the primary new-project entry experience
- `/` owns the full project-directory list and its controls
- `/` may still include concise work-entry framing copy, but it must no longer
  hold back the full directory as a secondary destination

### 5.2 Inline composer contract

The topmost section on `/` must become an inline project composer.

Required UI shape:

- one blank project title field
- one blank project description field
- one primary `Create project` action

Allowed supporting behavior:

- compact instructional copy
- secondary direct-session link if it remains visibly secondary
- inline validation and pending states

Not acceptable:

- a primary CTA that still just links away to `/projects/new`
- keeping the featured-project spotlight as the dominant above-the-fold surface

### 5.3 Directory reuse contract

The project-directory content already implemented on `/projects` should be
reused rather than reimagined from scratch.

Required result:

- the same summary truth, row density, next-action labels, and delete affordance
  currently available on `/projects` become visible on `/`
- the home route may restyle or reframe the container, but it should not throw
  away useful operating-directory behavior just to preserve a distinction the
  product no longer wants

### 5.4 Compatibility route contract

`/projects` must stop being a distinct user-facing destination.

Acceptable bounded outcomes:

- route-level redirect from `/projects` to `/`, or
- thin alias rendering of the same canonical home surface

The implementation must pick one explicit behavior and verify it.

Not acceptable:

- preserving a separate `Projects` page with materially different content
- leaving `/projects` reachable as a duplicate destination while also claiming
  the redundancy is resolved

### 5.5 Shell navigation contract

The shell must stop advertising redundant top-level destinations.

Required behavior:

- keep `Home` as the canonical entry to project work, or rename it if the
  implementation can prove a better single label
- remove the separate top-level `Projects` nav item once `/projects` is an
  alias or redirect

This phase is not complete if the route contract is consolidated but the shell
still presents both `Home` and `Projects`.

## 6. Product Decisions

### 6.1 Consolidate onto `/`

The retained destination is `/`, not `/projects`.

Reason:

- it keeps the shell simple
- it preserves the idea of "home is where work starts"
- it gives Builder editing one obvious landing surface for primary route design

### 6.2 Favor creation-first above the directory

The page should begin with a new-project composer, not with a featured
"continue this project" spotlight.

Reason:

- it better matches the current design preference
- it avoids choosing one existing project as the hero when the directory itself
  is now immediately available underneath

### 6.3 Keep project-first, but make it more literal

This phase does not walk back project-first product direction. It makes it more
literal:

- the home route is the project route
- the first interaction is project creation
- existing project work is visible immediately below

### 6.4 Preserve continuity for existing deep links

The repo should not silently strand users or proof surfaces that still reach for
`/projects`.

So this phase keeps compatibility through redirect or alias behavior instead of
breaking the route outright.

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/app.tsx`
- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/new.tsx`
- shared project-directory helpers if extraction is needed for reuse
- route tests and browser proof covering:
  - home inline project creation
  - project-directory visibility on `/`
  - shell nav without redundant `Projects`
  - `/projects` compatibility redirect or alias behavior

## 8. Acceptance Criteria

This slice is complete only when:

1. `/` shows a topmost inline project composer with blank title and
   description inputs plus a primary create action
2. `/` also shows the full projects-directory data that previously required
   navigating to `/projects`
3. the shell no longer exposes both `Home` and `Projects` as separate primary
   destinations
4. `/projects` no longer behaves as a distinct page and instead truthfully
   redirects or aliases to `/`
5. project summary truth, next-action labels, and delete behavior remain
   intact after the consolidation
6. Builder/frontend-mock browsing still reaches the canonical shared route
   surface without introducing a mock-only home implementation

## 9. Verification Plan

- route-level tests for:
  - inline project composer rendering and submission on `/`
  - full project-directory rendering on `/`
  - shell navigation without a separate `Projects` item
  - `/projects` redirect or alias continuity
- frontend mock browser proof for:
  - landing on `/`
  - creating a project from the inline composer
  - reopening an existing project from the home directory list
  - following an old `/projects` path and landing on the canonical surface
- standard `planner-solid` lint/build verification

## 10. Rollback / Fallback

If the full consolidation proves too broad in one pass:

- keep `/` as the canonical home surface
- land the inline composer and full project-directory content on `/` first
- preserve `/projects` as a thin alias temporarily
- do not keep the old distinct `Projects` shell nav item as a long-lived
  fallback

## 11. Open Questions

None block readiness for this bounded slice.

The main product decision is now closed:

- consolidate home and project directory onto `/`
- make inline project creation the topmost home interaction
- treat `/projects` as compatibility, not as a second real destination

## 12. Implementation Outcome

Implemented on 2026-03-31.

Phase 36 landed as the bounded home/projects consolidation slice:

- `/` now owns the inline project composer and the full projects directory
  list, including delete controls and truthful next-action summaries
- the shell no longer exposes `Projects` as a duplicate primary nav item
- `/projects` now behaves as compatibility-only redirect/alias flow back to `/`
- frontend mock project creation from the top home composer now preserves route
  continuity through a progressive-enhancement path:
  - hydrated browser flow can create directly and navigate into the project
    workspace
  - non-hydrated fallback submits through `/projects/new`
  - the mock provider now bridges the created project into browser-side mock
    state so returning home still shows the new project in the directory

Verification included targeted unit tests for the mock/data layer, frontend
mock Playwright proof for home creation plus `/projects` compatibility
continuity, and standard `planner-solid` lint/build verification.
