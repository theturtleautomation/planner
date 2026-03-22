# Planner Design System Phase 6 Operational Surfaces And Event Density Spec

**Status:** Implemented and verified on 2026-03-22  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)  
**Source Research:** Stitch-to-Planner design translation report dated 2026-03-22

## Objective

Improve Planner's densest operational surfaces by translating the Stitch
archive's strongest row, status, and secondary-pane patterns into real Planner
observability and review routes.

This slice exists to make dense review work feel deliberate without turning the
app into a themed security dashboard.

## User Outcome

After this slice:

- Admin and Events feel like serious operating surfaces rather than generic
  utility pages
- event streams, proposal review, and import history become faster to scan
- dense operational data gains stronger grouping, status emphasis, and clearer
  next actions without excessive borders or visual noise

## In Scope

- route-level density and hierarchy work in
  [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx),
  [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx),
  [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx),
  and the import review and history regions of
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- shared dense-event and dense-row refinement in
  [EventLogPanel.tsx](/home/thetu/planner/planner-web/src/components/EventLogPanel.tsx),
  [SessionEventsTable.tsx](/home/thetu/planner/planner-web/src/components/SessionEventsTable.tsx),
  and if needed
  [SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx)
- token and class support in
  [index.css](/home/thetu/planner/planner-web/src/index.css)

## Out Of Scope

- backend event semantics, admin product behavior, or discovery workflow changes
- adding a literal calendar-heavy events product if current Planner truth does
  not support it
- importing the Stitch archive's compliance or security-product language
- Knowledge and Blueprint redesign, which are covered by separate follow-on
  specs
- broad modal or overlay restyling beyond incidental inheritance

## Current-State Summary

Phase 4 aligned the utility routes with the command-center surface language,
but the densest operational surfaces still need a stronger point of view:

- Admin truth is strong, but the visual rhythm remains closer to generic status
  cards plus an event feed
- Events supports real timeline and snapshot work, but the route lacks a
  stronger primary-secondary composition
- Discovery and import review expose real decision surfaces, but the review rows
  can still feel generic and under-signaled
- shared event surfaces still read more like boxed viewers than tuned
  operational consoles

## Proposed Behavior

### Admin route

- make one system-health band or command summary the top anchor
- move supporting operational streams and status blocks into clearer secondary
  zones
- keep metadata compact and truthful; no executive-dashboard theater

### Events route

- strengthen the separation between the main event stream and the snapshot or
  secondary context pane
- event items should gain better severity, recency, and object identity
  hierarchy
- do not clone the Stitch calendar literally unless the existing product truth
  justifies it

### Discovery and import review

- proposal and import-history rows should become richer review objects with
  clearer status, confidence, scope, and next action
- the review surface should favor dense list or table-hybrid scanning over
  large isolated cards

### Shared dense event surfaces

- `EventLogPanel` should feel like a tight operational stream rather than a
  boxed drawer inside the page
- `SessionEventsTable` should preserve table truth, but gain clearer status,
  grouping, and expandable detail rhythm
- transient counts, warnings, and unread states should remain obvious without
  glowing or neon-heavy treatment

## Implementation Constraints

- stay within Planner's current calmer command-center tone
- no fake cyber, security, or infrastructure branding language
- preserve high information density; do not turn dense routes into oversized
  marketing cards
- if a visible rule is required in a table or stream, use a ghost-border or
  low-contrast separator rather than a hard dashboard frame
- preserve accessibility and filter/control legibility in both themes

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
- [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
- [DiscoveryPage.tsx](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [EventLogPanel.tsx](/home/thetu/planner/planner-web/src/components/EventLogPanel.tsx)
- [SessionEventsTable.tsx](/home/thetu/planner/planner-web/src/components/SessionEventsTable.tsx)

Expected supporting files, only if needed:

- [SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx)

## Acceptance Criteria

- `/admin` has one clearly dominant operating summary instead of multiple
  equally weighted blocks
- `/events` more clearly separates primary timeline work from secondary context
- Discovery and project import review rows expose better decision density and
  next actions
- `EventLogPanel` and `SessionEventsTable` feel aligned with the rest of the
  command-center system
- the resulting surfaces are denser and more authoritative, not more theatrical

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - [AdminPage](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  - [EventTimelinePage](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  - [DiscoveryPage](/home/thetu/planner/planner-web/src/pages/DiscoveryPage.tsx)
  - [ProjectSessionsPage](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  - shared event panel or session event table tests where structure changes
    materially
- run `npx tsc --noEmit`

### Manual

- verify populated Admin, Events, Discovery, and project import-history states
  in both themes
- verify dense rows remain readable and keyboard reachable
- verify severity and next-action cues are visible without neon styling
- verify event and proposal surfaces still feel product-truthful, not
  security-product cosplay

## Rollback And Fallback

- if one route becomes too dense, reduce secondary metadata first rather than
  reverting the broader hierarchy shift
- if a table surface needs stronger separation, add a ghost-border or muted row
  divider locally
- if one route needs more product work than styling alone can support, keep the
  shared operational-surface improvements and defer the blocked route

## Open Questions

None blocking readiness.

The route targets and visual constraints are concrete enough to support bounded
frontend implementation.
