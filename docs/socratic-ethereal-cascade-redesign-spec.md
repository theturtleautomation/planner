# Socratic Ethereal Cascade Redesign Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Socratic Ethereal Cascade Remediation Spec](/home/thetu/planner/docs/socratic-ethereal-cascade-remediation-spec.md)

> Planning note (2026-03-23): this spec remains the canonical record of the
> currently implemented Socratic Lobby baseline. The selected future
> replacement direction is now captured in
> [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md).
> Do not treat this document as the chosen next product target for future lobby
> redesign work.

## Purpose

This is the canonical execution-bounding spec for the Socratic Lobby redesign.
It consolidates the narrower March 22 session-page redesign concept slices into
one implementation-ready artifact for the focused lobby and question workspace.

## Implementation Sync

The initial bounded delivery landed on `planner-web` on 2026-03-22, and the
same-day remediation slice closed the material audit gaps that kept this spec
from an implemented status.

Delivered baseline in the redesign slice:

- integrated an ambient `SessionPulseBar` into the focused lobby path
- replaced the boxed `SocraticWorkspace` stack with a typography-first cascade
  built from the existing workspace and prompt payloads
- added thread-of-thought ancestor actions, terminal prompt emphasis, branch
  review states, and build-ready hero treatment
- kept belief state, draft, transcript, and events behind the existing Context
  Shelf instead of a visible companion pane
- removed the dead focused-lobby question-map toggle from the active surface

Baseline verification rerun for the delivered portion:

- `npm test -- src/pages/__tests__/SessionPage.test.tsx`
- `npm test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm test -- src/components/__tests__/PromptBatchPanel.test.tsx`
- `npm run build`

Remediation delivered in the follow-on slice:

- category-only interview states now remain inside the focused cascade even
  when only category state is present
- terminal-question mode now applies stronger centering and answer-surface
  focus handling
- older path history is compressed into a quieter `Earlier turns` treatment
  while preserving ancestor return
- the verification surface now includes targeted
  [SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
  coverage plus terminal autofocus assertions

Remediation verification rerun:

- `npm test -- src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx`
- `npm run build`

Manual verification expectations from this spec remain listed below and should
be rerun when a browser-based UX pass is needed, but the code and targeted
automated coverage are now aligned with the canonical redesign.

## Problem & Intent

The Socratic Lobby is the core product surface, but the current session UI still
reads like application chrome wrapped around a planning flow. It splits attention
between category navigation, active questions, readiness messaging, transcript,
draft context, and operational events. Deep recursive category paths are
supported by the server, but the default reading path still feels closer to a
dashboard than a calm planning room.

The redesign intent is to turn the Socratic Lobby into a threshold of focused
inquiry: a quiet, typography-first environment where the user reads, chooses,
and answers one thread of thought at a time. The interface must support deep
recursive drill-down without visual exhaustion, keep the active work at the
strict focal point, and move operational status into background layers that are
only revealed intentionally.

## Core Thesis

The Socratic Lobby should feel less like software administration and more like a
well-lit room for structured inquiry. Categories and questions are not separate
UI modes. They are the same recursive conversational primitive presented with
different depth and state.

The interface should communicate progression by spatial movement, typographic
weight, and controlled motion instead of boxes, borders, or persistent side
navigation.

## Chosen Direction

Implement the **Ethereal Cascade**: a typography-first, spatial column model
derived from a modern Miller Column pattern.

This direction is selected because it:

- scales to effectively unbounded recursive depth without breaking reading order
- keeps the experience word-first instead of diagram-first
- maps naturally to the user mental model of moving deeper into a topic and
  stepping back through prior reasoning
- can be built safely with standard 2D browser layout and motion primitives

Rejected alternatives:

- **Astrolabe / radial metaphor:** visually novel, but hostile to word-heavy
  deep reading and poor at 25-level recursive depth
- **Constellation / node graph:** useful for overview, but weak for linear
  decision making and easy to lose orientation within

## User Outcome

The user enters the lobby and sees one clear list of high-level categories in a
calm, editorial layout. Choosing a node glides the interface forward. The chosen
node becomes context, its children take over the active horizon, and the
linguistic trail of prior choices remains quietly visible as a readable thread
of thought.

When the user reaches a terminal question, the environment deliberately quiets
down. Competing UI recedes, the question moves to the center of the viewport,
and the answer interaction becomes a direct one-to-one exchange with the system.

## Scope Boundaries

### In Scope

- redesign of the focused lobby portion of
  `planner-web/src/pages/SessionPage.tsx`
- redesign of `planner-web/src/components/SocraticWorkspace.tsx` around the
  Ethereal Cascade model
- replacement of boxed card/list treatment with typography-led interactive
  statements and whitespace-defined targets
- a readable "Thread of Thought" that summarizes deeper history without
  conventional `>` breadcrumb UI
- terminal question isolation inside the active workspace
- relocation of context, transcript, draft, and event surfaces into explicit
  revealable background layers instead of default co-equal panes
- subtle ambient status affordances for readiness and background activity
- safe 2D motion for lateral progression, branch return, and dynamic insertion
- responsive behavior for mobile, tablet, and desktop while preserving the same
  interaction model
- keyboard and focus behavior for recursive navigation and drawer invocation

### Out of Scope

- any server-side change to Socratic payload structure, generation logic, or
  websocket message semantics
- changes to project/session routing outside the session page
- redesign of non-Socratic routes
- true 3D `translateZ` stacking, blur-depth illusions tied to perspective, or
  any effect likely to harm text rendering or focus order
- exposing logs, events, or system status in the main reading path by default

## Existing Constraints

- the server remains the source of truth for category hierarchy, active prompt,
  workspace grouping, branch notices, and build readiness
- the client already receives the required data through
  `SocraticWorkspaceSnapshot`, `SocraticCategorySnapshot`, and `PromptEnvelope`
- `planner-web` does not currently ship `framer-motion`; this spec must remain
  buildable with existing dependencies and CSS/React primitives
- current session behavior already supports focused category changes,
  `pendingCategoryId`, branch notices, build-ready state, and context-shelf
  toggles; the redesign must reinterpret those surfaces, not invent new backend
  truth

## Data Model & Interaction Contract

### Canonical State Inputs

The redesign uses the existing frontend state surfaces:

- `workspace.category_snapshot`
- `workspace.groups`
- `workspace.focused_category_id`
- `workspace.branch_notice`
- `currentPrompt`
- `pendingCategoryId`
- `currentStep`
- unread draft / event counts already used by the Context Shelf and pulse bar

### Client-Derived View Model

The implementation may derive a local cascade model, but it must be computed
from server-authored state rather than persisted as independent product truth.

Required derivations:

- **active focus id:** `pendingCategoryId`, otherwise
  `workspace.focused_category_id`, otherwise the current prompt category, then
  existing focused group fallback
- **active path:** prefer `currentPrompt.category_path`; otherwise use
  `workspace.category_snapshot.active_category_path`
- **active horizon:** the currently focusable list or terminal question
- **immediate parent:** the category directly above the active focus in the path
- **thread of thought:** a compressed, readable rendering of older path entries
  beyond the immediate parent

### Contract Rules

1. The UI must not invent categories, prompts, or branch relationships that were
   not authored by the server.
2. Recursive depth must use one interaction primitive at every level.
3. The client may compress history visually, but it may not discard the active
   path required to return to an ancestor.
4. Focus transitions must preserve keyboard reachability and visible focus.

## Page Architecture

The session lobby is divided into three zones:

### Primary: Active Horizon

The central reading path holds the active category list or terminal question.
This zone has the highest contrast, largest type, and full opacity.

### Secondary: Thread of Thought

The immediate parent and the compressed language trail of prior choices sit
above the active horizon or slightly recessed to the left. This zone maintains
orientation without competing with the active work.

### Background: Ambient Periphery

Context, transcript, events, readiness detail, and low-level operational status
live behind explicit invocation only. They should appear in drawers, shelves, or
edge-anchored layers and never remain open by default in the main reading path.

## Interaction Model

### Recursive Node Presentation

- categories and question-entry nodes render as unboxed typographic statements
- separation comes from whitespace, scale, and opacity rather than borders or
  cards
- click and keyboard selection must produce an unmistakable state change before
  the cascade moves forward

### Drill-Down

- selecting a node moves the interface laterally into the next depth
- the selected node becomes contextual anchor material
- children or the terminal question animate into the active horizon

### Move Up or Sideways

- selecting an ancestor from the Thread of Thought restores that historical
  state and collapses deeper branches
- selecting a sibling in the active column changes direction without changing
  the underlying interaction primitive

### Dynamic Injection

- when the server synthesizes new categories or prompt groups, new content must
  be inserted with measured expansion and fade-in rather than snap replacement
- the user’s reading position and orientation must remain stable during
  `pendingCategoryId` or workspace refresh transitions

### Terminal Question Reveal

When the active branch reaches a terminal prompt:

- parent context, sibling nodes, and ambient controls recede almost completely
- the terminal question shifts to the center of the viewport
- the answer surface auto-focuses if permitted by browser rules and accessibility
- the interaction reads as one uninterrupted question-and-answer moment

## Visual System

### Typography

- text is the interface
- use an editorial serif and sans pairing already compatible with the repo’s
  design system direction, or introduce it through route-local styles without
  forcing a site-wide typography migration
- terminal questions should use the most dramatic scale and strongest contrast
- list nodes should remain highly readable at all responsive breakpoints

### Color

- palette stays severely restricted and low-glare
- primary reading surface uses warm light backgrounds or their dark-mode
  equivalent if the app theme already supports it
- active content receives maximum contrast; historical or inactive content
  recedes into quieter grays
- avoid corporate blue links, glossy highlights, neon accents, and dashboard
  color coding

### Material

- flat, matte surfaces only
- depth is expressed with position, opacity, and slight scale change
- no heavy borders, boxed cards, glassmorphism-heavy main canvas, or decorative
  drop shadows in the active reading path

### Motion

- motion communicates weight and spatial relationship
- preferred implementation is CSS transitions with spring-like timing curves or
  small route-local motion helpers
- introducing a new animation dependency is optional, not required
- no sudden cuts for drill-down or ancestor return

## Information Architecture Rules

1. **Recursive Uniformity:** the same interaction primitive is used at every
   depth.
2. **Strict Focal Point:** only the active horizon stays at full opacity and
   full typographic emphasis.
3. **Contextual Culling:** no more than two hierarchy levels remain in strong
   view at once; older history compresses into the Thread of Thought.
4. **Absolute Backgrounding:** logs, events, and system state never render as a
   default main-column companion pane.
5. **Graceful Injection:** newly available categories or questions animate into
   place without replacing existing content abruptly.
6. **Terminal Isolation:** a final question must clear most structural context
   from the screen.
7. **Interaction Legibility:** selection, branch movement, and focus change must
   be immediately obvious.

## Contracts & Touched Surfaces

### Primary Route and Component Surfaces

- `planner-web/src/pages/SessionPage.tsx`
  - remove the effective split-pane mental model from the focused lobby branch
  - keep transcript, draft, belief state, and events available only through the
    revealable context surface
  - preserve existing session workflow truth, websocket wiring, and action
    handlers
- `planner-web/src/components/SocraticWorkspace.tsx`
  - replace the boxed category stack with the Ethereal Cascade composition
  - derive display state from existing workspace and prompt props
  - implement thread-of-thought, active horizon, terminal isolation, and branch
    return behavior
- `planner-web/src/components/SessionPulseBar.tsx`
  - reduce visual dominance so it behaves like an ambient trigger/status strip
    rather than a control-heavy toolbar
  - keep question-map and context invocation truthful and accessible
- `planner-web/src/index.css`
  - add or refine route-safe tokens/classes for typography, spacing, motion, and
    off-canvas behavior needed by the session page

### Likely Secondary Surfaces

- `planner-web/src/components/CategoryNavigator.tsx`
  - may be retired from the focused lobby path or preserved only as a bounded
    fallback during rollout
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`
- `planner-web/src/components/__tests__/SocraticWorkspace.test.tsx` if added
- `planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx` if display
  assumptions change around workspace focus handling

### Explicit Non-Contracts

- no change to `planner-web/src/types.ts` payload shapes is required
- no backend API or websocket message additions are required for this slice

## Acceptance Criteria

1. The focused lobby no longer presents the active work as a split between a
   main question pane and a permanently visible operational side pane.
2. The primary reading path renders as a calm, typography-first cascade rather
   than boxed cards or dashboard chrome.
3. Recursive depth greater than five levels remains navigable without loss of
   orientation or horizontal overflow chaos.
4. The active horizon is the only region at full visual emphasis; immediate
   parent context is visible but recessed, and older history is compressed into
   the Thread of Thought.
5. Clicking or keyboard-selecting a node produces an unmistakable selection
   change, then a lateral or depth-consistent transition into the next state.
6. Returning to an ancestor through the Thread of Thought restores that earlier
   state and collapses abandoned deeper branches.
7. When `pendingCategoryId` or equivalent preparation state is active, the UI
   shows localized preparation feedback without dumping the user into a new page
   or exposing noisy operations.
8. When `workspace.branch_notice` applies, the UI communicates branch drift or
   server focus changes inside the cascade without collapsing the mental model.
9. When a terminal question is active, sibling navigation and most ambient
   context recede, the prompt is centered, and the answer surface becomes the
   dominant interaction.
10. Logs, events, draft context, and belief state are reachable through explicit
    invocation only and are not visible by default.
11. The redesign works on mobile, tablet, and desktop with the same underlying
    recursive model and without trapping keyboard focus.
12. The implementation does not require backend contract changes or risky 3D
    transform effects.

## Verification Plan

### Automated

- run targeted `vitest` coverage for `SessionPage`, `SocraticWorkspace`, and
  websocket state hydration affected by the redesign
- add or update tests for:
  - active-focus derivation from `pendingCategoryId`, `focused_category_id`, and
    prompt path
  - ancestor return behavior through the thread-of-thought interaction
  - terminal question isolation and build-ready CTA visibility
  - context shelf remaining hidden by default and opening by explicit action
  - branch-notice and preparing-state rendering

### Manual

1. Open a session with multiple recursive category levels and verify forward
   drill-down, sibling changes, and ancestor return.
2. Verify the active node or terminal prompt is the only fully emphasized area.
3. Submit answers that trigger preparation of a new branch and confirm the UI
   remains anchored while new content is introduced.
4. Trigger a branch notice and confirm the message appears inside the cascade
   model rather than as a separate dashboard panel.
5. Reach a terminal question and confirm that context recedes, the prompt
   centers, and input focus remains usable.
6. Open and close the Context Shelf on desktop and mobile and confirm the main
   reading path remains uninterrupted.
7. Verify keyboard-only navigation for node selection, ancestor return, drawer
   invocation, and prompt answer submission.

## Rollback / Fallback

Primary fallback path:

- keep the existing server-authored workspace model and session handlers intact
- if the full compressed-history cascade is unstable, ship a reduced version
  that keeps only the active horizon plus a simplified thread-of-thought line
- if terminal isolation proves too disruptive on small screens, preserve the
  same centered prompt behavior with reduced surrounding fade instead of a full
  dramatic clear-out
- if motion polish causes layout regressions, retain the new information
  hierarchy while falling back to simpler opacity/translate transitions

Implementation must not depend on irreversible backend changes, so rollback is a
frontend-only revert to the prior session layout if needed.

## Open Questions

None blocking readiness.

Implementation choices left intentionally flexible:

- whether the motion layer is pure CSS or a narrowly introduced helper
- whether `CategoryNavigator.tsx` is fully removed from the focused lobby path
  or retained as a temporary fallback during rollout

## Readiness Judgment

This spec is ready for bounded implementation.

Reasoning:

- the product intent, user outcome, scope boundaries, and non-goals are now
  explicit
- the redesign is mapped to the current frontend route and component surfaces
- the state model is grounded in existing server-authored payloads
- verification and fallback paths are concrete
- remaining choices are implementation details, not material product blockers
