# Phase 6E Project Delete UI Execution Checklist

**Status:** Implemented  
**Date:** 2026-03-08

## Objective

Expose archive and delete in the projects UI with explicit destructive
confirmation and clear result handling.

## Scope Guardrails

### In scope

- web API client delete support
- typed delete summary response
- `/projects` delete action
- destructive confirmation prompt
- in-flight and error states
- list refresh after delete

### Explicitly out of scope

- backend delete semantics
- CXDB and blueprint purge internals
- archive implementation details already covered by `6A`

## Success Criteria

- the projects UI exposes a delete action
- users are warned that delete will stop and remove sessions
- delete is disabled while in flight
- successful delete refreshes the list and removes the project from view
- failed delete leaves the page stable and surfaces an error

## Current Code Anchors

- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/pages/ProjectsPage.tsx`
- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`
- `planner-web/src/api/__tests__/client.test.ts`

## Test-First Execution Order

## Step 1: Add web client delete tests first

### Files

- `planner-web/src/api/__tests__/client.test.ts`

### Tests to add first

1. `deleteProject makes DELETE request to /api/projects/:projectRef`
2. `deleteProject returns delete summary payload`

## Step 2: Add projects-page delete UX tests first

### Files

- `planner-web/src/pages/__tests__/ProjectsPage.test.tsx`

### Tests to add first

1. `delete confirmation warns that sessions will be stopped and removed`
2. `cancelled delete does not call API`
3. `confirmed delete calls deleteProject and reloads list`
4. `delete failure renders error and leaves project visible`
5. `delete action is disabled while request is in flight`

### Assertions to include

- confirmation copy includes:
  - project name
  - permanent deletion warning
  - session stop/removal warning
- list refresh occurs after success
- no navigation occurs on delete from `/projects`

## Step 3: Implement web client delete support

### Files

- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`

### Checklist

- add `DeleteProjectResponse` type
- add `deleteProject(projectRef)` client method
- keep response typing aligned with the backend summary contract

## Step 4: Implement `/projects` delete UX

### Files

- `planner-web/src/pages/ProjectsPage.tsx`

### Task order

1. Add delete action per project card.
2. Prompt for confirmation.
3. Call `deleteProject(project.slug)` or canonical ref.
4. Track in-flight deletion state by project ID.
5. Refresh the list after success.
6. Render failure inline.

### Guardrails

- do not add multi-step modal UX unless the existing page structure needs it
- use direct, factual copy
- keep delete separate from archive/unarchive actions visually

## Step 5: Regression pass

### Verification checklist

1. Run web client tests.
2. Run projects page tests.
3. Manually verify delete confirmation copy and success behavior.
4. Verify archive actions from `6A` still work.

## Recommended Command Order

```bash
# 1. Add failing client tests
cd planner-web && npm test -- --run client.test.ts

# 2. Add failing page tests
cd planner-web && npm test -- --run ProjectsPage.test.tsx

# 3. Implement client and UI delete flow

# 4. Run focused regression checks
cd planner-web && npm test -- --run client.test.ts ProjectsPage.test.tsx
```

## Exit Criteria For Phase 6E

- delete is exposed in `/projects`
- confirmation copy matches the agreed contract
- success and failure flows are test-covered
