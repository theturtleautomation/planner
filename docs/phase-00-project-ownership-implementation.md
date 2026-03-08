# Phase 00 Project Ownership Implementation

**Status:** Research complete, ready for implementation  
**Date:** 2026-03-07

## Objective

Define a first-class product `Project` model that becomes the parent container
for sessions, blueprint knowledge, scoped knowledge views, and project-level
activity.

This phase is complete when routing, API, and migration work can be estimated
against one canonical project definition instead of multiple incompatible
"project-like" identifiers.

## Non-Goals

- redesign the root landing page
- redesign the sidebar or homepage IA
- rename blueprint taxonomy or node labels
- redesign the knowledge filter bar
- redesign the session lobby or events panel
- implement team ACL or multi-user collaboration beyond basic ownership fields
- make projects duplicable or branchable product objects in this phase

## Decision Summary

- Introduce a persisted product `Project` entity in the server as the sole
  user-facing project container.
- Use **both** UUIDs and slugs:
  - UUID is the canonical internal identifier.
  - slug is the route key and human-friendly deep-link key.
- Do **not** use existing knowledge `project_id` strings as the source of
  truth. They become legacy aliases that map onto canonical projects.
- Every session must belong to exactly one project once intake begins.
- The pipeline must stop minting a new user-facing project identity per run.
  It should run under the session's canonical product project UUID.
- Project pages should be the top-level working surface and expose
  `Sessions`, `Blueprint`, `Knowledge`, and `Events`.
- Duplicate and branch remain session or knowledge actions, not project
  actions.
- Project archive is deferred until the product confirms it needs project-level
  hiding or read-only behavior.

## Current-State Summary

There is no single project model today. The codebase currently uses multiple
incompatible project concepts:

| Surface | Current field or model | Shape | Current problem |
| --- | --- | --- | --- |
| Session draft | `Session.project_description` | free text | description is not a durable project relationship |
| Session pipeline linkage | `Session.cxdb_project_id` | UUID | created per pipeline run, not a product container |
| Pipeline artifacts | `IntakeV1.project_id`, `NLSpecV1.project_id` | UUID | internal pipeline identity, not exposed as the main web project |
| Knowledge scope | `NodeScope.project.project_id` | string | used by the UI as if it were canonical, but not backed by a project entity |
| Knowledge routing | `/knowledge/projects/:projectId` | string route param | assumes a stable project key already exists |
| Runtime registry | `ProjectRegistry` | in-memory UUID + slug | ephemeral runtime helper, not persisted and not connected to web routes |

### Current code anchors

- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-core/src/pipeline/mod.rs`
- `planner-core/src/pipeline/project.rs`
- `planner-core/src/pipeline/blueprint_emitter.rs`
- `planner-core/src/blueprint.rs`
- `planner-web/src/types.ts`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/lib/knowledgeDeepLinks.ts`
- `docs/knowledge-library-project-scope-plan.md`

### Code findings that make the current state unsafe

- `POST /sessions` creates a session with no project input and no project
  relationship.
- `POST /sessions/:id/socratic` stores only `project_description`, then starts
  the interview.
- `run_pipeline_for_session()` generates a fresh UUID and stores it in
  `cxdb_project_id`, which means the main durable project identifier is minted
  after the session already exists.
- `KnowledgeLibraryPage` builds project cards by aggregating blueprint node
  scope fields instead of querying a canonical project store.
- `BlueprintStore` is global at `data_dir/blueprint/`, not project-owned.

### Critical mismatch discovered in Phase 0

The blueprint emitter already produces two different project identifiers for
the same logical project:

- `emit_from_intake()` uses `proj-{slug(project_name)}`
- `emit_from_spec()` uses `spec.project_id.to_string()` where `spec.project_id`
  is a UUID

That means one project can already fragment into multiple project buckets in
the knowledge UI. Existing knowledge `project_id` strings are therefore not
reliable enough to become the product source of truth.

### Visual audit findings from 2026-03-07

- `/knowledge` already behaves like a user-facing project chooser with
  project cards, health summaries, counts, and explicit `Open Project View`
  actions, which makes the lack of a canonical `Project` model visible in the
  product rather than purely architectural.
- The current project chooser already exposes mixed identities side by side:
  one card is titled with a UUID-like identifier and routes to
  `/knowledge/projects/afc2fb66-...`, while another uses the human-readable
  slug `proj-personal-task-tracker`. Project identity fragmentation is
  therefore already user-facing.
- `/` and `/session/new` still present session-first chrome and copy, so users
  move between two conflicting top-level concepts: sessions as the product
  root versus projects as the knowledge root.

## Proposed Behavior

### Canonical product model

Add a persisted `Project` entity owned by the web product:

```ts
interface Project {
  id: string;                 // UUID, canonical internal ID
  slug: string;               // unique route key
  name: string;
  description?: string | null;
  owner_user_id: string;
  team_label?: string | null;
  created_at: string;
  updated_at: string;
  archived_at?: string | null; // reserved, not enabled in phase 0 UX
  legacy_scope_keys: string[]; // existing knowledge aliases like "proj-alpha"
}
```

### Ownership rules

- A project owns many sessions.
- A project owns one project-scoped blueprint view.
- A project owns one project-scoped knowledge view.
- A project owns project-level activity and event history.
- A session belongs to exactly one project after the interview starts.
- A session may remain temporarily unassigned only during the current
  transitional `/session/new` draft flow, and must be assigned before the
  pipeline starts.

### Identifier strategy

#### Canonical ID

- `Project.id` is a UUID.
- This UUID becomes the canonical project identifier across:
  - server APIs
  - session records
  - pipeline artifact `project_id` values
  - CXDB project indices
  - blueprint node project scope

#### Route key

- `Project.slug` is the human-facing route key.
- New project routes should use slug-first URLs.
- Server lookup should accept either UUID or slug during the migration window.

#### Legacy knowledge aliases

- Existing knowledge scope strings such as `proj-alpha` do not remain
  canonical.
- They are stored in `Project.legacy_scope_keys`.
- The server resolves legacy aliases to `Project.id` during backfill and
  transitional reads.

### Session behavior

- The canonical session relationship is `Session.project_id`.
- `Session.project_description` remains the saved prompt or description, not
  the project relationship.
- `POST /projects/:projectRef/sessions` becomes the canonical creation path.
- The old `POST /sessions` path remains transitional and should accept
  `project_id` as soon as the UI is updated.
- By the end of the migration, starting Socratic intake without a resolved
  project must be rejected.

#### Session movement

- Allow moving a session between projects only while the session is still a
  draft or pre-pipeline interview.
- Do not support moving completed pipeline sessions in the first cut.
- If later enabled, move semantics must define what happens to blueprint nodes,
  knowledge links, and run history. That is intentionally deferred here.

### Pipeline and CXDB behavior

- Stop minting a new product-facing project UUID inside
  `run_pipeline_for_session()`.
- Use the session's canonical `project_id` when launching pipeline work.
- Keep `run_id` as the unique pipeline execution identifier.
- Add a session-to-run index so session pages can query only the runs created
  from that session, even though CXDB is now indexed by canonical product
  project UUID.

#### Naming cleanup

- `cxdb_project_id` is misleading once `project_id` becomes the real product
  project. Replace it with one of:
  - `project_id` for the canonical project relationship
  - `latest_run_id` or `run_ids` for session-owned execution history

### Blueprint and knowledge behavior

- Blueprint node scope must reference the canonical product project UUID, not
  ad hoc strings.
- Project cards in the knowledge library must come from the Projects API, not
  from raw node aggregation.
- Blueprint and knowledge routes should resolve project context from the
  canonical project record, then filter nodes within that context.
- The current global blueprint store can remain temporarily while APIs are
  normalized, but the target storage shape is project-owned state under
  `data/projects/{project_id}/...`.

### Project page shape

Project pages should become the new top-level working surface and expose these
tabs:

- `Sessions`
- `Blueprint`
- `Knowledge`
- `Events`

An optional `Overview` tab can exist later, but it is not required to establish
ownership.

## Impacted Files And Modules

### Server

- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-server/src/lib.rs`
- `planner-server/src/main.rs`
- new: `planner-server/src/project.rs`

### Core and schemas

- `planner-core/src/pipeline/mod.rs`
- `planner-core/src/pipeline/project.rs`
- `planner-core/src/pipeline/blueprint_emitter.rs`
- `planner-core/src/blueprint.rs`
- `planner-core/src/cxdb/durable.rs`
- `planner-schemas/src/artifacts/intake.rs`
- `planner-schemas/src/artifacts/blueprint.rs`

### Web

- `planner-web/src/types.ts`
- `planner-web/src/api/client.ts`
- `planner-web/src/App.tsx`
- `planner-web/src/components/Layout.tsx`
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/pages/BlueprintPage.tsx`
- `planner-web/src/pages/EventTimelinePage.tsx`
- `planner-web/src/lib/knowledgeDeepLinks.ts`

## API And Data Model Changes

### New project API surface

- `GET /projects`
- `POST /projects`
- `GET /projects/:projectRef`
- `PATCH /projects/:projectRef`
- `GET /projects/:projectRef/sessions`
- `POST /projects/:projectRef/sessions`
- `GET /projects/:projectRef/events`

`projectRef` should resolve in this order during migration:

1. UUID
2. slug
3. legacy knowledge alias

### Session API changes

- Add `project_id`, `project_slug`, and `project_name` to `Session` and
  `SessionSummary`.
- Keep `project_description` as the saved description field.
- Deprecate relying on `/sessions/:id/runs` and `/sessions/:id/turns` through
  `cxdb_project_id`; switch those endpoints to the session-to-run index.

### Blueprint and knowledge API changes

- Add a canonical Projects API lookup so blueprint and knowledge views can
  resolve project metadata without inferring it from nodes.
- Keep `/blueprint?project_id=...` and `/knowledge/projects/:projectRef`
  temporarily, but resolve them through canonical project lookup.
- Add project summary payloads for counts and health so the project chooser no
  longer derives projects directly from node IDs.

### Schema changes

#### Session

Add:

- `project_id?: Uuid`
- `project_slug?: String`
- `project_name?: String`
- `run_ids?: Vec<Uuid>` or equivalent session-to-run relation

Remove or rename:

- `cxdb_project_id`

#### Project scope inside blueprint

Continue exposing `project_id`, but make it the canonical product project UUID
string. Do not keep using mixed slug-style IDs and UUIDs.

If route generation needs to avoid extra lookups, add optional denormalized
metadata:

- `project_slug?: string`
- `project_name?: string`

That denormalization is optional and should not replace canonical lookup.

## UI And Routing Changes

### Canonical route map

- `/projects`
- `/projects/:projectSlug`
- `/projects/:projectSlug/sessions`
- `/projects/:projectSlug/blueprint`
- `/projects/:projectSlug/knowledge`
- `/projects/:projectSlug/events`

### Transitional compatibility routes

- `/session/:id` remains valid but should resolve the session's project context
  and eventually redirect into the project session surface.
- `/knowledge/projects/:projectRef` remains valid but should resolve through
  the canonical Projects API.
- `/blueprint` remains valid during the transition, but it should become a
  project-aware surface rather than a truly global default.

### UI contract changes

- Project selection or creation must happen before a session becomes an active
  planning run.
- Session cards on the current dashboard need project metadata so they can be
  grouped or linked back to the owning project.
- Blueprint, Discovery, and Event Timeline knowledge deep links must generate
  links from canonical project lookup, not from raw node scope strings alone.

## Migration And Backfill Plan

### Step 1: Add project persistence and alias resolution

- Create a `ProjectStore` in the server.
- Persist canonical projects with UUID, slug, and `legacy_scope_keys`.
- Add lookup helpers that resolve UUID, slug, or legacy alias.

### Step 2: Backfill canonical projects

- Seed canonical projects from existing knowledge scope keys where possible.
- Seed additional projects from existing sessions that cannot be deterministically
  mapped to knowledge scope.
- Mark backfilled records with a migration source such as:
  - `knowledge_alias`
  - `session_seed`
  - `cxdb_seed`

### Step 3: Backfill session ownership

- Add `project_id` to every session file.
- If a session has no deterministic project match, create a dedicated migrated
  project for it rather than guessing.
- Preserve `project_description` unchanged.

### Step 4: Normalize blueprint scope

- Resolve every blueprint node's `scope.project.project_id` through the alias
  map.
- Rewrite nodes to canonical project UUID strings.
- Preserve the old string in project alias metadata, not in the node as the
  long-term canonical key.

### Step 5: Normalize pipeline and CXDB usage

- Stop creating a fresh UUID in `run_pipeline_for_session()`.
- Use `Session.project_id`.
- Introduce the session-to-run index so session history remains session-local.

### Step 6: Cut the UI over to Projects API

- Replace knowledge-library project inference with project summaries returned by
  the server.
- Add project metadata to session payloads.
- Shift navigation and creation flows in the later UI phases.

### Migration safety rules

- Never merge legacy projects by name alone.
- Prefer alias or UUID evidence over fuzzy text similarity.
- If a migration cannot prove a match, create a dedicated migrated project and
  surface manual reassignment later.

## Tests To Add Or Update

### Server

- project CRUD and lookup by UUID, slug, and legacy alias
- session creation with required project ownership
- transitional session creation and assignment before intake starts
- session-to-run indexing and session-scoped run queries
- migration tests for mixed `proj-*` and UUID blueprint scope IDs
- blueprint scope normalization and alias resolution

### Core

- blueprint emitter tests that assert a single canonical project ID is used
  across intake and spec emissions
- pipeline tests that ensure the canonical product project UUID is reused
  across artifacts and CXDB registration

### Web

- knowledge-library project chooser powered by Projects API rather than raw
  node aggregation
- deep-link generation from Blueprint, Discovery, and Event Timeline with slug
  resolution
- session page and dashboard rendering of project metadata
- legacy route compatibility for `/session/:id` and
  `/knowledge/projects/:projectRef`

## Risks, Dependencies, And Rollout Order

### Main risks

- existing knowledge scope IDs are already inconsistent inside one pipeline run
- project migration cannot safely infer ownership from freeform descriptions
- changing CXDB project indexing without a session-to-run relation will break
  session-local history views
- project routes will remain awkward until the Phase 1 navigation work lands

### Dependencies

- a new persisted project store on the server
- a migration path for session MessagePack files
- alias-aware project lookup before the UI can switch routes safely
- blueprint normalization before project cards can be fully trusted

### Recommended rollout order

1. Land `ProjectStore`, project APIs, and alias resolution.
2. Add `project_id` to session records and backfill sessions safely.
3. Switch pipeline and CXDB to canonical project UUID plus session-to-run
   indexing.
4. Normalize blueprint node project scope.
5. Cut Knowledge Library and project chooser over to Projects API.
6. Use the new project routes in Phase 1 and later UI phases.

## Unresolved Questions

- Should project archive exist in the product at all, or should projects remain
  non-archivable scope containers as the current knowledge plan suggests?
- Should renaming a project change the slug, or should slugs be immutable with
  redirect aliases?
- Is ownership single-user for now, or should Phase 0 include team ownership
  and membership fields immediately?
- Should draft sessions without a project remain allowed until the Phase 1 UI
  cutover, or should the server force project creation immediately?
- Do we want a dedicated `/projects/:projectSlug` overview page, or should the
  canonical entry land directly on `/projects/:projectSlug/sessions`?

## Done When

Phase 0 is complete when the following statements are true:

- there is one canonical persisted project model
- session ownership is defined clearly enough to remove orphan active sessions
- pipeline, CXDB, blueprint, and knowledge scope all point at the same
  canonical project identity
- routes can be planned around project pages without guessing what `projectId`
  means in each subsystem
