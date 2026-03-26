# Planner SolidStart Phase 28 Session Entry And Startup Product Flow Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md)  
**Related Planning:** [Planner SolidStart Phase 26 Socratic Runtime Truth Completion Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-26-socratic-runtime-truth-completion-remediation-spec.md), [Planner SolidStart Phase 27 New Session Startup Truth Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-27-new-session-startup-truth-remediation-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 repo inspection plus the live Phase 26 browser proof follow-up on the real Rust server/runtime

## 1. Executive Judgment

Phase 26 closed the runtime-truth thread, and Phase 27 captured the remaining
`/sessions/new` startup-truth bug. But the product-flow review around that path
showed the next execution slice should be slightly wider than Phase 27 alone.

There is still one must-fix startup contract problem and several adjacent entry
flow issues that are small enough to close in the same bounded slice:

- `/sessions/new` still creates a blank session, calls `POST /sessions/:id/socratic`,
  and navigates immediately, which can leave the route in false restart-only
  truth before any live runtime or checkpoint exists
- the split `createSession()` then `startSocratic()` flow can leave behind an
  orphan saved-empty session if the second request fails
- the product currently has multiple saved-brief entry paths that do not yet
  converge on one explicit startup contract
- the new-session route still lacks a truthful startup transition and a clean
  retry story when startup fails early

Phase 28 therefore absorbs the Phase 27 startup-truth fix and closes the nearby
product-flow gaps that are directly attached to the same user journey.

## 2. User Outcome

After Phase 28:

- starting a session from `/sessions/new` follows the same startup truth as any
  other saved-brief session
- the user never lands on `Analysis needs a restart` immediately after just
  starting a session unless a real startup failure occurred
- the new-session flow no longer risks leaving behind a blank orphan session if
  the second startup call fails
- startup progress is visible and truthful between brief submission and the
  first bank reveal
- if startup genuinely fails, the user gets a truthful retry path that keeps
  the saved brief intact
- saved-brief session entry behavior is consistent enough that future product
  flows do not need route-specific heuristics to reach the Socratic workspace

## 3. Problems To Solve

### 3.1 `/sessions/new` still lands on false restart-only truth

Today the route:

1. creates a session
2. posts the brief through `POST /sessions/:id/socratic`
3. navigates to `/sessions/:id`

That backend call still has enough semantic drift that the route can classify
the session as restart-only before any live runtime or checkpoint exists.

### 3.2 Session creation and startup intent are split across two requests

The current product flow makes one request to create a blank session and a
second request to attach the brief/startup intent. If the second request fails,
the first session still exists and the user remains on `/sessions/new`.

That is truthful at the transport level, but it is not a clean product flow.
The user attempted to start a saved-brief analysis session, not to create an
empty abandoned record.

### 3.3 Entry paths still do not share one explicit saved-brief contract

Project-scoped session creation with a description and `/sessions/new` both
represent the same user intent: "I have a brief; start the Socratic analysis."

The product should not require one entry path to behave as waiting/startup
truth while another relies on restart-only recovery language for the same
pre-first-reveal state.

### 3.4 The route lacks a first-class startup transition

The current `/sessions/new` page changes the button label to `Starting…`, but
the meaningful product transition happens only after navigation. There is no
explicit, durable startup handoff state that connects:

- brief submission
- session creation
- startup intent persistence
- websocket open
- bank-first first reveal

### 3.5 Early startup failure recovery is still more technical than product-shaped

The session route has retry and restart controls, but the new-session product
flow does not yet define the truthful recovery path for:

- startup intent saved but websocket/open handshake failed
- backend startup failed before first reveal
- reload during startup

The product should preserve the saved brief and guide the user back into the
same startup contract rather than dropping into ambiguous recovery language.

## 4. Product And Technical Decision

Phase 28 chooses one strict product-flow direction:

### 4.1 One saved-brief startup contract across entry points

If a session has a saved brief but no live runtime, no prompt bank, and no
checkpoint yet, it belongs to the waiting/startup truth family.

That must hold regardless of whether the session was started from:

- `/sessions/new`
- a project-scoped session creation action
- any future saved-brief entry point

### 4.2 The preferred fix remains backend-owned startup truth

Phase 28 keeps the Phase 27 decision:

- `POST /sessions/:id/socratic` or its replacement contract must seed startup
  intent, not false interviewing/restart-only truth
- route behavior should continue to follow backend-computed startup truth, not
  accumulate special-case client heuristics

### 4.3 Saved-brief creation should become one explicit product contract

This slice should converge on one of these backend-authored shapes:

- a single create-with-brief session endpoint
- or a two-step implementation that is still exposed to the client as one
  explicit saved-brief startup contract and does not leave orphan empty
  sessions as the visible product outcome

The implementation choice is open, but the product contract is fixed:
starting analysis from a brief is one user action with one truthful result.

### 4.4 Startup transition must be visible and truthful

The product should expose a small but explicit startup transition between form
submission and first reveal. That transition must reflect real backend state,
not synthetic optimistic wording.

### 4.5 Early failure recovery must preserve the brief

If startup fails before first reveal:

- the saved brief remains intact
- the route offers a truthful retry action
- the user does not have to retype the description unless they explicitly want
  to edit it

## 5. Scope

### In Scope

- everything required by Phase 27 to fix the `/sessions/new` startup-truth bug
- converging saved-brief creation/startup on one explicit product contract
- removing or hiding the blank orphan-session outcome from the normal
  `/sessions/new` failure path
- adding a visible, truthful startup transition for the new-session path
- defining and verifying the early-failure retry path that preserves the saved
  brief
- verifying first-open and reload behavior on the real `/sessions/new` product
  path

### Out Of Scope

- renaming `start_socratic`
- broad session-route redesign beyond startup flow and early recovery
- changing the bank-first runtime contract selected in Phase 26
- redesigning project/session IA
- broader project creation or project ownership workflow changes

## 6. Must-Fix Contract

### 6.1 `/sessions/new` may not land in restart-only truth before real failure

Required behavior:

- after the user submits the brief, the product must not render restart-only
  messaging unless the live startup path actually failed after beginning
- saved brief plus no runtime plus no checkpoint must compute to startup truth,
  not restart-only truth

### 6.2 Saved-brief startup must not expose orphan blank-session outcome

Required behavior:

- a failed second-step startup call must not leave the user with a silent blank
  session artifact as the visible product result of "Start session"
- if the implementation still uses two backend operations internally, the
  product flow must either roll back, complete the session setup truthfully, or
  recover in place without presenting an abandoned empty session as success

### 6.3 Startup transition must remain inside one truthful flow

Required behavior:

- the user sees a coherent transition from brief submission to session startup
- reload during startup remains startup
- the route auto-opens and sends the live startup handshake when the saved
  brief is present and no checkpoint exists

### 6.4 Early retry must preserve the saved brief

Required behavior:

- when startup genuinely fails before first reveal, the retry path keeps the
  brief
- retry returns the user to the same startup contract instead of redirecting
  through restart-only or empty-session logic

### 6.5 Entry-path semantics must converge

Required behavior:

- `/sessions/new` and project-scoped saved-brief session start must reach the
  same backend truth model
- future entry points should be able to reuse that same contract without adding
  route-specific exception logic

## 7. Acceptance Criteria

This slice is complete when:

1. `/sessions/new` no longer lands on `Analysis needs a restart` before a real
   startup failure occurs
2. starting analysis from `/sessions/new` does not leave the user with a blank
   orphan session outcome if startup intent persistence or startup initiation
   fails
3. the route presents a truthful startup transition from brief submission to
   first reveal
4. startup retry preserves the saved brief and returns to startup truth
5. reload before first reveal still behaves like startup, not like restart-only
6. real browser-backed verification covers the actual `/sessions/new` path
7. the selected bank-first first-reveal/runtime contract from Phase 26 remains
   unchanged

## 8. Verification Plan

Required verification:

- targeted backend tests for the saved-brief startup projection and any new
  create-with-brief or equivalent backend contract
- route-level tests for startup truth and retry semantics in the session status
  computation layer
- real browser-backed proof that:
  - starts from `/sessions/new`
  - submits a brief
  - reaches truthful startup state
  - reaches bank-first first reveal
  - survives reload during startup
  - exercises an early failure path that preserves the saved brief and supports
    retry

Mocked browser proof may remain supplemental, but it does not close this slice.

## 9. Rollback And Fallback

If one explicit saved-brief creation contract cannot land in this slice, the
minimum acceptable fallback is:

- fix the backend startup truth from Phase 27
- keep the current two-step transport only temporarily
- add a truthful in-place failure recovery so the user does not experience the
  blank orphan-session outcome as the product result

That fallback is acceptable only if the user-visible product flow still behaves
as one coherent saved-brief startup path.

## 10. Nice-To-Haves After Phase 28

These are product-flow improvements worth tracking, but they should not delay
the bounded execution of Phase 28:

- clean up the naming drift around `start_socratic` so the API name reflects
  startup intent rather than implying immediate runtime start
- let project-first entry points preselect or require project context when that
  improves the user journey instead of always creating an unscoped session
- add a richer but still truthful startup progress narrative if the current
  single-line transition feels too opaque in live testing
- collapse duplicate saved-brief entry UIs once the backend contract is shared

## 11. Closeout

This spec is now implemented.

The implemented product decision is:

- Phase 27's startup-truth fix is required
- the saved-brief creation flow should be treated as one user action with one
  coherent outcome
- the adjacent startup transition and retry behavior are bounded enough to fix
  in the same slice without reopening the Phase 26 runtime contract
