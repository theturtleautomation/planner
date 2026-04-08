#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
cd "$REPO_ROOT"

failures=0
warnings=0

pass() {
  printf 'PASS: %s\n' "$1"
}

warn() {
  warnings=$((warnings + 1))
  printf 'WARN: %s\n' "$1"
}

fail() {
  failures=$((failures + 1))
  printf 'FAIL: %s\n' "$1"
}

check_present() {
  local path="$1"
  if [[ -e "$path" ]]; then
    pass "present: $path"
  else
    fail "missing expected path: $path"
  fi
}

check_missing() {
  local path="$1"
  if [[ -e "$path" ]]; then
    warn "expected absent but present: $path"
  else
    pass "absent as expected: $path"
  fi
}

printf 'OMX post-update verification\n'
printf '============================\n\n'

if command -v omx >/dev/null 2>&1; then
  printf 'omx version:\n'
  omx version | sed -n '1,4p'
  printf '\n'
else
  warn 'omx command not found in PATH'
fi

printf 'Repo-local audit basis:\n'
printf -- '- %s\n' 'docs/omx-update-change-preservation-audit.md'
printf -- '- %s\n' '.agents/plugins/marketplace.json'
printf -- '- %s\n' '.codex/project-skill-config.md'
printf '\n'

printf '1) Core repo-graph / Graphify surfaces\n'
check_present 'docs/omx-update-change-preservation-audit.md'
check_present 'AGENTS.md'
check_present '.codex/skills/repo-graph/SKILL.md'
check_present 'docs/repo-graph-workflow.md'
check_present 'docs/graphify-omx-handoff.md'
check_present 'scripts/repo-graph.sh'
check_present 'scripts/repo-graph-mcp.sh'
check_present 'scripts/repo-graph-status.sh'
check_present 'scripts/repo_graph.py'
check_present 'scripts/repo_graph_mcp.py'
printf '\n'

printf '2) Intentional legacy-skill cleanup state\n'
check_missing '.codex/skills/delivery-cycle'
check_missing '.codex/skills/project-bootstrap'
check_missing '.codex/skills/spec-lifecycle'

for target in \
  '/home/thetu/skills/delivery-cycle' \
  '/home/thetu/skills/project-bootstrap' \
  '/home/thetu/skills/spec-lifecycle'; do
  if [[ -e "$target" ]]; then
    pass "legacy shared skill target still exists: $target"
  else
    warn "legacy shared skill target missing: $target"
  fi
done
printf '\n'

printf '3) Marketplace / plugin registration alignment\n'
if command -v jq >/dev/null 2>&1; then
  plugin_path="$(jq -r '.plugins[] | select(.name == "planner-repo-graph") | .source.path // ""' .agents/plugins/marketplace.json 2>/dev/null)"
  if [[ "$plugin_path" == './plugins/planner-repo-graph' ]]; then
    pass 'planner-repo-graph marketplace registration is aligned'
  else
    fail "planner-repo-graph marketplace registration missing or mismatched (found: ${plugin_path:-<none>})"
  fi
else
  warn 'jq not available; skipped marketplace alignment check'
fi
check_present 'plugins/planner-repo-graph/.codex-plugin/plugin.json'
check_present 'plugins/planner-repo-graph/.mcp.json'
printf '\n'

printf '4) Project-skill config expectations\n'
if grep -Fq '.omx/ledger/planner-ledger.json' .codex/project-skill-config.md && \
   grep -Fq '.omx/ledger/current-status.md' .codex/project-skill-config.md; then
  pass 'project-skill-config points at OMX ledger surfaces'
else
  fail 'project-skill-config no longer points at expected OMX ledger surfaces'
fi
printf '\n'

printf '5) Repo-graph health\n'
if bash scripts/repo-graph-status.sh >/tmp/omx-post-update-repo-graph-status.$$ 2>&1; then
  pass 'repo-graph status script succeeded'
  sed -n '1,40p' /tmp/omx-post-update-repo-graph-status.$$
  if grep -Fq 'Stale: yes' /tmp/omx-post-update-repo-graph-status.$$; then
    warn 'repo-graph reports stale=yes; run npm run repo-graph:update or the post-refresh command'
  fi
  if grep -Fq 'MCP lifecycle state: refresh_needed' /tmp/omx-post-update-repo-graph-status.$$; then
    warn 'repo-graph MCP lifecycle state is refresh_needed'
  fi
else
  fail 'repo-graph status script failed'
  sed -n '1,80p' /tmp/omx-post-update-repo-graph-status.$$ || true
fi
rm -f /tmp/omx-post-update-repo-graph-status.$$
printf '\n'

printf '6) Targeted git status\n'
status_output="$(git status --short \
  .agents/plugins/marketplace.json \
  .codex/project-skill-config.md \
  .codex/skills/delivery-cycle \
  .codex/skills/project-bootstrap \
  .codex/skills/spec-lifecycle \
  docs/omx-update-change-preservation-audit.md \
  2>/dev/null || true)"
if [[ -n "$status_output" ]]; then
  warn 'targeted OMX local surfaces are dirty relative to HEAD; inspect below'
  printf '%s\n' "$status_output"
else
  pass 'targeted OMX local surfaces are clean relative to HEAD'
fi
printf '\n'

printf 'Summary\n'
printf -- '-------\n'
printf 'Failures: %d\n' "$failures"
printf 'Warnings: %d\n' "$warnings"

if (( failures > 0 )); then
  printf '\nResult: FAIL\n'
  exit 1
fi

if (( warnings > 0 )); then
  printf '\nResult: WARN\n'
  exit 0
fi

printf '\nResult: PASS\n'
