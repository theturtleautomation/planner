# Planner SolidStart Phase 19 Typography, Alignment, And Visual Consistency Remediation Spec

**Status:** implemented  
**Date:** 2026-03-25  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md), [Planner UI Reset Phase 02 Projects Directory Spec](/home/thetu/planner/docs/planner-ui-reset-phase-02-projects-directory-spec.md), [Planner UI Reset Phase 04 Sessions Queue Spec](/home/thetu/planner/docs/planner-ui-reset-phase-04-sessions-queue-spec.md), [Planner UI Reset Phase 06 Knowledge Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md), [Planner UI Reset Phase 08 Discovery Review Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-08-discovery-review-workspace-spec.md), [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md), [Planner UI Reset Phase 10 Admin Operations Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md), [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Audit:** 2026-03-25 typography, alignment, and visual-consistency audit of the active `planner-solid` UI against current repo state

> Implementation sync (2026-03-25): `planner-solid` now uses a stronger shared
> type ladder and framed-page rhythm in `src/app.css`, `/projects`,
> `/projects/new`, `/sessions`, `/sessions/new`, `/knowledge`, `/discovery`,
> `/events`, and `/admin` now align to the tightened route-family composition
> contract, and Playwright browser proof now covers representative desktop and
> mobile hierarchy and collapse behavior through
> `planner-solid/e2e/phase-19-visual-consistency.spec.ts`.

## 1. Executive Judgment

The next SolidStart slice should not add route breadth or new feature surface.

The active problem is quality conformance across the already-migrated UI:

- typography hierarchy is too shallow to distinguish page, section, group, row,
  and metadata roles cleanly
- multiple routes reuse the same hero-plus-control composition even when their
  product jobs differ
- container framing and panel alignment drift between routes
- responsive behavior is under-specified for shell, knowledge, and workspace
  layouts
- one core route, `/projects`, is currently weak both compositionally and
  operationally

This slice should therefore be a cross-page remediation pass on the active
SolidStart app, not a new route-family expansion and not a speculative visual
rebrand.

## 2. User Outcome

After Phase 19:

- page titles, section titles, grouped-list headings, row titles, and metadata
  read as a deliberate hierarchy instead of near-peer text
- the main route families feel like one product without feeling like the same
  page repeated
- shared alignment and container rhythm stay stable across home, projects,
  sessions, knowledge, blueprint, discovery, events, and admin
- small-screen behavior is intentionally designed instead of relying on a
  single coarse breakpoint
- `/projects` returns to a stable operating-directory model with one clean row
  grammar
- visual-quality claims for the Solid app are supported by live browser
  verification rather than inferred from CSS alone

## 3. Problems To Solve

### 3.1 Typography hierarchy collapse

The current shared scale in `planner-solid/src/app.css` is too compressed:

- eyebrows, metadata, support copy, row titles, section titles, and some major
  route headings sit too close together in size and emphasis
- routes reuse the same heading classes for materially different semantic roles
- grouped timeline and triage labels sometimes inherit page-section styling
  instead of a quieter group-heading treatment

This makes the product feel flat and slightly cramped even when the underlying
layout is orderly.

### 3.2 Composition sameness across distinct route families

Several routes currently open with the same broad composition:

- a hero panel
- a hero focus block
- a control band
- a follow-on section panel

That composition is too generic for the actual route jobs:

- directories
- queues
- inventories
- review desks
- timelines
- operational consoles
- graph workspaces

The result is consistency without enough route-specific authority.

### 3.3 Container and alignment drift

The active app mixes several near-duplicate framing systems:

- `page-frame` versus raw `stack`
- `panel` versus `section-panel`
- `panel-head` versus `section-head`

Sessions routes in particular still sit outside the newer frame rhythm.

This weakens left-edge consistency, top-of-page authority, and predictable
section spacing.

### 3.4 Responsive contract drift

The current responsive posture is too coarse for the active UI:

- the shell nav does not yet define a proper narrow-width strategy
- Knowledge lacks a strong collapse model for both the toolbar and the
  inventory-detail layout
- Blueprint relies on a large minimum canvas width without enough surrounding
  mobile fallback behavior
- route-level control clusters often rely on wrap luck rather than planned
  priority

### 3.5 Projects directory instability

The `/projects` route is currently the weakest operational and visual surface:

- the source is currently malformed, which blocks runtime review
- the row model has drifted away from the clean directory-object contract set
  by the earlier route-reset spec
- primary row scanning is weakened by a split row/action composition

This route must be repaired before the wider visual-consistency pass can be
considered complete.

### 3.6 Verification drift

The current evidence for UI quality is narrower than the problem requires:

- existing Playwright coverage is mostly mocked route interception
- the current `/projects` parse error prevents broad live runtime review
- there is not yet a small, explicit browser-verification set for desktop and
  mobile hierarchy and alignment across representative routes

## 4. Scope

### In Scope

- shared typography, spacing, and composition-token changes in
  `planner-solid/src/app.css`
- shared shell/layout adjustments in `planner-solid/src/app.tsx` when needed to
  support responsive or alignment consistency
- route-level markup cleanup where a page is misusing shared typography or
  composition primitives
- restoring `/projects` to a stable operating-directory structure
- bringing `/sessions` and `/sessions/new` into the same framing discipline as
  the rest of the Solid app
- route-specific responsive cleanup for Knowledge, Blueprint, and other
  affected workspaces
- live browser verification additions for representative desktop and mobile
  routes
- planning/doc truthfulness updates if the implementation intentionally
  supersedes any active route-level assumptions

### Out Of Scope

- new product capabilities unrelated to visual hierarchy or layout consistency
- backend changes unless a page is currently blocked from rendering
- a full design-system rewrite or component-library abstraction project
- color-theme reinvention, multi-theme work, or branding exploration detached
  from the audit
- speculative redesign of route information architecture beyond what is needed
  to satisfy the audit

## 5. Current-State Evidence

The 2026-03-25 audit established:

- `planner-solid/src/app.css` currently uses a shallow text ladder where
  multiple semantic roles sit only one step apart
- the same hero and section primitives are used across route families whose
  jobs should read differently at first glance
- sessions routes still use older raw-stack framing instead of the newer page
  frame
- the responsive CSS currently relies too heavily on one `max-width: 900px`
  breakpoint
- the current `/projects` implementation is malformed and could not be
  runtime-reviewed
- current Playwright evidence is mostly mocked and does not yet serve as live
  proof of cross-page hierarchy quality

This spec is the bounded response to that evidence.

## 6. Product And Technical Contract

### 6.1 Shared typography ladder

The shared Solid CSS must define a clearer and narrower set of semantic text
roles with explicit ownership:

- page title
- page intro or support copy
- section title
- section support copy
- grouped-list heading
- row title
- row metadata or fact text
- eyebrow or overline
- button and control label

Minimum constraints:

- page titles must read materially larger and more authoritative than section
  titles
- section titles must read materially stronger than grouped-list headings
- grouped-list headings must not reuse primary page-section styling
- metadata and supporting copy must remain readable without visually competing
  with row or section titles

This should be done through shared semantic primitives, not route-by-route font
size drift.

### 6.2 Shared page-family composition contract

The active Solid app must stop applying one generic top-of-page composition to
every route.

At minimum, the implementation must distinguish these page families:

- command or launch surfaces:
  home and project-entry surfaces may keep stronger headline framing
- directories and queues:
  projects and sessions should make the list the dominant object immediately
- inventory and review surfaces:
  knowledge, discovery, events, and admin should surface the working object or
  stream first, with compact route framing
- workspace surfaces:
  Socratic and Blueprint may keep route-specific framing because they are true
  workspaces, not utility stacks

The shared system should support those families without requiring each page to
invent its own bespoke visual grammar.

### 6.3 Container and alignment contract

The Solid app must converge on one standard framed-page rhythm for non-workspace
routes and one explicit workspace rhythm for dense tool surfaces.

Minimum requirements:

- left and right edges should align consistently across standard routes
- route headers, control bands, and section surfaces should use one shared
  spacing rhythm
- near-duplicate primitives such as `panel` and `section-panel` should either
  be unified or given clearly distinct semantic jobs
- `/sessions` and `/sessions/new` must no longer look like pre-reset holdovers

### 6.4 Responsive contract

The active UI must define explicit small-screen behavior for the routes already
identified as weak:

- shell navigation
- knowledge toolbar
- knowledge inventory-detail layout
- blueprint canvas framing and overflow support
- dense action clusters on directory and operations routes

The goal is not perfect mobile parity with every desktop composition.

The goal is truthful, intentional collapse behavior that preserves hierarchy and
next action.

### 6.5 Route-specific obligations

The shared system remediation must still honor route-specific truth:

- `/projects` remains a directory-first operating list with one clean row
  grammar and one obvious primary next move per project
- `/sessions` remains an attention queue and should not inherit generic hero
  chrome above the queue
- `/knowledge` remains inventory-first with attached detail and controlled
  filter framing
- `/discovery` remains a triage desk, so proposal objects must visually outrank
  surrounding controls
- `/events` remains a stream-first timeline, and day/group headings must not
  look like page-section headers
- `/admin` remains an operations console with one strong health posture and
  clearly subordinate supporting streams
- `/blueprint` may keep graph-specific framing, but the surrounding chrome must
  behave intentionally at narrower widths

### 6.6 Runtime-integrity requirement

This slice includes a hard stability gate:

- the active Solid app must build and boot successfully before visual
  verification is treated as complete
- `/projects` parse or render regressions block closeout because that route is
  one of the representative directory surfaces for the audit

### 6.7 Verification contract

This remediation slice is not complete when the CSS merely looks better in
diffs.

Minimum proof must include:

- a passing Solid build and lint pass
- live browser verification across representative desktop and mobile widths
- representative route coverage for:
  - home
  - projects
  - sessions
  - knowledge
  - one of discovery or events
  - blueprint or admin if changed materially
- evidence that shared-system fixes did not leave one route family on an older
  container rhythm

Mocked route interception tests may still exist, but they are not sufficient on
their own to close this slice.

## 7. Dependencies And Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/app.css`
- `planner-solid/src/app.tsx`
- `planner-solid/src/routes/index.tsx`
- `planner-solid/src/routes/projects/index.tsx`
- `planner-solid/src/routes/projects/new.tsx`
- `planner-solid/src/routes/sessions/index.tsx`
- `planner-solid/src/routes/sessions/new.tsx`
- `planner-solid/src/routes/knowledge/index.tsx`
- `planner-solid/src/routes/blueprint/index.tsx`
- `planner-solid/src/routes/discovery/index.tsx`
- `planner-solid/src/routes/events/index.tsx`
- `planner-solid/src/routes/admin/index.tsx`
- `planner-solid/e2e/*` for widened live browser proof

Additional route files may be touched if they are using shared hierarchy classes
incorrectly, but the implementation should stay bounded to the audit findings
and their direct supporting surfaces.

## 8. Acceptance Criteria

- the Solid app exposes a clearer shared typography ladder for page, section,
  group, row, and metadata roles
- standard routes use a consistent framed-page alignment system, and workspace
  routes use a clearly distinct but deliberate workspace system
- the most generic hero-plus-control composition is no longer repeated
  indiscriminately across utility routes
- `/projects` is stable, builds cleanly, and reads as a directory-first route
  again
- `/sessions` and `/sessions/new` no longer look visually detached from the
  rest of the Solid app
- knowledge and blueprint narrow-width behavior is intentionally defined rather
  than left to overflow accidents
- representative browser verification exists for desktop and mobile hierarchy
  across the updated routes

## 9. Verification Plan

### Automated

- run the Solid route test, lint, and build targets needed for the touched
  files
- widen Playwright coverage so the representative route set is exercised
  against the running Solid app rather than only mocked route payloads
- add or update assertions where route structure changes materially, especially
  for `/projects`, `/sessions`, and `/knowledge`

### Manual

- verify the representative route set at one desktop width and one mobile or
  narrow-tablet width
- verify title, section, grouped-list, and metadata hierarchy on each reviewed
  route
- verify left-edge and section-spacing consistency across at least home,
  projects, sessions, and one utility route
- verify no route now looks like an accidental outlier because the shared fixes
  overfit another page family

## 10. Rollback And Fallback

- if a full shared-primitive unification is too risky in one pass, preserve the
  new semantic contract and localize shims while keeping the naming and usage
  direction explicit
- if a route-specific composition change destabilizes workflow clarity, preserve
  the stronger shared typography and alignment fixes and defer only that
  localized composition adjustment
- if a narrow-width workspace layout cannot fully collapse in this slice, keep
  the primary surface truthful and defer secondary affordance polish rather
  than restoring broken desktop-first framing

## 11. Open Questions

None blocking this phase.

The audit was specific enough to bound the work without additional discovery.

## 12. Implementation Judgment

This spec is **implemented**.

The active Solid app now carries the bounded remediation this slice required:

- shared typography hierarchy is stronger across page, section, group, row,
  and metadata roles
- standard routes now use a more consistent framed-page alignment system
- `/projects` has a stable directory row grammar again, including the delete
  action without breaking primary row scanning
- `/sessions` and `/sessions/new` now align with the main Solid page rhythm
- Knowledge, Discovery, Events, and Admin now use tighter route-intro framing
  instead of the oversized repeated hero shell
- narrow-width shell, knowledge, and blueprint behavior is now backed by live
  browser proof rather than CSS-only inference

Verification completed with:

- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
- `cd planner-solid && npx playwright test e2e/phase-01-projects.spec.ts e2e/phase-08-events.spec.ts e2e/phase-09-admin.spec.ts e2e/phase-10-knowledge.spec.ts e2e/phase-11-blueprint.spec.ts e2e/phase-12-discovery.spec.ts e2e/phase-19-visual-consistency.spec.ts`
