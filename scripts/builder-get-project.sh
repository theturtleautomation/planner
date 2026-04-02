#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-fusion-common.sh"

usage() {
  cat <<'EOF'
Usage: builder-get-project.sh [options]

Resolve and inspect the canonical saved Fusion project or an explicit project
ID using the known repo read surfaces.

Options:
  --space-id ID         Builder space ID / public API key
  --project-id ID       Explicit project ID override
  --state-file PATH     Optional saved project state file
  -h, --help            Show this help
EOF
}

builder_require_cmd jq
builder_require_cmd curl

space_id="${BUILDER_PUBLIC_API_KEY:-}"
project_id=""
state_file="$REPO_ROOT/.codex/builder-fusion-project.json"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --space-id)
      space_id="$2"
      shift 2
      ;;
    --project-id)
      project_id="$2"
      shift 2
      ;;
    --state-file)
      state_file="$2"
      shift 2
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

if [[ -z "${BUILDER_PRIVATE_API_KEY:-}" ]]; then
  echo "BUILDER_PRIVATE_API_KEY is required" >&2
  exit 1
fi

if [[ -z "$space_id" ]]; then
  echo "--space-id or BUILDER_PUBLIC_API_KEY is required" >&2
  exit 1
fi

saved_state_json="$(builder_read_state_json "$state_file")"
saved_project_id="$(jq -r '.id // ""' <<<"$saved_state_json")"
saved_name="$(jq -r '.name // ""' <<<"$saved_state_json")"
saved_repo_full_name="$(jq -r '.repoFullName // ""' <<<"$saved_state_json")"
effective_project_id="$project_id"
if [[ -z "$effective_project_id" ]]; then
  effective_project_id="$saved_project_id"
fi

current_user_id="$(builder_repo_detect_current_user_id)"
org_tree_json="$(builder_repo_fetch_org_tree_json "$space_id" 2>/dev/null || printf '{"projects":[],"branches":[],"users":[]}\n')"
user_projects_json="$(builder_repo_fetch_user_projects_json "$space_id" "$current_user_id" 2>/dev/null || printf '{"projects":[]}\n')"
merged_projects_json="$(builder_repo_merge_project_surfaces_json "$org_tree_json" "$user_projects_json")"
branch_surface_json="$(builder_repo_fetch_project_branches_json "$space_id" "$effective_project_id")"
branch_surface_with_user_json="$(builder_repo_fetch_project_branches_json "$space_id" "$effective_project_id" "$current_user_id")"

remote_project_json="$(
  jq -c \
    --arg projectId "$effective_project_id" \
    --arg savedName "$saved_name" \
    --arg savedRepoFullName "$saved_repo_full_name" '
      first(
        if $projectId != "" then
          .[] | select(.id == $projectId)
        else
          .[]
          | select(
              ($savedRepoFullName != "" and .repoFullName == $savedRepoFullName)
              or
              ($savedName != "" and .name == $savedName)
            )
        end
      ) // empty
    ' <<<"$merged_projects_json"
)"

if [[ -z "$remote_project_json" ]]; then
  diagnosis_output="$("$SCRIPT_DIR/builder-diagnose-project-visibility.sh" \
    --state-file "$state_file" \
    ${effective_project_id:+--project-id "$effective_project_id"} \
    ${space_id:+--space-id "$space_id"} 2>/dev/null || true)"

  branch_visible="$(
    jq -r '
      (((.response.branches // []) | length) > 0)
    ' <<<"$branch_surface_json" 2>/dev/null || printf 'false\n'
  )"
  branch_visible_with_user="$(
    jq -r '
      (((.response.branches // []) | length) > 0)
    ' <<<"$branch_surface_with_user_json" 2>/dev/null || printf 'false\n'
  )"

  if [[ "$branch_visible" == "true" || "$branch_visible_with_user" == "true" ]]; then
    jq -cn \
      --arg projectId "$effective_project_id" \
      --arg stateFile "$state_file" \
      --argjson savedProject "$saved_state_json" \
      --argjson diagnosis "$(printf '%s\n' "${diagnosis_output:-null}" | jq -c . 2>/dev/null || printf 'null\n')" \
      --argjson branchSurface "$branch_surface_json" \
      --argjson branchSurfaceWithUser "$branch_surface_with_user_json" '
      {
        status: "partial",
        mode: "repo-wrapper",
        projectId: ($projectId | select(length > 0)),
        stateFile: ($stateFile | select(length > 0)),
        savedProject: $savedProject,
        visibility: {
          classification: ($diagnosis.classification // "branch_visible_only")
        },
        remoteProject: {
          id: ($projectId | select(length > 0)),
          name: ($savedProject.name // null),
          repoFullName: ($savedProject.repoFullName // null),
          visibleVia: {
            orgTree: false,
            userProjectList: false,
            branchSurface: (((($branchSurface.response.branches // []) | length) > 0) or ((($branchSurfaceWithUser.response.branches // []) | length) > 0))
          },
          branchSurface: $branchSurface,
          branchSurfaceWithUser: $branchSurfaceWithUser,
          metadataUnavailable: true
        },
        diagnosis: $diagnosis,
        warnings: [
          "The target Fusion project is visible on the Builder branch surface but not on the current metadata read surfaces."
        ]
      }'
    exit 0
  fi

  jq -cn \
    --arg projectId "$effective_project_id" \
    --arg stateFile "$state_file" \
    --argjson savedProject "$saved_state_json" \
    --argjson diagnosis "$(printf '%s\n' "${diagnosis_output:-null}" | jq -c . 2>/dev/null || printf 'null\n')" '
    {
      status: "not_found",
      mode: "repo-wrapper",
      projectId: ($projectId | select(length > 0)),
      stateFile: ($stateFile | select(length > 0)),
      savedProject: $savedProject,
      diagnosis: $diagnosis,
      warnings: [
        "The target Fusion project was not visible on the current Builder project read surfaces."
      ]
    }'
  exit 1
fi

visible_via_json="$(
  jq -cn \
    --argjson remote "$remote_project_json" \
    --argjson orgTree "$org_tree_json" \
    --argjson userList "$user_projects_json" '
    {
      orgTree: ([($orgTree.projects // [])[] | select(.id == $remote.id)] | length) > 0,
      userProjectList: ([($userList.projects // [])[] | select(.id == $remote.id)] | length) > 0
    }'
)"

jq -cn \
  --arg projectId "$effective_project_id" \
  --arg stateFile "$state_file" \
  --argjson savedProject "$saved_state_json" \
  --argjson remote "$remote_project_json" \
  --argjson visibleVia "$visible_via_json" '
  def runtime_profile($settings):
    (($settings.environmentVariables // []) | map(select(.key == "PLANNER_LLM_MOCK")) | .[0].value) as $mock
    | if $mock == "full_pipeline" then "mock-full-pipeline"
      elif $mock == "phase26_live" then "mock-socratic"
      elif ($mock // "") == "" then "live"
      else ("custom:" + $mock)
      end;
  {
    status: "ok",
    mode: "repo-wrapper",
    projectId: ($projectId | select(length > 0)),
    stateFile: ($stateFile | select(length > 0)),
    savedProject: $savedProject,
    remoteProject: {
      id: $remote.id,
      name: $remote.name,
      repoFullName: $remote.repoFullName,
      visibleVia: $visibleVia,
      metadataUnavailable: false,
      branchData: ($remote.branchData // null),
      runtimeProfile: runtime_profile($remote.settings // {}),
      settings: {
        installCommand: ($remote.settings.installCommand // null),
        devServerCommand: ($remote.settings.devServerCommand // null),
        devServerUrl: ($remote.settings.devServerUrl // null),
        mainBranchName: ($remote.settings.mainBranchName // null),
        recommendedRoot: ($remote.settings.recommendedRoot // null),
        environmentVariables: ($remote.settings.environmentVariables // [])
      }
    }
  }'
