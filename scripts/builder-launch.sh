#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"

command -v jq >/dev/null 2>&1 || {
  echo "Missing required command: jq" >&2
  exit 1
}

BASE_COMMAND="VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort"
CONFIG_SERVER_URL="http://127.0.0.1:3000"
CONFIG_PROFILE="frontend-mock-ui-review"

if [[ -f "$CONFIG_FILE" ]]; then
  config_command="$(jq -r '.command // ""' "$CONFIG_FILE")"
  config_server_url="$(jq -r '.serverUrl // ""' "$CONFIG_FILE")"
  config_profile="$(jq -r '.plannerBuilderProfile // ""' "$CONFIG_FILE")"
  if [[ -n "$config_command" ]]; then
    BASE_COMMAND="$config_command"
  fi
  if [[ -n "$config_server_url" ]]; then
    CONFIG_SERVER_URL="$config_server_url"
  fi
  if [[ -n "$config_profile" ]]; then
    CONFIG_PROFILE="$config_profile"
  fi
fi

CONFIG_PORT="$(sed -E 's#^[^:]+://[^:/]+:([0-9]+).*$#\1#' <<<"$CONFIG_SERVER_URL")"
if [[ -z "$CONFIG_PORT" || "$CONFIG_PORT" == "$CONFIG_SERVER_URL" ]]; then
  CONFIG_PORT="4174"
fi

PORT="${BUILDER_PROJECT_RUNTIME_PORT:-$CONFIG_PORT}"
APP_COMMAND="$BASE_COMMAND"

if [[ "$CONFIG_PROFILE" == "server-integration" ]]; then
  MOCK_MODE="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"
  if [[ "$MOCK_MODE" != "disabled" ]]; then
    APP_COMMAND="PLANNER_LLM_MOCK=${MOCK_MODE} ${BASE_COMMAND}"
  fi
fi

exec /home/thetu/.codex/skills/builder-workflow/scripts/launch-fusion.sh \
  --port "$PORT" \
  --command "$APP_COMMAND" \
  --no-open \
  "$@"
