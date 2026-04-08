# Socratic Question Canvas Alignment And Visual Refinement Spec

**Status:** implemented  
**Date:** 2026-03-23  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Socratic Ethereal Cascade Redesign Spec](/home/thetu/planner/docs/socratic-ethereal-cascade-redesign-spec.md), [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md), user-provided critique of the live question canvas dated 2026-03-23

## Problem & Intent

The focused lobby now uses the correct product model, but the question canvas
still feels uneven in practice:

- some question states still fight the dominant left-aligned reading flow
- the main prompt headline is too large for a conversational workspace
- text measure is too narrow in places, producing awkward wraps
- spacing inside the active question section is too flat
- non-terminal states such as `preparing` and branch review still feel dull and
  repetitive
- on short windows the active prompt action area can become hard or impossible
  to reach

This slice is a bounded visual and interaction refinement of the question
canvas. It does not reopen the lobby information architecture.

## User Outcome

After this slice:

- the active question reads as one coherent left-aligned conversational column
- prompt typography feels premium and intentional without turning into a hero
  banner
- question, support text, answer inputs, and actions follow a clear spacing
  rhythm
- `preparing`, branch review, and active question states feel distinct and
  readable instead of visually interchangeable
- the user can always reach the submit and completion actions, even on shorter
  viewports

## Locked Decisions

- the focused-lobby model stays intact; this is not a return to a permanent
  sidebar map
- the question canvas should remain flush-left on desktop rather than centering
  only the terminal state
- prompt typography must use semantic tokens and component-level caps rather
  than arbitrary oversized one-off clamps
- visual refinement must preserve readability and calm, not turn the lobby into
  a marketing splash
- action reachability is a product requirement, not optional polish
- the implementation should prefer extracting reusable Socratic component
  classes or tokens over expanding the existing inline-style footprint in the
  prompt components

## In Scope

- unify question-canvas alignment across terminal, preparing, branch-review,
  and build-ready states
- reduce question headline scale and tighten the typography hierarchy
- widen prompt/support text measure where current wrapping is premature
- introduce stepped spacing rhythm between:
  - kicker
  - question
  - supporting text
  - answer area
  - footer actions
- define semantic/component tokens for the Socratic canvas where needed
- differentiate active question, branch-review, and preparing surfaces more
  clearly
- fix the scroll/reachability trap so prompt footer actions are always
  reachable on short viewports
- tighten confusing or mechanical microcopy only if needed to support the
  visual hierarchy

## Out Of Scope

- changing the websocket or server-authored prompt contract
- redesigning the question map or context shelf IA
- broad restyling of unrelated routes
- model-routing or backend latency changes
- introducing a large new animation system

## Current-State Evidence

- the focused lobby and Ethereal Cascade are already the active product model in
  [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  and
  [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- the current visual treatment still relies heavily on one-off Socratic CSS in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- the prompt form itself is rendered through
  [planner-web/src/components/PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
  and
  [planner-web/src/components/PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
- live user feedback in-session called out three concrete problems:
  - dull presentation
  - oversized text
  - action-footer reachability failure on smaller windows

## Proposed Design

### 1. Unified reading column

- keep the active question section, including terminal state, on one flush-left
  conversational line
- align question detail padding to the same container rhythm as the thread
  above it
- avoid state-specific centering that breaks the reading line

### 2. Typography remediation

- reduce the question headline to a conversational-display scale rather than a
  landing-page hero scale
- widen headline measure to reduce ugly early wraps
- preserve stronger editorial type for the question while using calmer body
  treatment for support text
- keep support/body text at a more generous line-height than the headline

### 3. Stepped spacing rhythm

Replace flat vertical spacing with grouped spacing:

- kicker to question: tight
- question to support: medium-tight
- support to answer/input region: spacious
- input region to footer actions: clear but not detached

### 4. State differentiation

- active question state should feel most direct and least ornamental
- branch-review state should look intentionally secondary but still useful
- preparing state should feel transient and active, not identical to a normal
  branch card
- build-ready state should remain calm and conclusive rather than noisy

### 5. Reachable footer actions

The prompt form must not trap the primary action below an unreachable clipped
region.

The implementation may satisfy this by any bounded combination of:

- making the focused-lobby main content own a truthful vertical scroll region
- reducing internal nested scroll traps inside the prompt batch
- using a sticky or anchored footer action row inside the prompt region
- constraining card-grid height differently on short viewports

The outcome is locked even if the exact mechanism is not:

- `Submit answered items`
- `Done - start building`
- any equivalent primary action

must remain reachable without broken or hidden scrolling.

## Design-System Constraints

- prefer semantic or component tokens over one-off raw values where the same
  rhythm is likely to repeat
- if `PromptBatchPanel.tsx` or `PromptCard.tsx` currently hold inline styles
  that materially block the new hierarchy, moving those specific styles into
  bounded Socratic classes is in scope
- do not broaden token work into a new multi-route design-system phase
- preserve accessible contrast and focus visibility
- use motion only where it helps orientation
- preserve the established focused-lobby visual language rather than replacing
  it with a different layout ideology

## Contracts & Touched Surfaces

Expected primary files:

- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/components/PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
- [planner-web/src/components/PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)

Expected supporting files, only if needed:

- [planner-web/src/components/__tests__/PromptBatchPanel.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/PromptBatchPanel.test.tsx)
- [planner-web/src/components/__tests__/SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
- [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)

## Acceptance Criteria

1. The active question section reads as a single left-aligned conversational
   column across the major focused-lobby states.
2. The main question headline is visibly smaller and calmer than the current
   oversized treatment.
3. Prompt/support text wraps at a more natural measure and no longer collapses
   into premature narrow lines.
4. Active question, branch-review, and preparing states are visually distinct.
5. The prompt action/footer controls remain reachable on short viewports and do
   not disappear into an unscrollable trap.
6. The refinement improves perceived quality without reintroducing a noisy
   split-pane or dashboard feel.

## Verification Plan

### Automated

- add or update targeted tests for prompt action reachability behavior if the
  implementation changes DOM structure or sticky/footer semantics
- rerun the focused-lobby frontend coverage:
  - `npm --prefix planner-web test -- src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx`
- run `npm --prefix planner-web run build`

### Manual

- verify the active question canvas at desktop width and confirm the reading
  line stays flush-left
- verify the prompt headline no longer dominates like a hero banner
- verify `preparing`, branch review, and active question feel clearly
  differentiated
- verify on a short viewport that the prompt action row remains reachable and
  usable
- verify mobile and tablet widths do not regress the focused-lobby reading flow

## Rollback & Fallback

- if semantic token extraction broadens too far, localize the changes to the
  Socratic component classes and keep the design-system impact bounded
- if a sticky footer harms readability on mobile, prefer a truthful scroll
  container over forcing stickiness
- if one state treatment becomes too ornamental, reduce decoration before
  reverting the whole refinement slice

## Open Questions

- whether the prompt footer should become truly sticky or simply remain always
  reachable through a better scroll-container model is left to implementation.
  The outcome is fixed; the mechanism is not.

## Implementation Sync

Implemented on 2026-03-23.

- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  now lets the focused lobby own a truthful vertical scroll region instead of
  leaving the prompt footer below a clipped shell.
- [planner-web/src/components/PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
  and [planner-web/src/components/PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
  moved the question form off the largest inline-style block into bounded
  Socratic component classes.
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
  now carries the prompt-batch/card hierarchy, smaller mobile question scale,
  stepped spacing, and a sticky footer action row inside the active prompt
  region.

Verification completed in this implementation pass:

- `npm --prefix planner-web test -- src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx`
- `npm --prefix planner-web test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm --prefix planner-web run build`

Not completed in this slice:

- the spec's manual browser checks for desktop, short viewport, tablet, and
  mobile visual quality were not rerun in this pass, so those remain manual QA
  follow-through rather than claimed evidence here.
