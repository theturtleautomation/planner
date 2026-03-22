# Planner Design System Phase 5 Route Hierarchy And Operational Density Spec

**Status:** Implemented and verified on 2026-03-22  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md)  
**Source Research:** Stitch-to-Planner design translation report dated 2026-03-22

## Objective

Translate the strongest transferable layout and hierarchy patterns from the
Stitch archive into Planner's highest-traffic planning routes without copying
the source layouts, wording, or product theater.

This slice is about composition and density on the core working routes:

- stronger dominant-module hierarchy
- fewer equal-weight dashboard blocks
- richer operational rows for projects and sessions
- clearer route-level next actions

## User Outcome

After this slice:

- Home reads as a command surface with one obvious primary action area instead
  of a prompt plus loosely related support blocks
- Projects and sessions become easier to scan because rows expose status,
  freshness, and next action with less visual noise
- project-level session work feels denser and more directed without becoming
  border-heavy or theatrical

## In Scope

- route-level composition updates in
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx),
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx),
  and
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- shared hierarchy and active-state token support in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
  and
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- row, summary, and action treatment improvements for project and session
  surfaces where shared classes can carry the behavior

## Out Of Scope

- route or information-architecture changes
- hover-expanding navigation blades or icon-only default shell behavior
- literal reuse of the Stitch archive's dashboard-card mosaics
- Knowledge and Blueprint route redesign, which are scoped in later follow-on
  specs
- backend capability changes, sorting logic changes, or product-behavior
  changes

## Current-State Summary

Phases 1 through 4 established the command-center tonal system, editorial
headings, overlay restraint, and utility-route parity. The remaining gap on the
main planning routes is compositional:

- Home still leads with a contained prompt block and follow-on sections that
  are calmer than before but still relatively equal in weight
- Projects and sessions expose truthful data, but the scanning rhythm remains
  more utilitarian than directed
- row and card patterns still underuse module scale, state emphasis, and
  action clarity compared to the transferable parts of the Stitch analysis

## Proposed Behavior

### Shared route hierarchy

- each in-scope route should have one dominant working surface above the fold
- supporting modules should read as secondary context, not equal-weight peers
- page-level action clusters should collapse toward one primary action and a
  small set of quieter secondaries

### Home route

- preserve the existing project-first prompt intent, but treat the prompt and
  quick routing controls as the primary composition anchor
- add one stronger operational summary module near the prompt instead of
  multiple same-weight support blocks
- recent projects should read as a dense operational directory, not a gallery

### Projects route

- move toward a directory-first surface with denser project rows or table-hybrid
  objects
- each visible project entry should make status, freshness, and next move easy
  to scan in one line of sight
- CTA placement should favor project creation and import as route-level
  actions, not repeated inline chrome

### Sessions routes

- the global `/sessions` queue should use richer action-oriented rows with
  stronger state emphasis, not stat-card-first framing
- the project-local sessions surface should better distinguish:
  - active sessions
  - resumable or blocked sessions
  - import review or history work
  - secondary project navigation
- badges and phase markers should become clearer, but remain tonal and calm

### Shell polish inside this slice

- shared nav and page-header active states may be tightened where token-level
  work supports the new route hierarchy
- the shell must remain labeled, predictable, and project-first

## Implementation Constraints

- keep the shell structurally stable; no hover-to-reveal navigation
- keep the current command-center palette direction; no neon cyan promotion
- do not reintroduce structural borders as the default grouping device
- do not invent fake KPI modules that do not reflect real Planner product truth
- preserve mobile viability and keyboard focus clarity

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [Dashboard.tsx](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)

Expected supporting files, only if needed:

- shared row or summary components referenced by the in-scope pages

## Acceptance Criteria

- Home has one clearly dominant working module above the fold
- Projects no longer reads as a sequence of similarly weighted blocks when the
  directory is populated
- `/sessions` prioritizes actionable work through row density and hierarchy
  rather than route-level summary cards
- project-local session management more clearly separates active work from
  import review and history context
- the shell remains calmer and more restrained than the Stitch source even as
  hierarchy becomes stronger

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - [HomeHubPage](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  - [ProjectsPage](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  - [Dashboard](/home/thetu/planner/planner-web/src/pages/Dashboard.tsx)
  - [ProjectSessionsPage](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- run `npx tsc --noEmit`

### Manual

- verify Home, Projects, `/sessions`, and one populated project-local sessions
  route in both themes
- verify the first screen on each route exposes one obvious primary module
- verify denser rows remain readable on narrower desktop widths and mobile
- verify active and selected states read clearly without glow-heavy effects

## Rollback And Fallback

- if one route becomes visually ambiguous, keep the new shared hierarchy tokens
  and localize the rollback to that page
- if denser rows reduce readability, reduce metadata density before restoring
  border-heavy card separation
- if one project-local import section cannot adopt the new rhythm cleanly,
  defer that subsection and keep the main route composition work

## Open Questions

None blocking readiness.

The route targets, compositional problems, and non-goals are concrete enough to
support bounded implementation.
