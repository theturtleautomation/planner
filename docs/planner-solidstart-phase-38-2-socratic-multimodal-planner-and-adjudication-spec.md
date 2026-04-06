# Planner SolidStart Phase 38.2 Socratic Multimodal Planner And Adjudication Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md)  
**Related Planning:** [Planner SolidStart Phase 38.1 Socratic Prompt Contract And Transport Widening Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-1-socratic-prompt-contract-and-transport-widening-spec.md), [Phase 13 Socratic Realtime Workspace Deltas And Warm Prompt Library Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`, `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`, and `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`

## 1. Purpose

Make the Socratic engine choose and interpret the richer interaction modes
introduced by Phase 38.1 instead of emitting one default composer shape for
every prompt item.

## 2. Problem

Even if the widened transport exists, the product remains simplistic unless the
planner and adjudication path actually use it. Right now prompt generation
still collapses discovery, contradiction, verification, and draft prompts into
the same answer model.

## 3. User Outcome

After this slice:

- prompt kinds can choose interaction modes that match the reasoning task
- answer adjudication can read structured responses truthfully
- the system can prove one representative multi-modal flow without UI fakery

## 4. Scope

### In Scope

- prompt-batch planner rules for choosing response modes by prompt kind and
  target dimension
- adjudication logic for interpreting widened structured answers
- one explicit prompt-mode decision matrix grounded in current Socratic prompt
  kinds
- tests for mode selection and answer interpretation

### Out Of Scope

- full command-desk route layout work
- broad visual redesign
- changing first-reveal bank truth or route topology

## 5. Contract

### 5.1 Mode-selection matrix

This slice must define which current prompt families map to which interaction
styles.

Minimum required mappings:

- contradiction or challenge prompts may use `binary_with_rationale`
- short factual clarifiers may use `short_text`
- synthesis or tradeoff prompts may use `comparison_choice_with_rationale`
- prioritization prompts may use `ranked_choice`
- confidence or certainty checks may use `confidence_scale`

### 5.2 Adjudication rule

Adjudication must not assume every answer can be reduced to one option id plus
freeform text. It must read the structured payload or the legacy shim
truthfully, with compatibility for older answers.

## 6. Touched Surfaces

- `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- supporting tests in `planner-core`

## 7. Acceptance Criteria

1. the planner chooses more than one response mode across current prompt kinds
2. adjudication can interpret at least one non-legacy structured answer mode
3. the slice stays bounded to planner/adjudication behavior and does not claim
   route redesign completion

## 8. Verification Plan

- targeted unit tests for planner mode selection
- targeted adjudication tests for structured answer interpretation
- regression proof that legacy answers still parse correctly

## 9. Rollback / Fallback

If full multimodal adjudication is too broad in one pass:

- keep the selection matrix
- prove one representative non-legacy mode end to end
- leave additional prompt families on the compatibility mode temporarily

## 10. Implementation Outcome

Implemented on 2026-04-02 as the second bounded Phase 38 delivery slice.

Delivered behavior:

- the prompt planner now chooses more than one response mode across current
  Socratic prompt families instead of emitting one legacy mode everywhere
- contradiction prompts now advertise `binary_with_rationale`
- generated core questions now distinguish between option-backed prompts and
  pure freeform prompts through `single_select_with_optional_text` and
  `short_text`
- adjudication now reads structured selections and structured rationale fields
  truthfully instead of assuming only legacy option IDs and freeform text
- one representative non-legacy structured contradiction flow is proven end to
  end without forcing route redesign or a new UI composer in this slice

## 11. Verification Evidence

- `cargo test -p planner-core response_mode_matrix_selects_more_than_one_mode -- --nocapture`
- `cargo test -p planner-core structured_binary_answer_applies_direct_effect_and_resolves_contradiction -- --nocapture`
- `cargo test -p planner-core answer_to_input_text_uses_structured_payload_when_present -- --nocapture`
- `cargo test -p planner-server prompt_response_to_input_uses_structured_answer_payload -- --nocapture`
- `cargo check -p planner-core -p planner-server`
- `git diff --check`

## 12. Open Questions

- which current prompt families are best suited for the first representative
  non-legacy mode proof
- whether mode selection belongs entirely in planner code or partly in
  declarative prompt templates
