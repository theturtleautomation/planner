# Import Existing Project Phase 8 Canonical Reconciliation Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md)

## Objective

Advance `Import Existing Project` from history and draft comparison into
explicit canonical reconciliation on apply.

This slice should make `Apply Import Draft` do more than additive upsert. When
a new import draft is applied, Planner should reconcile previously
import-applied project-local blueprint state against the latest draft so stale
import-owned records do not remain active forever.

It does **not** yet introduce rollback to an arbitrary older import, per-node
cherry-picking, or destructive deletion of ambiguous project-local knowledge.

## User Outcome

After this slice:

- applying a new import draft updates canonical project blueprint knowledge to
  match the latest approved import result more truthfully
- newly discovered import nodes become active canonical project-local blueprint
  nodes as they do today
- previously import-applied nodes that are no longer present in the latest
  approved draft are archived and removed from active import ownership
- manually created or otherwise non-import-owned project-local knowledge
  remains untouched
- the existing project workflow can explain that apply will reconcile the
  project’s import-owned blueprint state, not just add more nodes

The user still does **not** get rollback to a prior import, manual merge
editing, or a full canonical-vs-draft visual reconciliation UI.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 8 delivery slice.

Execution landed in:

- `planner-server/src/api.rs`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- `Apply Import Draft` now reconciles active import-owned project-local
  blueprint state instead of only appending nodes
- previously applied import-owned nodes that disappear from the new draft are
  archived and removed from active project-root membership
- currently imported nodes are promoted as active project-local records and
  stamped with durable import-draft provenance for safer later reconciliation
- non-import-owned project-local nodes remain untouched during apply
- `ProjectSessionsPage` now tells the user that apply will reconcile import-
  owned blueprint state before they confirm the action

Verification completed:

- `cargo test -p planner-server apply_project_import_review -- --nocapture`
- `cargo test -p planner-server failed_jobs_appear_in_history -- --nocapture`
- `cargo test -p planner-server import_history -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/ProjectSessionsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- reconciliation happens only during the explicit `Apply Import Draft` action
- reconciliation targets only **import-owned, project-local blueprint state**
  that can be safely attributed to prior import apply operations
- nodes absent from the newly applied draft should be **archived**, not hard
  deleted, in this first reconciliation slice
- reconciliation must never touch:
  - shared blueprint records
  - global records
  - project-local records that cannot be safely proven import-owned
- the latest draft remains the only apply target; this slice does not add
  “apply older import” behavior
- the Phase 7 history and diff summary remain the review aid; this slice does
  not add a second reconciliation dashboard
- rollback of already archived import-owned nodes is a later spec, not part of
  this slice

## Scope

### In scope

- extend the existing apply path so canonical blueprint promotion is
  reconciliation-aware
- identify the project-local nodes previously introduced by import apply for
  the current project
- keep or upsert nodes still present in the newly applied draft
- archive previously import-owned nodes that are absent from the newly applied
  draft
- reconcile project-root `contains` membership so active import-owned nodes
  still appear on the canonical project root and archived/removed ones no
  longer do
- update any import-owned provenance bookkeeping needed to make later
  reconciliations safe and deterministic
- make the project workflow copy truthful that apply now reconciles import-
  owned blueprint state rather than only appending nodes
- add focused tests for reconciliation safety and archive semantics

### Out of scope

- rollback to any arbitrary older import job
- hard deletion of archived import-owned nodes
- per-node accept/reject controls
- editing draft contents before apply
- reconciliation of shared/global blueprint records
- semantic merging with manually edited project-local nodes
- a full graph diff viewer between canonical blueprint and import draft

## Current-State Evidence

- the current apply path in
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
  promotes import draft nodes into canonical blueprint state by upserting
  nodes and adding project-root `contains` edges, but it does not reconcile
  stale import-owned nodes removed from later drafts
- Phase 7 now exposes import history plus a lightweight draft-vs-last-applied
  summary, so the user can already see what changed before apply
- import apply currently records per-apply `contains` edge metadata such as
  `import-review:{job_id}`, which is the clearest current anchor for tracing
  import-owned membership
- the blueprint data model already supports `NodeLifecycle::Archived`, which is
  the safest first-cut outcome for removed import-owned nodes

## Requirements

### Reconciliation contract

When a project applies a latest `review_pending` import draft:

- the server must determine the set of prior import-owned canonical
  project-local nodes for that project
- the server must compare that set against the nodes present in the newly
  applied draft
- nodes present in the new draft must remain active and project-local
- nodes previously import-owned but absent from the new draft must be archived
  and removed from active project-root import membership

Implementation may choose the exact provenance mechanism, but the result must
be deterministic and safe across repeated applies.

### Safety contract

Reconciliation must be conservative:

- shared/global records must never be archived by import reconciliation
- manually created or manually maintained project-local records must not be
  swept up unless the system can safely prove they are import-owned
- if provenance is ambiguous, the record should remain untouched and the apply
  should still complete rather than guessing destructively

This slice prioritizes safe under-reconciliation over unsafe over-deletion.

### Idempotency contract

Repeated apply requests for the same latest draft must remain stable:

- re-applying an already applied draft must not archive additional records
- re-applying must not duplicate `contains` edges
- previously archived import-owned records absent from the current draft should
  remain archived

### Workflow contract

The project review workflow should stay coherent:

- the Phase 7 diff summary remains the user-visible explanation of changes
- apply copy on the project page should explain that the action now reconciles
  import-owned project blueprint state
- no second page or detached reconciliation control surface is introduced

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-core/src/blueprint.rs](/home/thetu/planner/planner-core/src/blueprint.rs)
- [planner-schemas/src/artifacts/blueprint.rs](/home/thetu/planner/planner-schemas/src/artifacts/blueprint.rs)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- focused server integration or API tests around apply behavior

Implementation should stay bounded to reconciliation during apply. If the work
starts requiring arbitrary rollback or manual merge tooling, stop and split
that into a later spec.

## Acceptance Criteria

- applying a new import draft reconciles previously import-owned project-local
  blueprint state instead of only appending nodes
- import-owned nodes absent from the new applied draft become archived rather
  than staying active forever
- active project-root membership reflects the latest applied import result
- non-import-owned project-local records remain untouched
- repeated apply requests stay idempotent
- no rollback UI or destructive hard-delete semantics are introduced in this
  slice

## Verification Plan

### Server

- tests proving apply archives previously import-owned nodes missing from the
  newly applied draft
- tests proving nodes still present in the new draft remain active and linked
- tests proving non-import-owned project-local nodes remain untouched
- idempotency tests for repeated apply after reconciliation

### Web

- `ProjectSessionsPage` tests for truthful apply copy or reconciliation copy
  updates
- ensure the existing Phase 7 history/diff review surface still works after
  apply behavior changes

## Rollback And Fallback

- if provenance is too weak to safely archive old import-owned nodes, fall back
  to adding explicit provenance markers during apply and keep archival bounded
  to only provable records
- if the implementation starts needing hard delete to stay consistent, stop and
  split that into a later cleanup slice rather than broadening this spec

## Open Questions

- whether provenance should live primarily on project-root `contains` edges,
  node tags, or another explicit import-ownership field
- whether archived import-owned nodes should stay visible in any project UI by
  default or remain audit-only
- whether a later rollback slice should restore archived import-owned nodes or
  always regenerate from historical draft payloads
