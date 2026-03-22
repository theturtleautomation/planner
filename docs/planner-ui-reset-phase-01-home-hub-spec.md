# Planner UI Reset Phase 01 Home Hub Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Phase 01 Root Landing And Navigation Implementation](/home/thetu/planner/docs/phase-01-root-landing-implementation.md), [Planner UI Reset Phase 00 Shell Navigation And Auth Spec](/home/thetu/planner/docs/planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md), [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)  
**Source Research:** [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx), route inventory in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), and external research on first-use hierarchy, recognizability, and explicit wayfinding from Nielsen Norman Group, Apple, Fluent, Carbon, and Material

## Objective

Reset Home into a quiet project-entry surface that answers one product question
immediately: "what should I open or start next?"

Home should stop behaving like a route switchboard.

It should remain useful for first-run orientation and repeat visits, but the
main impression should be:

- Planner starts from projects
- there is one obvious next move
- secondary utilities exist without competing for the same visual rank

## User Outcome

After this slice:

- a user can tell where to begin within a few seconds
- the dominant action is project-oriented rather than route-oriented
- recent work is easy to resume without scanning a grid of peer actions
- utility destinations stay reachable but visually defer to project launch
- Home feels like a briefing-and-launch surface, not a mini dashboard

## Design Research Synthesis

The research direction for this route is consistent:

- Nielsen Norman Group's recognition-over-recall guidance supports showing the
  most likely next move and recent work directly instead of asking the user to
  remember route names
- Apple discoverability guidance supports making the main action immediately
  visible while leaving secondary actions obviously available, not hidden
- Fluent layout guidance favors a small number of clear regions with strong
  hierarchy instead of several equally strong modules
- Carbon and Material guidance both support progressive disclosure for lower-
  frequency actions rather than presenting all destinations as peers

Planner implication:

- Home should elevate one project-launch surface
- recent projects should be the main support surface
- utility routes should move into a smaller explicit cluster, not a peer grid

## Locked Decisions

- Home remains project-first and does not become the canonical global work queue
- the route is a launch-and-briefing surface, not a reporting surface
- direct route jumps may remain, but they must not dominate the page
- Home should not become semantic search or command-palette product work
- project creation stays available from Home
- local dev mode remains visible, but only as supporting route context

## Scope

### In scope

- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
- Home-specific CSS or shared route hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any small supporting UI extraction needed to give Home a clearer focal model

### Out of scope

- shell-wide nav changes already covered by `UIR-00`
- projects directory redesign covered by `UIR-02`
- sessions queue redesign covered by `UIR-04`
- backend routing, project creation semantics, or search APIs

## Current-State Evidence

- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  currently combines a prompt input, a large quick-action matrix, operating
  stats, and recent projects near the top of the page
- the `quickActions` array includes `Projects`, `Knowledge Library`, `Events`,
  `Admin`, `Sessions`, `Blueprint`, and `Discovery`, which makes the route read
  like a route launcher instead of a product entry surface
- the right-hand "Operating picture" panel is informative, but it competes with
  the left launch surface rather than supporting it
- the prompt field is useful, but it currently shares top billing with too many
  direct buttons
- recent projects are valuable repeat-visit context, but they currently arrive
  after a fairly loud control cluster

## Proposed UI Model

## Route role

Home is the product briefing and launch page.

Its job is not to expose the whole application.
Its job is to get the user into the right project or the right project-adjacent
surface with minimal hesitation.

## Dominant surface

The dominant surface should be a single launch deck above the fold.

That deck should combine:

- one strong primary action for starting or creating project work
- a deterministic route prompt for users who already know what they want
- one concise statement of the user's current workspace situation

The deck should read as one object, not several stacked peer modules.

## Supporting surfaces

Home should expose two supporting surfaces only:

- `Recent Projects`
  this is the main repeat-visit support surface and should be clearly secondary
  to the launch deck but visually stronger than utilities
- `Utilities`
  a compact, explicitly labeled cluster for routes such as Knowledge, Events,
  Blueprint, Discovery, Admin, and Sessions

Utilities should no longer appear as a large equal-weight button matrix.

## Reveal model

- lower-frequency utilities should move into a quieter row, compact list, or
  explicit reveal cluster
- Home should not require drawers or modals for ordinary navigation, but it
  should stop using big persistent action grids for low-frequency routes
- if the prompt remains visible, its suggestion language should reinforce the
  project-first posture instead of generic app jumping

## State model

Home should explicitly cover these visible states:

- empty workspace:
  no projects yet, creation is primary, utilities are present but clearly
  secondary
- active workspace:
  recent projects and latest activity appear, with one clear launch path
- loading:
  the launch deck remains stable while recent work and counts resolve
- error:
  project loading failure appears inside the supporting content region, not as
  a full route collapse
- dev mode:
  visible as a compact environment cue, not as the main story of the page

## Design-System-Patterns Lens

- semantic surfaces:
  one primary launch surface, one secondary recent-work surface, one dormant
  utility cluster
- component-state modeling:
  empty, loaded, loading, and error states must preserve the same hierarchy
- theming discipline:
  no generic dashboard-card mosaic, no new ornamental stats styling, no route-
  level glass theater

## Contracts And Touched Surfaces

- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  remains the route owner
- project loading still depends on `listProjects`
- project creation still depends on existing create-project flow
- the intent prompt remains deterministic and route-based; this slice does not
  introduce natural-language interpretation beyond current behavior
- touched frontend surfaces:
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the upper portion of `/` has one obvious dominant launch object
- recent projects are clearly the primary supporting content
- utility destinations remain accessible without reading as peer CTAs
- the route feels project-first rather than app-directory-first
- empty, loading, and loaded states preserve the same hierarchy

## Verification Plan

- targeted frontend tests for
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  covering empty, populated, and error states
- `npx tsc --noEmit`
- manual verification for:
  - authenticated mode with several active projects
  - first-run or empty project state
  - narrow widths where supporting surfaces stack
  - dev mode where the environment cue is visible but quiet

## Rollback And Fallback

- if a fully reduced utility cluster proves too aggressive, keep one or two
  direct utility actions visible and demote the rest
- if the prompt and create/open actions do not coexist cleanly, preserve the
  primary project action and make the prompt subordinate before restoring a
  larger control surface

## Implementation Notes

Implemented on 2026-03-22 with these bounded outcomes:

- Home now centers one launch deck instead of a split hero plus equal-weight
  quick-action matrix
- the latest project becomes the primary resume path when project data exists
- utilities now live in a compact supporting strip instead of competing as peer
  launch actions
- recent projects remain the dominant secondary surface and now render before
  the quieter utilities section in both populated and empty route states

Verification executed:

- `npm test -- src/pages/__tests__/HomeHubPage.test.tsx`
- `npx tsc --noEmit`

Verification was refreshed in the tranche audit remediation slice to include
empty and error-state route assertions in the same frontend test file.
Residual tranche-correction follow-up work then added direct assertions that the
`Recent Projects` section stays ahead of `Utilities` in the rendered route
hierarchy.

## Open Questions

None blocking readiness.
