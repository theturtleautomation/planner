#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"

exec /home/thetu/.codex/skills/builder-workflow/scripts/get-fusion-project.sh \
  --state-file "$REPO_ROOT/.codex/builder-fusion-project.json" \
  "$@"
