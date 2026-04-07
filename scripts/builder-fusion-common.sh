#!/usr/bin/env bash

BUILDER_WORKFLOW_SCRIPT_DIR="/home/thetu/.codex/skills/builder-workflow/scripts"
source "$BUILDER_WORKFLOW_SCRIPT_DIR/fusion-project-common.sh"

builder_repo_read_cli_config_json() {
  if [[ -f "$HOME/.builder/config/data.json" ]]; then
    jq -c . "$HOME/.builder/config/data.json"
  else
    printf 'null\n'
  fi
}

builder_repo_read_dev_tools_json() {
  if [[ -f "$HOME/.builder/config/dev-tools.json" ]]; then
    jq -c . "$HOME/.builder/config/dev-tools.json"
  else
    printf 'null\n'
  fi
}

builder_repo_detect_current_space_id() {
  if [[ -n "${BUILDER_PUBLIC_API_KEY:-}" ]]; then
    printf '%s\n' "$BUILDER_PUBLIC_API_KEY"
    return
  fi

  local cli_config_json
  cli_config_json="$(builder_repo_read_cli_config_json)"
  jq -r '.credentials.builderPublicKey // ""' <<<"$cli_config_json"
}

builder_repo_detect_current_space_name() {
  local cli_config_json
  cli_config_json="$(builder_repo_read_cli_config_json)"
  jq -r '.credentials.spaceName // ""' <<<"$cli_config_json"
}

builder_repo_detect_current_user_id() {
  if [[ -n "${BUILDER_USER_ID:-}" ]]; then
    printf '%s\n' "$BUILDER_USER_ID"
    return
  fi

  local cli_config_json
  cli_config_json="$(builder_repo_read_cli_config_json)"
  local cli_user
  cli_user="$(jq -r '.credentials.userId // ""' <<<"$cli_config_json")"
  if [[ -n "$cli_user" ]]; then
    printf '%s\n' "$cli_user"
    return
  fi

  builder_detect_user_id
}

builder_repo_fetch_org_tree_json() {
  local space_id="$1"

  curl -fsS \
    -H "Authorization: Bearer $BUILDER_PRIVATE_API_KEY" \
    "https://api.builder.io/projects/org-tree?apiKey=${space_id}"
}

builder_repo_fetch_space_projects_json() {
  local space_id="$1"

  curl -fsS \
    -H "Authorization: Bearer $BUILDER_PRIVATE_API_KEY" \
    "https://api.builder.io/projects?apiKey=${space_id}"
}

builder_repo_fetch_user_projects_json() {
  local space_id="$1"
  local user_id="$2"

  if [[ -z "$user_id" ]]; then
    printf '{"projects":[]}\n'
    return
  fi

  curl -fsS \
    -H "Authorization: Bearer $BUILDER_PRIVATE_API_KEY" \
    "https://api.builder.io/projects?apiKey=${space_id}&userId=${user_id}"
}

builder_repo_fetch_direct_project_json() {
  local space_id="$1"
  local project_id="$2"
  local user_id="${3:-}"

  if [[ -z "$project_id" ]]; then
    printf 'null\n'
    return
  fi

  local url="https://api.builder.io/projects/${project_id}?apiKey=${space_id}"
  if [[ -n "$user_id" ]]; then
    url="${url}&userId=${user_id}"
  fi

  local tmp_body
  tmp_body="$(mktemp)"
  local http_code
  http_code="$(
    curl -sS \
      -o "$tmp_body" \
      -w "%{http_code}" \
      -H "Authorization: Bearer $BUILDER_PRIVATE_API_KEY" \
      "$url"
  )"
  local raw_body
  raw_body="$(cat "$tmp_body")"
  rm -f "$tmp_body"

  jq -cn \
    --arg url "$url" \
    --arg httpCode "$http_code" \
    --arg rawBody "$raw_body" '
    {
      url: $url,
      httpCode: ($httpCode | tonumber),
      response: (
        try ($rawBody | fromjson)
        catch {rawBody: $rawBody}
      )
    }'
}

builder_repo_fetch_project_branches_json() {
  local space_id="$1"
  local project_id="$2"
  local user_id="${3:-}"

  if [[ -z "$project_id" ]]; then
    printf 'null\n'
    return
  fi

  local url="https://api.builder.io/projects/branches?projectId=${project_id}&apiKey=${space_id}"
  if [[ -n "$user_id" ]]; then
    url="${url}&userId=${user_id}"
  fi

  local tmp_body
  tmp_body="$(mktemp)"
  local http_code
  http_code="$(
    curl -sS \
      -o "$tmp_body" \
      -w "%{http_code}" \
      -H "Authorization: Bearer $BUILDER_PRIVATE_API_KEY" \
      "$url"
  )"
  local raw_body
  raw_body="$(cat "$tmp_body")"
  rm -f "$tmp_body"

  jq -cn \
    --arg url "$url" \
    --arg httpCode "$http_code" \
    --arg rawBody "$raw_body" '
    {
      url: $url,
      httpCode: ($httpCode | tonumber),
      response: (
        try ($rawBody | fromjson)
        catch {rawBody: $rawBody}
      )
    }'
}

builder_repo_merge_project_surfaces_json() {
  local org_tree_json="$1"
  local space_projects_json="$2"
  local user_projects_json="$3"

  jq -cn \
    --argjson orgTree "$org_tree_json" \
    --argjson spaceList "$space_projects_json" \
    --argjson userList "$user_projects_json" '
    [
      (($orgTree.projects // [])[]),
      (($spaceList.projects // [])[]),
      (($userList.projects // [])[])
    ]
    | unique_by(.id)
  '
}

builder_repo_runtime_profile_for_project() {
  local project_json="$1"

  jq -r '
    ((.settings.environmentVariables // []) | map(select(.key == "PLANNER_LLM_MOCK")) | .[0].value) as $mock
    | if $mock == "full_pipeline" then "mock-full-pipeline"
      elif $mock == "phase26_live" then "mock-socratic"
      elif ($mock // "") == "" then "live"
      else ("custom:" + $mock)
      end
  ' <<<"$project_json"
}

builder_repo_enrich_saved_project_state_json() {
  local base_json="$1"
  local space_id="$2"
  local user_id="$3"
  local space_name="$4"
  local source="${5:-builder-wrapper}"
  local branch_name="${6:-}"

  jq -cn \
    --argjson base "$base_json" \
    --arg spaceId "$space_id" \
    --arg userId "$user_id" \
    --arg spaceName "$space_name" \
    --arg source "$source" \
    --arg branchName "$branch_name" '
    $base
    + {
        stateVersion: 2,
        savedVia: $source,
        savedAt: (now | todateiso8601),
        spaceId: ($spaceId | select(length > 0)),
        userId: ($userId | select(length > 0)),
        spaceName: ($spaceName | select(length > 0))
      }
    + (if $branchName != "" then {branchName: $branchName} else {} end)
  '
}
