# Socratic Session Page Redesign Spec P3 S1: Canvas States

**Status:** superseded planning artifact
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

> Planning note (2026-03-24): this document is a historical early-slice
> planning artifact from the pre-consultant-desk / pre-master-detail era of
> the Socratic lobby redesign. It no longer defines an active or future-state
> implementation target. Keep it only as historical design/planning context.

## Problem & Intent
The Socratic workspace has empty states (when no snapshot or groups exist) and completion states (when `build_ready` is true). Currently, the completion state just renders a small text block: "No active question groups remain. Build can start from this focused lobby." We want to style the "Build Ready" state as a massive, unmistakable primary action (a Hero state) to give closure to the session loop.

## User Outcome
When a user finishes answering all questions and the session reaches `build_ready`, the UI gracefully empties and presents a large, satisfying "Start Building" success block, celebrating the transition to the next phase of the project rather than feeling like a dead end.

## Scope Boundaries
**In Scope:**
- `planner-web/src/components/SocraticWorkspace.tsx`: Updating the empty state blocks at the bottom of the map list.
- Creating a visually distinct "Build Ready" hero banner with a primary button that triggers `onDone`.
- Styling the true "Empty" state (when `groups.length === 0` but *not* build ready) as a subdued loading state.

**Out of Scope:**
- Altering `SessionPage.tsx` logic.
- Rewriting `build_readiness_message` contents (these come from the server).

## Contracts and Touched Surfaces
- `planner-web/src/components/SocraticWorkspace.tsx`: Specifically the logic where `workspace.groups.length === 0`.

## Acceptance Criteria
- [ ] If `workspace.groups.length === 0` and `build_ready` is true, display a large success hero state with an explicit "Start building" call to action.
- [ ] The "Start building" button should be large, clear, and primary (e.g., green/success colored).
- [ ] If `workspace.groups.length === 0` but `build_ready` is false, display a quiet, pulse-loading "Preparing questions" state.

## Verification Plan
- `vitest` suite passes.
- Code inspection of `SocraticWorkspace.tsx`.

## Rollback or Fallback
If the hero state interferes with the layout constraints, revert to standard centered text but retain the explicit action button.

## Open Questions
- None.
