# Builder Fusion Project Management And Runtime Sync Spec

**Status:** draft  
**Date:** 2026-03-29  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md), [Admin & Observability Plan](/home/thetu/planner/docs/admin-observability-plan.md)  

## Purpose

Define the bounded planning model for making Builder Fusion project management
and runtime sync first-class repo capabilities instead of ad hoc CLI/API
inspection.

This spec is intentionally a parent capability spec. It captures the full
Builder/Fusion improvement plan, preserves the boundary between Fusion, CMS,
and DSI, and defines the tranche order for implementation. It does not pretend
that the entire capability set should land in one implementation pass.

> Planning sync update (2026-04-02): Planner now treats Builder Fusion
> projects as fire-and-forget remote workspaces. The repo creates fresh
> projects by default, stores the latest created project in
> `.codex/builder-fusion-project.json`, and records the durable local source of
> truth for created names/settings in
> `.codex/builder-fusion-project-history.jsonl`. Existing-project get/update/
> verify helpers remain supported, but they are follow-on utilities for the
> latest saved project or an explicit project ID override, not the default repo
> strategy.

## Problem

Planner now has a truthful local Builder workflow and a local full-pipeline
mock mode, but the parent Builder planning model is now out of sync with the
repo's actual operating posture:

- the repo can launch Builder Fusion locally
- the repo can create fresh Fusion projects and persist both a latest-project
  state file and a local append-only creation ledger
- the repo can connect or index a repo through Builder CLI wrappers
- the repo can sync a Builder CMS `project` content entry
- the repo can inspect, update, and verify an explicitly targeted existing
  Fusion project when Builder visibility allows
- but this parent spec still frames long-lived existing-project management as
  the default desired end state

That planning drift creates repeated operator confusion:

- the local launch wrapper can inject `PLANNER_LLM_MOCK=full_pipeline`, but the
  current tooling cannot prove or update whether the remote Fusion project is
  configured consistently
- the latest saved Fusion project ID can exist locally, but the parent plan
  still overstates one long-lived remote project as the normal path
- Builder CMS sync can succeed while the Fusion project remains out of date,
  which makes “sync” ambiguous
- repo users can create fresh Fusion projects with local tracking, but the
  parent plan still points them toward canonical-project reuse and future
  ensure-project behavior
- connection and indexing commands still behave like one-off Builder CLI tasks
  instead of part of a persistent repo-managed workflow

The result is that Builder capability is present and usable, but the parent
planning artifact is no longer describing the repo's actual default strategy.

## User Outcome

A repo user working on Planner with Builder should be able to:

1. create a fresh Builder Fusion project from the repo with a trackable,
   distinguishable name
2. keep a durable local record of which project was created and which config/
   settings were used at creation time
3. use the latest saved project for immediate repo-local follow-on commands
   such as inspection, dry-run update, or sync verification
4. inspect or update an explicitly targeted existing Fusion project when that
   narrower workflow is useful, without treating it as the default posture
5. verify whether local repo state, latest saved state, runtime configuration,
   and visible Builder-side project settings are actually aligned for the
   project in scope
6. understand whether a command is acting on Fusion project settings, Builder
   CMS content, or Builder DSI instead of mixing those surfaces together

## Scope

### In Scope

- fresh-by-default Builder Fusion project creation
- durable local tracking of latest-created and historically created Fusion
  projects
- repo-native Builder Fusion project inspection helpers
- repo-native Builder Fusion project update helpers
- remote Fusion environment-variable management where Builder visibility allows
- profile-based runtime configuration for explicitly targeted existing projects
- a single “verify sync” workflow that reports local-vs-remote alignment for
  the latest saved or explicitly targeted project
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
- `.codex/builder-fusion-project.json` persists the latest created Fusion
  project identity for immediate follow-on commands
- `.codex/builder-fusion-project-history.jsonl` records the durable local
  history of created project identities plus the exact create-time settings
- local Builder runtime mock mode now supports:
  - `phase26_live`
  - `full_pipeline`

### What is still missing

- the parent planning doc still overstates existing-project management as the
  primary goal
- the parent planning doc still implies `builder-ensure-project` style reuse as
  a likely future default even though the repo now prefers fresh creation
- the child Builder specs need planning-sync notes so they are read as retained
  helper capabilities, not as the default repo strategy

## Capability Model

The capability expansion should preserve the existing Builder surface split.

### Fusion Project Management

This capability thread now owns two layers:

- default repo posture:
  - fresh Fusion project creation
  - trackable generated names
  - local latest-project state
  - append-only local creation history
- follow-on targeted project management:
  - runtime command
  - runtime URL
  - install command
  - branch/main branch metadata
  - remote environment variables
  - sync verification between local repo intent and an explicitly targeted
    visible remote Fusion project

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
- [Builder Fusion Phase 04 Project Visibility Diagnosis And Remediation Spec](/home/thetu/planner/docs/builder-fusion-phase-04-project-visibility-diagnosis-and-remediation-spec.md)
- [Builder Fusion Phase 05 Branch Surface Visibility Reconciliation Spec](/home/thetu/planner/docs/builder-fusion-phase-05-branch-surface-visibility-reconciliation-spec.md)

Current child-slice state:

- Phase 01 is implemented for Builder skill/reference hardening
- Phase 02 is implemented for existing Fusion-project helpers with narrow
  internal-endpoint acceptance, but those helpers are no longer the default
  repo posture
- Phase 03 is implemented for repo-native sync verification across local
  config, latest saved project state, and visible remote Fusion settings
- Phase 04 is implemented for multi-surface visibility diagnosis, richer saved
  project context persistence on future create flows, and truthful
  classification of blocked states
- Phase 05 is implemented for branch-surface reconciliation and proves the
  current saved project is partially visible rather than generically blocked
- current Builder verification is still limited by metadata-surface
  visibility:
  the saved project is still not returned by either repo metadata read surface
  in the current auth context, and direct metadata read still returns `404`
- Phase 04 narrowed the blocked case to a concrete evidence trail:
  - the active Builder auth context matches the current CLI/env
  - `org-tree` returns zero projects
  - `projects?apiKey=...&userId=...` returns zero projects
  - direct read of the saved project returns `404` with and without `userId`
  - the older saved state lacks `spaceId`/`userId`, so the repo could not yet
    prove stale state vs space drift vs Builder-side limitation for that exact
    historical project
- Phase 05 then adds the missing Builder branch surface to the model and
  proves:
  - `projects/branches?projectId=...` returns live branch data for the saved
    project
  - the correct current visibility classification is `branch_visible_only`
  - `builder-get-project` must return partial visibility instead of `not_found`
  - `builder-verify-sync` must return a partial visibility state instead of
    pretending full remote settings comparison is possible

### Tranche 1: Local Tracking And Fresh Creation

Keep the repo's default Builder workflow centered on fresh project creation and
durable local tracking.

This tranche must preserve:

- fresh-by-default create behavior
- generated trackable names/branch names
- `.codex/builder-fusion-project.json` as the latest-project pointer
- `.codex/builder-fusion-project-history.jsonl` as the durable local ledger of
  created names/settings
- truthful docs that describe remote cleanup as manual and external

### Tranche 2: Explicit Existing-Project Read/Update

Add repo-native helpers for an existing Fusion project:

- `builder-get-project.sh`
- `builder-list-projects.sh`
- `builder-update-project.sh`

This tranche must:

- default to the latest saved project only as a convenience, not as a claim
  that Planner prefers one long-lived canonical remote project
- avoid creating a new Fusion project during read/update flows
- support explicit project ID override when local state is stale
- expose the currently effective remote runtime command and URL
- support updating an explicitly targeted existing Fusion project’s runtime
  command, runtime URL, and other project settings without forcing project
  recreation

### Tranche 3: Remote Environment And Profiles

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

### Tranche 4: Verify Workflows

Add:

- `builder-verify-sync.sh`

This tranche must let a repo user answer, in one command:

- which latest saved or explicitly targeted Fusion project is in scope
- whether local saved state matches visible remote project state
- whether the remote runtime command and env vars match the selected profile
- whether CMS sync metadata points at the same runtime/proxy values

`builder-ensure-project.sh` is no longer a default planned direction. A future
ensure-style helper should be promoted only if Planner explicitly decides it
still wants that behavior despite duplicate-project and visibility risks.

### Tranche 5: Safer Connection, Indexing, And Handoff

Tighten the existing Builder wrappers so they:

- make fresh project creation versus targeted existing-project mutation explicit
  in command output
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

- `.codex/builder-fusion-project.json` is the repo-local pointer to the latest
  created Fusion project used for immediate follow-on commands
- `.codex/builder-fusion-project-history.jsonl` is the durable local ledger for
  created project identities and create-time settings
- the latest-project pointer must not silently drift from the project being
  updated or verified in a follow-on command

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

1. a repo user can create a fresh Builder Fusion project with a trackable name
   and durable local record of the exact settings used
2. a repo user can inspect the latest saved or explicitly targeted Fusion
   project and its effective remote runtime settings without using manual curl
   or package-internals inspection
3. a repo user can update an explicitly targeted existing Fusion project’s
   runtime command, runtime URL, and environment variables from the repo when
   Builder visibility allows
4. a repo user can apply a named `mock-full-pipeline` profile that persists the
   mock runtime setting remotely for an explicitly targeted visible Fusion
   project
5. the repo exposes a single verification command that reports whether Fusion
   project settings, local state, and CMS sync metadata agree for the project
   currently in scope
6. Builder workflow docs explicitly distinguish Fusion project config, CMS
   project sync, DSI behavior, and the repo’s fire-and-forget remote-project
   posture
7. failure modes like missing auth, stale saved project IDs, visibility gaps,
   and plan-gated indexing are reported as explicit operator-facing states
   rather than ambiguous command failures

## Verification Plan

Implementation slices promoted from this parent spec must include proof for the
relevant subset of:

- authenticated Builder CLI status
- fresh project creation with the expected generated naming/tracking contract
- local ledger persistence in `.codex/builder-fusion-project-history.jsonl`
- readback of the target Fusion project before update when the slice involves
  explicit existing-project mutation
- update of remote runtime command and/or env var for a known project ID when
  Builder visibility allows
- readback after update showing the persisted change when the slice claims live
  remote mutation support
- application of at least one named profile, including `mock-full-pipeline`,
  when the slice claims remote profile support
- verification command output demonstrating aligned, drifted, and
  visibility-partial states where relevant
- documentation examples that match the implemented commands

## Rollback And Fallback

- if remote Fusion project update proves unsupported through the current
  Builder APIs, the implementation must stop at an explicit read-only helper
  tranche rather than claiming remote config management is complete
- local Builder launch must remain functional even if remote project update
  helpers are unavailable
- fresh project creation and local history tracking must remain functional even
  if existing-project readback stays partially blocked
- CMS sync must remain independently usable and must not be blocked by Fusion
  project update failures

## Open Questions

- what exact Builder API or dev-tools internal contract is stable enough for
  updating an existing Fusion project’s settings
- whether remote environment variables are project-scoped, branch-scoped, or
  stored only in a broader setup object
- whether Builder exposes last-indexed commit or similar sync metadata without
  enterprise-only features
- whether Planner should keep existing-project update helpers as a maintained
  secondary path if Builder never exposes reliable metadata readback
- whether a repo-local history inspection helper should be promoted as the next
  bounded Builder implementation slice

## Readiness Judgment

This parent capability spec remains intentionally **not** ready for direct
implementation as one broad change. It continues to serve as the source
planning artifact for follow-on bounded implementation slices.

The current honest next move is now narrower:

- synchronize the child Builder specs so they stop implying long-lived
  existing-project management as the default repo strategy, and
- decide whether Planner wants a small repo-local history inspection helper for
  `.codex/builder-fusion-project-history.jsonl`

No additional child slice is currently promoted automatically beyond Phase 05.
