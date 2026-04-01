#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
CONFIG_FILE="${BUILDER_PROJECT_CONFIG_PATH:-$REPO_ROOT/builder.config.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

source "$SCRIPT_DIR/builder-config-common.sh"

builder_require_jq
builder_resolve_config "$CONFIG_FILE"
builder_validate_resolved_config "$CONFIG_FILE"
builder_print_contract validate
printf 'Validation: ok\n'

