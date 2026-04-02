#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-config-common.sh"

usage() {
  cat <<'EOF'
Usage: builder-verify-sync.sh [--json]

Verify whether the active local Builder config, saved Fusion project state, and
visible remote Fusion project settings are aligned.

Options:
  --json       Print only the machine-readable summary
  -h, --help   Show this help
EOF
}

json_only=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json_only=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

builder_require_jq
builder_resolve_config "$CONFIG_FILE"

validation_errors=""
expected_runtime_profile="$(builder_expected_runtime_profile)"
state_file="$REPO_ROOT/.codex/builder-fusion-project.json"
saved_state_json='null'
saved_project_id=""

if [[ -f "$state_file" ]]; then
  saved_state_json="$(jq -c . "$state_file")"
  saved_project_id="$(jq -r '.id // ""' "$state_file")"
fi

summary_json=""

if ! validation_errors="$(builder_collect_validation_errors "$CONFIG_FILE")"; then
  summary_json="$(jq -cn \
    --arg status "config_invalid" \
    --arg configFile "$CONFIG_FILE" \
    --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
    --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
    --arg command "$BUILDER_CONFIG_COMMAND" \
    --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
    --arg expectedRuntimeProfile "$expected_runtime_profile" \
    --arg stateFile "$state_file" \
    --argjson savedProject "$saved_state_json" \
    --rawfile validationErrors <(printf '%s\n' "$validation_errors") '
    {
      status: $status,
      config: {
        file: $configFile,
        workflow: $workflow,
        plannerProfile: $plannerProfile,
        command: $command,
        serverUrl: $serverUrl,
        expectedRuntimeProfile: $expectedRuntimeProfile
      },
      stateFile: $stateFile,
      savedProject: $savedProject,
      validationErrors: ($validationErrors | split("\n") | map(select(length > 0)))
    }')"
elif [[ -z "$saved_project_id" ]]; then
  summary_json="$(jq -cn \
    --arg status "missing_saved_project" \
    --arg configFile "$CONFIG_FILE" \
    --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
    --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
    --arg command "$BUILDER_CONFIG_COMMAND" \
    --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
    --arg expectedRuntimeProfile "$expected_runtime_profile" \
    --arg stateFile "$state_file" '
    {
      status: $status,
      config: {
        file: $configFile,
        workflow: $workflow,
        plannerProfile: $plannerProfile,
        command: $command,
        serverUrl: $serverUrl,
        expectedRuntimeProfile: $expectedRuntimeProfile
      },
      stateFile: $stateFile,
      savedProject: null,
      warnings: [
        "No saved Fusion project identity was found in .codex/builder-fusion-project.json."
      ]
    }')"
elif [[ -z "${BUILDER_PRIVATE_API_KEY:-}" || -z "${BUILDER_PUBLIC_API_KEY:-}" ]]; then
  summary_json="$(jq -cn \
    --arg status "visibility_blocked" \
    --arg configFile "$CONFIG_FILE" \
    --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
    --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
    --arg command "$BUILDER_CONFIG_COMMAND" \
    --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
    --arg expectedRuntimeProfile "$expected_runtime_profile" \
    --arg stateFile "$state_file" \
    --argjson savedProject "$saved_state_json" '
    {
      status: $status,
      config: {
        file: $configFile,
        workflow: $workflow,
        plannerProfile: $plannerProfile,
        command: $command,
        serverUrl: $serverUrl,
        expectedRuntimeProfile: $expectedRuntimeProfile
      },
      stateFile: $stateFile,
      savedProject: $savedProject,
      diagnosis: "missing_builder_auth_environment",
      warnings: [
        "BUILDER_PRIVATE_API_KEY and BUILDER_PUBLIC_API_KEY are required for remote Fusion verification."
      ]
    }')"
else
  remote_project_output=""
  remote_project_status=0

  remote_project_output="$("$SCRIPT_DIR/builder-get-project.sh" 2>/dev/null)" || {
    remote_project_status=$?
  }

  if [[ $remote_project_status -eq 0 ]]; then
    remote_status="$(jq -r '.status // "ok"' <<<"$remote_project_output")"
    remote_command="$(jq -r '.remoteProject.settings.devServerCommand // ""' <<<"$remote_project_output")"
    remote_server_url="$(jq -r '.remoteProject.settings.devServerUrl // ""' <<<"$remote_project_output")"
    remote_runtime_profile="$(jq -r '.remoteProject.runtimeProfile // ""' <<<"$remote_project_output")"
    mismatch_file="$(mktemp)"
    trap 'rm -f "$mismatch_file"' EXIT

    if [[ "$remote_status" == "ok" ]]; then
      if [[ "$remote_command" != "$BUILDER_CONFIG_COMMAND" ]]; then
        jq -cn \
          --arg field "devServerCommand" \
          --arg expected "$BUILDER_CONFIG_COMMAND" \
          --arg actual "$remote_command" \
          '{field: $field, expected: $expected, actual: $actual}' >>"$mismatch_file"
      fi

      if [[ "$remote_server_url" != "$BUILDER_CONFIG_SERVER_URL" ]]; then
        jq -cn \
          --arg field "devServerUrl" \
          --arg expected "$BUILDER_CONFIG_SERVER_URL" \
          --arg actual "$remote_server_url" \
          '{field: $field, expected: $expected, actual: $actual}' >>"$mismatch_file"
      fi

      if [[ "$remote_runtime_profile" != "$expected_runtime_profile" ]]; then
        jq -cn \
          --arg field "runtimeProfile" \
          --arg expected "$expected_runtime_profile" \
          --arg actual "$remote_runtime_profile" \
          '{field: $field, expected: $expected, actual: $actual}' >>"$mismatch_file"
      fi
    fi

    mismatch_json="$(jq -s '.' "$mismatch_file")"
    rm -f "$mismatch_file"
    trap - EXIT

    overall_status="in_sync"
    visibility_state="visible"
    if [[ "$remote_status" == "partial" ]]; then
      overall_status="visibility_partial"
      visibility_state="branch_visible_only"
    elif [[ "$(jq 'length' <<<"$mismatch_json")" != "0" ]]; then
      overall_status="drifted"
    fi

    summary_json="$(jq -cn \
      --arg status "$overall_status" \
      --arg configFile "$CONFIG_FILE" \
      --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
      --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
      --arg command "$BUILDER_CONFIG_COMMAND" \
      --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
      --arg expectedRuntimeProfile "$expected_runtime_profile" \
      --arg stateFile "$state_file" \
      --argjson savedProject "$saved_state_json" \
      --arg visibilityState "$visibility_state" \
      --argjson remoteProject "$(jq -c '.remoteProject' <<<"$remote_project_output")" \
      --argjson diagnosis "$(jq -c '.diagnosis // null' <<<"$remote_project_output")" \
      --argjson mismatches "$mismatch_json" '
      {
        status: $status,
        config: {
          file: $configFile,
          workflow: $workflow,
          plannerProfile: $plannerProfile,
          command: $command,
          serverUrl: $serverUrl,
          expectedRuntimeProfile: $expectedRuntimeProfile
        },
        stateFile: $stateFile,
        savedProject: $savedProject,
        visibility: {
          state: $visibilityState,
          classification: (
            if $visibilityState == "branch_visible_only" then
              ($diagnosis.classification // "branch_visible_only")
            else null
            end
          )
        },
        remoteProject: $remoteProject,
        diagnosis: $diagnosis,
        mismatches: $mismatches
      }')"
  else
    diagnosis_output="$("$SCRIPT_DIR/builder-diagnose-project-visibility.sh" 2>/dev/null || true)"
    diagnosis_code="$(jq -r '.classification // "undetermined"' <<<"$diagnosis_output" 2>/dev/null || printf 'undetermined\n')"
    overall_status="visibility_blocked"

    if [[ "$diagnosis_code" == "saved_project_stale" ]]; then
      overall_status="drifted"
    fi

    summary_json="$(jq -cn \
      --arg status "$overall_status" \
      --arg configFile "$CONFIG_FILE" \
      --arg workflow "$BUILDER_CONFIG_WORKFLOW_LABEL" \
      --arg plannerProfile "$BUILDER_CONFIG_PROFILE" \
      --arg command "$BUILDER_CONFIG_COMMAND" \
      --arg serverUrl "$BUILDER_CONFIG_SERVER_URL" \
      --arg expectedRuntimeProfile "$expected_runtime_profile" \
      --arg stateFile "$state_file" \
      --argjson savedProject "$saved_state_json" \
      --argjson diagnosis "$(printf '%s\n' "${diagnosis_output:-null}" | jq -c . 2>/dev/null || printf 'null\n')" '
      {
        status: $status,
        config: {
          file: $configFile,
          workflow: $workflow,
          plannerProfile: $plannerProfile,
          command: $command,
          serverUrl: $serverUrl,
          expectedRuntimeProfile: $expectedRuntimeProfile
        },
        stateFile: $stateFile,
        savedProject: $savedProject,
        visibility: {
          state: "blocked",
          classification: ($diagnosis.classification // null)
        },
        diagnosis: $diagnosis
      }')"
  fi
fi

if [[ "$json_only" == true ]]; then
  jq . <<<"$summary_json"
  exit 0
fi

printf 'Builder sync verification\n'
printf 'Status: %s\n' "$(jq -r '.status' <<<"$summary_json")"
printf 'Config: %s\n' "$(jq -r '.config.file' <<<"$summary_json")"
printf 'Workflow: %s\n' "$(jq -r '.config.workflow' <<<"$summary_json")"
printf 'Planner profile: %s\n' "$(jq -r '.config.plannerProfile' <<<"$summary_json")"
printf 'Server URL: %s\n' "$(jq -r '.config.serverUrl' <<<"$summary_json")"
printf 'Expected runtime profile: %s\n' "$(jq -r '.config.expectedRuntimeProfile' <<<"$summary_json")"

saved_project_line="$(jq -r '.savedProject | if . == null then "none" else (.id + " (" + (.name // "unnamed") + ")") end' <<<"$summary_json")"
printf 'Saved Fusion project: %s\n' "$saved_project_line"

case "$(jq -r '.status' <<<"$summary_json")" in
  config_invalid)
    printf 'Validation errors:\n'
    jq -r '.validationErrors[] | "  - " + .' <<<"$summary_json"
    ;;
  missing_saved_project)
    jq -r '.warnings[] | "Warning: " + .' <<<"$summary_json"
    ;;
  in_sync)
    printf 'Remote project: %s\n' "$(jq -r '.remoteProject.id + " (" + (.remoteProject.name // "unnamed") + ")"' <<<"$summary_json")"
    printf 'Remote server URL: %s\n' "$(jq -r '.remoteProject.settings.devServerUrl // "unset"' <<<"$summary_json")"
    printf 'Remote runtime profile: %s\n' "$(jq -r '.remoteProject.runtimeProfile // "unknown"' <<<"$summary_json")"
    if [[ "$(jq '.mismatches | length' <<<"$summary_json")" == "0" ]]; then
      printf 'Mismatches: none\n'
    else
      printf 'Mismatches:\n'
      jq -r '.mismatches[] | "  - " + .field + ": expected `" + .expected + "` but remote is `" + .actual + "`"' <<<"$summary_json"
    fi
    ;;
  drifted)
    if [[ "$(jq '.remoteProject != null' <<<"$summary_json")" == "true" ]]; then
      printf 'Remote project: %s\n' "$(jq -r '.remoteProject.id + " (" + (.remoteProject.name // "unnamed") + ")"' <<<"$summary_json")"
      printf 'Remote server URL: %s\n' "$(jq -r '.remoteProject.settings.devServerUrl // "unset"' <<<"$summary_json")"
      printf 'Remote runtime profile: %s\n' "$(jq -r '.remoteProject.runtimeProfile // "unknown"' <<<"$summary_json")"
      if [[ "$(jq '.mismatches | length' <<<"$summary_json")" == "0" ]]; then
        printf 'Mismatches: none\n'
      else
        printf 'Mismatches:\n'
        jq -r '.mismatches[] | "  - " + .field + ": expected `" + .expected + "` but remote is `" + .actual + "`"' <<<"$summary_json"
      fi
    else
      printf 'Visibility classification: %s\n' "$(jq -r '.visibility.classification // "unknown"' <<<"$summary_json")"
      printf 'Visibility evidence: %s\n' "$(jq -r '(.diagnosis.evidence // []) | join(", ")' <<<"$summary_json")"
    fi
    ;;
  visibility_blocked)
    printf 'Visibility classification: %s\n' "$(jq -r '.visibility.classification // "unknown"' <<<"$summary_json")"
    printf 'Visibility evidence: %s\n' "$(jq -r '(.diagnosis.evidence // []) | join(", ")' <<<"$summary_json")"
    ;;
esac

printf '\nJSON summary:\n'
jq . <<<"$summary_json"
