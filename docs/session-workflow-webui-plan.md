# Session Workflow Web UI Implementation Plan

This document breaks the work into incremental implementation steps that should
fit into small, reviewable changes. The sequence is optimized for a 256K
context window: each step should be understandable and implementable without
loading the entire system into working memory.

## Rules For This Plan

- Ship backend truth before backend behavior changes.
- Ship observability before automation.
- Do not expose a UI action until the backend can support it truthfully.
- Keep each step small enough to complete and verify in one focused pass.
- Add tests at the same time as behavior changes.

## Implementation Status (Updated 2026-03-06)

- Phase 2 status: complete
- Phase 2.1: complete (`InterviewCheckpoint` persisted in session model)
- Phase 2.2: complete (stable `socratic_run_id` created and reused)
- Phase 2.3: complete (checkpoint writes on classification, belief updates, question, draft, contradiction, convergence)
- Phase 2.4: complete (checkpoint fields exposed and rendered in web UI)
- Phase 2.5: complete (checkpoint persistence + payload tests added)

- Phase 3 status: complete
- Phase 3.1: complete (server accepts detached `interviewing` checkpoint reconnect without new initial `socratic_response`)
- Phase 3.2: complete (pending question/draft re-emitted; regenerate-next-prompt path when no pending prompt)
- Phase 3.3: complete (capability-driven auto-attach + resumable session UI copy/state)
- Phase 3.4: complete (integration tests cover checkpoint reconnect + prompt replay, resumed answer progression, and session-page resume behavior)

---

## Phase 0: Make The Current UI Truthful

### Step 0.1: Add explicit session capability fields

Goal:
- stop inferring UI actions from `intake_phase` alone

Implement:
- add backend-computed capability fields to the session response
- start with current behavior only, no resume logic changes

Suggested fields:
- `can_resume_live`
- `can_resume_checkpoint`
- `can_restart_from_description`
- `can_retry_pipeline`
- `has_checkpoint`
- `resume_status`

Initial values should reflect current behavior:
- `pipeline_running|complete|error` can attach
- `interviewing` cannot resume
- waiting sessions can start

Likely files:
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-web/src/types.ts`

Done when:
- session API returns explicit capability state
- frontend types include the new fields

### Step 0.2: Render actions from capabilities

Goal:
- make the dashboard and session page reflect backend truth

Implement:
- replace hard-coded action labels with capability-based rendering
- keep the current interview warning, but drive it from `resume_status`
- show whether an interviewing session is resumable, restart-only, or unknown

Likely files:
- `planner-web/src/pages/Dashboard.tsx`
- `planner-web/src/pages/SessionPage.tsx`

Done when:
- the UI does not guess resume behavior from local phase logic alone

### Step 0.3: Lock in current truth with tests

Goal:
- prevent accidental UI/backend mismatch

Implement:
- add API tests for capability mapping
- update session page tests to assert capability-driven behavior

Likely files:
- `planner-server/tests/server_integration.rs`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`

Done when:
- the current behavior is encoded in tests

---

## Phase 1: Fix Disconnect Safety

### Step 1.1: Fix mid-interview disconnect phase corruption

Goal:
- a dropped socket must not advance an unfinished interview into
  `pipeline_running`

Implement:
- audit `handle_socratic_ws`
- ensure disconnect during `interviewing` leaves the session in a valid
  interview state
- only transition to `pipeline_running` after explicit convergence

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`

Done when:
- disconnecting mid-interview never marks the session as converged or building

### Step 1.2: Add explicit detached interview state

Goal:
- distinguish live interview, detached interview, and resumable interview

Implement:
- add a detached/resume status model without yet implementing resume
- backend should indicate whether the session is:
  - attached
  - detached but restart-only
  - detached and checkpoint-resumable

Likely files:
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-web/src/types.ts`

Done when:
- the UI can clearly tell the user whether a detached interview can continue

### Step 1.3: Add disconnect tests

Goal:
- lock down the failure mode that currently exists

Implement:
- add server integration coverage for:
  - disconnect before convergence
  - disconnect after convergence
  - reconnect to detached interview state

Likely files:
- `planner-server/tests/server_integration.rs`

Done when:
- regressions in disconnect handling are caught automatically

---

## Phase 2: Persist A Minimal Interview Checkpoint

### Step 2.1: Introduce `InterviewCheckpoint`

Goal:
- define one durable object for resumable interview state

Implement:
- add a checkpoint struct to the session model
- keep the first version minimal and serializable

Minimum fields:
- `socratic_run_id`
- `classification`
- `belief_state`
- `current_question`
- `pending_draft`
- `contradictions`
- `stale_turns`
- `draft_shown_at_turn`
- `last_checkpoint_at`

Likely files:
- `planner-server/src/session.rs`
- `planner-web/src/types.ts`

Done when:
- session persistence can store and reload a checkpoint structure

### Step 2.2: Persist a stable Socratic run identifier

Goal:
- make existing CXDB restore utilities usable

Implement:
- create `socratic_run_id` before `run_interview`
- store it on the session
- reuse it for checkpoint and CXDB persistence

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/session.rs`
- `planner-core/src/pipeline/steps/socratic/belief_state.rs`

Done when:
- a session has a stable interview run identifier for its lifetime

### Step 2.3: Write checkpoint state on every meaningful transition

Goal:
- make reconnect/restart possible without guessing

Implement:
- update checkpoint after:
  - classification
  - belief-state updates
  - question generation
  - draft generation
  - contradiction updates
  - convergence

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`

Done when:
- interrupting the process leaves a recent checkpoint on disk

### Step 2.4: Hydrate checkpoint into the web UI

Goal:
- let the UI display saved interview state before true resume exists

Implement:
- include checkpoint fields in session payloads
- display current question / pending draft / checkpoint timestamp in the session
  page as read-only saved state

Likely files:
- `planner-server/src/api.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/pages/SessionPage.tsx`

Done when:
- a detached interview shows saved state rather than only a generic banner

### Step 2.5: Add checkpoint persistence tests

Goal:
- prove the saved state round-trips

Implement:
- add tests for:
  - session save/load with checkpoint
  - checkpoint updated after question generation
  - checkpoint updated after draft generation

Likely files:
- `planner-server/src/session.rs`
- `planner-server/tests/server_integration.rs`

Done when:
- checkpoint round-trip behavior is tested

---

## Phase 3: Resume Interview From Checkpoint

### Step 3.1: Add checkpoint-resume server path

Goal:
- allow `interviewing` sessions to reconnect without restarting from the saved
  description

Implement:
- extend the Socratic WS handler to recognize checkpoint-resumable interviews
- on attach, restore the last checkpoint
- do not expect a new initial `socratic_response`

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-core/src/pipeline/steps/socratic/belief_state.rs`

Done when:
- the server can accept a reconnect to an interviewing session from saved state

### Step 3.2: Re-emit pending prompt state on resume

Goal:
- make the resumed UI actionable immediately

Implement:
- if a checkpoint contains a pending question, send it
- if a checkpoint contains a pending draft, send it
- if neither exists, send a resume message and regenerate the next prompt from
  saved state

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`

Done when:
- the user reconnects into a usable interview state, not a blank shell

### Step 3.3: Enable capability-driven auto-resume in the UI

Goal:
- replace restart-only behavior with real interview resume

Implement:
- auto-attach for interviewing sessions when
  `can_resume_checkpoint == true`
- remove or downgrade the warning banner for resumable sessions
- clearly label restored checkpoint state in the UI

Likely files:
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- the dashboard and session page expose “Resume Interview” only when it works

### Step 3.4: Add end-to-end resume tests

Goal:
- verify the full resume flow

Implement:
- cover:
  - start interview
  - generate question
  - disconnect
  - reload session
  - reconnect
  - answer resumed question

Likely files:
- `planner-server/tests/server_integration.rs`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`

Done when:
- checkpoint resume is covered by tests

---

## Phase 4: Support Live Runtime Reattach

### Step 4.1: Add a session runtime registry

Goal:
- keep a live interview runtime available across short disconnects

Implement:
- introduce an in-memory runtime registry keyed by session ID
- decouple the interview actor lifetime from a single WebSocket

Likely files:
- `planner-server/src/lib.rs`
- `planner-server/src/ws_socratic.rs`

Done when:
- the server can track whether a live interview actor still exists

### Step 4.2: Add lease and expiration behavior

Goal:
- avoid orphaned live runtimes

Implement:
- add lease timeout and cleanup rules
- define when the runtime falls back to checkpoint-only resume

Likely files:
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/main.rs`

Done when:
- live runtimes expire predictably and safely

### Step 4.3: Expose live-vs-checkpoint resume capabilities

Goal:
- let the UI distinguish a true reattach from a restored checkpoint

Implement:
- populate:
  - `can_resume_live`
  - `can_resume_checkpoint`
  - `is_attached`
  - `resume_status`

Likely files:
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-web/src/types.ts`

Done when:
- the UI can say “Reconnect Live” vs “Resume From Checkpoint”

### Step 4.4: Add live reattach tests

Goal:
- prove the fast path works

Implement:
- cover:
  - reconnect within lease
  - lease expiry fallback to checkpoint
  - duplicate connection rejection or rebinding behavior

Likely files:
- `planner-server/tests/server_integration.rs`

Done when:
- live reattach behavior is tested

---

## Phase 5: Add Workflow Controls To The Session Page

### Step 5.1: Add a session action bar

Goal:
- make workflow actions explicit and centralized

Implement:
- add a small action bar near the session header
- render actions from capability fields

Initial actions:
- Resume
- Restart from description
- Retry pipeline
- Back to dashboard

Likely files:
- `planner-web/src/components/SessionStatusHeader.tsx`
- `planner-web/src/pages/SessionPage.tsx`

Done when:
- the user can see available session actions in one place

### Step 5.2: Add restart-from-description endpoint

Goal:
- support the current fallback flow intentionally

Implement:
- add backend endpoint to restart an interview from saved description
- clear incompatible live state and create a fresh interview runtime

Likely files:
- `planner-server/src/api.rs`
- `planner-server/src/session.rs`

Done when:
- restart is an explicit backend action, not an implicit UI workaround

### Step 5.3: Add retry-pipeline endpoint

Goal:
- let the UI recover from pipeline failures cleanly

Implement:
- add backend endpoint to rerun pipeline from the current saved description or
  final intake state

Likely files:
- `planner-server/src/api.rs`
- `planner-server/src/session.rs`

Done when:
- errored pipeline sessions can be retried from the session page

### Step 5.4: Add workflow-control tests

Goal:
- prevent action bar regressions

Implement:
- add web tests for action visibility by capability
- add API tests for restart and retry endpoints

Likely files:
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`
- `planner-server/tests/server_integration.rs`

Done when:
- session controls are exercised by tests

---

## Phase 6: Improve Dashboard Workflow Visibility

### Step 6.1: Add resumability and activity indicators

Goal:
- turn the dashboard into a real workflow overview

Implement:
- show:
  - resumability
  - live-vs-checkpoint state
  - current step
  - last activity time
  - primary action

Likely files:
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- a user can decide which session to open and what state it is in from the
  dashboard alone

### Step 6.2: Add attention and failure indicators

Goal:
- surface sessions that need intervention

Implement:
- show warning/error badges
- sort by active and actionable sessions first

Likely files:
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- blocked or failed sessions are visually obvious

---

## Phase 7: Add Non-Resume Session Management

### Step 7.1: Rename sessions

Goal:
- make the dashboard operable at scale

Implement:
- add title/name field and rename endpoint

Likely files:
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- sessions are distinguishable without relying on ID or description snippet

### Step 7.2: Duplicate session

Goal:
- support branching from known-good context

Implement:
- add backend duplication from saved session state
- start with duplicate-from-description if full duplicate is too expensive

Likely files:
- `planner-server/src/api.rs`
- `planner-server/src/session.rs`
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- the UI supports “try again from this point” without destroying the original

### Step 7.3: Archive or hide sessions

Goal:
- keep the dashboard manageable

Implement:
- add archived flag and simple hide/archive action

Likely files:
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-web/src/pages/Dashboard.tsx`

Done when:
- inactive sessions can be removed from the main working surface

### Step 7.4: Export transcript and event log

Goal:
- make session history portable and auditable

Implement:
- add export endpoint and UI entry point

Likely files:
- `planner-server/src/api.rs`
- `planner-web/src/pages/SessionPage.tsx`

Done when:
- a user can export session history from the web UI

---

## Recommended PR Order

Use this as the default delivery sequence:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 5
6. Phase 6
7. Phase 4
8. Phase 7

Reason:
- truthfulness first
- safety second
- checkpoint resume before live actor complexity
- user-facing controls before deeper runtime optimization

If the team wants the best user impact per unit of effort, stop after Phase 5
first. That gets the product to truthful status, safe disconnect handling,
checkpoint-based interview resume, and explicit workflow controls from the web
UI.

---

## Success Line

The plan is successful when a user can manage the full session workflow from the
web UI without relying on server-side guesswork, hidden state, or restart-only
fallbacks.

---

## Review Gaps (Track At End Of Review)

### Gap R1: Checkpoint stale-turn accuracy for skip-only turns

Current status:
- Phase 2 checkpoint writes are event-driven and update `stale_turns` when a
  `belief_state_update` event is emitted.

Known gap:
- skip-only interview turns can increase engine `stale_turns` without emitting
  a belief-state update, which can leave checkpoint `stale_turns` lower than
  in-memory runtime state after detach.

Impact:
- low for UI hydration
- medium for future resume logic in Phase 3 if resume behavior depends on exact
  stale-turn count

Proposed follow-up:
- when implementing Phase 3 resume, either:
  - persist `stale_turns` directly from engine state on every loop iteration, or
  - emit an explicit stale-turn checkpoint event for skip/no-progress turns.
