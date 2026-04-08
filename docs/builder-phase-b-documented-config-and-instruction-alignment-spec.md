# Builder Phase B Documented Config And Instruction Alignment Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent Planning:** [Builder Developer Docs Phase A Exhaustive Analysis](/home/thetu/planner/docs/builder-developer-docs-phase-a-exhaustive-analysis.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  

## Purpose

Turn the Phase A Builder docs analysis into concrete repo artifacts aligned to
Builder's documented developer workflow.

This slice is intentionally narrow. It does not try to solve undocumented
Fusion project CRUD/readback. It makes Planner stronger on the documented path:

- `builder.config.json`
- Builder-specific instruction files
- repo wrappers that inherit from the documented config

## Problem

After the Phase A docs pass, Planner had a clear conclusion but not yet the
corresponding repo artifacts:

- Builder's strongest documented path is local repo launch plus committed repo
  config
- Planner still relied mostly on wrapper-script hardcoding
- the repo had no `builder.config.json`
- the repo had no Builder-specific `.builderrules`
- launch/create wrappers duplicated runtime assumptions instead of inheriting
  them from one documented source of truth

## User Outcome

A repo user working with Builder should now be able to:

1. see a committed `builder.config.json` in the repo root
2. see Builder-specific code-generation rules in a committed `.builderrules`
3. launch Builder with repo-native wrappers that inherit command and URL from
   the documented config file
4. create Fusion projects with runtime settings aligned to the same repo config
   and default mock profile

## Scope

### In Scope

- add `builder.config.json`
- add root `.builderrules`
- make `scripts/builder-launch.sh` inherit defaults from `builder.config.json`
- make `scripts/builder-create-project.sh` inherit runtime command/URL from
  `builder.config.json`
- document the new repo contract

### Out Of Scope

- solving Builder's undocumented existing-project readback gap
- replacing internal-fallback Fusion helper scripts
- full Privacy Mode configuration
- DSI or CMS MCP redesign

## Implementation Result

This slice is implemented through:

- [builder.config.json](/home/thetu/planner/builder.config.json)
- [.builderrules](/home/thetu/planner/.builderrules)
- [builder-launch.sh](/home/thetu/planner/scripts/builder-launch.sh)
- [builder-create-project.sh](/home/thetu/planner/scripts/builder-create-project.sh)
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
- [README.md](/home/thetu/planner/README.md)

Implemented behavior:

- the repo now has a documented Builder config file with canonical server URL,
  runtime command, workspace folders, and shell allowlist
- the repo now has Builder-specific instruction rules separate from general
  repo instructions
- Builder launch now reads command and server URL defaults from
  `builder.config.json`
- Builder project creation now reads command and server URL defaults from
  `builder.config.json`
- Builder project creation still defaults to the safe Planner mock profile via
  `PLANNER_BUILDER_LLM_MOCK_MODE=full_pipeline`, but users can opt out with
  `disabled`

## Verification Evidence

Verified with:

1. `bash -n scripts/builder-launch.sh scripts/builder-create-project.sh`
2. `jq . builder.config.json`
3. `./scripts/builder-create-project.sh --dryrun`
4. `git diff --check`

## Outcome

Planner is now better aligned with Builder's documented developer surfaces.
The repo still treats existing Fusion project CRUD/readback as internal
fallback, but the local-repo contract is no longer implicit or wrapper-only.
