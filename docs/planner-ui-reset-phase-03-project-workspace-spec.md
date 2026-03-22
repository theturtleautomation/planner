# Planner UI Reset Phase 03 Project Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md), [Planner UI Reset Phase 02 Projects Directory Spec](/home/thetu/planner/docs/planner-ui-reset-phase-02-projects-directory-spec.md), [Planner UI Reset Phase 04 Sessions Queue Spec](/home/thetu/planner/docs/planner-ui-reset-phase-04-sessions-queue-spec.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)  
**Source Research:** [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx), project route inventory in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), and external research on staged workspaces, review consoles, and progressive disclosure from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset the project-local sessions route into a staged workspace that makes one
project truth dominant at a time.

Today the route has to carry:

- project identity
- active sessions
- import progress
- import review
- import history
- historical comparisons

All of those are valid, but they should not compete as equal-weight modules.

## User Outcome

After this slice:

- users can tell what matters in the current project immediately
- active planning work and import review no longer compete ambiguously
- import history and comparison remain accessible without crowding primary work
- the route explains whether the project is in active planning, import review,
  import processing, or quiet maintenance mode

## Design Research Synthesis

- research on staged workspaces supports one dominant task region with clearly
  subordinate reference regions
- review-console guidance supports keeping pending review work highly visible
  while moving historical and comparative context into explicit reveals
- disclosure guidance supports attached history and comparison surfaces rather
  than always-open multi-module pages

Planner implication:

- the route should promote the "work now" truth over the "everything about this
  project" truth
- import review can dominate when it exists
- history and comparisons should remain attached, not co-equal

## Locked Decisions

- `/projects/:projectSlug/sessions` remains the canonical project workspace
- import review and import history remain attached to this route
- project-local tab navigation to Blueprint, Knowledge, and Events remains
- backend import semantics are unchanged
- this slice does not redesign the session detail page itself

## Scope

### In scope

- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- route-local session list, import review framing, import history framing, and
  comparison reveal patterns
- project workspace hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

### Out of scope

- backend import engine changes
- project tab-route architecture changes
- detailed Socratic session page changes beyond alignment with this workspace

## Current-State Evidence

- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  loads project data, sessions, import state, import review, import history,
  and comparison state together, which makes the route semantically rich but
  visually easy to overload
- the route also owns selection toggles, apply-review actions, restore actions,
  and pair comparison loading, which means the page is already functioning as a
  workflow console rather than a simple list
- project-local tabs are useful context, but they do not solve the question of
  which content region should dominate inside the route

## Proposed UI Model

## Route role

This route is the project workspace dispatch page.

Its job is to answer:

- what is happening in this project now
- what needs action now
- what reference material can be consulted if needed

## Dominant surface selection

The dominant surface should change based on project truth:

- `Import review waiting`
  the dominant surface becomes the import review desk
- `Import processing`
  the dominant surface becomes a compact import progress desk
- `Normal project work`
  the dominant surface becomes active and resumable sessions
- `Quiet project`
  the dominant surface becomes a project workspace empty state with clear next
  actions

Only one of those should be visually dominant at a time.

## Supporting surfaces

Supporting surfaces should include:

- project identity and local nav as compact top framing
- recent sessions or session history as a secondary region
- import history and comparison as explicitly labeled secondary or revealed
  content

Historical comparison should not sit at equal weight with the current task
surface.

## Reveal model

- import history comparison should live in a disclosed panel, attached tray, or
  lower reveal section
- pair comparison should only appear when requested
- secondary history should remain easy to reach without permanently occupying
  prime route real estate

## State model

The workspace must explicitly support:

- sessions available
- no sessions yet
- import queued or analyzing
- import review pending
- import review selection mutation
- import applied
- history comparison open
- restore pending
- route load failure

## Design-System-Patterns Lens

- semantic surfaces:
  one primary project-task surface, one secondary history surface, one
  conditional comparison surface
- reveal discipline:
  history and comparison are attached reveals, not default co-equal modules
- component-state modeling:
  current-task state drives the primary composition of the page

## Contracts And Touched Surfaces

- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  remains the route owner
- existing APIs remain unchanged:
  `getProject`, `listProjectSessions`, `getProjectImportState`,
  `getProjectImportReview`, `getProjectImportHistory`,
  `applyProjectImportReview`, comparison and restore endpoints
- project-local navigation contract remains unchanged
- touched surfaces:
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the route has one obvious dominant work surface based on current project truth
- import review no longer reads as just another block when it requires action
- import history and comparisons remain accessible without competing with active
  work
- the user can tell what deserves attention now within a few seconds

## Verification Plan

- targeted frontend tests for
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  covering:
  - active sessions state
  - import in progress
  - import review pending
  - history comparison open
  - quiet project state
- `npx tsc --noEmit`
- manual verification across projects with and without import history

## Rollback And Fallback

- if a fully dynamic primary-surface model is too large for the first pass,
  prioritize making import review visually dominant when present and move
  history downward or behind reveal first
- if comparison reveal mechanics slip, preserve hierarchy by demoting
  comparison below the primary workspace before restoring flatter stacking

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the first bounded workspace reset in
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  by making the hero band state-driven:
  import review, import failure, and in-flight import work now promote the
  import surface to the stronger semantic layer, while normal project work
  keeps sessions as the dominant route truth.
- Preserved the existing project-local import review, history, and comparison
  controls below the hero band so the route keeps one obvious focal surface
  without dropping import governance or historical recovery tools.
- Verification completed with:
  `npm test -- src/pages/__tests__/ProjectSessionsPage.test.tsx`
  and `npx tsc --noEmit`.
