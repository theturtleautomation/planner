# Builder Phase D Repo-Local DSI MCP Setup Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent Planning:** [Builder Developer Docs Phase A Exhaustive Analysis](/home/thetu/planner/docs/builder-developer-docs-phase-a-exhaustive-analysis.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  

## 1. Purpose

Make Builder DSI available as a repo-local capability for Planner without
blurring it into Fusion project management or CMS content mutation.

This slice is about local repo setup, operator clarity, and design-system-aware
Codex workflow. It is not about redesigning Planner's UI or broadening Builder
Fusion helpers.

## 2. Problem

Planner's Builder workflow docs currently say:

- Builder CMS is available through a repo-local plugin
- Builder DSI is still user-global unless the operator adds it manually

That leaves DSI in a weaker state than the rest of the repo-local Builder
workflow:

- two users on the same repo can have different DSI availability
- Codex setup for design-system-aware Builder work is not reproducible from the
  repo alone
- the docs acknowledge repo-local DSI as plausible, but the repo does not yet
  encode it

## 3. User Outcome

After this phase, a repo user should be able to:

1. discover Builder DSI from the repo itself instead of relying on a
   user-global manual setup
2. understand that DSI is the design-system surface, distinct from Fusion
   runtime config and CMS content sync
3. verify whether repo-local DSI is available in the current Codex session
4. use the same repo-local setup model for Builder CMS and Builder DSI

## 4. Scope

### In Scope

- repo-local plugin or MCP config for Builder DSI
- docs for enabling and verifying that repo-local DSI is available
- operator guidance separating Fusion, CMS, and DSI responsibilities
- bounded verification steps for the repo-local DSI setup

### Out Of Scope

- design-system indexing strategy redesign
- Builder Fusion project CRUD or runtime settings work
- CMS model/content changes
- forcing DSI usage into unrelated Planner workflows

## 5. Contract

- DSI must remain explicitly separate from Fusion project lifecycle management
  and CMS content sync
- repo-local DSI setup should follow the same discoverable repo pattern used
  for Builder CMS when practical
- the repo should not overclaim that DSI is required for all Builder work; it
  is the design-system-aware surface
- docs must explain when to use DSI and when not to use it

## 6. Required Decisions

### 6.1 Repo-local shape

Required direction:

- add a repo-local configuration path for Builder DSI instead of relying only
  on `codex mcp add ...` in user-global state
- keep the setup as simple and restart-tolerant as the repo allows

### 6.2 Workflow separation

Required direction:

- the docs must clearly separate:
  - Fusion runtime/project config
  - Builder CMS content operations
  - Builder DSI design-system work

### 6.3 Verification

Required direction:

- the repo should give one clear way to check whether Builder DSI is visible in
  the current environment
- the setup should fail clearly when required auth or binaries are missing

## 7. Touched Surfaces

- repo-local plugin and/or MCP config files for Builder DSI
- `.agents/plugins/marketplace.json` if repo discovery needs to be updated
- `docs/builder-local-workflow.md`
- `README.md` if the Builder tooling overview needs the same clarification
- optional shared Builder workflow references if they should point at the new
  repo-local DSI path

## 8. Acceptance Criteria

1. Planner exposes a repo-local Builder DSI setup path instead of only a
   user-global manual command
2. docs clearly explain what DSI is for and how it differs from Fusion config
   and CMS sync
3. the repo includes a bounded way to verify repo-local DSI availability
4. the new setup does not overclaim DSI as part of Fusion project-management
   behavior

## 9. Verification Plan

- inspect the repo-local plugin or MCP config artifacts
- verify discovery or listing behavior for the repo-local DSI setup
- review `docs/builder-local-workflow.md` for truthful separation of Fusion,
  CMS, and DSI
- confirm any verification instructions are runnable and specific

## 10. Rollback / Fallback

If a full repo-local DSI plugin proves too broad:

- land a smaller repo-owned config scaffold and truthful docs first
- keep the operator path explicit and reproducible
- do not leave DSI as an undocumented user-global side note

## 11. Open Questions

1. Should repo-local DSI mirror the existing Builder CMS plugin layout exactly,
   or is a lighter repo-local MCP config enough?
2. Does the repo want a thin helper command for DSI health/discovery, or is
   documentation plus plugin discovery sufficient?

## 12. Implementation Update

Implemented on 2026-04-01.

What landed:

- added a repo-local Builder DSI plugin at
  [plugins/planner-builder-dsi](/home/thetu/planner/plugins/planner-builder-dsi)
  with:
  - [plugin.json](/home/thetu/planner/plugins/planner-builder-dsi/.codex-plugin/plugin.json)
  - [.mcp.json](/home/thetu/planner/plugins/planner-builder-dsi/.mcp.json)
- registered the DSI plugin in
  [.agents/plugins/marketplace.json](/home/thetu/planner/.agents/plugins/marketplace.json)
- added
  [builder-dsi-status.sh](/home/thetu/planner/scripts/builder-dsi-status.sh)
  and the matching `make builder-dsi-status` target for repo-local DSI
  verification
- updated [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
  and [README.md](/home/thetu/planner/README.md) so Fusion, CMS, and DSI are
  described as separate Builder surfaces with different responsibilities

Verification completed:

- `bash -n scripts/builder-dsi-status.sh`
- `jq . plugins/planner-builder-dsi/.codex-plugin/plugin.json`
- `jq . plugins/planner-builder-dsi/.mcp.json`
- `jq . .agents/plugins/marketplace.json`
- `./scripts/builder-dsi-status.sh`

Verification result:

- repo-local DSI plugin wiring parses correctly
- marketplace discovery includes `planner-builder-dsi`
- local prerequisites passed with `Node.js v22.18.0`
- `npx @builder.io/dev-tools@latest dsi-mcp --help` completed successfully
- existing Codex sessions may still require a restart to discover the newly
  added repo-local plugin
