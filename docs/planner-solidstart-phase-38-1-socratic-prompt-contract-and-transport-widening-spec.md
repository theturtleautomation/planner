# Planner SolidStart Phase 38.1 Socratic Prompt Contract And Transport Widening Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md)  
**Related Planning:** [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-schemas/src/artifacts/socratic.rs`, `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`, `planner-server/src/ws_socratic.rs`, `planner-server/src/session.rs`, and `planner-solid/src/lib/types.ts`

## 1. Purpose

Create the first bounded implementation slice under Phase 38 by widening the
Socratic prompt and answer transport contract without yet redesigning planner
behavior or the widescreen workspace hierarchy.

## 2. Problem

The repo already has a real prompt-bank model, but its answer contract still
centers on one legacy interaction shape:

- `selected_option_id`
- `custom_text`
- `skipped`

That makes every richer prompt concept downstream feel speculative because the
schema, websocket payloads, persisted checkpoint shape, and Solid transport
types still assume one composer.

## 3. User Outcome

After this slice:

- prompt items can declare richer response modes and layout intent truthfully
- answers can carry structured payloads while preserving legacy compatibility
- frontend and backend transports can move the widened contract end-to-end
- the repo is ready for one representative multi-modal prompt path without
  fake UI affordances

## 4. Scope

### In Scope

- `planner-schemas` widening for prompt item and answer types
- serde-compatible transport evolution for prompt-bank and answer payloads
- session/checkpoint compatibility rules needed to read older payloads safely
- Solid frontend type widening so the route can observe the richer contract
- transport-level tests for serialization and backward-compatible reads

### Out Of Scope

- choosing which prompt kinds use which response modes
- planner or adjudicator behavior changes
- widescreen command-desk route redesign
- media upload or binary asset transport

## 5. Contract

### 5.1 Prompt item widening

Prompt items must be able to carry:

- `response_mode`
- optional mode configuration
- preferred layout intent beyond the current narrow enum
- text/rationale requirements where relevant

Minimum response modes this slice must model at the schema layer:

- `single_select_with_optional_text`
- `binary_with_rationale`
- `short_text`
- `long_text`
- `ranked_choice`
- `split_fields`
- `confidence_scale`
- `importance_scale`
- `comparison_choice_with_rationale`

### 5.2 Prompt answer widening

Prompt answers must preserve legacy fields while adding one structured payload
shape that can represent:

- ordered selections
- keyed field values
- scalar controls
- comparison-path decisions

Compatibility rule:

- older answers with only `selected_option_id` and `custom_text` remain valid
- new answers may populate both legacy shims and the structured payload where
  that simplifies migration

### 5.3 Transport rule

All widened fields must survive these boundaries without lossy frontend
flattening:

- prompt-bank REST reads
- websocket prompt updates
- answer submission payloads
- checkpoint persistence and reload

## 6. Touched Surfaces

- `planner-schemas/src/artifacts/socratic.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`
- `planner-server/src/session.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/api.rs`
- `planner-solid/src/lib/types.ts`

## 7. Acceptance Criteria

1. the schema supports richer prompt response modes and structured answer
   payloads
2. older serialized prompt-bank and answer data remains readable
3. Solid transport types no longer flatten the widened contract back to the
   legacy shape
4. no planner/adjudication or route-redesign claims are bundled into this slice

## 8. Verification Plan

- targeted Rust serialization tests in `planner-schemas`
- targeted server tests for transport compatibility
- type-level or unit-level Solid verification for widened prompt-bank payloads
- `git diff --check`

## 9. Rollback / Fallback

If the full answer payload widening is too broad in one pass:

- land item-level `response_mode` and mode configuration first
- preserve the legacy answer submission path temporarily
- do not fake new modes in the UI until the structured answer payload lands

## 10. Implementation Outcome

Implemented on 2026-04-02 as the first bounded Phase 38 delivery slice.

Delivered behavior:

- widens `PromptResponseMode` beyond the legacy single-mode enum while keeping
  the existing `single_select_with_custom_text` contract readable
- adds a structured prompt-answer payload that can carry ordered selections,
  keyed field values, scalar controls, and comparison-path decisions
- keeps older prompt-response payloads readable through serde defaults and
  compatibility-safe optional fields
- widens `SavedPromptAnswerDraft` and Solid transport types so the frontend no
  longer collapses the contract back to only `selected_option_id` and
  `custom_text`
- updates the shared Rust and frontend helper paths so structured payloads
  count as real answer content and survive draft/save/submit flows

## 11. Verification Evidence

- `cargo test -p planner-schemas prompt_response_serde_round_trip -- --nocapture`
- `cargo test -p planner-schemas prompt_response_legacy_payload_still_deserializes -- --nocapture`
- `cargo test -p planner-core answer_to_input_text_uses_structured_payload_when_present -- --nocapture`
- `cargo test -p planner-server prompt_response_to_input_uses_structured_answer_payload -- --nocapture`
- `cargo check -p planner-core -p planner-server`
- `npm --prefix planner-solid test -- --run src/lib/workspace.test.ts src/lib/prompt-bank.test.ts src/lib/mock/store.test.ts`
- `git diff --check`

## 12. Open Questions

- whether the structured answer payload should use one generalized map-plus-
  order model or a tagged union with separate mode payloads
- whether `PromptPreferredLayout` should widen in this same slice or remain a
  minimal compatibility enum until the UI child spec lands
