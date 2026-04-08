# Planner — OMX Project Plan

**Status:** Active  
**Date:** 2026-04-08

## Purpose

This is the OMX-native top-level planning surface for the Planner repo.

It does not replace feature-specific plans or specs. It exists to give a
lightweight top-level view of:

- the current repo-wide planning spine
- the active cross-family threads
- the next expected planning move before implementation

Canonical routing/status truth remains in:

- [.omx/ledger/current-status.md](/home/thetu/planner/.omx/ledger/current-status.md)
- [.omx/ledger/planner-ledger.json](/home/thetu/planner/.omx/ledger/planner-ledger.json)

## Current Planning Spine

These are the main repo-wide planning families:

- [Planner Ledger Population Analysis And Pass Plan](/home/thetu/planner/docs/planner-ledger-population-analysis-and-pass-plan.md)
- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)
- [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)
- `docs/planner-solidstart-*.md`
- `docs/planner-ui-reset-*.md`
- `docs/planner-design-system-*.md`
- `docs/socratic-*.md`
- `docs/builder-*.md`

## Current Active Thread

The repo-wide planning truth currently shows:

- **Planner project tracking library** as the main active planner-wide
  initiative, with **Planner-Ledger Review Remediation Pass** now the concrete next move
- **Planner design system command center plan** ready for `$ralplan`
- **Planner UI reset route-by-route queue** ready for `$ralplan`
- **Knowledge library project scope plan** ready for `$ralplan`
- **Socratic project picture workspace** still needing `$deep-interview`

Read `.omx/ledger/current-status.md` for the canonical routed queue and
maintenance state.

## Working Rule

Keep planning and implementation aligned to artifact state:

1. Define or update the relevant plan/spec.
2. Confirm the canonical ledger matches the intended next move.
3. Implement only from a planning artifact that is ready enough for bounded execution.
4. After implementation, verify and sync docs/ledger status.

## Next Expected Move

The next move is:

- use `.omx/ledger/current-status.md` as the canonical routed queue
- execute **Planner-Ledger Review Remediation Pass** as the current concrete planner-ledger next step
- then return to the canonical queue for the next routed item
