# Planner UI Reset Phase 08 Discovery Review Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner UI Reset Phase 06 Knowledge Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md), [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)  
**Source Research:** [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx) and external research on review consoles, moderation queues, and dense triage interfaces from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset Discovery into a proposal-triage workspace where review objects dominate
and route-level controls support that work instead of diluting it.

Discovery should feel like a review desk, not a loose utility page.

## User Outcome

After this slice:

- proposals are easier to scan, compare, and act on
- pending review work is immediately obvious
- related knowledge stays useful without crowding triage
- scan or refresh actions remain available without feeling like the page's main
  purpose

## Design Research Synthesis

- review-console research favors one dominant triage surface with attached
  detail rather than several equal panels
- visibility-of-status guidance supports surfacing pending and already-reviewed
  state directly on the review object
- disclosure guidance supports expandable or attached detail when the primary
  task is deciding on one item at a time

Planner implication:

- the proposal object is the main unit of the page
- status filters and scan actions should be quiet framing
- related context should help decisions without taking over the route

## Locked Decisions

- Discovery remains a review console
- node and edge proposal views both remain supported
- proposal triage remains the route's main job
- backend discovery semantics are unchanged
- this slice does not move Discovery into project-local routing

## Scope

### In scope

- [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
- proposal object layout, triage hierarchy, and attached detail behavior
- route hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

### Out of scope

- discovery scan backend changes
- knowledge route redesign
- blueprint schema or graph redesign

## Current-State Evidence

- [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  already maintains meaningful review semantics:
  `pending`, `accepted`, `rejected`, and `merged`
- the route also supports node-versus-edge proposal modes, a scan action,
  expansion state, and component-name overrides
- this already behaves like a real triage console, but the product model needs
  to make proposal review more dominant than route utility controls

## Proposed UI Model

## Route role

Discovery is the proposal review desk for inferred or suggested structural
changes.

Its main question is:
"what should be accepted, rejected, or left pending?"

## Dominant surface

The dominant surface should be a triage list or review table hybrid where each
proposal object exposes:

- proposal identity
- source
- current review status
- relevant suggested change
- direct review action

Pending items should visually outrank already-reviewed items.

## Supporting surfaces

Supporting surfaces should be compact:

- a quiet control bar for proposal mode and status filter
- a compact scan or refresh action area
- attached knowledge context for whichever proposal is open or expanded

## Reveal model

- detailed related-knowledge context should appear through expansion, attached
  side detail, or lower contextual region
- accepted, rejected, and merged proposals may remain visible, but they should
  not crowd pending review by default
- proposal-name override or editing affordances should remain attached to the
  relevant proposal object

## State model

The route should explicitly support:

- pending proposals present
- no pending proposals but reviewed items exist
- node proposal mode
- edge proposal mode
- scan in progress
- action mutation in progress
- empty state
- load failure

## Design-System-Patterns Lens

- semantic surfaces:
  one primary triage surface, one secondary control band, one attached detail
  surface
- component-state modeling:
  pending, accepted, rejected, merged, and scanning states must remain easy to
  distinguish
- theming discipline:
  no analytics-dashboard detour and no decorative reviewer chrome

## Contracts And Touched Surfaces

- [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  remains the route owner
- existing discovery APIs remain unchanged:
  listing proposals, running scan, and accept or reject actions
- knowledge deep-link behavior remains unchanged
- touched surfaces:
  [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the route reads as a proposal-review workspace rather than a utility page
- proposal objects dominate the page
- pending review work is easier to identify than already-reviewed items
- related context is accessible without competing with triage
- node and edge proposal modes preserve the same hierarchy

## Verification Plan

- targeted frontend tests for
  [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  covering:
  - pending proposals
  - no proposals
  - scan in progress
  - action mutation state
  - node and edge modes
- `npx tsc --noEmit`
- manual verification with mixed proposal statuses and expanded context

## Rollback And Fallback

- if a richer review object is too dense for the first pass, preserve the
  triage-first hierarchy and defer some related-context richness
- if reviewed items need stronger separation, collapse them into a secondary
  section before restoring a flatter list

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the bounded triage-hierarchy reset in
  [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  by grouping proposals into `Pending review` and `Reviewed` sections whenever
  the route is showing all statuses, so undecided work visually outranks
  already-reviewed items.
- Preserved the existing accept, reject, rename, edge-review, and knowledge
  deep-link flows while making the proposal object and review state more
  explicit than the surrounding utility controls.
- Verification completed with:
  `npm test -- src/pages/__tests__/DiscoveryPage.test.tsx`
  and `npx tsc --noEmit`.
- Verification was refreshed in the tranche audit remediation slice to cover
  grouped pending-versus-reviewed hierarchy, empty-state behavior, and scan
  mutation visibility in the same route test file.
