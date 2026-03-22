# Planner Design System Phase 2 Editorial Typography And CTA Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 1 Tonal Foundation Spec](/home/thetu/planner/docs/planner-design-system-phase-1-tonal-foundation-spec.md)  
**Source Research:** user-provided design-system analysis dated 2026-03-21

## Objective

Build the second bounded slice of the Planner visual-system refresh by adding:

- a deliberate display-versus-data typography hierarchy
- clearer CTA hierarchy for primary versus secondary actions
- stronger empty-state presentation on the highest-traffic surfaces

This phase should make the product feel more editorial and directed without
opening the broader overlay, modal-glass, or graph-visual redesign work that is
reserved for the next phase.

## User Outcome

After this slice:

- page headers feel more intentional and memorable instead of generic dashboard
  chrome
- primary actions read immediately as the next move on home, project, and
  session entry surfaces
- empty states feel designed rather than placeholder-like
- dense planning data still remains readable because display styling is kept to
  headers, section anchors, and empty-state framing

## In Scope

- display-font token introduction and typography scale updates in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- header and section-title typography updates in
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx),
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx),
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx),
  and
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- primary and secondary CTA restyling for the in-scope pages plus the directly
  related input or modal surfaces that expose first-order actions, including
  [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx),
  [CreateProjectModal.tsx](/home/thetu/planner/planner-web/src/components/CreateProjectModal.tsx),
  and
  [ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx)
- empty-state visual treatment upgrades on the in-scope home, projects,
  project-sessions, and session surfaces

## Out Of Scope

- changing routes, information architecture, or workflow behavior
- graph node styling, blueprint canvas work, or graph context-menu redesign
- restrained glass, blur, or overlay depth work beyond incidental modal polish
- redesigning admin, discovery, events, or knowledge-library detail pages as a
  broad sweep
- replacing the body/data font stack across dense tables, logs, or blueprint
  data
- Phase 3 floating-layer work such as custom ambient overlay systems and
  glassmorphic menus

## Current-State Summary

Phase 1 already established tonal layering and border removal, but the app
still lacks the editorial hierarchy described in the design analysis:

- display headings still use the same utilitarian body font token
- primary buttons are clearer than before, but they do not yet feel premium or
  directional
- empty states are calmer after tonal cleanup, but they do not yet anchor the
  eye with stronger type and action framing

## Proposed Behavior

### Display-versus-data hierarchy

- introduce a dedicated display font token in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- keep the current sans-serif body/data stack for tables, metadata, forms, and
  dense planning content
- apply the display font only to:
  - page-level `h1` and equivalent titles
  - major section headers on the in-scope pages
  - empty-state headlines
- avoid applying the display face to long-form paragraphs, logs, or compact
  metadata rows

### CTA hierarchy

- primary CTAs should become visually distinct through a restrained linear
  gradient, stronger weight, and slightly more prominent elevation
- secondary actions should remain tonal and subdued, not outlined by default
- destructive actions should stay clear but should not overpower the primary
  action path
- CTA hierarchy must remain consistent across:
  - page headers
  - empty states
  - project/session action clusters
  - the waiting-state session brief form

### Empty-state refinement

- empty states should gain:
  - stronger headline typography
  - more deliberate vertical spacing
  - clearer recommended primary action
- empty states should still stay lightweight and avoid illustration-heavy or
  novelty-heavy treatments

## Implementation Constraints

- choose one hosted display font only; do not add multiple new font families
- preserve current layout stability and avoid major header-height regressions
- preserve WCAG-compliant focus visibility and text contrast
- do not broaden button restyling into unrelated pages outside the in-scope
  surfaces unless token-only inheritance makes it automatic
- if a shared CTA token change impacts out-of-scope pages, verify that the
  result remains visually coherent and non-breaking

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

Expected supporting files, only if needed:

- [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx)
- [CreateProjectModal.tsx](/home/thetu/planner/planner-web/src/components/CreateProjectModal.tsx)
- [ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx)

## Acceptance Criteria

- in-scope page titles and major section headers use a distinct display
  hierarchy separate from the dense data/body font
- primary CTAs visually read as the main action through color, weight, and
  restrained gradient treatment
- secondary CTAs remain clearly interactive but are visually subordinate to the
  primary action
- empty states on the in-scope pages feel intentionally designed and direct the
  user toward the next move
- dense metadata, chat copy, and planning data remain in the body/data font and
  are not harmed by the new hierarchy
- light and dark themes preserve the same hierarchy and CTA logic

## Verification Plan

### Automated

- update or add targeted frontend tests where button labels, empty-state copy,
  or accessible names materially change
- run the web test targets that cover:
  - [HomeHubPage](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  - [ProjectsPage](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  - [ProjectSessionsPage](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  - [SessionPage](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- run `npx tsc --noEmit`

### Manual

- verify typography rendering in light and dark themes
- verify the chosen display font does not cause layout clipping on desktop or
  mobile
- verify primary CTAs remain obviously primary in home, projects, project
  sessions, session waiting-state, and related modals
- verify empty states still feel calm and do not become marketing-like or noisy

## Rollback And Fallback

- if the chosen display font causes loading or spacing regressions, keep the
  new hierarchy tokens and fall back to a safer local/system display stack
- if gradient CTAs feel too loud on one surface, reduce gradient contrast on
  that token rather than reverting the full CTA hierarchy work
- if a shared button token change harms out-of-scope pages, localize the CTA
  change to the in-scope surfaces and create a follow-up cleanup spec

## Open Questions

None blocking this phase.

The next deferred slice after this phase remains floating-depth and restrained
glass for overlays only.

## Delivery Summary

Implemented on 2026-03-22 with the following bounded outcomes:

- introduced a single hosted display font and reusable heading and empty-state
  primitives in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- upgraded primary-versus-secondary CTA hierarchy through restrained gradient
  primary actions, calmer secondary actions, and stronger focus treatment in
  the shared button and input tokens
- applied the editorial hierarchy and refined empty-state treatment to
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx),
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx),
  and the waiting-state intake surface in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- tightened directly related modal and input surfaces in
  [CreateProjectModal.tsx](/home/thetu/planner/planner-web/src/components/CreateProjectModal.tsx),
  [ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx),
  and
  [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx)

## Delivery Verification

Automated verification completed on 2026-03-22:

- `npm test -- --run src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/MessageInput.test.tsx src/components/__tests__/Layout.test.tsx`
- `npx tsc --noEmit`

Manual visual verification was not run in this delivery pass.
