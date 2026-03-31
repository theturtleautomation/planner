#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"

exec /home/thetu/.codex/skills/builder-workflow/scripts/diagnose-fusion-project-visibility.sh \
  --state-file "$REPO_ROOT/.codex/builder-fusion-project.json" \
  "$@"
