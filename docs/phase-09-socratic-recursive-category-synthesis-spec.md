# Phase 09 Socratic Recursive Category Synthesis Spec

**Status:** Implemented  
**Date:** 2026-03-21  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)  
**Prior Slice:** [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)

## Objective

Advance the Socratic category-driven lobby from the current deterministic
root-group plus leaf-category overlay into a true recursive, server-authored
category tree.

This slice should let users keep drilling down through dynamically generated
subcategories until they reach the exact scoped prompt batch they want to work
on, without flattening the experience back into the current two-level model.

It does **not** redesign the underlying belief-state model, replace the prompt
envelope transport, or move draft review into the category tree.

## User Outcome

After this slice:

- the main category screen can expose nested categories beyond one root and one
  leaf level
- answering prompts inside a category can create deeper follow-on
  subcategories, not just refresh the existing leaf set
- users can follow a long category path as far as the interview state supports
- entering a category can yield either a deeper category list or a scoped prompt
  batch, depending on what is available in that subtree
- returning to the main category screen still recomputes the category tree from
  the latest belief state

The user still does **not** get draft review as a category, client-authored
hierarchy, or concurrent belief-state mutation.

## Implementation Notes

Implemented on 2026-03-21 in the bounded Phase 09 delivery slice.

Execution landed in:

- `planner-core/src/pipeline/steps/socratic/category_planner.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-core/src/pipeline/steps/socratic/mod.rs`
- `planner-tui/src/app.rs`
- `planner-tui/src/pipeline.rs`
- `planner-web/src/components/CategoryNavigator.tsx`
- `planner-web/src/components/__tests__/CategoryNavigator.test.tsx`

Delivered behavior:

- category snapshots now preserve true parent-child relationships across more
  than one root and one leaf level
- `root_category_ids` remain the top-level entry points while active-path
  screens now render the children of the currently selected node
- contradiction branches can recurse through multiple nested category levels
  before reaching a prompt-ready leaf
- engine category entry now resolves the full selected path from the current
  snapshot rather than reconstructing only one parent-child hop
- prompt envelopes emitted from recursive leaves now carry the full breadcrumb
  path

Verification completed:

- `cargo test -p planner-core category_planner -- --nocapture`
- `cargo test -p planner-core recursive_category_entry_emits_nested_prompt_path -- --nocapture`
- `cargo test -p planner-tui tick_socratic_category_state_shows_active_branch_children -- --nocapture`
- `cargo test -p planner-server ws_socratic_io_ -- --nocapture`
- `npm test -- --run src/components/__tests__/CategoryNavigator.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx` in `planner-web/`

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- category hierarchy remains server-authored and deterministic from the current
  interview state
- category IDs remain authoritative only within a specific snapshot revision
- `PromptEnvelope.category_path` is the canonical breadcrumb for scoped prompt
  rendering and may contain more than two entries
- entering a category may return either `category_state` or `prompt`
- users remain in the current category path until they explicitly go back
- draft review remains a later prompt flow and must not appear in the category
  tree

## Scope

### In scope

- recursive category synthesis from remaining contradictions, verification
  needs, discovery gaps, and recently answered content
- arbitrary-depth `active_category_path` support in shared schema and runtime
- prompt planning from any category subtree, not just root-group plus leaf
- web and TUI navigation updates for deeper breadcrumbs and nested category
  entry
- checkpoint and replay support for deep category paths
- focused tests proving recursive category generation and drill-down behavior

### Out of scope

- redesigning prompt-card answer UX
- changing the dimension-based convergence model
- moving draft review into category navigation
- ranking categories by user-configurable preferences
- visual graph exploration of the category tree

## Current-State Evidence

- Phase 08 introduced the category-navigation contract and category snapshot
  checkpointing
- the current planner normalizes visible categories into deterministic root
  groups plus refreshed leaf categories
- the current implementation explicitly records arbitrary recursive category
  synthesis as the next likely follow-on investment

## Requirements

### Recursive category contract

The server must be able to emit a category snapshot where:

- any category node may have children regardless of depth
- `root_category_ids` identify only the top-level entry points
- `active_category_path` may represent a path of any supported depth
- category nodes can appear, disappear, or move across revisions as the belief
  state changes

### Category generation behavior

Category synthesis must remain derived from the underlying interview state:

- unresolved contradictions should continue to surface at higher priority than
  verification or discovery work inside the same subtree
- newly answered material may introduce deeper categories beneath the current
  path
- if a selected category has no prompt-ready items but does have child
  categories, the server should emit deeper `category_state` rather than
  flattening back to the main screen

### Prompt scoping behavior

Prompt planning must operate against the selected subtree:

- prompts emitted from a deep category must carry the full `category_path`
- prompt candidates outside the selected subtree must remain hidden while the
  user stays inside that path
- leaving the path and returning to the main category screen must recompute the
  global tree from the latest state

### Client behavior

Web and TUI clients must remain simple:

- clients render the latest server snapshot and do not invent hierarchy
- clients support breadcrumb-style visibility for deep paths
- clients preserve explicit back navigation semantics rather than auto-jumping
  back to root

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-schemas/src/artifacts/socratic.rs](/home/thetu/planner/planner-schemas/src/artifacts/socratic.rs)
- [planner-core/src/pipeline/steps/socratic/category_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/category_planner.rs)
- [planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs)
- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-server/src/session.rs](/home/thetu/planner/planner-server/src/session.rs)
- [planner-server/src/ws.rs](/home/thetu/planner/planner-server/src/ws.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)
- [planner-tui/src/app.rs](/home/thetu/planner/planner-tui/src/app.rs)
- [planner-tui/src/pipeline.rs](/home/thetu/planner/planner-tui/src/pipeline.rs)
- [planner-web/src/types.ts](/home/thetu/planner/planner-web/src/types.ts)
- [planner-web/src/hooks/useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)
- [planner-web/src/components/CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx)
- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

Implementation should stay bounded to recursive category synthesis and deep
navigation support. If the work starts implying category analytics, graph UI,
or user-authored category management, stop and split that into a later spec.

## Acceptance Criteria

- category snapshots can represent more than two visible levels of hierarchy
- entering a category can return additional nested categories without leaving
  the current path
- prompt envelopes emitted from nested categories carry the full breadcrumb path
- answers inside a category can generate deeper follow-on categories
- returning to the main category screen recomputes the recursive tree from the
  latest interview state
- draft review remains outside the category tree

## Verification Plan

### Shared and core

- serde coverage for deeper category snapshots and prompt breadcrumb paths
- planner-core tests proving recursive category generation beyond one root and
  one leaf layer
- engine tests proving deep category entry can yield deeper `category_state`
  before prompt emission

### Server

- websocket tests proving deep category paths replay correctly after reconnect
- tests proving checkpoint resume restores deep paths and associated prompt
  context

### Web and TUI

- web tests proving breadcrumb rendering and multi-level category navigation
- TUI tests proving repeated `open` and `back` navigation across deep paths

## Rollback And Fallback

- if recursive synthesis becomes unstable, preserve server-side hierarchy
  ownership and temporarily cap maximum visible depth rather than reverting to
  flat batches
- if some branches cannot synthesize meaningful child categories, fall back to
  emitting a scoped prompt batch for that subtree

## Open Questions

None. The slice is ready for bounded implementation.
