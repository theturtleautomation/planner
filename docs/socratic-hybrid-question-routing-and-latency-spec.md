# Socratic Hybrid Question Routing And Latency Spec

**Status:** implemented  
**Date:** 2026-03-23  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), official OpenAI models guidance reviewed on 2026-03-23

## Problem & Intent

The current Socratic intake path makes the user wait for a heavyweight
question-generation call even when the next question is obvious. In the live
product this shows up as repeated `preparing` states, slow first questions, and
long dead air between branch transitions.

The current implementation uses one heavy intake model for question generation:

- [planner-core/src/pipeline/steps/socratic/question_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/question_planner.rs)
  routes question generation through `DefaultModels::INTAKE_GATEWAY`
- [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs)
  currently binds `INTAKE_GATEWAY` to `claude-opus-4-6`
- [planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs)
  calls the question planner for core discovery items

That is defensible for hard questions, but it is the wrong default for the
common path.

The current official OpenAI models guidance now explicitly recommends
`gpt-5.4` as the flagship starting point for complex reasoning and coding, and
`gpt-5.4-mini` when optimizing for latency and cost. This spec adopts that
pair for the intake router.

This slice introduces a hybrid routing model:

1. no-LLM deterministic scaffolds for obvious first-pass questions
2. a fast default model lane for normal single-dimension questions
3. a deep model lane only for genuinely complex or escalated cases

The goal is to make the lobby feel immediate without weakening correctness,
constitution checks, or server authority.

## User Outcome

After this slice:

- early intake questions appear materially faster
- `preparing` states become shorter and rarer
- common requirement dimensions feel responsive instead of blocked on heavy
  generation
- complex or ambiguous branches can still escalate to a stronger model when the
  wording quality actually matters
- the backend keeps authorship of question content and lane choice; the client
  still renders server-authored prompts only

## Locked Decisions

- this slice is backend-first; it does not redesign the focused lobby layout
- the prompt-envelope contract remains the active transport model
- clients must not invent questions locally
- deterministic scaffolds are allowed only for well-understood standard
  dimensions
- a model router must exist as an explicit policy layer, not as scattered
  conditionals inside ad hoc prompt builders
- the implementation must remain provider-pluggable; lane selection should not
  hard-code one vendor into the architecture
- the initial default routing should use the OpenAI pair we selected from the
  current model research:
  - fast lane default: `gpt-5.4-mini`
  - deep lane default: `gpt-5.4`
- widening the repo model catalog as needed to support those defaults is part
  of this slice, not a separate blocker
- implementation must verify that the installed `codex` CLI/provider routing can
  resolve those model IDs truthfully inside Planner's existing CLI-native LLM
  layer

## In Scope

- introduce a hybrid question-routing policy for Socratic discovery and
  verification prompts
- add deterministic scaffold generation for standard early dimensions such as:
  - `Goal`
  - `Platform`
  - `Core Features`
  - `Success Criteria`
  - `User Flows`
  - `Out of Scope`
  - `Stakeholders`
- route normal live question generation to a fast model lane
- route escalated generation to a deep model lane
- widen the model catalog or routing configuration as needed for the chosen
  `gpt-5.4-mini` / `gpt-5.4` defaults
- keep a truthful fallback path if the selected OpenAI IDs are unavailable in a
  given runtime environment
- make the routing decision explicit and testable
- add observability for:
  - routing lane selected
  - whether the question came from a deterministic scaffold
  - generation latency and fallback reason
- tighten user-facing progress copy only if needed to keep the lobby truthful
  during generation

## Out Of Scope

- redesigning the focused-lobby layout or Ethereal Cascade visual model
- changing websocket transport shape or prompt-envelope schema unless a small
  metadata addition becomes necessary for observability
- cross-session prompt caching or a global organizational prompt library
- changing belief-state adjudication rules
- provider-catalog expansion unrelated to the intake router or the selected
  fast/deep defaults
- speculative fine-tuning work

## Dependencies

- [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs)
  must be widened to recognize the selected OpenAI model IDs
- [planner-core/src/llm/providers.rs](/home/thetu/planner/planner-core/src/llm/providers.rs)
  must continue routing those IDs through the `codex` CLI without special-case
  regressions
- the implementation environment must have a `codex` CLI version that supports
  the selected models

## Current-State Evidence

- the question planner always uses the heavy intake gateway in
  [planner-core/src/pipeline/steps/socratic/question_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/question_planner.rs)
- the default intake gateway is currently `claude-opus-4-6` in
  [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs)
- the current repo model catalog does not yet advertise `gpt-5.4` or
  `gpt-5.4-mini`, so catalog/config widening is a real implementation task, not
  implied magic
- the focused lobby already allows per-session warm question reuse in principle,
  but Phase 13 intentionally kept it optional and subordinate to correctness in
  [phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)
- the current product pain reported in-session is not lack of hierarchy alone;
  it is that users stay in `preparing` too long before a real question arrives

## Proposed Design

### 1. Routing tiers

#### Tier 0: deterministic scaffolds

Use no model call when all of the following are true:

- the target dimension is one of the locked standard dimensions
- there is no contradiction
- the prompt does not require self-critique regeneration
- the dimension is not custom
- the current state does not require long-history contextualization

The scaffold must still produce a full `QuestionOutput` shape with:

- a natural-language question
- 4-7 quick options when appropriate
- `allow_skip` semantics preserved

Scaffolds should sound like Planner, not like raw placeholders.

#### Tier 1: fast model lane

Use the fast lane for the common case:

- normal discovery prompt
- normal verification prompt
- short or moderate conversation history
- no contradiction
- no constitution-regeneration pass required

Initial bounded default:

- `gpt-5.4-mini`

The router implementation should allow this lane to be swapped later without
rewriting the intake planner.

#### Tier 2: deep model lane

Use the deep lane only when the question is actually hard:

- contradiction handling
- regenerated question after constitution violations
- custom dimensions
- long or messy history
- multiple unresolved dependencies
- high ambiguity where wording precision is more important than latency

Initial bounded default:

- `gpt-5.4`

### 2. Complexity chooser policy

The lane chooser should be explicit and deterministic.

At minimum it should consider:

- target dimension class
- question kind
- contradiction presence
- whether this is a regeneration pass
- whether the dimension is custom
- history length or complexity
- dependency pressure
- whether a verified scaffold exists for the target dimension

The result should be one of:

- `scaffold`
- `fast_model`
- `deep_model`

### 3. Provider-pluggable model configuration

Do not bury model IDs inside the chooser.

Implementation should expose explicit defaults for:

- fast question lane
- deep question lane

The first implementation should bind those defaults to the chosen OpenAI pair.
The shape should still support later swapping without redesigning the planner.

### 4. Runtime fallback policy

If `gpt-5.4-mini` or `gpt-5.4` is unavailable in the installed runtime, the
router must not silently pretend otherwise.

Allowed behavior:

- log or surface the exact unavailable model ID
- fall back to a clearly configured secondary pair
- keep the lane metadata truthful about which fallback model was actually used

Disallowed behavior:

- silently claiming the preferred OpenAI lane while using another model
- silently collapsing all requests back to the current heavy intake gateway

### 5. Observability

Every routed question generation should make the path inspectable.

At minimum record:

- chosen lane
- chosen model when a model lane is used
- scaffolded dimension when a scaffold was used
- elapsed generation time
- fallback reason if the fast lane escalated to deep

This may surface through planner events, logs, or both, but it must be
available for debugging and audit.

## Contracts & Touched Surfaces

Expected primary files:

- [planner-core/src/pipeline/steps/socratic/question_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/question_planner.rs)
- [planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs)
- [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs)

Expected supporting files, only if needed:

- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/hooks/useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)

## Acceptance Criteria

1. Common first-pass dimensions listed in this spec can be generated without an
   LLM call.
2. Normal discovery and verification questions use a fast lane by default
   rather than the deepest available model.
3. Complex or escalated questions still route to a deep lane when warranted.
4. The lane choice is explicit and testable rather than implied by scattered
   conditionals.
5. Prompt-envelope output shape remains stable for the web client.
6. The implementation exposes routing observability sufficient to explain why a
   question was scaffolded, generated quickly, or escalated.
7. If the fast lane fails or is unavailable, fallback behavior is truthful and
   does not strand the user in a silent dead state.
8. If the preferred OpenAI model IDs are unavailable, the fallback path is
   explicit, inspectable, and does not silently masquerade as `gpt-5.4-mini`
   or `gpt-5.4`.

## Verification Plan

### Automated

- add unit coverage for lane selection policy in
  [question_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/question_planner.rs)
- add unit coverage proving standard dimensions use deterministic scaffolds
  when eligible
- add prompt-batch tests proving common discovery items no longer require a
  heavy lane by default
- add failure-path tests proving fast-lane fallback behavior remains truthful
- add model-catalog or provider-routing coverage proving the selected OpenAI
  IDs are recognized, or that explicit fallback engages when they are not
- rerun the Socratic prompt-planning and websocket coverage:
  - `cargo test -p planner-core question_planner -- --nocapture`
  - `cargo test -p planner-core prompt_batch_planner -- --nocapture`
  - `npm --prefix planner-web test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx src/pages/__tests__/SessionPage.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx`

### Manual

- start a fresh Socratic session and confirm the first common intake questions
  appear materially faster than the current all-Opus path
- verify a standard branch such as `Platform` or `Core Features` reaches a real
  prompt with little or no visible dead time
- verify a deliberately ambiguous or custom branch can still escalate to the
  deep lane without malformed output
- confirm progress copy stays truthful when generation falls back or takes
  longer than expected

## Rollback & Fallback

- if deterministic scaffolds prove too rigid, keep the lane router and disable
  only the affected scaffold dimensions
- if the fast lane harms wording quality, narrow it to fewer question kinds and
  keep the router intact
- if model-catalog expansion becomes risky, ship the routing abstraction and
  Anthropic defaults first, then add alternate model families in a later slice

## Open Questions

- should routing telemetry surface only in backend logs/events, or should a
  minimal lane/debug marker be exposed in developer-facing admin surfaces later?
  This is not blocking the initial slice.

## Implementation Sync

Implemented on 2026-03-23.

- [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs)
  now advertises `gpt-5.4-mini` and `gpt-5.4` and exposes explicit fast/deep
  intake-question defaults.
- [planner-core/src/pipeline/steps/socratic/question_planner.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/question_planner.rs)
  now routes questions through three explicit lanes:
  deterministic scaffolds for standard first-pass dimensions, `gpt-5.4-mini`
  for normal generated questions, and `gpt-5.4` for deep or regenerated cases.
- Fast-lane failures now log a truthful warning and retry once through the deep
  lane instead of leaving the session stranded in ambiguous `preparing` state.

Verification completed in this implementation pass:

- `cargo test -p planner-core question_planner`
- `cargo test -p planner-core prompt_batch_planner`
- `npm --prefix planner-web test -- src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `codex exec --json -m gpt-5.4-mini "Reply with OK and nothing else."`
- `codex exec --json -m gpt-5.4 "Reply with OK and nothing else."`

Not completed in this slice:

- no live end-to-end Socratic session timing capture was recorded in-doc, so
  real user-visible latency improvement is still a product-level manual check
  rather than a benchmark claim here.
