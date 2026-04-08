# Planner SolidStart Phase 24 Socratic Runtime Contract Reset Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Audit:** 2026-03-26 Socratic lobby/runtime baseline audit across `planner-server/src/ws_socratic.rs`, `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`, `planner-server/src/session.rs`, `planner-server/src/api.rs`, and `planner-solid/src/routes/sessions/[sessionId].tsx`

## 1. Executive Judgment

The next Socratic slice should not be another visual refinement pass.

The current route now has a more serious problem than layout polish: the live
runtime still carries two incompatible product contracts at once.

The implemented system is split between:

- a newer full prompt-bank first-reveal contract in planning and Solid route
  gating
- an older single-prompt and category-selection runtime model in server and
  checkpoint behavior

That split is the direct cause of the current failures:

- a single lead question can still appear before a full bank
- answering that question can leave the route stuck on a bank-building state
  while the runtime is actually waiting on a different handoff

The next bounded slice must therefore be a **runtime contract reset**.

This slice also includes the performance and cleanup work previously labeled as
nice-to-haves. They are not deferred follow-on work for this phase. They are
part of the required bounded reset.

## 2. User Outcome

After Phase 24:

- the first Socratic reveal is driven by one strict backend truth model
- the route never shows a single lead question as if it were the full initial
  bank
- the first page is complete for the currently derivable prompt bank at the
  time of first reveal
- answering the active prompt always leads to one truthful next state:
  refreshed prompt bank with newly generated questions/categories where
  warranted by the answer, build-ready handoff, or explicit error/attention
- checkpoint resume restores the same runtime shape the live route expects
- the Solid session route no longer hides a runtime that is actually waiting
  on unsupported category-navigation actions
- initial prompt-bank generation is parallelized within a bounded concurrency
  limit instead of staying purely serial
- the route consumes a cleaner prompt-bank update contract with reduced
  unconditional refetch churn
- the startup transport stops using `start_pipeline` for Socratic analysis
  startup

## 3. Problems To Solve

### 3.1 First-reveal truth drift

The product contract now says first reveal depends on a real initial prompt
bank.

The active backend still allows `/sessions/{id}/prompt-bank` to synthesize
`banked_threads` from `checkpoint.current_prompt` when `checkpoint.prompt_bank`
is empty, and it can derive `initial_bank_complete` from heuristics instead of
strict persisted truth.

That preserves the old single-prompt reveal path under a new name.

### 3.2 Post-answer dead-end drift

After a prompt response, the runtime can clear the prompt bank and fall back
into category-selection mode, waiting for `enter_category` or
`back_to_categories`.

The Solid session route does not expose or send those actions.

That means the UI can show a loading/status surface while the runtime is not
actually building the bank anymore. It is waiting at a handoff the route
cannot complete.

This is especially wrong because the selected product behavior is not "static
bank forever." The initial bank must be complete for first reveal, and later
answers must create new prompt-ready categories and prompts dynamically when
the answer changes what the runtime can now derive.

### 3.3 Checkpoint/resume hybrid drift

Checkpoint state still treats `current_prompt` as a live first-class source of
truth:

- replay can emit a prompt bank from `current_prompt`
- resume promotes `current_prompt` into `pending_prompt`
- status logic treats `current_prompt` as prompt-bank progress

That is a compatibility shape, not the selected future contract.

### 3.4 Status and UI gating drift

The backend now exposes `workspace_status`, but the underlying computation
still conflates:

- active prompt-bank assembly
- any prompt/category checkpoint artifact
- hidden category-selection state

So the label can say "Building the initial prompt bank" even when the route is
not on an active bank-assembly path anymore.

### 3.5 Transport and runtime indirection drift

The Solid route still operates by:

- opening the Socratic websocket
- receiving a mixture of `prompt`, `prompt_bank`, `category_state`, and
  `workspace_state`
- refetching REST resources on most websocket messages

That keeps the route loosely coupled to a hybrid runtime instead of consuming
one authoritative session-workspace contract.

## 4. Product Decision

Phase 24 resets the Socratic route around four strict contracts.

### 4.1 One strict first-reveal truth model

For the Solid session route, first reveal is allowed only when the backend has
persisted one of:

1. a complete initial prompt bank
2. a build-ready state with no remaining prompt work
3. an explicit error or attention-required state

`current_prompt` must not qualify as a first-reveal surrogate.

### 4.2 One strict post-answer transition model

After a `prompt_response`, the runtime must transition into exactly one of:

1. a refreshed banked workspace, including any newly generated categories or
   prompts caused by the answer
2. a build-ready handoff
3. an explicit error or attention-required state

The Solid route must not depend on hidden category-navigation as an
intermediate recovery path.

This phase explicitly requires dynamic post-answer expansion:

- answers must generate new questions when the new information creates
  additional derivable prompt work
- answers must generate new categories when the new information creates
  additional derivable category structure
- answers must update which threads are now banked versus queued when the
  runtime truth changes

What changes in Phase 24 is not whether dynamic generation exists. What changes
is that the runtime must publish those changes back into one truthful route
contract instead of falling into unsupported hidden navigation state.

### 4.3 One strict checkpoint/resume model

Checkpoint resume for this route must restore one of the same states that the
live route can already render:

- prompt bank ready
- build ready
- startup still in progress
- attention required

Legacy `current_prompt` data may be read during migration, but it must be
promoted forward into the new contract before the route uses it.

### 4.4 One strict UI gating model

The Solid route must gate workspace reveal from backend-authored truth only:

- strict prompt-bank completeness
- build-ready
- explicit error/attention

The client must not infer revealability from fallback heuristics like "there is
at least one prompt-like thing."

This gating rule applies to first reveal only. After the initial page is
truthfully revealed, the route must continue to render backend-authored dynamic
question/category expansion caused by answers.

## 5. Scope

### In Scope

- removing `current_prompt` as a live first-reveal and prompt-bank fallback for
  the Solid route contract
- redefining the post-answer runtime so the route never lands in an
  unsupported hidden category-selection state
- tightening checkpoint persistence and replay around one bank-first contract
- tightening `/sessions/{id}/prompt-bank` so it reports only authoritative
  banked state
- aligning `workspace_status` with real runtime states instead of checkpoint
  artifact guesses
- tightening websocket/startup behavior so fresh start and checkpoint resume
  enter the same route-compatible state machine
- parallelizing initial prompt-bank generation for independent prompt-ready
  threads within a bounded concurrency limit
- tightening websocket transport so the route can consume authoritative
  prompt-bank updates with materially less default refetch churn
- renaming the Socratic startup handshake away from `start_pipeline` so the
  transport matches actual product semantics
- reducing legacy checkpoint fallback behavior as part of the same bounded
  runtime reset rather than treating it as a separate follow-on cleanup
- adding verification that proves first reveal, post-answer progression,
  checkpoint resume, and status truth against the real backend/frontend
  contract

### Out Of Scope

- a broad visual redesign of the session route
- changing the selected live-artifact split desktop direction from Phase 23
- rewriting the Socratic reasoning model beyond what the contract reset needs
- generalized TUI/runtime redesign outside the Solid session route boundary
- speculative multimodal or document-synthesis expansion

## 6. Must-Fix Contract Reset

### 6.1 First-reveal contract

Required behavior:

- `prompt_bank_response(...)` must stop fabricating `banked_threads` from
  `checkpoint.current_prompt`
- `initial_bank_complete` must be backend-authored contract truth, not a
  heuristic OR over partial state
- websocket replay for in-progress interview state must replay authoritative
  bank/build-ready/startup state, not silently convert a single prompt into a
  "complete" bank

### 6.2 Post-answer contract

Required behavior:

- after adjudicating a prompt response, the runtime must not leave the Solid
  route in a state that requires `enter_category` or `back_to_categories` to
  proceed
- if the answer creates new prompt-ready work, the runtime must publish the
  updated bank directly, including newly generated questions and categories
- if the answer changes category structure before new prompts are ready, the
  runtime must publish that updated structure through the same route-compatible
  workspace contract without dropping into hidden category-selection mode
- if no prompt-ready work remains, the runtime must publish build-ready
  directly
- if the runtime cannot produce either state, it must surface explicit
  attention/error state rather than a false bank-assembly label

### 6.3 Checkpoint and resume contract

Required behavior:

- checkpoint persistence must treat `prompt_bank`, `active_thread_id`, and
  explicit workspace state as the durable in-progress truth
- `current_prompt` may remain only as a migration shim during a bounded
  compatibility window
- checkpoint resume must not prefer `pending_prompt` over a known prompt bank
  for first reveal
- a resumed session must re-enter the same state family a fresh live session
  would expose to Solid

### 6.4 Session status contract

Required behavior:

- `workspace_status` must distinguish:
  - startup not yet begun
  - startup/classification in progress
  - prompt-bank assembly in progress
  - prompt-bank ready / awaiting response
  - build ready
  - attention required
- status must be computed from actual route-compatible runtime state, not from
  the mere presence of `current_prompt` or `current_category_snapshot`
- the false "Building the initial prompt bank" state must disappear when the
  runtime is actually waiting on some other unsupported branch

### 6.5 Solid route gating and transport contract

Required behavior:

- the Solid route must consume one authoritative in-progress workspace model
- the route may still refetch as recovery after error, but not as the normal
  way to discover whether the runtime has moved from prompt to bank to category
  state
- the route must not reveal a hidden unsupported state and must not hide a
  truthful banked state behind stale heuristics
- after first reveal, the route must continue to render dynamic backend-authored
  category/question growth caused by answers instead of behaving like a fixed
  one-time bank snapshot

## 7. Required Enhancements

The following enhancements are part of Phase 24 itself. They are included in
this spec as required delivery scope, not as deferred follow-on work.

### 7.1 Parallel prompt-bank generation

Required behavior:

- the initial bank must no longer be assembled as a purely serial per-thread
  prompt loop
- independent prompt-ready threads must be generated in parallel within a
  bounded configurable concurrency limit
- verification must prove the implementation still preserves deterministic
  bank completeness and thread identity

### 7.2 Prompt-bank transport tightening

Required behavior:

- the websocket/session contract must move closer to authoritative prompt-bank
  updates instead of treating prompt/category/workspace messages as triggers
  for routine full refetch
- the tightened transport must still support dynamic post-answer insertion of
  new categories, new prompts, and queued-to-banked transitions
- the Solid route must materially reduce unconditional `refetchSession()` and
  `refetchPromptBank()` churn during normal prompt-bank progression
- explicit refetch remains allowed as recovery after error or desynchronization

### 7.3 Startup protocol cleanup

Required behavior:

- the Socratic startup handshake must stop using `type: "start_pipeline"`
- client, websocket handler, and related types/tests must use a startup message
  name that matches Socratic analysis semantics
- migration compatibility may exist briefly inside the server if needed, but
  the active Solid route contract for this phase must use the corrected message

### 7.4 Legacy checkpoint cleanup

Required behavior:

- the Phase 24 implementation must reduce live reliance on legacy checkpoint
  promotion paths as part of the same runtime reset
- legacy `current_question` / `pending_draft` promotion may remain only as a
  narrow migration shim at the persistence boundary
- route-facing replay, resume, prompt-bank shaping, and status logic must no
  longer behave as though those legacy prompt forms are the active contract

## 8. Touched Surfaces

Expected touched surfaces include:

- `planner-server/src/ws_socratic.rs`
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-server/src/runtime.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-server/tests/server_integration.rs`
- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/session-status.ts`
- `planner-solid/src/lib/types.ts`
- `planner-solid/e2e/*` covering startup, first reveal, post-answer progression,
  and resume truth

## 9. Acceptance Criteria

This slice is complete only when:

1. a first-open session cannot reveal one lead prompt as if it were the full
   initial bank
2. the first revealed page is complete for the currently derivable prompt bank,
   not a one-question placeholder for later real hydration
3. `/sessions/{id}/prompt-bank` reports only authoritative banked threads for
   first reveal and does not fabricate bank completeness from `current_prompt`
4. answering a prompt never leaves the Solid route stranded in a hidden
   category-navigation state
5. answers that generate new questions or categories are reflected back into
   the route through the same truthful workspace contract
6. checkpoint resume restores the same route-compatible bank/build-ready state
   model as a fresh live session
7. `workspace_status` truthfully distinguishes startup, bank assembly, awaiting
   response, build-ready, and attention-required states
8. initial prompt-bank generation is parallelized with a bounded concurrency
   limit rather than remaining fully serial
9. the active Solid startup handshake no longer uses `start_pipeline`
10. backend/frontend verification proves the live contract end to end instead of
   only mocked banked payload rendering and also proves the tightened transport
   behavior without broad default refetch dependence

## 10. Verification Plan

- Rust unit coverage for:
  - checkpoint projection and status computation
  - prompt-bank response shaping
  - resume-state construction
- Rust integration coverage for:
  - fresh waiting-session startup into real banked reveal
  - post-answer progression into next bank or build-ready
  - post-answer generation of new categories/prompts through the same truthful
    route contract
  - checkpoint resume into truthful banked state
  - no fallback to false bank completeness from single-prompt state
  - prompt-bank generation and replay behavior under the bounded parallel path
- Solid unit coverage for:
  - reveal gating
  - session-status rendering
  - route behavior under bank/build-ready/attention states
  - startup handshake message semantics
  - local prompt-bank merge behavior under reduced refetch assumptions
- browser coverage for:
  - project -> session startup
  - first truthful bank reveal
  - answer commit -> next truthful state
  - answer commit -> dynamically added question/category state where applicable
  - reload or reconnect during in-progress interview without dead-end
  - no `start_pipeline` dependency in the active Socratic startup path

## 11. Rollback / Fallback

If one sub-part of the implementation proves unexpectedly risky during
delivery, fallback should preserve the full Phase 24 scope while narrowing only
the internal migration strategy:

- temporary server-side compatibility handling is allowed for the renamed
  startup handshake while the Solid route moves to the corrected message in the
  same slice
- temporary persistence shims are allowed for legacy checkpoint reads while the
  route-facing runtime contract moves fully to the bank-first model in the
  same slice
- bounded concurrency may ship with a conservative low limit if that is what
  keeps correctness and verification strong in the first pass

But the phase should not close if the required enhancements in Section 7 are
left for later. That would preserve the same hybrid runtime under a narrower
label.

## 12. Open Questions

These do not block readiness for implementation, but they should be answered
during delivery:

- should category-navigation remain as a TUI-only or legacy-only path once the
  Solid route contract is reset?
- what is the bounded concurrency limit for initial prompt-bank generation in
  the first implementation pass?
- should the route eventually expose richer optimistic artifact deltas over the
  websocket, or stay on authoritative prompt-bank snapshots plus local artifact
  projection for now?

## 13. Readiness Judgment

This spec is **ready for implementation**.

The repo now has enough grounded evidence to bound the work without guessing:

- the conflicting runtime states are known
- the first-reveal and post-answer failures are traced to exact handoff points
- the minimum reset boundary is clear
- the required enhancement work is now explicitly included in the same bounded
  slice instead of being deferred
