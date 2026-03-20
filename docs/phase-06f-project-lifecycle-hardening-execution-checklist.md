# Phase 6F Project Lifecycle Hardening Execution Checklist

**Status:** Implemented and hardened; focused validation green; residual manual regression remains release signoff work  
**Date:** 2026-03-08

## Objective

Finish the lifecycle work with integration coverage, edge-case handling, and a
safe rollout order.

## Scope Guardrails

### In scope

- server integration hardening
- focused unit-test follow-up near new helpers
- regression matrix for archive, stop, delete, purge, and UI flows
- rollout verification notes

### Explicitly out of scope

- new lifecycle features
- redesigning archive/delete UX
- replacing any earlier phase contract

## Success Criteria

- archive and delete flows are covered end to end
- edge cases no longer rely on manual verification only
- the rollout order is explicit and low-risk

## Current Code Anchors

- `planner-server/tests/server_integration.rs`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-web/src/api/__tests__/client.test.ts`
- focused helper test files introduced in `6B` through `6D`

## Hardening Checklist

## Active Execution Checklist

### Discovery / verification

- [x] Verify whether Phase `6A` through `6E` are still implementation candidates or already landed in code
- [x] Confirm the canonical implementation anchors still map to live code paths
- [x] Run focused server lifecycle integration coverage
- [x] Run focused planner-core delete/purge coverage
- [x] Run focused frontend lifecycle coverage

### Implementation follow-up

- [x] Fix any Phase 06 lifecycle gaps uncovered by focused validation
- [x] Keep the checklist aligned with actual coverage and behavior as gaps are resolved

### Tests / validation

- [x] Verify archive project coverage exists and passes
- [x] Verify unarchive project coverage exists and passes
- [x] Verify delete cascade coverage exists and passes for sessions, event files, CXDB, and blueprint scope behavior
- [x] Verify frontend coverage exists and passes for archive, unarchive, delete confirm/cancel/success/failure, and in-flight disabled states

### Rollout / cleanup

- [x] Update the Phase 06 docs from "ready for execution" language to reflect verified implementation reality
- [x] Record any residual risks or blockers that remain after validation

## Validation Snapshot (2026-03-18)

Focused validation passed:

- `cargo test -p planner-server archive_project -- --nocapture`
- `cargo test -p planner-server delete_project -- --nocapture`
- `cargo test -p planner-core purge_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectsPage.test.tsx`

Follow-on validation update (2026-03-20):

- project delete coverage now also verifies import-owned cleanup for import
  jobs, import drafts, and managed GitHub checkout directories
- delete coverage also verifies that external local import roots are preserved
  during project delete

Residual manual signoff still pending:

These checks are release-confidence verification tasks. They are not open
implementation gaps in the lifecycle feature itself.

- archive a project manually and confirm it disappears from the default list
- enable `Show archived` and restore it
- delete a quiet project and confirm it disappears cleanly
- delete a project with live work and confirm stop behavior
- confirm shared knowledge remains visible in linked projects
- confirm direct-route behavior for archived projects

## Step 1: Finish server integration coverage

### Tests to ensure exist

1. archive project
2. unarchive project
3. delete project with no sessions
4. delete project with multiple sessions
5. delete project with active interview runtime
6. delete project with active pipeline work
7. delete project removes event files
8. delete project removes CXDB data
9. delete project deletes local blueprint data
10. delete project preserves shared blueprint data by unlinking
11. delete forbidden for non-owner
12. delete not found

## Step 2: Finish frontend coverage

### Tests to ensure exist

1. archived projects hidden by default
2. show archived toggle behavior
3. archive action success and failure
4. unarchive action success
5. delete cancel path
6. delete success path
7. delete failure path
8. action disabled states while requests are in flight

## Step 3: Finish focused helper coverage

### Areas to verify

- pipeline registry stop and cleanup semantics
- project-store archive and delete helpers
- CXDB project delete helper
- blueprint project purge and shared unlink helper
- blueprint durable history compaction behavior

## Step 4: Manual regression matrix

### Verify manually

1. archive a project and confirm it disappears from the default list
2. enable `Show archived` and restore it
3. delete a quiet project and confirm it disappears cleanly
4. delete a project with live work and confirm stop behavior
5. confirm shared knowledge remains visible in linked projects
6. confirm direct route behavior for archived projects

## Step 5: Rollout signoff

### Confirm before calling the phase done

- Phase `6A` archive behavior is still stable
- Phase `6B` runtime stop behavior is reliable
- Phase `6C` delete cascade counts are accurate
- Phase `6D` blueprint/CXDB deletion is complete enough for the true-delete
  promise
- Phase `6E` UI copy still matches backend behavior

## Recommended Command Order

```bash
# 1. Run focused server integration coverage
cargo test -p planner-server delete_project -- --nocapture

# 2. Run focused planner-core delete/purge coverage
cargo test -p planner-core purge_project -- --nocapture

# 3. Run focused frontend lifecycle coverage
cd planner-web && npm test -- --run client.test.ts ProjectsPage.test.tsx
```

## Exit Criteria For Phase 6F

- lifecycle behavior is hardened by tests
- manual regression points have been exercised
- the archive/delete implementation is ready for normal delivery

## Tests To Add Or Update

### Server integration

Add cases for:

- archive project
- unarchive project
- delete project with no sessions
- delete project with multiple sessions
- delete project removes session event files
- delete project stops live interview runtime
- delete project stops active pipeline work
- delete project removes local blueprint nodes
- delete project unlinks shared blueprint nodes without deleting them
- delete project removes CXDB project-run data
- delete project forbidden for non-owner
- delete project not found

### Frontend tests

Add cases for:

- archived projects hidden by default
- archived projects shown when filter is enabled
- archive action triggers reload
- delete confirmation text is shown
- delete cancelled path does not call API
- delete success removes the project from the rendered list
- delete failure leaves the project visible and shows the error

## Risks, Dependencies, And Rollout Order

### Primary risks

- pipeline stop behavior may be incomplete if spawned work internally owns
  additional unmanaged tasks
- blueprint delete semantics can become unsafe if shared versus local ownership
  is inferred inconsistently
- blueprint event and history compaction is the main risk for honoring the
  promise of true delete
- CXDB delete can remove run metadata while leaving content-addressed blobs
  orphaned unless blob GC strategy is defined

### Dependencies

- archive can ship before delete
- true delete depends on pipeline cancellation support
- true delete depends on blueprint durable-history pruning support
- the UI should not ship a hard-delete affordance before backend summary and
  stop behavior exist

### Recommended rollout order

1. ship archive and archived filtering
2. ship pipeline runtime registry and stop primitives
3. ship backend delete cascade for projects, sessions, and session events
4. ship CXDB delete plus blueprint local delete and shared unlink
5. ship blueprint history and event compaction
6. ship the UI delete affordance
7. close with integration and regression hardening

## Unresolved Questions

- Should archived projects remain navigable from all deep links, or should some
  product surfaces redirect them into a dedicated archived view?
- Should deleting a project also delete project-level exports recorded in the
  blueprint event log, or are those fully covered by blueprint event compaction?
- Is orphaned CXDB blob garbage acceptable in the first delete implementation,
  or must blob-level GC ship in the same phase?
- Do we want a typed server-side confirmation token later, or is the first cut
  allowed to rely on client confirmation plus normal authorization?
