# Planner UI Reset Audit Evidence Closeout Spec

**Status:** Implemented
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)
**Related Planning:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md), [Planner UI Reset Phase 00 Shell Navigation And Auth Spec](/home/thetu/planner/docs/planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Planner UI Reset Tranche Audit Remediation Spec](/home/thetu/planner/docs/planner-ui-reset-tranche-audit-remediation-spec.md), [Planner UI Reset Residual Corrections Spec](/home/thetu/planner/docs/planner-ui-reset-residual-corrections-spec.md)
**Source Audit:** 2026-03-22 UI redesign tranche audit against specs, implementation, and route tests

## Objective

Close the remaining evidence and status-truth gaps from the latest Planner UI
reset tranche audit so the tranche can be treated as implemented and
trustworthy without relying on inference.

This is a bounded verification-and-doc-sync slice.
It is not a new UI redesign pass.

## Scope

### In scope

- direct automated coverage for `UIR-00` auth-root and callback entry behavior
  in:
  - [planner-web/src/auth/Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx)
  - a new targeted test file under `planner-web/src/auth/__tests__/`
- direct automated coverage for the `UIR-05` focused-lobby reveal and branch-
  transition behaviors in:
  - [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
- status and verification-note synchronization in:
  - [planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md](/home/thetu/planner/docs/planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md)
  - [phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)
  - [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  - [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)
  - [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

### Out of scope

- new route behavior beyond what the named tests expose directly
- broad shell, auth, or Socratic workspace redesign
- backend, schema, or websocket contract changes
- reopening previously closed tranche corrections outside `UIR-00` and `UIR-05`

## Current-State Evidence

- `UIR-00` already delivers the shared entry-shell treatment in
  [App.tsx](/home/thetu/planner/planner-web/src/App.tsx),
  [Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx),
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx),
  and
  [LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx),
  but the cited automated evidence only covers shell navigation and dev-mode
  login.
- `UIR-05` already delivers the focused question canvas, question-map reveal,
  context shelf, and transition messaging in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  and
  [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx),
  but the current tests do not directly prove the hidden-by-default context
  shelf or the focus-transition branch state described in the spec.
- [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)
  and
  [planner-ui-reset-route-by-route-spec-queue.md](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)
  still summarize the tranche as fully synchronized without pointing to a
  closeout slice for these last proof gaps.

## Requirements

### `UIR-00` verification closeout

- add direct automated assertions for Auth0 root entry:
  loading, authenticated, and anonymous states
- add direct automated assertions for callback loading, callback error, and
  post-callback redirect behavior
- keep the tests bounded to existing entry-shell behavior rather than mocking a
  new auth flow

### `UIR-05` verification closeout

- add direct automated assertions that the context shelf is hidden by default
  and becomes accessible through the visible `Context` trigger
- add direct automated assertions that the context shelf exposes the named
  secondary surfaces without making them permanent first-class panes
- add direct automated assertions that the focus-transition branch state is
  explained inline in the main lobby flow and provides visible controls to
  re-focus or follow server focus

### Status synchronization

- the affected child specs must cite the new automated evidence explicitly
- the queue and project tracker must describe the tranche as implemented with
  this closeout slice recorded as the evidence-hardening follow-up
- no top-level doc should imply broader verification than the tests actually
  rerun in this slice

## Acceptance Criteria

- a targeted auth test file proves the `UIR-00` auth-root and callback states
- `SessionPage.test.tsx` proves the `UIR-05` context-shelf and focus-transition
  behaviors named above
- the two affected child specs refresh their implementation or verification
  notes to cite the new evidence truthfully
- the queue, tracker, and doc index record this closeout slice and no longer
  leave the final tranche-proof gaps implicit
- the work stays bounded to verification and status truthfulness rather than
  broadening the product surface

## Verification Plan

Run the tranche route suite plus the new auth coverage after the closeout lands:

- `npm test -- src/auth/__tests__/Auth0Pages.test.tsx src/components/__tests__/Layout.test.tsx src/pages/__tests__/LoginPage.test.tsx src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/Dashboard.test.tsx src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/KnowledgeLibraryPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/DiscoveryPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx src/pages/__tests__/AdminPage.test.tsx`
- `npx tsc --noEmit`

## Rollback And Fallback

- if the new auth tests expose a mismatch, downgrade `UIR-00` status language
  instead of faking trustworthiness
- if the focused-lobby tests expose a mismatch, narrow `UIR-05` verification
  claims before broadening the product scope
- if a top-level summary still cannot be stated precisely, prefer a narrower
  truthful tracker statement over a blanket verified claim

## Implementation Notes

- Implemented this closeout as a bounded verification-and-status-truthfulness
  slice rather than reopening the UI reset product surface.
- Added direct Auth0 entry-state coverage in
  [Auth0Pages.test.tsx](/home/thetu/planner/planner-web/src/auth/__tests__/Auth0Pages.test.tsx)
  so `UIR-00` now has automated proof for root loading, anonymous root entry,
  authenticated root entry, callback loading, callback error, and callback
  redirect states.
- Added direct focused-lobby coverage in
  [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
  so `UIR-05` now proves the hidden-by-default context shelf and the inline
  branch-transition state with visible re-focus controls.
- Synchronized the affected child specs, queue container, tracker, and doc
  index so the tranche now points to this closeout slice instead of leaving the
  last proof gaps implicit.

## Verification Snapshot (2026-03-22)

Passed:

- `npm test -- src/auth/__tests__/Auth0Pages.test.tsx src/components/__tests__/Layout.test.tsx src/pages/__tests__/LoginPage.test.tsx src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/Dashboard.test.tsx src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/KnowledgeLibraryPage.test.tsx src/pages/__tests__/BlueprintPage.test.tsx src/pages/__tests__/DiscoveryPage.test.tsx src/pages/__tests__/EventTimelinePage.test.tsx src/pages/__tests__/AdminPage.test.tsx`
- `npx tsc --noEmit`

Result:

- `13/13` targeted test files passed
- `136/136` tests passed

## Open Questions

None blocking closure.
