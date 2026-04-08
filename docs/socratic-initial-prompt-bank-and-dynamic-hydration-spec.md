# Socratic Initial Prompt Bank And Dynamic Hydration Spec

**Status:** active  
**Date:** 2026-03-24  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Related Planning:** [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md), [Socratic Lobby First-Reveal Preload Gate Spec](/home/thetu/planner/docs/socratic-lobby-first-reveal-preload-gate-spec.md), [Socratic Hybrid Question Routing And Latency Spec](/home/thetu/planner/docs/socratic-hybrid-question-routing-and-latency-spec.md), [Socratic Lobby Local-First Browser Architecture Review](/home/thetu/planner/docs/socratic-lobby-local-first-browser-architecture-review.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)

> Planning note (2026-03-24): the currently implemented master-detail shell is
> still the correct live baseline, but the startup content contract is not.
> This spec replaces the current "one real prompt plus preview shells"
> behavior with a true initial prompt-bank contract. It now functions as a
> route-level product-requirement child under
> [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)
> and
> [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md),
> not as an isolated implementation-ready React slice.

## 1. Executive Judgment

The current Socratic lobby still breaks the core product promise.

The shell is now materially better: the route is master-detail, thread
switching is local, and the workspace no longer behaves like a giant article.
But the first-reveal experience is still structurally wrong because the backend
only materializes one real prompt at startup while the frontend renders the
rest of the thread index from preview hints and prompt-ready shells.

That produces the exact broken experience the user is reporting:

- one real question appears
- many sibling rows imply ready local work through `[0/1]` or `loading`
- clicking another thread often lands on `Awaiting questions...`
- the page feels remote and half-loaded instead of local and complete

The selected replacement direction is:

- keep the master-detail page model
- keep server-authored question generation
- generate an initial bank of fully materialized prompts before the first lobby
  reveal
- continue dynamic updates after that initial reveal without regressing back to
  shell-only rows that pretend to be loaded

## 2. Problem & Current Failure Mode

### Observed product failure

The current route reveals the lobby while only one active prompt is truly
materialized. The user can then see many categories and question counts that
read as actionable even though their prompt payloads do not exist yet.

This violates the intended product behavior:

- the first view is not coherent
- local thread switching is not truthful
- the user cannot trust that a visible thread is actually answerable

### Current-state evidence

- the frontend preload gate currently counts preview-only items toward
  `knownQuestionCount` in
  [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- the document graph records `has_prompt_ready` and `itemCountHint` from
  category nodes, but those are not the same thing as materialized prompts in
  [planner-web/src/stores/socraticDocumentStore.ts](/home/thetu/planner/planner-web/src/stores/socraticDocumentStore.ts)
- the workspace telemetry still derives totals from `item_count_hint` and
  prompt-ready flags in
  [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- the backend currently auto-enters the first prompt-ready leaf and plans a
  prompt batch only for that active leaf in
  [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)

The architectural result is clear:

- one real `current_prompt`
- many preview or prompt-ready shells
- a preload gate that treats shells as loaded enough

That is the wrong abstraction for the intended lobby.

## 3. User Outcome

After this slice:

- the first Socratic lobby view appears only when a real prompt bank exists
- every answerable row visible at first reveal has a fully materialized prompt
  behind it
- clicking a visible thread swaps the right workspace immediately with no
  spinner and no `Awaiting questions...` surprise
- dynamic category and question updates still remain possible after first
  reveal
- new work can appear incrementally later, but the initial reveal feels
  complete and local rather than partial and deceptive

## 4. Investigation Summary

### Option A: current model, one prompt plus previews

Keep generating a single active prompt and use preview rows plus prompt-ready
hints to populate the rest of the thread index.

**Rejected**

Why:

- this is the exact flow the user is rejecting
- it makes the page appear more complete than it is
- it cannot honestly satisfy a "fully fleshed question bank at page load"
  requirement

### Option B: client-synthesized bank from previews

Render synthetic client-side question placeholders from workspace previews until
the server later generates the full prompt.

**Rejected**

Why:

- clients would be inventing question content
- question wording, options, and prompt semantics would drift from server truth
- the product would become harder to reason about and debug

### Option C: server-authored initial prompt bank plus dynamic hydration

Generate a prompt bank for the initial prompt-ready threads before the first
lobby reveal, deliver that bank through session/websocket state, and continue
to append or update later prompts dynamically.

**Selected**

Why:

- preserves backend authority for prompt content
- keeps first reveal coherent
- supports local-fast browsing among known threads
- still allows new categories and prompts to appear over time

### Option D: fully pre-generate every future prompt before reveal

Attempt to generate all future possible prompts or all latent follow-up
questions before the user sees the first lobby.

**Rejected**

Why:

- not truthful to the branching nature of Socratic intake
- too expensive and slow
- many later prompts depend on user answers that do not exist yet

## 5. Product Decision

The first lobby reveal must be gated on a **server-authored initial prompt
bank**.

For this route:

- "loaded" means a prompt envelope exists in local client state for that thread
- "prompt-ready" alone does not count as loaded
- preview-only items do not count as loaded
- the first reveal must not show thread rows that imply answerability unless
  their prompt payload already exists locally

The product model becomes:

1. classification and category synthesis
2. initial prompt-bank assembly
3. first lobby reveal from the bank
4. dynamic background updates as answers create new work

## 6. Scope Boundaries

### In Scope

- defining the initial prompt-bank contract for the Socratic lobby
- changing backend startup behavior from single-prompt generation to bank
  generation
- updating session/checkpoint/websocket transport so the bank is durable and
  resumable
- updating preload-gate logic so only real banked prompts count toward reveal
- updating thread-index semantics so visible answerable rows are truthful
- preserving dynamic post-reveal category/question updates
- adding observability for bank size, bank assembly duration, and bank fill
  source

### Out Of Scope

- replacing the selected master-detail shell
- reintroducing the continuous document workspace model
- client-authored question synthesis
- generating all future hypothetical prompts before user input exists
- redesigning unrelated Planner routes

## 7. Backend Contract

### Initial bank assembly

After category/workspace synthesis succeeds, the backend must assemble an
initial prompt bank before first reveal.

The bank must be built from the full set of initial prompt-ready threads, not
just the first active leaf.

Minimum contract:

- gather all initial prompt-ready leaf categories visible in the first
  workspace/category snapshot
- generate one full prompt envelope per prompt-ready leaf
- preserve per-thread identity so the client can mount the chosen thread
  instantly
- do not emit a "lobby ready" state until the initial bank contract is met or a
  bounded fallback/error path is triggered

### Bank completeness rule

The selected default is strict:

- every thread that is visible as answerable in the initial thread index must
  already have a banked prompt

Allowed initial index rows:

- banked prompt threads
- non-answerable structural rows that are clearly non-interactive
- explicit queued rows only if the product copy makes it clear they are not
  locally available yet

Disallowed initial index rows:

- `[0/1]` or similar counters derived only from hints
- clickable rows that open into `Awaiting questions...`
- generic `loading` rows presented as if they are part of the local question
  bank

### Generation strategy

The initial bank must not be built as one serial deep-model queue.

The implementation must use the existing latency work as a foundation:

- deterministic scaffolds where applicable
- fast model lane as the default
- deep model lane only for complex/escalated cases

The planner must also support bounded parallel bank generation for independent
threads.

The exact concurrency limit can be implementation-configurable, but the first
implementation must not serialize all initial thread prompts when they could be
generated independently.

### Session and transport shape

The single `current_prompt` steady-state model is no longer sufficient for
first reveal.

The backend contract must support:

- an initial `prompt_bank` or equivalent per-thread prompt map
- durable checkpoint persistence for that bank
- websocket/session updates that can:
  - seed the initial bank
  - append a new banked thread
  - replace an existing thread prompt
  - retire or invalidate stale bank entries when answers change dependencies

The route may still maintain one `activeThreadId` or server-preferred focus,
but the data transport must carry more than one real prompt at startup.

## 8. Frontend Contract

### First-reveal gate

The preload gate must count only fully materialized banked prompts.

It must not count:

- preview-only items
- `item_count_hint`
- `has_prompt_ready`
- shell rows inferred from category state

The first reveal opens only when one of these is true:

- the initial prompt bank is complete
- build-ready is reached with no questions required
- the route enters an explicit error state

The old soft partial reveal behavior must not reopen the current "one real
question plus shells" failure mode.

### Thread index truthfulness

The left index must distinguish clearly between:

- banked answerable threads
- structural non-answerable nodes
- queued future work

Only banked answerable threads may present answer counts or local-ready
interaction affordances.

### Local-fast browsing

Once first reveal happens:

- clicking any banked thread must swap the workspace synchronously on the
  client
- no websocket acknowledgment is required for known-bank thread switching
- the right workspace must never show `Awaiting questions...` for a thread that
  is represented as locally banked

### Dynamic updates after reveal

After first reveal:

- new categories may appear
- new threads may become banked later
- existing bank entries may update as answers change the prompt graph

The client must merge those changes without regressing visible banked threads
back to misleading shell-only states.

## 9. State & Data Model Expectations

The normalized client model must be widened from "known questions plus active
thread" to an explicit bank shape.

Minimum client state semantics:

- `activeThreadId`
- `threadsById`
- `threadOrder`
- `promptBankByThreadId`
- `questionsById`
- `questionIdsByThread`
- `draftsByQuestionId`
- `queuedThreadIds`
- `bankAssemblyState`

The client must be able to answer these questions cheaply:

- is this thread banked?
- is this thread only structural?
- is this thread queued for later generation?
- can the workspace render this thread immediately with no further fetch?

## 10. Visual/Copy Contract

The route must stop implying that queued or structural work is loaded local
content.

Required copy behavior:

- initial loading page should state that Planner is assembling the first
  question bank
- banked threads should read as ready local work
- queued work should say `Queued` or similar, not `loading` or `[0/1]`
- empty active-thread copy must be rare; for banked threads it should not
  happen at all

## 11. Acceptance Criteria

The slice is complete only when all of the following are true:

1. a fresh Socratic session does not reveal the lobby until the initial prompt
   bank exists or the route enters a truthful non-success state
2. every thread shown as locally answerable at first reveal has a fully
   materialized prompt envelope in client state
3. switching between initial banked threads is purely local and immediate
4. the frontend no longer counts preview hints toward "loaded enough" reveal
   logic
5. the backend no longer limits initial prompt generation to one active leaf
6. prompt-bank data persists across checkpoint/resume
7. later dynamic updates can add or revise bank entries without breaking local
   navigation for already-banked threads
8. the thread index no longer shows fake `[0/1]` counters or equivalent for
   shell-only rows

## 12. Verification Plan

### Backend

- unit coverage for initial prompt-ready leaf collection across a category
  snapshot
- unit coverage for prompt-bank assembly from multiple initial threads
- unit coverage for bank persistence in checkpoint state
- routing coverage proving scaffold and fast-lane use still apply during bank
  generation

### Frontend

- preload-gate coverage proving preview-only items do not count toward first
  reveal
- workspace/index coverage proving only banked threads are locally answerable
- resume coverage proving a checkpointed bank rehydrates into local-fast thread
  switching
- regression coverage proving banked thread switching never lands on
  `Awaiting questions...`

### Browser verification

- real-browser proof that first reveal waits on the bank and then opens into a
  fully navigable local workspace
- proof that clicking among banked threads does not show per-thread loading
- proof that later dynamic thread additions merge into the index without
  regressing the visible banked set

## 13. Rollback / Fallback

If the full prompt-bank transport cannot be landed in one slice, the system
must degrade truthfully:

- reveal fewer threads, not more fake ones
- show only the actually banked thread set
- keep non-banked prompt-ready candidates out of the local-ready thread index

Disallowed fallback:

- keeping the current preview-shell behavior and relabeling it as complete

## 14. Open Questions

These do not block implementation, but they should be resolved during delivery:

- should queued non-banked future work appear in the same index list or in a
  separate collapsed `Queued` subsection?
- should the initial bank always include every prompt-ready leaf, or should
  there be a server-configurable hard cap for very large sessions with truthful
  overflow handling?
- should later dynamic prompt-bank additions visibly animate in the thread
  index, or remain visually quiet by default?

## 15. Readiness Judgment

This spec is **ready for implementation**.

The key architectural failure is now identified precisely:

- current product contract: one prompt plus previews
- required product contract: initial bank plus dynamic hydration

The shell direction, routing groundwork, and normalized client state already
exist. What remains is to widen the runtime contract and make the UI truthful
to it. That is a bounded implementation problem, not an ambiguous strategy
question.
