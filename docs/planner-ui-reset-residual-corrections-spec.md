# Planner UI Reset Residual Corrections Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md), [Planner UI Reset Tranche Audit Remediation Spec](/home/thetu/planner/docs/planner-ui-reset-tranche-audit-remediation-spec.md), [Planner UI Reset Phase 01 Home Hub Spec](/home/thetu/planner/docs/planner-ui-reset-phase-01-home-hub-spec.md), [Planner UI Reset Phase 07 Blueprint Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md), [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md)  
**Source Audit:** 2026-03-22 follow-up audit against current tranche specs, implementation, and route tests

## Objective

Close the remaining trust and alignment gaps in the Planner UI reset tranche so
the route queue, tracker, and child specs can be treated as fully reliable
planning artifacts.

This is a bounded correction pass, not a second redesign tranche.

It exists to resolve the specific residual gaps left after the first audit
remediation pass:

- the parent queue doc is synchronized, but still weaker than the child specs
  as a closure artifact
- the project tracker still uses one blanket verification claim that is broader
  than the current route-specific evidence
- Home still has one visible hierarchy drift between spec intent and rendered
  supporting-surface order
- Blueprint and Events still have narrower automated evidence than their doc
  language implies

## Audit Baseline

The follow-up audit found:

- 12 audited artifacts total:
  the queue container plus 11 route specs
- 8 artifacts currently trustworthy
- 4 artifacts with residual drift
- 0 artifacts currently severe enough to reopen outright
- named route verification green at audit time:
  `126/126` frontend tests passed across the named tranche files and
  `npx tsc --noEmit` passed

Residual correction targets from that audit:

- `QUEUE` still needs stronger closeout structure
- `UIR-01` still has a Home hierarchy mismatch
- `UIR-07` still has moderate rather than strong verification evidence
- `UIR-09` still lacks direct automated evidence for the failure state named in
  the spec
- `project-plan.md` still over-compresses tranche truth into one blanket
  "implemented and verified" statement

## Scope

### In scope

- parent planning-surface hardening in:
  - [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  - [project-plan.md](/home/thetu/planner/docs/project-plan.md)
- durable indexing of this follow-up spec in
  [session-start-and-doc-index.md](/home/thetu/planner/docs/session-start-and-doc-index.md)
- Home route hierarchy correction in:
  - [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  - [HomeHubPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/HomeHubPage.test.tsx)
  - [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- Blueprint route verification strengthening or truthful narrowing in:
  - [planner-ui-reset-phase-07-blueprint-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md)
  - [BlueprintPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/BlueprintPage.test.tsx)
- Events route verification strengthening in:
  - [planner-ui-reset-phase-09-events-timeline-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md)
  - [EventTimelinePage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/EventTimelinePage.test.tsx)

### Out of scope

- new route families or broader visual redesign
- backend or schema changes
- reopening already trustworthy routes without new evidence
- expanding Blueprint into a larger command-band or inspector rewrite
- adding new product behavior outside what is required to close the audited
  correction set

## Current-State Evidence

- [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  now has truthful implemented status and delivery notes, but it still does not
  carry the same explicit verification or fallback shape expected from the
  tranche's child specs.
- [project-plan.md](/home/thetu/planner/docs/project-plan.md)
  is now synchronized enough to track this follow-up spec explicitly, but the
  residual route corrections it names still need execution before the tranche
  can be treated as fully trustworthy.
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  now has a strong project-first launch deck, but it still renders the
  `Utilities` section ahead of `Recent Projects`, which weakens the intended
  supporting-surface hierarchy in
  [planner-ui-reset-phase-01-home-hub-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-01-home-hub-spec.md).
- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  clearly defaults to `graph` mode, but the named automated evidence still
  proves less than the route's broader manual verification list.
- [EventTimelinePage.tsx](/home/thetu/planner/planner-web/src/pages/EventTimelinePage.tsx)
  implements grouped timeline sections, snapshots mode, and fetch failure UI,
  but the named test file still does not directly assert the fetch-failure
  state called out in the route spec.

## Requirements

### Parent planning truthfulness

- the queue must gain an explicit closure-oriented section that records:
  - why the parent container is considered implemented
  - what verification snapshot supports that state
  - what kind of future work would belong outside the exhausted queue
- the project tracker must keep distinguishing:
  - trustworthy implemented routes
  - implemented routes with residual tracked corrections
  until this follow-up slice is delivered

### Home route alignment

- `/` must present `Recent Projects` as the primary supporting surface after
  the launch deck
- `Utilities` must remain available, but visually defer to recent project
  launch context
- populated and empty Home states must preserve that hierarchy
- route tests must directly prove the supporting-surface order and the
  maintained empty-state posture

### Blueprint verification truthfulness

- `UIR-07` must gain direct route-specific automated evidence for the bounded
  graph-first posture and preserved multi-view behavior
- if stronger Blueprint evidence proves impossible without broadening route
  scope, the slice must fall back to truthful doc narrowing instead of forcing
  confidence that the test surface cannot support

### Events verification truthfulness

- `UIR-09` must gain direct route-specific automated evidence for the
  fetch-failure state named in its state model and verification plan
- after that test lands, the Events spec must refresh its verification note so
  it cites the actual evidence that was rerun

## Acceptance Criteria

- this follow-up spec is indexed and tracked as the next bounded tranche move
- the queue parent container has an explicit closure-oriented verification story
- `project-plan.md` no longer uses one blanket tranche-verification claim that
  outruns current route-level evidence
- Home route implementation matches the spec's supporting-surface hierarchy more
  closely
- `UIR-07` is either strengthened by direct automated evidence or narrowed
  truthfully enough that no verification overstatement remains
- `UIR-09` has direct automated evidence for fetch failure in addition to the
  already-added grouped timeline, filter, and snapshot tests
- the slice stays bounded to residual correction work and does not reopen the
  broader redesign tranche

## Verification Plan

### Focused route verification

- `npm test -- src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx`
- `npx tsc --noEmit`

### Planning review

Confirm after delivery that:

- the queue and project plan tell the same residual-corrections story
- Home now reads as launch deck, then recent projects, then quieter utilities
- Blueprint only claims the evidence it can directly support
- Events now has direct automated coverage for the failure state named in the
  spec

## Rollback And Fallback

- if the Home hierarchy change proves too disruptive, prefer narrowing the Home
  spec and tracker language over keeping a false full-alignment claim
- if Blueprint verification cannot be strengthened without broadening route
  scope, narrow the spec's verification/result language instead of inventing
  confidence
- if the Events fetch-failure test exposes a real mismatch, correct the doc
  status first rather than silently preserving an overstated verified result
- if the parent queue still resists a stronger closure shape, prefer an
  explicit "implemented queue container with follow-on specs outside scope"
  statement over another vague delivery note

## Implementation Notes

- Implemented the residual tranche-correction slice as a bounded closeout pass
  rather than reopening the broader UI reset program.
- Reordered the Home route so `Recent Projects` now renders ahead of the
  quieter `Utilities` section while preserving the existing project-first launch
  deck and empty-state posture.
- Strengthened Blueprint route verification with direct automated evidence for
  the graph-first default and preserved multi-view switching behavior.
- Strengthened Events route verification with direct automated evidence for the
  fetch-failure state already named in the route spec.
- Synchronized the parent planning surfaces so the queue and tracker now record
  the tranche as implemented without leaving this residual correction set open.

## Verification Snapshot (2026-03-22)

Passed:

- `npm test -- src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx`
- `npx tsc --noEmit`

Result:

- `3/3` targeted route files passed
- `14/14` targeted tests passed

## Open Questions

None blocking closure.
