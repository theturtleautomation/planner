# Planner SolidStart Phase 27 New Session Startup Truth Remediation Spec

**Status:** complete  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md)  
**Related Planning:** [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Planner SolidStart Phase 26 Socratic Runtime Truth Completion Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-26-socratic-runtime-truth-completion-remediation-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 live Phase 26 browser proof against the real Rust server/runtime plus direct repo inspection of `planner-solid/src/routes/sessions/new.tsx`, `planner-solid/src/lib/session-status.ts`, and `planner-server/src/api.rs`

> Status sync note (2026-03-30): this slice was absorbed by
> [Planner SolidStart Phase 28 Session Entry And Startup Product Flow Spec](/home/thetu/planner/docs/planner-solidstart-phase-28-session-entry-and-startup-product-flow-spec.md)
> and should not remain marked `ready for implementation` as a standalone
> execution target.

## 1. Executive Judgment

Phase 26 closed the live runtime proof gap, but it also surfaced one
remaining startup-path drift on a real product entry point:

- `/sessions/new` still creates a session, calls `POST /sessions/:id/socratic`,
  and navigates to `/sessions/:id`
- that endpoint stores the saved brief but also forces
  `intake_phase = "interviewing"` before any live runtime, prompt bank, or
  checkpoint exists
- the session route then lands on restart-only truth instead of the
  waiting-with-saved-brief startup contract selected in Phase 21

This is not a verification gap anymore. It is product-flow drift on the main
new-session entry point.

The next slice should therefore be a small startup-truth remediation, not a
broad session redesign.

## 2. User Outcome

After Phase 27:

- starting a session from `/sessions/new` follows the same truthful startup
  contract as any other waiting session that already has a saved brief
- the route no longer lands in `analysis needs a restart` immediately after
  the user just started analysis
- the new-session path transitions from saved brief to websocket startup to
  bank-first first reveal without a restart-only detour
- reload during startup still behaves like startup, not like a failed resume
- session startup semantics are consistent across creation paths instead of
  depending on which route created the session

## 3. Problems To Solve

### 3.1 `/sessions/new` bypasses the selected startup truth model

The current `/sessions/new` flow does this:

1. `POST /api/sessions`
2. `POST /api/sessions/:id/socratic`
3. navigate to `/sessions/:id`

But `POST /api/sessions/:id/socratic` currently writes:

- `project_description`
- `intake_phase = "interviewing"`
- `interview_runtime_active = false`
- `interview_live_attached = false`
- `has_checkpoint = false`

That produces a state that is neither:

- a truthful waiting session ready to start
- nor a truthful active interview runtime

It is a hybrid state that Phase 21 did not intend.

### 3.2 Route startup heuristics then classify the new session as restart-only

The session route opens the websocket for interviewing sessions, but it sends
the startup handshake only for startup-like states:

- `ready_to_start`
- `starting_analysis`
- `classifying`

The `/sessions/new` path currently lands in a backend-computed state that
behaves like `interview_restart_only` / `attention_required`, so the route
does not send the startup handshake.

That is why a just-started session can render:

- `Analysis needs a restart`
- `The live interview stopped before a resume point was saved.`

even though nothing actually stopped yet.

### 3.3 Product entry paths no longer share one startup contract

Phase 26 live proof showed that a session created through the project-scoped
session creation path with a saved brief behaves correctly:

- session remains `waiting`
- route auto-opens the websocket
- route sends `start_socratic`
- bank-first reveal follows

But `/sessions/new` still goes through a different, less truthful path.

That means the product still has two entry contracts for the same user intent:
"I have a saved brief; start analysis."

### 3.4 `start_socratic` endpoint naming has drifted from its actual behavior

The REST `start_socratic` endpoint does not directly start the runtime. It
stores startup intent and route context, then the websocket route performs the
live startup handshake.

The current implementation still writes session state as if analysis has
already started, even though the runtime start remains deferred.

That mismatch is the root of the product-flow drift.

## 4. Product And Technical Decision

Phase 27 chooses one strict product contract:

### 4.1 Saved-brief startup must converge on one waiting-state truth

If the system has a saved brief but no live runtime and no checkpointed
interview state yet, the session must remain in the waiting/startup family,
not in restart-only or detached interviewing truth.

Required behavior:

- a saved brief created through `/sessions/new` is represented as a session
  that is ready to begin startup, not as a failed or detached interview
- startup only transitions into interviewing truth once the live startup path
  is actually underway
- the user-visible route contract must be the same regardless of whether the
  saved brief came from:
  - `/sessions/new`
  - a project-scoped session creation path
  - any future saved-brief creation action

### 4.2 Selected fix: make `start_socratic` seed startup intent, not false interviewing state

This slice explicitly chooses the backend-truth fix.

Required behavior:

- `POST /sessions/:id/socratic` stores the saved brief, project association,
  title, and startup intent
- it must not force `intake_phase = "interviewing"` before a live runtime,
  checkpoint, or banked workspace exists
- it must leave the session in a truthful startup-ready state so the session
  route can apply the existing Phase 21 waiting-with-saved-brief logic
- `/sessions/new` may keep using the same endpoint if the endpoint truth is
  corrected

This is preferred over adding more client heuristics for one special route.

### 4.3 Startup transition boundary must become explicit

There are only three truthful pre-first-reveal states for a saved-brief
session:

- saved brief present, startup not yet initiated by the live route
- startup handshake/routing in progress
- live interview runtime underway and building the initial bank

Restart-only is not one of those states unless something actually failed or a
runtime truly detached after partial progress.

### 4.4 Reload during startup must remain startup

If the user reloads after starting a new session but before the first bank is
revealed, the route must still behave like startup.

It must not reinterpret the just-created session as:

- restart-only
- detached interviewing
- checkpoint-resumable

unless real evidence for those states exists.

## 5. Scope

### In Scope

- fixing the `/sessions/new` startup path so it lands on truthful waiting or
  startup-in-progress semantics instead of restart-only semantics
- correcting `POST /sessions/:id/socratic` so it no longer writes false
  interviewing truth before runtime startup actually exists
- preserving the existing Phase 21 route behavior where waiting sessions with
  saved briefs auto-open and send the startup handshake
- verifying first-open and reload behavior for the `/sessions/new` product
  path
- tightening backend session-status/resume-status semantics so this path
  cannot regress into false restart messaging again

### Out Of Scope

- redesigning the `/sessions/new` page
- renaming the REST `start_socratic` endpoint
- broad session-route layout work
- changing the selected bank-first runtime contract from Phase 26
- project-first workflow redesign beyond what is needed to make startup truth
  consistent

## 6. Must-Fix Remediation Contract

### 6.1 `start_socratic` may not synthesize detached interviewing truth

Required behavior:

- after `POST /sessions/:id/socratic`, the session must not compute to
  `interview_restart_only` or equivalent restart-required truth unless a real
  failed/detached interview exists
- backend session projections must treat "saved brief, no runtime, no
  checkpoint yet" as startup-ready truth

### 6.2 `/sessions/new` must converge on the Phase 21 waiting-session contract

Required behavior:

- a fresh session started from `/sessions/new` must land on the same route
  contract as a waiting session that already has a saved brief
- the route must auto-open the websocket and send the startup handshake
  without requiring a manual restart action
- the route must not render restart-only messaging on first open unless the
  startup truly failed

### 6.3 Reload before first reveal must remain truthful

Required behavior:

- if the user reloads immediately after creating the session and before the
  first prompt bank reveals, the session still behaves as startup-ready or
  startup-in-progress
- the route must not lose startup intent simply because first reveal has not
  happened yet

### 6.4 One startup contract across creation paths

Required behavior:

- project-scoped session creation with description and `/sessions/new` startup
  must converge on the same backend semantics for saved-brief startup
- future product flows should not need to choose between two inconsistent
  startup contracts for the same user intent

## 7. Acceptance Criteria

This slice is complete when:

1. starting from `/sessions/new` no longer lands on `Analysis needs a restart`
   before first reveal
2. `POST /sessions/:id/socratic` no longer leaves the session in false
   interviewing-without-runtime truth
3. the session route automatically sends the startup handshake for the
   new-session path through the same waiting-with-saved-brief logic already
   used elsewhere
4. reload during startup still resumes truthful startup behavior rather than
   restart-only messaging
5. the bank-first first-reveal contract from Phase 26 remains unchanged
6. verification includes a real browser-backed `/sessions/new` or equivalent
   new-session product-path proof, not only mocked route state

## 8. Verification Plan

### Rust / integration

- `start_socratic` session mutation leaves the session in startup-ready truth
  rather than restart-only truth
- session summary/status projection for this state is `ready_to_start`,
  `starting_analysis`, or other selected startup truth, never restart-only
  without real failure evidence

### Browser / Playwright

- real `/sessions/new` path:
  - create session
  - submit description
  - land on `/sessions/:id`
  - websocket startup occurs
  - bank-first reveal appears
  - no restart-only messaging appears on first open
- real reload during startup or immediately after startup:
  - route still behaves like startup or revealed workspace truth
  - route does not regress to restart-only without real detach/failure

## 9. Rollback / Fallback

Allowed fallback:

- if startup intent needs an explicit backend marker before runtime attach,
  add a small truthful startup-state field rather than reusing detached
  interviewing truth

Not allowed:

- keeping `/sessions/new` on a known false restart-only path
- fixing the issue only by teaching the client to ignore restart-only truth for
  this one path while the backend still emits it
- broadening the slice into a general session-entry redesign

## 10. Nice-To-Haves To Watch Next

These are product-flow follow-ons worth watching, but they are not required
for Phase 27 completion:

- unify all saved-brief creation paths behind one explicit project-scoped
  session creation contract so `/sessions/new` does not need a special flow
- add a small visible "Starting analysis…" transition treatment on
  `/sessions/new` before navigation so the user gets immediate action feedback
- add a truthful retry affordance for genuine startup failures that preserves
  the saved brief without forcing the user back through a restart-only dead end
- reduce backend naming drift by eventually deciding whether `start_socratic`
  should be renamed or documented more explicitly as "seed startup intent"

## 11. Open Questions

No major product question blocks readiness.

The only implementation choice left open is whether startup-ready truth is
represented by:

- keeping `intake_phase = "waiting"` with corrected status projections
- or introducing a narrower dedicated startup phase if the backend needs one

This does not block implementation as long as the user-visible contract
remains the same and restart-only truth is no longer emitted on fresh startup.
