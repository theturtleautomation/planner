# Graphify / OMX Handoff

## High-level outcome

We explored `safishamsi/graphify`, decided **not** to transplant the Claude skill shell directly, and instead built an **OMX-native repo-graph workflow** that preserves the spirit/useful features of Graphify inside this repo.

Core decision:
- **Portable in spirit/backend, not as-is as a Claude skill**
- We wanted **Graphify feature parity + tight OMX coupling/enforcement**
- We explicitly **did not** want to invent or extend the Graphify platform itself

## Key planning artifacts

### Deep-interview specs
- `.omx/specs/deep-interview-graphify-portability.md`
- `.omx/specs/deep-interview-graphify-omx-integration-plan.md`
- `.omx/specs/deep-interview-repo-graph-next-step.md`
- `.omx/specs/deep-interview-repo-graph-deferred-queue-next-steps.md`

### Main planning PRDs / test specs
- `.omx/plans/prd-graphify-inspired-omx-repo-graph-default-path.md`
- `.omx/plans/test-spec-graphify-inspired-omx-repo-graph-default-path.md`

- `.omx/plans/prd-repo-graph-graphify-parity-three-phase-roadmap.md`
- `.omx/plans/test-spec-repo-graph-graphify-parity-three-phase-roadmap.md`

- `.omx/plans/prd-repo-graph-post-execution-refresh-and-freshness.md`
- `.omx/plans/test-spec-repo-graph-post-execution-refresh-and-freshness.md`

- `.omx/plans/prd-repo-graph-answer-quality-hardening.md`
- `.omx/plans/test-spec-repo-graph-answer-quality-hardening.md`

- `.omx/plans/prd-repo-graph-mcp-bootstrap-hardening.md`
- `.omx/plans/test-spec-repo-graph-mcp-bootstrap-hardening.md`

- `.omx/plans/prd-repo-graph-clustering-parity.md`
- `.omx/plans/test-spec-repo-graph-clustering-parity.md`

## What was implemented

### 1. Repo-graph v1
Implemented an OMX-native repo graph system with:
- repo-local plugin wiring
- repo-local MCP wiring
- repo-graph CLI/broker skill
- workflow docs
- routing guidance in OMX skills + `AGENTS.md`

Main files:
- `plugins/planner-repo-graph/.codex-plugin/plugin.json`
- `plugins/planner-repo-graph/.mcp.json`
- `scripts/repo_graph.py`
- `scripts/repo_graph_mcp.py`
- `scripts/repo-graph.sh`
- `scripts/repo-graph-mcp.sh`
- `scripts/repo-graph-status.sh`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

### 2. Phase 1 — extraction/build fidelity
Implemented:
- manifest-backed freshness tracking
- `update` command
- `cluster-only` command
- `status` with clean/dirty reporting
- community metadata persisted into graph

Main files touched:
- `scripts/repo_graph.py`
- `scripts/repo_graph_test.py`
- `scripts/repo-graph.sh`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

### 3. Phase 2 — query / explain / path tooling
Implemented:
- DFS query mode
- top-matches-first query output
- better explain output with “why it matters” + evidence
- better path narration with hop meaning
- `community`
- `god-nodes`
- improved MCP tool parity

Main files touched:
- `scripts/repo_graph.py`
- `scripts/repo_graph_mcp.py`
- `scripts/repo_graph_test.py`
- `scripts/repo-graph.sh`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

### 4. Phase 3 — OMX enforcement / routing coupling
Implemented:
- stronger heuristics for when to prefer `repo-graph` vs `omx explore`
- stronger repo-understanding routing guidance in:
  - `AGENTS.md`
  - `.codex/skills/plan/SKILL.md`
  - `.codex/skills/deep-interview/SKILL.md`
  - `.codex/skills/ralph/SKILL.md`
  - `.codex/skills/team/SKILL.md`
  - `.codex/skills/repo-graph/SKILL.md`
- durable deferred queue in docs

### 5. Freshness/performance follow-on
Implemented:
- Ralph-first `post-execution-refresh`
- explicit outcomes:
  - `skipped`
  - `refreshed-via-update`
  - `refreshed-via-rebuild`
  - `refresh-failed`

Main files touched:
- `scripts/repo_graph.py`
- `scripts/repo_graph_test.py`
- `scripts/repo-graph.sh`
- `.codex/skills/repo-graph/SKILL.md`
- `.codex/skills/ralph/SKILL.md`
- `docs/repo-graph-workflow.md`
- `package.json`

### 6. Answer-quality hardening
Implemented:
- stronger ranking for anchored questions
- top-matches-first query structure
- improved explain/path readability
- benchmark-oriented tests

Main files touched:
- `scripts/repo_graph.py`
- `scripts/repo_graph_test.py`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

### 7. MCP bootstrap hardening
Implemented:
- explicit MCP lifecycle states:
  - `not_bootstrapped`
  - `bootstrapped_healthy`
  - `bootstrapped_unhealthy`
  - `refresh_needed`
- explicit `status` and `ensure` subcommands
- dedicated bootstrap helper
- clearer operator-visible readiness

Main files touched:
- `scripts/repo-graph-mcp.sh`
- `scripts/repo-graph-status.sh`
- `scripts/repo_graph_mcp_bootstrap.py`
- `scripts/repo_graph_mcp_bootstrap_test.py`
- `plugins/planner-repo-graph/.mcp.json`
- `package.json`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

### 8. Clustering parity
Implemented:
- bounded oversized-community splitting
- cohesion-like community signals
- richer community metadata
- improved community reporting
- clustering-related regression tests

Main files touched:
- `scripts/repo_graph.py`
- `scripts/repo_graph_test.py`
- `scripts/repo_graph_mcp.py`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`

## Current repo-graph command surface

Main commands:
- `./scripts/repo-graph.sh build`
- `./scripts/repo-graph.sh update`
- `./scripts/repo-graph.sh cluster-only`
- `./scripts/repo-graph.sh status`
- `./scripts/repo-graph.sh query "<question>"`
- `./scripts/repo-graph.sh query --dfs "<question>"`
- `./scripts/repo-graph.sh path "<source>" "<target>"`
- `./scripts/repo-graph.sh explain "<term>"`
- `./scripts/repo-graph.sh community <id>`
- `./scripts/repo-graph.sh god-nodes`
- `./scripts/repo-graph.sh post-execution-refresh <changed-paths...>`

MCP commands:
- `./scripts/repo-graph-mcp.sh run`
- `./scripts/repo-graph-mcp.sh status`
- `./scripts/repo-graph-mcp.sh ensure`

npm helpers:
- `npm run repo-graph:status`
- `npm run repo-graph:build`
- `npm run repo-graph:update`
- `npm run repo-graph:cluster`
- `npm run repo-graph:post-refresh`
- `npm run repo-graph:query`
- `npm run repo-graph:mcp`
- `npm run repo-graph:mcp:status`
- `npm run repo-graph:mcp:ensure`

## Current behavior / rules

### When to use repo-graph
Prefer `repo-graph` when:
- the task is broad repo understanding
- the user asks how things connect
- the user asks what bridges/depends on what
- there is no single obvious file/symbol anchor
- repeated file reads would otherwise be needed

Prefer `omx explore` or direct reads when:
- the task already names a concrete file/symbol/error site
- it’s a tiny bounded lookup
- repo-graph returns weak/noisy signal

### Important boundary
Repo-graph output is:
- **retrieval/evidence context**
- **not blueprint truth**
- **not automatic architecture truth**

## Verification that has already passed repeatedly

Across the work we repeatedly ran combinations of:
- `python3 -m py_compile ...`
- `python3 scripts/repo_graph_test.py`
- `python3 scripts/repo_graph_mcp_bootstrap_test.py`
- `bash -n scripts/repo-graph.sh scripts/repo-graph-status.sh scripts/repo-graph-mcp.sh`
- repo-graph command smoke checks
- MCP smoke checks
- `npm run build`
- `npm run lint`
- `npm test`

Build note:
- `npm run build` passes, but emits the same pre-existing Nitro/h3 warning:
  - `"send" is not exported by "node_modules/h3/dist/_entries/node.mjs"`
- This was treated as **pre-existing**, not caused by repo-graph work.

## Current remaining queue

Essentially only:
1. **optional later Team-surface ergonomics**
   - if you want repo-graph freshness/routing polished for team/multi-agent execution surfaces too

Everything else in the originally discussed Graphify/OMX roadmap has been implemented:
- freshness/performance
- answer quality
- MCP bootstrap ergonomics
- clustering parity

## Short interpretation

If another session asks “what happened with Graphify?” the answer is:

> We built an OMX-native repo-graph system inspired by Graphify, then executed a multi-pass roadmap to bring it closer to Graphify parity while keeping it tightly coupled to OMX and preserving the repo-graph-as-evidence boundary. We did not transplant the Claude skill shell and did not extend the Graphify platform itself.
