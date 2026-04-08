# Socratic Session Page Redesign Spec P1 S2: Focused Canvas

**Status:** superseded planning artifact
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

> Planning note (2026-03-24): this document is a historical early-slice
> planning artifact from the pre-consultant-desk / pre-master-detail era of
> the Socratic lobby redesign. It no longer defines an active or future-state
> implementation target. Keep it only as historical design/planning context.

## Problem & Intent
The current session page forces a "split-pane" layout where the left pane holds category navigation and chat, and the right pane acts as a context/event surface. This dilutes the focus required to answer planning questions. The intent is to remove the split-pane layout completely and move to a centered, single-column constraint for the Socratic lobby, making the canvas itself the primary surface.

## User Outcome
Users interact with a singular, centered canvas that acts as the "Question Map" and prompt entry surface, drastically simplifying the layout and focusing their attention completely on the task at hand. The right context area is removed from the DOM's main flow.

## Scope Boundaries
**In Scope:**
- `SessionPage.tsx`: Removing `<div className="split-pane">`, `<div className="pane-left">`, and `<div className="pane-right">`.
- Wrapping the main Socratic lobby (both the interviewing state and the `showFocusedLobby` state) in a single centralized max-width container.
- Moving the `rightPanelContent` entirely into the Context Shelf drawer (which is opened by the button created in S1).
- Adjusting CSS classes or inline styles to enforce a single column flow.

**Out of Scope:**
- Actually redesigning the inside of `SocraticWorkspace.tsx` to display category cards (this is P2 S1).
- Changing the interior contents of the Context Shelf (P1 S3).

## Contracts and Touched Surfaces
- `planner-web/src/pages/SessionPage.tsx`: Major structural change to JSX wrapping.
- `planner-web/src/index.css` or layout styles: Deprecating or bypassing split-pane specific CSS.

## Acceptance Criteria
- [ ] The `split-pane` structure is removed from `SessionPage.tsx`.
- [ ] The page renders as a single column (centered, max width).
- [ ] The Belief State, Draft, Transcript, and Events are no longer permanently visible on the right half of the screen.
- [ ] They are instead placed into the Context Shelf, or temporarily hidden until the Context Shelf is built properly (if not already handled by SocraticWorkspace's context shelf logic).
- [ ] Tests continue to pass.

## Verification Plan
- Verify via `vitest` that all rendering logic (especially tests that might expect split panes) still functions correctly.
- Verify the layout CSS no longer splits 50/50.

## Rollback or Fallback
If the single-column layout breaks too many downstream components, we will wrap the legacy components in a centered container but keep their internal layouts intact.

## Open Questions
- None. This prepares the page for the unified Map view.
