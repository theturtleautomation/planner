#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-config-common.sh"

builder_require_jq
builder_resolve_config "$CONFIG_FILE"
builder_validate_resolved_config "$CONFIG_FILE"
builder_print_contract create

CREATE_PROFILE=""

if [[ "$BUILDER_CONFIG_PROFILE" == "server-integration" ]]; then
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
  --dev-server-command "$BUILDER_CONFIG_COMMAND" \
  --dev-server-url "$BUILDER_CONFIG_SERVER_URL" \
  --need-setup false \
  --state-file "$REPO_ROOT/.codex/builder-fusion-project.json" \
  "${PROFILE_ARGS[@]}" \
  "$@"
