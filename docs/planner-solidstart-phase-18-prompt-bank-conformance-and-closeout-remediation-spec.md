# Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 00 Shell, Sessions, And Socratic Anchor Spec](/home/thetu/planner/docs/planner-solidstart-phase-00-shell-sessions-and-socratic-anchor-spec.md), [Planner SolidStart Phase 17 Workflow Closeout And React Retirement Spec](/home/thetu/planner/docs/planner-solidstart-phase-17-workflow-closeout-and-react-retirement-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Audit:** 2026-03-24 implementation-completeness and spec-conformance review of the SolidStart migration against current repo state

> Implementation note (2026-03-24): this remediation slice is now delivered.
> The backend persists and replays a real prompt bank, the Solid session route
> runs on a local normalized prompt-bank graph with capability-driven controls,
> verification now covers the widened Rust contract plus Solid unit/build/e2e
> surfaces, and repo docs no longer present `planner-web` as the routine active
> frontend target.

## 1. Executive Judgment

The next SolidStart slice should not widen the route family again.

The route family is already broad enough.

The problem is now conformance and truthfulness:

- the Socratic route still ships a one-current-prompt-plus-queued-thread model
  instead of the required full initial prompt bank
- the Solid session workspace still uses a thin resource-refetch model instead
  of the required local prompt-bank graph
- critical verification is still mocked or too narrow to prove the required
  runtime behavior
- the repo still overstates both React retirement and tranche completion

This slice should therefore be a remediation pass, not a new route phase.

## 2. User Outcome

After Phase 18:

- the Solid Socratic workspace does not reveal partial answerable state
- every answerable thread visible at first reveal has a real banked prompt
- thread switching across banked work is immediate and local
- websocket updates can append, replace, and invalidate bank entries without
  corrupting active work
- session workflow controls reflect backend capability truth
- repo docs and planning surfaces describe the Solid migration honestly
- the SolidStart tranche can be marked complete without relying on claims the
  code and tests do not support

## 3. Problems To Solve

### 3.1 Prompt-bank contract drift

The current backend and Solid route still behave like:

- one durable `current_prompt`
- zero or one banked thread
- additional prompt-ready work represented as queued future rows

That is not the selected product contract.

The selected contract requires:

- a full initial prompt bank before first reveal
- one real prompt envelope per visible answerable thread
- more than one real prompt carried in transport and checkpoint state at
  startup

### 3.2 Local workspace model drift

The current Solid route is structurally present, but it still relies on:

- `getSession`
- `getPromptBank`
- coarse refetch on websocket events
- a narrow drafts store keyed by prompt/item

That is enough for a shell, but it is not the required local graph model for a
dense, native-feeling Socratic workspace.

### 3.3 Verification drift

The current tests prove that the route renders and that mocked payloads can be
displayed.

They do not yet prove:

- full-bank first reveal
- instant switching across multiple banked threads
- dirty draft retention on thread switch
- websocket-driven prompt-bank insertion or replacement stability
- active workspace ownership under background churn

### 3.4 Closeout truthfulness drift

The repo still overstates closure in two ways:

- Phase 00 and top-level tracking language describe the prompt-bank endpoint
  and verification as if they already match the spec
- repo docs still present `planner-web` as an active frontend target in some
  routine development and deployment paths

### 3.5 Session capability truth drift

The backend already computes workflow capability fields, but the Solid client
does not consume them yet.

The session route still derives some action visibility from local heuristics,
which is weaker than the backend truth model and can expose actions that the
API will reject.

## 4. Scope

### In Scope

- backend prompt-bank transport changes needed to deliver a full initial prompt
  bank
- durable checkpoint changes needed to persist that bank honestly
- websocket/session update changes needed to seed, append, replace, or retire
  bank entries truthfully
- Solid session-route state redesign around the required local prompt-bank
  graph
- capability-driven rendering for session lifecycle actions already present in
  Solid
- live verification for the Socratic prompt-bank contract and workflow loop
- repo-doc and planning-surface truthfulness updates needed to stop overstating
  closure or React retirement

### Out Of Scope

- widening the Solid route family beyond the current route set
- deleting `planner-web` from the repository
- auth redesign
- unrelated project/import/knowledge/blueprint feature expansion
- speculative UI redesign beyond what the existing Socratic specs require

## 5. Current-State Evidence

The 2026-03-24 review established:

- the active frontend runtime/build path is already Solid
- the major Solid route family exists
- the Socratic route still derives `banked_threads` from a single
  `current_prompt`
- the session checkpoint persists one `current_prompt` or one category snapshot,
  not a durable multi-thread bank
- the Solid session route renders as soon as any one banked thread exists and
  explicitly treats the rest as queued future hydration
- the Playwright route tests are primarily mocked, so they do not prove the
  real Solid-to-Rust contract for the core Socratic behavior
- several top-level docs still describe `planner-web` as an active development
  target

This spec is the bounded response to that evidence.

## 6. Product And Technical Contract

### 6.1 Initial prompt-bank truth

The backend must not reveal the Socratic workspace until one of these is true:

1. the initial prompt bank is complete
2. build-ready is reached with no questions required
3. the route is in an explicit error state

For the normal first-reveal path:

- every answerable thread visible in the initial thread index must already have
  a banked prompt
- queued rows may represent future or structurally unavailable work, but not
  visible answerable threads that still lack a real prompt
- the `/sessions/{id}/prompt-bank` contract must stop presenting a one-real-
  prompt startup as if it were prompt-bank-complete

### 6.2 Durable bank transport and checkpoint state

Checkpoint and transport state must support more than one prompt at startup.

Minimum backend responsibilities:

- persist the initial bank, not just one active prompt
- preserve per-thread identity for banked prompts
- expose active-thread focus separately from prompt storage
- support websocket or session updates that can:
  - seed the initial bank
  - append a newly banked thread
  - replace a banked prompt when upstream answers change dependencies
  - retire or invalidate stale bank entries when they are no longer truthful

Compatibility shims for older one-prompt checkpoint data may exist during
migration, but they must not define the future-state contract.

### 6.3 Solid local graph contract

The Solid Socratic route must own a local reactive graph with at least:

- `activeThreadId`
- `threadsById`
- `threadOrder`
- `promptBankByThreadId`
- `questionsById`
- `questionIdsByThread`
- `draftsByQuestionId`
- `queuedThreadIds`
- `workspaceSyncState`

The route may still refetch when it is the cleanest recovery path after an
error, but the primary interactive model must be local graph merge rather than
full-resource reload as the default response to websocket traffic.

### 6.4 Interaction and input-isolation contract

After first reveal:

- switching among already banked threads must be immediate on the client
- a banked thread must never render as `Awaiting questions...`
- dirty draft state must survive banked-thread switching safely
- incoming server updates must not steal focus from the active editor
- background prompt-bank changes must not demote visible banked work back into
  misleading shell rows

### 6.5 Session capability truth

The Solid session route must render lifecycle controls from backend capability
truth where those fields already exist.

Minimum requirement:

- the Solid client types must carry the backend capability fields
- action visibility and enabled state for restart/retry must be driven by those
  fields instead of local phase heuristics where the backend already computes
  truth

### 6.6 React retirement and planning truth

This remediation slice must also close the truthfulness gap in docs and
planning:

- repo-level docs and routine workflow notes must stop presenting
  `planner-web` as an active frontend target where Solid already replaces it
- `project-plan.md` must stop describing the SolidStart tranche as closed while
  the prompt-bank and verification gaps remain open
- Phase 00 and Phase 17 must not remain marked as fully implemented if their
  key acceptance criteria are still unmet at the time the docs are updated

The goal is not to erase history.

The goal is to make the repo truthful about what is active, what is historical,
and what remains under remediation.

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/tests/*` covering prompt-bank and capability truth
- `planner-solid/src/routes/sessions/*`
- `planner-solid/src/lib/*` for prompt-bank graph helpers and types
- `planner-solid/e2e/*` for real contract verification
- repo docs and planning docs that still overstate closure or React retirement

## 8. Acceptance Criteria

This phase is complete only when:

1. the backend can expose a full initial prompt bank for the first-reveal state
   instead of a single prompt plus queued answerable threads
2. the durable checkpoint and websocket/update path can represent prompt-bank
   seed, append, replacement, and invalidation truthfully
3. the Solid session route uses the required local prompt-bank graph and can
   switch among banked threads synchronously
4. dirty draft state survives banked-thread switching and background prompt-bank
   churn without losing active workspace ownership
5. restart/retry action rendering in Solid follows backend capability truth
6. browser verification proves the real first-reveal and switching contract
   against the live server, not only mocked route fixtures
7. repo docs and planning surfaces no longer present `planner-web` as the
   active frontend where Solid is now the replacement
8. `project-plan.md`, Phase 00, and Phase 17 use status language that matches
   the actual implementation and verification state
9. the tranche can be described as complete without contradicting the Solid
   product contract selected in the platform-direction and Socratic specs

## 9. Verification Plan

### Backend contract verification

Add or update focused Rust coverage for:

- full-bank startup behavior on `/sessions/{id}/prompt-bank`
- checkpoint persistence of multi-thread bank state
- websocket/category/prompt transitions that append, replace, and retire banked
  entries truthfully
- session capability mapping where Solid consumes backend truth

### Solid verification

Add or update Solid tests for:

- prompt-bank graph helpers and state transitions
- first-reveal gate behavior
- banked-thread switching and draft retention
- truthful queued-to-banked transitions
- capability-driven lifecycle action rendering

### Browser verification

The critical browser proof must run against the real server contract for the
Socratic route, not only mocked fixture payloads.

Minimum proof:

1. open a session whose initial snapshot exposes multiple answerable threads
2. verify first reveal does not occur until those threads are actually banked
3. switch among banked threads with no per-thread loading round trip
4. retain dirty input while switching between banked threads
5. receive at least one background prompt-bank update without losing active
   workspace ownership
6. exercise the project -> session -> import workflow loop inside Solid after
   the capability-truth and React-retirement doc updates land

### Planning and doc review

Confirm after implementation that:

- the child specs and tracker no longer overstate prompt-bank completion
- repo docs consistently present `planner-solid` as the active frontend target
- any remaining historical `planner-web` references are clearly framed as
  historical baseline or migration source, not routine developer workflow

## 10. Rollback / Fallback

If this remediation slice reveals that the full-bank backend change is too
large for one pass:

- do not re-close the tranche on partial-reveal behavior
- keep the status language downgraded and explicit
- split a narrower follow-on remediation only after the current gap is bounded
  precisely

Disallowed fallback:

- keeping the one-current-prompt model while relabeling it as prompt-bank
  complete
- treating mocked route tests as sufficient proof for the live Socratic
  contract

## 11. Open Questions

None block readiness.

The gaps are already identified concretely enough to support bounded
implementation.

## 12. Readiness Judgment

This spec is **ready for implementation**.

The missing work is now bounded:

- one backend truth correction
- one Solid state-model correction
- one verification correction
- one planning/doc truthfulness correction

The repo should execute this remediation before claiming the SolidStart tranche
is complete.
