# Model Routing Policy

This file is the human-readable model routing policy for Planner.

It is policy only. Editing this file does not change runtime behavior by itself.
Runtime defaults currently live in [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs).

The purpose of this document is to preserve:

- which model family is intended for each stage
- what failure looks like at that stage
- how the stage is gated
- what the fallback path should be

If `mod.rs` and this file ever disagree, treat this file as the design-policy source of truth and update runtime code deliberately.

## Principles

- Use the strongest planning model where ambiguity, architecture, or contradiction risk is highest.
- Use code-specialized models for implementation and patching.
- Use cheaper, faster models for translation and telemetry only when they are schema-constrained and cross-checked.
- Prefer multi-model disagreement only where it provides distinct signal, not everywhere.
- Escalate failures by tightening scope first, then promoting to a stronger model.

## Runtime Snapshot

This is the current runtime mapping implemented in [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs) as of March 15, 2026.

| Runtime Component | Current Default | Provider |
| --- | --- | --- |
| `INTAKE_GATEWAY` | `claude-opus-4-6` | Anthropic |
| `COMPILER_SPEC` | `claude-opus-4-6` | Anthropic |
| `COMPILER_GRAPH_DOT` | `claude-opus-4-6` | Anthropic |
| `FACTORY_WORKER` | `gpt-5.3-codex` | OpenAI |
| `SCENARIO_VALIDATOR` | `gemini-3.1-pro-preview` | Google |
| `TELEMETRY_PRESENTER` | `claude-haiku-4-5` | Anthropic |
| `RALPH_LOOPS` | `claude-sonnet-4-6` | Anthropic |
| `AR_REVIEWER_OPUS` | `claude-opus-4-6` | Anthropic |
| `AR_REVIEWER_GPT` | `gpt-5.2` | OpenAI |
| `AR_REVIEWER_GEMINI` | `gemini-3.1-pro-preview` | Google |
| `AR_REFINER` | `claude-opus-4-6` | Anthropic |

## Policy Table

| Step | Preferred Model(s) | What failure looks like | Detection (gate) | Fallback |
| --- | --- | --- | --- | --- |
| intake | Opus + Gemini | Missed constraints, wrong stack assumptions, contradictions introduced early | Counter-signal consistency checks, user correction loop, intake amendment review | Re-run intake with focused deltas; if disagreement persists, escalate to Opus-only Socratic pass |
| specify | Opus | Sacred anchors remain ambiguous, acceptance criteria are not testable, architecture is vague | Spec linter, adversarial review rejects, inability to derive testable scenarios | Force an anchor-tightening sub-step with Opus, then re-lint |
| planpack | Gemini -> Opus | Planpack is suggestive but non-binding, contracts incomplete, weak decomposition | Implement/AR disagreement, repeated contract failures, graph-dot drift | Regenerate options with Gemini, then re-bind and normalize with Opus |
| ar | Opus + Gemini + Codex | Review misses real defects, overproduces noise, or fails to isolate actionable change sets | Tool-truth failures, regressions after review, low-signal findings | Promote failed area to Opus deep review and add targeted anchor/test coverage |
| tasks | Opus | Task graph misses dependencies, parallel work conflicts, sequencing breaks delivery | Implement deadlocks, repeated merge conflicts, blocked downstream nodes | Rebuild task graph with stricter dependency extraction and narrower scopes |
| implement | Codex | Code compiles but violates contracts, patch quality is brittle, security bugs leak through | Compile/test/lint tool-truth, constitution sidecar, AR code critic | Retry once with tighter context, then escalate to Sonnet review plus Codex patch pass |
| translate | Haiku | Status summary misstates reality, underreports failures, or rewrites gate outcome | Schema validation and cross-check against gate results and telemetry | Re-run with Sonnet; if still mismatched, surface raw telemetry excerpt |
| ralph | Haiku + Sonnet | Misses high-risk signals or produces too much low-value advisory noise | Budget caps, evidence requirements, advisory dedupe | Tighten query/filter stage; escalate one finding to Opus for deep dive |
| adversarial_test | Opus + Codex | Tests do not map to anchors, become flaky, or miss critical acceptance paths | Tool-truth execution, anchor-to-test traceability, flake review | Re-derive test plan with Opus, regenerate tests with Codex under tighter constraints |

## Additional Planned Policy Slots

These are policy placeholders for stages that either do not exist yet as dedicated runtime roles or are not fully wired.

| Step | Preferred Model(s) | Intent |
| --- | --- | --- |
| code_graph | Pending research, likely single-model dedicated worker | Build or enrich code-structure graphs without polluting architectural blueprint semantics |
| blueprint_relationships | Pending research | Infer stable project/component/technology relationships from code and discovery sources |
| model_research | Strong frontier model under active evaluation | Compare candidate replacements before changing runtime defaults |

## Candidate Models Under Evaluation

These are not runtime defaults. They are included here so they do not get lost again.

| Candidate | Status | Notes |
| --- | --- | --- |
| `gpt-5.4` | Under evaluation | Intended to be considered as a possible replacement or escalation target for selected high-cognition stages |

## Stage Notes

### intake

The intake stage is allowed to be multi-model because disagreement is useful signal. The output must still converge to a single coherent intake artifact before moving forward.

### specify

Specification work is intentionally conservative. A stronger planning model is preferred over a faster model because ambiguity here creates amplified downstream cost.

### ar

Adversarial review is not just “more model calls.” Each family should contribute a different lens:

- Opus: intent completeness and design coherence
- GPT/Codex-family: contradiction, implementation realism, proof-like reasoning
- Gemini: scope integrity and wide-context cross-reference

### implement

Implementation should optimize for code quality under tool-truth, not raw prose quality. Passing compile/test/lint gates matters more than eloquent explanation.

### translate

Translation is never authoritative. It is a presentation layer over gate results and telemetry, and must be treated as lossy unless cross-checked.

## Change Rules

When updating this policy:

1. Update this file first.
2. Decide whether runtime defaults in [planner-core/src/llm/mod.rs](/home/thetu/planner/planner-core/src/llm/mod.rs) should change.
3. If runtime changes, update the code explicitly in a separate step.
4. Record why the model moved, what gate it is expected to improve, and what regression risk it introduces.

## Open Questions

- Which stages should remain multi-model versus single-owner?
- Should `code_graph` be a dedicated single-model worker with hard restrictions?
- Which stages are appropriate candidates for `gpt-5.4` replacement or escalation?
- Should runtime defaults eventually be generated from this file rather than duplicated by hand?
