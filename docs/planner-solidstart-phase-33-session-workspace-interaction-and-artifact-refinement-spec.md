# Planner SolidStart Phase 33 Session Workspace Interaction And Artifact Refinement Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 22 Session Workspace Master-Detail Density And Autosave Spec](/home/thetu/planner/docs/planner-solidstart-phase-22-session-workspace-master-detail-density-and-autosave-spec.md), [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 follow-on review of the implemented session workspace split, the current `session-workspace-screen.tsx` artifact/interview surface, and the precursor Phase 22/23 interaction decisions after Phase 32 route-topology clarification
**Planning Note (2026-03-26):** follow-on user validation after implementation rejected the artifact-first split as the preferred session workspace because the route must keep all banked questions directly available from the start. The active correction is now [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md).

## 1. Executive Judgment

The active session workspace no longer has a structural blocker. Phase 31 split
the route internals cleanly, and Phase 32 clarified the surrounding route
topology. What remains is a pure product/interaction pass on the workspace
itself.

The repo is now clear enough to lock the next workspace decision:

- keep the Phase 23 artifact-first split as the long-term direction
- do not swing back to a question-map-first desktop model
- refine the workspace so the artifact feels like the dominant object and the
  interview lane feels like a compact control surface instead of a second equal
  page
- reduce topbar/action sprawl and repeated heading chrome without changing the
  runtime contract

This phase should therefore be a bounded interaction-and-artifact refinement
slice, not another structural refactor and not a runtime rewrite.

## 2. User Outcome

After this phase:

- the active session workspace has one current interaction model rather than a
  functional but transitional split
- the artifact is visually dominant and reads more like a working document than
  a mirrored answer projection
- the interview lane is calmer, denser, and more task-led
- topbar actions, return navigation, and status chrome feel grouped and
  intentional
- narrow-width behavior still preserves continuity, but with cleaner emphasis
  and tab hierarchy

## 3. Problems To Solve

- the current workspace still shows residue from both the Phase 22
  master-detail model and the Phase 23 artifact split
- the topbar exposes too many same-weight actions before the user reaches the
  actual working surface
- the interview lane still spends too much space on repeated thread framing and
  meta copy
- the artifact lane is truthful, but it still reads as a prompt projection more
  than a clear evolving planning document
- narrow-width tabbing works, but the hierarchy between interview and artifact
  emphasis is still visually underspecified

## 4. Scope

### In Scope

- the user-facing interaction model for `/sessions/:sessionId`
- topbar/action grouping and density
- interview versus artifact emphasis and switching behavior
- pane hierarchy for desktop and narrow widths
- artifact-document readability and section hierarchy
- visual/interaction refinement built on the existing truthful runtime state
- browser proof for desktop and narrow-width continuity after the refinement

### Out Of Scope

- startup/runtime truth changes
- websocket/protocol redesign
- deleting the session route family
- project/work-entry IA decisions beyond what the session workspace itself
  requires
- another structural controller split

## 5. Contract

- the bank-first runtime and saved-brief startup contracts remain fixed
- Phase 31 structural decomposition and Phase 32 topology clarification are
  implemented prerequisites
- the redesign should consume the decomposed boundaries rather than reopening
  route-controller sprawl
- the artifact-first split selected by Phase 23 remains the active direction

## 6. Product Decision

### 6.1 Retained interaction model

The selected future-state remains artifact-first.

Required direction:

- desktop keeps the two-lane interview/artifact split
- the artifact remains the visually dominant lane
- the interview side becomes a compact Socratic control surface, not a second
  full document
- narrow-width layouts retain explicit surface switching instead of stacked
  dual-scroll panes

This phase explicitly does not select:

- a return to permanent question-map-first desktop
- a single-column tall document with embedded prompts
- another interaction-model reset that would obsolete Phase 23

### 6.2 Workspace refinement targets

This phase should refine:

- topbar into a slimmer, grouped status-and-action strip
- interview lane into a calmer lane with less repeated heading/meta weight
- artifact lane into a stronger planning document with clearer section status,
  hierarchy, and queued-work treatment
- narrow-width tabs so artifact versus interview emphasis feels intentional
  rather than merely responsive

### 6.3 Action hierarchy

Required hierarchy:

- return navigation and current session status stay instantly visible
- duplicate/export/restart/retry/import actions should stop reading as one
  undifferentiated row of equal-priority controls
- low-frequency actions may be grouped, collapsed, or visually subordinated,
  but no current capability may disappear silently

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- session-workspace styling in `planner-solid/src/app.css`
- any extracted session-workspace presentational components if needed
- browser proof for desktop and narrow-width session workspace behavior

## 8. Acceptance Criteria

1. the active session workspace now has one explicit artifact-first interaction
   model grounded against the implemented Phase 23 surface
2. topbar and session-action chrome are materially calmer and more clearly
   grouped without removing current capabilities
3. the interview lane is denser and more task-led, with less repeated heading
   and framing weight
4. the artifact lane reads more like a working planning document and less like
   raw prompt projection
5. narrow-width behavior preserves truthful interview/artifact switching
   without regressing the current tabbed fallback
6. the redesign does not reopen runtime truth as a disguised UI change

## 9. Verification Plan

- targeted browser proof for:
  - desktop artifact/interview emphasis and grouped topbar behavior
  - narrow-width tab continuity
  - preserved restart/retry/return affordances where applicable
- reuse of the current Phase 23, Phase 26, and Phase 28 browser proof surfaces
  where they already cover runtime truth and session continuity
- standard `planner-solid` lint/build verification

## 10. Rollback / Fallback

If the full refinement pass is too broad in one slice:

- keep the artifact-first split unchanged
- land the topbar/action grouping and interview-lane density cleanup first
- defer deeper document polish rather than reopening the interaction model

## 11. Open Questions

None block readiness for the bounded refinement slice. The major decision is
now explicit: keep artifact-first, refine the current workspace instead of
replacing it.

## 12. Readiness Judgment

This is ready for implementation because the structural and route-topology
prerequisites have landed, the interaction model decision is now explicit, and
the remaining work is a bounded UI refinement slice rather than an ambiguous
future redesign thread.
