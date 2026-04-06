# Planner SolidStart Phase 38.3 Session Command Desk Ultra-Wide Layout Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md)  
**Related Planning:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md), [Planner SolidStart Phase 37.5 Session Header Signal Consolidation Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-5-session-header-signal-consolidation-spec.md), [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md), [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-solid/src/routes/sessions/session-workspace-screen.tsx`, `planner-solid/src/routes/sessions/session-workspace-controller.ts`, and `planner-solid/src/app.css`

## 1. Purpose

Redesign the session route for ultra-wide displays so a 5K screen becomes a
true command desk instead of a narrow editor floating in unused space.

## 2. Problem

Phase 37 deliberately centered one active-thread work area, which solved the
previous hierarchy problem. On very wide screens that same decision now leaves
too much width unused and hides supporting live context that could coexist
without reintroducing dashboard clutter.

## 3. User Outcome

After this slice:

- the session route uses ultra-wide width for simultaneous comprehension
- thread navigation, active answering, and supporting context each get a clear
  home
- the route still feels calm and operational rather than theatrical
- laptop and narrow widths preserve the same underlying product truth through
  intentional collapses

## 4. Scope

### In Scope

- ultra-wide session-route layout hierarchy
- left-rail, center-canvas, and right-context responsibilities
- responsive fallback behavior for desktop, laptop, and narrow widths
- route-level design constraints for motion, density, and supporting context

### Out Of Scope

- backend prompt transport changes
- planner/adjudication logic
- introducing mock-only layout behavior
- unrelated route redesign

## 5. Design Direction

This slice should follow the repo's denser command-center lineage rather than a
generic dashboard pattern.

Required visual direction:

- calm, dense, premium operational layout
- low-noise depth and separation, not hard borders everywhere
- intentional motion only where it clarifies focus or layout transition
- consistent spacing rhythm and touch-target sanity on collapsed states

Anti-goals:

- oversized cards
- decorative glass layers that compete with content
- equal-weight three-column dashboards with no dominant task surface

## 6. Layout Contract

### 6.1 Ultra-wide structure

The route should resolve into:

- left command rail for thread map, progress, and queued-work visibility
- center answer canvas as the dominant working surface
- right insight rail for contradictions, synthesis, build-readiness, and other
  supporting live context

### 6.2 Width behavior

- the current `1180px`-style route ceiling must be replaced or relaxed for
  ultra-wide breakpoints
- the center canvas must remain dominant even when side rails are present
- secondary rails must not become peer dashboards louder than the answer flow

### 6.3 Fallback behavior

- standard desktop may collapse the right rail first
- laptop may reduce the route to left rail plus center canvas
- narrow/mobile widths may convert side surfaces into drawers, tabs, or stacked
  disclosures without changing the underlying product contract

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- route-level tests or Playwright proof for the session workspace

## 8. Acceptance Criteria

1. the spec defines a real ultra-wide three-surface hierarchy
2. the center answer canvas remains the dominant task surface
3. responsive fallback behavior is explicit and product-truthful
4. the spec stays aligned with the repo's command-center design system instead
   of generic dashboard patterns

## 9. Verification Plan

- route-level browser proof for ultra-wide and standard desktop states
- responsive checks for laptop and narrow-width continuity
- `git diff --check`

## 10. Rollback / Fallback

If the full three-surface route is too broad in one slice:

- keep the widened width model
- add one bounded right-context rail first
- do not revert to the old narrow centered shell as the fallback claim

## 11. Implementation Outcome

Implemented on 2026-04-02 as the third bounded Phase 38 delivery slice.

Delivered behavior:

- the session workspace now widens into a real three-surface shell on ultra-wide
  viewports instead of staying capped to one narrow center column
- the left command rail remains the local thread-navigation surface
- the center answer canvas remains the dominant task surface
- a new right insight rail exposes live state, current focus, up-next context,
  and build-readiness support without reopening dashboard clutter
- standard desktop widths keep the prior two-surface layout by collapsing the
  right rail first, while narrow layouts keep the existing mobile thread sheet

## 12. Verification Evidence

- `npm --prefix planner-solid test -- --run src/routes/sessions/session-workspace-view.test.ts src/lib/workspace.test.ts src/lib/prompt-bank.test.ts src/lib/mock/store.test.ts`
- `npm --prefix planner-solid run build`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `git diff --check`

## 13. Open Questions

- what exact breakpoint should trigger the full three-surface layout
- which supporting context belongs in the right rail on day one versus later
  follow-ons
