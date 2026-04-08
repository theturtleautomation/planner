# Phase 13 Socratic Focused Question Lobby Reset Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md), [Phase 11 Socratic Category Replay And Validation Spec](/home/thetu/planner/docs/phase-11-socratic-category-replay-and-validation-spec.md), [Phase 10 Socratic Category Status And Refresh Spec](/home/thetu/planner/docs/phase-10-socratic-category-status-and-refresh-spec.md), [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md), [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md), [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md), [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md), plus external research on visibility of system status, recognition over recall, discoverability, progress signaling, disclosure patterns, and layout hierarchy from Nielsen Norman Group, Apple, Fluent, Material, and Carbon  
**Prior Slice:** [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md)

## Objective

Break the current split-lobby product shape and replace it with a focused
question lobby where the active Socratic work is the only real focal point.

Phase 12 correctly changed the product model from category-first drill-down to
live workspace. That solved the largest structural issue, but not the deeper
experience issue. The current lobby still feels like several surfaces competing
for attention at once:

- a status header trying to explain the session
- a category frame trying to orient the user
- a question workspace trying to be the main task surface
- a right-side context area still visually present enough to compete with the
  question flow

That model is still too busy for a planning product where dynamic categories,
convergence, and newly synthesized questions are the whole point.

The reset in this slice is more opinionated:

- when the user is in the Socratic question lobby, the active question surface
  becomes the dominant and nearly exclusive focal point
- everything else remains available, but it is hidden by default and revealed
  explicitly through simple, labeled affordances
- dynamic categories, convergence, and branch change stay central to the
  product idea, but they are moved into a calmer reveal model instead of being
  permanently visible as competing panes

This slice is still bounded to the existing Planner product and current live
workspace model. It does **not** redesign Socratic reasoning, replace
server-authored question generation, or broaden into a general pipeline IA
overhaul.

## User Outcome

After this slice:

- the active Socratic question is visually dominant and easier to think inside
- users can still see what Planner is doing, but that system visibility is
  compact, legible, and secondary to the current question task
- dynamic categories remain real and important, but they are revealed through a
  dedicated map layer instead of occupying permanent equal-weight screen space
- users can inspect all active category questions without serial hunting, but
  that inspection happens in an intentional reveal surface rather than in a
  permanently noisy page
- when answers converge, spawn new categories, move work, or resolve a branch,
  the system explains that change in-context and updates the reveal surfaces
  truthfully

The user still does **not** get editable question banks, collaborative
multi-user planning, or a broader redesign of post-intake pipeline surfaces.

## Design Research Synthesis

The following guidance directly informed this reset:

- Nielsen Norman Group's visibility-of-system-status heuristic argues that users
  need timely feedback that helps them understand current state and next steps
  instead of inferring system state indirectly
- Nielsen Norman Group's recognition-rather-than-recall heuristic argues that
  required information should be visible or easily retrievable when needed,
  rather than forcing users to remember it across surfaces
- Nielsen Norman Group's heuristic summary reinforces that every extra unit of
  information competes with the relevant units of information
- Apple's discoverability guidance recommends prioritizing essential features so
  they are immediately visible, while letting non-essential parts require some
  navigation to reach
- Fluent's layout guidance shows that space and proximity should create
  hierarchy and that the most important area should have the strongest visual
  focus
- Carbon's disclosure guidance supports hiding advanced or secondary controls in
  explicit popovers or panels, but only when the disclosure mechanism is
  obvious, labeled, and low-friction
- Material's progress guidance warns against showing multiple competing progress
  indicators for the same operation

Planner implication:

- the current question must be immediate
- system state must be compact and truthful
- category and context surfaces must be easy to retrieve, not permanently
  dominant
- reveal patterns must use visible labeled triggers, not invisible gestures or
  implicit hot zones

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- the Socratic lobby remains the existing `SessionPage` route; the break is in
  page structure and interaction model, not route architecture
- the default lobby state after intake begins or resumes is `focused question`
  mode, not `all surfaces visible` mode
- the active question canvas is the primary and dominant module of the lobby
- category navigation remains server-authored, but it moves into a dedicated
  reveal surface instead of a permanently visible co-equal pane
- belief state, draft review, and events remain available through a dedicated
  context reveal surface instead of permanent split-pane presence
- users must have a first-class way to inspect all active category questions
  without serial hunting, but this inspection surface is revealed intentionally
  rather than left open by default
- websocket updates remain the default update model for the lobby, but only the
  most meaningful state changes should surface in the main focal area
- any warm question or prompt reuse remains per-session and server-authored and
  must never outrank clarity or truthfulness
- no hidden-gesture-only navigation is allowed for map, context, or change
  surfaces; all reveal patterns require visible, labeled triggers

## Scope

### In scope

- replacing the current persistent split-pane lobby with a focused question
  lobby model
- defining the default focal surface, the reveal surfaces, and their state
  contracts
- preserving dynamic categories and convergence while changing how they are
  exposed in the UI
- making "what is happening" visible without letting status chrome dominate the
  question task
- providing a revealable question-map or question-index surface so users can
  inspect all active category questions without serial branch hunting
- defining truthful branch transition behavior for:
  - new category created
  - question set prepared
  - branch moved
  - branch resolved
  - branch blocked
  - build ready
- using websocket-driven updates where they materially improve focused-lobby
  clarity
- verification planning for the focused-lobby reset

### Out of scope

- redesigning the belief-state engine, prompt adjudication rules, or category
  synthesis logic
- changing the broader project/session route tree
- cross-session prompt libraries or reusable organizational question banks
- generic theme-system modernization or design-system expansion outside what the
  lobby needs
- post-intake pipeline UX redesign

## Current-State Evidence

- in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx),
  the session page now supports a live workspace, but still presents multiple
  always-on areas that compete for first attention
- in
  [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx),
  the current workspace still behaves as a visible rail plus visible group list
  plus visible status layer, which makes the page more legible than Phase 08
  but not yet focused
- in
  [SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx),
  [BeliefStatePanel.tsx](/home/thetu/planner/planner-web/src/components/BeliefStatePanel.tsx),
  [SpeculativeDraftView.tsx](/home/thetu/planner/planner-web/src/components/SpeculativeDraftView.tsx),
  and
  [SessionEventsTable.tsx](/home/thetu/planner/planner-web/src/components/SessionEventsTable.tsx),
  important context already exists, but its current placement encourages
  simultaneous reading instead of intentional reveal
- the existing workspace snapshot and websocket flows already expose enough
  focused-question state for a bounded web reset, even if a richer future
  contract could support more explicit lobby summary or change semantics later

## Product Thesis

The right product surface is not "persistent workspace plus persistent side
modules."

The right product surface is:

- one focused question canvas
- one compact session pulse
- two reveal surfaces:
  - question map
  - context shelf

Everything else follows from that.

## UI Model

### Lobby structure

The focused question lobby should have three persistent pieces and two reveal
surfaces.

Persistent pieces:

1. session pulse bar
2. focused question canvas
3. minimal answer controls

Reveal surfaces:

1. question map
2. context shelf

This is a deliberate break from the current split workspace.

### Session pulse bar

The pulse bar is the only always-visible status surface above the question
canvas.

It should stay compact and operational, not descriptive and sprawling.

Its responsibilities:

- communicate the current lobby state:
  - `ready now`
  - `preparing`
  - `changed`
  - `build ready`
- expose the next-action sentence from server truth
- show compact counts or badges for:
  - ready question groups
  - changed branches
  - preparing branches
- provide visible labeled triggers for:
  - `Question map`
  - `Context`

The pulse bar should not duplicate belief-state detail, event feeds, or full
category rows. It exists to keep the user oriented while preserving focus on
the active question.

### Focused question canvas

The question canvas is the center of the product.

Default behavior:

- show the currently active question group or draft-review group
- keep the active group visually dominant and spacious
- suppress other product surfaces by default
- make answering, reviewing, and advancing the current group feel like the only
  primary task on the page

The canvas should also be able to render key non-question states without
throwing the user out of the focused model:

- `preparing next questions`
- `branch changed`
- `branch moved`
- `branch resolved`
- `build ready`

Those states should appear as first-class inline canvases or transition cards,
not as hidden side effects that require opening another surface to understand.

### Question map

The question map is the reveal surface for dynamic categories and all active
question inspection.

It is the replacement for the always-visible category rail and all-groups feed.

The map may be implemented as a drawer, sheet, or large overlay, but its
product responsibilities are fixed:

- expose the dynamic category structure
- show which categories are:
  - ready
  - changed
  - preparing
  - blocked
  - resolved
- show all currently active question groups without requiring serial category
  entry
- allow the user to inspect question previews or titles for every active group
- allow the user to focus any question group and return to the question canvas

The map is not a destination route. It is a revealable planning index for the
current session.

### Context shelf

The context shelf is the reveal surface for secondary operational context.

It should house:

- belief state
- draft context or draft review support
- events

The shelf may use tabs internally, but it should remain hidden by default while
the user is answering or reviewing questions.

Its product role is:

- provide support when the user asks for it
- expose unread or elevated event state through the pulse bar
- never compete visually with the focused question canvas unless the user opens
  it

### Minimal answer controls

Answer controls should remain attached to the active question canvas rather than
distributed across surrounding chrome.

The user should not need to look left for the category, right for the context,
up for readiness, and down for actions. The current question flow should feel
vertically coherent.

## Dynamic Categories And Convergence Model

The main idea of the product remains unchanged:

- categories are dynamic
- answers converge the interview
- new answers can produce:
  - new categories
  - new questions in existing categories
  - moved work
  - removed work

The change in this slice is how that truth is revealed.

### In focused-question mode

The canvas should expose only the change that matters now:

- if a new branch appears, show a compact transition notice with the new
  category name and a quick path to focus it
- if work moved, explain where it moved
- if work resolved, explain that the branch is complete and what remains
- if preparation is happening, show one inline preparing state for the current
  next step rather than multiple competing loading treatments

### In question-map mode

The reveal surface should expose the full shape of convergence:

- new categories highlighted
- changed categories marked
- moved work surfaced in both origin and destination where useful
- blocked or resolved categories still visible long enough for comprehension

## Visibility Model

This slice uses a focus-first visibility model:

- primary task is always visible
- secondary information is easily retrievable
- tertiary detail is hidden until requested

Applied to Planner:

- active question is always visible
- category/question index is one click away
- belief state, draft context, and events are one click away
- no important system change should require memory alone; it must be visible
  either in the pulse bar, in the current question canvas, or in the revealed
  map

## Realtime Model

Realtime updates are part of the experience, but not the product headline.

### Main-canvas realtime events

The focused question canvas should react in realtime for only the most relevant
changes:

- current question group prepared
- current branch updated
- current branch moved or resolved
- build readiness changed

### Reveal-surface realtime events

The question map and context shelf should update in realtime so they are
truthful when opened:

- category counts
- ready or changed badges
- newly created categories
- event counts
- draft availability

### Progress signaling

The lobby should avoid multiple competing progress indicators for the same
operation.

Rules:

- one primary progress treatment per active operation
- pulse bar may show compact status text or badge
- canvas may show the main preparing treatment
- map may show preparing badges when opened
- do not show several unrelated spinners or bars for the same synthesis event

## Warm Question Reuse

Per-session warm question reuse remains optional and subordinate.

If used, it should support this lobby model in exactly two ways:

- making the next likely focus target feel immediate
- letting the question map reveal real question previews for ready groups

If warm reuse increases correctness risk or makes branch-change explanation
harder to trust, remove or reduce it before weakening the focused lobby model.

## Contracts And Touched Surfaces

This implemented slice stays within the existing web contract.

Touched surfaces for the delivered reset:

- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/components/SessionStatusHeader.tsx](/home/thetu/planner/planner-web/src/components/SessionStatusHeader.tsx)
- [planner-web/src/components/CategoryNavigator.tsx](/home/thetu/planner/planner-web/src/components/CategoryNavigator.tsx)
- [planner-web/src/components/PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
- [planner-web/src/components/BeliefStatePanel.tsx](/home/thetu/planner/planner-web/src/components/BeliefStatePanel.tsx)
- [planner-web/src/components/SpeculativeDraftView.tsx](/home/thetu/planner/planner-web/src/components/SpeculativeDraftView.tsx)
- [planner-web/src/components/SessionEventsTable.tsx](/home/thetu/planner/planner-web/src/components/SessionEventsTable.tsx)

Out of scope for the delivered pass:

- schema expansion in
  [planner-schemas/src/artifacts/socratic.rs](/home/thetu/planner/planner-schemas/src/artifacts/socratic.rs)
- new Socratic core planning or engine behavior
- websocket protocol redesign beyond current snapshot and refresh behavior

Implementation should stay bounded to the Socratic lobby/workspace model. If
the work starts broadening into pipeline IA redesign, global prompt management,
or generic design-system refactoring, stop and split it into a later spec.

## Acceptance Criteria

- the default Socratic lobby after intake begins or resumes is a focused
  question experience, not a multi-pane operational dashboard
- the active question or review group is the dominant and nearly exclusive
  focal point on the page
- category map, all-question inspection, belief state, draft context, and
  events are hidden by default but easily revealed through visible labeled
  triggers
- users can inspect all active category questions without serial hunting by
  opening the question map or question-index surface
- dynamic category growth, branch movement, and convergence outcomes remain
  truthful and visible despite the more focused layout
- important changes are explained in the pulse bar, active question canvas, or
  question map; the user does not need to infer them from missing content
- websocket updates materially improve the focused-lobby experience without
  turning transport detail into the product story
- any warm prompt reuse stays per-session and never presents stale work as
  actionable truth

## Verification Plan

### Web

- session-page and workspace tests proving the default visible surface is the
  focused question canvas
- tests proving the question map can reveal all active question groups without
  requiring serial category entry
- tests proving belief state, draft, and events remain hidden until revealed but
  are still accessible and truthful when opened
- tests proving moved or resolved branches surface explanatory transition UI in
  the main lobby flow instead of only in secondary surfaces
- tests proving visible labeled triggers exist for all reveal surfaces and that
  the model does not depend on hidden gestures

### Manual

- open an active Socratic session and confirm the current question is the clear
  primary focus before any reveal surface is opened
- open the question map and confirm all active categories and question groups
  can be inspected without serial hunting
- answer a question that creates or moves work and confirm the change is
  explained inline and reflected in the map
- open the context shelf and confirm belief state, draft, and events are
  available without having competed for attention beforehand
- verify build readiness becomes obvious in the pulse bar and focused canvas
  when required work is complete

## Rollback And Fallback

- if the full focused-lobby reset is too large for one delivery pass, prioritize
  hiding the persistent side surfaces, establishing the focused question canvas,
  and introducing the question-map reveal surface first
- if the all-question reveal surface becomes too large, allow it to start as a
  drawer with grouped sections before promoting it to a larger overlay, but do
  not revert to serial category hunting
- if fine-grained realtime deltas are too risky, keep snapshot refresh as the
  transport baseline and preserve the focus-first lobby behavior with coarser
  updates

## Implementation Notes

- Implemented the bounded web pass of the focused-lobby reset in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx),
  [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx),
  and
  [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx).
- The delivery pass followed the spec's approved fallback path:
  it removed the permanently visible split-pane Socratic layout during active
  workspace use, established a dominant focused question canvas, introduced a
  revealable question-map overlay, and moved belief state, draft, transcript,
  and events into a hidden-by-default context shelf.
- Existing websocket snapshot and workspace contracts were sufficient for this
  pass, so backend contract expansion was not required to ship the primary
  product reset.
- Verification completed with:
  [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx),
  including direct assertions for the hidden-by-default context shelf and the
  inline focus-transition branch state,
  and `npx tsc --noEmit`.
- if warm question reuse adds correctness risk, remove it before weakening the
  focused-lobby model

## Open Questions

None blocking readiness.

This implemented slice is the bounded web fallback path from the original spec:
replace the split-pane lobby with a focused question canvas plus revealable map
and context surfaces while preserving the existing Planner category and
convergence contracts.
