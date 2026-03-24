# Socratic Session Page Redesign Spec P2 S2: Category Visuals

**Status:** Ready for implementation
**Date:** 2026-03-22
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)

## Problem & Intent
The new central Question Map uses a standard React rendering cycle to snap between active and inactive states. This creates a jarring spatial change when a category expands and pushes others down. Following the `ui-ux-pro-max-skill` constraints, we must use Framer Motion layout animations to make category transitions smooth. The active category should fluidly expand its height, and sibling categories should subtly collapse or recede (opacity/scale changes) rather than just instantly disappearing or jumping.

## User Outcome
When a user clicks an inactive category card, the card smoothly slides open to reveal the nested questions. The layout gracefully shifts downwards, feeling natural, highly responsive, and spatial.

## Scope Boundaries
**In Scope:**
- `planner-web/src/components/SocraticWorkspace.tsx`: Wrap the category list and individual cards in `motion.div` from `framer-motion`.
- Apply `layout` props to ensure smooth bounding-box transitions.
- Apply `AnimatePresence` for the appearing/disappearing `PromptBatchPanel` inside the card.
- Sibling cards should have slight visual recession (e.g., lower opacity or smaller font) when not active, transitioning smoothly.

**Out of Scope:**
- Adding Framer Motion to other pages or components beyond the `SocraticWorkspace.tsx` map.
- Rewriting `PromptBatchPanel` itself to use internal animations.

## Contracts and Touched Surfaces
- `planner-web/src/components/SocraticWorkspace.tsx`
- Note: `framer-motion` is likely already in `package.json` since `ui-ux-pro-max-skill` was referenced previously, but we must verify its presence. (If not, we can fall back to standard CSS transitions).

## Acceptance Criteria
- [ ] Clicking a category expands it with a smooth easing animation.
- [ ] Siblings transition smoothly to their collapsed states without snapping.
- [ ] The prompt panel fades/slides in via `AnimatePresence` when a category becomes active.
- [ ] Performance remains high (using `layout` prop effectively without breaking nested scroll containers).

## Verification Plan
- `vitest` suite passes.
- Inspect `package.json` for `framer-motion`. If not present, install it, or fallback to CSS `grid-template-rows` transitions if adding dependencies is blocked.

## Rollback or Fallback
If `framer-motion` causes layout bugs with the complex `PromptBatchPanel`, we will fall back to a CSS `transition: height 0.3s` trick or `max-height` hack, or simply rely on the existing opacity transitions.

## Open Questions
- Is `framer-motion` already installed in the repo?
