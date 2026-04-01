# Builder Fusion Project Management And Runtime Sync Spec

**Status:** draft  
**Date:** 2026-03-29  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md), [Admin & Observability Plan](/home/thetu/planner/docs/admin-observability-plan.md)  

## Purpose

Define the bounded planning model for making Builder Fusion project management
and runtime sync first-class repo capabilities instead of ad hoc CLI/API
inspection.

This spec is intentionally a parent capability spec. It captures the full
Builder/Fusion improvement plan, preserves the boundary between Fusion, CMS,
and DSI, and defines the tranche order for implementation. It does not pretend
that the entire capability set should land in one implementation pass.

## Problem

Planner now has a truthful local Builder workflow and a local full-pipeline
mock mode, but the repo-local Builder tooling is still incomplete in one
critical area:

- the repo can launch Builder Fusion locally
- the repo can create a Fusion project and persist a local state file
- the repo can connect or index a repo through Builder CLI wrappers
- the repo can sync a Builder CMS `project` content entry
- the repo cannot treat an **existing Fusion project** as a first-class managed
  object

That gap creates repeated operator confusion:

- the local launch wrapper can inject `PLANNER_LLM_MOCK=full_pipeline`, but the
  current tooling cannot prove or update whether the remote Fusion project is
  configured consistently
- the saved Fusion project ID can exist locally, but the repo still lacks a
  first-class way to update that existing remote project’s runtime settings
- Builder CMS sync can succeed while the Fusion project remains out of date,
  which makes “sync” ambiguous
- repo users can create or reuse Fusion projects, but they cannot reliably
  inspect, update, profile-switch, or verify an existing project from the repo
- connection and indexing commands still behave like one-off Builder CLI tasks
  instead of part of a persistent repo-managed workflow

The result is that Builder capability is present, but not yet operationally
complete.

## User Outcome

A repo user working on Planner with Builder should be able to:

1. identify the canonical Fusion project for this repo
2. inspect the remote project’s runtime settings from the repo
3. update the remote Fusion project’s runtime command, URL, and environment
   without creating a new project
4. apply named runtime profiles such as `live`, `mock-socratic`, and
   `mock-full-pipeline`
5. verify whether local repo state, saved Fusion state, runtime configuration,
   and Builder-side project settings are actually aligned
6. understand whether a command is acting on Fusion project settings, Builder
   CMS content, or Builder DSI instead of mixing those surfaces together

## Scope

### In Scope

- repo-native Builder Fusion project inspection helpers
- repo-native Builder Fusion project update helpers
- remote Fusion environment-variable management
- profile-based runtime configuration for existing Fusion projects
- a single “ensure project” workflow that reuses existing saved project state
  when present
- a single “verify sync” workflow that reports local-vs-remote alignment
- explicit repo documentation that separates Fusion project config, CMS sync,
  and DSI capabilities
- CLI/API fallback implementation inside the Builder skill when MCP coverage is
  absent

### Out Of Scope

- redesigning Planner’s Builder DSI workflow
- replacing Builder CMS with a different content model
- automatic enterprise-only indexing workarounds
- broad Builder-hosted deployment orchestration outside Fusion runtime config
- using this capability work as a pretext to rework unrelated admin or product
  UI surfaces

## Current State

### What already exists

- [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md)
  is the truthful local workflow doc for Planner
- repo wrappers exist for:
  - `make builder-launch`
  - `make builder-create-project`
  - `make builder-connect-repo`
  - `make builder-connect-repo-dryrun`
  - `make builder-index-repo`
  - `make builder-sync-project`
- `.codex/builder-fusion-project.json` persists the locally chosen Fusion
  project ID and URL
- local Builder runtime mock mode now supports:
  - `phase26_live`
  - `full_pipeline`

### What is still missing

- no repo-native `get-project` helper for an existing Fusion project
- no repo-native `update-project` helper for an existing Fusion project
- no repo-native remote env-var helper for Fusion project settings
- no profile abstraction that can be applied locally and remotely
- no single verification command that answers “is Builder actually in sync?”
- no durable planning artifact for the full capability expansion itself

## Capability Model

The capability expansion should preserve the existing Builder surface split.

### Fusion Project Management

This tranche owns:

- saved Fusion project identity
- runtime command
- runtime URL
- install command
- branch/main branch metadata
- remote environment variables
- sync verification between local repo intent and remote Fusion settings

### Builder CMS Project Sync

This remains separate and continues to own:

- the `project` content entry
- runtime/proxy URL metadata stored for Builder-side browsing and content
  context

Fusion project update must not be hidden behind CMS sync, and CMS sync must not
be described as if it updates Fusion runtime settings.

### Builder DSI

This remains separate and should not be broadened by this capability plan.

## Planned Tranches

### Current Child Slice

The current bounded child slices under this parent capability plan are now:

- [Builder Fusion Phase 01 API-Grounded Skill And Existing Project Contract Spec](/home/thetu/planner/docs/builder-fusion-phase-01-api-grounded-skill-and-existing-project-contract-spec.md)
- [Builder Fusion Phase 02 Existing Project Helper Contract Spec](/home/thetu/planner/docs/builder-fusion-phase-02-existing-project-helper-contract-spec.md)
- [Builder Fusion Phase 03 Sync Verification Workflow Spec](/home/thetu/planner/docs/builder-fusion-phase-03-sync-verification-workflow-spec.md)

Current child-slice state:

- Phase 01 is implemented for Builder skill/reference hardening
- Phase 02 is implemented for existing Fusion-project helpers with narrow
  internal-endpoint acceptance
- Phase 03 is implemented for repo-native sync verification across local
  config, saved project state, and visible remote Fusion settings
- current Builder verification is still limited by auth-context visibility:
  the saved project is not returned by the current project-list query, so the
  new verify-sync workflow truthfully reports `visibility_blocked` and live
  remote update against that exact project remains unproven in this environment

### Tranche 1: Fusion Project Read/Update

Add repo-native helpers for an existing Fusion project:

- `builder-get-project.sh`
- `builder-list-projects.sh`
- `builder-update-project.sh`

This tranche must:

- reuse `.codex/builder-fusion-project.json` when available
- avoid creating a new Fusion project during read/update flows
- support explicit project ID override when local state is stale
- expose the currently effective remote runtime command and URL
- support updating the existing saved Fusion project’s runtime command, runtime
  URL, and other project settings without forcing project recreation

### Tranche 2: Remote Environment And Profiles

Add:

- `builder-get-project-env.sh`
- `builder-set-project-env.sh`
- `builder-sync-project-env.sh`
- named runtime profiles:
  - `live`
  - `mock-socratic`
  - `mock-full-pipeline`

This tranche must make `PLANNER_LLM_MOCK=full_pipeline` a first-class Fusion
project setting when the mock-full-pipeline profile is applied, rather than a
local launch-only convention.

### Tranche 3: Ensure And Verify Workflows

Add:

- `builder-ensure-project.sh`
- `builder-verify-sync.sh`

This tranche must let a repo user answer, in one command:

- which Fusion project is canonical for this repo
- whether local saved state matches remote project state
- whether the remote runtime command and env vars match the selected profile
- whether CMS sync metadata points at the same runtime/proxy values

### Tranche 4: Safer Connection, Indexing, And Handoff

Tighten the existing Builder wrappers so they:

- prefer reuse of known Fusion project state instead of accidental project
  creation
- classify plan-limit failures such as enterprise-only indexing clearly
- print the exact Builder project/branch URL and active runtime profile after
  successful operations
- make local-only vs remote-persisted changes explicit in command output

## Contracts And Touched Surfaces

### Scripts

Primary repo surface:

- `scripts/builder-launch.sh`
- `scripts/builder-create-project.sh`
- `scripts/builder-connect-repo.sh`
- `scripts/builder-index-repo.sh`
- `scripts/builder-sync-project.sh`
- new repo-native Fusion project helpers introduced by this spec

### State Files

- `.codex/builder-fusion-project.json` remains the repo-local source for the
  selected Fusion project identity
- the state file must not silently drift from the project being updated

### Shared Skill Scripts

The repo may continue delegating to shared Builder skill scripts, but the repo
must own any Planner-specific safety logic around:

- state-file reuse
- runtime profiles
- Fusion/CMS boundary clarity
- sync verification output

### Documentation

- [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md)
  must remain the truthful user-facing workflow doc
- the new helper set must be documented there as Fusion-project management,
  not as CMS sync

## Acceptance Criteria

This parent capability plan is satisfied only when all of the following are
true:

1. a repo user can inspect the saved Fusion project and its effective remote
   runtime settings without using manual curl or package-internals inspection
2. a repo user can update an existing Fusion project’s runtime command, runtime
   URL, and environment variables from the repo without creating a new project
3. a repo user can apply a named `mock-full-pipeline` profile that persists the
   mock runtime setting remotely for the existing Fusion project
4. the repo exposes a single verification command that reports whether Fusion
   project settings, local state, and CMS sync metadata agree
5. Builder workflow docs explicitly distinguish Fusion project config, CMS
   project sync, and DSI behavior
6. failure modes like missing auth, stale saved project IDs, and plan-gated
   indexing are reported as explicit operator-facing states rather than
   ambiguous command failures

## Verification Plan

Implementation slices promoted from this parent spec must include proof for:

- authenticated Builder CLI status
- readback of the target Fusion project before update
- update of remote runtime command and/or env var for a known project ID
- update of the existing saved Fusion project referenced by
  `.codex/builder-fusion-project.json`, not just a newly created disposable
  project
- readback after update showing the persisted change
- application of at least one named profile, including `mock-full-pipeline`
- verification command output demonstrating aligned and drifted states
- documentation examples that match the implemented commands

## Rollback And Fallback

- if remote Fusion project update proves unsupported through the current
  Builder APIs, the implementation must stop at an explicit read-only helper
  tranche rather than claiming remote config management is complete
- local Builder launch must remain functional even if remote project update
  helpers are unavailable
- CMS sync must remain independently usable and must not be blocked by Fusion
  project update failures

## Open Questions

- what exact Builder API or dev-tools internal contract is stable enough for
  updating an existing Fusion project’s settings
- whether remote environment variables are project-scoped, branch-scoped, or
  stored only in a broader setup object
- whether Builder exposes last-indexed commit or similar sync metadata without
  enterprise-only features
- whether `builder-create-project` should eventually evolve into
  `builder-ensure-project`, or remain a lower-level primitive beneath it

## Readiness Judgment

This parent capability spec remains intentionally **not** ready for direct
implementation as one broad change. It continues to serve as the source
planning artifact for follow-on bounded implementation slices.

The current honest next move is now narrower:

- either verify live remote update from a Builder auth context that can
  actually see the saved Fusion project, or
- explicitly decide whether Planner wants a future `builder-ensure-project`
  slice despite the duplicate-project risk when visibility is blocked

No additional child slice is currently promoted automatically beyond Phase 03.
