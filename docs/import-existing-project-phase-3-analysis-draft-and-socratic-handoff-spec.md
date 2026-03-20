# Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec

**Status:** Implemented  
**Date:** 2026-03-20  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)  
**Prior Slice:** [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md)

> Delivery update (2026-03-20): Phase 3 is now implemented across durable
> import-draft persistence, background checkout analysis, seeded project
> session creation, and `ProjectsPage` routing into the seeded session. The
> verification snapshot for this slice is recorded below.

## Objective

Advance `Import Existing Project` from source acquisition into a usable
analysis-driven planning handoff.

This slice should take a prepared local checkout, analyze it into
project-scoped import draft state, generate a planning brief, create a seeded
draft session for the project, and route the user into the existing Socratic
entry path.

It does **not** yet commit imported findings into the canonical blueprint.

## User Outcome

After a GitHub import finishes acquisition, Planner can:

- inspect the checked-out repo
- build a project-owned import draft from the discovered structure
- generate an initial planning brief from import findings
- create a draft session attached to the imported project
- prefill that session’s `project_description`
- expose the seeded session ID on the import job
- route the user into the existing session flow with imported context already
  attached

The user still does **not** get canonical blueprint mutation or final import
approval in this slice. Imported findings remain draft state pending a later
review/apply spec.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- Phase 2 acquisition remains the prerequisite source of a prepared local root
- import analysis writes to **project-scoped import draft state**, not the
  canonical blueprint and not the global `ProposalStore`
- the import job becomes multi-stage and truthful to actual work:
  - `queued`
  - `cloning`
  - `analyzing`
  - `review_pending`
  - `failed`
- `review_pending` means:
  - import draft state exists
  - planning brief exists
  - seeded draft session exists
  - no canonical blueprint commit has happened yet
- analysis remains intentionally conservative in this slice:
  - README or top-level doc summary when available
  - directory/component discovery
  - Cargo/workspace manifest scanning where currently supported
  - no broader provider expansion
  - no final review/apply workflow
- the UI reuses the existing session flow rather than requiring a full new
  import review surface in this slice

## Scope

### In scope

- extend import-job state to represent active analysis and review-pending
  readiness
- analyze a prepared checkout after acquisition succeeds
- add an import-draft persistence model owned by the project/import job
- write discovered draft records with explicit project scope
- persist an import-generated planning brief or analysis summary on the job or
  draft record
- create a draft session attached to the imported project
- prefill `session.project_description` from the import-generated planning brief
- persist `seed_session_id` on the import job
- expose the seeded session and draft-analysis metadata through
  `GET /projects/imports/{jobId}`
- update the projects import feedback flow so a completed import can route to
  the seeded session instead of stopping at the project page

### Out of scope

- committing imported findings into the canonical blueprint
- review/apply endpoints or approval UI
- local-path provider acquisition or hardening
- broader technology inference beyond the currently strong discovery paths
- branch selection, private GitHub auth, or re-import semantics
- project delete cleanup for import drafts beyond current lifecycle behavior
- a dedicated import dashboard or full import-review-first custom page

## Current-State Evidence

- Phase 2 already delivers managed GitHub checkout acquisition and truthful job
  progress in
  [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs),
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs),
  and [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx).
- Discovery and scanner logic already exist in
  [planner-core/src/discovery.rs](/home/thetu/planner/planner-core/src/discovery.rs),
  but the current product-facing discovery path writes into the global
  `ProposalStore`, which is the wrong ownership model for imports.
- Project-root and project-scope blueprint semantics already exist from
  [blueprint-project-root-codegraph-integration.md](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md),
  so draft records can be shaped with correct project scope without mutating
  canonical knowledge yet.
- Session creation and `project_description` persistence already exist in
  [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
  and [planner-server/src/session.rs](/home/thetu/planner/planner-server/src/session.rs).
- There is currently no import-draft store, no seeded-session creation in the
  import job flow, and no `seed_session_id` on import jobs.

## Requirements

### Analysis contract

Once a GitHub import checkout is available:

- the server advances the job from acquisition into `analyzing`
- analysis runs against the prepared local root from the source binding
- analysis in this slice should include:
  - README or top-level documentation summary when present
  - directory/component discovery
  - Cargo/workspace manifest scanning where currently supported
- analysis output must be shaped into project-owned import draft state rather
  than canonical blueprint state

The import draft should be strong enough to support later review/apply work
without requiring analysis to be rerun for every UI refresh.

### Import draft ownership contract

Add durable import draft state with these properties:

- owned by `project_id` and tied to the current import job
- explicitly project-scoped
- separate from:
  - the global `ProposalStore`
  - the canonical blueprint store
  - the final review/apply records that land later

Implementation may refine the exact structure, but the persisted draft surface
must capture enough to support later review/apply work:

- a concise analysis summary / planning brief
- discovered draft records or normalized draft artifacts
- source metadata needed to understand where the draft came from

### Import job contract

The import job should gain the fields needed for the handoff:

- `seed_session_id`
- `analysis_summary` or equivalent summary field

The status contract for this slice becomes:

- `queued`: request accepted, work not started
- `cloning`: source acquisition in progress
- `analyzing`: checkout exists and repo analysis is running
- `review_pending`: import draft exists and a seeded session is ready
- `failed`: acquisition or analysis stopped without a usable handoff

For GitHub imports, `GET /projects/imports/{jobId}` must return the current
job, source binding, and any seeded-session metadata needed by the UI.

### Session handoff contract

When analysis succeeds:

- create exactly one seeded draft session for the imported project
- attach the session to the canonical project
- prefill `session.project_description` from the import-generated planning brief
- persist the resulting `seed_session_id` on the import job

The seeded session must route into the existing session experience. This slice
does not require a brand-new import-specific page, but it should leave room for
later import-review-first UX refinement.

### UI contract

The projects import flow should remain lightweight but truthful:

- continue using the latest-import feedback surface on `ProjectsPage`
- surface `analyzing` and `review_pending` states without implying canonical
  blueprint approval has happened
- when the job reaches `review_pending` and has a `seed_session_id`, offer a
  direct route into the seeded session
- avoid language that suggests imported draft findings are already canonical

### Failure behavior

Analysis failures must be explicit and durable:

- if acquisition succeeds but analysis fails, the job moves to `failed`
- failure messaging should make clear whether the failure was during clone or
  analysis
- partial import draft state should not be surfaced as review-ready unless the
  job reaches `review_pending`

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-core/src/discovery.rs](/home/thetu/planner/planner-core/src/discovery.rs)
- new `planner-core/src/import.rs`
- [planner-core/src/blueprint.rs](/home/thetu/planner/planner-core/src/blueprint.rs)
- [planner-server/src/import.rs](/home/thetu/planner/planner-server/src/import.rs)
- [planner-server/src/api.rs](/home/thetu/planner/planner-server/src/api.rs)
- [planner-server/src/session.rs](/home/thetu/planner/planner-server/src/session.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-web/src/api/client.ts](/home/thetu/planner/planner-web/src/api/client.ts)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/pages/ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [planner-web/src/pages/__tests__/ProjectsPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectsPage.test.tsx)
- [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)

Execution should stay bounded to import draft generation and seeded-session
handoff. Review/apply and later hardening stay out.

## Acceptance Criteria

- a successful GitHub import advances beyond acquisition into `analyzing` and
  then `review_pending`
- the system persists project-owned import draft state without mutating the
  canonical blueprint or the global `ProposalStore`
- the import job persists a planning brief / analysis summary plus
  `seed_session_id`
- the seeded session is attached to the imported project and carries a prefixed
  `project_description` generated from import findings
- `GET /projects/imports/{jobId}` returns the seeded-session metadata required
  by the UI
- the projects UI can route the user into the seeded session once the job
  reaches `review_pending`
- no review/apply endpoint, canonical blueprint commit, or local-provider
  hardening is introduced in this slice

## Verification Plan

### Server

- import-flow tests for `analyzing` -> `review_pending` transitions
- persistence tests for import draft state and seeded-session metadata
- integration tests proving import draft state is project-owned and not written
  into the global `ProposalStore`
- integration tests proving seeded sessions preserve project identity and
  `project_description`
- failure-path tests for analysis errors after successful acquisition

### Web

- `ProjectsPage` tests for `analyzing` and `review_pending` status messaging
- `ProjectsPage` tests for routing into the seeded session once available
- `SessionPage` tests for seeded import sessions carrying the imported planning
  brief

### Manual

- import a small GitHub repo and verify the job reaches `review_pending`
- confirm the resulting seeded session opens with the imported project context
- confirm imported findings are not yet committed to canonical blueprint views

## Verification Snapshot (2026-03-20)

Passed:

- `cargo test -p planner-server project_import -- --nocapture`
- `cargo test -p planner-server github_import -- --nocapture`
- `cargo test -p planner-server import_store -- --nocapture`
- `cargo test -p planner-server archive_project -- --nocapture`
- `npm --prefix planner-web test -- --run src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/SessionPage.test.tsx`

## Rollback / Fallback

- if project-scoped import draft persistence proves unstable, stop at Phase 2
  acquisition rather than surfacing misleading review-ready state
- if seeded-session creation is unreliable, keep the import job from reaching
  `review_pending` and preserve a clear failure message instead

## Open Questions

These are explicitly deferred and do not block this slice:

- the exact review/apply API and UI that will promote import draft state into
  canonical blueprint knowledge
- how local-path imports should reuse the same draft/handoff pipeline
- whether later re-import should replace, diff, or version existing import
  draft state
