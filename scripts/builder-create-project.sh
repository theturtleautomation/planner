#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
STATE_FILE="$REPO_ROOT/.codex/builder-fusion-project.json"
HISTORY_FILE="$REPO_ROOT/.codex/builder-fusion-project-history.jsonl"

source "$SCRIPT_DIR/builder-config-common.sh"
source "$SCRIPT_DIR/builder-fusion-common.sh"

builder_require_jq
builder_resolve_config "$CONFIG_FILE"
builder_validate_resolved_config "$CONFIG_FILE"
builder_print_contract create

builder_slugify() {
  tr '[:upper:]' '[:lower:]' \
    | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//; s/-{2,}/-/g'
}

builder_generated_project_name() {
  local repo_name workflow timestamp
  repo_name="$(basename "$REPO_ROOT" | builder_slugify)"
  workflow="$(printf '%s' "$BUILDER_CONFIG_PROFILE" | builder_slugify)"
  timestamp="$(date -u +%Y%m%d-%H%M%S)"
  printf '%s-%s-%s\n' "$repo_name" "$workflow" "$timestamp"
}

builder_append_project_history() {
  local project_json="$1"
  local latest_state_file="$2"
  local history_file="$3"

  mkdir -p "$(dirname "$latest_state_file")"
  mkdir -p "$(dirname "$history_file")"

  jq -cn \
    --arg savedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg stateFile "$latest_state_file" \
    --arg configFile "$BUILDER_CONFIG_FILE" \
    --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
    --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
    --arg expectedRuntimeProfile "$(builder_expected_runtime_profile)" \
    --arg command "$BUILDER_CONFIG_COMMAND" \
    --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
    --argjson project "$project_json" '
    {
      historyVersion: 1,
      mode: "fire-and-forget",
      savedAt: $savedAt,
      stateFile: $stateFile,
      config: {
        file: $configFile,
        workflow: $workflow,
        plannerProfile: $plannerProfile,
        expectedRuntimeProfile: $expectedRuntimeProfile,
        command: $command,
        serverUrl: $serverUrl
      },
      project: $project
    }' >>"$history_file"
}

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

dryrun=false
custom_name_supplied=false
custom_branch_supplied=false
for arg in "$@"; do
  case "$arg" in
    --name)
      custom_name_supplied=true
      ;;
    --branch-name)
      custom_branch_supplied=true
      ;;
    --dryrun)
      dryrun=true
      ;;
  esac
done

CREATE_NAME="$(builder_generated_project_name)"
CREATE_BRANCH="$CREATE_NAME"

NAME_ARGS=()
BRANCH_ARGS=()
if [[ "$custom_name_supplied" != true ]]; then
  NAME_ARGS+=(--name "$CREATE_NAME")
fi
if [[ "$custom_branch_supplied" != true ]]; then
  BRANCH_ARGS+=(--branch-name "$CREATE_BRANCH")
fi

result="$(
  /home/thetu/.codex/skills/builder-workflow/scripts/create-fusion-project.sh \
  --cwd "$REPO_ROOT" \
  --dev-server-command "$BUILDER_CONFIG_COMMAND" \
  --dev-server-url "$BUILDER_CONFIG_SERVER_URL" \
  --need-setup false \
  --state-file "$STATE_FILE" \
  --force-create \
  "${NAME_ARGS[@]}" \
  "${BRANCH_ARGS[@]}" \
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
builder_append_project_history "$enriched_result" "$STATE_FILE" "$HISTORY_FILE"
printf '%s\n' "$enriched_result"
