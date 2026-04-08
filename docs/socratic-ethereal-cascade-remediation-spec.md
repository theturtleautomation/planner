# Socratic Ethereal Cascade Remediation Spec

**Status:** Implemented  
**Date:** 2026-03-22  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Related Planning:** [Socratic Ethereal Cascade Redesign Spec](/home/thetu/planner/docs/socratic-ethereal-cascade-redesign-spec.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md)  
**Source Audit:** 2026-03-22 implementation audit against the redesign spec, current `planner-web` code, and named verification surfaces

## Objective

Close the audited implementation gaps between the canonical Ethereal Cascade
redesign spec and the current `planner-web` session lobby so the redesign can
be treated as actually complete rather than partially delivered.

This remediation slice is intentionally bounded:

- keep the existing frontend data contracts and websocket model
- finish the missing focused-lobby behaviors the audit found off-spec
- strengthen the verification surface so the completion claim is evidence-based

It is not a second Socratic redesign pass.
It does not reopen backend payloads, route architecture outside the session
page, or broader visual experimentation.

## Audit Baseline

The audit found four material trust gaps:

- category-only interviewing states can still fall back to the legacy
  split-pane layout in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- terminal-question treatment is only partial:
  stronger typography and sibling recession exist, but explicit centering and
  answer-surface focus handling do not
- the Thread of Thought supports ancestor return, but older history is not
  compressed the way the redesign spec requires
- the named verification evidence is real but thinner than the redesign spec
  claims:
  there is no dedicated
  [SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
  coverage yet, and several acceptance criteria still rely on manual inference

## Scope

### In scope

- focused-lobby state routing in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- focused-lobby composition and state rendering in
  [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- terminal question answer-surface behavior in
  [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
  and
  [PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
- route-local visual treatment in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- focused-lobby state hydration or pending-state behavior in
  [useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)
  only if needed to support truthful rendering of existing server-authored
  state
- automated verification in:
  - [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
  - [useSocraticWebSocket.test.tsx](/home/thetu/planner/planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx)
  - [PromptBatchPanel.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/PromptBatchPanel.test.tsx)
  - [SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
    if added in this slice

### Out of scope

- backend API or websocket contract changes
- non-Socratic route redesign
- a new category-map product surface or a separate navigation mode
- broader typography-system migration outside the focused lobby
- speculative design polish not tied to an audited gap

## Current-State Evidence

- [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  now renders the focused-lobby shell when a workspace is available, but still
  falls back to the legacy `split-pane` interview layout when `displayWorkspace`
  is null.
- [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
  already derives active focus from `pendingCategoryId`, workspace focus,
  prompt origin, and focused-group fallback, and it already renders a Thread of
  Thought plus branch-review and build-ready states.
- [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
  and
  [PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
  currently provide no explicit autofocus or terminal-specific focus behavior.
- [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
  covers ancestor return, question-map removal on the active focused path,
  branch-review controls, and context-shelf gating, but it does not yet prove
  category-only cascade rendering, build-ready hero visibility, or terminal
  centering/focus behavior.
- [useSocraticWebSocket.test.tsx](/home/thetu/planner/planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx)
  proves focus derivation and pending-state clearing, which makes the missing
  UI coverage more clearly a rendering gap rather than a transport gap.

## Requirements

### Focused-lobby state completeness

- interviewing states that currently have only category or prompt context must
  remain inside the Ethereal Cascade model instead of falling back to the
  legacy split-pane interview layout
- the focused-lobby branch must remain the only interview rendering model once
  the session has entered the redesign path
- transcript, draft, belief state, and events must remain explicit-invocation
  surfaces only on that path

### Thread-of-thought compression

- the Thread of Thought must preserve ancestor return behavior
- older path history must compress once the path grows beyond the immediate
  parent context instead of always rendering the full ancestor chain at equal
  weight
- the active branch and immediate parent must remain visually legible without
  turning deep paths into horizontal overflow noise

### Terminal-question completion

- terminal-question mode must clearly center or otherwise spatially privilege
  the active prompt beyond the current large-type treatment
- sibling nodes and nonessential chrome must recede further than they do now
  while keeping the session truthful and navigable
- the answer surface must receive explicit focus handling when browser and
  accessibility rules allow

### Verification hardening

- the redesign may only return to a fully implemented claim once the targeted
  frontend evidence covers:
  - category-only focused-lobby rendering
  - ancestor return through the Thread of Thought
  - preparing-state rendering
  - branch-notice rendering
  - build-ready hero visibility
  - terminal-question emphasis and answer-surface focus behavior
- the verification notes must cite only the commands actually rerun in this
  slice

## Contracts And Touched Surfaces

- no change to
  [types.ts](/home/thetu/planner/planner-web/src/types.ts)
  payload shapes is required
- no backend routes, websocket message types, or schema additions are allowed in
  this slice
- the remediation may derive local view state from:
  `currentPrompt`,
  `pendingCategoryId`,
  `workspace.focused_category_id`,
  `workspace.category_snapshot`,
  and existing unread draft or event counts
- touched implementation surfaces are bounded to:
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx),
  [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx),
  [SessionPulseBar.tsx](/home/thetu/planner/planner-web/src/components/SessionPulseBar.tsx),
  [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx),
  [PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx),
  [useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts),
  and
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

## Acceptance Criteria

- category-only interview states no longer render the legacy split-pane layout
  and instead stay within the focused-lobby cascade model
- the Thread of Thought still supports ancestor return while compressing older
  history more aggressively than the current full inline chain
- terminal-question mode gives the prompt a more clearly isolated spatial
  treatment than the current implementation and provides explicit answer-surface
  focus handling when allowed
- context surfaces remain hidden by default and reachable only through explicit
  invocation in the focused-lobby path
- the redesign still uses only the existing frontend contracts and CSS/React
  primitives
- the completion claim for the Ethereal Cascade slice is backed by rerun tests
  that directly cover the audited gaps

## Verification Plan

### Automated

- `npm test -- src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx`
- `npm run build`

If
[SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
is not added, the remaining targeted assertions must be absorbed explicitly into
[SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
and the verification note must say so directly.

### Manual

Confirm after implementation that:

- a category-only or prompt-hydrated interview state stays inside the cascade
  instead of reverting to the split pane
- deep recursive paths still read cleanly on desktop and mobile after history
  compression
- terminal-question mode centers or clearly isolates the active prompt and the
  first answer control is reachable immediately
- keyboard-only navigation still works for ancestor return, category selection,
  context-shelf invocation, and prompt submission

## Rollback And Fallback

- if full history compression proves unstable, ship a smaller compression rule
  that still distinguishes older path history from the immediate parent instead
  of keeping the current fully expanded chain
- if browser-safe autofocus proves unreliable, keep the stronger terminal layout
  treatment and fall back to deterministic focus styling plus manual tab reach,
  but do not claim autofocus was delivered
- if category-only cascade rendering exposes a deeper model gap, prefer keeping
  the user inside one focused-lobby composition with a reduced state rather than
  preserving the legacy split-pane interview branch

## Open Questions

None blocking readiness.

## Implementation Sync

The remediation slice landed on `planner-web` on 2026-03-22.

Delivered in this slice:

- category-only interview state hydration now stays inside the Ethereal
  Cascade instead of dropping back to the legacy split-pane interview layout
- the Thread of Thought now compresses older history into a quieter
  `Earlier turns` row while preserving immediate-parent return behavior
- terminal-question mode now applies stronger centering and answer-surface
  focus handling through the existing prompt components
- targeted frontend verification was expanded with dedicated
  [SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
  coverage plus terminal autofocus assertions in
  [PromptBatchPanel.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/PromptBatchPanel.test.tsx)

Touched implementation surfaces:

- [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [PromptBatchPanel.tsx](/home/thetu/planner/planner-web/src/components/PromptBatchPanel.tsx)
- [PromptCard.tsx](/home/thetu/planner/planner-web/src/components/PromptCard.tsx)
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- [SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
- [PromptBatchPanel.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/PromptBatchPanel.test.tsx)
- [SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)

Verification rerun in this slice:

- `npm test -- src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx`
- `npm run build`

Manual verification from the verification plan was not rerun in this delivery
slice, so this spec is implemented rather than promoted to a stronger
completion claim.
