# Repo Graph Workflow

## Purpose

Provide a repo-local, graph-backed repo-understanding path for Planner that can return bounded cross-file context during active sessions without collapsing code-graph evidence into blueprint truth.

## Files

- `plugins/planner-repo-graph/.codex-plugin/plugin.json`
- `plugins/planner-repo-graph/.mcp.json`
- `.codex/skills/repo-graph/SKILL.md`
- `scripts/repo_graph.py`
- `scripts/repo_graph_mcp.py`
- `scripts/repo-graph.sh`
- `scripts/repo-graph-mcp.sh`
- `scripts/repo-graph-status.sh`

## Commands

- Build or refresh the repo graph:
  - `./scripts/repo-graph.sh build`
- Refresh only when tracked files changed:
  - `./scripts/repo-graph.sh update`
- Recompute communities from the current graph:
  - `./scripts/repo-graph.sh cluster-only`
- Show graph status:
  - `./scripts/repo-graph.sh status`
  - `./scripts/repo-graph-status.sh`
- Query bounded graph context:
  - `./scripts/repo-graph.sh query "how do project-ledger and AGENTS routing connect"`
- Trace a path between two concepts:
  - `./scripts/repo-graph.sh path "project-ledger" "AGENTS.md"`
- Explain one node in plain language:
  - `./scripts/repo-graph.sh explain "repo-graph"`
- Inspect one community:
  - `./scripts/repo-graph.sh community 0`
- Show the most connected nodes:
  - `./scripts/repo-graph.sh god-nodes`
- Refresh after relevant OMX execution work:
  - `./scripts/repo-graph.sh post-execution-refresh <changed-paths...>`
- Check MCP bootstrap lifecycle state:
  - `./scripts/repo-graph-mcp.sh status`
- Ensure MCP bootstrap explicitly:
  - `./scripts/repo-graph-mcp.sh ensure`

## Scope Boundaries

- V1 focuses on **code and local repo docs**.
- V1 defers **export/report** features.
- Repo graph output is **retrieval/evidence**, not automatic blueprint truth.
- If repo-graph evidence is used to justify an automated ledger/routing mutation, that mutation should leave an inspectable why-trail.
- The current trust model may distinguish high-confidence and medium-confidence auto-mutations from low-confidence non-mutations while preserving the same evidence-only boundary.
- Trivial file/symbol lookups should stay on `omx explore` or direct reads.
- Build fidelity now tracks explicit manifest freshness so status/update can distinguish clean vs dirty graphs before forcing a rebuild.

## Routing Intent

For broad repo-understanding work spanning many files or asking for cross-file relationships, prefer the repo-graph workflow first when available. For simple anchored lookups, keep using `omx explore`.

## Enforcement Heuristics

Prefer `repo-graph` first when:
- the user asks how two areas connect
- the user asks what bridges, depends on, or relates across multiple files
- there is no single obvious file/symbol anchor
- repeated file reads would otherwise be needed just to build context

Prefer `omx explore` or direct reads first when:
- the user already names a concrete file, symbol, route, or error site
- the question is a tiny lookup rather than a graph/context question
- repo-graph returns weak or noisy signal for the current question

## Phase 1 Build Fidelity Notes

- `build` forces a full graph rebuild.
- `update` only rebuilds when tracked inputs changed since the last manifest snapshot.
- `cluster-only` recomputes community metadata from the existing graph without rescanning repo files.
- `status` reports whether tracked repo inputs differ from the last manifest snapshot.

## Phase 2 Query / Tooling Notes

- `query` now supports bounded traversal modes and highlights top matches before supporting context.
- `path` prints a relation-aware hop-by-hop explanation instead of raw node IDs.
- `explain` now includes a short “why it matters” summary plus graph evidence.
- `community`, and `god-nodes` expose Graphify-style graph interrogation without broadening the corpus scope.

## Clustering Parity Notes

- community metadata now aims to be more informative than raw connected components alone
- oversized communities should be split more aggressively when the graph structure supports it
- community views may surface lightweight cohesion signals to help judge whether a cluster is meaningful
- `community` output may now show cohesion alongside the sampled labels for faster cluster inspection

## Freshness Follow-on Notes

- Generic always-on watch mode remains deferred.
- The preferred next freshness path is **OMX-triggered post-execution refresh/update**.
- `post-execution-refresh` is Ralph-first: it should run only when the changed-file set intersects repo-graph-tracked code/docs paths.
- The command should report one explicit outcome: `skipped`, `refreshed-via-update`, `refreshed-via-rebuild`, or `refresh-failed`.

## Deferred Queue After Phase 3

Keep these out of Phase 3 implementation but queued afterward:
- query / explain quality hardening
- richer clustering parity
- finer-grained incremental update beyond rebuild-on-change
- watch-mode parity
- MCP bootstrap ergonomics

## MCP Bootstrap Lifecycle States

- `not_bootstrapped`
- `bootstrapped_healthy`
- `bootstrapped_unhealthy`
- `refresh_needed`

Bootstrap hardening should improve startup and inspectability ergonomics only; it must not change repo-graph content or graph-quality semantics.
