# Import Existing Project Phase 7 History And Draft Diff Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md)

## Objective

Advance `Import Existing Project` from a durable re-import lifecycle into a
reviewable import history surface with lightweight draft comparison.

This slice should let a user see prior import attempts for a project, inspect
the latest pending or applied handoff in context, and understand how the
current draft differs from the last applied import without yet introducing
automatic reconciliation or rollback of canonical blueprint state.

It does **not** yet introduce apply-time rollback, per-node cherry-picking, or
automatic cleanup of previously applied canonical blueprint records.

## User Outcome

After this slice:

- an imported project exposes a concise import history on the project workflow
  surface
- the user can inspect the latest pending draft and the last applied import in
  one project-owned place
- when a re-import produces a new draft, the user can see a lightweight
  summary of what changed relative to the last applied import
- review/apply remains explicit and project-scoped

The user still does **not** get automatic blueprint reconciliation, rollback of
older applied imports, or a dedicated import management dashboard.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 7 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`
- `planner-web/src/api/__tests__/client.test.ts`

Delivered behavior:

- `GET /projects/{projectRef}/import-history` now returns project-scoped import
  history entries in reverse chronological order
- history entries include durable job metadata plus draft/source metadata when
  available
- the server now computes a lightweight draft-vs-last-applied diff summary for
  the latest pending import draft
- `ProjectSessionsPage` now renders an `Import History` section and a concise
  `Changes Since Last Applied Import` summary alongside the existing review and
  re-import workflow
- failed import attempts remain visible in history without breaking the latest
  review/apply anchor for the project

Verification completed:

- `cargo test -p planner-server import_history -- --nocapture`
- `cargo test -p planner-server failed_jobs_appear_in_history -- --nocapture`
- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server reimport -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- import history is a **project-local workflow aid**, not a separate global
  dashboard
- the comparison target is the **last applied import draft for the same
  project**, not the entire canonical blueprint graph
- diff output for this slice is **summary-first**, not a full visual graph diff
- apply remains an explicit action on the current pending draft only
- history should include failed attempts for operator visibility, but failed
  jobs do not need diff output
- this slice must not auto-delete or auto-revert canonical blueprint nodes that
  were applied from older imports
- richer reconciliation behavior remains a later spec

## Scope

### In scope

- add a project-scoped import history read API
- return import jobs in reverse chronological order with enough metadata for a
  project timeline:
  - status
  - provider
  - source reference
  - timestamps
  - seed session linkage when available
- expose the latest pending/applied import handoff together with a comparison
  summary against the last applied import when both exist
- define a lightweight diff summary for import drafts, such as:
  - added discovered nodes
  - removed discovered nodes
  - node-type counts
  - changed analysis summary/source revision metadata when useful
- surface the history and diff summary on the existing
  `ProjectSessionsPage` import workflow surface
- keep the current apply and re-import actions bound to the latest draft and
  latest import state

### Out of scope

- rollback of previously applied canonical blueprint state
- per-node accept/reject controls
- editing or mutating historical import drafts
- a global import jobs page
- automatic reconciliation of canonical blueprint records with removed draft
  nodes
- diffing directly against the full live canonical blueprint graph
- provider expansion, branch selection, or private GitHub auth

## Current-State Evidence

- Phase 6 now persists multiple import jobs per project and keeps the latest
  import state available through
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
  and [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs).
- The import store already persists durable draft payloads per import job, so
  historical draft-vs-draft comparison is now possible without inventing a new
  storage model.
- `ProjectSessionsPage` already exposes the current review/apply surface and
  latest import state, which is the right existing place to add history and
  comparison without fragmenting the workflow.
- The current product still only exposes the latest import state; there is no
  project-scoped history list and no user-visible explanation of what changed
  between a new re-import and the previously applied handoff.

## Requirements

### History contract

For a project with any import activity:

- the server must expose a project-scoped history payload that returns import
  jobs in descending recency
- each history entry must include enough metadata for the UI to explain what
  happened without knowing a job id in advance
- history should include at minimum:
  - job identity
  - provider
  - requested source reference
  - status
  - created/updated timestamps
  - seed session id when available
  - source revision metadata when known

This slice does not require pagination unless the implementation surface needs
it immediately.

### Diff summary contract

When a project has:

- a latest `review_pending` draft, and
- a prior `applied` import draft,

the server should return a lightweight comparison summary derived from the two
persisted draft payloads.

The comparison summary must be stable and easy for the UI to render. A valid
minimum shape is:

- `added_nodes`
- `removed_nodes`
- `added_node_types`
- `removed_node_types`
- optional source revision change summary

Implementation may choose exact field names, but the user-visible outcome must
answer: “what is new, what disappeared, and which draft is this compared
against?”

This slice does **not** require semantic graph reconciliation or blueprint
mutation planning.

### Review workflow contract

The project workflow surface should continue to feel single-threaded:

- history is visible alongside the latest import workflow, not in a detached
  admin surface
- the current pending draft remains the only apply target
- the last applied import remains visible as the comparison anchor
- failed jobs should be visible in history, but they should not block access to
  the latest reviewable draft

### UI contract

`ProjectSessionsPage` should gain a lightweight history section that:

- shows the latest import state and current review/apply card as it does today
- lists recent import attempts with status and timestamp
- surfaces a concise “changes since last applied import” summary when present
- lets the user open the seeded session tied to the relevant latest draft when
  available

This slice does **not** require a visual diff viewer, timeline scrubber, or
full-page history route.

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- optional focused server integration coverage if the history route is exercised
  there

Implementation should stay bounded to history plus draft comparison. If the
work starts drifting into rollback semantics, stop and split the next slice.

## Acceptance Criteria

- a project with multiple import attempts exposes project-scoped import history
- the history surface shows enough metadata to distinguish queued, failed,
  review-pending, and applied attempts
- when a new pending draft exists and a prior applied draft exists, the UI can
  show a concise diff summary between them
- the existing apply and re-import workflow remains intact on the same project
  page
- no automatic rollback or canonical blueprint reconciliation is introduced in
  this slice

## Verification Plan

### Server

- tests for project-scoped import history ordering
- tests for diff summary generation between a pending draft and the last
  applied draft
- tests proving failed jobs appear in history without breaking the latest
  review/apply lookup

### Web

- `ProjectSessionsPage` tests for rendering import history entries
- `ProjectSessionsPage` tests for showing diff summary when both pending and
  applied drafts exist
- client tests for any new history/diff payload shapes

## Rollback And Fallback

- if draft comparison logic becomes too noisy, ship history first with a very
  small summary payload instead of broadening into full reconciliation
- if canonical blueprint comparison is required to make the diff trustworthy,
  stop and split that into a later reconciliation spec rather than silently
  expanding this one

## Open Questions

- how many history entries should be shown by default before the UI needs a
  “show more” affordance
- whether the comparison summary should be generated eagerly on the server or
  computed lazily for only the latest pending draft
- whether a later reconciliation slice should compare against the last applied
  draft only or also reason about the current canonical blueprint state
