# Phase 6A Project Archive Execution Checklist

**Status:** Implemented  
**Date:** 2026-03-08

## Objective

Define the lifecycle contract in the API and make archive/unarchive functional
before destructive delete work begins.

## Scope Guardrails

### In scope

- project API/archive contract
- project store archive helpers
- active-only default project listing
- optional `include_archived` list query
- web API client support for archive filtering and archive patching
- `/projects` UI archive and unarchive actions
- frontend and backend tests for archive behavior

### Explicitly out of scope

- `DELETE /projects/{projectRef}`
- stopping active sessions or pipeline tasks
- session deletion
- CXDB project-run deletion
- blueprint local delete or shared unlink
- delete confirmation UX
- project detail page archive UX beyond direct-route compatibility

## Success Criteria

- active projects remain visible in `/projects`
- archived projects are hidden by default
- archived projects appear when the user enables `Show archived`
- archiving a project does not delete any sessions or project data
- archived projects can be restored
- direct `GET /projects/{slug-or-id}` still works for archived projects
- server and frontend tests cover the behavior before Phase 6B begins

## Current Code Anchors

- `planner-server/src/api.rs`
- `planner-server/src/project.rs`
- `planner-web/src/types.ts`
- `planner-web/src/api/client.ts`
- `planner-web/src/api/__tests__/client.test.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`

## Test-First Execution Order

## Step 1: Add server API tests first

### Goal

Lock the archive contract at the HTTP layer before changing implementation.

### Files

- `planner-server/src/api.rs`

### Tests to add first

1. `test_list_projects_excludes_archived_by_default`
2. `test_list_projects_can_include_archived`
3. `test_update_project_can_archive`
4. `test_update_project_can_unarchive`
5. `test_get_archived_project_by_slug_still_works`

### Assertions to include

- default `GET /projects` returns only active projects
- `GET /projects?include_archived=true` returns both active and archived
  projects
- `PATCH /projects/{projectRef}` with `{"archived": true}` sets
  `archived_at != null`
- `PATCH /projects/{projectRef}` with `{"archived": false}` sets
  `archived_at == null`
- archived projects remain fetchable by direct slug route

### Notes

- keep these tests near the existing project route coverage around
  [`api.rs`](/home/thetu/planner/planner-server/src/api.rs#L3577)
- do not add delete-oriented cases here

## Step 2: Add project-store unit tests

### Goal

Keep archive timestamp semantics out of the route handler and test them at the
storage layer.

### Files

- `planner-server/src/project.rs`

### Tests to add first

1. `project_store_archive_sets_archived_at`
2. `project_store_unarchive_clears_archived_at`
3. `project_store_list_for_user_excludes_archived_when_requested`

### Assertions to include

- archive helper sets `archived_at`
- unarchive helper clears `archived_at`
- archived projects are still stored and retrievable by ID/slug
- list helpers support both active-only and include-archived behavior

### Implementation preference

- add explicit `archive`, `unarchive`, or `set_archived` helper(s) in the store
- avoid duplicating archive timestamp logic in route handlers

## Step 3: Implement backend archive behavior

### Goal

Make the tests from Steps 1 and 2 pass with the smallest coherent backend
change set.

### Files

- `planner-server/src/project.rs`
- `planner-server/src/api.rs`

### Task order

1. Add project-store archive helper(s).
2. Add active-only and include-archived list helper(s).
3. Extend `UpdateProjectRequest` with `archived: Option<bool>`.
4. Add `ListProjectsQuery` with `include_archived: bool`.
5. Update `list_projects()` to honor the query flag.
6. Update `update_project()` to toggle archive state.
7. Keep `get_project()` behavior unchanged so archived direct fetch still works.

### Concrete backend checklist

- `planner-server/src/project.rs`
  - add `set_archived(project_id, archived: bool) -> Option<Project>`
  - set `archived_at` to `Utc::now().to_rfc3339()` when archiving
  - clear `archived_at` when unarchiving
  - add `list_for_user_active()` or `list_for_user(user_id, include_archived)`
- `planner-server/src/api.rs`
  - add `ListProjectsQuery`
  - update `list_projects()` signature to accept `Query(ListProjectsQuery)`
  - default `include_archived` to `false`
  - include `archived` in the empty-update guard for `update_project()`
  - delegate archive mutation to the store helper instead of hand-rolling it

### Guardrail

- do not change session or project detail route behavior in this step beyond
  archive visibility in the list endpoint

## Step 4: Add web API client tests

### Goal

Lock request-shape behavior before touching the projects page.

### Files

- `planner-web/src/api/__tests__/client.test.ts`

### Tests to add first

1. `listProjects sends include_archived query when requested`
2. `updateProject sends archived true in patch body`
3. `updateProject sends archived false in patch body`

### Assertions to include

- `listProjects({ includeArchived: true })` calls `/api/projects?include_archived=true`
- `updateProject(projectRef, { archived: true })` serializes `archived: true`
- `updateProject(projectRef, { archived: false })` serializes `archived: false`

## Step 5: Implement web API client and types

### Goal

Make the web client understand archive semantics cleanly before wiring UI
behavior.

### Files

- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`

### Task order

1. Extend `listProjects()` to accept `{ includeArchived?: boolean }`.
2. Extend `updateProject()` patch typing with `archived?: boolean`.
3. Keep the existing `Project` interface as-is unless an archive-specific view
   helper type becomes necessary.

### Concrete checklist

- `planner-web/src/api/client.ts`
  - build query string for `include_archived`
  - include `archived` in the patch JSON body
- `planner-web/src/types.ts`
  - keep `Project.archived_at` as the source of truth
  - add request helper types only if they reduce duplication materially

## Step 6: Add projects-page tests

### Goal

Define the intended archive UX before implementation.

### Files

- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`

### Tests to add first

1. `hides archived projects by default`
2. `shows archived projects when filter is enabled`
3. `archives a project and reloads the list`
4. `unarchives a project and reloads the list`

### Assertions to include

- archived records returned by the API are not shown until the filter is on
- the page calls `listProjects()` with and without `includeArchived` correctly
- clicking `Archive` calls `updateProject(..., { archived: true })`
- clicking `Unarchive` calls `updateProject(..., { archived: false })`
- the list refreshes after a successful mutation
- the action is disabled while the mutation is in flight

### Mocking note

- extend the current `createApiClient` mock in
  [`ProjectsPage.test.tsx`](/home/thetu/planner/planner-web/src/pages/__tests__/ProjectsPage.test.tsx#L10)
  with `updateProject`
- reuse the existing router harness instead of creating a second test mount
  pattern

## Step 7: Implement the `/projects` archive UX

### Goal

Make the projects list usable for archive and restore without leaking delete
work into this slice.

### Files

- `planner-web/src/pages/ProjectsPage.tsx`

### Task order

1. Add `showArchived` state or URL param.
2. Pass `includeArchived` through `loadProjects()`.
3. Add archive and unarchive actions per project card.
4. Add a mutation-in-flight state keyed by project ID.
5. Refresh the list after mutation success.
6. Show inline error feedback if archive mutation fails.

### Concrete UI checklist

- add a `Show archived` checkbox or toggle near the search bar
- keep lifecycle actions separate from navigation actions
- show `Archive` only for active projects
- show `Unarchive` only for archived projects
- preserve existing `Open`, `Knowledge`, `Blueprint`, and `Events` actions
- do not add `Delete` yet

### UX guardrails

- archive should not navigate away from `/projects`
- avoid `window.prompt`; use regular buttons for archive/unarchive
- keep the UI copy factual and minimal

## Step 8: Regression pass and polish

### Goal

Finish the archive slice without letting Phase 6B work leak in.

### Verification checklist

1. Run the targeted server tests for the new project archive routes.
2. Run the targeted frontend tests for client and projects page archive flows.
3. Manually verify:
   - active-only default list
   - `Show archived` toggle
   - archive and restore behavior
   - direct archived project fetch remains valid
4. Confirm no delete API or delete UI was introduced in this slice.

## Recommended Command Order

```bash
# 1. Add failing server tests
cargo test -p planner-server test_create_and_get_project_by_slug -- --nocapture

# 2. Add failing project-store tests
cargo test -p planner-server project_store_archive -- --nocapture

# 3. Implement backend until server tests pass

# 4. Add failing web client and page tests
cd planner-web && npm test -- --run client.test.ts ProjectsPage.test.tsx

# 5. Implement client and UI until frontend tests pass

# 6. Run focused regression checks
cargo test -p planner-server test_list_projects -- --nocapture
cd planner-web && npm test -- --run client.test.ts ProjectsPage.test.tsx
```

## Exit Criteria For Phase 6A

- backend archive API is live and test-covered
- archived projects are hidden by default in `/projects`
- archived projects can be restored
- direct archived project fetch still works
- frontend archive flows are test-covered
- no delete, runtime-stop, blueprint, or CXDB work has started yet
