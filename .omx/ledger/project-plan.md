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

- **Socratic project picture workspace** as the planning center for the
  project-picture direction
- **Project-Picture-Centered Planning Consolidation Plan** as landed
- **Planner design system**, **Planner UI reset**, and **Knowledge library**
  sitting beneath that center as the immediate experience layer
- **Hidden truth-model / blueprint relationship** now chosen as the first
  structural branch
- **Overlay / reorientation** next in the structural sequence
- **Whole-project recoverability** later in that structural sequence

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
- treat **Project-Picture-Centered Planning Consolidation Plan** and the broad
  Socratic thesis as landed parent surfaces
- advance **Hidden truth-model / blueprint relationship** as the current first
  structural branch
- then approach **overlay / reorientation** as the next structural branch and
  **whole-project recoverability** after that
- then return to the canonical queue for the next routed item
