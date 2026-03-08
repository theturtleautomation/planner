# Phase 01 Root Landing And Navigation Implementation

**Status:** Research complete, ready for implementation  
**Date:** 2026-03-07

## Objective

Replace the current session-first root experience with a project-first landing
page and navigation shell.

This phase is complete when `/` is no longer a session dashboard, the global
navigation clearly points users toward projects first, and the route model is
explicit enough to support project-scoped work without another shell rewrite.

## Non-Goals

- define the canonical persisted `Project` model itself
- redesign project-local `Sessions`, `Blueprint`, `Knowledge`, or `Events`
  pages in detail
- redesign the knowledge filter bar
- rename blueprint or knowledge taxonomy
- redesign the session lobby event rendering
- add backend semantic search or an AI command palette
- remove all legacy global routes in this phase

## Decision Summary

- `/` becomes a protected `HomeHubPage`, not the session dashboard.
- `Dashboard` stops being the product entry point and becomes a global
  `SessionsPage` at `/sessions`.
- `/projects` becomes the dedicated project directory and should reuse the
  project-card interaction pattern already proven in Knowledge Library, but it
  must read from the canonical Projects API, not blueprint aggregation.
- `/projects/:projectSlug` is a canonical route and should immediately redirect
  to `/projects/:projectSlug/sessions` until a project overview page is
  explicitly justified.
- The root prompt is a local intent router and autocomplete affordance. It does
  **not** send arbitrary commands to the backend and does **not** become a
  semantic search surface in this phase.
- The landing page should show recent projects and quick links, not a dense
  operational dashboard.
- The sidebar becomes home-and-project-first:
  - `Home` is the root hub.
  - `Projects` is the first main work surface.
  - `Sessions` remains available as a transitional global queue, but it is no
    longer the product default.
- Creating new work from the root must be project-first. The canonical CTA is
  `New Project`, or `Start Planning` only if it first resolves a project.
- Global `/blueprint` remains a transitional route, but it should leave the
  primary navigation because it conflicts with the project-first shell.
- Auth behavior stays simple:
  - Auth0 anonymous users still see `LoginPage` at `/`.
  - Authenticated Auth0 users land in the new hub.
  - Local dev mode lands directly in the new hub.

## Current-State Summary

The current web shell is still rooted in sessions:

| Surface | Current behavior | Why it blocks a project-first product |
| --- | --- | --- |
| Root route | `/` renders `Dashboard` in dev mode and for authenticated Auth0 users | the entry point is a global session queue, not a project chooser or hub |
| Sidebar | `Sessions` points to `/` and is treated as active for `/session/*` routes | the primary nav encodes the old mental model directly into the shell |
| Dashboard | global list of sessions, direct `+ new session` CTA, header shortcuts to blueprint/admin | creation and navigation are session-first from the first screen |
| Knowledge Library | `/knowledge` already lands on project cards first | the project-first pattern exists, but it lives off to the side instead of at the main entry point |
| Session detail | `SessionPage` still offers `Back to Dashboard` | detail navigation still assumes the root is a session list |
| Session summary data | `SessionSummary` has no project metadata | the app cannot render project-aware session navigation cleanly yet |

### Current code anchors

- `planner-web/src/App.tsx`
- `planner-web/src/auth/Auth0Pages.tsx`
- `planner-web/src/components/Layout.tsx`
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/pages/LoginPage.tsx`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/pages/AdminPage.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/types.ts`

### Code findings that make the current shell misleading

- `App.tsx` has no `/projects` route and binds `/` directly to `Dashboard`.
- `Layout.tsx` hard-codes `Sessions` to `/` and marks it active for
  `/session/*`.
- `Dashboard.tsx` owns the product landing experience, empty state, and primary
  creation CTA, all of which are session-first.
- `LoginPage.tsx` in dev mode jumps straight to `/session/new`, which bypasses
  the project-first model entirely.
- `SessionPage.tsx` and `AdminPage.tsx` still use dashboard-oriented back-link
  language.
- `KnowledgeLibraryPage.tsx` already demonstrates that project-card navigation
  is viable, but it currently derives projects from blueprint data rather than
  the product project model defined in Phase 0.

### Visual audit findings from 2026-03-07

- `/` currently reads as an operations dashboard, not a hub. The first screen
  shows KPI boxes such as `Actionable` and `Attention Needed`, a primary
  `+ New Session` CTA, and session cards. There is no prompt affordance,
  recent-project area, or project-first navigation cue.
- The sidebar still teaches the old mental model directly: `Sessions` is the
  first nav item, it maps to `/`, and the most project-like screen is hidden
  under `Knowledge`.
- `/knowledge` already provides the strongest reusable project-directory
  pattern in the product: search, sort, favorites, project cards, health
  labels, counts, and explicit project open actions. Phase 1 should reuse this
  rhythm for `/projects` instead of inventing a different first-cut directory.
- On mobile, the current root view compresses into the same session queue with
  cards and operational badges. It still does not behave like a navigation hub
  or prompt surface.

## Proposed Behavior

### Route And IA Model

### Canonical route map

| Route | Page | Purpose |
| --- | --- | --- |
| `/` | `HomeHubPage` | product landing page and prompt-like navigation hub |
| `/projects` | `ProjectsPage` | dedicated project directory |
| `/projects/:projectSlug` | redirect | canonical project entry that redirects to `/projects/:projectSlug/sessions` |
| `/projects/:projectSlug/sessions` | `ProjectSessionsPage` | project-local session working surface |
| `/projects/:projectSlug/blueprint` | `ProjectBlueprintPage` | project-local blueprint route |
| `/projects/:projectSlug/knowledge` | `ProjectKnowledgePage` | project-local knowledge route |
| `/projects/:projectSlug/events` | `ProjectEventsPage` | project-local event route |
| `/sessions` | `SessionsPage` | transitional global all-sessions queue |
| `/knowledge` | existing project-first landing | global entry into knowledge library |
| `/knowledge/all` | existing explicit global knowledge view | cross-project exploration |
| `/events` | existing global event timeline | global operational timeline |
| `/admin` | existing admin page | admin and observability |

### Transitional compatibility routes

- `/session/new` remains valid during the migration window, but it should no
  longer be the primary CTA from `/` or `/sessions`.
- `/session/:id` remains valid, but once session project ownership is available
  it should resolve project context for breadcrumbs and back navigation.
- `/knowledge/projects/:projectRef` remains valid and should eventually align
  with `/projects/:projectSlug/knowledge`.
- `/blueprint` remains valid as a transitional utility route, but it should not
  remain a primary navigation destination.
- `/discovery` remains valid as a utility surface until a project-scoped
  discovery model is defined.

### Home Hub

### Core behavior

`HomeHubPage` should feel like a simple prompt-driven router with visible
fallback actions.

The page should have four clear regions:

1. A prompt card at the top.
2. A row of explicit quick actions.
3. A `Recent Projects` section.
4. A compact utilities section for cross-project surfaces.

### Root prompt behavior

The root prompt should be deterministic and cheap:

- It matches known intents such as:
  - `open projects`
  - `new project`
  - `knowledge`
  - `events`
  - `admin`
- It matches recent project names and slugs from the loaded project list.
- If the text does not map cleanly to a known action, it navigates to
  `/projects?query=...` so the user still lands in a useful place.

What it should **not** do in Phase 1:

- no backend command execution
- no full-text semantic search API
- no direct session creation that bypasses project selection
- no hidden natural-language automation promises

### Quick actions

The quick actions under the prompt should be explicit buttons, not prompt-only
discoverability:

- `Open Projects`
- `New Project`
- `Knowledge Library`
- `Events`
- `Admin`

Optional utility actions:

- `Open Sessions`
- `Legacy Blueprint`
- `Discovery`

Those utility actions should be visibly secondary to avoid reintroducing the
old session-first shell by accident.

### Recent projects

The hub should show a small, recent-first project list:

- 4-6 items maximum
- project name
- short description
- last activity
- session count
- knowledge count
- open action

This keeps `/` useful for repeat visits without turning it back into a dense
operational dashboard.

### Empty, loading, and local-dev states

#### Loading

- render the prompt immediately
- show skeleton cards or placeholder rows for recent projects
- keep quick actions usable even while project data is loading

#### No projects yet

- title: `No projects yet`
- primary CTA: `Create your first project`
- secondary CTA: `Open Knowledge Library`
- explain that sessions now live inside projects

#### Auth0 anonymous

- preserve the existing login behavior at `/`
- do not render the hub behind the login state

#### Local dev mode

- render the same authenticated hub as production
- show a small `dev mode` status cue, but do not route directly to
  `/session/new`

### Projects Directory

`/projects` should be the dedicated directory and should not be hidden inside
`/knowledge`.

Recommended first cut:

- reuse the card density and interaction style already present in
  `KnowledgeLibraryPage`
- read from the Projects API, not from blueprint-node aggregation
- support:
  - search
  - recent/default sort
  - favorites later if already provided by project summaries
- include a top-level `New Project` action

This lets Knowledge Library stay focused on knowledge while `/projects` becomes
the main product directory.

### Sessions Page

The current `Dashboard` should be repositioned, not deleted.

### New role

- rename the concept to `SessionsPage`
- move it to `/sessions`
- keep its existing capability-driven sorting and intervention logic
- add project metadata to each session card once Phase 0 fields are available

### Required behavior changes

- global CTA should no longer be raw `+ new session`
- use `Start From Project` or `New Project Session`, which first resolves a
  project
- empty state copy should stop implying session-first creation
- session cards should show project name and link back to the owning project

This keeps the valuable operational queue while removing it from the root of
the product.

### Navigation Shell

### Sidebar model

The sidebar should move from one flat list to a project-first order:

#### Primary

- `Home` -> `/`
- `Projects` -> `/projects`
- `Knowledge` -> `/knowledge`

#### Secondary

- `Sessions` -> `/sessions`
- `Events` -> `/events`
- `Admin` -> `/admin`

#### Utility or transitional

- `Discovery` -> `/discovery`
- `Blueprint` -> `/blueprint` labeled as legacy or utility until project-local
  blueprint routes are live

### Active-state rules

- `Home` is active only on `/`
- `Projects` is active for all `/projects/*`
- `Sessions` is active for `/sessions`, `/session/new`, and `/session/:id`
  until session detail is fully project-scoped
- `Knowledge` is active for `/knowledge*`
- `Events` is active for `/events*`

This removes the current incorrect behavior where the root page and session
detail routes both imply that `Sessions` is the product home.

### Project Route Contract

Phase 0 already defined project routes as the long-term working surface. Phase
1 should lock the shell to that contract now, even if some destinations still
resolve through aliases in the short term.

### Canonical project entry

- `/projects/:projectSlug` should redirect to
  `/projects/:projectSlug/sessions`

Reason:

- it avoids adding a redundant overview page before the product proves it needs
  one
- it keeps the project-local working surface explicit
- it aligns with the Phase 0 decision that `Sessions`, `Blueprint`,
  `Knowledge`, and `Events` are the main project tabs

### Short-term alias behavior

Until all project-local pages exist:

- `/projects/:projectSlug/knowledge` can delegate to the existing knowledge
  route and preserve project context
- `/projects/:projectSlug/blueprint` can delegate to the existing blueprint
  page with a resolved project context
- `/projects/:projectSlug/events` can delegate to the existing events page with
  project filtering once that exists

The important thing in Phase 1 is to make the route contract visible in the
shell now, not after more UI work accumulates.

## Impacted Files And Modules

### Web shell and routing

- `planner-web/src/App.tsx`
- `planner-web/src/components/Layout.tsx`
- new: `planner-web/src/pages/HomeHubPage.tsx`
- new: `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/auth/Auth0Pages.tsx`
- `planner-web/src/pages/LoginPage.tsx`

### Project and session entry surfaces

- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/pages/AdminPage.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/types.ts`
- `planner-web/src/api/client.ts`

### Tests

- new: `planner-web/src/pages/__tests__/HomeHubPage.test.tsx`
- new: `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-web/src/components/__tests__/Layout.test.tsx`
- `planner-web/src/pages/__tests__/Dashboard.test.tsx`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`

## API And Data Model Changes

Phase 1 should build on the Phase 0 project model instead of inventing a second
summary layer.

### Required project summary data

The web shell needs a lightweight project summary shape for `/` and
`/projects`:

```ts
interface ProjectNavSummary {
  id: string;
  slug: string;
  name: string;
  description?: string | null;
  owner_label?: string | null;
  last_activity_at?: string | null;
  session_count: number;
  knowledge_count: number;
}
```

Suggested API usage:

- `GET /projects?sort=recent&limit=6` for the home hub
- `GET /projects?sort=recent` or `GET /projects?sort=name` for `/projects`

### Required session summary additions

`SessionSummary` should gain enough project metadata to support global session
navigation:

```ts
interface SessionSummary {
  project_id?: string | null;
  project_slug?: string | null;
  project_name?: string | null;
}
```

This is necessary for:

- project badges on `/sessions`
- project-aware back links from session detail
- future redirects from `/session/:id` into project-local routes

### Explicit non-change

The root prompt does **not** require a new backend command API in this phase.
All prompt routing can be handled in the client from known routes and loaded
project summaries.

## UI And Routing Changes

### Route updates in `App.tsx`

- add `/projects`
- add `/projects/:projectSlug`
- add `/projects/:projectSlug/*` shells or redirects
- add `/sessions`
- change `RootPage` to render `HomeHubPage` instead of `Dashboard`

### `Layout.tsx` updates

- replace the current `Sessions -> /` entry
- add explicit `Home -> /`
- move `Projects` ahead of session utilities
- remove global `Blueprint` from the main work-surface position
- update active-route logic for `/projects/*` and `/sessions`

### `Dashboard.tsx` evolution

- keep the implementation logic
- rename the product concept from `Dashboard` to `Sessions`
- update copy:
  - `sessions` header remains valid
  - CTA and empty-state language must stop implying session-first creation
- optionally keep the filename during the first cut and export it through a new
  route alias to reduce churn

### `SessionPage.tsx` and nearby copy updates

- replace `Back to Dashboard` with:
  - `Back to Project` when project context exists
  - `Back to Sessions` as the transitional fallback
- update any `dashboard` wording in admin or secondary pages

## Migration And Backfill Plan

No new persistent data migration should be introduced here beyond Phase 0.
This phase is mostly a route, shell, and copy migration.

### Step 1: Ship project summary data

- expose project summaries from the Projects API
- expose project metadata on session summaries

### Step 2: Introduce new pages without deleting old logic

- add `HomeHubPage`
- add `ProjectsPage`
- add `/sessions` and point it at the current dashboard implementation

### Step 3: Cut root over to the new hub

- update `RootPage` and `RootPageAuth0`
- keep anonymous login behavior unchanged

### Step 4: Change the sidebar and active-route rules

- swap in the new nav order
- remove the assumption that `/` means sessions

### Step 5: Update copy and back-navigation

- session detail buttons
- admin back links
- login dev-mode enter flow
- empty-state language

### Step 6: Add project-route aliases

- `/projects/:projectSlug`
- transitional delegation from project-local routes to existing global pages

## Tests To Add Or Update

- Add route-level tests for `App.tsx` covering:
  - `/` authenticated hub
  - `/projects`
  - `/sessions`
  - `/projects/:projectSlug` redirect behavior
- Update `Layout` tests to assert:
  - `Home` exists
  - `Projects` exists
  - `Sessions` is no longer mapped to `/`
  - active states follow the new rules
- Add `HomeHubPage` tests for:
  - quick actions
  - prompt routing
  - empty state
  - recent projects rendering
- Add `ProjectsPage` tests for:
  - project list rendering
  - search/filter behavior
  - navigation into a project route
- Convert current `Dashboard` tests into `SessionsPage` behavior tests and add
  project metadata assertions
- Update `SessionPage` tests for the new back-link language and fallback rules

## Risks, Dependencies, And Rollout Order

### Dependencies

- Phase 0 project identity and Projects API contract must exist first, or this
  phase will be forced to fake projects from blueprint data again.
- Session summaries need project metadata before the session queue and detail
  pages can navigate cleanly inside the new shell.

### Primary risks

- Shipping `/projects` and `/knowledge` with duplicate project directories can
  feel redundant if their responsibilities are not clearly separated.
- Leaving a strong `+ new session` CTA in `/sessions` will quietly preserve the
  old product model even after the new hub ships.
- If `/projects/:projectSlug` semantics are left ambiguous, later project-page
  work will need another route migration.
- If dev mode still jumps to `/session/new`, local development will bypass the
  new shell and hide regressions.

### Recommended rollout order

1. Land project summary types and API support.
2. Add `HomeHubPage` and `ProjectsPage`.
3. Move the dashboard implementation to `/sessions`.
4. Update the sidebar and root route.
5. Fix session/admin/back-link copy.
6. Add project-route redirects and compatibility tests.

### Unresolved Questions

- After the project tabs exist, do we still want `/projects/:projectSlug` to
  redirect to `sessions`, or do we want a dedicated project overview page?
- Should `Discovery` remain a global utility after project-local routes are
  established, or should it become project-scoped as well?
- Once project metadata is available on sessions, should the home hub show only
  recent projects, or should it also surface a compact `Recent Sessions`
  module?
