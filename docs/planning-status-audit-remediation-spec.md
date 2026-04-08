# Planning Status Audit Remediation Spec

**Status:** Implemented  
**Date:** 2026-03-19  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Audit:** 2026-03-19 repo audit against code, tests, and planning docs

## Objective

Close the remaining planning-status drift surfaced by the 2026-03-19 audit so
the repo's durable docs accurately describe what is implemented, what is only
awaiting manual signoff, and what is intentionally deferred.

This slice is explicitly about status truthfulness and planning hygiene. It is
not a product-feature spec.

## User Outcome

A contributor can trust the main planning and implementation docs when deciding
what to build next.

After this slice lands:

- the active import thread remains the product focus
- Phase 07 no longer looks like active implementation work when the code is
  already cut over
- Phase 06 lifecycle docs no longer blur together "implemented and test-covered"
  with "manual regression still pending"
- the project tracker points to this remediation slice explicitly before
  returning to the next import spec

## Scope

### In scope

- update the Phase 07 execution prompt doc so it matches the implemented source
  doc and current code reality
- update the Phase 06 lifecycle docs so they clearly distinguish automated
  closure from remaining manual signoff
- record this remediation slice in the top-level tracker as the immediate
  bounded closeout task
- keep the session-start doc index aligned with the new durable remediation doc
- rerun the narrow validation needed to support any status-language changes

### Out of scope

- new import feature work
- Phase 07 legacy-adapter deletion before the recorded migration-window gate
- Phase 06 manual regression execution itself
- changing server, core, or web behavior
- broad planning rewrites outside the affected status surfaces

## Current-State Evidence

- The import thread already has a canonical execution artifact in
  [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md),
  so the earlier "missing canonical spec" finding is already closed.
- The session-start index already includes the previously missing durable docs,
  so the earlier doc-index drift finding is already closed.
- [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md)
  now says `Implemented except scheduled post-window legacy-adapter removal`.
- [Phase 07 Socratic Prompt Protocol Redesign Implementation Prompt](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation-prompt.md)
  still says `In progress`, which overstates the remaining work.
- [Phase 06 Project Archive And Delete Implementation](/home/thetu/planner/docs/phase-06-project-archive-delete-implementation.md)
  and
  [Phase 6F Project Lifecycle Hardening Execution Checklist](/home/thetu/planner/docs/phase-06f-project-lifecycle-hardening-execution-checklist.md)
  both record focused validation as green, but they still retain manual
  regression/signoff language that should be framed as release verification, not
  unfinished implementation.

## Requirements

### Phase 07 status alignment

The execution prompt doc must align with the source implementation doc and
current code reality:

- treat prompt-envelope cutover as implemented
- describe the remaining work as scheduled post-window legacy-adapter cleanup
- preserve the migration-window gate and earliest removal date language
- avoid wording that implies active feature implementation remains open

### Phase 06 status alignment

The lifecycle docs must make the delivery state explicit:

- implementation is complete
- focused automated validation is green
- residual manual regression is still pending
- manual signoff is a release-confidence task, not unfinished code delivery

### Tracker synchronization

The top-level tracker must reflect both truths at once:

- `Import Existing Project` remains the active product thread
- this remediation spec is tracked explicitly as the bounded closeout slice and
  then the tracker advances back to the next import execution spec after
  closeout

### Documentation hygiene

- the session-start doc index must include this spec
- no other durable doc should be described as canonical for this remediation
  slice

## Acceptance Criteria

- the Phase 07 execution prompt doc no longer claims the phase is actively in
  progress beyond scheduled migration cleanup
- the Phase 06 and 06F docs explicitly distinguish implemented/test-covered
  behavior from pending manual regression
- `.omx/ledger/current-status.md` tracks this remediation slice explicitly and then
  advances cleanly back to the next import spec without changing the active
  product thread away from import
- `.omx/ledger/session-start-and-doc-index.md` includes this spec
- all status changes are backed by focused verification evidence or explicitly
  cite the existing verification snapshot they rely on
- no product implementation surface changes are introduced in this slice

## Verification Plan

### Focused verification

Run the narrowest checks needed to support the status updates:

- `cargo test -p planner-server legacy_question -- --nocapture`
- `cargo test -p planner-server legacy_draft -- --nocapture`
- `cargo test -p planner-server test_get_session_includes_current_prompt_in_checkpoint_payload -- --nocapture`
- `cargo test -p planner-server archive_project -- --nocapture`
- `cargo test -p planner-server delete_project -- --nocapture`
- `cargo test -p planner-core purge_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx`

### Review

Confirm after the edits that:

- Phase 07 source doc and execution prompt describe the same remaining work
- Phase 06 source docs do not imply implementation is still open
- project-plan next-move language is coherent with the import thread and this
  bounded remediation slice

## Verification Snapshot (2026-03-19)

Passed:

- `cargo test -p planner-server legacy_question -- --nocapture`
- `cargo test -p planner-server legacy_draft -- --nocapture`
- `cargo test -p planner-server test_get_session_includes_current_prompt_in_checkpoint_payload -- --nocapture`
- `cargo test -p planner-server archive_project -- --nocapture`
- `cargo test -p planner-server delete_project -- --nocapture`
- `cargo test -p planner-core purge_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/SessionPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx`

## Rollback / Fallback

- if focused verification no longer supports the current status language, do not
  force the docs to a more complete state; instead, revise the spec to reflect
  the newly discovered implementation gap
- if additional cross-cutting drift is discovered, capture it as a follow-on
  bounded spec instead of widening this slice silently

## Open Questions

- Should the project plan keep this remediation slice visible after it is
  implemented, or should it collapse back to import-only guidance once the docs
  are synchronized?
