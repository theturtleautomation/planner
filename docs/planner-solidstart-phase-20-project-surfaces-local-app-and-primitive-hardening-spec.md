# Planner SolidStart Phase 20 Project Surfaces Local-App And Primitive Hardening Spec

**Status:** implemented  
**Date:** 2026-03-25  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 03 Project Review And Build Readiness Spec](/home/thetu/planner/docs/planner-solidstart-phase-03-project-review-and-build-readiness-spec.md), [Planner SolidStart Phase 14 Project Import Review Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-14-project-import-review-route-spec.md), [Planner SolidStart Phase 19 Typography, Alignment, And Visual Consistency Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-19-typography-alignment-and-visual-consistency-remediation-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Audit:** user-provided 2026-03-25 executive-grade review of the active `planner-solid` project surfaces plus direct repo inspection of `planner-solid`

## 1. Executive Judgment

The next SolidStart slice should not widen route coverage again.

The active problem is now inside the already-migrated project surfaces:

- `/projects` still breaks the local-app illusion with a native
  `window.confirm`
- `/projects/:projectSlug` still behaves like a giant route-owned HTML/CSS
  composition instead of a bounded workspace made from reusable surfaces
- attached project tabs are manually wired, manually annotated for ARIA, and
  do not survive reload, back/forward, or direct linking
- the route-family visual cleanup from Phase 19 improved hierarchy, but it did
  not yet establish a durable primitive layer, a strict surface-depth model,
  or scoped styling ownership for the highest-traffic project flows

This slice should therefore harden the two core project surfaces so they feel
like a local app and become maintainable enough for later follow-on work.

## 2. User Outcome

After Phase 20:

- deleting a project happens inside a Planner-owned confirmation overlay rather
  than a browser prompt
- attached project surfaces open deterministically from the URL and remain
  stable across reload and sharing
- project-workspace interaction logic is owned by reusable components and
  primitives instead of one 995-line route file
- repeated cards, badges, and attached-tab chrome become shared project-surface
  building blocks
- project surfaces use one explicit depth hierarchy instead of each panel
  reinventing its own border and background treatment
- new project-surface components own their structure in co-located CSS instead
  of extending the global stylesheet indefinitely
- the project directory and project workspace keep their current density while
  feeling more deliberate, accessible, and app-owned

## 3. Problems To Solve

### 3.1 Browser-native destructive flow

`planner-solid/src/routes/projects/index.tsx` still uses `window.confirm(...)`
before deletion.

This is the most visible break in the intended "local-first workspace" feel:
the browser takes over the interaction, the visual system disappears, and the
destructive action cannot share the same focus, motion, copy, or pending-state
language as the rest of the app.

### 3.2 Project-workspace route monolith

`planner-solid/src/routes/projects/[projectSlug].tsx` is currently 995 lines
long and owns:

- route resources
- mutation handlers
- workspace hero
- session list
- advanced panel disclosure
- eight attached tab surfaces
- repeated summary-card and list-row markup

This makes the route hard to reason about, hard to review safely, and too easy
to regress when a single attached surface changes.

### 3.3 Manual tabs and non-persistent attached-surface state

The project workspace currently renders raw `<button>` elements with
`role="tab"` and `aria-selected={...}` in route markup. The selected tab is
local state only.

That creates three problems:

- accessibility behavior depends on hand-maintained wiring
- reload and back/forward lose the user’s place
- attached surfaces cannot be linked or reopened directly in a stable way

### 3.4 Missing project-surface primitive layer

The active Solid app still lacks a bounded primitive/component layer for the
project surfaces that carry the most user time:

- delete confirmation
- attached tabs
- metric cards
- status badges
- reusable list/object rows inside attached surfaces

Consistency is currently enforced by repeating class names and markup patterns,
not by shared component ownership.

### 3.5 Styling-architecture risk without bounded response

`planner-solid/src/app.css` is now 1499 lines and still owns project-surface
layout, advanced-panel chrome, metric-card styling, and breakpoint behavior.

This slice should respond to that risk by creating reusable component
boundaries and clearer ownership, but it must not explode into a full styling
architecture rewrite.

### 3.6 Surface-depth inconsistency inside the project routes

The audit correctly called out that the active project surfaces still mix
`hero-panel`, `section-panel`, `advanced-panel`, and summary-card treatments
without one strict depth rule.

If this slice only extracts components without fixing that contract, it will
just make the current inconsistency reusable.

### 3.7 Mutation-model drift on the new delete flow

The broader recommendation to move mutations toward SolidStart actions is still
valid even if the app is not ready for a full mutation-model conversion in one
pass.

At minimum, the new project-delete confirm flow should not land as another
manual try/catch island after this spec already rebuilds that interaction.

## 4. Scope

### In Scope

- project directory deletion UX on `/projects`
- project workspace attached-surface behavior on `/projects/:projectSlug`
- bounded reusable primitives/components for project surfaces
- standardizing project-surface depth against a strict three-tier model
- URL-backed attached-tab persistence on the base project workspace route
- replacing the native `<details>` project-surface disclosure with an app-owned
  attached-surface shell controlled by URL-backed state
- splitting the project workspace route into smaller owned surfaces
- co-located CSS Modules for the new project-surface primitives and extracted
  route components
- using a SolidStart action for the new project-delete confirm flow
- keeping the existing dark, dense visual direction while replacing the raw
  interaction plumbing underneath it
- regression-proofing the already-landed `StartClient` hydration fix through
  verification

### Out Of Scope

- broad app-wide migration to a full component library
- full replacement of `planner-solid/src/app.css` with CSS Modules, Tailwind,
  or Vanilla Extract
- migration of discovery, events, admin, knowledge, or blueprint routes onto
  the new primitive layer unless the project surfaces directly require a shared
  file
- backend contract changes unrelated to project deletion or attached-surface
  URL state
- converting every existing route mutation to SolidStart actions in this slice
- reopening the already-fixed duplicate-shell hydration bug as a new product
  thread

## 5. Current-State Evidence

- `planner-solid/src/entry-client.tsx` already uses `StartClient`, so the
  duplicate app-shell/header bug identified in the audit is treated as an
  already-landed fix and a regression target for this slice, not the main
  delivery item.
- `planner-solid/src/routes/projects/index.tsx` still uses `window.confirm(...)`
  and route-local pending/error state for deletion.
- `planner-solid/src/routes/projects/[projectSlug].tsx` is 995 lines long and
  directly renders all attached-surface tabs, summaries, and list rows.
- the same route manually renders tab buttons with `role="tab"` and
  `aria-selected`, with no URL persistence.
- the current project attached-surface shell still uses a native
  `<details>/<summary>` disclosure wrapper rather than an app-owned panel
  controlled by the actual workspace state.
- `planner-solid/src/app.css` still owns `advanced-tab`,
  `advanced-summary-card`, and the current `1080px`, `900px`, and `720px`
  breakpoint behavior from one stylesheet.
- the active project routes still mix multiple panel treatments without one
  explicit base/elevated/overlay depth contract.
- direct repo inspection found no existing `planner-solid/src/components`
  project-surface primitive layer to own these repeated patterns.

## 6. Product And Technical Contract

### 6.1 Local-app destructive-action contract

Project deletion on `/projects` must move to an app-owned confirm flow.

Required behavior:

- replace `window.confirm` with an in-app alert dialog
- keep the existing warning semantics about deleting sessions, owned knowledge,
  and unlinking shared knowledge
- route the delete confirm through a SolidStart action so pending/error state is
  derived from the framework mutation model rather than rebuilt manually for
  the new flow
- preserve escape, focus-trap, cancel, and explicit confirm behavior

Selected dependency direction for this slice:

- adopt `@kobalte/core` for the destructive dialog and attached tabs
- do not adopt SolidUI or Tailwind as part of this slice
- keep the current CSS-variable and density system and wrap the headless
  behavior in app-owned components

### 6.2 Attached-surface state contract

The project workspace attached-surface state must become URL-backed on the base
project route.

Required behavior:

- replace the native `<details>` wrapper with an app-owned attached-surface
  panel and explicit toggle row
- the selected tab is represented by a `tab` search param on
  `/projects/:projectSlug`
- a valid `tab` value opens the attached surface reveal and selects that tab
- removing `tab` closes the attached-surface reveal
- invalid or unsupported `tab` values fall back to `review`
- tab changes update the URL without full-page navigation
- browser back/forward restores the prior attached-surface state

### 6.3 Visual-depth contract

The new project-surface primitives must normalize the existing dark palette into
one explicit three-tier depth model:

- `App Base`
  - route background and non-emphasized workspace canvas
- `Elevated Surface`
  - hero, section, card, and attached-surface containers
- `Overlay`
  - modal or alert-dialog layers only

Required behavior:

- map the existing project-surface backgrounds and borders onto this hierarchy
  instead of preserving separate ad hoc panel treatments
- define the shared depth tokens in `planner-solid/src/app.css` as the single
  source of truth for these three levels
- ensure new metric cards, attached surfaces, and project panels consume the
  normalized depth tokens rather than reintroducing custom one-off fills and
  border recipes
- do not port `hero-panel`, `section-panel`, `advanced-panel`, and card styling
  one-to-one into extracted components without first reconciling them against
  the depth model

### 6.4 Project-surface primitive contract

This slice must introduce a bounded primitive/component layer for the project
surfaces.

Minimum required shared ownership:

- app-owned confirm dialog component
- app-owned attached tabs wrapper
- reusable metric-card or stat-card surface
- reusable status badge surface

Minimum required route decomposition:

- `/projects/:projectSlug` becomes an orchestration route that delegates major
  surface rendering to child components
- attached-surface tab content no longer lives as one unbroken block of route
  JSX
- repeated advanced-surface rows and summary objects should be owned by shared
  project-surface components where their structure is the same

This slice does not need to solve every route in the app. It does need to
establish a truthful pattern for the two project surfaces.

### 6.5 Accessibility contract

Interactive project-surface primitives must stop depending on route-authored
manual ARIA wiring as the primary behavior model.

Required behavior:

- attached tabs inherit keyboard navigation and selected-state semantics from a
  headless primitive
- delete confirmation inherits focus management and dismiss behavior from a
  headless primitive
- the resulting surfaces remain compatible with the current density and custom
  styling direction

### 6.6 Styling contract

This slice must improve ownership without pretending to solve the entire CSS
architecture.

Required behavior:

- shared global tokens in `planner-solid/src/app.css` remain the source of
  variables, depth tokens, typography scales, and route-wide utility classes
  only
- new project-surface primitives and extracted route components must use
  co-located CSS Modules for their component-specific structure and styling
- do not add new component-specific classes for the extracted project-surface
  primitives to the global `app.css` file
- if a new shared token is needed for the project-surface primitive layer, add
  the token globally and consume it locally from CSS Modules
- a broader styling-architecture decision remains a later follow-on if still
  needed after component extraction

### 6.7 Mutation contract

This slice does not convert the entire app to SolidStart actions, but it does
set a stricter boundary than the previous draft.

Required behavior:

- the new project-delete confirm flow must use a SolidStart action
- existing import-review and project-workspace mutations may remain on the
  current model in this slice if changing them would broaden scope materially
- any later mutation conversion should build from this Phase 20 delete-flow
  pattern rather than introducing another manual confirm path

### 6.8 Hydration-regression contract

The already-landed `StartClient` fix remains part of the contract.

Required behavior:

- the app shell and header render once on initial load
- no follow-on refactor in this slice reintroduces duplicate hydration/mounting
  behavior

## 7. Dependencies And Touched Surfaces

Expected primary files:

- `planner-solid/package.json`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/[projectSlug].tsx`
- `planner-solid/src/app.css`

Expected new files:

- bounded project-surface components under `planner-solid/src/components/ui/`
  and/or `planner-solid/src/components/projects/`
- co-located `*.module.css` files for the new project-surface primitives and
  extracted project-route components
- a small route-state helper if needed for valid tab parsing and URL mapping
- a bounded delete-action helper or route-owned action definition for the
  project delete flow
- targeted route/component tests for the new primitives or state helper
- a new Playwright proof file for the project-surface hardening slice

Expected supporting files, only if needed:

- `planner-solid/src/lib/advanced.ts`
- `planner-solid/src/lib/projects.ts`
- existing Phase 01 and workflow-closeout Playwright files if they are the
  right place to absorb one regression assertion rather than creating a new
  isolated spec file

## 8. Acceptance Criteria

This slice is complete only when:

1. `/projects` no longer uses `window.confirm` for project deletion.
2. deleting a project is confirmed inside a Planner-owned alert dialog with
   truthful warning copy, cancel, confirm, pending, and inline error handling.
3. the project-delete flow uses a SolidStart action rather than a new manual
   mutation block.
4. `/projects/:projectSlug` no longer uses a native `<details>` wrapper for the
   attached-surface reveal; the reveal is app-owned and state-driven.
5. `/projects/:projectSlug` attached surfaces are driven by a headless tab
   primitive rather than route-authored manual tab semantics.
6. the selected attached tab persists through the `tab` search param and
   survives reload and browser back/forward.
7. new project-surface primitives normalize to the explicit three-tier depth
   model instead of copying existing panel treatments one-to-one.
8. the project workspace route no longer owns the full repeated attached-tab
   markup as one giant file; major surfaces are extracted into child
   components.
9. reusable project-surface primitives exist for at least confirm dialog,
   attached tabs, metric cards, and status badges.
10. new extracted project-surface components use co-located CSS Modules, with
   `app.css` limited to shared tokens and route-wide utilities.
11. the already-landed `StartClient` hydration fix remains intact and is covered
   by verification.
12. the project directory and project workspace retain their dense dark visual
   direction without broadening this slice into a full-app restyle.

## 9. Verification Plan

### Automated

- targeted component or route-state tests for:
  - attached-tab URL parsing and fallback behavior
  - confirm-dialog open/close and pending-state behavior
  - delete-action pending/error behavior
  - shared project-surface primitives where structure materially changes
- run `npm test` inside `planner-solid`
- run `npm run lint` inside `planner-solid`
- run `npm run build` inside `planner-solid`

### Browser

- open `/projects` and confirm project deletion opens a styled in-app dialog
  instead of a browser prompt
- cancel the delete dialog and confirm the route remains stable
- confirm a delete path shows the correct pending and post-mutation behavior
- verify the extracted project surfaces use the normalized depth hierarchy
  rather than mixed legacy panel chrome
- open `/projects/:projectSlug?tab=readiness` and verify the attached surface
  is already open on `Build readiness`
- switch tabs and verify the `tab` param updates without a full route change
- use browser back/forward and confirm attached-surface state restores
- hard-refresh with an attached-surface URL and confirm the same tab is still
  selected
- verify the app shell/header appears once on first load

## 10. Rollback And Fallback

- if `@kobalte/core` integration exposes an unexpected compatibility problem
  with the current SolidStart stack, the fallback is a small app-owned wrapper
  that preserves the same public component contract, keyboard behavior, and URL
  semantics; the fallback is not a return to `window.confirm` or manual
  ARIA-only tabs.
- if the full styling move is larger than expected, preserve the global token
  normalization and keep component-specific structure inside CSS Modules; the
  fallback is not adding another round of project-component classes to
  `app.css`.
- if the full route split proves larger than one pass, the minimum truthful
  fallback is:
  - move delete confirmation to the app-owned dialog
  - route project delete through the SolidStart action
  - land URL-backed attached tabs with the app-owned reveal shell
  - extract the attached-surface shell and at least the highest-churn tabs
    first
- if styling starts to sprawl, preserve the new component ownership and defer
  any broader CSS-architecture decision instead of reopening the entire app
  stylesheet in this slice

## 11. Open Questions

These do not block readiness:

- whether the new primitives should live under `src/components/ui/` or a
  project-scoped `src/components/projects/` tree
- whether the new project-surface Playwright proof should stand alone as a
  Phase 20 spec file or extend the existing project-route/browser specs

## 12. Delivery Outcome

This spec is **implemented**.

Delivered in `planner-solid`:

- `/projects` now deletes through an app-owned Kobalte alert dialog backed by a
  SolidStart action instead of `window.confirm`
- `/projects/:projectSlug` now uses extracted project-surface components,
  URL-backed attached tabs, and an app-owned attached-surface shell instead of
  route-local manual tab wiring and the native disclosure wrapper
- the new project-surface primitives and extracted surfaces use co-located CSS
  Modules against normalized base/elevated/overlay tokens in `app.css`
- the client entry now mounts through `@solidjs/start/client/spa`, which keeps
  the already-fixed duplicate-shell regression closed without crashing the
  exported SPA build

Verification completed:

- `npm run lint`
- `npm test`
- `npm run build`
- `npx playwright test e2e/phase-01-projects.spec.ts --grep "phase 02 keeps advanced items hidden while attached knowledge and blueprint stay local|phase 02 allows deleting an open project from the directory|phase 02 keeps the project visible when delete is cancelled|phase 02 keeps the directory stable and shows an inline error when delete fails|phase 02 disables delete while the request is in flight|phase 03 keeps review and build readiness attached to the project workspace" --workers=1 --reporter=list`
- `npx playwright test e2e/phase-17-workflow-closeout.spec.ts --grep "phase 17 closes the workflow loop and keeps Solid as the active surface" --workers=1 --reporter=list`

Still intentionally deferred:

- full app-wide primitive adoption
- full CSS architecture migration
- broad mutation-model conversion to SolidStart actions outside the project
  delete flow
