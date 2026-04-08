# Builder Fusion Phase 04 Project Visibility Diagnosis And Remediation Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent Spec:** [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  
**Prerequisite:** [Builder Fusion Phase 03 Sync Verification Workflow Spec](/home/thetu/planner/docs/builder-fusion-phase-03-sync-verification-workflow-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-01 inspection of the current repo Builder wrappers, the saved Fusion state file, the Builder/Fusion planning thread, and live visibility-diagnosis output from `builder-verify-sync` and `builder-diagnose-project-visibility`

> Planning sync update (2026-04-02): this slice remains the diagnosis model for
> the latest saved or explicitly targeted Fusion project, but it no longer
> implies that Planner should prefer preserving one long-lived remote Builder
> project. The default repo posture is now fresh creation plus local tracking;
> this diagnosis workflow is the follow-on path when a specific project is being
> inspected or verified.

## 1. Purpose

Diagnose and, when possible, remediate the current Builder Fusion project
visibility failure so Planner can tell the difference between:

- a repo-local misconfiguration
- saved-project state drift
- Builder auth/user/space mismatch
- a true Builder-side visibility limitation

The goal of this slice is not to paper over the blocked read path. It is to
make the cause explicit and fix repo-side causes if they exist.

## 2. Problem

Planner now has:

- a latest-saved Fusion project state file in
  `.codex/builder-fusion-project.json`
- repo-native existing-project helpers
- repo-native config inspection/validation helpers
- a read-only sync-verification workflow
- a repo-native visibility diagnosis wrapper

But the current Builder auth context still cannot read back the saved Fusion
project:

- `.codex/builder-fusion-project.json` points at project
  `84f6b4304c4a4525807f6c9dcfbf25dc`
- `builder-verify-sync` reports `visibility_blocked`
- `builder-diagnose-project-visibility` reports:
  - project-list/org-tree surface returns zero visible projects
  - direct read of the saved project returns `404`
  - diagnosis: `no_visible_projects_on_current_project_api_surface`

This is not an acceptable long-term state because it leaves Planner unable to
prove whether the saved Fusion project is genuinely inaccessible or whether the
repo is targeting the wrong project, space, identity, or API surface.

The current user concern is reasonable: it is suspicious that Builder would
allow write-oriented flows but deny all readback of the same project. The repo
needs a bounded diagnosis/remediation slice instead of treating that behavior
as a settled platform truth.

## 3. User Outcome

A repo user should be able to run a bounded diagnosis workflow and learn:

1. whether the latest saved or explicitly targeted Fusion project ID is
   internally consistent with repo
   state
2. whether the active Builder auth context matches the expected Builder user
   and space for that saved project
3. whether the saved project is missing from the currently visible project
   surface because of stale state, auth drift, space drift, or API-surface
   mismatch
4. whether a repo-side correction is possible without creating a duplicate
   project
5. what exact external follow-up is required if the problem is truly on the
   Builder side

## 4. Scope

### In Scope

- diagnosing the current `visibility_blocked` state for the latest saved Fusion
  project
- explicit classification of likely causes:
  - stale saved project ID
  - wrong Builder user/auth context
  - wrong Builder space/project family
  - Builder API surface mismatch between create/update and read/list
  - true Builder-side visibility limitation
- repo-native diagnosis output improvements if current helpers are not specific
  enough
- repo-side remediation of misconfiguration or stale-state causes when that can
  be done safely
- documentation updates that explain the exact blocked state and how to verify
  the active Builder identity/space/project targeting

### Out Of Scope

- broad `builder-ensure-project` automation
- creating a replacement Fusion project by default
- changing Planner's Builder runtime profile model
- Builder CMS or DSI changes
- speculative workarounds that normalize duplicate-project creation as the fix

## 5. Current Evidence

As of 2026-04-01, the repo evidence is:

- saved project state exists in `.codex/builder-fusion-project.json`
- the saved project URL is
  `https://builder.io/app/projects/84f6b4304c4a4525807f6c9dcfbf25dc/happy-cli`
- `builder-get-project` cannot read that saved project in the current auth
  context
- `builder-verify-sync` and `builder-diagnose-project-visibility` both report
  blocked visibility instead of drifted remote settings

That evidence is enough to promote this slice. It is not enough to conclude
that Builder intentionally supports write-without-read for this project.

## 6. Product Decision

### 6.1 Required diagnostic posture

Required direction:

- treat the current blocked readback as an unresolved diagnosis problem
- do not encode "Builder just works this way" into repo docs or workflow
- bias toward proving or disproving repo misconfiguration first

### 6.2 Safe remediation posture

Required direction:

- prefer identity/auth/space correction over project recreation
- do not create or "ensure" a new project as part of ordinary diagnosis
- allow explicit operator-driven override or recovery guidance only after the
  repo has classified the failure mode

### 6.3 Output model

Required direction:

- produce one diagnosis result that clearly names the most likely class:
  - `saved_project_stale`
  - `auth_context_mismatch`
  - `space_context_mismatch`
  - `api_surface_mismatch`
  - `builder_visibility_limitation`
  - `undetermined`
- include the evidence used to reach that classification

## 7. Touched Surfaces

The implementation is allowed to touch only the bounded Builder/Fusion
diagnosis surface, for example:

- `scripts/builder-auth-status.sh`
- `scripts/builder-list-projects.sh`
- `scripts/builder-get-project.sh`
- `scripts/builder-diagnose-project-visibility.sh`
- `scripts/builder-verify-sync.sh`
- `scripts/builder-config-common.sh` if shared identity/space helpers are
  justified
- shared Builder skill scripts under
  `/home/thetu/.codex/skills/builder-workflow/scripts/` when the repo wrappers
  depend on them
- `docs/builder-local-workflow.md`
- `.omx/ledger/current-status.md`

The saved state file `.codex/builder-fusion-project.json` may be corrected only
if the diagnosis proves it is stale and the correction is explicit and safe.

## 8. Acceptance Criteria

1. Planner exposes a bounded diagnosis workflow that goes beyond generic
   `visibility_blocked` and classifies the likely failure mode
2. the diagnosis can distinguish between stale saved state, auth/user drift,
   space drift, and Builder-side visibility failure when evidence supports that
   distinction
3. if the repo is misconfigured, the implementation fixes that misconfiguration
   or makes the correction path explicit without creating a duplicate project
4. if the repo is not misconfigured, the implementation leaves behind precise,
   user-facing evidence for the external Builder limitation
5. the Builder workflow docs explain the exact verification path for the saved
   Fusion project identity and current auth context

## 9. Verification Plan

Implementation verification should include the relevant subset of:

- `bash -n` on any touched shell scripts
- `./scripts/builder-auth-status.sh`
- `./scripts/builder-list-projects.sh`
- `./scripts/builder-get-project.sh`
- `./scripts/builder-diagnose-project-visibility.sh`
- `./scripts/builder-verify-sync.sh`
- `make builder-verify-sync`

If the diagnosis adds new classification output, verification must prove it on
the actual current saved project state rather than a fabricated fixture.

## 10. Implementation Result

Implemented in:

- `scripts/builder-fusion-common.sh`
- `scripts/builder-diagnose-project-visibility.sh`
- `scripts/builder-list-projects.sh`
- `scripts/builder-get-project.sh`
- `scripts/builder-create-project.sh`
- `scripts/builder-verify-sync.sh`
- `Makefile`
- `docs/builder-local-workflow.md`
- `README.md`

Delivered behavior:

- the repo now diagnoses Fusion project visibility across both known read
  surfaces:
  - `projects/org-tree`
  - `projects?apiKey=...&userId=...`
- direct project read is now checked both with and without `userId`
- `builder-diagnose-project-visibility` now reports:
  - current auth context
  - read-surface counts
  - direct-read evidence
  - a bounded `classification`
  - evidence strings explaining why that classification was reached
- `builder-verify-sync` now surfaces the classification instead of only
  generic `visibility_blocked`
- future create flows now persist richer saved-state context for the Fusion
  project, including `spaceId`, `userId`, `spaceName`, `branchName`,
  `savedAt`, and `stateVersion`
- the current saved project remains blocked, but the repo now proves that the
  active Builder auth context matches the current CLI/env and that both read
  surfaces plus direct read return no visible project; because the older saved
  state lacks space/user context, the truthful classification for the current
  project is `undetermined`, not a guessed mismatch

## 11. Verification Evidence

- `bash -n scripts/builder-fusion-common.sh scripts/builder-diagnose-project-visibility.sh scripts/builder-list-projects.sh scripts/builder-get-project.sh scripts/builder-create-project.sh scripts/builder-verify-sync.sh`
- `./scripts/builder-auth-status.sh`
- `./scripts/builder-list-projects.sh`
- `./scripts/builder-get-project.sh`
- `./scripts/builder-diagnose-project-visibility.sh`
- `./scripts/builder-verify-sync.sh`
- `make builder-diagnose-project-visibility`

## 12. Rollback / Fallback

If full root-cause classification cannot be made in one pass:

- preserve the current truthful blocked-state behavior
- improve evidence quality and operator guidance
- stop short of unsafe automatic correction

Do not widen this slice into automatic replacement-project creation or generic
Builder provisioning.

## 13. Open Questions

1. Does the current Builder auth context expose enough user/space identity to
   prove mismatch directly, or will the repo only be able to infer it from
   project-list/read behavior?
2. Is the saved Fusion project actually in a different Builder space/family
   than the current project-list surface, or is the mismatch purely identity
   related?
3. If the diagnosis proves the saved state is stale, what is the least risky
   repo-side recovery flow that does not normalize accidental duplicate
   projects?
