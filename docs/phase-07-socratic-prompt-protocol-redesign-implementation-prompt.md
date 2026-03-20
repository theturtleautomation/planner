# Phase 07 Socratic Prompt Protocol Redesign Implementation Prompt

**Status:** Implemented except scheduled post-window legacy-adapter removal  
**Date:** 2026-03-09  
**Source:** `docs/phase-07-socratic-prompt-protocol-redesign-implementation.md`

## Usage

This document is now primarily a historical execution brief for the delivered
prompt-envelope cutover.

Use the source implementation doc as the canonical status record. Do not treat
this file as an active implementation prompt except for the dedicated
post-window legacy-adapter removal once the recorded migration-window gate is
satisfied.

## Current Execution State (2026-03-09)

- Completed in code:
  - Phase 0 (prompt schema primitives, `current_prompt` checkpoint model,
    historical checkpoint promotion, initial checkpoint projection updates).
  - Phase 1 transport primitives (`prompt`, `prompt_response`,
    `ui_capabilities`) for websocket/session payloads.
  - Phase 2 partial backend refactor (prompt-loop engine, prompt-envelope
    resume state, prompt batch planner/protocol modules, structured
    prompt-response handling, serial ordered answer application).
  - Phase 5 core pipeline/event visibility implementation:
    - structured pipeline stage/retry/validation/artifact events emitted from
      `planner-core/src/pipeline/mod.rs`
    - API runtime forwards pipeline events onto session event streams
      (`planner-server/src/api.rs`)
    - session stage state now derives from events (`planner-server/src/session.rs`)
    - prompt lifecycle event coverage expanded in `planner-server/src/ws_socratic.rs`
    - web stage state now updates from `planner_event` metadata and optimistic
      `sendDone` pipeline messaging was removed
    - Session page auto-foregrounds the Events pane during `pipeline_running`
  - Phase 6 draft-review in-flow implementation:
    - draft review emitted as `PromptEnvelope.kind = "draft_review"` with
      `draft_snapshot`
    - draft-review items now cover draft sections, `not_discussed` gaps, and
      unresolved verification concerns
    - assumptions removed from first-class draft UI rendering
    - draft review responses flow through `PromptResponse`
  - Phase 7 core protocol deletion:
    - removed legacy websocket runtime protocol paths:
      `speculative_draft`, `draft_reaction_ack`, `skip_question`,
      `draft_reaction`
    - removed compatibility event/projection paths:
      `Question`, `SpeculativeDraftReady`
    - checkpoint projection now uses `PromptGenerated` envelopes
    - legacy checkpoint read-time promotion remains intentionally active
- Delivered after the 2026-03-08 snapshot:
  - `W1 Draft Planner Unification`
  - `W2 Typed Prompt Runtime`
  - `W3 Prompt Identity Hardening`
  - `W4 Pipeline Visibility Polish`
  - `W5 Legacy Cleanup`
  - `W6 Validation Proof`
  - `planner-tui` prompt-envelope migration
- Remaining: scheduled legacy-adapter removal after the recorded
  migration-window date/threshold.

Open risks from the implemented state:

- The recorded benchmark snapshot is a local-machine baseline, not a
  cross-environment SLA.
- Legacy checkpoint adapters remain intentionally active pending migration
  window closure.
- The execution prompt can drift from the source implementation doc if both are
  not updated together.

## Outstanding Items To Complete

Treat these as the remaining scheduled cleanup items, not active feature
implementation.

- Delivered in code:
  - `W1 Draft Planner Unification`
  - `W2 Typed Prompt Runtime`
  - `W3 Prompt Identity Hardening`
  - `W4 Pipeline Visibility Polish`
  - `W5 Legacy Cleanup`
  - `W6 Validation Proof`
- Scheduled migration cleanup
  - Keep historical checkpoint promotion active until the recorded
    migration-window exit criteria are met.
  - Remove the adapter in a dedicated post-window PR only after the threshold
    and date gate are satisfied.

Current targeted validation already green:

- `cargo test -p planner-core prompt_batch_planner -- --nocapture`
- `cargo test -p planner-core benchmark_high_answer_count_prompt_batch_adjudication -- --ignored --nocapture`
- `cargo test -p planner-server tier2_socratic_ws_ -- --nocapture`
- `cargo test -p planner-tui -- --nocapture`
- `npm --prefix planner-web test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/MessageInput.test.tsx src/components/__tests__/PromptBatchPanel.test.tsx`

Recorded benchmark snapshot:

- `benchmark.prompt_batch_adjudication iterations=200 total_ms=11 per_iteration_ms=0.055`

## Additional Code And Test Anchors

Inspect these in addition to the original phase files before editing:

- Runtime / protocol hardening:
  - `planner-server/src/runtime.rs`
  - `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`
  - `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`
- Batch planning / draft integration:
  - `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`
- Web prompt-batch UI / remaining compatibility surface:
  - `planner-web/src/components/PromptBatchPanel.tsx`
  - `planner-web/src/components/PromptCard.tsx`
  - `planner-web/src/components/PromptOptionGroup.tsx`
  - `planner-web/src/components/SessionEventsTable.tsx`
- Tests and fixtures that still need migration or expansion:
  - `planner-server/tests/server_integration.rs`
  - `planner-server/src/ws_socratic.rs`
  - `planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx`
  - `planner-web/src/pages/__tests__/SessionPage.test.tsx`
  - `planner-web/src/components/__tests__/MessageInput.test.tsx`
  - `planner-web/src/components/__tests__/PromptBatchPanel.test.tsx`

## Prompt

```text
Implement the "Phase 07 Socratic Prompt Protocol Redesign" in /home/thetu/planner.

Source of truth:
- docs/phase-07-socratic-prompt-protocol-redesign-implementation.md

Current code anchors you must inspect before editing:
- planner-schemas/src/artifacts/socratic.rs
- planner-core/src/pipeline/steps/socratic/socratic_engine.rs
- planner-core/src/pipeline/steps/socratic/question_planner.rs
- planner-core/src/pipeline/steps/socratic/belief_state.rs
- planner-core/src/pipeline/steps/socratic/speculative_draft.rs
- planner-server/src/session.rs
- planner-server/src/ws.rs
- planner-server/src/ws_socratic.rs
- planner-server/src/api.rs
- planner-core/src/pipeline/mod.rs
- planner-web/src/hooks/useSocraticWebSocket.ts
- planner-web/src/pages/SessionPage.tsx
- planner-web/src/components/MessageInput.tsx
- planner-web/src/components/SpeculativeDraftView.tsx
- planner-web/src/components/BeliefStatePanel.tsx
- planner-web/src/types.ts

Primary outcome:
- Replace the singular Socratic question protocol with a prompt-envelope
  protocol that supports multiple visible items, partial submission,
  deterministic serial belief-state updates, verification-first behavior,
  contradiction-first behavior, draft review in the same flow, and live
  session-visible pipeline progress after intake convergence.

Non-negotiable product decisions:
- Prompt batches may be submitted partially.
- Unanswered items may be re-asked later and may be reworded using new context.
- Batch size should be as large as fits on screen.
- Each item must support single-select plus custom text.
- Verification and contradiction resolution outrank new discovery questions.
- Draft review stays in the same interaction flow, while still exposing a full
  draft view in the UI.
- Breaking the current websocket contract is allowed.

Global guardrails:
- Keep backend belief-state mutation serial and deterministic.
- Do not introduce concurrent writes to RequirementsBeliefState.
- Do not keep free-form text as the sole source of truth for prompt answers.
- Do not preserve current_question, pending_draft, question, or
  socratic_response as active protocol concepts after the cutover.
- Do keep a safe read-time promotion path for historical checkpoints until the
  migration window is explicitly over.
- Do not split draft review into a separate mode or detached contract.
- Do not synthesize pipeline progress in the client before the backend emits it.
- Do not redesign project routing, ownership, or downstream pipeline execution
  beyond the prompt/progress integration required by the spec.
- Avoid touching generated files under planner-web/dist unless a final build
  step requires regeneration.

Implementation assumptions to use unless current code makes them clearly wrong:
- In the first cut, each PromptOption should support at most one direct effect.
- Prompt responses should carry free-text corrections, not structured draft
  edit diffs.
- The client should send both a coarse viewport_class and an explicit
  max_visible_items value derived from layout capacity.

Working style:
- Work phase by phase.
- Before each phase, inspect the current implementation in the files listed for
  that phase and identify any nearby tests.
- Add or update targeted tests before or alongside implementation, not after a
  large unverified refactor.
- Keep diffs coherent. If a phase exposes missing primitives needed by the next
  phase, add only the smallest forward-compatible surface.
- At the end of each phase, run the narrowest relevant test set plus any fast
  compile/typecheck validation for touched crates/packages.
- If the workspace contains unrelated edits, do not revert them. Work around
  them.

Remaining work that must be treated as active implementation, not background
context:
- Keep the source implementation doc and this execution prompt synchronized if
  any new follow-up work lands.
- Do not remove historical checkpoint promotion until the recorded
  migration-window exit criteria are met.

When reporting historical or active workstreams in status updates and final
summaries, use these labels:
- `W1 Draft Planner Unification`
- `W2 Typed Prompt Runtime`
- `W3 Prompt Identity Hardening`
- `W4 Pipeline Visibility Polish`
- `W5 Legacy Cleanup`
- `W6 Validation Proof`

Additional code and test anchors you must inspect before editing the remaining
work:
- `planner-server/src/runtime.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`
- `planner-web/src/components/PromptBatchPanel.tsx`
- `planner-web/src/components/PromptCard.tsx`
- `planner-web/src/components/PromptOptionGroup.tsx`
- `planner-web/src/components/SessionEventsTable.tsx`
- `planner-server/tests/server_integration.rs`
- `planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`
- `planner-web/src/components/__tests__/MessageInput.test.tsx`
- `planner-web/src/components/__tests__/PromptBatchPanel.test.tsx`

Non-negotiable remaining deliverables:
- Do not re-open `W1`-`W6` work unless a regression is found.
- Treat the recorded benchmark snapshot as local evidence, not a universal SLA.
- Do not remove historical checkpoint promotion until the migration-window exit
  criteria are satisfied.
- Do not allow this prompt to drift from the source implementation doc.

Use this phase order as historical context only. Active implementation work is
now limited to the scheduled migration-cleanup and documentation-sync items
described above.

Phase 0: Ratify the prompt domain model
Goal:
- Define the shared protocol and checkpoint model that replaces the singular
  question/draft state.

Implement:
- Add PromptEnvelope, PromptKind, PromptItem, PromptOption, PromptResponse,
  PromptAnswer, and UiCapabilities in planner-schemas/src/artifacts/socratic.rs.
- Add serde support and TypeScript-facing compatibility for the new structures.
- Replace InterviewCheckpoint.current_question and
  InterviewCheckpoint.pending_draft with current_prompt in
  planner-server/src/session.rs.
- Implement read-time promotion for historical checkpoints so old
  current_question values become one-item PromptEnvelope values and old
  pending_draft values become draft_review PromptEnvelope values.
- Keep the historical adapter explicit and isolated so it can be removed later.

Tests to add or update first:
- Schema serialization/deserialization coverage for PromptEnvelope and
  PromptResponse.
- Session checkpoint tests proving historical promotion works for both legacy
  question and legacy draft checkpoints.

Done when:
- Shared schema types compile.
- Session checkpoints can store exactly one active current_prompt envelope.
- Historical checkpoints can still be loaded safely.

Phase 1: Replace the wire contract
Goal:
- Make prompt envelopes, prompt responses, and UI capabilities the transport
  truth.

Implement:
- In planner-server/src/ws.rs, replace ServerMessage::Question with a prompt
  message and replace ClientMessage::SocraticResponse with prompt_response.
- Add a ui_capabilities client message and thread it through connection state.
- In planner-server/src/ws_socratic.rs, replay current_prompt on reconnect
  instead of rebuilding a singular question/draft projection.
- In planner-server/src/api.rs, expose current_prompt in session payloads.
- In planner-web/src/hooks/useSocraticWebSocket.ts, update the protocol layer
  so the client receives prompt envelopes and submits structured prompt
  responses.

Tests to add or update first:
- Websocket prompt round-trip for a fresh session.
- Websocket prompt replay from checkpoint resume.
- Session API payload coverage for current_prompt.

Done when:
- The client can attach, receive a PromptEnvelope, send a PromptResponse, and
  restore current_prompt after reconnect.
- No active transport path depends on question or socratic_response.

Phase 2: Refactor the engine around prompt envelopes
Goal:
- Replace the single-question blocking loop with a prompt-loop that can emit
  batches while still applying answers serially.

Implement:
- Replace ResumePendingPrompt with a prompt-envelope equivalent in
  planner-core/src/pipeline/steps/socratic/socratic_engine.rs.
- Introduce a dedicated prompt protocol module and batch planner module if that
  keeps the engine clean:
  - planner-core/src/pipeline/steps/socratic/prompt_protocol.rs
  - planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs
- Move batch planning responsibility out of the singular question planner path.
- Accept PromptResponse input instead of singular receive_input() handling.
- Preserve stable item ordering when carrying a prompt through generation,
  response handling, and reissue.
- Replace transitional prompt/item identifiers with deterministic reissue and
  invalidation identity rules.
- Keep unanswered items eligible for later reissue.

Batch planning rules to enforce:
- Prioritize unresolved contradictions first.
- Then prioritize uncertain dimensions needing verification.
- Then prioritize draft-section review items and unresolved gaps.
- Then ask new discovery questions.
- Do not include items whose dependencies are unresolved in the same batch.
- Do not include multiple competing items for the same dimension in one batch.
- Respect ui_capabilities.max_visible_items as an upper bound, not a target.

Tests to add or update first:
- Batch planner prioritizes contradiction over verification over discovery.
- Batch planner excludes dependent items from the same prompt.
- Partial responses preserve unanswered items for future prompts.
- Reissued items may be reworded after earlier answers change context.
- Reissued items keep stable identity when the semantic target is unchanged.
- Invalidated items receive explicit replacement identity when a new semantic
  target supersedes them.

Done when:
- The engine no longer assumes exactly one outstanding question.
- Partial prompt responses are accepted cleanly.
- A subsequent prompt can reissue remaining unresolved items.

Phase 3: Deterministic answer application and verification
Goal:
- Separate deterministic option effects from interpreted free-text handling and
  apply results in a single stable adjudication pass.

Implement:
- Add direct_effect support on PromptOption.
- Introduce a prompt response adjudicator module, for example
  planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs.
- Apply direct effects without an LLM when the selected option is fully
  deterministic.
- Route custom text and ambiguous responses through a targeted batch
  adjudication prompt.
- Apply adjudicated updates in stable item order.
- Run contradiction detection and convergence once after the full batch is
  applied, not once per item.

Tests to add or update first:
- Direct-effect answers update the belief state without LLM interpretation.
- Mixed structured and custom-text answers are applied in stable order.
- Contradiction detection and convergence run once per submitted prompt.

Done when:
- Deterministic answers avoid unnecessary LLM calls.
- Custom text still updates the belief state accurately.
- Latency is reduced relative to per-item free-text interpretation.

Phase 4: Rebuild the lobby UI around prompt batches
Goal:
- Make the UI truthfully render multiple answerable items at once and submit
  only the answers the user actually completed.

Implement:
- Add prompt-batch UI components as needed:
  - planner-web/src/components/PromptBatchPanel.tsx
  - planner-web/src/components/PromptCard.tsx
  - planner-web/src/components/PromptOptionGroup.tsx
- Keep or reuse existing input components only if they fit the new contract.
- Send ui_capabilities from the web client on attach and when viewport
  capacity changes.
- Replace the singular MessageInput question flow with prompt cards that
  support:
  - single-select choices
  - optional custom text per item
  - partial submission
- Preserve the human-facing transcript instead of exposing raw protocol
  placeholders.

Tests to add or update first:
- Prompt cards render from PromptEnvelope.
- Each item supports single-select plus custom text.
- Partial submit sends only answered items.
- Viewport capability changes affect requested batch size.

Done when:
- Multiple prompt cards can be answered and submitted together.
- Unanswered items do not block progress.
- Batch size responds to viewport capacity rather than a fixed server guess.

Phase 5: Add end-to-end pipeline visibility
Status (2026-03-08):
- Core implementation landed; UX polish and hardening follow-up remains open.

Goal:
- Keep the session lobby trustworthy after intake convergence by exposing real
  pipeline stage progress, retries, validation results, and artifact progress.

Implement:
- Add prompt-oriented events:
  - socratic.prompt.generated
  - socratic.prompt.submitted
  - socratic.prompt.partial_submitted
  - socratic.prompt.reissued
  - socratic.prompt.invalidated
  - socratic.response.adjudicated
  - socratic.draft.generated
- Add pipeline-oriented session events on the same channel:
  - pipeline.stage.started
  - pipeline.stage.completed
  - pipeline.stage.failed
  - pipeline.retry.started
  - pipeline.retry.feedback
  - pipeline.validation.completed
  - pipeline.artifact.persisted
- Emit these events from planner-core/src/pipeline/mod.rs and
  planner-server/src/api.rs through the same session-visible channel already
  used by Socratic.
- Update planner-server/src/session.rs so pipeline stage state is derived from
  events, not only from coarse start/end snapshots.
- Elevate retry feedback categories and persisted artifact progress into
  dedicated summary UI elements instead of leaving them as raw feed metadata.
- Normalize or gracefully fall back when stage metadata omits canonical `stage`
  keys so the stage UI does not remain stale until terminal events.
- In planner-web/src/pages/SessionPage.tsx, foreground the live events/build
  feed when the pipeline begins, or provide an equally strong unread/live
  treatment.
- Remove the optimistic "(Done — starting pipeline)" style client-side message
  from planner-web/src/hooks/useSocraticWebSocket.ts unless backed by a real
  backend event.

Tests to add or update first:
- Pipeline stage events update session stages in real time.
- Validation feedback and retry events are visible over the session event
  channel.
- Retry feedback and artifact persistence are visible through first-class
  session UI summaries, not only raw event metadata.
- Missing stage metadata degrades gracefully without leaving stale non-terminal
  stage bars.
- The UI no longer shows optimistic pipeline-start text without backend
  confirmation.

Done when:
- A retrying pipeline can be understood from the session UI alone.
- Users can see stage starts, completions, failures, retries, validation
  feedback, and artifact persistence without consulting logs or disk state.

Phase 6: Fold draft review into the prompt system
Status (2026-03-08):
- Mostly delivered; shared batch-planner integration remains open.

Goal:
- Unify discovery, verification, contradiction handling, and draft review
  under the same prompt envelope model.

Implement:
- Represent draft review as PromptEnvelope.kind = "draft_review".
- Add draft_snapshot support to PromptEnvelope.
- Render the full draft in a right-pane or sidebar UI while keeping active
  review items in the same flow.
- Merge draft-review candidate selection into the same batch picker pass used
  for contradiction, verification, and discovery work.
- Tie draft review items to sections, missing areas, or unresolved
  verification concerns.
- Remove assumptions as a first-class user-facing section and express
  uncertainty through verification prompts instead.

Files to inspect and update:
- planner-core/src/pipeline/steps/socratic/speculative_draft.rs
- planner-web/src/components/SpeculativeDraftView.tsx
- planner-web/src/pages/SessionPage.tsx

Tests to add or update first:
- Draft review prompts reference sections and unresolved dimensions correctly.
- Draft sidebar renders alongside draft-review prompt items.
- Draft edits and confirmations flow through PromptResponse.
- Mixed-priority prompts preserve the same contradiction -> verification ->
  draft-review -> discovery ordering after draft integration.

Done when:
- The draft is visible without switching to a separate interaction mode.
- Draft review uses the same PromptResponse protocol as the rest of Socratic.
- Uncertainty is surfaced as verification work instead of assumptions.

Phase 7: Remove legacy prompt paths and harden the system
Goal:
- Finish the cutover, delete the old singular protocol, and leave the system in
  a coherent post-migration state.

Implement:
- Remove legacy websocket types and singular checkpoint fields once the prompt
  path is fully active.
- Remove one-question-only UI paths and draft-reaction-only protocol code.
- Remove the internal serialized `PromptResponse` compatibility bridge so the
  runtime stays typed end to end.
- Clean up observability names, tests, dead adapters, and compatibility-only
  hook APIs.
- Keep deterministic prompt/item identity rules and tests in place during this
  cleanup; do not leave placeholder identities behind.
- Keep the historical checkpoint read adapter only for the agreed migration
  window; if the implementation doc or release context does not yet authorize
  removal, leave it in place and mark the cleanup point clearly.

Tests to add or update first:
- No production code depends on current_question, pending_draft, question, or
  socratic_response.
- No active runtime path serializes `PromptResponse` through an internal
  compatibility string bridge.
- Legacy checkpoint promotion remains covered as long as the adapter exists.
- Migration-window exit criteria are recorded explicitly before adapter
  deletion is attempted.
- Transcript and event surfaces stay human-facing after cleanup.

Done when:
- Prompt envelopes are the only active interview interaction contract.
- No production UI or server path depends on the old singular prompt model.

Cross-phase validation requirements:
- Add or compare a benchmark for high-answer-count prompt batches before
  claiming latency improvement from the new protocol.
- Rust: run focused cargo test/cargo check for touched crates after each phase
  and a broader validation pass before finishing.
- Web: run targeted frontend tests for touched components/hooks/pages and at
  least one final typecheck/build-oriented validation pass before finishing.
- Verify resume behavior from both fresh sessions and historical checkpoints.
- Verify partial submission, prompt reissue, contradiction prioritization, and
  draft-in-flow behavior end to end.
- Verify live pipeline progress reaches the session UI without optimistic
  client-only placeholders.
- If broader workspace validation is blocked by an unrelated existing failure,
  report the blocker explicitly and do not misattribute it to this phase.

Final response requirements:
- Summarize what changed by phase.
- List every test suite or command you ran and whether it passed.
- Call out remaining risks, especially around migration cleanup and any open
  questions you had to resolve by assumption.
```
