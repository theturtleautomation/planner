# Builder Fusion Phase 02 Existing Project Helper Contract Spec

**Status:** implemented  
**Date:** 2026-03-29  
**Parent Spec:** [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  
**Prerequisite:** [Builder Fusion Phase 01 API-Grounded Skill And Existing Project Contract Spec](/home/thetu/planner/docs/builder-fusion-phase-01-api-grounded-skill-and-existing-project-contract-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  

> Planning sync update (2026-04-02): this implemented slice remains the repo’s
> helper contract for inspecting or updating an explicitly targeted existing
> Builder Fusion project. It is no longer the default Planner Builder strategy.
> Planner now creates fresh Fusion projects by default and uses
> `.codex/builder-fusion-project.json` only as the latest-project pointer for
> immediate follow-on commands, with `.codex/builder-fusion-project-history.jsonl`
> as the durable local source of truth for created project identities/settings.

## Purpose

Split the deferred existing-project helper implementation into its own bounded
child slice so Planner can decide on the internal-endpoint risk explicitly,
without reopening the already-completed Builder skill/reference hardening work.

This spec is specifically about helper behavior for an explicitly targeted or
latest-saved Fusion project, not about the repo’s default creation posture or
general Builder skill guidance.

## Problem

Planner now has:

- a truthful local Builder workflow
- a completed Builder skill/reference layer that distinguishes documented
  Builder APIs from internal Fusion-project fallbacks
- a latest-saved Fusion project identity in
  `.codex/builder-fusion-project.json`

But it still lacks repo-native helper flows for the most important operational
need:

- inspect the existing Fusion project by saved project ID
- update the existing Fusion project's runtime command and URL
- manage existing Fusion project environment variables
- verify whether local intent and remote Fusion state agree

This is now a narrower problem than the original parent capability gap. The
remaining blocker is not conceptual confusion. It is implementation risk:

- Builder's analyzed public docs describe Project settings semantics but do not
  establish a stable public Fusion project CRUD/settings API
- Planner can probably implement existing-project helpers by using internal
  Builder endpoints or dev-tools-adjacent behavior, but doing so should be an
  explicit repo decision

## User Outcome

A repo user working on Planner with Builder should be able to:

1. inspect an explicitly targeted or latest-saved Fusion project without
   creating a new project
2. update that project's runtime command and URL from the repo
3. manage that project's environment variables from the repo
4. apply a named runtime profile to that project
5. verify whether that project matches local Planner intent
6. see clearly when a helper is using documented Builder semantics versus an
   internal fallback transport

## Scope

### In Scope

- `builder-get-project.sh`
- `builder-list-projects.sh`
- `builder-update-project.sh`
- helper-level support for saved project ID reuse and explicit override
- helper-level output that classifies documented semantics versus internal
  fallback transport
- helper contracts for runtime command, runtime URL, env vars, and profile
  application

### Out Of Scope

- reopening Builder skill/reference hardening
- broad CMS model/content work
- Builder DSI changes
- enterprise-only indexing remediation
- broad deployment automation outside targeted Builder Fusion project helpers

## Preconditions

This slice depends on Phase 01 already being complete, because the repo now has
the documented vocabulary and safety guidance needed to keep helper output
truthful.

Planner is now explicitly accepting the internal-endpoint risk for this narrow
helper slice.

That acceptance is bounded:

- only existing Fusion-project helper behavior for an explicitly targeted or
  latest-saved project is in scope
- helper output must continue to call the transport an internal fallback, not a
  documented Builder API
- the repo is not treating internal endpoints as a generally stable Builder
  integration contract

If Builder later documents a supported project-management API, this slice
should be tightened to use that contract instead.

## Risk Decision

Planner accepts the current internal-endpoint risk for implementing:

- `builder-get-project.sh`
- `builder-list-projects.sh`
- `builder-update-project.sh`

under the following guardrails:

1. helpers may default to the latest saved project identity in
   `.codex/builder-fusion-project.json` as a convenience
2. helpers must not recreate a project during read/update flows
3. helpers must support explicit project ID override for stale local state
4. helpers must print that they are using documented settings semantics plus an
   internal fallback transport
5. helpers must keep destructive or ambiguous behavior out of the normal path
6. helper output must warn before mutating the latest saved or explicitly
   targeted project

## Proposed Helper Surface

### 1. `builder-get-project.sh`

Responsibilities:

- read `.codex/builder-fusion-project.json` by default
- allow `--project-id` override
- print the current effective project identity and settings relevant to Planner
- avoid project creation
- print whether the read path is using an internal fallback

### 2. `builder-list-projects.sh`

Responsibilities:

- enumerate candidate Fusion projects for the current Builder space/user
- help recover from stale local saved-project state
- avoid silently rewriting `.codex/builder-fusion-project.json`

### 3. `builder-update-project.sh`

Responsibilities:

- target the saved Fusion project by default
- allow updating runtime command, runtime URL, and related settings without
  recreating the project
- refuse destructive project replacement behavior in a normal update path
- print the exact setting classes being changed
- warn when the mutation depends on an internal fallback transport

### 4. Environment/Profile Follow-On

This slice may define the contract for env/profile application, but full env
helper implementation can remain a follow-on if necessary.

At minimum, the helper design must preserve:

- `PLANNER_LLM_MOCK=full_pipeline` as the canonical `mock-full-pipeline`
  profile value
- separation between local launch-only behavior and remote persisted project
  settings

## Contracts And Touched Surfaces

### Repo Scripts

- `scripts/builder-create-project.sh`
- new existing-project helpers added by this slice

### Repo State

- `.codex/builder-fusion-project.json`

### Skill/Reference Context

- `/home/thetu/.codex/skills/builder-workflow/SKILL.md`
- `/home/thetu/.codex/skills/builder-workflow/references/api-surfaces.md`
- `/home/thetu/.codex/skills/builder-workflow/references/fusion-project-settings.md`

The Phase 01 skill work remains the source of truth for terminology and safety
language. This slice should reuse that language instead of redefining it.

## Acceptance Criteria

This helper slice is coherent when all of the following are true:

1. the saved Fusion project in `.codex/builder-fusion-project.json` is the
   default target for helper operations
2. the helper design explicitly forbids project recreation during read/update
   flows
3. the helper design supports explicit project ID override for stale saved
   state
4. helper output clearly distinguishes documented Builder settings semantics
   from internal fallback transport
5. the slice keeps runtime command/URL updates and env/profile work within a
   bounded helper-focused scope
6. the slice does not overclaim a documented public Fusion project
   CRUD/settings API that the analyzed Builder docs did not establish

## Verification Plan

Planning verification for this split is:

1. confirm the new Phase 02 spec is linked from the parent Builder capability
   thread
2. confirm the doc index and project plan reference the new spec
3. confirm the project plan now treats helper work as its own slice separate
   from the implemented Phase 01 skill/reference slice
4. confirm the spec leaves risk acceptance explicit instead of silently
   normalizing internal endpoints

## Rollback / Fallback

If Planner decides not to accept internal-endpoint risk, this spec can remain
`draft` indefinitely without invalidating Phase 01.

If Builder later documents a supported project-management API, tighten this
slice around that contract and promote it without changing the completed Phase
01 work.

## Open Questions

1. Should env/profile helper commands be included in this same slice, or split
   into a dedicated Phase 03 after basic get/list/update helpers land?
2. Should helper scripts live only under `scripts/`, or should shared Builder
   skill scripts also gain generic equivalents?
3. What operator-facing approval language should the helpers print before
   mutating the canonical saved project?

## Implementation Result

This slice is now implemented through:

- `scripts/builder-list-projects.sh`
- `scripts/builder-get-project.sh`
- `scripts/builder-update-project.sh`
- shared Builder skill helpers under
  `/home/thetu/.codex/skills/builder-workflow/scripts/`

Implemented behavior includes:

- saved-project targeting by default from `.codex/builder-fusion-project.json`
- explicit `--project-id` override
- non-recreation guardrails for read/update flows
- output that labels the transport as `internal-fallback`
- update support for runtime command, runtime URL, install command, main
  branch, env vars, and named profiles
- explicit mutation warning for the canonical saved Fusion project

## Verification Evidence

This slice was verified with:

1. `bash -n` over the new shared and repo wrapper scripts
2. `./scripts/builder-list-projects.sh`
3. `./scripts/builder-get-project.sh`
4. `./scripts/builder-update-project.sh --project-id ee5c85a61a1447dbae6b7c7765e80f20 --dev-server-command 'PLANNER_LLM_MOCK=full_pipeline cargo run -p planner-server -- --port 4174 --static-dir ./planner-solid/dist/static' --dev-server-url http://127.0.0.1:4174 --profile mock-full-pipeline --dryrun`
5. `./scripts/builder-update-project.sh --profile mock-full-pipeline --dryrun`

Verified result:

- the helper surface is implemented and behaves coherently for list, get,
  saved-project targeting, and dry-run update payload generation
- the current Builder auth context still does not expose the saved Fusion
  project in the project list, so `builder-get-project.sh` returns a truthful
  `not_found` payload and live update is intentionally blocked when remote
  settings cannot be merged safely

## Follow-On Note

The next valid move is not more Phase 02 work. It is either:

1. verify the live update path from a Builder auth context that can actually
   see the saved project, or
2. continue to the next bounded parent-capability tranche for broader remote
   env/profile and sync verification workflows
