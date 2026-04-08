# Socratic Session Page Redesign Spec P2 S3: Dynamic Feedback

**Status:** superseded planning artifact
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

> Planning note (2026-03-24): this document is a historical early-slice
> planning artifact from the pre-consultant-desk / pre-master-detail era of
> the Socratic lobby redesign. It no longer defines an active or future-state
> implementation target. Keep it only as historical design/planning context.

## Problem & Intent
When the Socratic engine evaluates a batch of answers, it sends events over the WebSocket indicating that the next set of questions is being prepared or that the server has chosen to focus on a different branch. The old UI represented this via a `SessionStatusHeader` text update or a full page swap. The intent is to keep the user anchored in the Map and provide localized, high-visibility feedback ("preparing", "resolved", "moved") *inside* the relevant category card or Pulse Bar.

## User Outcome
When the user submits a batch of questions, the card immediately transitions into an inline "Preparing..." pulse. If the server decides that branch is complete, the card smoothly collapses, marked as "resolved", and the next active card slides open automatically. The UI feels completely synchronized with the server's backend loop without full-page re-renders.

## Scope Boundaries
**In Scope:**
- `planner-web/src/components/SocraticWorkspace.tsx`: Enhancing the `isPreparing` inline state. Currently it renders: `Preparing next questions...` text. We will replace this with a beautiful, premium skeleton loader / shimmer effect (as defined by `ui-ux-pro-max-skill` rules for dynamic micro-interactions).
- The "Focus transition" view needs to be visually distinct.

**Out of Scope:**
- Changing the WebSocket API payload or the `pendingCategoryId` logic.

## Contracts and Touched Surfaces
- `planner-web/src/components/SocraticWorkspace.tsx`

## Acceptance Criteria
- [ ] When `isPreparing` is true, the active card displays an animated skeleton/shimmer loader indicating backend generation is occurring.
- [ ] The loader is styled consistently with the UI (e.g., using `linear-gradient` with `backgroundSize` animation).
- [ ] The transition state ("This branch is in view even though...") includes a clear, polished warning UI (yellow/gold tone).

## Verification Plan
- Verify via `vitest` that the "Preparing next questions" state is still accessible to tests.
- Visual inspection of the CSS animations in the code.

## Rollback or Fallback
If the shimmer animation causes performance issues or CSS validation errors, revert to the simple static "Preparing next questions..." text.

## Open Questions
- None.
