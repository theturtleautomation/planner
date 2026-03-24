# Socratic Session Page Redesign Spec P2 S1: Question Map

**Status:** Ready for implementation
**Date:** 2026-03-22
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)

## Problem & Intent
In `SocraticWorkspace.tsx`, the Question Map is still rendered as a legacy 320px left sidebar alongside a right "Focused Question" canvas. This violates the primary redesign goal: the question map *is* the workspace. The intent is to rebuild `SocraticWorkspace.tsx` so the Question Map becomes a central vertical stack of large, prominent "Bento-style" category cards. The active question prompt will live inside the active category card rather than in a separate right-hand pane.

## User Outcome
When a user enters the lobby, they see a beautiful, centralized list of categories. The active category is structurally dominant, clearly presenting its questions in the center of the screen, completely eliminating the "side navigation vs. main content" duality.

## Scope Boundaries
**In Scope:**
- `planner-web/src/components/SocraticWorkspace.tsx`: Removing the `<section>` that splits into `<aside>` and `<div>`.
- Wrapping the categories (`snapshot.nodes`) in a central flex column.
- Rendering each category as a card.
- The `PromptBatchPanel` will be nested *inside* the card of the currently active category (or rendered directly below it if nesting breaks too many styles).

**Out of Scope:**
- Framer Motion animation logic for expanding/collapsing (handled in P2 S2).
- Dynamic transition states like "preparing" (handled in P2 S3).
- Deep changes to `PromptBatchPanel` itself.

## Contracts and Touched Surfaces
- `planner-web/src/components/SocraticWorkspace.tsx`: The primary restructuring target. The exported component API remains unchanged.

## Acceptance Criteria
- [ ] `SocraticWorkspace.tsx` no longer uses a left sidebar and right canvas layout.
- [ ] Categories are rendered as a single vertical list/grid in the center.
- [ ] The active category (matching `workspace.focused_category_id`) displays the `PromptBatchPanel`.
- [ ] Inactive categories display their title, status, and question count as a compact row.
- [ ] Clicking an inactive category triggers `onFocusCategory`.
- [ ] The "Show All" or Map toggle button is no longer necessary as the map is always the primary view.

## Verification Plan
- `vitest` suite passes.
- Validate that the component successfully renders the active prompt inside the correct category card.

## Rollback or Fallback
If embedding the prompt inside the category card breaks layout constraints of `PromptBatchPanel`, render the categories as a top-level map, and render the active prompt directly below the map in the same single column.

## Open Questions
- What happens if there is no `snapshot` yet? It should fall back to a minimal loading/empty state (covered by P3 S1).
