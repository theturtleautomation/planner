#!/usr/bin/env bash

set -euo pipefail

builder_require_jq() {
  command -v jq >/dev/null 2>&1 || {
    echo "Missing required command: jq" >&2
    exit 1
  }
}

builder_default_command() {
  printf '%s\n' \
    "VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort"
}

builder_default_server_url() {
  printf '%s\n' "http://127.0.0.1:3000"
}

builder_default_profile() {
  printf '%s\n' "frontend-mock-ui-review"
}

builder_expected_runtime_profile() {
  local mock_mode="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"

  if [[ "${BUILDER_CONFIG_PROFILE:-}" != "server-integration" ]]; then
    printf '%s\n' "live"
    return
  fi

  case "$mock_mode" in
    disabled)
      printf '%s\n' "live"
      ;;
    full_pipeline)
      printf '%s\n' "mock-full-pipeline"
      ;;
    phase26_live)
      printf '%s\n' "mock-socratic"
      ;;
    *)
      printf 'custom:%s\n' "$mock_mode"
      ;;
  esac
}

builder_resolve_config() {
  local config_file="$1"

  BUILDER_CONFIG_FILE="$config_file"
  BUILDER_CONFIG_COMMAND="$(builder_default_command)"
  BUILDER_CONFIG_SERVER_URL="$(builder_default_server_url)"
  BUILDER_CONFIG_PROFILE="$(builder_default_profile)"

  if [[ -f "$config_file" ]]; then
    local config_command config_server_url config_profile
    config_command="$(jq -r '.command // ""' "$config_file")"
    config_server_url="$(jq -r '.serverUrl // ""' "$config_file")"
    config_profile="$(jq -r '.plannerBuilderProfile // ""' "$config_file")"

    if [[ -n "$config_command" ]]; then
      BUILDER_CONFIG_COMMAND="$config_command"
    fi
    if [[ -n "$config_server_url" ]]; then
      BUILDER_CONFIG_SERVER_URL="$config_server_url"
    fi
    if [[ -n "$config_profile" ]]; then
      BUILDER_CONFIG_PROFILE="$config_profile"
    fi
  fi

  BUILDER_CONFIG_PORT="$(sed -E 's#^[^:]+://[^:/]+:([0-9]+).*$#\1#' <<<"$BUILDER_CONFIG_SERVER_URL")"
  if [[ -z "$BUILDER_CONFIG_PORT" || "$BUILDER_CONFIG_PORT" == "$BUILDER_CONFIG_SERVER_URL" ]]; then
    BUILDER_CONFIG_PORT=""
  fi

  case "$BUILDER_CONFIG_PROFILE" in
    frontend-mock-ui-review)
      BUILDER_CONFIG_WORKFLOW_LABEL="default frontend-mock UI-review"
      ;;
    server-integration)
      BUILDER_CONFIG_WORKFLOW_LABEL="alternate server-backed integration"
      ;;
    *)
      BUILDER_CONFIG_WORKFLOW_LABEL="unknown"
      ;;
  esac
}

builder_collect_validation_errors() {
  local config_file="$1"
  local errors=()

  if [[ ! -f "$config_file" ]]; then
    errors+=("Config file not found: $config_file")
  fi

  if [[ -z "$BUILDER_CONFIG_COMMAND" ]]; then
    errors+=("Missing required field '.command'")
  fi

  if [[ -z "$BUILDER_CONFIG_SERVER_URL" ]]; then
    errors+=("Missing required field '.serverUrl'")
  elif [[ ! "$BUILDER_CONFIG_SERVER_URL" =~ ^https?:// ]]; then
    errors+=("Invalid '.serverUrl': must start with http:// or https://")
  fi

  if [[ -z "$BUILDER_CONFIG_PORT" ]]; then
    errors+=("Invalid '.serverUrl': expected an explicit port")
  elif [[ ! "$BUILDER_CONFIG_PORT" =~ ^[0-9]+$ ]]; then
    errors+=("Invalid '.serverUrl': port must be numeric")
  fi

  case "$BUILDER_CONFIG_PROFILE" in
    frontend-mock-ui-review)
      if [[ "$BUILDER_CONFIG_PORT" != "3000" ]]; then
        errors+=("frontend-mock-ui-review must use port 3000")
      fi
      if [[ "$BUILDER_CONFIG_COMMAND" != *"VITE_PLANNER_FRONTEND_MOCK=1"* ]]; then
        errors+=("frontend-mock-ui-review command must enable VITE_PLANNER_FRONTEND_MOCK=1")
      fi
      if [[ "$BUILDER_CONFIG_COMMAND" != *"--port 3000"* ]]; then
        errors+=("frontend-mock-ui-review command must target port 3000")
      fi
      ;;
    server-integration)
      if [[ "$BUILDER_CONFIG_PORT" != "4174" ]]; then
        errors+=("server-integration must use port 4174")
      fi
      if [[ "$BUILDER_CONFIG_COMMAND" != *"cargo run -p planner-server"* ]]; then
        errors+=("server-integration command must launch planner-server")
      fi
      if [[ "$BUILDER_CONFIG_COMMAND" != *"--static-dir ./planner-solid/dist/static"* ]]; then
        errors+=("server-integration command must pass --static-dir ./planner-solid/dist/static")
      fi
      ;;
    *)
      errors+=("Invalid '.plannerBuilderProfile': expected 'frontend-mock-ui-review' or 'server-integration'")
      ;;
  esac

  if ((${#errors[@]} > 0)); then
    local error
    for error in "${errors[@]}"; do
      printf '%s\n' "$error"
    done
    return 1
  fi

  return 0
}

builder_validate_resolved_config() {
  local config_file="$1"
  local validation_errors=""

  if ! validation_errors="$(builder_collect_validation_errors "$config_file")"; then
    printf 'Builder config validation failed for %s\n' "$config_file" >&2
    while IFS= read -r error; do
      [[ -n "$error" ]] || continue
      printf '  - %s\n' "$error" >&2
    done <<<"$validation_errors"
    exit 1
  fi
}

builder_remote_profile_for_action() {
  local action="$1"
  local mock_mode="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"

  if [[ "$BUILDER_CONFIG_PROFILE" != "server-integration" ]]; then
    if [[ "$action" == "update" ]]; then
      printf '%s\n' "none (clears environment variables)"
    else
      printf '%s\n' "default (no --profile override)"
    fi
    return
  fi

  case "$mock_mode" in
    disabled)
      if [[ "$action" == "create" ]]; then
        printf '%s\n' "default (no --profile override)"
      else
        printf '%s\n' "none (clears environment variables)"
      fi
      ;;
    full_pipeline)
      printf '%s\n' "mock-full-pipeline"
      ;;
    phase26_live)
      printf '%s\n' "mock-socratic"
      ;;
    *)
      printf '%s\n' "$mock_mode"
      ;;
  esac
}

builder_launch_command() {
  if [[ "$BUILDER_CONFIG_PROFILE" == "server-integration" ]]; then
    local mock_mode="${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"
    if [[ "$mock_mode" != "disabled" ]]; then
      printf 'PLANNER_LLM_MOCK=%s %s\n' "$mock_mode" "$BUILDER_CONFIG_COMMAND"
      return
    fi
  fi

  printf '%s\n' "$BUILDER_CONFIG_COMMAND"
}

builder_print_contract() {
  local action="$1"
  local remote_mutation_label remote_profile

  case "$action" in
    launch)
      remote_mutation_label="local-only launch"
      remote_profile="n/a"
      ;;
    create)
      remote_mutation_label="remote-persisted project create"
      remote_profile="$(builder_remote_profile_for_action create)"
      ;;
    update)
      remote_mutation_label="remote-persisted project update"
      remote_profile="$(builder_remote_profile_for_action update)"
      ;;
    inspect)
      remote_mutation_label="inspection only"
      remote_profile="n/a"
      ;;
    validate)
      remote_mutation_label="validation only"
      remote_profile="n/a"
      ;;
    *)
      remote_mutation_label="$action"
      remote_profile="n/a"
      ;;
  esac

  printf 'Builder config: %s\n' "$BUILDER_CONFIG_FILE"
  printf 'Workflow: %s\n' "$BUILDER_CONFIG_WORKFLOW_LABEL"
  printf 'Planner config profile: %s\n' "$BUILDER_CONFIG_PROFILE"
  printf 'Server URL: %s\n' "$BUILDER_CONFIG_SERVER_URL"
  printf 'Runtime port: %s\n' "${BUILDER_CONFIG_PORT:-unknown}"
  printf 'Command: %s\n' "$BUILDER_CONFIG_COMMAND"
  printf 'Action: %s\n' "$remote_mutation_label"
  if [[ "$BUILDER_CONFIG_PROFILE" == "server-integration" ]]; then
    printf 'Server mock mode: %s\n' "${PLANNER_BUILDER_LLM_MOCK_MODE:-full_pipeline}"
  else
    printf 'Server mock mode: n/a (frontend mock runtime)\n'
  fi
  printf 'Remote Builder profile: %s\n' "$remote_profile"
}
