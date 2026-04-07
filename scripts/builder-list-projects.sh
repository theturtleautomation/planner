#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-fusion-common.sh"

usage() {
  cat <<'EOF'
Usage: builder-list-projects.sh [options]

List Fusion projects visible to the current Builder auth context across the
known repo read surfaces.

Options:
  --space-id ID         Builder space ID / public API key
  --project-id ID       Optional project ID filter
  --name NAME           Optional project name filter
  --repo-full-name STR  Optional repo full name filter
  --state-file PATH     Optional saved project state file
  -h, --help            Show this help
EOF
}

builder_require_cmd jq
builder_require_cmd curl

space_id="${BUILDER_PUBLIC_API_KEY:-}"
project_id=""
project_name=""
repo_full_name=""
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
    --name)
      project_name="$2"
      shift 2
      ;;
    --repo-full-name)
      repo_full_name="$2"
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
current_user_id="$(builder_repo_detect_current_user_id)"
org_tree_json="$(builder_repo_fetch_org_tree_json "$space_id" 2>/dev/null || printf '{"projects":[],"branches":[],"users":[]}\n')"
space_projects_json="$(builder_repo_fetch_space_projects_json "$space_id" 2>/dev/null || printf '{"projects":[]}\n')"
user_projects_json="$(builder_repo_fetch_user_projects_json "$space_id" "$current_user_id" 2>/dev/null || printf '{"projects":[]}\n')"
merged_projects_json="$(builder_repo_merge_project_surfaces_json "$org_tree_json" "$space_projects_json" "$user_projects_json")"
branch_surface_json="$(builder_repo_fetch_project_branches_json "$space_id" "$project_id" "$current_user_id")"

jq -cn \
  --arg stateFile "$state_file" \
  --arg savedProjectId "$saved_project_id" \
  --arg spaceId "$space_id" \
  --arg userId "$current_user_id" \
  --arg projectId "$project_id" \
  --arg projectName "$project_name" \
  --arg repoFullName "$repo_full_name" \
  --argjson orgTree "$org_tree_json" \
  --argjson spaceList "$space_projects_json" \
  --argjson userList "$user_projects_json" \
  --argjson mergedProjects "$merged_projects_json" \
  --argjson branchSurface "$branch_surface_json" '
  def runtime_profile($settings):
    (($settings.environmentVariables // []) | map(select(.key == "PLANNER_LLM_MOCK")) | .[0].value) as $mock
    | if $mock == "full_pipeline" then "mock-full-pipeline"
      elif $mock == "phase26_live" then "mock-socratic"
      elif ($mock // "") == "" then "live"
      else ("custom:" + $mock)
      end;
  def project_matches($project):
    ($projectId == "" or $project.id == $projectId)
    and ($projectName == "" or $project.name == $projectName)
    and ($repoFullName == "" or $project.repoFullName == $repoFullName);
  def branch_only_entry:
    if ($projectId != ""
        and ([ $mergedProjects[] | select(.id == $projectId) ] | length) == 0
        and (($branchSurface.response.branches // []) | length) > 0) then
      [{
        id: $projectId,
        name: null,
        repoFullName: null,
        isSaved: ($projectId == $savedProjectId),
        visibleVia: {
          orgTree: false,
          bareProjectList: false,
          userProjectList: false,
          branchSurface: true
        },
        runtimeProfile: null,
        settings: null,
        branchData: null,
        branchSurface: $branchSurface,
        metadataUnavailable: true
      }]
    else
      []
    end;
  ([ 
      $mergedProjects[]
      | select(project_matches(.))
      | . as $project
      | {
          id,
          name,
          repoFullName,
          isSaved: (.id == $savedProjectId),
          visibleVia: {
            orgTree: ([($orgTree.projects // [])[] | select(.id == $project.id)] | length) > 0,
            bareProjectList: ([($spaceList.projects // [])[] | select(.id == $project.id)] | length) > 0,
            userProjectList: ([($userList.projects // [])[] | select(.id == $project.id)] | length) > 0,
            branchSurface: false
          },
          runtimeProfile: runtime_profile(.settings // {}),
          settings: {
            installCommand: (.settings.installCommand // null),
            devServerCommand: (.settings.devServerCommand // null),
            devServerUrl: (.settings.devServerUrl // null),
            mainBranchName: (.settings.mainBranchName // null),
            recommendedRoot: (.settings.recommendedRoot // null),
            environmentVariableCount: ((.settings.environmentVariables // []) | length)
          },
          branchData: (.branchData // null),
          metadataUnavailable: false
        }
    ] + branch_only_entry) as $projects
  | {
    mode: "repo-wrapper",
    stateFile: ($stateFile | select(length > 0)),
    savedProjectId: ($savedProjectId | select(length > 0)),
    authContext: {
      spaceId: $spaceId,
      userId: ($userId | select(length > 0))
    },
    surfaces: {
      orgTreeProjectCount: (($orgTree.projects // []) | length),
      bareProjectListCount: (($spaceList.projects // []) | length),
      userProjectListCount: (($userList.projects // []) | length),
      mergedProjectCount: ($mergedProjects | length),
      branchSurfaceCount: (($branchSurface.response.branches // []) | length)
    },
    projectCount: ($projects | length),
    projects: $projects
  }'
