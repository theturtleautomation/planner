# Planner UI Reset Route-By-Route Spec Queue

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Project-First UI Research Sessions](/home/thetu/planner/docs/project-first-ui-research-sessions.md), [Planner UI Reset Tranche Audit Remediation Spec](/home/thetu/planner/docs/planner-ui-reset-tranche-audit-remediation-spec.md), [Planner UI Reset Residual Corrections Spec](/home/thetu/planner/docs/planner-ui-reset-residual-corrections-spec.md), [Planner UI Reset Audit Evidence Closeout Spec](/home/thetu/planner/docs/planner-ui-reset-audit-evidence-closeout-spec.md)  
**Source Research:** route inventory from [App.tsx](/home/thetu/planner/planner-web/src/App.tsx), current page components under `planner-web/src/pages/`, current implemented design-system follow-on specs, and external research on visibility of system status, recognition rather than recall, discoverability, disclosure patterns, progress signaling, and layout hierarchy from Nielsen Norman Group, Apple, Fluent, Material, and Carbon

## Purpose

Open a methodical, route-by-route UI reset program for Planner.

This document is not a single implementation spec. It is the planning container
that turns "reset the whole UI" into a sequence of bounded child specs that can
be researched, tightened, and implemented one page family at a time.

## Why This Exists

Planner's command-center visual system work improved tonal layering, route
hierarchy, and operational density, but it did not fully reset the product
model of every page.

The current need is broader and deeper:

- several routes still expose too many simultaneously visible modules
- some pages are visually calmer than before but still not structurally clear
- different route families need different product models:
  - focused workspaces
  - directory surfaces
  - review consoles
  - graph-adjacent tools
  - inventory-and-context pages
- the right answer will not be the same on every page

This queue therefore treats each route family as its own bounded spec-lifecycle
artifact.

## Cross-Cutting Reset Thesis

The next UI reset should build on Planner's current direction, not discard it.

Keep:

- calmer command-center surfaces
- tonal layering over border-heavy frames
- strong hierarchy
- product-first clarity
- dense real data without fake KPI theater

Reset:

- pages where multiple panes still compete for first attention
- page models that default to showing every support surface at once
- routes that feel like stacked reports instead of clear working surfaces
- weak reveal models for secondary context
- route-specific ambiguity about what the user should look at first

## Research Principles To Reuse Across Child Specs

Each child spec should reuse these web-research principles and then add route-
specific research where needed:

- visibility of system status:
  the route must make current state and next step understandable without hidden
  inference
- recognition over recall:
  important context should be visible or easy to retrieve when needed
- essential-first discoverability:
  the primary task should be immediately visible; secondary tools may require a
  reveal step
- explicit disclosure:
  drawers, sheets, inspectors, and overlays are valid only when their triggers
  are obvious and labeled
- single progress narrative:
  avoid several competing indicators for the same operation
- hierarchy by space and surface:
  important work gets the most visual priority; supporting context gets a
  quieter semantic surface

## Route Inventory

This queue is based on the actual routed React page set in
[App.tsx](/home/thetu/planner/planner-web/src/App.tsx).

### Page families in scope

- shell and auth entry:
  `/`, `/callback`, auth-dependent entry and login states
- Home hub:
  `/`
- projects directory:
  `/projects`
- project workspace:
  `/projects/:projectSlug/sessions`
- sessions queue:
  `/sessions`
- Socratic session lobby:
  `/session/new`, `/session/:id`
- knowledge workspace:
  `/knowledge`, `/knowledge/all`, `/knowledge/projects/:projectId`
- blueprint workspace:
  `/blueprint`
- discovery review:
  `/discovery`
- events timeline:
  `/events`
- admin operations:
  `/admin`

## Child Spec ID Queue

These IDs are the canonical route-reset sequence. Each child spec should become
its own durable doc.

| ID | Route family | Target doc | Current state |
| --- | --- | --- | --- |
| `UIR-00` | Shell, navigation, auth entry, and login states | `docs/planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md` | implemented |
| `UIR-01` | Home hub | `docs/planner-ui-reset-phase-01-home-hub-spec.md` | implemented |
| `UIR-02` | Projects directory | `docs/planner-ui-reset-phase-02-projects-directory-spec.md` | implemented |
| `UIR-03` | Project workspace sessions and import review/history | `docs/planner-ui-reset-phase-03-project-workspace-spec.md` | implemented |
| `UIR-04` | Global sessions queue | `docs/planner-ui-reset-phase-04-sessions-queue-spec.md` | implemented |
| `UIR-05` | Socratic focused question lobby | [phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md) | implemented |
| `UIR-06` | Knowledge workspace | `docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md` | implemented |
| `UIR-07` | Blueprint workspace | `docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md` | implemented |
| `UIR-08` | Discovery review workspace | `docs/planner-ui-reset-phase-08-discovery-review-workspace-spec.md` | implemented |
| `UIR-09` | Events timeline workspace | `docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md` | implemented |
| `UIR-10` | Admin operations workspace | `docs/planner-ui-reset-phase-10-admin-operations-workspace-spec.md` | implemented |

## What Each Child Spec Must Decide

Every route-reset spec should answer these questions explicitly:

1. What is the route's primary job?
2. What should be the one dominant focal surface?
3. What supporting context should be visible by default?
4. What context should move into a reveal surface?
5. What state changes must be visible immediately?
6. What page-specific product truth must not be hidden behind ornamental UI?
7. What existing design-system patterns still apply, and where do they need a
   stronger product-model reset?

Each child spec must still satisfy the standard spec-lifecycle requirements:

- problem framing
- user outcome
- scope boundaries
- dependencies
- contracts and touched surfaces
- acceptance criteria
- verification plan
- rollback or fallback
- open questions

## Design-System-Patterns Lens

Use `design-system-patterns` in child specs only where it sharpens the route
model:

- semantic surface hierarchy:
  define which surfaces are primary, secondary, overlay, and dormant
- reveal-model discipline:
  specify when drawers, inspectors, sheets, or overlays are justified
- component-state modeling:
  define the meaningful visible states for the route
- token implications:
  note when a route needs new semantic tokens or state tokens

Do not let this queue drift into:

- generic component-library modernization
- multi-brand theming
- dark-mode ideology
- Tailwind or CVA migration work
- token work without a concrete route problem

## Sequencing

The queue should be worked in this order unless a stronger dependency appears:

1. `UIR-00` shell, navigation, auth entry
2. `UIR-01` home hub
3. `UIR-02` projects directory
4. `UIR-03` project workspace
5. `UIR-04` sessions queue
6. `UIR-05` Socratic lobby
7. `UIR-06` knowledge workspace
8. `UIR-07` blueprint workspace
9. `UIR-08` discovery review
10. `UIR-09` events timeline
11. `UIR-10` admin operations

Reasoning:

- shell and entry states set the global frame
- home and project routes define the product's default operating posture
- project workspace and sessions queue determine the work-dispatch model
- Socratic then fits into the broader reset instead of remaining a special-case
  redesign
- knowledge, blueprint, discovery, events, and admin can then be reset using a
  clearer page-family vocabulary

## Current Route-Specific Starting Points

These are the first-pass route lenses each child spec should start from.

### `UIR-00` shell, navigation, auth

- decide whether the shell is still overexposing secondary destinations
- decide how auth, login, and root entry states should feel coherent with the
  project-first product
- define shell-level reveal patterns versus always-visible navigation

### `UIR-01` home hub

- decide whether home is a launchpad, command desk, or briefing surface
- define what should be dominant above the fold
- remove equal-weight quick-link behavior if a stronger home posture is needed

### `UIR-02` projects directory

- decide whether projects is best framed as directory, operating list, or
  portfolio dashboard
- define row density, next-action clarity, and project identity hierarchy

### `UIR-03` project workspace

- decide whether sessions, import review, and history belong in one page model
  or a clearer staged workspace within the page
- define the dominant project-local focal surface

### `UIR-04` sessions queue

- treat the route as an attention queue, not a generic list
- define resumable, blocked, active, and stale states as first-class row
  semantics

### `UIR-05` Socratic lobby

- already drafted as the focused-question-lobby reset
- should be revisited only as this wider queue clarifies shell and project
  context around it

### `UIR-06` knowledge workspace

- decide whether the route is primarily inventory, project overview, or detail
  context
- determine how much context should be attached versus revealed

### `UIR-07` blueprint workspace

- preserve graph performance
- define command chrome, inspector, and selection authority more cleanly
- avoid graph theater and keep the graph as the primary canvas

### `UIR-08` discovery review

- treat the route as a proposal-review console, not a loose utility page
- clarify proposal triage, related knowledge context, and next action

### `UIR-09` events timeline

- treat the route as a chronological operating surface
- define primary timeline focus versus secondary filters or snapshot context

### `UIR-10` admin operations

- treat the route as a bounded operations console
- define the single dominant health or action surface and subordinate the rest

## Planning Rules

- do not create all child specs in one pass unless the route is trivially small
- do one child spec at a time and sync planning docs after each material change
- if a child spec materially changes the queue order, update this parent doc and
  [project-plan.md](/home/thetu/planner/docs/project-plan.md)
- if a route can be solved by tightening an existing spec instead of opening a
  new child spec, record that explicitly
- if a route has no real product problem after review, close its ID as
  intentionally skipped instead of forcing a cosmetic spec

## Delivery Status

This parent planning container is implemented.

Completed:

- the route inventory
- the child spec IDs
- the cross-cutting research method
- the full child spec set for `UIR-00` through `UIR-10`
- route delivery for `UIR-00` through `UIR-10`
- the 2026-03-22 bounded tranche audit remediation that synchronized queue and
  tracker status, narrowed overstated child-spec language, and strengthened
  route-specific verification where the audit found thin evidence
- the residual correction follow-up tracked explicitly in
  [Planner UI Reset Residual Corrections Spec](/home/thetu/planner/docs/planner-ui-reset-residual-corrections-spec.md)
  instead of reopening this exhausted queue as if it were a fresh route
  delivery container
- the audit evidence closeout in
  [Planner UI Reset Audit Evidence Closeout Spec](/home/thetu/planner/docs/planner-ui-reset-audit-evidence-closeout-spec.md)
  closed the last `UIR-00` auth-entry proof gap and `UIR-05` focused-lobby
  proof gap without broadening the original route scope

Verification support for tranche closure now lives in:

- [Planner UI Reset Tranche Audit Remediation Spec](/home/thetu/planner/docs/planner-ui-reset-tranche-audit-remediation-spec.md)
  for the broad route-suite rerun and initial planning-truthfulness sync
- [Planner UI Reset Residual Corrections Spec](/home/thetu/planner/docs/planner-ui-reset-residual-corrections-spec.md)
  for the focused follow-up on parent closeout language, Home hierarchy,
  Blueprint verification, and Events failure-state verification
- [Planner UI Reset Audit Evidence Closeout Spec](/home/thetu/planner/docs/planner-ui-reset-audit-evidence-closeout-spec.md)
  for the final auth-root, callback, context-shelf, and branch-transition
  verification hardening

## Open Questions

None blocking this parent container.

The original queue is now exhausted and fully closed.

Any remaining work should be cumulative QA or follow-on route specs opened from
real product gaps, not additional delivery against this original queue.
