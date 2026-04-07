# Builder Fusion Phase 05 Branch Surface Visibility Reconciliation Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent Spec:** [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  
**Prerequisite:** [Builder Fusion Phase 04 Project Visibility Diagnosis And Remediation Spec](/home/thetu/planner/docs/builder-fusion-phase-04-project-visibility-diagnosis-and-remediation-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-01 and 2026-04-02 live repo diagnosis output, direct authenticated API probes from the repo auth context, and authenticated Builder web-app network evidence showing `projects/branches?projectId=...&apiKey=...`

> Planning sync update (2026-04-02): this visibility slice still describes the
> correct diagnosis model for a latest saved or explicitly targeted Fusion
> project. It should not be read as a recommendation that Planner preserve one
> long-lived remote Builder project by default. Planner now creates fresh
> projects by default and uses this slice only when reasoning about a specific
> project after creation.

## 1. Purpose

Reconcile Builder Fusion project visibility across the branch surface and the
metadata surfaces so Planner can distinguish:

- fully visible Fusion projects
- branch-visible projects whose metadata surface is missing
- metadata-visible projects whose branch surface is missing
- projects that are genuinely not visible on any known Builder project surface

The goal of this slice is to correct Planner's current diagnosis model, which
is still centered on metadata-oriented project read surfaces and therefore
under-classifies the saved project's real visibility state.

## 2. Problem

Phase 04 improved Builder visibility diagnosis, but its read model is still
incomplete.

Current repo evidence now proves a stronger contradiction:

- direct metadata read of saved project
  `84f6b4304c4a4525807f6c9dcfbf25dc` returns `404`
- `projects/org-tree` returns zero visible projects
- `projects?apiKey=...&userId=...` returns zero visible projects
- but authenticated direct branch read succeeds:
  `GET https://api.builder.io/projects/branches?projectId=84f6b4304c4a4525807f6c9dcfbf25dc&apiKey=c302669d31c74e7fa80574973c437cfa`
  returns `200` with live branch data for the same project

That means the saved project is not truthfully "not visible." It is visible on
the branch surface while remaining invisible on the metadata surfaces.

Planner's current diagnosis and verification wrappers do not model that state,
so they currently over-compress the real Builder behavior into `undetermined`
or generic blocked visibility.

## 3. User Outcome

A repo user should be able to run the Builder visibility tooling and learn:

1. whether the saved project is visible on the Builder branch surface
2. whether the saved project is visible on Builder metadata surfaces
3. whether the current failure is a branch-only visibility mismatch, not a
   total read failure
4. whether remote settings drift can be partially verified from branch truth
   even when project metadata read is unavailable
5. what exact external Builder follow-up is still needed if the metadata
   surface remains missing

## 4. Scope

### In Scope

- treating `projects/branches?projectId=...` as a first-class Builder project
  visibility surface
- repo-native helpers that probe the branch surface for a saved or explicit
  project ID
- updated visibility classification that can express branch-only visibility
- updated verify-sync and get-project flows so they do not overstate "not
  found" when branch truth exists
- documentation updates that explain the new branch-surface diagnosis model

### Out Of Scope

- broad project recreation or ensure-project automation
- changing Builder runtime config or launch behavior
- Builder CMS or DSI work
- trying to reverse-engineer all Builder web-app endpoints beyond the bounded
  branch surface already observed

## 5. Product Decision

### 5.1 Required visibility model

Required direction:

- the branch surface is a real Builder project visibility surface
- Planner must not classify a project as effectively invisible if the branch
  surface returns live branches for that project ID
- metadata and branch visibility must be reported separately and reconciled

### 5.2 Required classification model

Required direction:

- replace the current coarse visibility diagnosis with a model that can
  express:
  - `fully_visible`
  - `branch_visible_only`
  - `metadata_visible_only`
  - `not_visible`
  - `undetermined`
- retain the Phase 04 drift/mismatch classifications when they remain
  supported by evidence, but surface branch visibility as an additional truth

### 5.3 Saved-project posture

Required direction:

- when reasoning about one specific saved or explicitly targeted project, do
  not classify it as deleted or stale by default while branch truth exists for
  that same project ID
- treat that project as a live remote Builder project with partial visibility,
  not as stale or deleted by default

## 6. Touched Surfaces

This slice is allowed to touch only the bounded Builder visibility tooling and
docs, for example:

- `scripts/builder-fusion-common.sh`
- `scripts/builder-list-projects.sh`
- `scripts/builder-get-project.sh`
- `scripts/builder-diagnose-project-visibility.sh`
- `scripts/builder-verify-sync.sh`
- `Makefile`
- `docs/builder-local-workflow.md`
- `docs/project-plan.md`
- `docs/session-start-and-doc-index.md`

## 7. Acceptance Criteria

1. Planner can probe the Builder branch surface for a saved or explicit project
   ID
2. the saved project is no longer classified as generically invisible when the
   branch surface succeeds
3. diagnosis output explicitly reports:
   - metadata visibility state
   - branch visibility state
   - the reconciled classification
4. `builder-get-project` and `builder-verify-sync` surface truthful partial
   visibility instead of collapsing to plain `not_found`
5. docs explain that Builder may expose branch truth and metadata truth on
   different surfaces

## 8. Verification Plan

Implementation verification should include the relevant subset of:

- `bash -n` on any touched shell scripts
- `./scripts/builder-auth-status.sh`
- `./scripts/builder-list-projects.sh`
- `./scripts/builder-get-project.sh`
- `./scripts/builder-diagnose-project-visibility.sh`
- `./scripts/builder-verify-sync.sh`
- authenticated direct probe of:
  - `projects/org-tree`
  - `projects?apiKey=...&userId=...`
  - `projects/:id`
  - `projects/branches?projectId=...&apiKey=...`
- `make builder-diagnose-project-visibility`
- `make builder-verify-sync`

Verification must prove the current saved project lands in a branch-visible
state if the branch endpoint continues to succeed.

## 9. Rollback / Fallback

If the branch surface cannot be integrated cleanly in one pass:

- preserve the Phase 04 diagnosis
- explicitly record that the current diagnosis is incomplete in the presence
  of branch-surface evidence
- do not widen this slice into recreate-project behavior

## 10. Open Questions

1. Does the Builder web app use only `projects/branches` plus route-level app
   state for project open, or does it also have a separate metadata endpoint
   that the repo has not yet modeled?
2. Should `builder-get-project` return a partial-success shape when branch
   truth exists but project metadata remains unavailable, or should that
   remain the responsibility of `builder-diagnose-project-visibility` and
   `builder-verify-sync` only?

## 11. Implementation Result

Implemented in:

- `scripts/builder-fusion-common.sh`
- `scripts/builder-diagnose-project-visibility.sh`
- `scripts/builder-get-project.sh`
- `scripts/builder-list-projects.sh`
- `scripts/builder-verify-sync.sh`
- `docs/builder-local-workflow.md`
- `README.md`
- `docs/project-plan.md`

Delivered behavior:

- the repo now treats `projects/branches?projectId=...` as a first-class
  Builder visibility surface
- `builder-diagnose-project-visibility` now distinguishes:
  - `fully_visible`
  - `branch_visible_only`
  - `metadata_visible_only`
  - `not_visible`
  - `undetermined`
- the saved Builder Fusion project is now truthfully classified as
  `branch_visible_only` in the current auth context
- `builder-get-project` now returns `status: "partial"` instead of collapsing
  to `not_found` when branch truth exists but metadata remains unavailable
- `builder-verify-sync` now returns `status: "visibility_partial"` and avoids
  pretending that full remote settings comparison succeeded
- `builder-list-projects --project-id ...` now surfaces a synthetic
  branch-visible entry instead of misleadingly returning zero projects
- repo docs now explain that opening `https://api.builder.io/...` directly in a
  browser tab is not authoritative for API diagnosis because Builder serves the
  web-app shell there; network XHR capture or repo-native authenticated probes
  are the reliable evidence sources

Current evidence after implementation:

- `projects/org-tree` still returns zero visible projects
- `projects?apiKey=...&userId=...` still returns zero visible projects
- direct metadata read of the saved project still returns `404`
- `projects/branches?projectId=84f6b4304c4a4525807f6c9dcfbf25dc&apiKey=...`
  succeeds in the authenticated repo context and proves the saved project is
  live on the branch surface
