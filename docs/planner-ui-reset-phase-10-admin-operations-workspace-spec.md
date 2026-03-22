# Planner UI Reset Phase 10 Admin Operations Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md), [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)  
**Source Research:** [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx) and external research on operations consoles, health summaries, and dense event review from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset Admin into a bounded operations workspace with one dominant health or
command surface and quieter supporting operational streams.

This route should look like a tool for operating the system, not a collage of
status widgets.

## User Outcome

After this slice:

- users can tell where to look first on the Admin page
- overall health and immediate operational posture are easier to parse
- logs and event streams remain dense but better subordinated
- the route stays serious and practical without drifting into security-dashboard
  theater

## Design Research Synthesis

- operations-console guidance favors a dominant health summary or control band
  with secondary streams attached below or beside it
- visibility-of-status guidance supports making the current system posture
  obvious before exposing lower-level detail
- dense-interface guidance supports keeping logs readable through rhythm and
  hierarchy rather than by adding more boxes and dashboards

Planner implication:

- one health or action band should anchor the route
- event and log detail should support that anchor
- density is good, but equal-weight block competition is not

## Locked Decisions

- Admin remains a dense operational route
- one dominant health or action surface should anchor the page
- event streams and detailed logs remain supportive, not primary
- no decorative infrastructure drama, neon warning walls, or fake SOC styling
- backend admin semantics and permissions remain unchanged

## Scope

### In scope

- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
- admin route hierarchy styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any small extracted admin display structure needed for stronger hierarchy

### Out of scope

- backend admin semantics
- auth permission changes
- events route redesign beyond coordination with Admin

## Current-State Evidence

- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  already contains meaningful operational primitives:
  service status, uptime, level filtering, source filtering, and recent event
  entries
- the route therefore already has truthful operational content, but it risks
  feeling like several status blocks and stream fragments without a strong top
  anchor

## Proposed UI Model

## Route role

Admin is the bounded operations console for Planner runtime status and recent
system activity.

Its main questions are:

- is the system healthy enough
- what needs operational attention
- what changed recently

## Dominant surface

The dominant surface should be one operating summary band or health desk that
makes these points immediately visible:

- current service posture
- uptime or freshness
- whether anything appears degraded or noisy
- the most important immediate operator interpretation

## Supporting surfaces

Supporting surfaces should include:

- one compact filter bar for level or source narrowing
- one or two dense operational streams below the summary
- any supporting links into related context, such as Knowledge, as tertiary
  affordances

Supporting streams should remain dense and readable, but they should not read
as peer hero modules.

## Reveal model

- lower-priority operational detail can sit behind explicit reveals or compact
  subsections if the route becomes too busy
- per-event deep detail should appear through expansion or attached context, not
  through permanently oversized event blocks

## State model

The route should explicitly support:

- healthy system
- degraded or warning-heavy system
- filtered event stream
- no recent events
- loading and failure states

## Design-System-Patterns Lens

- semantic surfaces:
  one primary health desk, one secondary event stream, one tertiary filter band
- component-state modeling:
  info, warning, and error conditions must remain distinguishable without
  overpowering the route
- theming discipline:
  keep Planner's restrained command-center tone and avoid generic infra cards

## Contracts And Touched Surfaces

- [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  remains the route owner
- existing admin APIs remain unchanged
- knowledge deep-link behavior remains unchanged where used
- touched surfaces:
  [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- `/admin` has one obvious dominant operating surface
- logs and events support that surface instead of competing equally with it
- dense data remains readable without dashboard clutter
- warning and error conditions are clearer without visual melodrama

## Verification Plan

- targeted frontend tests for
  [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  covering:
  - healthy status
  - warning or degraded status
  - filtered stream
  - empty events
  - load failure
- `npx tsc --noEmit`
- manual verification with populated real or fixture-backed operational data

## Rollback And Fallback

- if a stronger health summary is not fully stable in the first pass, preserve a
  compact top status band and demote the rest of the route before restoring
  several equal blocks
- if the stream requires more room, let the health desk stay concise instead of
  inflating multiple summary modules

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the bounded admin hierarchy reset in
  [AdminPage.tsx](/home/thetu/planner/planner-web/src/pages/AdminPage.tsx)
  by adding an explicit operator posture summary inside the primary health desk,
  so the route now states whether runtime is healthy, warning-heavy, or needs
  attention instead of forcing the user to infer that from several separate
  metrics.
- Preserved the existing runtime status, provider availability, and event-log
  surfaces while making the top health desk the clear first reading target.
- Verification completed with:
  `npm test -- src/pages/__tests__/AdminPage.test.tsx`
  and `npx tsc --noEmit`.
- Verification was refreshed in the tranche audit remediation slice with
  route-specific assertions for operator posture summaries, filtered or empty
  event streams, and load-failure handling.
