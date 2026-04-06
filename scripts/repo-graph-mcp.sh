#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
TOOL_ROOT="$REPO_ROOT/.omx/tooling/repo-graph-mcp"
PYTHON_BIN="${PYTHON_BIN:-python3}"

command -v "$PYTHON_BIN" >/dev/null 2>&1 || {
  echo "Missing required command: $PYTHON_BIN" >&2
  exit 1
}

command="${1:-run}"
shift || true

exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph_mcp_bootstrap.py" \
  --repo-root "$REPO_ROOT" \
  --tool-root "$TOOL_ROOT" \
  --python-bin "$PYTHON_BIN" \
  "$command" "$@"
