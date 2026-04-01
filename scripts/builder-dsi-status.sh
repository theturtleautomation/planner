#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
PLUGIN_DIR="$REPO_ROOT/plugins/planner-builder-dsi"
PLUGIN_MANIFEST="$PLUGIN_DIR/.codex-plugin/plugin.json"
MCP_CONFIG="$PLUGIN_DIR/.mcp.json"
MARKETPLACE="$REPO_ROOT/.agents/plugins/marketplace.json"

command -v jq >/dev/null 2>&1 || {
  echo "Missing required command: jq" >&2
  exit 1
}

command -v node >/dev/null 2>&1 || {
  echo "Missing required command: node" >&2
  exit 1
}

command -v npx >/dev/null 2>&1 || {
  echo "Missing required command: npx" >&2
  exit 1
}

[[ -f "$PLUGIN_MANIFEST" ]] || {
  echo "Missing DSI plugin manifest: $PLUGIN_MANIFEST" >&2
  exit 1
}

[[ -f "$MCP_CONFIG" ]] || {
  echo "Missing DSI MCP config: $MCP_CONFIG" >&2
  exit 1
}

[[ -f "$MARKETPLACE" ]] || {
  echo "Missing plugin marketplace file: $MARKETPLACE" >&2
  exit 1
}

jq . "$PLUGIN_MANIFEST" >/dev/null
jq . "$MCP_CONFIG" >/dev/null
jq . "$MARKETPLACE" >/dev/null

marketplace_path="$(
  jq -r '
    .plugins[]
    | select(.name == "planner-builder-dsi")
    | .source.path // ""
  ' "$MARKETPLACE"
)"

if [[ "$marketplace_path" != "./plugins/planner-builder-dsi" ]]; then
  echo "planner-builder-dsi is missing or misconfigured in .agents/plugins/marketplace.json" >&2
  exit 1
fi

node_version_raw="$(node --version)"
node_version="${node_version_raw#v}"
node_major="${node_version%%.*}"
if [[ -z "$node_major" || ! "$node_major" =~ ^[0-9]+$ ]]; then
  echo "Unable to parse Node.js version: $node_version_raw" >&2
  exit 1
fi

if (( node_major < 20 )); then
  echo "Builder DSI requires Node.js v20+; found $node_version_raw" >&2
  exit 1
fi

probe_status="skipped"
probe_message="Command probe skipped because timeout is not available."
if command -v timeout >/dev/null 2>&1; then
  if timeout 30s npx -y @builder.io/dev-tools@latest dsi-mcp --help >/tmp/planner-builder-dsi-help.$$ 2>&1; then
    probe_status="ok"
    probe_message="npx @builder.io/dev-tools@latest dsi-mcp --help completed successfully."
  else
    probe_status="failed"
    probe_message="$(cat /tmp/planner-builder-dsi-help.$$)"
  fi
  rm -f /tmp/planner-builder-dsi-help.$$
fi

if [[ "$probe_status" == "failed" ]]; then
  echo "Builder DSI command probe failed." >&2
  echo "$probe_message" >&2
  exit 1
fi

printf 'Builder DSI plugin: %s\n' "$PLUGIN_DIR"
printf 'Plugin manifest: %s\n' "$PLUGIN_MANIFEST"
printf 'MCP config: %s\n' "$MCP_CONFIG"
printf 'Marketplace entry: %s\n' "$marketplace_path"
printf 'Node.js: %s\n' "$node_version_raw"
printf 'Command probe: %s\n' "$probe_status"
printf 'Probe detail: %s\n' "$probe_message"
printf 'Codex session note: restart existing sessions if the repo-local DSI plugin was just added.\n'

