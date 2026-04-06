#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
GRAPH_DIR="${REPO_GRAPH_DIR:-$REPO_ROOT/.omx/graphs/repo-graph}"
PYTHON_BIN="${PYTHON_BIN:-python3}"

command -v "$PYTHON_BIN" >/dev/null 2>&1 || {
  echo "Missing required command: $PYTHON_BIN" >&2
  exit 1
}

if [[ $# -lt 1 ]]; then
  cat >&2 <<'USAGE'
Usage: scripts/repo-graph.sh <build|update|cluster-only|status|query|path|explain|node|neighbors|community|god-nodes|post-execution-refresh> [args...]
USAGE
  exit 1
fi

command="$1"
shift

case "$command" in
  build)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" build "$@"
    ;;
  update)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" update "$@"
    ;;
  cluster-only)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" cluster-only "$@"
    ;;
  status)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" status "$@"
    ;;
  query)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" query --ensure-fresh "$@"
    ;;
  path)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" path --ensure-fresh "$@"
    ;;
  explain)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" explain --ensure-fresh "$@"
    ;;
  node)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" node --ensure-fresh "$@"
    ;;
  neighbors)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" neighbors --ensure-fresh "$@"
    ;;
  community)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" community --ensure-fresh "$@"
    ;;
  god-nodes)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" god-nodes --ensure-fresh "$@"
    ;;
  post-execution-refresh)
    exec "$PYTHON_BIN" "$REPO_ROOT/scripts/repo_graph.py" --root "$REPO_ROOT" --graph-dir "$GRAPH_DIR" post-execution-refresh "$@"
    ;;
  *)
    echo "Unknown repo-graph command: $command" >&2
    exit 1
    ;;
esac
