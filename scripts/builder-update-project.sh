#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-config-common.sh"

builder_require_jq
builder_resolve_config "$CONFIG_FILE"
builder_validate_resolved_config "$CONFIG_FILE"
builder_print_contract update

PROFILE_ARGS=()
CLEAR_ENV_ARGS=()
if [[ "$BUILDER_CONFIG_PROFILE" == "server-integration" ]]; then
  PROFILE="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"
  case "$PROFILE" in
    disabled)
      CLEAR_ENV_ARGS+=(--clear-env)
      ;;
    full_pipeline)
      PROFILE_ARGS+=(--profile mock-full-pipeline)
      ;;
    phase26_live)
      PROFILE_ARGS+=(--profile mock-socratic)
      ;;
    *)
      PROFILE_ARGS+=(--profile "$PROFILE")
      ;;
  esac
else
  CLEAR_ENV_ARGS+=(--clear-env)
fi

exec /home/thetu/.codex/skills/builder-workflow/scripts/update-fusion-project.sh \
  --state-file "$REPO_ROOT/.codex/builder-fusion-project.json" \
  --dev-server-command "$BUILDER_CONFIG_COMMAND" \
  --dev-server-url "$BUILDER_CONFIG_SERVER_URL" \
  "${PROFILE_ARGS[@]}" \
  "${CLEAR_ENV_ARGS[@]}" \
  "$@"
