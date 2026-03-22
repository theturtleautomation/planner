# Planner Design System Phase 1 Tonal Foundation Spec

**Status:** Implemented  
**Date:** 2026-03-21  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Source Research:** user-provided design-system analysis dated 2026-03-21

> Alignment update (2026-03-22): residual shell and session-surface inset
> divider lines were removed, and the main command-center page wrappers were
> loosened further to better match the original no-line and macro-spacing
> acceptance criteria without broadening the scope beyond Phase 1 surfaces.

## Objective

Establish the visual foundation of the new Planner design system by replacing
border-heavy structural chrome with tonal layering across the shared shell and
the highest-traffic entry and session surfaces.

This slice is intentionally the low-risk, high-value first pass from the
adoption plan:

- no-line layout sectioning
- four-tier surface foundations
- calmer page zoning through tonal contrast and whitespace

It does not yet introduce the later typography, CTA, or glassmorphism-heavy
work.

## User Outcome

After this slice:

- the app shell feels calmer and less like a spreadsheet grid
- top-level pages read as deliberate page zones instead of boxed widgets
- project and session lists rely more on whitespace and tonal contrast than
  repeated 1px outlines
- the redesign direction is visible without destabilizing graph-heavy or
  overlay-heavy interactions

## In Scope

- token-layer updates for surface tiers, ghost-border support, and softer
  ambient shadow primitives in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- sidebar and shell restyling in
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- tonal restyling of the home hub in
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
- tonal restyling of the project directory in
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- tonal restyling of project session management surfaces in
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- tonal restyling of the session workspace frame and its high-frequency panels
  in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- updates to directly supporting session components where required for the new
  tonal model, such as
  [ChatPanel.tsx](/home/thetu/planner/planner-web/src/components/ChatPanel.tsx),
  [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx),
  [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx),
  [CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx),
  and
  [SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx)

## Out Of Scope

- changing routes, IA, or session behavior
- introducing a new external display font import
- redesigning blueprint graph rendering, node visuals, or graph context menus
- adding blur/glass to general page containers
- sweeping restyles of admin, discovery, event timeline, or knowledge-library
  detail surfaces beyond incidental shell inheritance
- button-gradient, empty-state, and editorial typography work reserved for the
  next phase

## Current-State Summary

The current frontend still reflects the older dark-first shell:

- [index.css](/home/thetu/planner/planner-web/src/index.css) uses
  `border-right`, card borders, and compact legacy shadows as the default
  structure language
- [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
  visually divides the sidebar from the app body with a hard line
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  and the project/session entry pages still lean on bordered cards and dashed
  empty states
- the session workspace and its supporting panels rely on repeated outline
  treatment rather than nested tonal surfaces

## Proposed Behavior

### Tonal shell and surface stack

- define an explicit four-tier surface system for:
  - app canvas
  - base layout planes
  - elevated cards/modules
  - floating surfaces
- remove structural shell borders where tonal separation can carry the layout
- update shadow tokens toward broader, softer ambient depth instead of compact
  black drop shadows

### No-line layout sectioning

- remove the sidebar divider in the main shell
- remove border-heavy top-level page cards where the parent and child surfaces
  can carry separation through tone and padding
- list and directory surfaces should prefer vertical spacing and hover-tonal
  feedback over row dividers

### Ghost-border fallback

- preserve visible focus clarity using a ghost-border or explicit focus ring
  treatment
- persistent visible outlines should be the exception, not the default card
  structure

### Macro-loose page zoning

- increase spacing between major page zones such as:
  - home prompt block versus recent projects
  - project-level controls versus session lists
  - session workspace header versus active interview or pipeline content
- keep inner module density intact for logs, metadata, and dense planning data

## Implementation Constraints

- preserve existing responsiveness on desktop and mobile
- do not regress contrast or focus visibility in either theme
- avoid introducing restyle-only churn into unrelated pages not named above
- if one shared component is used on both in-scope and out-of-scope pages,
  prefer token-driven changes over local forks

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
- [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
- [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
- [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

Expected supporting files, only if needed:

- [ChatPanel.tsx](/home/thetu/planner/planner-web/src/components/ChatPanel.tsx)
- [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx)
- [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
- [CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx)
- [SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx)

## Acceptance Criteria

- the shared shell no longer uses a structural sidebar divider line as the main
  separation device
- top-level in-scope pages visually rely on tonal layers more than bordered
  cards
- in-scope list and directory rows do not depend on repeated row dividers for
  legibility
- session workspace framing reads as nested surfaces rather than stacked
  bordered boxes
- focus visibility remains explicit and accessible after border removal
- light and dark themes both preserve the same tiered surface logic

## Verification Plan

### Automated

- update or add targeted frontend tests for the in-scope pages and components
  whose structure or labels change materially
- run the web test targets that cover:
  - [HomeHubPage](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx)
  - [ProjectsPage](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx)
  - [ProjectSessionsPage](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx)
  - [SessionPage](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

### Manual

- verify the shell, home hub, project directory, project sessions, and session
  workspace in both light and dark mode
- verify keyboard focus remains obvious on navigation items, primary inputs, and
  prompt controls
- verify the session workspace still feels performant after the shadow and
  surface updates

## Rollback And Fallback

- if a surface becomes ambiguous without borders, reintroduce only a low-opacity
  ghost border for that surface instead of reverting the whole tonal approach
- if one page proves too risky, keep the token foundation and shell changes but
  defer that page-local restyle to a follow-on spec

## Open Questions

None blocking this phase.

Typography, CTA gradients, and restrained overlay glass are intentionally
deferred to later slices.

## Delivery Outcome

Phase 1 is now implemented in the bounded web slice.

Delivered changes:

- shared surface tokens and ambient shadow primitives now reflect the command
  center tonal model in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
- the shared shell/sidebar now relies on tonal separation instead of a hard
  structural divider in
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- the home hub, projects directory, project sessions workspace, and session
  workspace now use nested surfaces and spacing over repeated bordered cards in
  [HomeHubPage.tsx](/home/thetu/planner/planner-web/src/pages/HomeHubPage.tsx),
  [ProjectsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectsPage.tsx),
  [ProjectSessionsPage.tsx](/home/thetu/planner/planner-web/src/pages/ProjectSessionsPage.tsx),
  and
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- high-frequency session panels and prompt controls now follow the same tonal
  treatment in
  [ChatPanel.tsx](/home/thetu/planner/planner-web/src/components/ChatPanel.tsx),
  [MessageInput.tsx](/home/thetu/planner/planner-web/src/components/MessageInput.tsx),
  [CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx),
  [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx),
  [PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx),
  and
  [PromptOptionGroup.tsx](/home/thetu/planner/planner-web/src/components/PromptOptionGroup.tsx)

Known bounded follow-up:

- editorial typography hierarchy, CTA emphasis, and empty-state refinement are
  still intentionally deferred to the next design-system phase
- no manual light/dark visual sweep was completed in this delivery pass

## Delivery Verification

- `npm test -- --run src/pages/__tests__/HomeHubPage.test.tsx src/pages/__tests__/ProjectsPage.test.tsx src/pages/__tests__/ProjectSessionsPage.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/Layout.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npx tsc --noEmit`
