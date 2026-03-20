# Import Existing Project Plan

**Status:** Research complete, source plan for execution specs  
**Date:** 2026-03-18

> Planning sync update (2026-03-20): this document remains the source research
> plan for the import feature. The canonical execution artifacts are now
> [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)
> and
> [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md),
> with
> [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)
> now implemented as the current analysis-and-handoff slice, and
> [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)
> now implemented as the explicit canonical blueprint approval slice. Remaining
> import work should be promoted through follow-on specs rather than reopening
> this source research plan as an execution artifact. The next ready execution
> artifact is now
> [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md),
> which is now implemented as the local-path provider parity slice on top of
> the existing import pipeline. Remaining work should be promoted through
> follow-on specs for re-import and lifecycle cleanup. The next ready execution
> artifact is now
> [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md),
> which is now implemented as the explicit project-level re-import,
> duplicate-source protection, and import-owned delete-cleanup slice.
> Remaining work should be promoted through follow-on specs for import history
> and richer diff/reconciliation behavior rather than reopening completed Phase
> 6 work. The next ready execution artifact is now
> [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md),
> which scopes project-scoped import history plus lightweight draft-vs-last-
> applied comparison without yet reopening rollback or canonical reconciliation
> semantics. That slice is now implemented. Remaining work should be promoted
> through a follow-on spec for richer reconciliation behavior against canonical
> blueprint state. The next ready execution artifact is now
> [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md),
> which scopes apply-time reconciliation of import-owned project-local
> blueprint state without yet introducing rollback or destructive hard-delete
> semantics. That slice is now implemented. Remaining work should be promoted
> through a later bounded spec for rollback and more advanced reconciliation
> behavior against historical imports rather than reopening completed Phase 8
> work. The next ready execution artifact is now
> [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md),
> which scopes restore to an older applied import through the existing
> project-scoped history surface without yet reopening per-node merge or broad
> time-travel semantics. That slice is now implemented. Remaining work should
> be promoted through a later bounded spec for broader historical
> reconciliation beyond applied-import restore rather than reopening completed
> Phase 9 work. The next ready execution artifact is now
> [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md),
> which scopes reopening an older historical `review_pending` draft into the
> current review slot without yet introducing merge controls or direct blueprint
> mutation at restore time. That slice is now implemented. Remaining work
> should be promoted through a later bounded spec for more advanced merge
> controls and broader historical reconciliation rather than reopening
> completed Phase 10 work.

## Findings

### Current architecture already gives us the project and session backbone

- The canonical product project already exists in the server as a persisted
  `Project` with UUID, slug, description, and legacy aliases in
  `planner-server/src/project.rs`.
- The main project endpoints already exist in `planner-server/src/api.rs`:
  - `POST /projects`
  - `GET /projects`
  - `POST /projects/{projectRef}/sessions`
- The web app is already project-first at the shell level:
  - `HomeHubPage` routes users into projects
  - `ProjectsPage` is the directory
  - `ProjectSessionsPage` is the canonical project-local session surface
- The existing Socratic lobby flow is session-based, not project-based:
  - `/session/new?project=<slug>` creates a draft session attached to a project
  - `SessionPage` renders the waiting lobby and starts the interview
  - `POST /sessions/{id}/socratic` transitions the session into Socratic intake

### The import-adjacent subsystem already exists, but it is not productized

- The discovery API already scans an arbitrary filesystem root via
  `POST /blueprint/discovery/scan` with an optional `root_path`.
- The scanner pipeline already supports:
  - cargo dependency discovery
  - directory/component discovery
  - code-graph edge proposal import
- The code-graph path already understands both Cargo and npm workspace
  manifests.

This is the strongest existing reuse point for import analysis.

### The current discovery flow is the wrong abstraction to expose directly as “Import GitHub”

Discovery today is a global utility surface, not a project-owned import flow:

- proposals are stored in a global `ProposalStore`
- list/accept/reject routes are not scoped to a project
- `DiscoveryPage` has no project context and no `root_path` input
- scans currently emit **unscoped** nodes (`NodeScope::default()`), so naïvely
  accepting import-generated proposals would merge unscoped records into the
  global blueprint

That is the main architectural gap.

### Blueprint and project scoping are already strong enough for imported projects

- Pipeline blueprint emission already creates project root nodes and `contains`
  edges.
- `BlueprintStore` already backfills project roots and project scope.
- Project deletion already purges project-local blueprint state and unlinks
  shared nodes.

So once import analysis can emit **project-scoped** blueprint records, the
existing blueprint/project model can carry the imported project cleanly.

### There is no existing source acquisition model

The repo has no first-class concept of:

- GitHub repo URL
- local repo path
- branch or revision selection
- clone/check-out location
- import job state
- re-import/sync history

The current `Project` model is product metadata only. It is not yet a source
binding.

### Existing partial support that matters

- `run_discovery_scan` already accepts an arbitrary `root_path`, so local repo
  import later is compatible with current analysis machinery.
- `collect_code_graph_edge_proposals` already works from a local checkout path,
  not from a GitHub API object.
- `ProjectSessionsPage` already routes to `/session/new?project=<slug>`, which
  is the clean handoff point once import analysis has created the project.

### Existing partial support that is insufficient

- There is no GitHub clone/auth flow.
- There is no import-specific job runner or status polling.
- Discovery node proposals are not project-scoped.
- Edge proposal generation depends on accepted filesystem-backed components and
  matching technology nodes, so it cannot be the first analysis step.
- Technology discovery is currently cargo-centric; npm package relationships
  can be inferred by code graph tooling, but npm dependencies do not yet become
  technology nodes from scanning alone.

## Recommended Design

### Product framing decision

Use **Import Existing Project** as the product concept, with **GitHub** as the
first provider.

Reasoning:

- the real reusable boundary in the current codebase is a **filesystem root**
  passed into discovery and analysis
- GitHub is only one way to acquire that root
- a GitHub-only product model would force source-specific fields into the
  project model too early
- local repo import for `~/recipes` becomes straightforward if acquisition is
  separated from analysis

The initial UI can still present GitHub first, but the architecture should not
be GitHub-shaped.

### Recommended architecture

Build the feature in four layers:

1. **Source acquisition**
   - Input: `ImportSource`
   - v1 provider: `GitHubRepository`
   - later provider: `LocalGitRepository`
   - Output: `PreparedImportSource { local_root, source_metadata }`

2. **Import job orchestration**
   - New server-owned job model for long-running clone + analysis
   - Responsible for status, progress, errors, and idempotency

3. **Repo analysis**
   - Reuse the existing discovery/code-graph machinery
   - Do **not** reuse the global `ProposalStore` as the primary import state
   - Instead, run scanners ephemerally and merge project-scoped results into
     the blueprint directly

4. **Socratic handoff**
   - Create a draft session attached to the imported project
   - Prefill `project_description` with an import-generated planning brief
   - Route the user into the existing waiting lobby on `SessionPage`

### Why not use the current discovery proposal queue directly

Two plausible options exist:

#### Option A: import by reusing the current discovery proposal queue

Pros:

- lowest short-term code reuse cost
- existing scanners and proposal acceptance code already exist

Cons:

- proposals are global, not project-owned
- scanner output is currently unscoped
- import would require either user review before readiness or hidden
  auto-accept behavior on a queue designed for manual review
- multi-user contamination risk remains
- poor fit for eventual local repo import and re-import

#### Option B: dedicated import job plus project-scoped blueprint merge

Pros:

- matches the user outcome directly
- keeps imported analysis owned by the project from the start
- reuses scanner logic without inheriting discovery UI limitations
- future-safe for GitHub and local repo providers
- cleaner place to add re-import semantics later

Cons:

- requires a new import job/store/orchestrator
- requires refactoring some scanner/acceptance logic for reuse

**Recommendation:** choose Option B.

### Proposed server/domain model

Keep `Project` as the canonical product container and add a separate import
model rather than overloading `Project`.

Recommended new records:

```rust
enum ImportProvider {
    GitHub,
    LocalGit,
}

enum ImportStatus {
    Queued,
    Cloning,
    Analyzing,
    Ready,
    Failed,
}

struct ProjectSourceBinding {
    project_id: Uuid,
    provider: ImportProvider,
    canonical_ref: String,      // normalized GitHub URL or absolute local path
    default_branch: Option<String>,
    head_revision: Option<String>,
    local_root: Option<String>,  // managed clone path or validated external path
    managed_checkout: bool,
    created_at: String,
    updated_at: String,
}

struct ProjectImportJob {
    id: Uuid,
    project_id: Uuid,
    provider: ImportProvider,
    requested_ref: String,
    status: ImportStatus,
    seed_session_id: Option<Uuid>,
    progress_message: Option<String>,
    error_message: Option<String>,
    analysis_summary: Option<String>,
    created_at: String,
    updated_at: String,
}
```

Why separate records:

- import is long-running and retryable
- a project can later support re-import
- the project record should stay stable even if import jobs fail or rerun

### Recommended API shape

Add import APIs alongside the current project APIs:

- `POST /projects/imports`
  - starts an import
  - request includes `provider` and source reference
  - response returns `{ project, import_job }`
- `GET /projects/imports/{jobId}`
  - returns status and any created `seed_session_id`
- `POST /projects/{projectRef}/reimport`
  - later phase; not required in MVP

Do not put GitHub-specific semantics in the route path. Keep the provider in
the request body.

Example v1 request:

```json
{
  "provider": "github",
  "repo_url": "https://github.com/org/repo"
}
```

### Recommended UI placement

Primary entry points:

- `ProjectsPage`
- `HomeHubPage`

Reasoning:

- import creates a project container first
- the imported project becomes a project-local session/knowledge/blueprint
  workspace
- `SessionPage` is the wrong entry because it assumes the project already
  exists and the user is entering the Socratic brief step

Recommended first-cut UX:

- add `Import Existing Project` next to `New Project`
- open a modal with provider choice
- in v1, only GitHub is enabled
- after submission, show import job progress and then route to the seeded
  session lobby or project sessions page

### Recommended import pipeline

From GitHub URL to Socratic readiness:

1. User clicks `Import Existing Project`.
2. User pastes a GitHub repo URL.
3. Server normalizes the URL into a canonical source reference.
4. Server creates the canonical `Project`.
5. Server creates an import job linked to that project.
6. Server acquires a local checkout root.
   - v1 GitHub: clone default branch into managed storage
7. Server analyzes the checkout.
   - read README or top-level docs when present
   - run directory/component discovery
   - run manifest scanning
   - run code-graph edge inference only after node candidates are mergeable
8. Server merges analysis into blueprint as **project-scoped** nodes/edges.
   - create or reuse project root
   - attach discovered nodes via `contains`
9. Server generates an initial planning brief for Socratic intake.
   - based on repo metadata, README summary, and discovered structure
10. Server creates a draft session for that project with
    `project_description` prefilled.
11. Import job is marked `ready`.
12. UI routes the user to the seeded session.
13. User enters the existing Socratic lobby with the imported project context
    already attached.

### Minimal viable import flow

Ship this first:

- provider support:
  - **public GitHub repos**
  - **local repo import**
- branch support: default branch only
- auth: none in v1
- acquisition:
  - GitHub: managed clone under Planner data
  - local import: validated absolute local path
- analysis:
  - directory/component scan
  - existing Cargo scanner when applicable
  - optional README summary extraction
- merge behavior:
  - create a project-scoped import draft
  - no auto-merge into canonical blueprint
  - require user review before canonical blueprint commit
- Socratic handoff:
  - create one seeded draft session
  - prefill planning brief from import findings
  - let the user begin from an import-review-first lobby

Do **not** block MVP on:

- private repo auth
- branch picker
- re-import diffing
- bidirectional sync
- project-scoped discovery UI redesign
- broader provider expansion beyond GitHub + local path

### Future-safe path for local repo import

Local repo support is part of v1 and should reuse exactly the same pipeline
after source acquisition.

The only provider-specific difference should be:

- GitHub provider: clone to managed path
- local provider: validate an existing absolute path and use it directly

Everything after “produce a local filesystem root” should be shared:

- analysis
- project-scoped blueprint merge
- summary generation
- seed session creation

That is why the architecture should pivot on `PreparedImportSource.local_root`
instead of on GitHub-specific repository objects.

### Recommended core refactor

Refactor discovery so import can reuse the scanner logic without writing to the
global proposal store or directly into the canonical blueprint:

- keep `scan_cargo_toml` and `scan_directory_structure`
- add an import-oriented draft builder that:
  - takes proposed nodes/edges
  - applies project scope
  - normalizes component naming
  - writes to a project-scoped import draft store
- add an explicit review/apply step that commits approved findings into the
  canonical blueprint

This avoids duplicating scanner logic while keeping import state out of both
the global review queue and the canonical blueprint until the user reviews it.

### Known risks

#### Auth

- private GitHub repos require a product decision:
  - public-only first
  - per-user OAuth/device flow
  - service token

This should remain out of MVP unless explicitly approved.

#### Clone location and storage

- managed clones increase storage and cleanup burden
- ephemeral clones reduce reproducibility and make re-import harder
- local repo support requires the model to distinguish managed vs external
  roots

#### Repo size and scan time

- large repos can exceed request timeouts
- background jobs are the safe default
- impose repo/file-size limits and ignore rules early

#### Branch selection

- default branch only is acceptable for MVP
- explicit branch selection should be a later enhancement unless product
  requires review of non-default branches immediately

#### Re-import and idempotency

- repeated imports of the same source must not mint duplicate projects by
  accident
- the system should normalize source URLs and detect “same repo imported again”
- re-import should later become an explicit project action, not a second hidden
  project creation path

#### Cleanup

- project delete must eventually remove managed clone state and import job
  records
- local path imports must never delete the user’s source repo

#### Multi-user isolation

- the current discovery proposal store is global and not safe as the main
  import state for multi-user product flows

#### Analysis quality

- current manifest scanning is stronger for Cargo than npm tech discovery
- imported JS/TS repos will need better dependency-to-technology mapping for a
  fuller architectural picture
- a review-first import flow reduces the risk of weak scanner output becoming
  false canonical project knowledge

## Implementation Plan

### Phase 1: Import Domain Skeleton

**Goal**

Introduce a project-owned import job model and provider abstraction without yet
shipping full GitHub analysis UX.

**Likely files/modules**

- `planner-server/src/api.rs`
- new `planner-server/src/import.rs`
- `planner-server/src/lib.rs`
- `planner-server/src/main.rs`
- `planner-server/src/project.rs`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`

**API/UI/core/data model changes**

- add `ProjectImportJob` and `ProjectSourceBinding`
- add `POST /projects/imports` and `GET /projects/imports/{jobId}`
- add shared provider enums and status types
- no broad UI yet beyond minimal modal + pending state

**Validation strategy**

- server tests for job creation and state transitions
- serialization/persistence tests for new import records
- web client tests for API typing and modal submission

This phase is now tracked canonically in
[Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md).

### Phase 2: GitHub Acquisition And Managed Checkout

**Goal**

Support public GitHub URL import through a background job that clones the
default branch into managed storage.

**Likely files/modules**

- new `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-server/src/project.rs`
- `planner-server/src/main.rs`

**API/UI/core/data model changes**

- normalize GitHub URLs
- clone repo into managed storage
- persist provider metadata, default branch, and head revision
- expose job progress in API

**Validation strategy**

- unit tests for URL normalization
- integration tests against a local temp git repo simulating a remote source
- failure-path tests for invalid URLs and clone errors

### Phase 3: Analysis Merge And Socratic Handoff

**Goal**

Turn a prepared checkout into a project-scoped import draft plus a seeded draft
session that opens in an import-review-first Socratic flow.

**Likely files/modules**

- `planner-core/src/discovery.rs`
- new `planner-core/src/import.rs`
- `planner-core/src/blueprint.rs`
- `planner-core/src/pipeline/blueprint_emitter.rs`
- `planner-server/src/api.rs`
- new `planner-server/src/import.rs`
- `planner-server/src/session.rs`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/HomeHubPage.tsx`
- `planner-web/src/pages/SessionPage.tsx`

**API/UI/core/data model changes**

- refactor scanner reuse away from global proposal persistence
- write imported results into project-scoped import draft state
- add review/apply endpoints for committing approved import findings
- create seeded session with `project_description`
- return `seed_session_id` when import is ready
- route UI to `/session/{id}`

**Validation strategy**

- server integration tests:
  - import creates project
  - import creates project-scoped import draft records
  - import creates seeded session
  - approving import findings writes canonical blueprint nodes
  - starting Socratic on seeded session preserves project identity
- web tests for import CTA, progress state, review state, and completion flow

### Phase 4: Hardening, Re-import, And Local Provider

**Goal**

Make the feature durable enough for real use and prepare the local repo path.

**Likely files/modules**

- `planner-server/src/import.rs`
- `planner-server/src/api.rs`
- `planner-server/src/project.rs`
- `planner-core/src/discovery.rs`
- `planner-core/src/blueprint.rs`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/ProjectsPage.tsx`

**API/UI/core/data model changes**

- explicit re-import endpoint
- duplicate-source detection and idempotency rules
- cleanup on project delete
- optional local repo provider
- optional branch support
- optional import history/status on project pages

**Validation strategy**

- project delete cascade tests include clone cleanup
- re-import tests prove no duplicate project corruption
- local path tests use temp git repos
- manual validation against `~/recipes`

## Open Decisions

The following choices need explicit approval before implementation:

1. **Product framing**
   - Approve `Import Existing Project` with GitHub as the first provider
   - Reject GitHub-only product framing unless you want a later refactor

2. **GitHub auth model for v1**
   - Public repos only
   - or private repo support now via a chosen auth path

3. **Clone/storage policy**
   - Managed clone under Planner data dir
   - or ephemeral clone per import
   - or direct external-path binding for some providers

4. **Analysis merge policy**
   - Auto-merge imported analysis into project-scoped blueprint
   - or require a review step before the project becomes “ready”

5. **Socratic handoff**
   - Auto-create a seeded draft session and open an import-review-first lobby
   - or finish import on the project page and make the user start the session
     manually after reviewing import findings

6. **Local repo timeline**
   - Must ship in v1
   - local path validation and import UX should therefore be in the initial
     provider design

7. **Re-import semantics**
   - same source imported twice should reopen/reuse the existing project
   - or create a new project every time unless the user chooses refresh

## Concrete integration points

### Existing creation and Socratic flow

- `planner-server/src/api.rs`
  - `create_project`
  - `create_project_session`
  - `create_session`
  - `start_socratic`
  - `run_pipeline_for_session`
- `planner-server/src/project.rs`
  - `ProjectStore::resolve_ref`
  - `ProjectStore::create`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/HomeHubPage.tsx`
- `planner-web/src/pages/ProjectSessionsPage.tsx`
- `planner-web/src/pages/SessionPage.tsx`

### Existing analysis/discovery flow

- `planner-server/src/api.rs`
  - `run_discovery_scan`
  - `accept_proposal`
  - `accept_edge_proposal`
- `planner-core/src/discovery.rs`
  - `scan_cargo_toml`
  - `scan_directory_structure`
  - `collect_code_graph_edge_proposals`
  - `collect_workspace_packages`
- `planner-web/src/pages/DiscoveryPage.tsx`

### Existing blueprint/project scoping flow

- `planner-core/src/pipeline/blueprint_emitter.rs`
  - `ensure_project_root_node`
  - `emit_from_intake`
  - `emit_from_spec`
- `planner-core/src/blueprint.rs`
  - `backfill_project_root_nodes`
  - `backfill_project_scope_from_contains_edges`
  - `purge_project`
