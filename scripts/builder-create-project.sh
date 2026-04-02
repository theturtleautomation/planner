#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
STATE_FILE="$REPO_ROOT/.codex/builder-fusion-project.json"

source "$SCRIPT_DIR/builder-config-common.sh"
source "$SCRIPT_DIR/builder-fusion-common.sh"

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

force_create=false
dryrun=false
for arg in "$@"; do
  case "$arg" in
    --force-create)
      force_create=true
      ;;
    --dryrun)
      dryrun=true
      ;;
  esac
done

if [[ -f "$STATE_FILE" && "$force_create" != true && "$dryrun" != true ]]; then
  cat "$STATE_FILE"
  exit 0
fi

result="$(
  /home/thetu/.codex/skills/builder-workflow/scripts/create-fusion-project.sh \
  --cwd "$REPO_ROOT" \
  --dev-server-command "$BUILDER_CONFIG_COMMAND" \
  --dev-server-url "$BUILDER_CONFIG_SERVER_URL" \
  --need-setup false \
  --state-file "$STATE_FILE" \
  "${PROFILE_ARGS[@]}" \
  "$@"
)"

if ! jq -e . >/dev/null 2>&1 <<<"$result"; then
  printf '%s\n' "$result"
  exit 0
fi

if [[ "$dryrun" == true ]]; then
  printf '%s\n' "$result"
  exit 0
fi

current_space_id="$(builder_repo_detect_current_space_id)"
current_user_id="$(builder_repo_detect_current_user_id)"
current_space_name="$(builder_repo_detect_current_space_name)"
branch_name="$(jq -r '.branchName // (.url // "" | split("/") | last) // ""' <<<"$result")"
enriched_result="$(builder_repo_enrich_saved_project_state_json "$result" "$current_space_id" "$current_user_id" "$current_space_name" "builder-create-project" "$branch_name")"

mkdir -p "$(dirname "$STATE_FILE")"
printf '%s\n' "$enriched_result" >"$STATE_FILE"
printf '%s\n' "$enriched_result"
