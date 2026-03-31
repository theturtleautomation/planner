#!/usr/bin/env bash
set -euo pipefail

SKILL_SCRIPT="${BUILDER_SYNC_SKILL_SCRIPT:-/home/thetu/.codex/skills/builder-workflow/scripts/upsert-project-entry.sh}"

if [[ ! -x "$SKILL_SCRIPT" ]]; then
  echo "Builder sync helper not found or not executable: $SKILL_SCRIPT" >&2
  exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
RUNTIME_URL="${BUILDER_PROJECT_RUNTIME_URL:-http://127.0.0.1:4174}"
PROXY_URL="${BUILDER_PROJECT_PROXY_URL:-http://127.0.0.1:48752}"

exec "$SKILL_SCRIPT" \
  --path "$REPO_ROOT" \
  --runtime-url "$RUNTIME_URL" \
  --proxy-url "$PROXY_URL" \
  "$@"
