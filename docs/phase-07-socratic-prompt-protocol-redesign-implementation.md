# Phase 07 Socratic Prompt Protocol Redesign Implementation

**Status:** Implemented except scheduled post-window legacy-adapter removal  
**Date:** 2026-03-09

> Verification update (2026-03-18): adapter removal is still intentionally
> deferred. The recorded migration-window gate in this document requires an
> earliest removal date of `2026-04-30` plus 14 consecutive UTC days with zero
> persisted checkpoints containing legacy `current_question` or
> `pending_draft` fields. The legacy read adapter and promotion tests are still
> present in `planner-server/src/session.rs`, so deletion is not yet eligible.

## Implementation Status Snapshot (2026-03-09)

Completed:

- Phase 0 prompt domain model primitives are implemented in shared schema code.
- Session checkpoints now persist a single `current_prompt` envelope.
- Historical checkpoint read-time promotion exists for legacy
  `current_question` and `pending_draft`.
- Phase 1 transport primitives are active for websocket/session payloads:
  `prompt`, `prompt_response`, and `ui_capabilities`.
- Phase 2 backend engine refactor is partially implemented:
  - `socratic_engine` now runs a prompt-loop (`PromptEnvelope` out,
    `PromptResponse` in) instead of a strict single-question blocking loop.
  - `ResumePendingPrompt` now stores prompt envelopes.
  - prompt protocol and prompt batch planner modules were added.
  - stable prompt item ordering is preserved during response handling.
  - partial submissions are accepted and unanswered items remain eligible for
    reissue by subsequent batch planning.
- Web shared types and detached-checkpoint hydration now understand
  `current_prompt`.
- Phase 5 core pipeline visibility plumbing is implemented:
  - `planner-core` now emits structured pipeline stage/retry/validation/artifact
    events through a shared event sink.
  - `planner-server` records pipeline events into session state in real time and
    derives stage transitions from event metadata.
  - Socratic prompt lifecycle events now include generated/submitted/partial
    submitted/reissued/invalidated/adjudicated coverage.
  - `planner-web` updates stage state from `planner_event` metadata, removed
    optimistic done/start pipeline transcript text, and auto-foregrounds the
    events panel when pipeline execution starts.
- Phase 6 draft-review in-flow implementation is mostly delivered:
  - draft review is emitted as `PromptEnvelope.kind = "draft_review"` with
    `draft_snapshot`
  - draft-review prompts now include section review items, unresolved gap
    items (`not_discussed`), and unresolved verification concerns from
    `belief_state.uncertain`
  - assumptions are no longer rendered as a first-class user-facing section in
    draft UI
  - session draft view remains visible in the right pane while prompt answers
    are submitted through `PromptResponse`
- Phase 7 core cutover/deletion is mostly implemented:
  - legacy websocket runtime paths were removed:
    `speculative_draft`, `draft_reaction_ack`, `skip_question`,
    `draft_reaction`
  - compatibility event/projection paths were removed:
    `SocraticEvent::Question`, `SocraticEvent::SpeculativeDraftReady`
  - checkpoint projection now relies on `SocraticEvent::PromptGenerated`
    envelopes only
  - historical checkpoint read-time promotion (`current_question`,
    `pending_draft` -> `current_prompt`) remains intentionally active

Delivered after the 2026-03-08 snapshot:

- `W1 Draft Planner Unification`
  - draft-review candidate selection now runs through the shared
    contradiction/verification/discovery picker
  - mixed-priority draft coverage exists in `prompt_batch_planner`
- `W2 Typed Prompt Runtime`
  - websocket/runtime intake is typed end to end through
    `SocraticRuntimeInput::PromptResponse(PromptResponse)`
- `W3 Prompt Identity Hardening`
  - stable reissue identity and explicit replacement identity semantics are in
    place and covered by tests
- `W4 Pipeline Visibility Polish`
  - retry feedback and artifact persistence are surfaced as first-class session
    summaries
  - stage hydration now falls back gracefully when canonical metadata keys are
    missing
- `W5 Legacy Cleanup`
  - compatibility-only web hook API surface was removed from production paths
  - legacy websocket integration fixtures were migrated to `prompt` /
    `prompt_response`
  - migration-window exit criteria are recorded in this document
- `W6 Validation Proof`
  - benchmark output is now recorded for the high-answer-count adjudication
    harness
  - retry-heavy web stage-coherence coverage and stronger reconnect-heavy server
    replay coverage are part of the validation snapshot below
  - the current validation command matrix is recorded in this document
- `planner-tui` prompt-envelope migration landed and no longer blocks Phase 07
  validation

Actual remaining work:

- Scheduled migration cleanup
  - historical checkpoint read-time promotion remains intentionally active until
    the migration-window criteria below are met
  - adapter deletion is a post-window cleanup step, not current implementation
    work before the recorded earliest removal date

## Known Risks and Incomplete Areas

- The recorded benchmark snapshot is a local-machine baseline for deterministic
  direct-effect adjudication, not a cross-environment SLA.
- Legacy checkpoint read adapters are intentionally retained; removal is
  deferred until the migration window closes.
- The source implementation doc and execution prompt can drift if follow-up work
  lands without updating both documents in the same change.

## Remaining Work Snapshot (2026-03-09)

| Workstream | Current gap | Exit signal |
| --- | --- | --- |
| Migration cleanup | Historical checkpoint promotion remains active by design until the migration-window threshold and date are met. | A post-window PR removes the legacy adapter only after the recorded criteria are satisfied. |

## Current Validation Snapshot (2026-03-09)

Targeted validation currently green:

- `cargo test -p planner-core prompt_batch_planner -- --nocapture`
- `cargo test -p planner-core benchmark_high_answer_count_prompt_batch_adjudication -- --ignored --nocapture`
- `cargo test -p planner-server tier2_socratic_ws_ -- --nocapture`
- `cargo test -p planner-tui -- --nocapture`
- `npm --prefix planner-web test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/MessageInput.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx`

Recorded benchmark snapshot:

- `benchmark.prompt_batch_adjudication iterations=200 total_ms=11 per_iteration_ms=0.055`
- fixture shape: 24 verification items with deterministic direct effects

Targeted soak-style evidence:

- reconnect-heavy prompt replay remains stable across 8 attach/detach cycles
  before answer submission (`tier2_socratic_ws_reconnect_heavy_cycles_keep_prompt_replay_stable`)
- retry-heavy planner-event stage coherence remains green in the web hook suite
  (`handles retry-heavy planner_event sequences without losing stage coherence`)

## Outstanding Items To Complete

Use these labels consistently in follow-up implementation and status reporting.

- Delivered workstreams:
  - `W1 Draft Planner Unification`
  - `W2 Typed Prompt Runtime`
  - `W3 Prompt Identity Hardening`
  - `W4 Pipeline Visibility Polish`
  - `W5 Legacy Cleanup`
  - `W6 Validation Proof`
- Scheduled migration cleanup
  - keep historical checkpoint promotion active until the migration-window exit
    criteria are met
  - remove the adapter in a dedicated post-window PR only after the threshold
    and date gate are satisfied

## Objective

Replace the current single-question Socratic websocket protocol with a
prompt-envelope protocol that supports:

- multiple visible questions at once
- partial submission
- deterministic serial belief-state updates on the backend
- verification-first behavior
- contradiction-first behavior
- draft review in the same interaction flow
- a full draft view in the UI without treating draft review as a separate mode
- visible, trustworthy pipeline progress after intake convergence

This phase is complete when the Socratic lobby no longer depends on
`current_question` plus free-form `socratic_response`, and the active interview
state is instead represented by a single structured `current_prompt` model, and
the pipeline transition emits real session-visible progress instead of looking
stalled until terminal success or failure.

## Non-Goals

- redesign project routing or project ownership
- redesign pipeline execution after intake convergence
- preserve the current Socratic websocket contract for active clients
- introduce true concurrent belief-state mutation
- convert the intake engine into a generic form builder
- optimize provider latency independent of protocol redesign

## User-Validated Product Decisions

These decisions were confirmed directly with the user during research:

- prompt batches may be submitted partially
- unanswered items may be re-asked later and may be reworded using new context
- batch size should be as large as fits on screen
- each item should support single-select plus custom text
- verification and contradiction resolution outrank new discovery questions
- draft review should stay in the same interaction flow, while still exposing a
  full draft view in the UI
- the current websocket contract may be broken during the redesign

## Decision Summary

- The current protocol is the wrong abstraction. It is built around one
  outstanding question, one outstanding reply, and one pending draft.
- The backend should remain serial when applying answers to the belief state.
  It should not mutate shared interview state concurrently.
- The frontend should support multiple simultaneously answerable prompt items.
- The backend should generate structured prompt batches, not rely on the client
  to simulate batching on top of a singular transport.
- Unverified facts should no longer surface as first-class `assumptions`.
  Uncertainty should become explicit verification prompts.
- Draft review should become one prompt kind inside the unified protocol, with
  the full draft rendered alongside the active review items.
- The client should advertise UI capacity so the server can size prompt batches
  to the available screen real estate.
- The transition from Socratic intake to pipeline execution must share the same
  event truth model, so users can see live stage starts, completions, retries,
  validation failures, and retry feedback in the session lobby.

## Historical Baseline (Pre-Phase 1/2)

The following baseline described the pre-cutover singular model:

- [QuestionOutput](/home/thetu/planner/planner-schemas/src/artifacts/socratic.rs#L484)
  models one question.
- [InterviewCheckpoint.current_question](/home/thetu/planner/planner-server/src/session.rs#L97)
  stores one question.
- [InterviewCheckpoint.pending_draft](/home/thetu/planner/planner-server/src/session.rs#L99)
  stores one separate draft prompt.
- [ServerMessage::Question](/home/thetu/planner/planner-server/src/ws.rs#L79)
  and [ClientMessage::SocraticResponse](/home/thetu/planner/planner-server/src/ws.rs#L137)
  define a one-question, one-answer websocket contract.
- The engine sends one question and then blocks on one
  `receive_input()` in
  [socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs#L345)
  and
  [socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs#L364).
- Resume logic is also singular through `ResumePendingPrompt::Question` or
  `ResumePendingPrompt::Draft` in
  [socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs#L84)
  and checkpoint projection in
  [ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs#L355).

That model causes three product problems:

1. The UI cannot truthfully expose several answerable items at once.
2. The server cannot represent partially completed batches or stale items.
3. Draft review, verification, and discovery are separate interaction paths
   instead of one coherent prompt system.

It also hides pipeline reality after the interview completes:

- [run_pipeline_for_session()](/home/thetu/planner/planner-server/src/api.rs#L1834)
  marks the pipeline as started and only writes session-visible state again on
  terminal success or failure.
- The pipeline core currently emits tracing logs rather than session-visible
  progress events during front-office execution and the factory/validation retry
  loop in
  [pipeline/mod.rs](/home/thetu/planner/planner-core/src/pipeline/mod.rs#L501)
  and
  [pipeline/mod.rs](/home/thetu/planner/planner-core/src/pipeline/mod.rs#L1110).
- The session page defaults to the belief-state tab during `pipeline_running`
  in
  [SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx#L165),
  which buries the only useful operational surface.
- The client also injects an optimistic `(Done — starting pipeline)` message in
  [useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts#L642)
  before any backend-confirmed pipeline progress event arrives.

## Proposed Architecture

### Core principle

The system should support multiple visible prompt items, but the backend should
apply answers in a stable serial order. This preserves determinism while fixing
the lobby UX.

### Replace singular prompt state with a prompt envelope

Introduce a new top-level active prompt model:

- `PromptEnvelope`
- `PromptKind`
- `PromptItem`
- `PromptOption`
- `PromptResponse`
- `PromptAnswer`
- `UiCapabilities`

Recommended `PromptKind` values:

- `question_batch`
- `verification_batch`
- `contradiction_batch`
- `draft_review`

Recommended `PromptEnvelope` shape:

```ts
type PromptEnvelope = {
  prompt_id: string
  kind: "question_batch" | "verification_batch" | "contradiction_batch" | "draft_review"
  title: string
  instructions?: string
  items: PromptItem[]
  draft_snapshot?: DraftSnapshot
  required_item_ids: string[]
  allow_partial_submit: boolean
  ui_hints: {
    preferred_layout: "cards" | "review"
    show_draft_sidebar: boolean
  }
  based_on_turn: number
  created_at: string
}

type PromptItem = {
  item_id: string
  kind: "discovery" | "verification" | "contradiction" | "draft_section"
  target_dimension?: string
  section_ref?: string
  text: string
  options: PromptOption[]
  response_mode: "single_select_with_custom_text"
  required: boolean
  priority: number
  dependency_item_ids: string[]
}

type PromptOption = {
  option_id: string
  label: string
  semantic_value: string
  direct_effect?: DirectEffect
}
```

Recommended response shape:

```ts
type PromptResponse = {
  prompt_id: string
  answers: PromptAnswer[]
  submitted_at: string
  client_context?: {
    viewport_class?: "mobile" | "tablet" | "desktop"
  }
}

type PromptAnswer = {
  item_id: string
  selected_option_id?: string
  custom_text?: string
  skipped?: boolean
}
```

### Client capability negotiation

The server cannot infer what "fits on screen" by itself. The client should send
UI capabilities on attach and when the viewport class changes.

Recommended shape:

```ts
type UiCapabilities = {
  viewport_class: "mobile" | "tablet" | "desktop"
  max_visible_items: number
  supports_split_draft_view: boolean
}
```

Rules:

- the web client computes `max_visible_items`
- the server uses that value as the upper bound when planning a batch
- the server may return fewer items when dependency or quality constraints
  require it
- mobile clients should naturally request smaller batches than desktop clients

### Batch planning rules

The batch planner should create a prompt from a prioritized pool:

1. unresolved contradictions
2. uncertain dimensions needing verification
3. draft-section corrections or confirmation items
4. new discovery questions

Selection rules:

- do not include items that depend on unanswered higher-priority items in the
  same batch
- do not include multiple competing items for the same dimension
- do include independent questions up to `max_visible_items`
- unanswered items return to the candidate pool after submission
- re-issued items may be reworded using newly learned context

### Prompt/item identity strategy

Prompt batches can only support safe partial submission, reissue, and stale-item
invalidation if identifiers remain stable for the same unresolved semantic
target.

Rules:

- `prompt_id` identifies one emitted envelope instance
- `item_id` should remain stable across reissue when the semantic target is the
  same and only wording/context changes
- if the semantic target changes materially, the old item should be invalidated
  and a new `item_id` emitted
- prompt lifecycle events should reference the same stable item identifiers used
  by the response path
- tests should distinguish "same item reworded" from "old item invalidated and
  replaced"

### Answer adjudication rules

Backend application remains serial:

1. accept the structured `PromptResponse`
2. ignore unanswered items for this turn
3. apply direct option effects without LLM where possible
4. interpret custom text and ambiguous replies through a targeted batch
   adjudication prompt
5. apply resulting updates in stable item order
6. run contradiction detection and convergence once after the batch is applied
7. generate the next `PromptEnvelope`

Important constraint:

- no concurrent writes to `RequirementsBeliefState`

Important optimization:

- if an option carries a deterministic `direct_effect`, do not spend an LLM
  call interpreting it

### Drafts in the same flow

Drafts should not be a detached mode with a separate wire contract.

Instead:

- `PromptEnvelope.kind = "draft_review"` can include `draft_snapshot`
- the UI renders the full draft in the right pane
- the active prompt items target draft sections, missing areas, or unresolved
  verification items
- unverified assumptions are not shown as a dedicated assumptions block
- section review answers are submitted through the same `PromptResponse`
  protocol as discovery and verification items

### Resume and checkpoint model

Replace:

- `current_question`
- `pending_draft`
- `ResumePendingPrompt`

With:

- `current_prompt: Option<PromptEnvelope>`
- `ResumePendingPromptEnvelope`

Checkpoint replay rules:

- the active prompt envelope is persisted directly
- reconnect re-emits `current_prompt`
- partial submissions are not persisted as hidden client state; only committed
  answers affect the checkpoint

### Event and observability model

Add prompt-oriented events:

- `socratic.prompt.generated`
- `socratic.prompt.submitted`
- `socratic.prompt.partial_submitted`
- `socratic.prompt.reissued`
- `socratic.prompt.invalidated`
- `socratic.response.adjudicated`
- `socratic.draft.generated`

These should replace the current question-centric assumptions baked into event
names and status text.

Add pipeline-oriented session events on the same channel:

- `pipeline.stage.started`
- `pipeline.stage.completed`
- `pipeline.stage.failed`
- `pipeline.retry.started`
- `pipeline.retry.feedback`
- `pipeline.validation.completed`
- `pipeline.artifact.persisted`

Rules:

- pipeline stage state in the session model should be updated from these events,
  not only from start/end snapshots
- pipeline retries and validation gate failures must be visible without reading
  tracing logs or disk state manually
- the session lobby should automatically foreground the events/build feed when
  the pipeline begins, or at minimum surface an explicit unread/live state
  stronger than the current default-belief view
- the UI should not synthesize a success-sounding transition message before the
  backend confirms pipeline start

## Protocol Changes

### Remove

- websocket server message `question`
- websocket client message `socratic_response`
- websocket draft reaction contract as a first-class input path
- singular checkpoint fields `current_question` and `pending_draft`

### Add

- websocket server message `prompt`
- websocket client message `prompt_response`
- websocket client message `ui_capabilities`
- session/checkpoint field `current_prompt`
- prompt-specific event metadata

### Historical persistence strategy

No active-client compatibility bridge is required, but persisted sessions on
disk still need a safe read path.

Recommended approach:

- provide read-time promotion for historical checkpoints:
  - `current_question` -> one-item `PromptEnvelope`
  - `pending_draft` -> `draft_review` `PromptEnvelope`
- once the migration has been stable for at least one release window, remove
  legacy checkpoint reads and associated tests

## Impacted Files And Modules

### Shared schema

- `planner-schemas/src/artifacts/socratic.rs`
- `planner-web/src/types.ts`

### Core intake engine

- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-core/src/pipeline/steps/socratic/question_planner.rs`
- `planner-core/src/pipeline/steps/socratic/belief_state.rs`
- `planner-core/src/pipeline/steps/socratic/speculative_draft.rs`

Recommended new modules:

- `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`

### Server runtime and checkpointing

- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-core/src/pipeline/mod.rs`

### Web client

- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/components/MessageInput.tsx`
- `planner-web/src/components/SpeculativeDraftView.tsx`
- `planner-web/src/components/BeliefStatePanel.tsx`

Recommended new components:

- `planner-web/src/components/PromptBatchPanel.tsx`
- `planner-web/src/components/PromptCard.tsx`
- `planner-web/src/components/PromptOptionGroup.tsx`
- `planner-web/src/components/DraftSidebar.tsx`

## Phased Implementation Plan

## Phase 0: Ratify The Prompt Domain Model

Goal:

- define the replacement protocol and state model in shared schema code

Implement:

- add `PromptEnvelope`, `PromptItem`, `PromptOption`, `PromptResponse`,
  `PromptAnswer`, and `UiCapabilities`
- add serde and TypeScript support
- add a read-time adapter for historical `current_question` and `pending_draft`

Likely files:

- `planner-schemas/src/artifacts/socratic.rs`
- `planner-web/src/types.ts`
- `planner-server/src/session.rs`

Done when:

- shared types compile
- session checkpoints can express one active prompt envelope
- historical checkpoint promotion is specified and tested

## Phase 1: Replace The Wire Contract

Goal:

- stop treating one question plus one free-form response as the transport truth

Implement:

- replace websocket `question` with `prompt`
- replace websocket `socratic_response` with `prompt_response`
- add websocket `ui_capabilities`
- update session REST payloads to expose `current_prompt`

Likely files:

- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/api.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`

Done when:

- the client can attach, receive a prompt envelope, and submit a structured
  prompt response
- checkpoint replay re-emits `current_prompt`

## Phase 2: Refactor The Engine Around Prompt Envelopes

Goal:

- replace the single-question blocking loop with a prompt loop

Implement:

- replace `ResumePendingPrompt` with a prompt-envelope equivalent
- replace question generation with batch planning
- replace per-turn singular input handling with prompt-response handling
- preserve deterministic serial update application

Likely files:

- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-core/src/pipeline/steps/socratic/question_planner.rs`
- new batch-planner and response-adjudicator modules

Done when:

- the engine no longer assumes one outstanding question
- partial prompt responses are accepted cleanly
- unanswered items can be reissued in the next prompt

## Phase 3: Deterministic Answer Application And Verification

Goal:

- reduce latency and ambiguity by separating direct effects from interpreted
  free text

Implement:

- support `direct_effect` mappings on prompt options
- add a batch adjudication pass for custom text answers
- process answers in stable order
- run contradiction and convergence once per submitted prompt

Likely files:

- `planner-core/src/pipeline/steps/socratic/belief_state.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`
- `planner-core/src/pipeline/steps/socratic/convergence.rs`

Done when:

- deterministic answers do not require LLM interpretation
- custom text can still update the belief state accurately
- latency is reduced relative to per-item free-text handling

## Phase 4: Rebuild The Lobby UI Around Prompt Batches

Goal:

- make the lobby truthfully render several answerable items at once

Implement:

- send `ui_capabilities` from the web client
- replace the singular `MessageInput` question UX with prompt cards
- support partial submission
- support single-select plus custom text on every item
- preserve free-form conversation history in the transcript

Likely files:

- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- new prompt-batch components

Done when:

- multiple visible prompt cards can be answered in one submit
- unanswered items do not block progress
- batch size responds to viewport capacity

## Phase 5: Add End-To-End Pipeline Visibility

Status (2026-03-08):

- Core implementation landed end to end across core runtime, API/session state,
  and web event consumption.
- Some UX hardening and observability polish remain open (tracked below).

Goal:

- ensure that once intake converges, the session lobby remains truthful about
  active pipeline work, retries, and failures

Implement:

- emit structured session-visible pipeline events from front-office, factory,
  validation, and retry paths
- persist those events through the same session event channel already used by
  Socratic
- update session stage statuses from those events
- expose retry feedback categories and latest persisted-artifact progress in the
  session UI
- remove optimistic client-side pipeline transition messaging that is not backed
  by server events
- auto-foreground the events/build feed when the pipeline starts, or apply an
  equally strong live-state treatment

Likely files:

- `planner-server/src/api.rs`
- `planner-core/src/pipeline/mod.rs`
- `planner-server/src/session.rs`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/hooks/useSocraticWebSocket.ts`

Implemented in this phase:

- Structured pipeline events now emit from front-office, validation, retry,
  and terminal failure paths in `planner-core/src/pipeline/mod.rs`.
- API pipeline runtime forwards those events through the same session event
  channel used by Socratic (`planner-server/src/api.rs`).
- Session stage state is now derived from pipeline event metadata
  (`planner-server/src/session.rs`).
- Socratic prompt lifecycle event coverage was expanded in
  `planner-server/src/ws_socratic.rs` to include submit/partial submit/reissue/
  invalidation/adjudication and draft generation signals.
- Web runtime now updates stage state directly from `planner_event` metadata,
  and the Session page auto-foregrounds Events during `pipeline_running`.
- Optimistic client-only pipeline start messaging was removed from
  `useSocraticWebSocket.sendDone()`.

Not fully implemented yet (risks/gaps):

- Retry feedback and artifact persistence are now elevated into dedicated
  summary widgets.
- Stage derivation now falls back when canonical metadata keys are missing.
- Retry-heavy and reconnect-heavy targeted coverage is in place, but exhaustive
  long-session soak beyond the validation snapshot remains intentionally modest.

Done when:

- a live retrying pipeline can be understood from the session UI alone
- stage progression and retry state are reflected without manual inspection of
  worktrees, run directories, or tracing logs

## Phase 6: Fold Draft Review Into The Prompt System

Status (2026-03-09):

- User-visible implementation and batch-planner unification landed.
- Reconnect-heavy coverage is in place.

Goal:

- unify discovery, verification, contradiction handling, and draft review

Implement:

- convert draft review into `PromptEnvelope.kind = "draft_review"`
- render the full draft in a dedicated sidebar or right-pane view
- tie prompt items to draft sections and unresolved gaps
- remove assumptions as a first-class user-facing section

Likely files:

- `planner-core/src/pipeline/steps/socratic/speculative_draft.rs`
- `planner-web/src/components/SpeculativeDraftView.tsx`
- `planner-web/src/pages/SessionPage.tsx`

Implemented in this phase:

- Draft review is emitted as `PromptEnvelope.kind = "draft_review"` with
  `draft_snapshot`.
- Draft-review prompts now include:
  - section review items
  - unresolved gap items from `not_discussed`
  - unresolved verification items from `belief_state.uncertain`
- The draft sidebar no longer renders assumptions as a first-class section.
- Draft review answers flow through `PromptResponse` in the same protocol path
  as other prompt items.

Not fully implemented yet (risks/gaps):

- Draft review now flows through the shared contradiction/verification/discovery
  prompt picker rather than a dedicated branch.
- Reconnect-heavy prompt replay coverage exists, though the overall soak matrix
  remains intentionally lighter than the targeted unit/integration suites.

Done when:

- the draft is visible without switching to a separate interaction model
- draft edits and confirmations flow through `PromptResponse`
- uncertainty appears as verification prompts rather than assumptions

## Phase 7: Remove Legacy Prompt Paths And Harden The System

Status (2026-03-09):

- Core legacy-path deletion and hardening landed.
- Historical adapter retention remains intentional until the migration-window
  criteria are met.
- Remaining work is validation closure plus post-window adapter removal.

Goal:

- finish the cutover and delete the old singular protocol

Implement:

- remove legacy websocket types and singular checkpoint fields
- remove one-question-only UI code paths
- remove draft-reaction-only protocol paths
- remove the internal serialized `PromptResponse` compatibility bridge so the
  runtime stays typed end to end
- replace transitional prompt/item identifiers with a deterministic reissue and
  invalidation identity strategy
- clean up tests and observability labels
- record explicit migration-window exit criteria (owner, release/date, and
  checkpoint threshold) before removing historical adapters
- remove historical adapters after the agreed migration window

Likely files:

- `planner-server/src/ws.rs`
- `planner-server/src/session.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/components/MessageInput.tsx`

Implemented in this phase:

- Removed legacy websocket runtime protocol paths:
  - server messages: `speculative_draft`, `draft_reaction_ack`
  - client messages: `skip_question`, `draft_reaction`
- Removed compatibility Socratic event/projection branches:
  - `SocraticEvent::Question`
  - `SocraticEvent::SpeculativeDraftReady`
- Checkpoint projection now updates pending interview state only from
  `SocraticEvent::PromptGenerated`.
- Legacy checkpoint read-time promotion remains intentionally active for the
  migration window.

Additional hardening delivered after the first cut:

- Removed the internal serialized `PromptResponse` runtime bridge.
- Deterministic reissue/replacement identity semantics landed with dedicated
  tests.
- Retry/artifact summary UI and stage fallback behavior landed.
- Legacy websocket integration fixtures were migrated to prompt-envelope flows.
- `planner-tui` now consumes prompt envelopes directly and its suite is green.

Not fully implemented yet (risks/gaps):

- Historical checkpoint read adapters are still active by design until the
  migration-window criteria are satisfied.
- Validation closure is complete for the current targeted matrix; remaining
  follow-up is the scheduled post-window adapter removal only.

Recorded migration-window exit criteria (2026-03-08):

- Owner: Planner server maintainers (`planner-server` runtime + session store).
- Earliest removal date: 2026-04-30 (not before one full release window after
  prompt-envelope cutover).
- Required threshold: 14 consecutive UTC days with zero persisted checkpoints
  containing legacy `current_question` or `pending_draft` fields in production
  session storage scans.
- Required validation before adapter deletion:
  - server integration coverage for prompt resume/replay remains green
  - explicit migration dry-run proves no legacy checkpoints remain readable only
    through `InterviewCheckpointLegacyWire`.
- Removal step (single PR): delete `InterviewCheckpointLegacyWire` and
  `legacy_checkpoint_prompt_adapter`, remove legacy promotion tests, and note
  the deletion in release notes.

Done when:

- no production code depends on `current_question`, `pending_draft`, or
  `socratic_response`
- prompt envelopes are the only active interview interaction contract

## Coverage Checklist

Most of the focused core/server/web/TUI items below are now implemented. Treat
this section as a coverage checklist, not a pure outstanding-task list. The
primary remaining additions are benchmark execution evidence and broader
retry-heavy / longer-session soak reporting.

### Core

- batch planner chooses contradictions before verification before discovery
- batch planner excludes dependent items from the same prompt
- response adjudicator applies direct effects without LLM
- partial responses preserve unanswered items for future prompts
- reissued items can be reworded after earlier answers change context
- reissued items retain stable identity when the semantic target is unchanged
- invalidated items emit replacement identity when the semantic target changes
- draft review prompts reference sections and unresolved dimensions correctly

### Server

- websocket prompt round-trip for fresh sessions
- websocket prompt replay from checkpoint resume
- checkpoint promotion from historical `current_question` and `pending_draft`
- partial prompt submission updates belief state and emits a new prompt
- stale item invalidation is surfaced in events and session state
- pipeline stage events update session stages in real time
- validation feedback and retry events are visible over the same session event
  channel as Socratic
- runtime prompt intake no longer depends on an internal serialized
  `PromptResponse` bridge
- missing or malformed pipeline stage metadata degrades gracefully without
  leaving stale non-terminal session stages

### Web

- prompt cards render from `PromptEnvelope`
- each item supports single-select plus custom text
- partial submit only sends answered items
- viewport capability changes adjust requested batch size
- draft sidebar renders alongside draft-review prompt items
- transcript remains human-facing and does not show protocol placeholders
- pipeline start auto-surfaces the build feed or equivalent live event view
- the UI no longer shows optimistic pipeline-start text without backend
  confirmation
- retry feedback is elevated into a dedicated summary UI treatment
- persisted artifact progress is elevated into a dedicated summary UI treatment
- stage UI remains truthful when event metadata omits canonical `stage` keys

## Validation Constraints (2026-03-09)

- the recorded benchmark is a local baseline for deterministic direct-effect
  adjudication; do not generalize it into a hardware-independent SLA
- `planner-tui` no longer blocks broad validation; its prompt-envelope
  migration and test suite are green as of 2026-03-09
- reconnect-heavy coverage exists, but retry-heavy and longer soak coverage
  should still be reported separately from the targeted unit/integration matrix

## Risks And Design Constraints

- letting the server guess screen-fit batch size will create poor prompts;
  client capability hints are required
- allowing mutually dependent items into the same batch will reduce answer
  quality and increase reissue churn
- preserving free-form text only, without structured answers, will keep the
  protocol ambiguous and slow
- trying to parallelize belief-state writes will create nondeterministic
  convergence behavior
- removing the old contract without historical checkpoint promotion risks
  breaking persisted in-progress sessions on disk
- leaving pipeline visibility as a separate concern will recreate the same trust
  gap at the exact moment the new prompt protocol hands control off to the
  build pipeline

## Rollout Order

Recommended merge order:

1. shared schema and checkpoint model
2. websocket and REST transport replacement
3. engine refactor
4. deterministic answer application
5. lobby UI rebuild
6. pipeline visibility alignment
7. draft-in-flow conversion
8. legacy deletion and cleanup

## Open Questions

These are implementation questions, not product-intent questions:

- whether prompt options should support more than one direct belief-state effect
  in the first cut, or exactly one effect per option
- whether prompt responses should include explicit client-generated draft edit
  diffs for section-level review, or only free-text corrections in phase 1
- whether mobile should request fewer items through a coarse viewport class, or
  whether the client should compute an exact `max_visible_items` count from
  layout measurements
