# Planner SolidStart Phase 25 Socratic Runtime Verification Hardening Remediation Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 24 Socratic Runtime Contract Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-24-socratic-runtime-contract-reset-spec.md)  
**Related Planning:** [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md), [Planner SolidStart Phase 24 Socratic Runtime Contract Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-24-socratic-runtime-contract-reset-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 closeout review of the delivered Phase 24 contract reset against its own verification commitments

## 1. Executive Judgment

The next Socratic slice should not reopen the runtime contract reset itself.

That contract work is already implemented.

The remaining problem is narrower and more concrete:

- the route/runtime contract is now materially better aligned
- the highest-risk browser and unit surfaces are partially covered
- but the verification surface still does not completely prove every Phase 24
  claim end to end

This is therefore a **verification hardening remediation slice**, not another
runtime redesign.

## 2. User Outcome

After Phase 25:

- the repo has direct proof for the full Phase 24 runtime contract instead of
  relying on a mix of compile checks, narrow unit tests, and a few browser
  slices
- first reveal, post-answer progression, dynamic prompt/category expansion,
  checkpoint resume, reconnect/reload, and startup transport truth are all
  backed by explicit verification
- the remaining verification gap between "implemented" and "fully proven" is
  closed without changing the selected product/runtime contract

## 3. Problem To Solve

### 3.1 Phase 24 implementation is ahead of its proof surface

Phase 24 reset the contract and landed the code changes, but its own
verification plan was broader than the proof now present in the repo.

The remaining shortfall is not primarily product behavior drift. It is proof
coverage drift.

### 3.2 Missing or underpowered proof areas

The residual gaps are:

- no explicit Rust integration proof for fresh waiting-session startup into a
  truthful first bank reveal
- no explicit Rust integration proof for post-answer progression into the next
  bank or build-ready state
- no explicit Rust integration proof for dynamic post-answer category/prompt
  insertion through the same route-facing contract
- no explicit Rust integration proof that legacy single-prompt state cannot
  reappear as false first-reveal truth
- no explicit reconnect/reload browser proof for an in-progress Socratic
  session using the bank-first replay contract
- no direct proof that the bounded parallel prompt-bank path preserves bank
  completeness and stable thread identity under realistic multi-thread input

### 3.3 Closeout honesty still depends on finishing that proof

If these proof gaps remain, the repo will again be in a familiar bad state:

- the implementation may be correct enough in practice
- but the docs and plan will still overstate how fully the phase is closed

That is exactly the pattern this remediation slice should avoid.

## 4. Scope

### In Scope

- adding the remaining Rust integration coverage needed to prove the delivered
  Phase 24 runtime contract
- adding the remaining Solid/browser proof needed to prove reconnect/reload and
  no-hidden-state behavior on the live route
- tightening existing tests where they still encode older single-prompt or
  partial-proof assumptions
- syncing planning surfaces so Phase 24 and Phase 25 together describe the
  real closure state honestly

### Out Of Scope

- changing the selected first-reveal contract
- reopening the runtime architecture or transport design from Phase 24 unless a
  test exposes a concrete bug
- widening the session route UI beyond what is needed for truthful proof
- speculative performance tuning beyond what the verification slice must
  observe

## 5. Current-State Evidence

The delivered repo now has:

- compile-clean Rust and Solid surfaces for the Phase 24 reset
- route-level browser proof for startup handshake truth
- route-level browser proof for dynamic prompt-bank expansion after answering a
  first-reveal thread
- unit coverage for checkpoint replay/resume and workspace-status truth

But the repo does not yet have all of the explicit verification listed in the
Phase 24 verification plan.

This spec is the bounded response to that gap.

## 6. Verification Contract For This Slice

Phase 25 is complete only when the repo can directly prove the following
runtime truths.

### 6.1 Fresh-start bank truth

Required proof:

- a waiting session with a saved brief can start Socratic analysis and arrive
  at a truthful bank-first reveal
- that proof must demonstrate the route or backend does not treat a legacy
  single prompt as equivalent to a complete initial bank

### 6.2 Post-answer transition truth

Required proof:

- after a `prompt_response`, the runtime reaches one truthful next state:
  refreshed bank, build-ready, or explicit error/attention
- there is no route-visible dead-end that depends on unsupported hidden
  category navigation

### 6.3 Dynamic expansion truth

Required proof:

- answers that create newly derivable questions or categories are reflected
  back through the same prompt-bank contract
- queued-to-banked transitions and newly added thread/category state preserve
  thread identity and usable route focus

### 6.4 Resume and replay truth

Required proof:

- checkpoint resume restores a bank-first route-compatible state instead of
  reopening a legacy single-prompt-first path
- reconnect or reload during an in-progress session replays truthful banked
  state and does not strand the route in a hidden or stale state

### 6.5 Parallel prompt-bank truth

Required proof:

- the bounded parallel bank-generation path preserves deterministic bank
  completeness for the currently derivable work
- stable thread identity is preserved even when multiple prompt-ready threads
  are generated in the same bank build

### 6.6 Startup transport truth

Required proof:

- the active Solid Socratic startup path depends on `start_socratic`, not
  `start_pipeline`
- any temporary server compatibility for `start_pipeline` remains a migration
  shim, not the active contract under test

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-server/tests/server_integration.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-core/tests/*` or `planner-core/src/pipeline/steps/socratic/*` test
  modules where the cleanest proof belongs
- `planner-solid/e2e/*` covering reconnect/reload and truthful post-answer
  progression
- `planner-solid/src/lib/*.test.ts` where local merge or startup semantics need
  tighter proof
- `docs/project-plan.md`

## 8. Acceptance Criteria

This remediation slice is complete only when:

1. the repo contains explicit verification for fresh waiting-session startup
   into truthful bank-first reveal
2. the repo contains explicit verification for post-answer progression into the
   next bank or build-ready state
3. the repo contains explicit verification for dynamic post-answer generation
   of new questions/categories through the same route-facing contract
4. the repo contains explicit verification that legacy single-prompt state does
   not reassert itself as false first-reveal truth
5. the repo contains explicit verification for reconnect or reload during an
   in-progress interview without dead-end or unsupported hidden state
6. the repo contains explicit verification for the bounded parallel prompt-bank
   path preserving complete/stable bank output
7. the repo contains explicit verification that the active Solid startup path
   uses `start_socratic`, not `start_pipeline`
8. planning and repo status surfaces can describe Phase 24 as implemented and
   Phase 25 as the closeout-proof slice without overstating closure

## 9. Verification Plan

### Rust integration coverage

- fresh waiting-session startup into bank-first reveal
- post-answer progression into next bank
- post-answer progression into build-ready
- dynamic post-answer category/question insertion through the same truthful
  contract
- checkpoint resume into truthful bank-first state
- no false first-reveal completeness from legacy single-prompt state
- bounded parallel prompt-bank generation preserving stable identities

### Rust/unit closeout

- prompt-bank response shaping under legacy checkpoint input
- replay-state construction under resume/reconnect conditions
- status computation under startup, assembly, awaiting-response, build-ready,
  and attention-required states

### Solid/unit coverage

- startup handshake message semantics
- local prompt-bank merge behavior for queued-to-banked transitions
- route gating under bank/build-ready/error states with reduced refetch
  assumptions

### Browser coverage

- startup into first truthful bank reveal
- answer commit into next truthful state
- answer commit into dynamically added question/category state
- reconnect or reload during in-progress interview with truthful bank replay
- no active-path dependency on `start_pipeline`

## 10. Rollback / Fallback

If one proof surface proves awkward to land in the first pass, fallback may
narrow only the *location* of the proof, not the proof requirement itself.

Allowed examples:

- a browser scenario may be proven by a narrower but still route-realistic
  mocked websocket harness if full backend orchestration would be too brittle
  for the first pass
- a parallel-path proof may live in Rust integration or focused engine-level
  tests, whichever gives the clearest deterministic assertion

Not allowed:

- dropping one of the listed proof obligations and still treating the
  remediation slice as complete
- replacing end-to-end contract proof with compile-only or snapshot-only checks

## 11. Open Questions

These should be answered during delivery, but they do not block readiness:

- should the reconnect/reload proof be a browser test only, or paired with a
  server integration test for checkpoint replay?
- where is the cleanest deterministic home for the bounded parallel
  prompt-bank proof: engine-level tests, server integration, or both?
- should Phase 24 planning text gain a small implementation note pointing to
  Phase 25 as verification closeout once this slice lands?

## 12. Readiness Judgment

This remediation spec is **ready for implementation**.

The slice is bounded, repo-specific, and does not require another architecture
decision:

- the missing proof areas are already known from the Phase 24 verification plan
- the runtime contract under test is already selected and implemented
- the remaining work is verification hardening plus planning-sync, not feature
  redefinition
