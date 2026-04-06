#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
PLUGIN_DIR="$REPO_ROOT/plugins/planner-repo-graph"
PLUGIN_MANIFEST="$PLUGIN_DIR/.codex-plugin/plugin.json"
MCP_CONFIG="$PLUGIN_DIR/.mcp.json"
MARKETPLACE="$REPO_ROOT/.agents/plugins/marketplace.json"
GRAPH_SCRIPT="$REPO_ROOT/scripts/repo-graph.sh"
MCP_WRAPPER="$REPO_ROOT/scripts/repo-graph-mcp.sh"
PY_GRAPH="$REPO_ROOT/scripts/repo_graph.py"
PY_MCP="$REPO_ROOT/scripts/repo_graph_mcp.py"
PY_MCP_BOOTSTRAP="$REPO_ROOT/scripts/repo_graph_mcp_bootstrap.py"

command -v jq >/dev/null 2>&1 || { echo "Missing required command: jq" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "Missing required command: python3" >&2; exit 1; }
command -v bash >/dev/null 2>&1 || { echo "Missing required command: bash" >&2; exit 1; }

for file in "$PLUGIN_MANIFEST" "$MCP_CONFIG" "$MARKETPLACE" "$GRAPH_SCRIPT" "$MCP_WRAPPER" "$PY_GRAPH" "$PY_MCP" "$PY_MCP_BOOTSTRAP"; do
  [[ -f "$file" ]] || { echo "Missing repo-graph file: $file" >&2; exit 1; }
done

jq . "$PLUGIN_MANIFEST" >/dev/null
jq . "$MCP_CONFIG" >/dev/null
jq . "$MARKETPLACE" >/dev/null
bash -n "$GRAPH_SCRIPT"
bash -n "$MCP_WRAPPER"
python3 -m py_compile "$PY_GRAPH" "$PY_MCP" "$PY_MCP_BOOTSTRAP"

marketplace_path="$(
  jq -r '.plugins[] | select(.name == "planner-repo-graph") | .source.path // ""' "$MARKETPLACE"
)"

if [[ "$marketplace_path" != "./plugins/planner-repo-graph" ]]; then
  echo "planner-repo-graph is missing or misconfigured in .agents/plugins/marketplace.json" >&2
  exit 1
fi

status_output="$("$GRAPH_SCRIPT" status)"
mcp_status_json="$("$MCP_WRAPPER" status --json)"
mcp_state="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"$mcp_status_json")"

printf 'Repo graph plugin: %s\n' "$PLUGIN_DIR"
printf 'Plugin manifest: %s\n' "$PLUGIN_MANIFEST"
printf 'MCP config: %s\n' "$MCP_CONFIG"
printf 'Marketplace entry: %s\n' "$marketplace_path"
printf 'Graph status:\n%s\n' "$status_output"
printf 'MCP lifecycle state: %s\n' "$mcp_state"
printf 'MCP note: use `scripts/repo-graph-mcp.sh ensure` for explicit bootstrap or `scripts/repo-graph-mcp.sh status --json` for machine-readable state.\n'
