# Planner SolidStart Phase 26 Socratic Runtime Truth Completion Remediation Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 24 Socratic Runtime Contract Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-24-socratic-runtime-contract-reset-spec.md)  
**Related Planning:** [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Planner SolidStart Phase 24 Socratic Runtime Contract Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-24-socratic-runtime-contract-reset-spec.md), [Planner SolidStart Phase 25 Socratic Runtime Verification Hardening Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-25-socratic-runtime-verification-hardening-remediation-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 Socratic lobby/runtime implementation review across `planner-server/src/ws_socratic.rs`, `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`, `planner-server/src/session.rs`, `planner-server/src/api.rs`, `planner-solid/src/routes/sessions/[sessionId].tsx`, `planner-solid/src/lib/prompt-bank.ts`, `planner-solid/src/lib/session-status.ts`, `planner-server/tests/server_integration.rs`, and `planner-solid/e2e/*`

> Status sync note (2026-03-30): this slice was implemented and should no
> longer advertise `ready for implementation`. The active planning spine in
> [project-plan.md](/home/thetu/planner/docs/project-plan.md) already records
> Phase 26 as closed by implementation and live browser proof.

## 1. Executive Judgment

Phase 24 and Phase 25 materially improved the Socratic runtime, but they do
not yet close the runtime thread honestly.

The remaining gaps are no longer broad architecture questions. They are now
specific contract breaches and closeout-proof gaps:

- legacy `current_prompt` still materially shapes live replay and checkpoint
  resume
- the post-answer runtime and the Solid route still do not share one strict
  post-reveal contract when the workspace temporarily has no banked threads
- the Phase 25 browser proof is still mocked, so it does not prove the live
  backend/frontend contract it claims to close
- `start_pipeline` remains a live Socratic websocket compatibility path even
  though `start_socratic` is the selected product contract

The next slice should therefore be a bounded **truth-completion remediation**
pass, not another verification-only pass and not another broad redesign.

## 2. User Outcome

After Phase 26:

- first reveal, checkpoint resume, reconnect, and reload all obey one strict
  bank-first truth model
- legacy `current_prompt` may still be read as migration data, but it no
  longer reappears as authoritative route-facing prompt-bank completeness
- after the workspace has been truthfully revealed once, later runtime updates
  do not collapse the route back into a first-reveal loading gate
- answering a prompt never leaves the route depending on hidden category
  navigation or a bank-empty dead-end
- dynamic post-answer category and question expansion remains supported
  through the same route-facing contract
- browser closeout proof now covers at least one live backend-backed
  reconnect/reload or post-answer scenario instead of relying only on mocked
  websocket payloads
- the startup transport truth is explicit: `start_socratic` is the active
  contract, and any `start_pipeline` compatibility is quarantined as migration
  behavior instead of active route truth

## 3. Problems To Solve

### 3.1 Legacy single-prompt replay drift

`/sessions/{id}/prompt-bank` no longer fabricates first-reveal bank truth from
`current_prompt`, but the live websocket replay and checkpoint resume path
still promote a legacy `current_prompt` into `prompt_bank` shape and mark
`initial_bank_complete = true`.

That means the system still has two incompatible truths:

- REST first reveal says one prompt is not a full bank
- live replay/resume can still say one prompt is a complete bank

That is not an acceptable migration boundary.

### 3.2 Post-reveal route collapse drift

The Solid route currently uses the same reveal gate before and after first
reveal:

- reveal only when `initialBankComplete && threadOrder.length > 0`
- or build-ready
- or error

But the engine still has a route-facing empty-bank/category-only publish path
and still accepts `EnterCategory` and `BackToCategories`.

That creates a loophole:

- the first page can reveal truthfully
- a later answer can still leave the runtime in a bank-empty non-build-ready
  state
- the route then falls back to the loading surface again even though the user
  is already in the revealed workspace

This breaks the selected contract that first-reveal gating is special, while
post-reveal dynamic updates must stay within the same visible workspace model.

### 3.3 Verification closeout drift

Phase 25 added real Rust proof for several important paths, but its Playwright
coverage still replaces the backend with mocked REST and websocket payloads.

That is useful client coverage, but it is not full closeout proof for:

- real reconnect or reload against the live Rust websocket/runtime
- real live backend-driven dynamic expansion behavior in the browser
- real route truth under non-mocked startup and replay conditions

The repo should not describe mocked browser proof as if it closes those live
contract claims.

### 3.4 Startup compatibility drift

The active Solid route now sends `start_socratic`, but the Socratic websocket
still accepts `start_pipeline` during startup.

That is tolerable only as an explicitly bounded migration shim.

It is not acceptable for the closeout spec and tests to leave that alias
looking like an equally valid active contract.

## 4. Product And Technical Decision

Phase 26 completes the runtime thread around four strict decisions.

### 4.1 One strict replay and resume truth model

Legacy `current_prompt` may be read only as migration input.

It must not directly define route-facing truth for:

- websocket replay
- checkpoint resume
- reconnect or reload state
- `workspace_status`
- prompt-bank completeness

Required behavior:

- if checkpoint state already contains a real bank, replay and resume use it
- if checkpoint state is legacy and lacks a real bank, the runtime must
  rebuild a truthful route-compatible state from belief/category data or fall
  back to an explicit restart or attention-required state
- the system must not synthesize "complete bank" truth from a lone legacy
  prompt

### 4.2 Selected fix: runtime-owned post-reveal route contract

The first-reveal gate is only for first reveal.

After the route has already displayed a truthful workspace once, later runtime
updates must keep the route inside a visible workspace state machine instead of
dropping back to the first-reveal loader.

Phase 26 explicitly selects the runtime-first fix.

Required behavior:

- once a session has revealed a banked workspace or build-ready handoff, the
  route keeps rendering that workspace family until explicit error,
  completion, or route exit
- the backend/runtime must stop publishing a route-facing bank-empty,
  non-build-ready state that requires client-side inference about hidden
  category navigation
- a temporary bank-empty category-only update after first reveal must render as
  a truthful runtime-owned in-workspace refresh or queued-state surface, not as
  a reset to the initial loading panel
- the engine must not rely on hidden `EnterCategory` or `BackToCategories`
  actions as the required next step after a `prompt_response`

The route may still render a lighter in-workspace "updating workspace" state,
but that state must be driven by an explicit route-compatible backend contract,
not by reusing the initial reveal gate against a bank-empty payload.

### 4.3 One strict dynamic expansion route contract

Later answers must still be allowed to change what is derivable.

Required behavior:

- answers may add newly derivable categories
- answers may move threads from queued to banked
- answers may replace or retire stale bank entries
- those changes must come back through one route-compatible contract with
  stable thread identity and usable focus selection

If the runtime needs an intermediate structural update, that update must still
be published as a route-compatible revealed-workspace state and must not
require hidden navigation or a client-side fallback to the first-reveal loader.

### 4.4 One strict closeout-proof contract

Closeout proof must distinguish clearly between:

- mocked client-state tests
- real backend/runtime tests
- real browser proof

Required proof boundary:

- mocked Playwright or websocket-harness tests may remain for local merge and
  UI-state assertions
- at least the highest-risk reconnect/reload or post-answer progression path
  must be proven in a browser scenario backed by the real Rust server/runtime
  rather than only synthetic websocket payloads

### 4.5 One explicit startup transport boundary

`start_socratic` is the active contract for Socratic startup.

Allowed compatibility:

- `start_pipeline` may remain temporarily only as an explicitly documented
  migration shim

Not allowed:

- treating `start_pipeline` as an equally valid active startup path in route
  code, closeout proof, or planning status

## 5. Scope

### In Scope

- removing legacy `current_prompt` as a route-facing live replay or resume
  truth source
- tightening checkpoint replay, reconnect, and reload into one strict
  bank-first truth model
- fixing the runtime and Solid route together so the first-reveal gate is not
  reused as the post-reveal steady-state gate
- removing the hidden category-navigation loophole after prompt responses from
  the runtime contract itself instead of preserving it behind richer client
  heuristics
- preserving dynamic post-answer category and question expansion through the
  same route-facing contract
- adding live browser proof for at least one real reconnect/reload or
  post-answer expansion scenario
- tightening planning language so Phase 25 is no longer treated as complete
  closeout for the runtime thread
- clarifying or shrinking the `start_pipeline` compatibility surface

### Out Of Scope

- a broad visual redesign of the session route
- changing the selected Phase 23 live-artifact split direction
- rewriting the Socratic reasoning model beyond what the contract completion
  needs
- speculative transport abstraction work outside the Socratic route boundary
- deleting every legacy checkpoint field from persistence in the same slice if
  a narrower compatibility boundary is sufficient

## 6. Must-Fix Remediation Contract

### 6.1 Legacy checkpoint compatibility boundary

Required behavior:

- `build_checkpoint_resume_state(...)` or its replacement must stop promoting
  a lone `current_prompt` into a route-facing complete bank
- websocket replay must not emit `initial_bank_complete = true` from only
  legacy single-prompt state
- session summaries or status labels must not present legacy single-prompt
  data as "waiting for your response" unless a real route-compatible bank or
  other supported revealed state exists

### 6.2 Post-answer route completion

Required behavior:

- after `prompt_response`, the runtime must end in one route-compatible state:
  refreshed bank, revealed queued/refreshing workspace state, build-ready, or
  explicit attention/error
- the runtime must not require hidden category navigation to continue
- the runtime must not publish a route-facing bank-empty non-build-ready state
  that forces the client to collapse a previously revealed workspace back into
  the initial loading panel

### 6.3 Dynamic expansion completion

Required behavior:

- newly derivable categories and questions still surface after answers
- queued-to-banked transitions preserve thread identity
- focus selection remains valid when the active thread retires or is replaced
- already-banked work does not regress into fake shell-only states

### 6.4 Verification completion

Required behavior:

- at least one live browser proof covers reconnect/reload or post-answer
  expansion against the real server/runtime
- mocked browser tests are kept only as complementary client-behavior proof
- closeout language must not describe mocked websocket injection as if it were
  live backend proof

### 6.5 Startup transport completion

Required behavior:

- the Solid session route and active closeout proof use `start_socratic`
- if `start_pipeline` remains accepted by the websocket parser, the spec and
  tests treat it as migration-only compatibility instead of active product
  behavior

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-server/src/ws_socratic.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-server/tests/server_integration.rs`
- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/session-status.ts`
- `planner-solid/e2e/*`
- `docs/project-plan.md`

## 8. Acceptance Criteria

This remediation slice is complete only when:

1. legacy checkpoint data cannot reappear as a false complete prompt bank on
   websocket replay, reconnect, reload, or checkpoint resume
2. `/sessions/{id}/prompt-bank`, websocket replay, and live resume all agree
   on one route-facing bank-first truth model
3. after first reveal, the Solid route does not fall back to the initial
   loading panel merely because later runtime updates temporarily contain no
   banked threads
4. after a prompt answer, the runtime never requires hidden category
   navigation to continue the visible route
5. dynamic post-answer category and question expansion still occurs through
   the same route-facing contract, including queued-to-banked transitions and
   stable thread identity
6. the repo contains at least one live browser-backed proof for reconnect,
   reload, or post-answer expansion against the real Rust server/runtime
7. mocked browser proof is described only as mocked client proof, not as full
   live contract closeout
8. `start_socratic` is the explicit active Socratic startup contract, and any
   `start_pipeline` compatibility is documented as migration-only behavior
9. planning surfaces describe Phase 24 and Phase 25 honestly and identify this
   slice as the remaining runtime truth-completion work

## 9. Verification Plan

### Rust integration coverage

- replay or resume from legacy single-prompt checkpoint data does not declare
  false bank completeness
- replay or resume from real banked checkpoint data still restores the
  expected revealed state
- post-answer progression never lands in a hidden-navigation dead-end
- category-only or queued-only post-answer updates remain route-compatible
  instead of requiring hidden navigation

### Rust/unit coverage

- `workspace_status` does not treat legacy `current_prompt` as equivalent to a
  real revealed bank
- prompt-bank response shaping stays bank-first across REST and replay helpers
- startup transport parsing makes the active and compatibility paths explicit

### Solid/unit coverage

- first-reveal gating is one-time behavior, not the route's entire steady-state
  visibility rule
- post-reveal runtime-owned refresh states keep the workspace visible without
  reviving hidden client inference about category navigation
- queued-to-banked merges preserve focus and do not reset local workspace
  state unnecessarily

### Browser coverage

- one live backend-backed reconnect or reload scenario proving bank-first
  replay without false restart behavior
- one live backend-backed post-answer scenario proving the route stays in a
  truthful visible workspace contract while expansion or branch refresh occurs
- if mocked browser tests remain for local UI assertions, they are clearly
  supplemental and not the only proof for those contract claims

## 10. Rollback / Fallback

If full legacy compatibility cannot be preserved without lying about runtime
truth, the system must degrade truthfully.

Allowed fallbacks:

- convert legacy single-prompt-only checkpoint state into an explicit
  startup-rebuild or restart-required state
- keep a temporary `start_pipeline` parser alias while documenting it as
  migration-only behavior
- use a narrower runtime-authored in-workspace "Updating workspace" state after
  first reveal if the engine still needs one intermediate structural publish
  step

Not allowed:

- treating a lone legacy prompt as a complete bank
- collapsing a previously revealed workspace back into the initial loading gate
  because the bank is temporarily empty
- preserving the current loophole by pushing the entire post-reveal recovery
  burden into client-only heuristics while the runtime still emits hidden-
  navigation states
- keeping mocked websocket injection as the only browser proof for live route
  closure
- claiming the Socratic runtime thread is fully closed before this slice lands

## 11. Open Questions

No material product questions remain.

Implementation may choose only between:

- removing `start_pipeline` acceptance from the Socratic websocket entirely
- or keeping it as a clearly documented migration-only parser alias

That choice does not block readiness as long as the active route contract,
tests, and planning surfaces remain explicit.
