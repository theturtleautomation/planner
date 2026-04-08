# Planner SolidStart Phase 37.1 Session Command Rail Narrow-Width And Focus Continuity Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md)  
**Related Planning:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 35.3 Session Workspace Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-3-session-workspace-frontend-mock-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Implementation Update (2026-04-01):** the session route now collapses the command rail into a sheet-style thread selector for all widths below `1024px`, keeps queued-later work subordinate inside that selector, preserves the active thread across viewport changes, and returns focus to the workspace heading after narrow-width thread switches. Frontend-mock proof now covers the collapsed selector behavior at tablet width.

## 1. Purpose

Turn the implemented desktop command-rail session hierarchy into a truthful,
usable narrow-width workspace without reopening the Phase 37 desktop layout
decision.

This is a continuity slice, not a new redesign. The route should still feel
like one session workspace with one active thread and one bank-first runtime
truth, even when the rail can no longer live as a permanent desktop column.

## 2. Problem

Phase 37 fixed the desktop hierarchy, but it intentionally did not close the
mobile and narrow-width interaction contract.

What is still undefined:

- how the command rail behaves when there is no room for a sticky left column
- how users switch threads on narrow widths without losing active-question
  context
- where queued-later work lives when the rail collapses
- whether commit-and-advance, focus, and scroll position remain coherent after
  thread switches, resizes, or disclosure toggles

Without a bounded follow-on, the route risks becoming desktop-truthful and
narrow-width incidental.

## 3. User Outcome

After this phase:

- `/sessions/:sessionId` remains one truthful workspace on desktop and narrow
  widths
- the rail collapses into a narrow-width pattern that still exposes all banked
  threads without a route change or reload
- switching threads preserves clear focus on the active work area
- commit, advance, and return-to-work behavior stay legible after thread
  switches or viewport changes
- queued-later work stays available but subordinate on narrow widths

## 4. Scope

### In Scope

- narrow-width behavior for the session command rail
- focus and scroll continuity after thread selection
- continuity behavior after commit-and-advance
- continuity behavior across viewport size changes
- accessibility and keyboard behavior for the collapsed rail pattern
- route-level browser proof for narrow-width Builder/frontend-mock review

### Out Of Scope

- changing backend prompt-bank or session transport contracts
- changing route topology
- changing the Phase 37 desktop layout model
- broad visual redesign of the session route

## 5. Contract

- the bank-first runtime truth from Phase 34 remains fixed: all banked threads
  and prompt items are already local at first reveal
- narrow-width treatment must not introduce a second route, a modal-only fork,
  or a separate mock-only session surface
- the route must still have one dominant active-thread work area
- the collapsed rail must still provide direct local access to every banked
  thread without a server wait
- focus should return to the active work area after rail-driven thread changes
  unless the user explicitly keeps focus in the rail
- queued-later work may move behind disclosure, but it must remain clearly
  non-answerable until runtime truth changes

## 6. Product Decision

### 6.1 Narrow-width rail model

Required direction:

- desktop keeps the sticky left rail
- narrow widths collapse the rail into a top-level disclosure, drawer, sheet,
  segmented control, or similarly compact selector surface
- the selector surface must show the active thread clearly and make switching
  explicit
- the selector surface must not become a second heavy workspace competing with
  the active thread

### 6.2 Focus continuity

Required behavior:

- after selecting a different thread, the route should place the user back in
  the active-thread work area predictably
- after commit-and-advance within the same thread, focus should remain in the
  answering flow
- after a viewport resize from desktop to narrow or back, the current active
  thread must remain selected and legible

### 6.3 Scroll continuity

Required behavior:

- switching threads should not strand the user at a stale scroll position from
  the prior thread
- opening or closing the narrow-width rail should not lose the user's place in
  the active work area
- the route should avoid large accidental scroll jumps when the rail toggles

### 6.4 Accessibility

Required behavior:

- the collapsed rail must be keyboard reachable
- the active thread state must be programmatically clear
- disclosure open/close state must be exposed accessibly
- focus order must stay truthful when the rail opens and closes

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts`
- optional targeted session-route view tests if helper extraction is needed

## 8. Acceptance Criteria

1. the session route keeps the Phase 37 desktop command rail intact while also
   providing a truthful narrow-width rail pattern
2. every banked thread remains directly reachable on narrow widths without a
   route change or new fetch
3. switching threads on narrow widths keeps the active-thread work area as the
   primary focal surface
4. the current active thread remains selected across viewport-size changes
5. focus and scroll behavior remain coherent after thread switches and
   commit-and-advance interactions
6. queued-later work remains available but subordinate on narrow widths
7. the Builder-facing frontend-mock runtime proves the same route behavior
   rather than introducing a narrow-width mock-only shortcut

## 9. Verification Plan

- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
- targeted narrow-width browser proof that:
  - the rail collapses into the selected narrow-width control
  - thread switching works locally
  - focus returns to the active work area appropriately
  - the active thread survives viewport changes
  - queued-later work remains present but subordinate

## 10. Rollback / Fallback

If the preferred narrow-width disclosure pattern proves too broad for one
delivery slice:

- keep the desktop rail unchanged
- ship a simpler narrow-width selector that still preserves truthful local
  thread switching and focus continuity
- do not fall back to re-expanding all threads into a long stacked page just
  because the rail is compressed

## 11. Open Questions

None block readiness.

The implementation choice is still open between a drawer-like disclosure and a
lighter inline selector, but both are valid as long as the contract above is
kept.
