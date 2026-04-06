---
name: repo-graph
description: Build and query a bounded repo graph for repo-understanding tasks
argument-hint: "[query|status|build|update|cluster-only|path|explain|node|neighbors|community|god-nodes|post-execution-refresh] [args]"
---

# Repo Graph

Use the repo-local graph-backed workflow when broad repo understanding needs cross-file relationships or bounded retrieval across many files.

## Use When
- the task needs cross-file context rather than one-file lookup
- the user asks how modules connect, what depends on what, or what bridges two concepts
- the task would otherwise require reading many files just to build context

## Do Not Use When
- the task is a trivial path/symbol lookup that `omx explore` already answers cheaply
- the task depends on curated blueprint truth rather than code/doc retrieval
- the repo is too small or noisy for graph build/query to add value
- the user already gave one concrete file/symbol/error site and the answer probably lives there

## Boundary
- Repo graph output is **evidence/query context**, not automatic blueprint truth
- Export/reporting is out of scope for this v1 skill
- V1 focuses on code and local repo docs only
- If repo-graph is used to justify an automatic ledger/routing mutation, the mutation should leave an inspectable why-trail; graph evidence still does not become blueprint truth
- Current trust policy may distinguish high-confidence and medium-confidence auto-mutations from low-confidence non-mutations, but repo-graph still supplies evidence rather than truth.

## Commands
- `./scripts/repo-graph.sh build`
- `./scripts/repo-graph.sh update`
- `./scripts/repo-graph.sh cluster-only`
- `./scripts/repo-graph.sh status`
- `./scripts/repo-graph.sh query "<question>"`
- `./scripts/repo-graph.sh path "<source>" "<target>"`
- `./scripts/repo-graph.sh explain "<term>"`
- `./scripts/repo-graph.sh node "<term>"`
- `./scripts/repo-graph.sh community <id>`
- `./scripts/repo-graph.sh god-nodes`
- `./scripts/repo-graph.sh post-execution-refresh <changed-paths...>`
- `./scripts/repo-graph-status.sh`
- `./scripts/repo-graph-mcp.sh status`
- `./scripts/repo-graph-mcp.sh ensure`

## Workflow
1. If no explicit command is provided, default to `status` for orientation.
2. Prefer `update` over `build` when you want Graphify-style freshness behavior without forcing unnecessary rebuilds.
3. Use `cluster-only` when you only need current graph communities regrouped.
4. Use `repo-graph` by default when the question is about connections, bridges, dependency paths, or broad repo context without one obvious file/symbol anchor.
5. Keep `omx explore` or direct reads as the default when the task is already anchored to one concrete file, symbol, or error site.
6. For broad repo-understanding tasks, run `./scripts/repo-graph.sh query "<question>"` before falling back to whole-file reads.
7. Use `path`, `explain`, `community`, and `god-nodes` when you need more specific graph interrogation after the first query pass; `query` should surface top matches first and keep supporting graph evidence visible.
8. Prefer `community` when you want to inspect whether a cluster is coherent enough to trust; cohesion/sample metadata should help you decide whether the cluster is meaningful.
9. If the answer needs exact file/symbol confirmation after the graph result, follow up with `omx explore` or direct file reads.
10. If graph build/query fails or provides no useful signal, fall back to existing brownfield lookup surfaces.
11. For OMX-driven completion flows, use `post-execution-refresh` with the actual changed-file list when you want a Ralph-first freshness update without enabling generic watch mode.

## Examples
- `repo-graph query "how does project ledger routing connect to AGENTS guidance"`
- `repo-graph path "project-ledger" "AGENTS.md"`
- `repo-graph explain "repo-graph"`
- `repo-graph community 0`
- `repo-graph god-nodes`
- `repo-graph node "builder-dsi-status"`

## Notes
- The repo-local plugin wiring lives under `plugins/planner-repo-graph/`
- The optional MCP server bootstraps its Python dependency on first launch via `scripts/repo-graph-mcp.sh`
- Prefer `status` for inspection and `ensure` for explicit bootstrap when operators want deterministic setup
