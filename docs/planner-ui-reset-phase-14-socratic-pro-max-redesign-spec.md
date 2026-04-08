# Planner UI Reset Phase 14: Socratic Pro Max Redesign Spec

**Status:** deferred
**Date:** 2026-03-22
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

> Planning sync update (2026-03-23): this doc remains deferred as a superseded
> precursor. The selected replacement direction is now captured in
> [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md),
> which is the authoritative split-pane consultant-desk artifact for future
> Socratic Lobby work. Do not promote or implement this doc as written.

## Problem & Intent
The Socratic question lobby is the core of Planner, but currently, the "Question Map" (the architecture of categories) is hidden behind an overlay toggle. The user must manually reveal it, treating the core product mental model as secondary chrome. Furthermore, selection states are unclear and transitions feel disjointed.

We will rebuild the Socratic Workspace so the Question Map is a first-class, permanently visible left sidebar, and the Active Question is the highly elevated dominant right canvas.

## Scope Boundaries
- **In Scope:** 
  - Restructure `SessionPage.tsx` and `SocraticWorkspace.tsx` to a side-by-side layout (Map on Left, Canvas on Right).
  - Implement clear visual selection states for categories in the Map.
  - Move the Context Shelf (Belief, Draft, Events) into an explicitly triggered slide-over Drawer/Modal.
  - Add Tailwind/Framer Motion fluid transitions for category selection and dynamic category appearance.
- **Out of Scope:**
  - Changes to Socratic engine backend or websocket payload.
  - Changes to other pages (Home, Projects).

## Acceptance Criteria
1. The Question Map is permanently visible on desktop (e.g., left sidebar).
2. The selected category is unmistakably highlighted in the map.
3. The Active Question Canvas is cleanly separated from the map and contains no "card soup".
4. The Context Shelf is hidden by default and opens smoothly without shifting the main layout (e.g., an overlay drawer).
5. Dynamic questions animate in calmly.

## Verification Plan
1. `npx tsc --noEmit`
2. Open Socratic session at 1440px -> verify Map is visible on left, canvas on right.
3. Open at 375px -> verify responsive stack (Map collapses or stacks cleanly).
4. Click a category -> verify clear selected state and canvas update.

## Rollback
- Revert changes to `SessionPage.tsx` and `SocraticWorkspace.tsx`.
