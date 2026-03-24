# Socratic Lobby First-Reveal Preload Gate Spec

**Status:** implemented precursor  
**Date:** 2026-03-24  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Socratic Lobby Live Virtualized Document Spec](/home/thetu/planner/docs/socratic-lobby-live-virtualized-document-spec.md), [Socratic Hybrid Question Routing And Latency Spec](/home/thetu/planner/docs/socratic-hybrid-question-routing-and-latency-spec.md), [Socratic Lobby Document Chrome And Scroll De-escalation Spec](/home/thetu/planner/docs/socratic-lobby-document-chrome-and-scroll-de-escalation-spec.md)

## Problem & Intent

> Planning note (2026-03-24): this slice remains the record of the first-view
> preload gate work that landed on the continuous-document route. It no longer
> defines the active future-state architecture after the pivot to
> [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md).

The current lobby starts rendering too early.

Even when the route is technically functional, the first user view can show a
partially hydrated workspace with a few question groups, shifting labels, and
incremental reveal states that feel confusing rather than intentional. The
result is a product-level coherence failure:

- the user sees a busy partial document before the lobby is meaningfully ready
- the first paint feels like a live debug surface instead of a finished
  planning desk
- the user has to mentally distinguish between what is already known, what is
  still being generated, and what the system will eventually reveal

The product decision for this slice is explicit:

- prefer a short, honest loading gate over an early partial lobby reveal
- first reveal should feel substantially loaded and coherent
- after that first reveal, later updates may remain incremental

## User Outcome

After this slice:

- starting a Socratic session shows a focused loading state first, not a
  confusing partial lobby
- the live document does not render until it has crossed a meaningful preload
  threshold
- the first visible lobby state feels intentionally loaded rather than
  half-built
- once the lobby appears, known categories and questions are already available
  for local-fast browsing
- later branch changes and question insertions may still appear incrementally,
  but the first impression is coherent

## Locked Product Decision

The first meaningful lobby reveal must be gated by preload readiness.

The client must not render the full Socratic lobby document immediately on the
first `category_state` / `workspace_state` if the currently known content is
still too thin.

The default preload target for first reveal is:

- **at least 8 currently known question items**

The lobby may reveal earlier only when one of the explicit fallback rules in
this spec is met.

## In Scope

- add a dedicated first-reveal preload state for the Socratic lobby
- define what counts toward the initial preload threshold
- gate first lobby reveal on preload readiness instead of rendering the live
  document immediately
- keep already-known content locally available once the gate opens
- define timeout and low-availability fallbacks so the route cannot hang
  forever
- add truthful copy for the preload state
- add targeted verification for the reveal gate behavior

## Out Of Scope

- redesigning the steady-state live document after first reveal
- changing the left-index / right-desk architecture
- changing the backend question-authoring contract
- broad routing-model changes outside the initial reveal window

This slice may require bounded backend support for preload readiness metadata or
additional early question generation, but it does not reopen the overall
Socratic architecture.

## Product Contract

### 1. Initial reveal gate

Before the first meaningful render of the live Socratic lobby:

- show a dedicated loading surface instead of the full document
- do not render the continuous document desk until preload readiness is met

The initial loading surface should communicate:

- Planner is preparing the first working set of questions
- the lobby will open once a meaningful initial set is ready

The loading surface must feel intentional and calm, not like a blank or broken
screen.

### 2. Preload threshold

The default first-reveal threshold is:

- `8` currently known question items

Question items count toward the threshold when they are already locally
available for rendering in the document, including:

- active prompt items
- retained known questions
- locally known preview items that are intended to render immediately in the
  document

Category shells without actual question items do not satisfy the threshold.

### 3. Early-reveal fallback rules

The lobby may reveal before reaching `8` question items only when one of these
is true:

- the server has no more immediately preloadable question content available for
  the current state
- the interview reaches `build_ready`
- a bounded reveal timeout is hit

The reveal timeout must be explicit and truthful. Initial recommendation:

- `4s` soft target for normal first reveal
- `8s` hard fallback after which the best currently known lobby state may
  render even if the threshold is not met

The UI copy after timeout must remain truthful that the lobby opened with a
partial initial set.

### 4. Post-reveal behavior

After the first reveal:

- the lobby returns to the normal live incremental model
- later question/category insertions may render incrementally
- already-known content must remain local-fast and not regress into a second
  loading gate

This spec only gates the **first meaningful reveal**, not every subsequent
delta.

### 5. Local-fast requirement after reveal

Once the lobby is visible:

- browsing already-known categories and questions must remain immediate on the
  client
- the preload gate must not introduce a slower browsing model after reveal

### 6. Truthful observability and copy

The system must distinguish between:

- initial preload in progress
- normal post-reveal incremental generation
- partial reveal after timeout fallback

It must not label the first-view gate as ordinary `Preparing question` state.

## Design Constraints

- prefer a single coherent loading gate over showing partial category clutter
- do not turn the initial state into a generic spinner-only screen
- preserve the dense consultant-desk identity once the lobby reveals
- keep the first visible lobby state as stable as possible

## Touched Surfaces

Expected primary surfaces:

- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [planner-web/src/hooks/useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/components/SessionPulseBar.tsx](/home/thetu/planner/planner-web/src/components/SessionPulseBar.tsx)
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

Potential backend/support surfaces, only if needed:

- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)

Expected supporting tests:

- [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
- [planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx](/home/thetu/planner/planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx)
- targeted component tests if new preload-state components are introduced

## Acceptance Criteria

1. Starting a Socratic session no longer renders the live lobby immediately on
   the first thin workspace/category payload.
2. The route stays on a dedicated initial loading surface until either:
   - at least 8 question items are locally available, or
   - an explicit early-reveal fallback rule is met.
3. The first rendered lobby state feels substantially loaded rather than like a
   partial incremental debug view.
4. After reveal, browsing already-known categories and questions remains
   immediate on the client.
5. If the reveal timeout fallback is hit, the copy remains truthful that the
   initial set is partial.
6. The preload gate does not regress build-ready or existing resume flows.

## Verification Plan

### Automated

- add or update tests proving that thin early workspace/category payloads keep
  the route on the preload screen
- add or update tests proving the lobby reveals once the threshold is met
- add or update tests proving timeout/low-availability fallback does reveal the
  best-known lobby state
- rerun:
  - `npm --prefix planner-web test -- src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
  - `npm --prefix planner-web run build`

### Manual

- start a fresh Socratic session and verify the first screen is the preload
  state, not a partial live document
- verify the lobby appears only after a meaningful working set is ready
- verify the first reveal feels coherent and substantially loaded
- verify post-reveal category browsing is still immediate
- verify the timeout fallback is truthful if the threshold is not reached in
  time

## Implementation Sync

Implemented on `planner-web`:

- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
  now arms a dedicated first-reveal preload gate for locally started and
  restarted Socratic interviews, hydrates the document graph before first
  reveal, and keeps the lobby hidden until either the initial known-question
  threshold is met or the hard timeout reveals the best currently known lobby
  state
- [planner-web/src/stores/socraticDocumentStore.ts](/home/thetu/planner/planner-web/src/stores/socraticDocumentStore.ts)
  now exposes a known-question-count selector so the gate can react to locally
  retained document content without broad store reads
- [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
  now covers the three core reveal paths:
  thin first payload stays gated, sufficiently loaded first payload reveals
  immediately, and the hard-timeout fallback reveals a truthful partial desk

Implementation notes:

- this first slice is intentionally client-inferred; it does not add backend
  `no_more_initial_preload` metadata yet
- the gate is currently scoped to locally started/restarted interviews in this
  page lifecycle, which preserves existing resume flows while still fixing the
  bad first-view experience called out in the spec

Verification rerun:

- `npm --prefix planner-web test -- src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm --prefix planner-web run build`

## Rollback & Fallback

- if backend preload expansion is not ready, the first implementation may gate
  only on currently known client-side question count plus timeout, while
  remaining truthful that the lobby opened with a partial set
- if `8` proves too strict in real sessions, the threshold may be tuned later,
  but the preload-gate model itself should remain
- if the loading surface becomes too slow in practice, prefer a better preload
  strategy over reverting to immediate partial reveal

## Open Questions

- whether the backend should explicitly declare `no_more_initial_preload` or
  whether the first implementation should infer low-availability from the
  current workspace/prompt state is left to implementation

This is not a blocker for readiness because the client-side gate plus timeout
fallback can land first if needed.

## Readiness Judgment

This spec is ready for the remaining closeout work, not a new broad
implementation pass.

The bounded client-side preload gate is now implemented and automatically
verified. The remaining honest work is manual browser validation of the first
reveal feel, plus a later product decision on whether backend preload
availability metadata is worth adding beyond this client-inferred slice.
