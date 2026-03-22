# Planner UI Reset Phase 02 Projects Directory Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner UI Reset Phase 01 Home Hub Spec](/home/thetu/planner/docs/planner-ui-reset-phase-01-home-hub-spec.md), [Planner UI Reset Phase 03 Project Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-03-project-workspace-spec.md), [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)  
**Source Research:** [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx), route map in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), and external research on operational directories, row scanning, data tables, and filter disclosure from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset `/projects` into a true operating directory where each project reads as a
working object with a clear next move.

This route should stop feeling like stacked sections around a list and instead
read as a decisive directory surface:

- project identity first
- state and freshness second
- next action third
- route-level creation and import visible but contained

## User Outcome

After this slice:

- users can scan projects quickly without reading through card-like blocks
- project identity, freshness, and route-forward actions are visible in one
  pass
- import activity is visible without taking over the whole page
- archived state is understandable without muddying the default active list

## Design Research Synthesis

- data-dense route guidance from Carbon and Fluent favors consistent scanning
  structures and discourages competing summary modules around the main list
- Nielsen Norman Group guidance on information hierarchy supports keeping the
  list as the obvious anchor rather than asking users to interpret several peer
  sections before reaching the directory
- Material list and table guidance supports compact row rhythm with clear
  secondary metadata and controlled filtering

Planner implication:

- the route should behave as a directory, not a dashboard
- rows should carry the page
- creation, import, search, and archive controls should frame the directory,
  not compete with it

## Locked Decisions

- `/projects` remains a directory-first route
- row or row-like directory objects remain the dominant unit
- create and import remain route-level actions, not repeated per project card
- the route stays operational and project-first, not editorial or portfolio-
  style
- archived projects stay reachable through an explicit directory mode rather
  than being mixed ambiguously into the default view

## Scope

### In scope

- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- project directory row styles and route hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any small supporting extraction required for a clearer row model or import
  status framing

### Out of scope

- backend project data-model changes
- import pipeline redesign beyond directory framing
- project-local workspace redesign covered by `UIR-03`

## Current-State Evidence

- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  already loads and sorts projects by `updated_at`, which supports an
  operational directory posture
- the route also owns search, archived filtering, create-project, import flow,
  and polling of `latestImport`, which means route-level framing matters more
  than on a plain list page
- the current implementation can present project rows alongside route controls
  and import-state feedback, but the page still risks reading as several block
  sections rather than one directory
- import status is important, but it should not become a co-equal persistent
  module when the user's real task is usually "find the right project and open
  it"

## Proposed UI Model

## Route role

`/projects` is the operating directory for all projects in the current
workspace.

It is not a portfolio dashboard and not a project home substitute.

## Dominant surface

The project list must dominate the route.

Each row should make these facts legible in one scan line:

- project identity:
  name, slug or short descriptor, and whether the project is archived
- freshness:
  last updated time or relative activity cue
- current posture:
  whether the project looks active, quiet, or import-related
- next move:
  open project workspace as the main forward action

## Route framing

The top framing should be compact and practical:

- search
- archived toggle or mode switch
- create project
- import project

If import work is active, present it as a compact route-level status band or
attached progress notice, not a peer content block that pushes the directory
down the page.

## Supporting surfaces

Supporting surfaces should stay minimal:

- a compact route summary if needed
- a compact import-progress band when an import is live
- empty-state guidance when there are no projects or no matches

No summary-card mosaic should sit above the directory.

## Reveal model

- richer project metadata should appear through row expansion, attached detail,
  or secondary text inside the row, not through standalone cards
- archived-only context should appear when the archived mode is active
- import conflict explanation may remain modal because it is an exceptional flow

## State model

The route should explicitly handle:

- populated active directory
- filtered directory with query matches
- no-match filtered directory
- archived mode
- import in progress
- import conflict redirect
- empty workspace
- load failure

## Design-System-Patterns Lens

- semantic surfaces:
  one primary directory surface, one secondary control band, one conditional
  import status surface
- component-state modeling:
  rows must preserve hierarchy across normal, hover, archived, and loading
  states
- theming discipline:
  use existing Planner tonal layering; avoid generic data-table chrome or
  spreadsheet borders

## Contracts And Touched Surfaces

- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  remains the route owner
- existing APIs remain unchanged:
  `listProjects`, `createProject`, `createProjectImport`, and
  `getProjectImport`
- navigation contract remains:
  opening a project routes to `/projects/:slug/sessions`
- touched surfaces:
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- `/projects` reads as a directory first and a control surface second
- the project list is visually dominant over all route-level framing
- each project row makes identity, freshness, and next move clear in one pass
- import activity is visible without displacing the directory as the primary
  route object
- archived mode is explicit and does not muddy the default active-project view

## Verification Plan

- targeted frontend tests for
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  covering:
  - empty state
  - filtered results
  - archived mode
  - import in progress
  - import conflict redirect behavior
- `npx tsc --noEmit`
- manual verification with sparse and heavily populated project sets

## Rollback And Fallback

- if a denser row model becomes too compressed, reduce secondary metadata before
  restoring card-like blocks
- if active import visibility needs stronger emphasis, use a compact sticky
  status band before reintroducing a large summary module

## Implementation Notes

Implemented on 2026-03-22 with these bounded outcomes:

- the top route framing now behaves as a compact directory control band instead
  of a split summary layout
- visible and archived counts stay present without competing with the list
- per-row route-launch buttons were removed so each project row has one obvious
  primary next move: open the project workspace
- import visibility remains route-level and attached rather than card-like

Verification executed:

- `npm test -- src/pages/__tests__/ProjectsPage.test.tsx`
- `npx tsc --noEmit`

## Open Questions

None blocking readiness.
