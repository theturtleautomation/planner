# Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-session-workspace-command-rail-hierarchy-spec.md), [Planner SolidStart Phase 37.5 Session Header Signal Consolidation Spec](/home/thetu/planner/docs/planner-solidstart-phase-37-5-session-header-signal-consolidation-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Phase 13 Socratic Realtime Workspace Deltas And Warm Prompt Library Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-schemas/src/artifacts/socratic.rs`, `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`, `planner-solid/src/lib/types.ts`, `planner-solid/src/routes/sessions/session-workspace-controller.ts`, `planner-solid/src/routes/sessions/session-workspace-screen.tsx`, and `planner-solid/src/app.css`

## 1. Executive Judgment

The implemented SolidStart session route is denser and calmer than the older
artifact-first workspace, but it still stops one layer short of the product
you now want.

Two coupled ceilings remain:

- the Socratic prompt contract is still effectively one response mode dressed
  in different prompt copy
- the route hierarchy still spends ultra-wide width on whitespace around one
  active-thread editor instead of turning width into concurrent operational
  comprehension

This phase is therefore not a cosmetic restyle. It is a follow-on parent spec
that reopens both the backend prompt contract and the session-route hierarchy
so the workspace can become a true multi-modal command desk without breaking
the bank-first runtime truth already established in earlier phases.

## 2. User Outcome

After this phase:

- Socratic prompts can declare multiple structured response modes rather than
  forcing every question into `selected_option_id + custom_text`
- the backend and frontend share an explicit item-level contract for response
  shape and preferred layout intent
- the session route uses widescreen space as a multi-panel operational desk
  with left navigation, a dominant answer canvas, and a right-side context
  surface
- smaller widths still preserve truthful access to the same work without
  inventing a separate product model

## 3. Problems To Solve

### 3.1 Prompt schema is richer in naming than in behavior

The schema already exposes prompt-bank structure, layout, and item kinds, but
the actionable answer contract is still narrow:

- `PromptPreferredLayout` currently only distinguishes `Cards` and `Review`
- `PromptResponseMode` currently only exposes
  `single_select_with_custom_text`
- `PromptAnswer` still centers on `selected_option_id`, `custom_text`, and
  `skipped`

That means prompt copy can vary, but the system still assumes one composer
shape.

### 3.2 Planner output still collapses all prompt intent into one mode

`prompt_batch_planner.rs` currently emits the same response mode for
discovery, contradiction, verification, and draft-related prompt items. The
planner can choose different item kinds, but it does not yet choose different
interaction models.

### 3.3 Frontend transport lags the backend prompt artifact

`planner-solid/src/lib/types.ts` still flattens prompt items and answers down
to the legacy fields the current route needs. That blocks the UI from acting on
real response-mode or layout metadata even if the backend starts emitting it.

### 3.4 The current route hierarchy still behaves like a widened editor, not a command desk

Phase 37 was correct for the previous problem: it removed dashboard repetition
and made one active thread dominant. But on a 5K display the current route
still behaves like a narrow workbench centered inside a large shell.

`planner-solid/src/app.css` still hard-caps the session shell to roughly
`1180px`, so ultra-wide width is lost before the route can turn it into
secondary live context.

## 4. Scope

### In Scope

- widening the Socratic prompt-item and prompt-answer contract
- widening frontend transport types to carry the new contract truthfully
- defining which prompt kinds use which interaction modes
- redefining the session workspace hierarchy for ultra-wide displays
- responsive fallback rules for laptop and mobile widths
- backward-compatible migration from the current single-mode answer model

### Out Of Scope

- media upload or image/audio/file response capture
- changing the bank-first runtime truth or first-reveal local availability
- rewriting unrelated routes such as projects, knowledge, discovery, or admin
- replacing the entire design system
- removing legacy answer fields before migration safety is proven

## 5. Product Decision

### 5.1 Multi-modal means structured interaction modes, not media

This phase uses "multi-modal" to mean multiple structured response shapes
inside the Socratic prompt flow:

- option chips
- yes/no or keep/discard decisions
- ranked choices
- short text
- long rationale text
- split-field answers
- confidence and importance controls
- compare-two-path prompts

It does not include image, audio, or file upload.

### 5.2 5K widescreen should become a command desk

The widescreen route should stop treating width as margin.

Required hierarchy:

- left: thread map, progress, branch health, and local navigation
- center: active answer canvas
- right: context, contradictions, synthesis, and build-readiness support

This is a real information-architecture change, not just a wider card layout.

### 5.3 Backend contract widening is mandatory

The UI must not fake richer interaction modes with presentation-only changes.
Every new input mode must come from the backend contract and remain visible in
the persisted prompt-bank and answer payloads.

## 6. Contract

### 6.1 Prompt item contract

Prompt items must be able to declare:

- a response mode
- optional field-level configuration for that mode
- preferred layout intent for the route
- whether rationale text is required, optional, or disallowed

Representative response modes to support:

- `single_select_with_optional_text`
- `binary_with_rationale`
- `short_text`
- `long_text`
- `ranked_choice`
- `split_fields`
- `confidence_scale`
- `importance_scale`
- `comparison_choice_with_rationale`

The exact enum names can tighten during implementation, but Phase 38 must
preserve this richer shape at the contract level.

### 6.2 Prompt answer contract

Answers must widen beyond the legacy pair of fields.

Required direction:

- preserve `selected_option_id` and `custom_text` for backward compatibility
- add a structured payload that can represent ranked selections, field maps,
  scalar controls, and compare-path choices
- keep `skipped` behavior explicit

### 6.3 Planner and adjudication contract

The planner and downstream Socratic adjudication must be able to choose and
interpret response modes based on prompt kind and target dimension rather than
assuming one composer for every item.

### 6.4 Route contract

The session route must preserve:

- bank-first first reveal
- truthful draft save and commit behavior
- local thread switching without refetch
- canonical/runtime parity between frontend-mock and `planner-server`

But on ultra-wide screens it may no longer be constrained to one narrow active
thread column.

## 7. Layout Model

### 7.1 Ultra-wide command desk

For 5K and similar widths, the session route should resolve into three
cooperating surfaces:

- a left command rail that keeps thread switching, coverage, and queued-work
  visibility local
- a center canvas for the active answer flow
- a right insight rail for contradictions, synthesis, draft status, and build
  implications

Secondary live threads should remain visible enough for concurrent
comprehension, even if only one answer canvas is primary at a time.

### 7.2 Desktop and laptop fallback

For standard desktop and laptop widths:

- keep the route dense and calm
- allow the right-side insight rail to collapse into tabs, drawers, or stacked
  attached context
- preserve fast thread switching and answer continuity

### 7.3 Mobile and narrow-width fallback

For mobile and narrow widths:

- preserve truthful access to the same prompt-bank content
- collapse the side surfaces into intentional disclosures or step-through
  sections
- do not fork the product model into a different interaction contract

## 8. Touched Surfaces

- `planner-schemas/src/artifacts/socratic.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_batch_planner.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_protocol.rs`
- `planner-core/src/pipeline/steps/socratic/prompt_response_adjudicator.rs`
- `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`
- `planner-server/src/session.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-server/src/api.rs`
- `planner-solid/src/lib/types.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/app.css`

## 9. Migration Strategy

### 9.1 Compatibility first

Implementation must preserve the existing single-select plus freeform flow
while the new contract lands.

Required strategy:

- treat the current answer model as the compatibility baseline
- add widened fields in parallel
- keep older prompt-bank reads and older saved answers valid where practical

### 9.2 Incremental delivery order

Phase 38 should not be implemented as one giant redesign. The intended order
is:

1. schema and transport widening
2. planner and adjudication enablement
3. one representative multi-modal item path
4. one bounded command-desk route slice

### 9.3 Child spec sequence

Phase 38 now splits into these child lanes:

1. [Planner SolidStart Phase 38.1 Socratic Prompt Contract And Transport Widening Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-1-socratic-prompt-contract-and-transport-widening-spec.md)
2. [Planner SolidStart Phase 38.2 Socratic Multimodal Planner And Adjudication Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-2-socratic-multimodal-planner-and-adjudication-spec.md)
3. [Planner SolidStart Phase 38.3 Session Command Desk Ultra-Wide Layout Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-3-session-command-desk-ultra-wide-layout-spec.md)

They are intentionally parallel in topic but sequential in delivery pressure:
38.1 establishes the contract, 38.2 makes the engine use it, and 38.3
re-architects the route once the richer prompt shape is real.

Implementation state:

- 38.1 is now implemented
- 38.2 is now implemented
- 38.3 is now implemented
- Phase 38 is now closed as an implementation thread; no additional child slice
  is promoted from this parent yet

## 10. Acceptance Criteria

1. a durable spec exists for widening the Socratic prompt contract beyond the
   legacy `selected_option_id + custom_text` answer shape
2. the spec explicitly defines multi-modal interaction as structured response
   modes rather than media upload
3. the spec explicitly reopens the Phase 37 single-active-thread ceiling for
   ultra-wide layouts
4. the spec preserves backward compatibility and responsive fallback as
   first-class constraints
5. `docs/project-plan.md` and `docs/session-start-and-doc-index.md` both point
   to this phase as the next truthful planning branch

## 11. Verification Plan

Planning verification for this draft:

- direct source review of the current schema, planner, frontend transport, and
  route layout files named above
- planning sync in `docs/project-plan.md`
- doc-index sync in `docs/session-start-and-doc-index.md`

Implementation verification for follow-on child slices should include:

- targeted Rust unit tests for widened prompt/answer serialization
- targeted server/websocket coverage for backward-compatible transport
- Solid unit coverage for mode-specific rendering helpers
- browser proof for ultra-wide, desktop, and narrow-width continuity

## 12. Rollback / Fallback

If the full command-desk delivery proves too broad in one slice:

- keep the widened schema and transport work
- prove one representative multi-modal composer inside the existing route
  hierarchy first
- do not claim the 5K command-desk outcome until the route hierarchy actually
  changes

## 13. Readiness Judgment

This spec is intentionally `draft`.

It is truthfully bounded enough to guide the next child slice, but not yet
ready for one-pass delivery because the first implementation tranche should be
split into at least:

- contract widening
- planner/adjudication proof
- route hierarchy proof

## 14. Open Questions

No blocker changes the parent direction, but these should be tightened before
promotion of the first child implementation slice:

- which exact structured answer payload shape is least disruptive to current
  checkpoint persistence
- whether compare-path prompts and ranked-choice prompts should share one
  generalized field-map representation or separate mode-specific payloads
- what width threshold should trigger the full three-surface command-desk
  layout in the current design system

## 14. Implementation Outcome

Implemented on 2026-04-02 across three bounded child slices:

- 38.1 widened the Socratic prompt and answer transport contract
- 38.2 made planner and adjudication use the richer response modes for a real
  non-legacy contradiction flow
- 38.3 turned the session route into a truthful ultra-wide command desk while
  preserving desktop and narrow-width fallback behavior

Phase 38 is now complete as the multimodal Socratic and ultra-wide session
workspace follow-on.
