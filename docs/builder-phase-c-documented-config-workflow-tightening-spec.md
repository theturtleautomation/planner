# Builder Phase C Documented Config Workflow Tightening Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent Planning:** [Builder Phase B Documented Config And Instruction Alignment Spec](/home/thetu/planner/docs/builder-phase-b-documented-config-and-instruction-alignment-spec.md), [Builder Developer Docs Phase A Exhaustive Analysis](/home/thetu/planner/docs/builder-developer-docs-phase-a-exhaustive-analysis.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  

## 1. Purpose

Tighten Planner's documented Builder-config workflow so the committed
`builder.config.json` contract is easier to trust, easier to inspect, and
harder to misapply.

Phase B created the repo-local config and made the main launch/create wrappers
inherit from it. This follow-on keeps the work on the documented Builder path
and focuses on making that path operationally sharper.

## 2. Problem

Planner now has a committed `builder.config.json`, but the overall workflow is
still looser than it should be.

Current pain points:

- the effective Builder runtime contract is still spread across config, wrapper
  behavior, alternate configs, and docs
- it is still too easy to confuse the default frontend-mock UI-review path
  with the alternate server-backed path
- wrapper output does not yet give one clear answer for "which config am I
  using, what URL/command/profile does it resolve to, and is that contract
  coherent"
- config validation and drift checks remain mostly implicit

The result is that the documented path exists, but it is not yet as self-
describing as it should be.

## 3. User Outcome

After this phase, a repo user should be able to:

1. see exactly which Builder config file is active
2. see the effective runtime URL, command, and profile derived from that config
3. validate that a config is structurally coherent before launching or creating
   a project
4. distinguish the default frontend-mock UI-review workflow from the alternate
   server-backed workflow without rereading multiple docs
5. rely on wrapper output and docs that tell the same story

## 4. Scope

### In Scope

- tighter wrapper behavior around `builder.config.json` and any explicit
  override path
- config inspection and validation behavior
- clearer operator-facing output about effective runtime contract
- stronger documentation examples for default and alternate Builder profiles
- bounded dry-run or verification workflow if needed

### Out Of Scope

- undocumented Fusion project CRUD/readback changes
- DSI MCP setup itself
- CMS content-model redesign
- changing the chosen default Builder runtime contract

## 5. Contract

- `builder.config.json` remains the canonical default Builder UI-review config
- alternate configs such as `builder.server.config.json` remain explicit opt-in
  overrides rather than competing defaults
- wrapper output must state the effective config path, runtime URL, command,
  and relevant profile/env assumptions
- config tightening must not silently reintroduce the wrong `4174` default for
  UI-review work
- docs and wrapper output must agree on the same runtime story

## 6. Required Workflow Decisions

### 6.1 Default versus alternate path

Required direction:

- the default repo Builder workflow continues to target frontend-mock UI review
- the alternate server-backed workflow remains available, but must be labeled
  as alternate everywhere the user would reasonably choose between them

### 6.2 Validation and inspection

Required direction:

- the repo should expose one simple way to inspect the effective Builder config
- the repo should expose one simple way to validate or dry-run the resolved
  config before mutation or launch
- failures should point at the exact config field or assumption that is wrong

### 6.3 Wrapper clarity

Required direction:

- launch/create/update helpers that rely on Builder config should print the
  resolved contract clearly
- output should distinguish local-only launch behavior from remote-persisted
  project settings where relevant

## 7. Touched Surfaces

- `builder.config.json`
- `builder.server.config.json` if needed for clearer alternate-path structure
- `scripts/builder-launch.sh`
- `scripts/builder-create-project.sh`
- `scripts/builder-update-project.sh` if config inheritance needs the same
  clarity
- `Makefile` if wrapper targets need clearer naming or help text
- `docs/builder-local-workflow.md`
- `README.md`

## 8. Acceptance Criteria

1. the repo exposes a clearer documented-config workflow around
   `builder.config.json`
2. wrapper output identifies the effective config path and the resolved runtime
   URL/command contract
3. the default frontend-mock path and alternate server-backed path are clearly
   distinguished in both docs and wrapper behavior
4. config-validation or dry-run behavior catches obvious contract mistakes
   before launch or project creation
5. the resulting workflow stays on Builder's documented repo-config path rather
   than widening into undocumented project-management work

## 9. Verification Plan

- `bash -n` for touched scripts
- config validation or dry-run checks for default and alternate config paths
- `jq . builder.config.json`
- `jq . builder.server.config.json` if touched
- doc review to confirm workflow examples match the implemented wrapper output

## 10. Rollback / Fallback

If a full validation command is too broad for one slice:

- ship clearer wrapper output and a smaller config-inspection helper first
- keep the documented default-versus-alternate workflow explicit
- do not revert to ambiguous wrapper behavior that requires docs archaeology to
  understand the active contract

## 11. Open Questions

1. Should this slice add a dedicated `builder-print-config.sh` or
   `builder-validate-config.sh`, or keep the functionality inside existing
   wrappers?
2. Should `make` targets gain a small help surface that prints the default and
   alternate Builder paths?

## 12. Implementation Update

Implemented on 2026-04-01.

What landed:

- added shared config resolution and validation in
  [builder-config-common.sh](/home/thetu/planner/scripts/builder-config-common.sh)
- added explicit inspection and validation entrypoints:
  [builder-print-config.sh](/home/thetu/planner/scripts/builder-print-config.sh)
  and
  [builder-validate-config.sh](/home/thetu/planner/scripts/builder-validate-config.sh)
- updated
  [builder-launch.sh](/home/thetu/planner/scripts/builder-launch.sh),
  [builder-create-project.sh](/home/thetu/planner/scripts/builder-create-project.sh),
  and
  [builder-update-project.sh](/home/thetu/planner/scripts/builder-update-project.sh)
  to validate config, print the resolved contract, and distinguish local launch
  from remote-persisted create/update flows
- added matching `make` targets for default and alternate config inspection and
  validation
- synced [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
  and [README.md](/home/thetu/planner/README.md) so docs and wrapper output
  describe the same Builder runtime split

Verification completed:

- `bash -n scripts/builder-config-common.sh scripts/builder-print-config.sh scripts/builder-validate-config.sh scripts/builder-launch.sh scripts/builder-create-project.sh scripts/builder-update-project.sh`
- `jq . builder.config.json`
- `jq . builder.server.config.json`
- `./scripts/builder-print-config.sh`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-print-config.sh`
- `./scripts/builder-validate-config.sh`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-validate-config.sh`
- `./scripts/builder-create-project.sh --dryrun`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-create-project.sh --dryrun`
- `./scripts/builder-update-project.sh --dryrun`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-update-project.sh --dryrun`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json PLANNER_BUILDER_LLM_MOCK_MODE=disabled ./scripts/builder-update-project.sh --dryrun`

Notable implementation detail:

- syncing the default frontend-mock config or the server-backed config with
  `PLANNER_BUILDER_LLM_MOCK_MODE=disabled` now clears Builder project
  environment variables instead of trying to drive a `live` profile mutation,
  because the repo-owned contract for those cases is an empty env set
  (`environmentVariables: []`)
