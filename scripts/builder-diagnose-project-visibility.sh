#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-fusion-common.sh"

usage() {
  cat <<'EOF'
Usage: builder-diagnose-project-visibility.sh [options]

Diagnose why the saved Fusion project is or is not visible in the current
Builder auth context.

Options:
  --space-id ID         Builder space ID / public API key
  --project-id ID       Explicit Fusion project ID override
  --state-file PATH     Optional saved project state file
  -h, --help            Show this help

Environment:
  BUILDER_PRIVATE_API_KEY   Required
  BUILDER_PUBLIC_API_KEY    Used as default --space-id
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
effective_project_id="$project_id"
if [[ -z "$effective_project_id" ]]; then
  effective_project_id="$saved_project_id"
fi

cli_config_json="$(builder_repo_read_cli_config_json)"
dev_tools_json="$(builder_repo_read_dev_tools_json)"
current_user_id="$(builder_repo_detect_current_user_id)"
current_space_name="$(builder_repo_detect_current_space_name)"

org_tree_json="$(builder_repo_fetch_org_tree_json "$space_id" 2>/dev/null || printf '{"projects":[],"branches":[],"users":[]}\n')"
user_projects_json="$(builder_repo_fetch_user_projects_json "$space_id" "$current_user_id" 2>/dev/null || printf '{"projects":[]}\n')"
merged_projects_json="$(builder_repo_merge_project_surfaces_json "$org_tree_json" "$user_projects_json")"
direct_read_json="$(builder_repo_fetch_direct_project_json "$space_id" "$effective_project_id")"
direct_read_with_user_json="$(builder_repo_fetch_direct_project_json "$space_id" "$effective_project_id" "$current_user_id")"
branch_surface_json="$(builder_repo_fetch_project_branches_json "$space_id" "$effective_project_id")"
branch_surface_with_user_json="$(builder_repo_fetch_project_branches_json "$space_id" "$effective_project_id" "$current_user_id")"

jq -cn \
  --arg spaceId "$space_id" \
  --arg spaceName "$current_space_name" \
  --arg currentUserId "$current_user_id" \
  --arg stateFile "$state_file" \
  --arg projectId "$effective_project_id" \
  --arg envPrivateKey "${BUILDER_PRIVATE_API_KEY}" \
  --argjson savedProject "$saved_state_json" \
  --argjson cliConfig "$cli_config_json" \
  --argjson devTools "$dev_tools_json" \
  --argjson orgTree "$org_tree_json" \
  --argjson userList "$user_projects_json" \
  --argjson mergedProjects "$merged_projects_json" \
  --argjson directRead "$direct_read_json" \
  --argjson directReadWithUser "$direct_read_with_user_json" \
  --argjson branchRead "$branch_surface_json" \
  --argjson branchReadWithUser "$branch_surface_with_user_json" '
  def branch_count(surface):
    if surface == null then 0
    else ((surface.response.branches // []) | length)
    end;
  def body_or_raw(obj):
    if obj == null then null else obj end;
  def candidate_matches(projects; saved):
    [
      projects[]
      | select(
          ((saved.repoFullName // "") != "" and .repoFullName == saved.repoFullName)
          or
          ((saved.name // "") != "" and .name == saved.name)
        )
    ];
  def cause_classification(
    saved;
    currentSpaceId;
    currentUserId;
    orgProjects;
    userProjects;
    mergedProjects;
    exactVisible;
    candidateMatches;
    directHttp;
    directUserHttp
  ):
    if ((saved.spaceId // "") != "" and (saved.spaceId // "") != currentSpaceId) then
      "space_context_mismatch"
    elif ((saved.userId // "") != "" and (saved.userId // "") != "" and currentUserId != "" and (saved.userId // "") != currentUserId) then
      "auth_context_mismatch"
    elif ((candidateMatches | map(select(.id != (saved.id // ""))) | length) > 0) then
      "saved_project_stale"
    elif exactVisible and (((orgProjects | length) != (userProjects | length)) or (([orgProjects[] | select(.id == (saved.id // ""))] | length) != ([userProjects[] | select(.id == (saved.id // ""))] | length))) then
      "api_surface_mismatch"
    elif (((orgProjects | length) != (userProjects | length)) and ((orgProjects | length) > 0 or (userProjects | length) > 0)) then
      "api_surface_mismatch"
    elif (((mergedProjects | length) > 0) and ((saved.id // "") != "") and (exactVisible | not)) then
      "saved_project_stale"
    elif (((directHttp == 404) or (directUserHttp == 404)) and ((mergedProjects | length) == 0)) then
      if ((saved.spaceId // "") != "" and (saved.userId // "") != "" and (saved.spaceId // "") == currentSpaceId and (saved.userId // "") == currentUserId) then
        "builder_visibility_limitation"
      else
        "undetermined"
      end
    else
      "undetermined"
    end;
  def visibility_classification(metadataVisible; branchVisible):
    if metadataVisible and branchVisible then
      "fully_visible"
    elif (metadataVisible | not) and branchVisible then
      "branch_visible_only"
    elif metadataVisible and (branchVisible | not) then
      "metadata_visible_only"
    elif (metadataVisible | not) and (branchVisible | not) then
      "not_visible"
    else
      "undetermined"
    end;
  ($orgTree.projects // []) as $orgProjects
  | ($userList.projects // []) as $userProjects
  | $mergedProjects as $merged
  | (candidate_matches($merged; $savedProject)) as $candidateMatches
  | ([ $merged[] | select(.id == ($projectId // "")) ] | length > 0) as $exactVisible
  | ((branch_count($branchRead) > 0) or (branch_count($branchReadWithUser) > 0)) as $branchVisible
  | visibility_classification($exactVisible; $branchVisible) as $visibilityClassification
  | cause_classification(
      $savedProject;
      $spaceId;
      $currentUserId;
      $orgProjects;
      $userProjects;
      $merged;
      $exactVisible;
      $candidateMatches;
      ($directRead.httpCode // 0);
      ($directReadWithUser.httpCode // 0)
    ) as $causeClassification
  | {
      status: "ok",
      mode: "repo-wrapper",
      transport: "builder-cli-aligned-project-read",
      stateFile: ($stateFile | select(length > 0)),
      spaceId: $spaceId,
      spaceName: ($spaceName | select(length > 0)),
      savedProject: $savedProject,
      targetProjectId: ($projectId | select(length > 0)),
      auth: {
        envPrivateKeyPrefix: ($envPrivateKey[0:16] + "..."),
        cliConfigPresent: ($cliConfig != null),
        cliKeyMatchesEnv: (
          if $cliConfig == null then null
          else (($cliConfig.credentials.builderPrivateKey // "") == $envPrivateKey)
          end
        ),
        cliSpaceId: (
          if $cliConfig == null then null
          else ($cliConfig.credentials.builderPublicKey // null)
          end
        ),
        cliSpaceName: (
          if $cliConfig == null then null
          else ($cliConfig.credentials.spaceName // null)
          end
        ),
        cliUserId: (
          if $cliConfig == null then null
          else ($cliConfig.credentials.userId // null)
          end
        ),
        deviceUserId: (
          if $devTools == null then null
          else ($devTools.userId // null)
          end
        ),
        deviceId: (
          if $devTools == null then null
          else ($devTools.deviceId // null)
          end
        )
      },
      surfaces: {
        orgTree: {
          projectCount: ($orgProjects | length),
          branchCount: (($orgTree.branches // []) | length),
          userCount: (($orgTree.users // []) | length),
          firstUser: (($orgTree.users // [])[0] // null),
          response: body_or_raw($orgTree)
        },
        userProjectList: {
          projectCount: ($userProjects | length),
          response: body_or_raw($userList)
        },
        mergedVisibleProjects: {
          projectCount: ($merged | length),
          projects: $merged
        },
        directProjectRead: body_or_raw($directRead),
        directProjectReadWithUser: body_or_raw($directReadWithUser),
        branchSurface: body_or_raw($branchRead),
        branchSurfaceWithUser: body_or_raw($branchReadWithUser)
      },
      visibleMatches: {
        exactSavedProjectVisible: $exactVisible,
        branchSurfaceVisible: $branchVisible,
        candidateProjects: $candidateMatches
      },
      classification: $visibilityClassification,
      causeClassification: $causeClassification,
      legacyDiagnosis: (
        if $visibilityClassification == "branch_visible_only" then
          "branch_surface_visible_but_metadata_surface_missing"
        elif (($orgProjects | length) == 0 and ($userProjects | length) == 0 and (($directRead.httpCode // 0) == 404)) then
          "no_visible_projects_on_current_project_api_surface"
        elif (($orgProjects | length) == 0 and ($userProjects | length) == 0) then
          "org_tree_empty"
        elif (($directRead.httpCode // 0) == 404) then
          "saved_project_missing_but_other_projects_visible"
        else
          "project_surface_visible"
        end
      ),
      evidence: [
        (if (($savedProject.spaceId // "") == "") then "saved_project_missing_space_context" else empty end),
        (if (($savedProject.userId // "") == "") then "saved_project_missing_user_context" else empty end),
        (if (($orgProjects | length) == 0) then "org_tree_empty" else empty end),
        (if (($userProjects | length) == 0) then "user_project_list_empty" else empty end),
        (if (($directRead.httpCode // 0) == 404) then "direct_project_read_404" else empty end),
        (if (($directReadWithUser.httpCode // 0) == 404) then "direct_project_read_with_user_404" else empty end),
        (if (branch_count($branchRead) > 0) then "branch_surface_visible" else empty end),
        (if (branch_count($branchReadWithUser) > 0) then "branch_surface_visible_with_user" else empty end),
        (if (([ $candidateMatches[] | select(.id != ($savedProject.id // "")) ] | length) > 0) then "alternate_project_candidate_visible" else empty end),
        (if (((($orgProjects | length) != ($userProjects | length)) and (($orgProjects | length) > 0 or ($userProjects | length) > 0))) then "project_read_surfaces_disagree" else empty end)
      ]
    }'
