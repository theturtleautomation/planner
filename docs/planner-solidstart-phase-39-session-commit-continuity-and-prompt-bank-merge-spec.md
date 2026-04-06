# Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec

**Status:** implemented  
**Date:** 2026-04-02  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md), [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md), [Planner SolidStart Phase 38.3 Session Command Desk Ultra-Wide Layout Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-3-session-command-desk-ultra-wide-layout-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-02 direct inspection of `planner-solid/src/routes/sessions/session-workspace-controller.ts`, `planner-solid/src/lib/prompt-bank.ts`, `planner-solid/src/lib/session-transport.ts`, `planner-solid/src/lib/mock/store.ts`, `planner-server/src/ws_socratic.rs`, `planner-server/src/api.rs`, and `planner-core/src/pipeline/steps/socratic/socratic_engine.rs`

## 1. Purpose

Fix the session-workspace regression where `Commit and next` behaves like a reset event instead of a dynamic local-first progression step.

This is not another Phase 38 layout pass. It is a post-Phase-38 continuity hardening slice focused on answer submission semantics, prompt-bank update handling, and graceful workspace evolution.

## 2. Problem

The current route still mixes a local prompt-bank graph with fetch-and-replace behavior in the moments that matter most.

Two concrete issues drive the bad UX:

- `handleCommitAnswer()` only submits a `prompt_response` when the whole thread is processed, so `Commit and next` is not actually the primitive for “submit this answer and evolve the workspace now”
- the websocket update path still uses `refetchSession()` and `refetchPromptBank()` as part of the normal live-update flow, which can observe transient checkpoint states while the engine is clearing and rebuilding the bank

That combination can make committed work feel like it caused a page reset, a workspace collapse, or an over-severe question disappearance.

## 3. User Outcome

After this slice:

- `Commit and next` persists and submits the current answer immediately
- the route advances locally without waiting for a route reset
- prompt/category updates merge in place through the prompt-bank graph
- the active workspace preserves continuity unless the active task truly becomes invalid
- when the runtime supersedes the active question or thread, the route explains the change calmly and hands off to the next valid task

## 4. Scope

### In Scope

- answer-level submit semantics in the Solid session controller
- continuity-preserving prompt-bank merge behavior in the session route
- removing normal-happy-path prompt-bank/session refetch behavior from websocket updates
- graceful invalidation handling for active thread or active question replacement
- targeted frontend-mock continuity proof and focused unit coverage

### Out Of Scope

- broader route redesign
- planner/adjudication logic changes
- backend transport redesign beyond the smallest seam needed for truthful answer-level progression
- unrelated project, knowledge, discovery, or admin route work

## 5. Findings Bound Into This Spec

### 5.1 Commit semantics are too coarse

`handleCommitAnswer()` advances locally, but `submitThread()` only sends the `prompt_response` when every item in the thread is processed.

That means the UX affordance says “commit this answer now” while the controller behavior still says “stage locally until the thread is done.”

### 5.2 Live-update handling is too refetch-driven

The controller currently treats websocket updates too much like cache invalidation events:

- `prompt_bank` merges, then still calls `refetchSession()`
- `planner_event`, `converged`, `pipeline_complete`, and `error` can trigger `refetchPromptBank()` and `refetchSession()` in normal interactive flow

### 5.3 Transient empty-bank states are observable through refetch

The Socratic engine clears `prompt_bank` while re-planning after a response, then republishes the next prompt bank.

That is acceptable inside the runtime, but it is not acceptable for the route to refetch the HTTP prompt-bank resource during that transient window and treat it as the new UI truth.

## 6. Contract

### 6.1 Commit contract

`Commit and next` must mean:

1. save the current draft state
2. submit the current answered item immediately
3. mark the committed item processed locally
4. advance focus locally to the next best task
5. merge server prompt-bank updates afterward without wiping continuity

It must not mean “wait until the whole thread is complete before the runtime learns about the answer.”

### 6.2 Merge contract

Normal prompt-bank evolution must be local-first.

Required behavior:

- prefer websocket `prompt_bank` payloads over immediate REST refetch
- preserve the current active thread and question when they still exist in the next bank
- if the active task disappears, select the next valid task and surface a calm continuity notice
- keep REST refetch as explicit recovery or lifecycle fallback, not as the main post-answer path

### 6.3 Invalidation contract

If the runtime supersedes a question or thread:

- do not blank the route
- do not collapse to loading if a valid next task exists
- preserve the shell and hand off to the next valid task
- surface restrained feedback such as “Planner updated the next questions after your last answer.”

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/lib/session-transport.ts`
- `planner-solid/src/lib/mock/store.ts`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts`
- `planner-server/src/ws_socratic.rs` only if a smallest backend seam is required

## 8. Acceptance Criteria

1. committing one answer submits that answer immediately instead of waiting for whole-thread completion
2. websocket `prompt_bank` updates no longer cause normal-happy-path prompt-bank refetch resets
3. the active task is preserved across prompt-bank updates when it remains valid
4. if the active task is superseded, the route transitions gracefully instead of collapsing
5. browser proof covers commit-and-next continuity in frontend-mock mode

## 9. Verification Plan

- targeted Solid unit tests around prompt-bank continuity helpers and answer-level submit shaping
- updated mock/session tests for answer-level progression where needed
- frontend-mock Playwright proof that `Commit and next` keeps the route mounted and progresses without a reset
- `npm --prefix planner-solid run build`
- `git diff --check`

## 10. Rollback / Fallback

If full answer-level progression proves too broad in one pass:

- keep immediate answer submission
- keep the prompt-bank merge continuity fix
- leave secondary continuity polish for a later follow-on

Do not keep the old whole-thread submit behavior as the default fallback if the route already claims per-answer commit semantics.

## 11. Implementation Outcome

Implemented on 2026-04-02 as a bounded post-Phase-38 continuity hardening slice.

Delivered behavior:

- `Commit and next` now submits the current answered item immediately instead of waiting for whole-thread completion
- the controller applies websocket `prompt_bank` updates through the local graph without normal-happy-path prompt-bank refetch resets
- active thread and active question continuity are preserved when they remain valid in the next bank
- when the active item is superseded, the route preserves the shell, hands off locally to the next valid task, and surfaces a calm continuity notice
- the frontend-mock runtime now simulates one real post-answer prompt-bank progression so browser proof covers the continuity contract instead of only converged-complete behavior

## 12. Verification Evidence

- `npm --prefix planner-solid test -- --run src/lib/workspace.test.ts src/lib/prompt-bank.test.ts src/lib/mock/store.test.ts`
- `npm --prefix planner-solid run build`
- `cd planner-solid && VITE_PLANNER_FRONTEND_MOCK=1 npx playwright test --config playwright.frontend-mock.config.ts e2e/phase-35-frontend-mock.spec.ts`
- `git diff --check`

## 13. Open Questions

None block implementation.

The regression is concrete, the fix surface is clear, and the smallest truthful delivery slice is bounded.
