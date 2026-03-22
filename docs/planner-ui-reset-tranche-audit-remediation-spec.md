# Planner UI Reset Tranche Audit Remediation Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Planner UI Reset Phase 07 Blueprint Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md), [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md), [Planner UI Reset Phase 10 Admin Operations Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md)  
**Source Audit:** 2026-03-22 UI redesign tranche audit against specs, implementation, and route tests

## Objective

Close the documented-tranche trust gaps surfaced by the 2026-03-22 audit so
the Planner UI reset queue, child specs, and verification evidence describe the
current `planner-web` reality truthfully.

This remediation slice is intentionally bounded:

- fix planning-surface contradictions
- narrow overstated child-spec claims where the implementation is smaller than
  the spec body
- strengthen route-specific verification where the implementation exists but
  the evidence is too thin

It is not a second redesign pass.
It does not broaden product scope beyond status truthfulness and missing tests.

## Audit Baseline

The audit found:

- 12 audited artifacts total:
  the queue container plus 11 route specs
- 4 fully trustworthy artifacts
- 5 artifacts with meaningful drift
- 3 artifacts whose status or readiness claims were overstated enough to
  downgrade or reopen
- full cited frontend verification still green at audit time:
  `114/114` tests passed across the named route files and `npx tsc --noEmit`
  passed

The tranche score at audit time was:

- average spec completeness: `7.9/10`
- average implementation alignment: `7.3/10`
- average verification confidence: `2.8/5`
- overall tranche score: `70/100`

## Scope

### In scope

- queue and tracker synchronization for the UI reset tranche:
  [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  and
  [project-plan.md](/home/thetu/planner/docs/project-plan.md)
- durable indexing of this remediation spec in
  [session-start-and-doc-index.md](/home/thetu/planner/docs/session-start-and-doc-index.md)
- child-spec truthfulness updates for:
  - [phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)
  - [planner-ui-reset-phase-07-blueprint-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md)
  - [planner-ui-reset-phase-09-events-timeline-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md)
  - [planner-ui-reset-phase-10-admin-operations-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md)
- verification-strengthening tests for the routes flagged as under-evidenced:
  - [HomeHubPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/HomeHubPage.test.tsx)
  - [KnowledgeLibraryPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx)
  - [DiscoveryPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/DiscoveryPage.test.tsx)
  - [EventTimelinePage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/EventTimelinePage.test.tsx)
  - [AdminPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/AdminPage.test.tsx)
- verification-note refreshes in affected route specs once the tests are rerun

### Out of scope

- broad route redesign or follow-on UI polish
- backend or schema changes for the Socratic lobby
- a larger Blueprint command-band or inspector refactor
- new user-facing route behavior unless a missing test exposes a real defect
- changes to `UIR-00`, `UIR-02`, `UIR-03`, or `UIR-04` beyond noting that they
  remain trustworthy

## Audit Ledger And Required Action

| Artifact | Audit verdict | Required action in this slice |
| --- | --- | --- |
| `QUEUE` | `needs rewrite or reopen` | Rewrite queue status/readiness so the parent container matches the implemented child table and no longer implies undelivered work. |
| `UIR-00` | `implemented and trustworthy` | No product or status change. Preserve as trustworthy baseline and do not dilute it with unnecessary edits. |
| `UIR-01` | `implemented with drift` | Keep implemented status, but strengthen route tests so empty and error states are actually evidenced. |
| `UIR-02` | `implemented and trustworthy` | No product or status change. Preserve as trustworthy baseline. |
| `UIR-03` | `implemented and trustworthy` | No product or status change. Preserve as trustworthy baseline. |
| `UIR-04` | `implemented and trustworthy` | No product or status change. Preserve as trustworthy baseline. |
| `UIR-05` | `implemented with drift` | Narrow the child spec so it clearly describes the delivered bounded web pass and no longer implies unevidenced backend/schema expansion. |
| `UIR-06` | `implemented with drift` | Keep implemented status, but add a route-specific assertion that inventory remains the default dominant section on project-scoped Knowledge. |
| `UIR-07` | `spec stronger than implementation` | Narrow the spec language so it matches the delivered graph-first posture reset instead of a broader command-chrome rewrite. |
| `UIR-08` | `implemented with drift` | Keep implemented status, but strengthen tests around grouped review state, empty state, and scan mutation evidence. |
| `UIR-09` | `status overstated` | Add the missing timeline-route tests for stream grouping, filters, snapshots, and snapshot creation, then refresh verification notes to match what was rerun. |
| `UIR-10` | `status overstated` | Add the missing admin-route tests for healthy/warning/degraded posture, filtered stream, empty state, and load failure, then refresh verification notes. |

## Requirements

### Planning truthfulness

- [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  and
  [project-plan.md](/home/thetu/planner/docs/project-plan.md)
  must stop contradicting themselves about whether the tranche is merely ready
  or already implemented.
- The queue must explicitly describe the tranche as implemented and audited
  rather than as pending delivery.
- The project plan must stop listing `UIR-03` through `UIR-10` as both `ready`
  and already delivered.

### Child-spec truthfulness

- `UIR-05` must clearly describe the delivered focused-lobby slice as a bounded
  web reset using existing contracts.
- `UIR-07` must clearly describe the delivered slice as the graph-first posture
  reset, not a completed command-band and inspector overhaul.
- `UIR-09` and `UIR-10` may keep implemented status only if their verification
  notes cite the newly added route tests run in this slice.

### Verification strengthening

- `UIR-01` must gain automated evidence for Home empty and error states.
- `UIR-06` must gain automated evidence that project-scoped Knowledge defaults
  to inventory-first posture rather than overview-first posture.
- `UIR-08` must gain automated evidence for grouped pending-vs-reviewed
  hierarchy, empty-state behavior, and scan-in-progress state.
- `UIR-09` must gain automated evidence for timeline grouping, filtered events,
  snapshots mode, empty snapshots, and snapshot creation.
- `UIR-10` must gain automated evidence for healthy, warning-heavy, and
  operator-attention postures, filtered or empty event streams, and load
  failure.

### No-regression baseline

- `UIR-00`, `UIR-02`, `UIR-03`, and `UIR-04` remain part of the remediation
  ledger, but this spec must not invent work for them when the audit found them
  trustworthy.

## Acceptance Criteria

- the new remediation spec records the full audit baseline and the required
  action for all 12 audited artifacts
- queue and tracker language no longer contradict actual tranche delivery state
- `UIR-05` and `UIR-07` describe the shipped bounded slices truthfully
- `UIR-01`, `UIR-06`, `UIR-08`, `UIR-09`, and `UIR-10` have stronger
  route-specific automated evidence for the gaps identified in the audit
- `UIR-09` and `UIR-10` no longer claim verification beyond the tests actually
  present and rerun
- the remediation work does not broaden into a second UI redesign pass
- `UIR-00`, `UIR-02`, `UIR-03`, and `UIR-04` remain unchanged except for any
  top-level tracker references needed for tranche coherence

## Verification Plan

### Frontend route verification

Run the cited route suite again after the remediation work lands:

- `npm test -- src/components/__tests__/Layout.test.tsx src/pages/__tests__/LoginPage.test.tsx src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/Dashboard.test.tsx src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/KnowledgeLibraryPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/DiscoveryPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx src/pages/__tests__/AdminPage.test.tsx`
- `npx tsc --noEmit`

### Review

Confirm after implementation that:

- the queue and project plan tell the same tranche story
- the new remediation spec still matches the executed slice exactly
- the widened test evidence covers the audit gaps and does not rely on manual
  inference
- the child specs only claim the breadth they can actually support

## Rollback And Fallback

- if a route-specific test reveals that the code does not actually satisfy the
  claimed behavior, do not keep the stronger status language; downgrade the doc
  instead of forcing a misleading pass state
- if the queue and project-plan state cannot be synchronized without wider
  planning churn, prefer a smaller truthful tracker state over another broad
  rewrite
- if a child spec still needs a bigger follow-on feature after this slice, note
  that explicitly rather than broadening this remediation spec

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the tranche audit remediation as a bounded planning-and-
  verification slice rather than a second redesign pass.
- Added this durable remediation spec and synchronized:
  [session-start-and-doc-index.md](/home/thetu/planner/docs/session-start-and-doc-index.md),
  [project-plan.md](/home/thetu/planner/docs/project-plan.md),
  and
  [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  so the queue and tracker now describe the tranche coherently.
- Narrowed the child-spec truthfulness gaps called out by the audit in:
  [phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)
  and
  [planner-ui-reset-phase-07-blueprint-workspace-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md),
  while refreshing verification notes in the affected route specs.
- Strengthened route-specific verification for the under-evidenced routes in:
  [HomeHubPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/HomeHubPage.test.tsx),
  [KnowledgeLibraryPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx),
  [DiscoveryPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/DiscoveryPage.test.tsx),
  [EventTimelinePage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/EventTimelinePage.test.tsx),
  and
  [AdminPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/AdminPage.test.tsx).

## Verification Snapshot (2026-03-22)

Passed:

- `npm test -- src/components/__tests__/Layout.test.tsx src/pages/__tests__/LoginPage.test.tsx src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/Dashboard.test.tsx src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/KnowledgeLibraryPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/DiscoveryPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx src/pages/__tests__/AdminPage.test.tsx`
- `npx tsc --noEmit`

Result:

- `12/12` route files passed
- `126/126` tests passed
