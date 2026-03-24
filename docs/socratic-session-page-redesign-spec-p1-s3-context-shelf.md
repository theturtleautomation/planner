# Socratic Session Page Redesign Spec P1 S3: Context Shelf

**Status:** Ready for implementation
**Date:** 2026-03-22
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)

## Problem & Intent
The Context Shelf currently lives as a simple drawer with standard grey background and a segmented control. To align with the new ultra-premium, focused UX (leveraging `ui-ux-pro-max-skill`), the Context Shelf should be refactored into a sleek, glassmorphic side sheet that clearly differentiates itself from the main workspace while maintaining read clarity. The tabs (Belief State, Draft, Transcript, Events) should be styled as high-fidelity segmented controls, and the drawer should gracefully animate in and out using CSS transitions or Framer Motion.

## User Outcome
When the user clicks the "Context" button in the Pulse Bar, a beautiful, translucent drawer slides in from the right. It feels like a secondary layer floating above the main canvas, ensuring they never lose visual context of where they are in the app.

## Scope Boundaries
**In Scope:**
- `SessionPage.tsx`: Enhancing the `contextShelfOpen` wrapper to use `backdrop-filter: blur(20px)` and refined translucent backgrounds (`var(--color-surface)` with opacity overrides).
- Restyling the inner tab controls to look like a modern iOS/macOS segmented control.
- Adjusting the typography and borders of the right panel wrapper.

**Out of Scope:**
- Restyling the *internals* of the `BeliefStatePanel`, `SpeculativeDraftView`, or `SessionEventsTable` beyond their outer container constraints.
- Changing `ChatPanel` layout (it just rides along in the Transcript tab).

## Contracts and Touched Surfaces
- `planner-web/src/pages/SessionPage.tsx`: The main `<aside>` rendering the Context Shelf.

## Acceptance Criteria
- [ ] Context Shelf uses `backdrop-filter` to blur the underlying main canvas slightly.
- [ ] The drawer background is slightly translucent (e.g., `rgba(20, 20, 22, 0.85)` or similar mapped variables).
- [ ] The segmented control for tabs (Belief State, Draft, Transcript, Events) uses a pill-shaped indicator style rather than flat buttons.
- [ ] Closing the drawer feels snappy and maintains spatial consistency.

## Verification Plan
- `vitest` suite passes.
- Visual inspection via layout code confirms usage of `backdrop-filter` and appropriate `rgba` values.

## Rollback or Fallback
If `backdrop-filter` causes severe performance issues in complex DOMs, fall back to a solid `var(--color-surface)` but keep the refined tab controls.

## Open Questions
- None.
