# Builder Fusion Phase 03 Sync Verification Workflow Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent Spec:** [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  
**Prerequisite:** [Builder Fusion Phase 02 Existing Project Helper Contract Spec](/home/thetu/planner/docs/builder-fusion-phase-02-existing-project-helper-contract-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-01 inspection of the implemented Builder wrapper surface in `scripts/`, the shared Builder helper behavior in `/home/thetu/.codex/skills/builder-workflow/scripts/`, and the current Builder planning thread in `docs/project-plan.md`

## 1. Purpose

Add a repo-native Builder Fusion sync-verification workflow so Planner can
answer whether local intent and the remote Fusion project are actually aligned
without mutating project state.

The existing Builder surface can create, inspect, update, and diagnose project
visibility, but it still lacks one command that resolves the operational
question users actually have: "is Builder in sync right now, and if not, what
exactly is mismatched or blocked?"

## 2. Problem

Planner now has:

- committed Builder config files
- config inspection and validation helpers
- saved Fusion project state in `.codex/builder-fusion-project.json`
- existing-project list/get/update helpers
- a visibility diagnosis helper for blocked auth contexts

But the repo still does not have one bounded verification surface that:

- reads the active Builder config contract
- checks the saved Fusion project identity
- checks whether the current auth context can see that project
- compares the expected runtime command, URL, and profile against the visible
  remote project when possible
- reports blocked states truthfully when remote comparison cannot happen

Without that workflow, Builder operations still require manual command chaining
and mental reconciliation across multiple outputs.

## 3. User Outcome

A repo user should be able to run one read-only command and learn:

1. which Builder config file/profile is active
2. which Fusion project the repo considers canonical
3. whether the current auth context can see that project
4. whether the remote project's runtime command, URL, and effective profile
   match local intent
5. whether the result is "in sync", "drifted", or "blocked by visibility/auth"

## 4. Scope

### In Scope

- a repo-native `builder-verify-sync.sh` helper
- read-only comparison of:
  - resolved Builder config
  - saved Fusion project state
  - remote Fusion project settings when visible
- explicit classification of visibility/auth-blocked states
- clear human-readable output plus machine-readable summary
- `make` entrypoints for the default and alternate Builder config paths
- Builder workflow documentation updates for the new verification command

### Out Of Scope

- mutating remote Fusion project settings
- creating or ensuring a Fusion project
- broad CMS readback verification
- Builder DSI changes
- changing the existing config/profile model

## 5. Contract

- verification must be read-only
- the command must work even when remote visibility is blocked, and report
  that blocked state instead of failing opaquely
- comparison must treat the local Builder config as the source of runtime
  intent for command and URL
- profile comparison must remain truthful to the existing config/profile rules
  in `scripts/builder-config-common.sh`
- the command must not silently recreate, overwrite, or update the saved Fusion
  project

## 6. Product Decision

### 6.1 Verification surface

Required direction:

- add one repo-native command: `scripts/builder-verify-sync.sh`
- default to `builder.config.json`
- support alternate config selection through
  `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json`

### 6.2 Result model

Required direction:

- report one overall state:
  - `in_sync`
  - `drifted`
  - `visibility_blocked`
  - `missing_saved_project`
  - `config_invalid`
- show the specific mismatches instead of a generic failure

### 6.3 Visibility handling

Required direction:

- if the saved project is not visible in the current auth context, surface the
  existing diagnosis truthfully instead of pretending comparison succeeded
- reuse the existing `builder-get-project.sh` and
  `builder-diagnose-project-visibility.sh` behavior rather than inventing a
  second remote-read path

## 7. Touched Surfaces

- `scripts/builder-verify-sync.sh`
- `scripts/builder-config-common.sh` only if shared comparison helpers are
  justified
- `Makefile`
- `docs/builder-local-workflow.md`
- `docs/project-plan.md`
- `docs/session-start-and-doc-index.md` only because this new durable spec is
  created in this session

## 8. Acceptance Criteria

1. the repo exposes a single read-only Builder sync verification command
2. the command clearly reports the active config contract and saved project
   identity
3. when the remote project is visible, the command reports whether command,
   URL, and effective profile are aligned or drifted
4. when the remote project is not visible, the command reports a truthful
   blocked state instead of a misleading generic error
5. the workflow is documented for both default frontend-mock and alternate
   server-backed Builder config paths

## 9. Implementation Update

Implemented in:

- `scripts/builder-verify-sync.sh`
- `scripts/builder-config-common.sh`
- `Makefile`
- `docs/builder-local-workflow.md`
- `README.md`

Delivered behavior:

- adds one repo-native read-only verification command for Builder Fusion sync
- compares the active config contract, saved Fusion project state, and visible
  remote project settings when possible
- reports `visibility_blocked` truthfully when the current auth context cannot
  see the saved remote project
- exposes matching `make builder-verify-sync` and
  `make builder-server-verify-sync` entrypoints for the default and alternate
  config paths

## 10. Verification Evidence

- `bash -n scripts/builder-verify-sync.sh scripts/builder-config-common.sh`
- `./scripts/builder-verify-sync.sh`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-verify-sync.sh`
- `./scripts/builder-diagnose-project-visibility.sh`
- `make builder-verify-sync`
- `make builder-server-verify-sync`

## 11. Rollback / Fallback

If full structured comparison proves too broad in one pass:

- keep the config/state/visibility summary
- keep the blocked-state classification
- leave deeper mismatch reporting as a follow-on

Do not widen this slice into remote mutation or ensure-project behavior.

## 12. Open Questions

None block readiness.

The remaining implementation freedom is output shape, not capability scope:
human-readable plus JSON is acceptable as long as the command stays read-only
and the blocked-state truth remains explicit.
