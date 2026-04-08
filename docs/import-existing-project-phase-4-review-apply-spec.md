# Import Existing Project Phase 4 Review Apply Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)

## Objective

Advance `Import Existing Project` from a review-pending draft handoff into an
explicit canonical blueprint promotion step.

This slice should let a user review the current import draft for a project and
apply it into the canonical project blueprint through an explicit approval
action. It should preserve the Phase 3 seeded session flow, but stop treating
`review_pending` as the end of the product path.

It does **not** yet solve local-provider hardening, re-import semantics, or
import lifecycle cleanup.

## User Outcome

After a GitHub import reaches `review_pending`, Planner can:

- surface the pending import review state on the project sessions surface
- show a concise review summary of what was discovered
- let the user open the seeded session for context if needed
- let the user explicitly apply the persisted import draft into the canonical
  project blueprint
- mark the import job as applied once canonical promotion succeeds
- route the user into the existing project blueprint view to inspect the
  applied result

The user still does **not** get per-node cherry-picking, re-import diffing, or
provider hardening in this slice.

## Implementation Notes

Implemented on 2026-03-20 in the bounded Phase 4 delivery slice.

Execution landed in:

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx`

Delivered behavior:

- project-level `GET /projects/{projectRef}/import-review` lookup for the
  current pending/applied import handoff
- explicit `POST /projects/{projectRef}/import-review` apply action
- durable `applied` import-job state
- idempotent promotion of persisted import draft nodes into canonical
  project-scoped blueprint knowledge
- explicit project-root `contains` membership edges for applied nodes
- `ProjectSessionsPage` review/apply banner with seeded-session and blueprint
  navigation

Verification completed:

- `cargo test -p planner-server project_import_review -- --nocapture`
- `cargo test -p planner-server apply_project_import_review -- --nocapture`
- `cargo test -p planner-server project_import -- --nocapture`
- `npm --prefix planner-web test -- --run src/api/__tests__/client.test.ts src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- canonical blueprint mutation remains **explicit approval only**
- the approval unit for this slice is the **entire persisted import draft** for
  one import job, not per-node accept/reject controls
- imported draft findings promote only into **project-local canonical
  blueprint knowledge**
- apply must **not** write into the global `ProposalStore`
- apply must create or reuse project-root membership via canonical blueprint
  `contains` edges where needed
- apply must be **idempotent** for a given import job
- the project-level review/apply surface lives on
  `ProjectSessionsPage`, not in a brand-new import dashboard
- the seeded session remains the planning-context handoff, but approval is a
  project review action rather than a hidden side effect of entering Socratic
- local provider hardening, re-import, duplicate-source detection, and delete
  cleanup remain deferred to later specs

## Scope

### In scope

- add a project-scoped import review read API for the current pending/applied
  import handoff
- add an explicit apply API that promotes one persisted import draft into the
  canonical blueprint
- extend import-job status to represent a successful apply result
- preserve `seed_session_id` and review metadata after apply
- ensure applied blueprint nodes remain project-scoped and attached to the
  canonical project root
- ensure repeated apply requests for the same import job do not duplicate the
  canonical result
- surface pending import review on `ProjectSessionsPage`
- surface a project-level `Apply Import Draft` action and post-apply success
  state on `ProjectSessionsPage`
- offer direct navigation to the seeded session before apply and to the
  project blueprint after apply

### Out of scope

- per-node or per-section accept/reject controls
- diff visualization between import draft and canonical blueprint
- import-draft editing in the UI
- promotion of code-graph edge proposals beyond the persisted draft node set
- local repo provider hardening
- branch selection, private GitHub auth, or re-import semantics
- import history views across multiple jobs
- project delete cleanup for draft/apply artifacts beyond current lifecycle
  behavior
- a dedicated import review page outside the existing project workflow

## Current-State Evidence

- Phase 3 now persists project-owned import draft state and seeded session
  metadata in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
  and [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs).
- The latest import flow already reaches truthful `review_pending` state and
  exposes `seed_session_id` plus `analysis_summary` to the web layer through
  [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
  and [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx).
- Canonical blueprint mutation already exists through the general blueprint
  APIs and store primitives in
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
  and [planner-core/src/blueprint.rs](/home/thetu/planner/planner-core/src/blueprint.rs),
  but imports do not yet call that path.
- Project-scoped session review surfaces already exist in
  [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx),
  which is the cleanest existing project-local place to expose review/apply
  state.
- There is currently no project-level import review endpoint, no explicit apply
  endpoint, no `applied` import-job state, and no UI that turns `review_pending`
  into canonical blueprint knowledge.

## Requirements

### Review state contract

Once an import job reaches `review_pending` or `applied`:

- the server must expose a project-scoped review payload for the current import
  handoff
- the payload must include:
  - canonical project identity
  - current import job status
  - `seed_session_id`
  - `analysis_summary`
  - source metadata from the persisted draft
  - a compact summary of discovered draft findings suitable for a project-level
    review banner

Implementation may choose the exact route shape, but the review path must bind
cleanly to an existing project page without requiring the client to already
know a job ID.

### Apply contract

The explicit approval action must:

- read the persisted import draft for the selected import job
- validate that the draft still belongs to the canonical project
- upsert the draft findings into the canonical blueprint as project-scoped
  records
- create or reuse `contains` membership edges from the project root to applied
  nodes where needed
- avoid touching the global `ProposalStore`
- mark the import job as applied only after canonical blueprint persistence
  succeeds

This slice should introduce a clear applied status, for example:

- `applied`: the persisted import draft has been promoted into canonical
  blueprint knowledge for that import job

The exact response shape may vary, but the server must return enough
information for the UI to show successful promotion and route into the
project-scoped blueprint view.

### Idempotency contract

Apply must be safe to repeat:

- re-applying the same import job must not duplicate canonical nodes or
  membership edges
- a repeated apply request for an already-applied job should return the current
  applied result rather than failing ambiguously
- a failed apply attempt should leave the import draft available for retry and
  should not falsely mark the job as applied

Implementation may satisfy this either through job-state gating, canonical
dedupe, or both, but the user-visible result must stay stable.

### Blueprint ownership contract

Applied import findings must preserve project-local ownership:

- promoted nodes remain explicitly project-scoped
- project-root membership remains canonical through `contains`
- applied findings appear in project-scoped blueprint views
- this slice must not silently promote imported findings into shared/global
  scope

This slice does not require broader inference or post-apply reconvergence. It
only needs a clean promotion of the current persisted draft into canonical
project-local blueprint state.

### UI contract

The review/apply experience should stay lightweight and project-first:

- add a project-level import review banner or card on `ProjectSessionsPage`
- when a project has a `review_pending` import, show:
  - concise import summary
  - source metadata context
  - `Open Seeded Session`
  - `Apply Import Draft`
- when apply succeeds, show:
  - applied success messaging
  - `Open Blueprint`
  - continued access to the seeded session if needed

This slice does **not** require:

- a standalone import review page
- rich diff tables
- node-level checkboxes

### Failure behavior

Apply failures must be explicit and truthful:

- if canonical blueprint promotion fails, return a clear error to the caller
- do not mark the job as applied on partial failure
- keep the draft reviewable/retryable after a failed apply attempt
- do not leave the UI implying canonical blueprint approval has happened when
  it has not

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-core/src/blueprint.rs](/home/thetu/planner/planner-core/src/blueprint.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectSessionsPage.test.tsx)
- [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)

Implementation may add a small import-review view model/helper, but execution
should stay bounded to review/apply and canonical blueprint promotion.

## Acceptance Criteria

- a project with a `review_pending` import exposes project-level review data
  without the UI needing to guess a job ID
- explicit apply promotes the persisted import draft into canonical
  project-scoped blueprint nodes and project-root membership edges
- the import job transitions from `review_pending` to `applied` only after
  canonical promotion succeeds
- repeated apply requests for the same job do not duplicate the canonical
  blueprint result
- the apply path does not write into the global `ProposalStore`
- `ProjectSessionsPage` surfaces the pending review state and allows the user
  to open the seeded session or apply the draft
- after apply, the user can route into the project blueprint view to inspect
  the canonical result
- local provider hardening, re-import semantics, and per-node review remain
  deferred

## Verification Plan

### Server

- import API tests for project-level review payload lookup
- apply API tests proving `review_pending` imports transition to `applied`
- blueprint integration tests proving applied nodes are project-scoped and
  attached to the project root
- idempotency tests proving re-apply does not duplicate canonical nodes or
  edges
- tests proving apply failure leaves the job reviewable rather than falsely
  applied
- tests proving the global `ProposalStore` remains untouched by apply

### Web

- `ProjectSessionsPage` tests for review-pending banner rendering
- `ProjectSessionsPage` tests for `Open Seeded Session` and `Apply Import Draft`
  actions
- `ProjectSessionsPage` tests for post-apply success state and blueprint
  navigation
- client typing tests for any new import review/apply payload shape

### Manual

- import a small GitHub repo, wait for `review_pending`, and confirm the
  project sessions page shows the review banner
- apply the import draft and confirm the project blueprint view shows the
  imported project-local nodes
- retry the apply action and confirm no duplicate canonical result appears

## Rollback / Fallback

- if canonical apply is unstable, keep the product at `review_pending` and do
  not surface a false applied state
- if project-level review UI becomes noisy, keep approval available through the
  minimal project sessions surface rather than inventing a second workflow
- if canonical dedupe proves riskier than expected, gate repeated apply via job
  status first and defer richer duplicate handling

## Open Questions

These are explicitly deferred and do not block this slice:

- whether later review/apply should support per-node cherry-picking
- whether imported edge proposals should become part of the same approval unit
- whether rejection/discard deserves its own explicit endpoint and UI
- how re-import should diff, version, or replace previously applied draft state
- how local provider hardening should share this same review/apply contract
