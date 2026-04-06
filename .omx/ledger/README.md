# Planner Ledger

The planner ledger is the **canonical OMX-linked tracking surface** for planner-wide work.

## Canonical files

- JSON source of truth: `.omx/ledger/planner-ledger.json`
- Human/project-readable surface: `.omx/ledger/current-status.md`
- Machine-readable automation trace: `.omx/ledger/automation-trace.json`
- Human-readable automation operator report: `.omx/ledger/automation-report.md`
- Project skill: `.codex/skills/project-ledger/SKILL.md`
- Command entrypoint: `scripts/project-ledger.mjs`

## What it tracks

The first slice tracks these object classes:

- governance artifacts
- initiatives
- workstreams
- slices
- plans
- implementations
- reviews
- deferred items
- decisions
- risks
- links between them

## Why it exists

This ledger is not just an index of documents.
It exists to:

- preserve linkage between artifacts
- keep deferred work visible across slices
- give OMX explicit lifecycle / routing signals
- help both OMX and the user see what needs what next

## Routing model

The ledger uses routing states such as:

- `needs_deep_interview`
- `ready_for_ralplan`
- `ready_for_ralph`
- `needs_testing`
- `needs_analysis`
- `monitoring`
- `complete`

These are intentionally semi-automated in v1.
The ledger must be useful before full automation exists.

## Update protocol

When a durable artifact is created or an item's workflow stage changes:

1. update `.omx/ledger/planner-ledger.json`
2. run `npm run project:ledger:validate`
3. run `npm run project:ledger:refresh`
4. if the ledger model or command behavior changes, run `npm run test:ledger`
5. add any new durable doc/skill to `docs/session-start-and-doc-index.md` and `.codex/project-skill-config.md` when appropriate

## Commands

- `npm run project:status`
- `npm run project:ledger:validate`
- `npm run project:ledger:refresh`
- `npm run test:ledger`
