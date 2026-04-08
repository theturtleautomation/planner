# Planner Ledger Population Analysis And Pass Plan

**Status:** draft analysis artifact  
**Date:** 2026-04-04  
**Parent plan:** [PRD — Planner Ledger Population Analysis](/home/thetu/planner/.omx/plans/prd-planner-ledger-population-analysis.md)  
**Canonical ledger:** [planner-ledger.json](/home/thetu/planner/.omx/ledger/planner-ledger.json)  
**Readable surface:** [current-status.md](/home/thetu/planner/.omx/ledger/current-status.md)

## 1. Purpose

This artifact implements the **population-analysis** slice for the planner-wide
canonical ledger.

It does **not** attempt full planner-wide population.
It does **not** attempt OMX automation.

It exists to make the later population work deep, consistent, and executable by:

1. inventorying the major planner artifact/workstream families,
2. defining how those families should map into the full ledger model,
3. tightening the relationship vocabulary needed for planner-wide linkage,
4. producing a concrete multi-pass population strategy.

## 2. Evidence Base

### 2.1 Current repo-grounded evidence

- `docs/` currently contains **173** markdown files.
- Current durable OMX artifacts include:
  - `.omx/plans/` → **20** files
  - `.omx/specs/` → **19** files
  - `.omx/interviews/` → **10** files
  - `.omx/context/` → **10** files
  - `.omx/ledger/` → **3** files before this analysis doc was added to the repo docs
- Repo-local skills under `.codex/skills/` currently total **29** directories.
- Major code/package surfaces currently include:
  - `planner-solid` → **3348** `ts/tsx` files discovered by repo scan
  - `planner-web` → **3012** `ts/tsx` files discovered by repo scan
  - `planner-server` → **14** Rust files
  - `planner-core` → **55** Rust files
  - `planner-schemas` → **21** Rust files
  - `planner-tui` → **6** Rust files

### 2.2 Major doc-family counts from repo scan

The current `docs/` tree is not random; it already clusters into major families:

| Family | Count | Examples |
| --- | ---: | --- |
| `planner-solidstart-*` | 62 | route/spec phases, runtime/remediation specs |
| `socratic-*` | 27 | Socratic workspace, lobby, project-picture direction |
| `phase-*` | 21 | earlier phase implementation / execution docs |
| `import-existing-project-*` | 17 | import/history/reconciliation family |
| `planner-ui-reset-*` | 15 | UI reset route family |
| `builder-*` | 11 | builder fusion / documented config / runtime sync |
| `planner-design-system-*` | 9 | design-system phases |
| blueprint/admin/knowledge singletons | 3 | blueprint, admin, knowledge scope docs |
| blueprint research subfolder | 3 | research/deep-dive references |
| top-level planning/index docs | small but critical | `.omx/ledger/project-plan.md`, `.omx/ledger/session-start-and-doc-index.md`, `project-first-ui-research-sessions.md` |

This means planner-wide population should be decomposed by **artifact family** and
**planning role**, not by a flat per-file sweep.

## 3. What Full-Model Coverage Means

The user explicitly wants the ledger to become deep across the planner project.
For this analysis, **full-model coverage** means every major tracked item should
ultimately have more than a placeholder row.

Minimum eventual full-model fields per item:

- stable id
- kind
- title
- status
- routing state
- summary
- artifact links
- parent/child or container relationship where relevant
- follow-on/deferred relationships where relevant
- review / implementation / decision / risk links where relevant
- enough linkage to explain what it is, how it relates, and what should happen next

This is deeper than “file exists in repo.”
It is also deeper than “one note per doc.”

## 4. Planner-Wide Artifact Families To Populate

The first population execution passes should treat these as the major families.

### 4.1 Governance / root surfaces

These anchor the whole planner project:

- `README.md`
- `AGENTS.md`
- `CLAUDE.md`
- `.codex/project-skill-config.md`
- `.omx/ledger/session-start-and-doc-index.md`
- `.omx/ledger/project-plan.md`
- `.omx/ledger/*`

**Ledger role:** initiative/workstream containers, governance decisions, root risks,
canonical tracking infrastructure.

### 4.2 Planner-wide planning families

These are the major durable planning clusters already visible in the docs tree:

- SolidStart migration / route families
- Socratic workspace/product families
- UI reset families
- import-existing-project families
- design-system families
- builder/runtime-sync families
- audit/remediation families
- blueprint/knowledge/admin families

**Ledger role:** mostly workstreams and slices, plus some reviews and risks.

### 4.3 OMX planning/runtime artifacts

These are not just implementation traces; many are durable planning surfaces:

- `.omx/plans/*`
- `.omx/specs/*`
- `.omx/interviews/*`
- `.omx/context/*`
- `.omx/specs/socratic-deferred-items/*`

**Ledger role:** plans, slices, deferred items, decisions, follow-ons.

### 4.4 Code/package surfaces

These should not all become independent top-level ledger items by default, but
some workstreams and implementations should link to them explicitly:

- `planner-solid`
- `planner-web`
- `planner-server`
- `planner-core`
- `planner-schemas`
- `planner-tui`

**Ledger role:** implementations and platform/workstream anchors, not a file-by-file catalog.

### 4.5 Repo-local skills and workflow helpers

These are now part of how planner work moves:

- `.codex/skills/*`
- `scripts/*` workflow/verification helpers

**Ledger role:** implementations, workflow infrastructure, and in some cases
initiative/supporting-system artifacts.

## 5. Recommended Full-Model Mapping Rules

### 5.1 Initiative
Use for planner-wide strategic lanes that own multiple workstreams or major
follow-on slices.

Examples:
- Planner project tracking library
- Planner SolidStart platform direction
- Import existing project program

### 5.2 Workstream
Use for coherent product/implementation streams with multiple child slices or
follow-on concerns.

Examples:
- Socratic project picture workspace
- Planner design system
- Builder fusion runtime sync
- Import existing project history/reconciliation

### 5.3 Slice
Use for bounded buildable or analyzable increments.

Examples:
- first reveal
- area workspace
- seed handling
- planner-ledger population analysis
- future planner-ledger population pass 1

### 5.4 Plan
Use for approved planning artifacts that govern a slice or workstream.

Examples:
- PRDs
- test specs

### 5.5 Review
Use for evaluative artifacts.

Examples:
- current-state-vs-thesis review
- audits
- architecture reviews
- remediation review docs

### 5.6 Deferred item
Use for still-live concerns that must remain linked even when not yet active.

Examples:
- hidden truth-model relationship
- overlay/reorientation
- provenance/change inspection

### 5.7 Decision
Use for durable constraints that shape later work.

Examples:
- seed loop boundedness
- ledger is canonical
- user authority / autonomy boundary decisions

### 5.8 Risk
Use for unresolved coordination or quality threats that deserve visibility.

Examples:
- artifact sprawl
- ledger staleness
- route-family drift

## 6. Relationship Vocabulary To Standardize

The current ledger proves basic linking, but planner-wide population needs a
more explicit relationship vocabulary.

Recommended standard vocabulary:

- `parent` / `children`
- `plan_for` / `planned_by`
- `implementation_for` / `implemented_by`
- `review_of` / `reviewed_by`
- `decision_for` / `constrained_by_decision`
- `risk_for` / `at_risk_because`
- `follow_on` / `follow_on_from`
- `deferred_from` / `has_deferred_item`
- `blocked_by` / `unblocks`
- `supersedes` / `superseded_by`
- `informs` / `informed_by`

Two important rules:

1. **Every populated family should use the same vocabulary.**
   No one-off edge naming per workstream.
2. **Readable surfaces should derive from canonical links.**
   Do not maintain a separate hand-written relationship map when the ledger can express it directly.

## 7. External Best-Practice Signals

This slice is planner-specific, but a few outside patterns are useful guardrails:

1. **Canonical catalog/entity model first, readable views second**
   Backstage’s software catalog emphasizes a canonical entity model plus
   relations, then derives views from that source model rather than treating the
   human-readable surface as the canonical truth.
   - https://backstage.io/docs/features/software-catalog/descriptor-format
   - https://backstage.io/docs/features/software-catalog/extending-the-model

2. **Decision records should live close to the code/work they constrain**
   ADR practice reinforces keeping decisions close to the repo and linking them
   clearly to context, status, and consequences.
   - https://github.com/thomvaill/log4brains

3. **Do not over-automate before the taxonomy is stable**
   The first automation layer should come after the model and relations are
   trustworthy. Otherwise automation amplifies bad structure rather than helping.

These are supporting references, not product truth.
The planner ledger should still be shaped by repo reality first.

## 8. Multi-Pass Population Strategy

The user wants eventual full-model planner-wide coverage.
The safest path is a **deep but staged population program**.

### Pass 0 — This analysis slice
**Goal:** inventory + mapping rules + pass plan  
**Output:** this artifact and the follow-on pass definitions.

### Pass 1 — Root governance + canonical planning spine
Populate the full model for the root planner tracking spine:

- root governance docs
- ledger artifacts
- session bootstrap/index docs
- project-plan top-level coordination
- the highest-level planner initiatives/workstreams

**Why first:** these define the containers and routing anchors every later family should attach to.

### Pass 2 — Socratic + SolidStart active families
Populate full-model entries for the currently most active product families:

- Socratic project picture family
- key SolidStart/route-family planning spines
- current reviews / active deferred items / current implementations

**Why second:** these already have the richest recent linkage evidence and are the best place to prove the model at scale.

### Pass 3 — Import / blueprint / knowledge / builder families
Populate the next highest-value planner families that already span multiple docs
and follow-on lanes:

- import-existing-project family
- builder fusion family
- blueprint/knowledge families

### Pass 4 — Design-system + UI reset + audits/remediation families
Populate the large historical-but-still-relevant planning families:

- planner design system
- planner UI reset
- audits / remediation / closeout families

### Pass 5 — Implementation linkage enrichment
Once the planning families are in place, add deeper implementation/review links
across code/package surfaces where they materially clarify reality.

### Pass 6 — Coverage integrity sweep
Review for:

- orphaned artifacts
- inconsistent status/routing states
- missing parent/child containers
- weak follow-on/deferred links
- stale readable-surface assumptions

## 9. Recommended Next Valid Move

The next valid execution move after this analysis slice is **not** automation.
It is:

- a bounded first population execution pass for **root governance + canonical planning spine**

That next pass should likely enter:
- `$ralplan`, using this analysis artifact as the grounding surface for the first execution-oriented population pass.

My recommendation:
- run `$ralplan` for **Planner Ledger Population Pass 1 — Root Governance And Planning Spine**

## 10. Automation Remains Later

Automation should stay separate until:

- the artifact family taxonomy is stable,
- the relationship vocabulary is stable,
- at least the first one or two population passes prove the model can be maintained consistently.

Only then should OMX automation attempt to:

- suggest ledger updates,
- propose routing state changes,
- or auto-read project status with stronger confidence.
