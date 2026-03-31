#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"

command -v jq >/dev/null 2>&1 || {
  echo "Missing required command: jq" >&2
  exit 1
}

DEV_SERVER_COMMAND="VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort"
DEV_SERVER_URL="http://127.0.0.1:3000"
CONFIG_PROFILE="frontend-mock-ui-review"

if [[ -f "$CONFIG_FILE" ]]; then
  config_command="$(jq -r '.command // ""' "$CONFIG_FILE")"
  config_server_url="$(jq -r '.serverUrl // ""' "$CONFIG_FILE")"
  config_profile="$(jq -r '.plannerBuilderProfile // ""' "$CONFIG_FILE")"
  if [[ -n "$config_command" ]]; then
    DEV_SERVER_COMMAND="$config_command"
  fi
  if [[ -n "$config_server_url" ]]; then
    DEV_SERVER_URL="$config_server_url"
  fi
  if [[ -n "$config_profile" ]]; then
    CONFIG_PROFILE="$config_profile"
  fi
fi

CREATE_PROFILE=""

if [[ "$CONFIG_PROFILE" == "server-integration" ]]; then
  PROFILE="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"

  case "$PROFILE" in
    disabled)
      ;;
    full_pipeline)
      CREATE_PROFILE="mock-full-pipeline"
      ;;
    phase26_live)
      CREATE_PROFILE="mock-socratic"
      ;;
    *)
      CREATE_PROFILE="$PROFILE"
      ;;
  esac
fi

PROFILE_ARGS=()
if [[ -n "$CREATE_PROFILE" ]]; then
  PROFILE_ARGS+=(--profile "$CREATE_PROFILE")
fi

exec /home/thetu/.codex/skills/builder-workflow/scripts/create-fusion-project.sh \
  --cwd "$REPO_ROOT" \
  --dev-server-command "$DEV_SERVER_COMMAND" \
  --dev-server-url "$DEV_SERVER_URL" \
  --need-setup false \
  --state-file "$REPO_ROOT/.codex/builder-fusion-project.json" \
  "${PROFILE_ARGS[@]}" \
  "$@"
