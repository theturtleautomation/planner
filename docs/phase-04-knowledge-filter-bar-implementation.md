# Phase 04 Knowledge Filter Bar Implementation

**Status:** Research complete, ready for implementation  
**Date:** 2026-03-07

## Objective

Replace the current stacked chip grid in Knowledge Library with a single
horizontal filter bar built from dropdown controls, while preserving the
existing filter semantics, project-scoped persistence, and contextual deep-link
behavior.

This phase is complete when `/knowledge/all` and
`/knowledge/projects/:projectId` both use the same bar model, users can still
recover their scoped views after navigation or remount, and no existing
project/deep-link flow breaks because the control surface changed.

## Non-Goals

- change the meaning of any existing filter field in `ScopedFiltersState`
- introduce multi-select, boolean grouping, or saved filter presets
- add backend filtering APIs or server-side query parsing
- redesign the `/knowledge` project chooser landing page
- redesign project section tabs such as `Overview`, `Inventory`, or `Quality`
- add free-text node search inside the scoped knowledge view
- widen URL query params beyond the current contextual deep-link contract
- change the Phase 02 taxonomy contract beyond consuming its display labels
- remove the project/global route split or change project-scoped loading rules

## Decision Summary

- Keep the current filter state model scalar and single-select:
  - one value per filter family
  - same defaults as today
  - no arrays in local storage or URLs
- Replace `FilterChipGroup` with a reusable horizontal `KnowledgeFilterBar`
  composed of dropdown controls.
- The always-visible desktop control order should be:
  - `Type`
  - `Feature Area`
  - `Surface`
  - `Artifact`
  - `Related Component`
  - `More Filters`
- `More Filters` should contain the less-frequent expert controls:
  - `Placement`
  - `Availability`
  - `Owner`
  - `Tag`
  - `Status`
  - `Freshness`
  - `Connectivity`
  - `Docs`
  - `Lifecycle`
  - `Updated`
- Keep option counts visible inside dropdown menus, but standardize their
  meaning:
  - counts are computed against the current route scope and all active filters
    except the family currently being rendered
  - this replaces the current mixed behavior where some filters use full-node
    counts and others reuse `filteredNodes.length` for every option
- Keep active filters visible below the bar as compact chips, but make them
  dismissible and omit the baseline `Lifecycle: Active` default from the chip
  row.
- Preserve existing persistence and link rules:
  - local storage remains keyed by project or global scope
  - contextual deep links remain limited to `project`, `feature`, `widget`,
    `artifact`, `component`, `from`, and `from_label`
  - contextual deep-link filters still override stored state on entry instead
    of merging with it
- Remove the redundant `Broaden to all project knowledge` action once the bar
  ships, because it currently resets the same state as `Clear filters`.

## Current-State Summary

Knowledge Library already has a meaningful scoped filtering model, but the UI
still exposes it as a long vertical column of chip rows.

| Surface | Current behavior | Current issue |
| --- | --- | --- |
| Filter layout | 15 chip groups render in a stacked column under the scope header | the page reads as a settings form, not a compact filter bar |
| Filter state | every family is a scalar field in `ScopedFiltersState` | the current state model is already single-select, but the UI does not make that obvious |
| Persistence | scoped filters are stored in `localStorage` under `knowledge-scoped-filters:${projectId \|\| global}` | persistence works, but the dense chip layout makes it hard to tell what was restored |
| Contextual deep links | `/knowledge?project=...&feature=...&widget=...&artifact=...&component=...` redirects into project scope and preloads those four secondary filters | deep links are useful, but only those filters are URL-backed today |
| Active filter display | active filters render as read-only chips under the header | users can see the active state, but cannot clear one filter directly from that row |
| Count display | categorical filters show per-option counts, but `stale`, `orphan`, `documentation`, and `updatedDate` currently reuse one shared count for every option | counts are visually present but semantically inconsistent |
| Option density | `buildFilterOptions` caps several option lists at 12 or 16 values | chip-density constraints leak into data visibility and hide lower-ranked options |
| Project actions | `Clear filters`, `Reset to project scope`, and `Broaden to all project knowledge` sit beside the filter surface | two of those actions currently do the same reset work |

### Current filter inventory

The current page exposes these filter families:

- `knowledgeType`
- `scopeClass`
- `scopeVisibility`
- `feature`
- `widget`
- `artifact`
- `component`
- `tag`
- `owner`
- `status`
- `stale`
- `orphan`
- `documentation`
- `lifecycle`
- `updatedDate`

Defaults today:

- every family defaults to `all` except `lifecycle`
- `lifecycle` defaults to `active`
- project-scoped routes include shared records and exclude global records at the
  data-loading level before UI filters run

### Current code anchors

- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- `planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx`
- `planner-web/src/index.css`
- `planner-web/src/lib/knowledgeDeepLinks.ts`
- `planner-web/src/pages/BlueprintPage.tsx`
- `planner-web/src/pages/DiscoveryPage.tsx`
- `planner-web/src/pages/EventTimelinePage.tsx`

### Code findings that matter for the redesign

- `FilterChipGroup` is purely presentational. Replacing it does not require a
  backend change.
- `ScopedFiltersState` and `readScopedFilters` already sanitize stored values,
  so the current storage format can survive the redesign unchanged.
- The deep-link parser only knows about project plus secondary context filters;
  Phase 4 should not silently invent new query params.
- Project review shortcuts set filter state imperatively, for example
  `setScopedFilter('stale', 'stale')` and `setScopedFilter('scopeClass', 'unscoped')`.
  The new bar must reflect those programmatic changes immediately.
- `clearScopedFilters` and `broadenToAllProjectKnowledge` currently reset to the
  same default state, so both controls are not justified after the redesign.
- Current option limits exist because chips take too much room, not because the
  filter model requires truncation.

### Visual audit findings from 2026-03-07

- On desktop, `/knowledge/all` dedicates most of the first viewport to stacked
  chip rows and pushes the actual results table below the fold. The page reads
  like a settings panel before it reads like a knowledge browser.
- On project-scoped knowledge views, the chip wall competes with multiple
  adjacent actions such as `Create knowledge`, `Clear filters`,
  `Reset to project scope`, `Broaden to all project knowledge`, and
  `Open global view`, which makes the header feel overloaded before the user
  reaches the records.
- On mobile, the current layout collapses into a narrow vertical column of
  chips where labels wrap or clip and the result table becomes effectively
  unreachable without excessive scrolling. A horizontal bar plus overflow model
  is therefore required for usability, not just aesthetics.

## Proposed Behavior

### Filter Bar Architecture

Create a dedicated presentation layer for scoped knowledge filters, for example:

- new: `planner-web/src/components/KnowledgeFilterBar.tsx`
- optional new: `planner-web/src/components/KnowledgeFilterSelect.tsx`

The page should stop rendering one `FilterChipGroup` per family inline inside
`KnowledgeLibraryPage`.

The new component should receive:

- the current `ScopedFiltersState`
- normalized option lists
- a `setScopedFilter` callback
- clear/reset handlers
- route context such as `isProjectScoped` and `isGlobalView`

Suggested view-model shape:

```ts
interface KnowledgeFilterDescriptor<K extends keyof ScopedFiltersState> {
  key: K;
  label: string;
  shortLabel: string;
  placement: 'primary' | 'overflow';
  defaultValue: ScopedFiltersState[K];
  options: Array<{
    value: ScopedFiltersState[K];
    label: string;
    count: number;
  }>;
}
```

This keeps option generation declarative and makes count logic testable outside
the JSX tree.

### Control Model

#### Primary controls

The bar should always render these controls in this order:

| Filter key | Visible label | Why it stays primary |
| --- | --- | --- |
| `knowledgeType` | `Type` | broadest first-cut filter and already highly visible in the current UI |
| `feature` | `Feature Area` | direct input to current deep links and project-context navigation |
| `widget` | `Surface` | direct input to current deep links and aligns with Phase 02 terminology |
| `artifact` | `Artifact` | direct input to current deep links and useful for project drill-down |
| `component` | `Related Component` | direct input to current deep links and the narrowest contextual drill-down |
| `more` | `More Filters` | preserves access to expert filters without turning the whole page back into a form |

Rules:

- Keep the same primary order on project and global views so the bar does not
  reflow unpredictably across routes.
- If a primary family has no concrete options beyond `All`, render it disabled
  rather than removing it. Stable layout matters more than saving one control.
- Reuse Phase 02 labels where available:
  - `Feature Area`, not `Feature`
  - `Surface`, not `Widget`
  - `Related Component`, not raw `Component`

#### Overflow controls

`More Filters` should open a compact panel containing the remaining families:

- `scopeClass` -> `Placement`
- `scopeVisibility` -> `Availability`
- `owner` -> `Owner`
- `tag` -> `Tag`
- `status` -> `Status`
- `stale` -> `Freshness`
- `orphan` -> `Connectivity`
- `documentation` -> `Docs`
- `lifecycle` -> `Lifecycle`
- `updatedDate` -> `Updated`

Overflow layout rules:

- desktop: two-column panel under the `More Filters` trigger
- narrow widths: one-column panel under the bar
- all overflow controls still use the same single-select dropdown component as
  the primary controls

This satisfies the "single horizontal list with dropdowns" requirement while
avoiding 15 always-visible controls.

### Single-Select Behavior

Keep every family single-select in Phase 4.

Reason:

- current state, persistence, and deep-link helpers are scalar
- project review shortcuts assume one value per family
- multi-select would force a storage, chip, and URL redesign that is outside
  the scope of this interaction-only phase

Selection rules:

- each dropdown includes an explicit `All` or `Any` option first
- selecting a non-default option replaces the current value for that family
- clearing one filter returns that family to its default only
- `Clear all` resets the whole state to `DEFAULT_SCOPED_FILTERS`

### Count Semantics

Counts should remain visible, but they need one consistent meaning.

#### New rule

For each dropdown family:

- compute counts after applying:
  - the current route scope
  - all currently active filters except the family being rendered
- then measure the result count for each option in that family

Examples:

- if `Type = Decision` and the user opens `Status`, the status counts should
  reflect only decision nodes within the current route scope
- if the user opens `Type`, the counts should reflect all active filters except
  the current `Type` selection

#### `All` option count

The `All` or `Any` option should show the count of records visible when that
family is cleared but every other active filter remains in effect.

#### Option list size

Drop the current hard cap of 12 or 16 values that existed for chip density.
Dropdown menus may scroll.

Recommended UI limits:

- max menu height: about `280px`
- internal search inside dropdown menus: out of scope for Phase 4

### Active Filter Row

Keep the active filter row below the bar, but change its role from passive
summary to active control.

Rules:

- render a chip for every non-default filter value
- each chip includes a dismiss button that resets only that family
- keep a separate `Clear all` action near the bar controls
- do not render `Lifecycle: Active` as an active chip, because it is the
  baseline default, not an explicit narrowing action
- if `Lifecycle: Archived` is selected, render it as a normal active chip

This gives users immediate visibility into deep-link-applied state without
reintroducing the giant chip matrix.

### Baseline And Reset Behavior

#### Baseline

The route baseline remains the current default filter object:

```ts
{
  knowledgeType: 'all',
  scopeClass: 'all',
  scopeVisibility: 'all',
  feature: 'all',
  widget: 'all',
  artifact: 'all',
  component: 'all',
  tag: 'all',
  owner: 'all',
  status: 'all',
  stale: 'all',
  orphan: 'all',
  documentation: 'all',
  lifecycle: 'active',
  updatedDate: 'all',
}
```

#### Action behavior

- `Clear filters` should be relabeled to `Clear all` and reset to the baseline
  above
- `Reset to project scope` remains available only on project routes and should:
  - reset to baseline
  - clear selected nodes
  - navigate to `/knowledge/projects/:projectId`
- remove `Broaden to all project knowledge`, because after the redesign it adds
  no behavior beyond `Clear all`

### Deep-Link And Persistence Rules

#### Local storage

Keep the current storage key format:

- project scope: `knowledge-scoped-filters:${projectId}`
- global scope: `knowledge-scoped-filters:global`

No migration is required as long as the internal values remain unchanged.

#### Contextual deep links

Keep the current supported URL params exactly as they are today:

- `project`
- `feature`
- `widget`
- `artifact`
- `component`
- `from`
- `from_label`

Do not add URL params for `type`, `status`, `owner`, `tag`, or other filters in
Phase 4.

Reason:

- current link producers in Blueprint, Discovery, and Event Timeline already
  depend on the existing contract
- widening the URL contract would create new compatibility work with little UX
  gain for this phase

#### Precedence rules

When a contextual deep link is present on a project route:

- start from the baseline filter object
- apply the contextual `feature`, `widget`, `artifact`, and `component` values
- do not merge unrelated stored filters from the same project

This preserves the current behavior and prevents hidden saved filters from
making a shared deep link look broken.

### Responsive Behavior

The redesign still needs to work on narrower screens without falling back to
the old stacked matrix.

#### Desktop

- primary controls sit in one horizontal row
- `More Filters` opens a floating panel aligned to the trigger
- active chips wrap below the row

#### Tablet and narrow desktop

- the primary row becomes horizontally scrollable
- each control keeps a minimum width so the selected value remains legible
- `More Filters` stays at the end of the row

#### Mobile

- keep the same horizontal row model with horizontal scrolling
- `More Filters` opens an in-flow panel below the row instead of a detached
  floating popover
- active chips remain a separate wrapped row below the bar

This keeps the core interaction model consistent across breakpoints.

### Accessibility And Interaction Rules

Phase 4 should improve accessibility rather than make the filter surface more
custom and brittle.

Recommended implementation constraints:

- prefer native `<select>` controls for the first rollout unless the codebase
  already has a fully accessible dropdown/listbox primitive ready to reuse
- every control must have a visible label or an accessible name
- counts should be part of the option text so they are screen-reader visible
- dismiss buttons on active chips need explicit labels such as
  `Remove filter: Feature Area tasking`
- keyboard users must be able to tab through the bar, open overflow controls,
  and clear filters without pointer-only gestures

Native selects are acceptable for this phase. The requirement is a horizontal
dropdown bar, not a custom combobox system.

## Impacted Files And Modules

### Main page and filter presentation

- `planner-web/src/pages/KnowledgeLibraryPage.tsx`
- new: `planner-web/src/components/KnowledgeFilterBar.tsx`
- optional new: `planner-web/src/components/KnowledgeFilterSelect.tsx`
- `planner-web/src/index.css`

### Shared labels and filter metadata

- `planner-web/src/lib/knowledgeDeepLinks.ts`
- `planner-web/src/lib/taxonomy.ts` from Phase 02, if that phase lands first

### Deep-link producers that must keep working

- `planner-web/src/pages/BlueprintPage.tsx`
- `planner-web/src/pages/DiscoveryPage.tsx`
- `planner-web/src/pages/EventTimelinePage.tsx`

### Tests

- `planner-web/src/pages/__tests__/KnowledgeLibraryPage.test.tsx`
- optional new unit tests if the filter count/descriptor logic is extracted into
  a standalone helper

## API And Data Model Changes

No backend or schema changes are required in Phase 4.

### Explicit non-changes

- no server API changes
- no Rust schema changes
- no changes to `NodeSummary`, `BlueprintResponse`, or query parameter parsing
  beyond existing deep-link helpers
- no change to the shape of `ScopedFiltersState`

### UI-only refactor allowed

The frontend may add a local descriptor/helper layer for rendering and count
calculation, but the persisted values must remain the current raw internal keys.

## UI And Routing Changes

### Knowledge Library layout changes

Apply the new bar only to the scoped knowledge surfaces:

- `/knowledge/all`
- `/knowledge/projects/:projectId`

Do not render the bar on the `/knowledge` project chooser landing page.

### Header and action changes

Keep the existing scope header, project/global context text, and selection
summary, but update the control area as follows:

- replace the current chip grid with `KnowledgeFilterBar`
- keep `Create knowledge`
- replace `Clear filters` with `Clear all`
- keep `Reset to project scope` on project routes
- remove `Broaden to all project knowledge`
- keep the active filter row directly below the bar

### Project review and shortcut flows

Any existing shortcut that sets filters programmatically must update the bar
state without special-case UI handling, including:

- review queue shortcuts such as `Stale records`
- `Needs scope` / unscoped interventions
- archived review flows
- contextual deep-link entry from Blueprint, Discovery, or Event Timeline

### Label application

Use Phase 02 labels in the new surface wherever available:

- `Type`
- `Placement`
- `Availability`
- `Feature Area`
- `Surface`
- `Artifact`
- `Related Component`
- `Docs`

Do not ship a new bar that reintroduces raw internal labels such as
`Scope Class`, `Scope Visibility`, or `Project Contextual`.

## Migration And Backfill Plan

Phase 4 is a client-only UI migration.

### Implementation order

1. Extract filter definition and count helpers out of `KnowledgeLibraryPage`.
2. Introduce the new bar component and wire it to the existing filter state.
3. Swap the current chip grid for the bar.
4. Convert the active chip row into dismissible chips.
5. Remove the redundant `Broaden to all project knowledge` action.
6. Update tests for persistence, deep links, and the new control layout.

### Storage migration

No local-storage key migration is needed.

Existing stored values remain valid because:

- the internal keys do not change
- `readScopedFilters` already sanitizes invalid values
- only the presentation changes

### Rollback safety

If the new bar needs to be reverted, the old chip grid can be restored without
touching stored filter data or route contracts.

## Tests To Add Or Update

### Existing page tests to update

- replace chip-grid-specific assertions with dropdown-bar assertions on
  `/knowledge/all` and `/knowledge/projects/:projectId`
- keep the existing deep-link redirect test and assert the new primary controls
  reflect the contextual selections
- keep the scoped persistence remount test and assert the restored dropdown
  value rather than chip `aria-pressed`

### New behavior tests

- project routes render the primary controls in the expected order:
  - `Type`
  - `Feature Area`
  - `Surface`
  - `Artifact`
  - `Related Component`
  - `More Filters`
- `More Filters` exposes the expert filter families
- dismissing one active filter chip resets only that family
- `Clear all` restores baseline defaults, including `Lifecycle = active`
- baseline `Lifecycle: Active` does not render as an active chip
- `Reset to project scope` clears query-applied context and navigates to the
  canonical project route
- project review shortcut actions update the dropdown state and active chip row

### Helper-level tests

If count calculation is extracted:

- counts are computed against all active filters except the rendered family
- `All` counts reflect clearing only that family
- high-cardinality option lists are not truncated to the old chip-era limits

### Accessibility checks

- each dropdown has an accessible label
- active chip dismiss buttons have accessible names
- overflow controls remain keyboard reachable

## Risks, Dependencies, And Rollout Order

### Dependencies

- Phase 02 taxonomy work should land first or be mirrored exactly in the new
  bar labels
- if a reusable dropdown primitive already exists elsewhere in the app, use it;
  otherwise prefer native `select` for the first rollout

### Risks

- count recomputation can become expensive if every dropdown recalculates on
  every render
- removing chip-era option limits may surface very long menus for tags or
  owners
- if the baseline `Lifecycle: Active` filter is hidden too aggressively, users
  may not realize archived records are excluded by default

### Mitigations

- memoize filter descriptors and count calculations by `nodes`, `edges`, and
  `scopedFilters`
- make dropdown menus scroll instead of truncating options silently
- keep the `Lifecycle` control visible in overflow at all times, even when its
  default chip is hidden

### Rollout order

1. Land shared labels from Phase 02 if not already available.
2. Land the descriptor/count helper refactor.
3. Replace the chip grid with the bar on Knowledge Library.
4. Update deep-link and persistence tests.
5. Validate the responsive layout manually on desktop and mobile widths.

## Unresolved Questions

- Should `Placement` remain an overflow-only expert filter long-term, or should
  it move into the primary row if user testing shows frequent scope triage?
- Should future phases make all filter state shareable in the URL, or should
  URL-backed state stay limited to contextual project drills?
- If the tag and owner lists grow substantially, do we want in-dropdown search
  later, or is a scrollable menu sufficient for the expected dataset size?
