# Planner UI Reset Phase 00 Shell Navigation And Auth Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md)  
**Related Planning:** [Phase 01 Root Landing And Navigation Implementation](/home/thetu/planner/docs/phase-01-root-landing-implementation.md), [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md), [Planner Design System Phase 1 Tonal Foundation Spec](/home/thetu/planner/docs/planner-design-system-phase-1-tonal-foundation-spec.md), [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md)  
**Source Research:** [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx), [LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx), [Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx), and external research on discoverability, navigation hierarchy, progressive disclosure, and explicit wayfinding from Nielsen Norman Group, Apple, Fluent, Carbon, and Material

## Objective

Reset Planner's global shell, navigation model, and auth-entry surfaces so the
app stops overexposing secondary destinations and starts teaching one coherent
product posture from first load onward.

This slice exists because the current shell is visually calmer than before but
still product-noisy:

- the sidebar exposes too many destinations at the same time
- primary work, secondary work, and utility work all compete in the same
  persistent rail
- session-specific status still leaks into the global shell
- anonymous, callback, loading, and dev-mode entry states do not feel like one
  product
- the login view still uses an older terminal-theater frame that no longer
  matches Planner's project-first command-center direction

This phase should make the shell more opinionated:

- persistent navigation should be brief and goal-oriented
- utility destinations should be visible but not permanently co-equal
- auth and entry states should feel like the same product, not a separate demo
  surface

It does **not** redesign the content model of Home, Projects, Sessions,
Knowledge, Blueprint, Discovery, Events, Admin, or the Socratic lobby itself.

## User Outcome

After this slice:

- the global shell makes it obvious where the main work starts
- primary destinations are easier to scan because utility routes no longer
  compete for equal priority
- users can still reach utilities such as Admin, Discovery, and Blueprint, but
  they no longer dominate the default shell
- root entry, login, auth callback, and loading or error states feel visually
  related and product-credible
- local dev mode uses the same product entry posture as authenticated mode
  instead of shortcutting into an older route assumption

## Design Research Synthesis

The following research directly shaped this slice:

- Nielsen Norman Group's heuristics on visibility of system status and
  recognition rather than recall support keeping current location, next action,
  and core destinations visible while minimizing memory burden
- Nielsen Norman Group's heuristic summary warns that every extra unit of
  information competes with the relevant units of information
- Apple guidance on discoverable design argues that essential features should be
  immediately visible, while secondary actions should remain easy to find
  through explicit visual cues rather than hidden gestures
- Fluent navigation guidance recommends brief, scannable, goal-oriented nav with
  consistent order and cautions against cluttered or overly deep structures
- Carbon UI shell guidance supports separating high-frequency wayfinding from
  secondary panels and shell controls
- Material navigation guidance distinguishes persistent drawers for larger
  layouts from temporary drawers on smaller layouts and warns against complex
  hierarchy inside the primary drawer

Planner implication:

- the shell should expose fewer permanent destinations
- the default navigation must emphasize main product goals, not the full
  utility inventory
- utility routes can move into explicit reveal or overflow behavior
- loading, auth, and callback states should reuse the same semantic shell logic
  instead of ad hoc full-screen placeholders

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- the existing route map in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx)
  remains structurally intact in this slice
- the shell stays sidebar-based on larger screens; this is not a top-nav or
  bottom-nav rewrite
- the persistent nav should be shortened to the most important destinations
  only
- secondary and utility destinations should move into an explicit `More` or
  equivalent reveal surface rather than remaining permanently visible as
  co-equal links
- project-first wayfinding remains the shell's primary organizing idea
- page-local status such as session connectivity should not live as a global
  sidebar concern by default
- auth entry, login, callback, loading, and error states should share one
  coherent entry-shell language
- theme or appearance controls may remain available, but they must stay
  low-priority and must not become a design-system rabbit hole

## Scope

### In scope

- global shell hierarchy in
  [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- active-navigation rules and route selection logic tied to the shell
- auth-root entry behavior in
  [App.tsx](/home/thetu/planner/planner-web/src/App.tsx)
  and
  [Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx)
- login and dev-entry surface framing in
  [LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx)
- shell-level reveal model for lower-priority destinations and account or theme
  utilities
- token or shared-class support in
  [index.css](/home/thetu/planner/planner-web/src/index.css)
  only where required by the shell reset

### Out of scope

- detailed redesign of Home, Projects, Sessions, Knowledge, Blueprint,
  Discovery, Events, Admin, or Session page content
- route removals or backend auth changes
- project switcher or command palette product work beyond what the shell needs
- a full user-account settings surface
- redesigning the actual auth provider experience hosted by Auth0

## Current-State Evidence

- in [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx),
  the nav currently renders `Home`, `Projects`, `Knowledge`, `Sessions`,
  `Events`, `Admin`, `Discovery`, and `Blueprint` as persistent sidebar items,
  which makes the shell read like a route inventory rather than a focused
  product map
- `Sessions` is still active for `/session/*` routes, which is accurate but
  keeps the global shell responsible for page-local session framing
- the sidebar footer still mixes account identity, theme toggle, and optional
  session connectivity state in one always-visible global area
- in [LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx),
  the entry experience still uses a terminal-window treatment and product copy
  shaped around older Planner positioning rather than the current project-first
  command-center product
- in [Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx),
  callback loading and error states are standalone full-screen monospace blocks
  rather than part of a shared entry-shell language
- in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx),
  root entry behavior is already project-first, but the shell and auth states
  still need a stronger unified product posture around that route map

## Proposed Behavior

## Shell model

### Persistent shell content

The persistent shell should contain only the highest-frequency product
destinations:

- `Home`
- `Projects`
- `Knowledge`
- `Sessions`

This is the default persistent set because it reflects Planner's real product
flow:

- start
- choose a project
- work with project knowledge
- access the cross-project session queue when needed

### Revealed utility destinations

Lower-frequency or specialized destinations should move behind an explicit,
visible shell-level reveal trigger, for example `More`.

That revealed utility surface should contain:

- `Events`
- `Admin`
- `Discovery`
- `Blueprint`

Rules:

- the reveal trigger must be labeled and always visible
- utility destinations must remain easy to reach
- the reveal surface may be a drawer, sheet, popover, or similar explicit
  secondary shell surface
- the reveal surface should use quieter semantic surfacing than the primary nav

### Active-state model

- active state should remain obvious for both persistent items and any revealed
  utility destination
- when a revealed utility route is active, the shell should still show that
  state clearly even if the utility surface is collapsed
- page-local context such as a specific project or session should not force the
  global shell to expose more permanent items than necessary

### Session-specific shell behavior

- remove default session connectivity badges from the persistent sidebar footer
- session connection state should become page-local in the relevant session
  surface instead of a shell-global concern
- if session-specific shell affordances remain necessary, they should appear
  only inside session routes and in a quieter location than the main nav

## Auth and entry model

### Root entry language

Authenticated root, anonymous root, callback loading, callback error, and local
dev entry should all feel like variants of one product entry shell.

That shared entry language should be:

- calm
- project-first
- concise
- product-credible
- aligned with Planner's current command-center direction

### Login surface

The login view should:

- stop relying on terminal-window nostalgia as the main frame
- explain the product briefly in current language
- foreground one clear entry action
- optionally mention dev mode or Auth0 state in secondary copy
- keep the experience intentionally sparse rather than decorative

### Callback and loading states

Callback loading and error states should:

- reuse the same entry-shell tokens and spacing model as login
- clearly state what is happening now
- avoid ad hoc monospace-only full-screen placeholders
- keep recovery options legible when error state appears

### Local dev mode

Local dev mode should still preserve fast entry, but the experience should not
feel like a bypass into a different product.

That means:

- dev mode should use the same entry-shell framing
- the primary CTA can still be immediate
- any dev badge or mode cue should remain secondary

## Design-System-Patterns Lens

This slice uses `design-system-patterns` in a narrow, route-specific way:

- semantic surface hierarchy:
  - primary shell rail
  - revealed utility surface
  - dormant footer utilities
  - entry-shell base and overlay states
- component-state modeling:
  - anonymous
  - authenticated
  - loading
  - auth callback
  - auth error
  - revealed utility nav open
  - revealed utility nav closed
- token implications:
  shell may need explicit semantic tokens for:
  - nav-primary
  - nav-secondary-reveal
  - entry-shell
  - entry-status-muted

This slice should not introduce a new theming system, CVA migration, or broad
token-pipeline work.

## Dependencies And Touched Surfaces

Likely touched surfaces:

- [planner-web/src/App.tsx](/home/thetu/planner/planner-web/src/App.tsx)
- [planner-web/src/components/Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
- [planner-web/src/pages/LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx)
- [planner-web/src/auth/Auth0Pages.tsx](/home/thetu/planner/planner-web/src/auth/Auth0Pages.tsx)
- [planner-web/src/components/UserInfoAuth0.tsx](/home/thetu/planner/planner-web/src/components/UserInfoAuth0.tsx)
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- any new shell-level utility-reveal component introduced to keep `Layout.tsx`
  bounded

Implementation should stay bounded to shell hierarchy and auth entry coherence.
If the work starts redesigning Home content, project content, or route-local
workflows, stop and split that into the relevant later child spec.

## Acceptance Criteria

- the persistent shell nav exposes only the main product destinations by
  default
- utility routes remain reachable but no longer compete as permanent co-equal
  sidebar items
- active-state behavior remains obvious for both primary and revealed utility
  destinations
- session connectivity no longer appears as a default global-sidebar concern
- login, callback, loading, and auth error states feel visually and verbally
  related to the same product entry shell
- local dev mode uses the same entry posture as authenticated mode rather than
  an obviously separate shortcut path
- the shell feels more directed and less like a route inventory without hiding
  important destinations behind invisible gestures

## Verification Plan

### Automated

- update or add targeted frontend tests for:
  - [Layout.tsx](/home/thetu/planner/planner-web/src/components/Layout.tsx)
  - [LoginPage.tsx](/home/thetu/planner/planner-web/src/pages/LoginPage.tsx)
  - auth-root behavior in [App.tsx](/home/thetu/planner/planner-web/src/App.tsx)
    or auth page tests where applicable
- run `npx tsc --noEmit`

### Manual

- verify root entry in:
  - authenticated mode
  - anonymous Auth0 mode
  - local dev mode
- verify the persistent nav feels shorter and clearer on desktop
- verify utility destinations remain easy to find and activate
- verify mobile or narrow-width shell behavior still preserves access to both
  primary and utility navigation
- verify callback loading and callback error states feel like intentional entry
  states rather than raw placeholders

## Rollback And Fallback

- if moving all utility routes behind one reveal surface is too disruptive,
  first move only `Discovery` and `Blueprint` behind the reveal and leave
  `Events` or `Admin` visible as an intermediate step
- if auth-state unification is too large for one pass, keep the shared entry
  shell for login and callback loading first, then add callback error and dev
  refinements in the same bounded slice before closing the spec
- if one visible shell footer utility still proves necessary, keep it there but
  demote its visual emphasis before reverting the broader hierarchy reset

## Implementation Notes

Implemented on 2026-03-22 with these bounded outcomes:

- persistent shell navigation now focuses on `Home`, `Projects`, `Knowledge`,
  and `Sessions`
- `Events`, `Discovery`, `Blueprint`, and `Admin` now sit behind an explicit
  `More` reveal surface in the shell
- session connectivity no longer renders in the global sidebar footer
- login, callback loading and error, and shared route-loading entry states now
  use a common entry-shell presentation

Verification executed:

- direct auth-entry coverage now exists in
  [Auth0Pages.test.tsx](/home/thetu/planner/planner-web/src/auth/__tests__/Auth0Pages.test.tsx)
  alongside
  [Layout.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/Layout.test.tsx)
  and
  [LoginPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/LoginPage.test.tsx)
- `npx tsc --noEmit`

## Open Questions

None blocking readiness.

This slice is ready because the shell boundaries are concrete, the route map is
already stable, and the required break is structural but still bounded to the
shared shell and auth-entry surfaces.
