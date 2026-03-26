# Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec

**Status:** implemented  
**Date:** 2026-03-25  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 17 Workflow Closeout And React Retirement Spec](/home/thetu/planner/docs/planner-solidstart-phase-17-workflow-closeout-and-react-retirement-spec.md), [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Audit:** user-reported live Solid session test on 2026-03-25 plus direct repo inspection of `planner-solid` session-route startup behavior and the current websocket/session status contract

## 1. Executive Judgment

The next SolidStart slice should not be another broad visual or route-family pass.

The active session route now has a smaller but more damaging truth gap:

- a newly created session can sit in `waiting` while the page claims it is
  "Building the initial prompt bank…"
- the header exposes raw internal state (`waiting`, raw socket state, raw
  `current_step`) instead of telling the user what is actually happening
- the Solid route never starts the websocket handshake for `waiting` sessions,
  so the first-open path can dead-end before prompt-bank assembly even begins

This slice should therefore harden **session startup truth** and the
**session status line** together.

## 2. User Outcome

After Phase 21:

- opening a newly created session no longer strands the user on a false
  bank-building screen
- the session page distinguishes "ready to start" from "actively assembling
  the initial prompt bank"
- the session header uses one concise, low-noise status line that tells the
  user what is happening or what the route is blocked on
- the user sees a compact, truthful explanation of the current startup stage
  without being flooded by raw event text
- prompt-bank first-reveal behavior from Phase 18 remains intact, but the
  pre-reveal state is finally understandable

## 3. Problems To Solve

### 3.1 False loading state for idle `waiting` sessions

The current session route shows the same fallback panel whenever the workspace
is not yet revealable:

- headline: `Building the initial prompt bank…`
- copy: `Waiting for a truthful initial bank or a build-ready handoff before revealing the Socratic workspace.`

That is only truthful when prompt-bank assembly has actually begun.

For `waiting` sessions, the backend is not assembling the initial bank yet.
`planner-server/src/ws_socratic.rs` still waits for an initial description
message before it can start the interview runtime. The current Solid route
never performs that handshake for `waiting` sessions, so the page can stall in
an idle state while describing active work that has not started.

### 3.2 Raw status line leaks internal backend vocabulary

The current session header renders:

- raw `intake_phase`
- raw socket state
- raw `current_step`

This is implementation detail, not product language.

The user does not benefit from seeing `waiting` without context, and
`current_step` values like `socratic.workspace.generated` are too low-level to
be the primary explanation of route state.

### 3.3 Missing startup-stage contract

The route currently has no explicit, user-facing startup-stage model.

It can distinguish only:

- workspace revealable
- build-ready with no active thread
- not revealable yet

That is too coarse.

The user needs one compact answer to:

- what is happening right now?
- what are we waiting on?
- do I need to do anything?

### 3.4 First-open project-to-session handoff is incomplete

`planner-solid/src/routes/projects/[projectSlug].tsx` creates a session and
navigates directly to `/sessions/:id`, but the session route only opens the
live Socratic websocket when `intake_phase === "interviewing"`.

That means the "Start analysis" action can land the user on a session page
that is not actually starting analysis yet.

### 3.5 Prompt-bank truth is present but not translated into usable status

Phase 18 correctly introduced the prompt-bank completeness rule and the backend
does emit meaningful event stages such as:

- `socratic.classify.complete`
- `socratic.category_state.generated`
- `socratic.workspace.generated`
- `socratic.prompt.generated`

But the active session UI does not translate that into a restrained, truthful
status contract. The information exists, but the session page still feels
opaque.

## 4. Scope

### In Scope

- Solid session-route startup behavior on `/sessions/:sessionId`
- truthful treatment of `waiting` versus actively starting/interviewing
- one compact session header status line and startup-state surface
- automatic or explicit session-start handoff for `waiting` sessions
- backend-computed or backend-grounded startup/status semantics needed to avoid
  client guesswork
- preserving the Phase 18 prompt-bank first-reveal contract while making the
  pre-reveal path understandable
- verification for waiting-session startup, prompt-bank assembly status, and
  low-noise status rendering

### Out Of Scope

- a broad redesign of the session workspace layout
- changing the selected prompt-bank or master-detail product model
- adding a verbose event log or console-like trace to the header
- widening project, import, discovery, or admin route scope
- reopening the Phase 20 project-surface work

## 5. Current-State Evidence

- `planner-solid/src/routes/sessions/[sessionId].tsx` shows the same
  "Building the initial prompt bank…" fallback any time the workspace is not
  revealable yet.
- the same route only opens the Socratic websocket when
  `current.session.intake_phase === "interviewing"`.
- the same route currently renders raw header pills for `intake_phase`, socket
  state, and `current_step`.
- `planner-server/src/ws_socratic.rs` still treats `waiting` sessions as a
  pre-start state and calls `wait_for_initial_description(...)` before moving
  into interview runtime startup.
- `planner-server/src/session.rs` still exposes `intake_phase`, `resume_status`,
  and `current_step`, but not one structured, user-facing startup/status
  projection.
- `planner-solid/src/lib/prompt-bank.ts` correctly holds first reveal until the
  prompt bank is complete, build-ready, or errored, but it does not explain
  whether the route is idle, actively assembling, or blocked on user action.

## 6. Product And Technical Contract

### 6.1 Waiting-session truth contract

The session route must stop treating all unrevealed states as "building the
initial prompt bank."

Required behavior:

- a session in `waiting` with `resume_status === "ready_to_start"` must render
  as an **idle start state**, not an active loading state
- if a persisted starting description already exists for that session, the
  Solid route must automatically initiate the existing startup handshake
  instead of marooning the user
- if no persisted starting description exists, the page must render one compact
  explicit start state with a primary action rather than a false loading panel

This slice chooses the minimal bounded direction:

- reuse the existing websocket startup protocol rather than inventing a broad
  new route family or utility flow
- auto-start when the route already has authoritative description text
- fall back to an explicit single-action idle start state only when the route
  genuinely lacks enough data to begin

### 6.2 Session status-line contract

The session header must replace raw internal pills as the primary user-facing
status model.

Required behavior:

- do not show raw `waiting` as the main explanation
- do not show raw `current_step` strings as the main explanation
- render one concise primary status line plus at most one supporting detail
  line
- keep the surface low-noise and operational, not chatty

The status line should answer:

- what the route is doing now
- what it is blocked on, if anything
- whether the user needs to act

### 6.3 Structured startup/status truth contract

This slice should introduce one structured status projection for the session
workspace.

Preferred contract:

- backend exposes a small `workspace_status` or equivalent structured field on
  session payloads
- the field is computed from backend truth such as:
  - `intake_phase`
  - `resume_status`
  - `current_step`
  - checkpoint/prompt-bank completeness
  - error state

Minimum required semantic states:

- `ready_to_start`
- `starting_analysis`
- `classifying`
- `assembling_prompt_bank`
- `awaiting_response`
- `build_ready`
- `pipeline_running`
- `complete`
- `attention_required`

Minimum required payload shape:

- stable state id
- short user-facing label
- short supporting detail or blocking reason
- optional tone/severity

The client may still layer local socket state as a secondary diagnostic signal,
but it must not invent the primary startup/status meaning from heuristics
alone.

### 6.4 Prompt-bank reveal-state contract

The prompt-bank first-reveal rule remains unchanged from Phase 18.

What changes here is the pre-reveal explanation:

- if the prompt bank is actively assembling, say so
- if the route is still waiting for an initial description or explicit start,
  say that instead
- if build-ready has already been reached with no prompt work left, say that
  directly
- if the route is detached, errored, or restart-only, surface that truthfully

The reveal gate stays strict.

The status explanation becomes understandable.

### 6.5 First-open project handoff contract

Project-created sessions must not dead-end on first open.

Required behavior:

- the project "Start analysis" path must land on a session that either:
  - begins startup automatically because a persisted description is already
    known, or
  - shows a truthful explicit start state immediately
- the user must not land on a page claiming prompt-bank assembly while no
  interview runtime has actually started

### 6.6 Low-noise presentation contract

This slice must improve status clarity without turning the header into a log.

Required behavior:

- keep the header compact
- prefer one principal line of status language
- keep raw event-stream detail out of the main session chrome
- allow raw/internal step data only as a secondary diagnostic hint if it is
  materially useful and visually subordinate

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/lib/types.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/*` for session-status helpers if extracted
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-solid/e2e/*` covering waiting-session startup and session status
- targeted Rust/server tests for any new session status projection

## 8. Acceptance Criteria

This phase is complete only when:

1. a `waiting` session no longer renders the false "Building the initial prompt
   bank…" state by default
2. a project-created session can start truthfully from the session page without
   becoming stranded in idle `waiting`
3. the session header no longer relies on raw `intake_phase` plus raw
   `current_step` as its primary user-facing status explanation
4. the session route renders one concise status line that tells the user what
   is happening or what the route is blocked on
5. the prompt-bank pre-reveal state distinguishes idle start, active bank
   assembly, build-ready, and attention/error states truthfully
6. the status surface remains restrained and does not become an event-log dump

## 9. Verification Plan

### Automated

- targeted Solid unit tests for any new session-status mapping helpers
- targeted Rust or server tests for any new structured session/workspace status
  projection
- run `npm test` inside `planner-solid`
- run `npm run lint` inside `planner-solid`
- run `npm run build` inside `planner-solid`

### Browser

- open a freshly created project session and confirm it does not strand on a
  false bank-building state
- verify that a `waiting` session with a known description auto-starts or
  presents an explicit truthful start state immediately
- verify the header status line changes across:
  - idle ready-to-start
  - startup/bank assembly
  - live answerable workspace
  - build-ready with no active prompt thread
  - error/attention state
- confirm the header remains low-noise and does not expand into raw event text
- verify the prompt-bank first reveal still waits for a truthful bank or
  build-ready handoff

## 10. Rollback And Fallback

- if a full backend status projection is larger than expected, the minimum
  truthful fallback is:
  - stop rendering the false bank-building state for `waiting` sessions
  - add explicit waiting-session start behavior
  - derive a small bounded status line from existing backend truth plus
    prompt-bank state without exposing raw internal strings as the primary UI
- if auto-start from persisted description proves risky, fall back to an
  explicit single-action start state for `waiting` sessions; do not keep the
  current false loading panel
- if the header treatment starts to sprawl, prefer the compact primary line
  plus one supporting line rather than adding more pills or event rows

## 11. Open Questions

These do not block readiness:

- whether the structured session status projection should live directly on the
  session payload or in a dedicated session-workspace endpoint helper
- whether the subdued diagnostic hint should still surface raw socket state in
  any form once the primary status line is truthful

## 12. Readiness Judgment

This spec is **ready for implementation**.

The slice is narrow, the current failure mode is concrete, and the product
contract is clear:

- do not lie about bank assembly
- do not strand a `waiting` session
- do not expose raw internal state as the main status language

This is a bounded follow-on to the implemented Phase 18 and Phase 20 work, not
a new route-family expansion.
