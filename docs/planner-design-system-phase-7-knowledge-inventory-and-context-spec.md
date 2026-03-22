# Planner Design System Phase 7 Knowledge Inventory And Context Spec

**Status:** Implemented and verified on 2026-03-22  
**Date:** 2026-03-22  
**Parent:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)  
**Previous Phase:** [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)  
**Source Research:** Stitch-to-Planner design translation report dated 2026-03-22

## Objective

Refine the Knowledge Library into a clearer inventory-and-context workspace that
borrows the best structural ideas from the Stitch archive without copying its
document-library hero tiles or branded editorial framing.

This slice focuses on making knowledge browsing feel faster, calmer, and more
project-aware.

## User Outcome

After this slice:

- project-scoped knowledge reads as a deliberate workspace instead of a single
  long multi-purpose page
- inventory, filters, and detail context are easier to scan and understand
- the route gains stronger hierarchy without drifting into glossy content
  marketing

## In Scope

- route-level composition and section hierarchy in
  [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
- shared knowledge filter and inventory support where needed, including
  [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx),
  [NodeListPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeListPanel.tsx),
  and
  [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)
- token and layout primitives in
  [index.css](/home/thetu/planner/planner-web/src/index.css)

## Out Of Scope

- taxonomy or data-model changes
- new filter semantics or backend search work
- Blueprint graph redesign or broad drawer-system changes beyond local
  knowledge-detail inheritance
- literal recreation of featured-document hero tiles, fake asset counters, or
  archive-branded content framing

## Current-State Summary

The Knowledge Library is already functionally rich and project-first, but the
visual hierarchy remains broad and busy:

- project summaries, filter controls, inventory, and detail context compete for
  attention
- the route exposes deep scope and lifecycle semantics, but the page rhythm can
  feel uniformly weighted
- the current detail context is useful, yet the overall route still lacks a
  cleaner primary-versus-secondary split

## Proposed Behavior

### Inventory-first route framing

- make the inventory or current project overview the clear primary anchor
- subordinate secondary summaries so the route reads as a workspace, not a
  stacked report

### Filter treatment

- keep the existing horizontal filter direction, but tighten its grouping and
  visual hierarchy
- emphasize active filters and current scope while reducing generic control
  chrome

### Detail context

- make detail context feel like a purposeful secondary pane or attached context
  layer
- selected-item state, project context, and scope metadata should be easier to
  scan at a glance

### Project-scoped knowledge rhythm

- preserve project-first navigation and overview sections
- stronger section hierarchy should clarify:
  - current project context
  - inventory or list focus
  - detail or preview context
  - secondary quality or activity context

## Implementation Constraints

- preserve the current project-first truth model and deep-link behavior
- do not flatten the route into a generic file manager
- do not use decorative glass on the base page
- keep dense labels and taxonomy language readable and stable
- preserve mobile responsiveness for filter controls and detail access

## Touched Surfaces

Expected primary files:

- [index.css](/home/thetu/planner/planner-web/src/index.css)
- [KnowledgeLibraryPage.tsx](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
- [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx)
- [NodeListPanel.tsx](/home/thetu/planner/planner-web/src/components/NodeListPanel.tsx)
- [DetailDrawer.tsx](/home/thetu/planner/planner-web/src/components/DetailDrawer.tsx)

Expected supporting files, only if needed:

- any knowledge-specific list or summary subcomponents used directly by the
  route

## Acceptance Criteria

- the route has one clear primary inventory or overview anchor
- active filters, selected items, and current project context are easier to
  distinguish
- detail context feels intentionally attached to the main inventory flow
- the resulting page is calmer and more directed without losing dense knowledge
  semantics
- the route does not resemble the Stitch archive's featured-document layout
  literally

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - [KnowledgeLibraryPage](/home/thetu/planner/planner-web/src/pages/KnowledgeLibraryPage.tsx)
  - [KnowledgeFilterBar.tsx](/home/thetu/planner/planner-web/src/components/KnowledgeFilterBar.tsx)
  - supporting knowledge panels if their structure changes materially
- run `npx tsc --noEmit`

### Manual

- verify project-scoped and all-project knowledge states in both themes
- verify dense filter selections remain legible and operable
- verify selected-node context remains obvious on large and smaller widths
- verify the route still feels like Planner knowledge, not a borrowed content
  portal

## Rollback And Fallback

- if a split-pane treatment harms smaller-width usability, keep the hierarchy
  improvements and collapse back to a stronger single-column rhythm
- if one knowledge subsection becomes ambiguous, localize the hierarchy change
  to the inventory and selection surfaces first
- if filter density suffers, simplify control grouping before reintroducing
  heavier chrome

## Open Questions

None blocking readiness.

The route boundaries, non-goals, and touched surfaces are concrete enough for a
bounded implementation slice.
