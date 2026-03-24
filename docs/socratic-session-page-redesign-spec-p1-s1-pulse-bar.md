# Socratic Session Page Redesign Spec P1 S1: Pulse Bar

**Status:** superseded planning artifact
**Date:** 2026-03-22
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)

> Planning note (2026-03-24): this document is a historical early-slice
> planning artifact from the pre-consultant-desk / pre-master-detail era of
> the Socratic lobby redesign. It no longer defines an active or future-state
> implementation target. Keep it only as historical design/planning context.

## Problem & Intent
The current Socratic session page uses `SessionStatusHeader`, `ConvergenceBar`, and `PipelineBar` simultaneously at the top of the layout. This creates a noisy, dashboard-like "chrome" that competes with the question workspace. The intent is to condense system status into a single, minimal floating utility header (the "Pulse Bar") that communicates session readiness, elapsed time, and entry points to the Context Shelf without dominating the screen.

## User Outcome
The user sees a clean, distraction-free interface where the top of the screen gently informs them of the session's health (e.g., "Ready", "Preparing") and gives them a single place to access secondary tools (Context Shelf, Back button), making the page feel like a focused document rather than a monitoring tool.

## Scope Boundaries
**In Scope:**
- Creating a unified, minimal header for `SessionPage.tsx`.
- Removing the heavy `SessionStatusHeader`, `PipelineBar`, and `ConvergenceBar` from the default view.
- Moving session actions (Rename, Duplicate, Export, Archive) into an overflow menu or simplified utility row.

**Out of Scope:**
- Changing the underlying WebSocket event data or the logic for determining session status.
- Redesigning the Context Shelf internals (handled in P1 S3).
- Moving the actual question map (handled in P2).

## Contracts and Touched Surfaces
- `planner-web/src/pages/SessionPage.tsx`: Remove old headers and integrate the new pulse bar layout.
- `planner-web/src/components/SessionStatusHeader.tsx`: May be deprecated or significantly thinned down into a floating pill.
- `planner-web/src/components/ConvergenceBar.tsx` / `PipelineBar.tsx`: Remove from standard render flow in `SessionPage.tsx`.

## Acceptance Criteria
- [ ] The top of the `SessionPage` features a single, compact row or floating pill.
- [ ] The session title is visible.
- [ ] The overall status (e.g., green dot, "idle", or "pipeline running") is summarized in one line.
- [ ] The Context Shelf trigger is integrated cleanly into this header.
- [ ] The page layout retains its flex structure but has significantly less vertical space consumed by headers.

## Verification Plan
- Load a live Socratic session in the browser (or run `npx tsc --noEmit` and tests).
- Verify the header is minimal (< 60px height).
- Ensure the session title, back button, and context toggle are still accessible.

## Rollback or Fallback
If the consolidated header removes too much actionable info (like pipeline errors), we will re-introduce a toast notification system for critical errors.

## Open Questions
- None. This is a straightforward UI consolidation.
