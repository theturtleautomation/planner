# OMX Update Change Preservation Audit

## Purpose

One-time audit of whether a recently run OMX update affected the **repo-local Graphify / repo-graph OMX lane** in `/home/thetu/planner`.

This audit is intentionally **repo-local**. It does **not** attempt to preserve or restore the globally installed npm package for `oh-my-codex` because no evidence was found that the Graphify/repo-graph work was implemented there.

## Scope Boundary

### In scope
- Historical committed repo-local Graphify / repo-graph / ledger OMX changes
- Current post-update repo-local OMX-ish state
- Comparison of expected-vs-current repo-local surfaces
- Remediation guidance for likely drift

### Out of scope
- Global npm-installed `oh-my-codex` package restoration
- Re-running `omx update`
- Automatic restore actions in this audit

## Evidence Sources

### Current OMX install context
- `omx` resolves to `/home/thetu/.nvm/versions/node/v22.18.0/bin/omx`
- real script path: `/home/thetu/.nvm/versions/node/v22.18.0/lib/node_modules/oh-my-codex/dist/cli/omx.js`
- version: `oh-my-codex v0.12.3`

### Managed/regeneration signal
- `.codex/config.toml` contains: `Managed by omx setup - manual edits preserved on next setup`

### Historical commit trail reviewed
- `ae79e80` — planner-ledger and repo-graph workflow state preserved as auditable project artifacts
- `5434c9b` — newly authored Socratic and repo-graph planning artifacts preserved without leaking local Codex state
- `8f11976` — repo-graph reporting stabilized around one canonical analyzer path

### Current post-update repo-local OMX deltas
- modified `.agents/plugins/marketplace.json`
- modified `.codex/project-skill-config.md`
- deleted `.codex/skills/delivery-cycle`
- deleted `.codex/skills/project-bootstrap`
- deleted `.codex/skills/spec-lifecycle`

## Historical Graphify / Repo-Graph Lane

### `ae79e80` — foundational committed repo-graph + ledger lane
This commit preserved the core repo-graph workflow and planner-ledger surfaces, including:
- `.codex/skills/project-ledger/SKILL.md`
- `.codex/skills/repo-graph/SKILL.md`
- `.omx/ledger/*`
- `docs/graphify-omx-handoff.md`
- `docs/repo-graph-workflow.md`
- `scripts/repo-graph.sh`
- `scripts/repo-graph-mcp.sh`
- `scripts/repo-graph-status.sh`
- `scripts/repo_graph.py`
- `scripts/repo_graph_mcp.py`
- related repo-graph tests/bootstrap helpers

### `5434c9b` — repo-local plugin wiring preserved
This commit preserved the repo-local repo-graph plugin surfaces:
- `plugins/planner-repo-graph/.codex-plugin/plugin.json`
- `plugins/planner-repo-graph/.mcp.json`

### `8f11976` — reporting/analyzer stabilization
This commit preserved the reporting/analyzer lane, including:
- `.codex/skills/repo-graph/SKILL.md`
- `.omx/ledger/automation-report.md`
- `.omx/ledger/automation-trace.json`
- `.omx/ledger/planner-ledger.json`
- `docs/graphify-omx-handoff.md`
- `docs/repo-graph-workflow.md`
- `scripts/repo_graph.py`
- `scripts/repo_graph_test.py`
- family-fidelity reporting files

## Current Post-Update State

### Core committed repo-graph surfaces still present
The following key Graphify/repo-graph surfaces still exist after the update:
- `AGENTS.md`
- `.codex/skills/repo-graph/SKILL.md`
- `docs/repo-graph-workflow.md`
- `docs/graphify-omx-handoff.md`
- `.omx/specs/deep-interview-repo-graph-over-grep-routing.md`
- `.omx/plans/prd-repo-graph-over-grep-routing-first-pass.md`
- `.omx/plans/test-spec-repo-graph-over-grep-routing-first-pass.md`
- `scripts/repo-graph.sh`
- `scripts/repo-graph-mcp.sh`
- `scripts/repo-graph-status.sh`
- `scripts/repo_graph.py`
- `scripts/repo_graph_mcp.py`
- `.agents/plugins/marketplace.json`

### Current post-update deltas
#### 1. `.agents/plugins/marketplace.json` — modified
Current diff adds a local plugin registration for:
- `planner-repo-graph`

Interpretation:
- This is **aligned** with the committed repo-local plugin wiring from `5434c9b`.
- It looks like repo-local plugin registration drift/resync, not loss of the Graphify/repo-graph lane.

#### 2. `.codex/project-skill-config.md` — modified
Current diff changes project bootstrap/planning truth from legacy docs such as:
- `docs/session-start-and-doc-index.md`
- `docs/project-plan.md`

to OMX ledger surfaces such as:
- `.omx/ledger/session-start-and-doc-index.md`
- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/current-status.md`
- `.omx/ledger/project-plan.md`
- `.omx/ledger/README.md`

Interpretation:
- This appears to be **OMX bootstrap/planning model regeneration or migration**, not loss of repo-graph itself.
- It affects repo-local OMX workflow entrypoints, but it does **not** indicate the committed repo-graph lane was wiped.

#### 3. `.codex/skills/delivery-cycle`, `.codex/skills/project-bootstrap`, `.codex/skills/spec-lifecycle` — deleted
Important evidence:
- These paths still exist at `HEAD` as symlinked entries.
- Their targets still exist under:
  - `/home/thetu/skills/delivery-cycle`
  - `/home/thetu/skills/project-bootstrap`
  - `/home/thetu/skills/spec-lifecycle`

Interpretation:
- The underlying skills are **not missing**.
- The repo-local `.codex/skills/*` entries were deleted from the working tree after the update.
- This is likely **repo-local link removal / regeneration drift**, not loss of the core Graphify/repo-graph lane.
- These deleted skills are also **not core Graphify/repo-graph artifacts**; they belong to the older bootstrap/spec workflow lane.

## Comparison — What Likely Changed vs What Survived

### Survived intact
The **main Graphify / repo-graph lane appears preserved** in committed repo-local artifacts:
- repo-graph docs are still present
- repo-graph scripts are still present
- repo-graph MCP/plugin wiring is still present
- repo-graph-related plans/specs are still present

This means the most important Graphify/repo-graph work was **not wiped** by the update.

### Likely update/regeneration drift
The update appears to have affected **repo-local OMX overlay/bootstrapping surfaces**, especially:
- `.codex/project-skill-config.md`
- `.agents/plugins/marketplace.json`
- repo-local `.codex/skills/*` symlink entries for legacy external skills

These changes look more like **setup/update/regeneration drift** than destruction of the core repo-graph implementation.

### Confirmed intentional cleanup
The deleted legacy skill links are now confirmed as **intentional cleanup**, not update damage:
- `.codex/skills/delivery-cycle`
- `.codex/skills/project-bootstrap`
- `.codex/skills/spec-lifecycle`

The symlink targets still exist under `/home/thetu/skills/`, but the repo-local links were intentionally removed as part of the older bootstrap/spec-workflow cleanup direction already reflected in:
- `.omx/specs/deep-interview-remove-legacy-bootstrap-skills-and-docs.md`
- `.omx/plans/prd-remove-legacy-bootstrap-skills-and-docs.md`

Conclusion: these three deletions should be treated as **preserved intended state**, not as update-caused drift.

## Remediation Recommendations

### Recommendation 1 — No emergency restore needed for the core Graphify/repo-graph lane
Because the key committed repo-graph surfaces are still present, there is **no evidence of catastrophic loss** of the main Graphify integration work.

### Recommendation 2 — Treat `.codex/project-skill-config.md` as an update/regeneration review item
Action:
- compare whether the new ledger-centric bootstrap model is now the intended repo truth
- if yes, keep and commit the change
- if no, restore the previous bootstrap/tracker references intentionally

Why:
- this file changes how OMX grounds the repo
- it is meaningful drift, but not specifically a Graphify wipeout

### Recommendation 3 — Treat `.agents/plugins/marketplace.json` as plugin-registration alignment work
Action:
- verify the `planner-repo-graph` registration matches `plugins/planner-repo-graph/.codex-plugin/plugin.json`
- if aligned and desired, keep/commit it

Why:
- this change appears additive and supportive of the repo-graph lane, not destructive

### Recommendation 4 — Keep the three deleted legacy skill links deleted
Action:
- treat these deletions as intentional cleanup, not update remediation targets
- do not restore the repo-local symlinks unless repo direction changes later

Why:
- the deletion intent is now confirmed
- the underlying shared skill targets still exist, so nothing was actually lost

### Recommendation 5 — Run a focused restoration pass only if you want repo-local OMX overlays normalized immediately
If you want a clean post-update repo-local OMX state, the next execution lane should:
1. audit these five changed/deleted OMX-ish surfaces,
2. decide keep/restore/remove intentionally,
3. commit the resulting normalized state.

## Bottom Line

The evidence shows that the **core committed Graphify/repo-graph lane survived** the update. What changed is the **repo-local OMX overlay/bootstrap layer**: the ledger/bootstrap config shifted, the plugin marketplace drifted, and three legacy repo-local skill symlinks were deleted.

So the important Graphify/repo-graph work appears preserved. The remaining issue is **repo-local OMX overlay normalization**, not recovery of a lost repo-graph system.

## Quick Verification Command

After future OMX updates/setup refreshes, run:

```bash
npm run omx:post-update:verify
```

This checks the preserved repo-graph lane, the intentional absence of legacy skill links, marketplace/plugin alignment, project-skill-config expectations, repo-graph health, and targeted git status for the repo-local OMX surfaces we care about.

## Suggested Next Move
If you want, the next best action is a bounded restore/normalize pass over:
- `.codex/project-skill-config.md`
- `.agents/plugins/marketplace.json`
- `.codex/skills/delivery-cycle`
- `.codex/skills/project-bootstrap`
- `.codex/skills/spec-lifecycle`

with explicit decisions for each: **keep, restore, or intentionally remove**.
