# Phase 08 Socratic Category Drill-Down Implementation

**Status:** Implemented  
**Date:** 2026-03-21

## Objective

Replace the flat default Socratic lobby prompt batch with a category-driven
intake flow that lets the user:

- start from a dynamic category list
- enter one category at a time
- answer scoped prompts within that category
- go back to the main category screen explicitly
- receive a refreshed category list after answers are applied

This phase keeps the existing hidden truth model intact. Requirement
convergence, contradiction handling, verification, and draft review still run
on top of the dimension-based `RequirementsBeliefState`.

## Delivered Behavior

The implemented interaction model now works as follows:

- the server synthesizes a `SocraticCategorySnapshot` from the current prompt
  candidates before asking the next interview question
- the client receives a `category_state` message and renders a category list
  instead of assuming that intake always starts with a visible prompt batch
- entering a category yields either:
  - a scoped prompt batch for that category subtree, or
  - another category-state refresh if that scope now contains only deeper
    category choices
- users stay inside the chosen category until they explicitly return to the
  main category screen
- returning to the main category screen recomputes the category snapshot from
  the latest belief state and remaining prompt candidates
- `done` and build/start actions remain valid only on the main category screen
  when the underlying interview state is build-ready
- draft review remains a later prompt kind and does not appear as a category

## Shared Schema And Runtime Changes

Shared Socratic schema now includes:

- `SocraticCategorySnapshot`
- `SocraticCategoryNode`
- `SocraticCategoryStatus`
- `SocraticCategoryPathEntry`

`PromptEnvelope` now carries scoped navigation context:

- `origin_category_id`
- `category_path`

The runtime protocol now includes:

- server message `category_state { snapshot }`
- client message `enter_category { category_id, revision }`
- client message `back_to_categories`

Checkpoint persistence now stores both:

- the latest `current_prompt`
- the latest `current_category_snapshot`

Reconnect and resume replay whichever interview state is active.

## Backend Implementation Snapshot

`planner-core` now has a dedicated `category_planner` ahead of prompt emission.
It derives the visible category tree from the current unresolved prompt
candidates and their priority class.

The current implementation is intentionally bounded:

- categories are server-generated and deterministic
- the visible hierarchy is currently a root-group plus leaf-category overlay
  rather than arbitrary recursive depth
- answers can still refresh the visible category list and produce new leaf
  choices after each adjudication cycle

Prompt planning still preserves the original priority rules inside a chosen
category:

- contradiction work first
- verification work second
- discovery work third

## Client Surfaces

`planner-web` now renders:

- a main category navigator during interview mode
- scoped prompt batches within a chosen category
- explicit back navigation to the main category screen

`planner-tui` now supports:

- selecting categories by number
- `open <n>` for explicit category entry
- `back` to return to the main category list
- `done` or `build` only from the main category screen

## Validation Snapshot

The following targeted checks were run successfully for this phase:

- `cargo test -p planner-schemas -p planner-core -p planner-server -p planner-tui --no-run`
- `cargo test -p planner-core category_planner -- --nocapture`
- `cargo test -p planner-core socratic_engine -- --nocapture`
- `cargo test -p planner-server ws_socratic_io_ -- --nocapture`
- `npx tsc --noEmit` in `planner-web/`
- `npm test -- --run src/hooks/__tests__/useSocraticWebSocket.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx` in `planner-web/`

## Known Limits

This implementation delivers dynamic category drill-down, but it does not yet
fully realize the open-ended "rabbit hole forever" model as an arbitrary-depth
recursive taxonomy.

Current bounds:

- category generation is dynamic but currently normalized into deterministic
  root groups plus refreshed leaf categories
- clients do not invent hierarchy; they render the latest server snapshot only
- deeper recursive category synthesis should be treated as a separate follow-on
  planning decision if we want to push beyond this bounded implementation

## Follow-On Guidance

If we decide to keep investing in this area, the next bounded spec should focus
on true recursive category synthesis and navigation rather than reopening the
prompt-envelope protocol itself.

Current follow-on specs:

- [Phase 09 Socratic Recursive Category Synthesis Spec](/home/thetu/planner/docs/phase-09-socratic-recursive-category-synthesis-spec.md) now implemented
- [Phase 10 Socratic Category Status And Refresh Spec](/home/thetu/planner/docs/phase-10-socratic-category-status-and-refresh-spec.md)
- [Phase 11 Socratic Category Replay And Validation Spec](/home/thetu/planner/docs/phase-11-socratic-category-replay-and-validation-spec.md)
