---
name: project-ledger
description: Show the canonical planner ledger, validate its linkage, and surface what needs what next across the project.
---

# Project Ledger

Use the planner-wide canonical ledger to level-set the project and identify the next valid workflow mode.

## Use When

- the user asks "where are we at?"
- the user wants to know what needs deep-interview / ralplan / ralph / testing / analysis next
- a session needs project-wide grounding before planning or execution
- durable artifacts are starting to sprawl and need linkage/status clarity

## Canonical Sources

- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/current-status.md`
- `scripts/project-ledger.mjs`

## Commands

- `npm run project:status` — print the current ledger summary
- `npm run project:ledger:validate` — validate ledger structure and artifact links
- `npm run project:ledger:refresh` — regenerate the readable status surface
- `npm run project:ledger:auto` — apply bounded ledger/status/routing automation and refresh the readable surface
- `npm run test:ledger` — verify the ledger command/model

## Workflow

1. Read `.omx/ledger/current-status.md`.
2. If the request depends on current routing/tracking truth, run:
   - `npm run project:ledger:validate`
   - `npm run project:status`
3. If durable artifacts were added or statuses changed and deterministic automation should apply, run:
   - `npm run project:ledger:auto`
   - The automation path may now use repo-graph evidence directly when evaluating bounded routing / next-mode mutations.
   - Treat any resulting routing mutation as inspectable; the automation trace should explain why it happened.
   - Confidence/provenance may also appear in durable ledger state when the trust policy requires it.
   - Operators should use the human-readable report at `.omx/ledger/automation-report.md` for rolling history rather than relying on the compact status surface alone.
4. If durable artifacts were added but automation should not mutate state, run:
   - `npm run project:ledger:refresh`
5. Use the routing queue to identify the next valid OMX move.

## Output Expectations

Provide a compact status report covering:
- active initiatives/workstreams
- queued next-mode work
- deferred items still alive
- important risks or blockers
- next valid move

## Refuse To

- treat the ledger as optional when project-wide status truth matters
- invent lifecycle state without updating the canonical ledger
- leave durable new work unlinked when the ledger is in scope
