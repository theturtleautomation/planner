# Socratic Session Page Redesign Spec P3 S2: Cleanup

**Status:** superseded planning artifact
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

> Planning note (2026-03-24): this document is a historical early-slice
> planning artifact from the pre-consultant-desk / pre-master-detail era of
> the Socratic lobby redesign. It no longer defines an active or future-state
> implementation target. Keep it only as historical design/planning context.

## Problem & Intent
Now that the Socratic lobby redesign is effectively complete (focused canvas, pulse bar, context shelf, bento cards, animations), we need to ensure the layout constraints apply cleanly across both massive 1440px desktop screens and 375px mobile viewports. Additionally, we need to delete any legacy components that are completely orphaned to reduce codebase weight.

## User Outcome
The user sees a beautifully responsive UI that doesn't break or overlap on narrow screens. The codebase is tighter and cleaner.

## Scope Boundaries
**In Scope:**
- `planner-web/src/components/SessionStatusHeader.tsx`: Ensure the flex layout handles wrapping on 375px safely.
- `planner-web/src/components/SocraticWorkspace.tsx`: Ensure horizontal scrolling exists for tight inner panes if needed, though most flex containers should wrap.
- `planner-web/src/pages/SessionPage.tsx`: Ensure `maxWidth` on the central column is enforced without cutting off padding on mobile.
- Remove `ConvergenceBar.tsx` and `PipelineBar.tsx` if they are no longer used anywhere.

**Out of Scope:**
- Redesigning other pages (like Dashboard or Projects).

## Contracts and Touched Surfaces
- `planner-web/src/components/ConvergenceBar.tsx`
- `planner-web/src/components/PipelineBar.tsx`
- Any leftover imports of the above.

## Acceptance Criteria
- [ ] Mobile viewports (<= 400px) do not cause horizontal scrolling outside of the intended card bounds. Flex elements in headers wrap correctly.
- [ ] `ConvergenceBar.tsx` is deleted if completely unused.
- [ ] `PipelineBar.tsx` is deleted if completely unused.

## Verification Plan
- `vitest` suite passes.
- Code grep for `ConvergenceBar` and `PipelineBar`. If zero occurrences, safe to delete.

## Rollback or Fallback
If deleting the components breaks a test or route I missed, I will restore them and leave them deprecated.

## Open Questions
- None.
