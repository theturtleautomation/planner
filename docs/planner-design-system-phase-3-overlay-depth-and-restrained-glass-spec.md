# Planner Design System Phase 3 Overlay Depth And Restrained Glass Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md)  
**Source Research:** user-provided design-system analysis dated 2026-03-21

## Objective

Build the third bounded slice of the Planner visual-system refresh by improving
the depth treatment of transient overlay surfaces:

- replace the remaining flat or legacy shadow treatment on shared modals and
  drawers with calmer ambient elevation
- add restrained translucency and backdrop blur only where the surface is
  already transient and floating
- tighten overlay chrome so modals and drawers feel related to the command
  center system without spreading glass treatment across base pages

This phase is about floating surfaces only. It should not become a broad page
restyle or a graph-canvas redesign.

## User Outcome

After this slice:

- modals and drawers feel visually separated from the base layout without
  becoming heavy or muddy
- the UI gains a more tactile sense of depth on create/edit/review flows
- floating overlays feel consistent across project, blueprint, and session
  support flows
- graph-heavy and data-heavy base screens remain stable because blur and glass
  stay confined to small, transient layers

## In Scope

- shared drawer and modal token updates in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- the floating blueprint detail drawer in
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- modal flows that already rely on the shared overlay system, including:
  [CreateProjectModal.tsx](/home/thetu/planner/planner-web/src/components/CreateProjectModal.tsx),
  [ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx),
  [CreateNodeModal.tsx](/home/thetu/planner/planner-web/src/components/CreateNodeModal.tsx),
  [AddEdgeModal.tsx](/home/thetu/planner/planner-web/src/components/AddEdgeModal.tsx),
  [DeleteNodeDialog.tsx](/home/thetu/planner/planner-web/src/components/DeleteNodeDialog.tsx),
  and
  [ImpactPreviewModal.tsx](/home/thetu/planner/planner-web/src/components/ImpactPreviewModal.tsx)
- overlay close affordance consistency and ambient shadow consistency across the
  in-scope transient surfaces

## Out Of Scope

- applying backdrop blur or glass treatment to base page shells, chat panes,
  category panels, or standard cards
- redesigning the Blueprint graph canvas, graph nodes, or hover tooltip system
- creating a new context-menu architecture when no stable shared context-menu
  primitive exists yet
- broad restyling of inline panels such as
  [NodeDetailPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeDetailPanel.tsx),
  [BeliefStatePanel.tsx](/home/thetu/planner/planner-web/src/components/BeliefStatePanel.tsx),
  or [InterviewProgressPanel.tsx](/home/thetu/planner/planner-web/src/components/InterviewProgressPanel.tsx)
- route, workflow, or information-architecture changes
- introducing animation systems beyond small opacity, blur, and transform polish

## Current-State Summary

Phases 1 and 2 established tonal layering, typography hierarchy, and CTA
clarity, but the overlay system still shows older visual conventions:

- drawer surfaces still use border-led separation and flatter edge treatment
- modal backdrops are functional but not yet calibrated to the new command
  center tone
- some modal surfaces use shared primitives, while others still apply local
  header or spacing conventions that drift from the updated system
- depth is present, but it still reads as generic drop-shadow depth rather than
  intentional ambient elevation

## Proposed Behavior

### Shared floating-surface treatment

- define a shared floating-surface language in
  [index.css](/home/thetu/planner/planner-web/src/index.css) for:
  - backdrops
  - modal containers
  - drawer containers
  - overlay close buttons
- shared floating surfaces should use:
  - soft tinted ambient shadows with larger blur radii than base cards
  - subtle translucency on the overlay surface itself
  - restrained backdrop blur on the layer behind the floating surface, not on
    the base page container
  - ghost-border outlines only where they help separation or focus clarity

### Modal refinement

- in-scope modals should inherit one coherent header/body/footer structure
- modal containers should feel elevated and slightly translucent, but their body
  content must remain fully legible in both themes
- destructive and review-heavy modals should keep semantic emphasis through
  content and CTA hierarchy, not through louder glass effects

### Drawer refinement

- the floating blueprint drawer should feel like the highest local layer in the
  current screen
- it should gain ambient elevation and calmer separation from the underlying
  canvas without relying on a hard edge line as the main visual divider
- drawer tabs, relation rows, and inline action pills should remain primarily
  tonal and readable; the spec is about the drawer container and chrome first,
  not a full re-skin of every drawer subsection

## Implementation Constraints

- keep blur restrained and only on transient overlays with a bounded footprint
- do not apply backdrop blur over the full Blueprint canvas in a way that risks
  scroll or interaction jank
- preserve keyboard escape behavior, click-outside behavior, and existing modal
  semantics
- preserve light/dark parity
- do not introduce new font families or expand typography work beyond overlay
  headers inheriting the Phase 2 hierarchy
- prefer shared token and class improvements over one-off inline visual patches,
  unless a component has a justified local exception

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- [CreateProjectModal.tsx](/home/thetu/planner/planner-web/src/components/CreateProjectModal.tsx)
- [ImportProjectModal.tsx](/home/thetu/planner/planner-web/src/components/ImportProjectModal.tsx)
- [CreateNodeModal.tsx](/home/thetu/planner/planner-web/src/components/CreateNodeModal.tsx)
- [AddEdgeModal.tsx](/home/thetu/planner/planner-web/src/components/AddEdgeModal.tsx)
- [DeleteNodeDialog.tsx](/home/thetu/planner/planner-web/src/components/DeleteNodeDialog.tsx)
- [ImpactPreviewModal.tsx](/home/thetu/planner/planner-web/src/components/ImpactPreviewModal.tsx)

Expected supporting files, only if needed:

- [BlueprintPage.tsx](/home/thetu/planner/planner-web/src/pages/BlueprintPage.tsx)
  only if the floating drawer trigger or overlay layering needs a bounded
  z-index adjustment
- component tests covering the in-scope overlay surfaces

## Acceptance Criteria

- shared modal and drawer surfaces read as elevated floating layers rather than
  flat containers pasted on top of the page
- in-scope overlay surfaces gain restrained translucency and ambient depth
  without compromising readability
- the blueprint detail drawer no longer relies primarily on a hard border to
  separate itself from the canvas
- modal headers, close affordances, and backdrop treatment are visually
  consistent across the in-scope flows
- the resulting treatment remains calm and authoritative rather than flashy,
  neon, or novelty-driven
- graph-heavy and standard base surfaces remain unchanged outside the floating
  overlay layer

## Verification Plan

### Automated

- update or add targeted frontend tests where class names, headings, or overlay
  structure materially change
- run the web test targets that cover:
  - modal flows on projects and blueprint-related create/edit/delete paths
  - [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  - [ImpactPreviewModal.tsx](/home/thetu/planner/planner-web/src/components/ImpactPreviewModal.tsx)
- run `npx tsc --noEmit`

### Manual

- verify overlay readability in both themes
- verify backdrop blur and translucency remain subtle over the Blueprint view
- verify open/close transitions still feel responsive and do not smear text
- verify drawer and modal stacking order behaves correctly when launched from
  blueprint interactions
- verify the overlays still feel clearly separate from the base app shell on
  desktop and mobile widths

## Rollback And Fallback

- if blur causes visible jank over blueprint or other heavy surfaces, keep the
  ambient shadow and translucency changes but remove backdrop blur from the
  affected primitive
- if translucency harms text contrast in one theme, increase surface opacity
  before rolling back the broader overlay treatment
- if one modal family cannot cleanly adopt the shared overlay treatment, keep
  the shared token work and localize the exception in that component

## Open Questions

None blocking this phase.

The next deferred work after this slice would be any later context-menu or
popover-specific visual pass, but that should not be bundled into this phase
unless a stable shared primitive is introduced first.

## Delivery Summary

Implemented on 2026-03-22 with the following bounded outcomes:

- upgraded shared floating-surface tokens in
  [index.css](/home/thetu/planner/planner-web/src/index.css) so drawers and
  modals now use calmer ambient elevation, restrained translucency, and
  backdrop blur only on transient overlay layers
- refined the floating blueprint drawer chrome in
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  without broadening the restyle into the base blueprint canvas
- normalized the older blueprint-related modal structures in
  [AddEdgeModal.tsx](/home/thetu/planner/planner-web/src/components/AddEdgeModal.tsx),
  [DeleteNodeDialog.tsx](/home/thetu/planner/planner-web/src/components/DeleteNodeDialog.tsx),
  [CreateNodeModal.tsx](/home/thetu/planner/planner-web/src/components/CreateNodeModal.tsx),
  and [ImpactPreviewModal.tsx](/home/thetu/planner/planner-web/src/components/ImpactPreviewModal.tsx)
  so they now inherit the same modal framing as the project-scope overlays
- added direct component verification for the normalized overlay surfaces in
  [AddEdgeModal.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/AddEdgeModal.test.tsx),
  [DeleteNodeDialog.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/DeleteNodeDialog.test.tsx),
  and
  [ImpactPreviewModal.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/ImpactPreviewModal.test.tsx)

## Delivery Verification

Automated verification completed on 2026-03-22:

- `npm test -- --run src/pages/__tests__/ProjectsPage.test.tsx src/components/__tests__/DetailDrawerOverride.test.tsx src/components/__tests__/AddEdgeModal.test.tsx src/components/__tests__/ImpactPreviewModal.test.tsx src/components/__tests__/DeleteNodeDialog.test.tsx`
- `npx tsc --noEmit`

Manual visual verification was not run in this delivery pass.
