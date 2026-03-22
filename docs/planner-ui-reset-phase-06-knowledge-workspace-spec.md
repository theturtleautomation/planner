# Planner UI Reset Phase 06 Knowledge Workspace Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Planner Design System Phase 7 Knowledge Inventory And Context Spec](/home/thetu/planner/docs/planner-design-system-phase-7-knowledge-inventory-and-context-spec.md), [Planner UI Reset Phase 03 Project Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-03-project-workspace-spec.md), [Planner UI Reset Phase 07 Blueprint Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md)  
**Source Research:** [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx), [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx), [NodeListPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeListPanel.tsx), [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx), and external research on inventory workspaces, filter disclosure, and detail-context hierarchy from Nielsen Norman Group, Carbon, Fluent, and Material

## Objective

Reset Knowledge into an inventory-first workspace where the browsing surface is
clearly dominant and selected-node context behaves like attached detail rather
than a competing page.

The route currently contains genuine power, but too much context can be visible
at the same time.

## User Outcome

After this slice:

- users can browse and filter knowledge without losing the page's main axis
- project scope and inventory state are easier to read
- selected-node detail feels attached and purposeful
- overview, activity, and quality context remain available without crowding the
  inventory

## Design Research Synthesis

- inventory research favors a dominant collection surface with compact filter
  framing and attached detail
- disclosure guidance supports drawers, inspectors, and side panels when the
  main job is still browsing the collection
- recognition-over-recall guidance supports visible scope and filter state so
  users do not have to remember which knowledge slice they are viewing

Planner implication:

- inventory is the page's main truth
- filters should be visible but disciplined
- detail belongs beside or above the inventory flow, not as a peer page

## Locked Decisions

- Knowledge remains project-first and inventory-driven
- the route continues to support all-project and project-scoped views
- selected-node detail remains part of the same workspace
- this slice does not change taxonomy, search semantics, or knowledge backend
  behavior
- the route should not drift into a generic document library or file browser

## Scope

### In scope

- [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
- [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx)
- [NodeListPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeListPanel.tsx)
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- knowledge route hierarchy and layout styles in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

### Out of scope

- knowledge data-model changes
- backend search redesign
- blueprint graph redesign

## Current-State Evidence

- [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
  currently supports multiple project sections:
  `overview`, `inventory`, `architecture`, `quality`, and `activity`
- the route also owns an extensive scoped filter model, favorites, deep links,
  node creation, deletion, and selected-node detail behavior
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  already provides an attached-detail concept, which means the route does not
  need more always-open peer panels
- the route is capable, but the number of visible contexts can make it feel
  like several workspaces combined

## Proposed UI Model

## Route role

Knowledge is the structured inventory workspace for Planner's captured truth.

It answers:

- what knowledge exists
- what slice of it I am looking at
- what the selected item means

## Dominant surface

The dominant surface should be the inventory list or inventory grid equivalent,
not the summary context.

Project overview framing may appear above it, but inventory should remain the
route anchor in both all-project and project-scoped modes.

## Supporting surfaces

Supporting surfaces should be disciplined:

- a compact scope header that explains whether the user is in all-project or a
  specific project slice
- a visible but controlled filter band
- an attached detail drawer or inspector for the selected node
- overview, quality, architecture, and activity content either as explicit
  modes or as secondary sections that do not outrank inventory

## Reveal model

- node detail should remain attached through a drawer or inspector pattern
- lower-frequency summaries should move behind explicit tabs, sections, or
  reveals when they do not help immediate browsing
- filter density may remain high, but inactive filter detail should collapse
  cleanly on smaller widths

## State model

The route should explicitly support:

- all-project inventory
- project-scoped inventory
- filtered results
- no results after filtering
- selected-node detail open
- no node selected
- empty project slice
- loading and error states

## Design-System-Patterns Lens

- semantic surfaces:
  one primary inventory surface, one secondary filter-and-scope surface, one
  attached detail surface
- reveal discipline:
  detail and lower-frequency summaries should be attached or mode-based rather
  than always visible peers
- token hierarchy:
  use tonal distinction to separate inventory, filter framing, and selected
  detail without introducing generic file-manager chrome

## Contracts And Touched Surfaces

- [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
  remains the route owner
- current deep-link and filter semantics remain unchanged
- node create, update, and delete flows remain within the current route
  contract
- touched surfaces:
  [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
  [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx)
  [NodeListPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeListPanel.tsx)
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- the route reads as an inventory workspace first
- the selected-node detail surface is clearly supporting context
- scope and active filter state remain visible without dominating the page
- overview or activity content no longer competes equally with inventory
- all-project and project-scoped modes preserve the same basic hierarchy

## Verification Plan

- targeted frontend tests for the touched knowledge route components
- `npx tsc --noEmit`
- manual verification for:
  - all-project mode
  - project-scoped mode
  - heavy filter combinations
  - selected-node detail open and closed
  - empty or no-match states

## Rollback And Fallback

- if a stronger attached-detail layout is too aggressive on smaller widths,
  preserve the inventory-first hierarchy and fall back to stacked detail below
  the list before restoring multiple peer panels
- if overview content still needs stronger presence, keep it compact and scoped
  above inventory rather than as a separate large module

## Open Questions

None blocking readiness.

## Implementation Notes

- Implemented the first bounded hierarchy reset in
  [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
  by making project-scoped Knowledge default to `Inventory` instead of
  `Overview`, promoting a compact inventory summary band above the section tabs,
  and suppressing the node list and attached detail drawer when the user
  explicitly switches into overview mode.
- Preserved existing filter, activity, quality, and detail contracts while
  making the inventory list the route's default anchor and relegating
  higher-level summaries to supporting sections instead of the starting page
  posture.
- Verification completed with:
  `npm test -- src/pages/__tests__/KnowledgeLibraryPage.test.tsx`
  and `npx tsc --noEmit`.
- Verification was refreshed in the tranche audit remediation slice with an
  explicit assertion that project-scoped Knowledge still opens inventory-first
  and suppresses the inventory list when the user switches into overview mode.
