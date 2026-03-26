# Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec

**Status:** implemented  
**Date:** 2026-03-26  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 22 Session Workspace Master-Detail Density And Autosave Spec](/home/thetu/planner/docs/planner-solidstart-phase-22-session-workspace-master-detail-density-and-autosave-spec.md), [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md), [Planner SolidStart Phase 33 Session Workspace Interaction And Artifact Refinement Spec](/home/thetu/planner/docs/planner-solidstart-phase-33-session-workspace-interaction-and-artifact-refinement-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-03-26 direct inspection of the implemented `planner-solid` session workspace, the current `session.png` desktop layout, and live user feedback rejecting the artifact-first split because it hides editable work behind redundant mirrored surfaces
**Implementation Update (2026-03-26):** the Solid session workspace, controller commit path, route stylesheet, and targeted Playwright proof now implement the single-surface question-bank model. The session page renders all banked questions in one local workspace, keeps compact jump navigation, removes the mirrored artifact pane, and preserves truthful draft-save and commit behavior.

## 1. Executive Judgment

The active session workspace is solving the wrong primary problem.

The repo already has the correct runtime truth for first reveal:

- a real initial prompt bank exists before the lobby becomes usable
- the user can switch locally across banked threads
- drafts save truthfully

But the implemented screen turns that truth into the wrong interaction model:

- only one editable prompt is mounted at a time
- the second half of the page mirrors the same information as an artifact shell
- the route still asks the user to infer the full work set instead of directly
  exposing it

The bounded correction is now explicit:

- remove the artifact-first split as the preferred session workspace
- make the question bank itself the primary workspace
- render all banked questions from the start in one local, navigable surface
- preserve truthful draft save, commit, and queued-work contracts

## 2. User Outcome

After this phase:

- every banked question is visible at first reveal inside the main workspace
- the user can click around locally across threads and questions without
  waiting for a server round trip or swapping to a second pane
- the route has one dominant surface instead of an interview lane plus a
  redundant artifact lane
- queued later work remains visible, but clearly separate from answerable work
- the session page reads like a planning desk, not a split dashboard

## 3. Problems To Solve

- the current artifact pane duplicates question state instead of exposing
  editable work directly
- the current split layout hides most answerable questions behind one-current-
  prompt gating
- the topbar and summary strips spend space reinforcing the artifact concept
  instead of helping the user move through the actual bank
- narrow-width behavior preserves the same wrong split by turning it into tabs

## 4. Scope

### In Scope

- `/sessions/:sessionId` interaction model and presentational structure in
  `planner-solid`
- replacing the desktop artifact split with a single question-bank workspace
- rendering all banked prompt items in the DOM at once
- local thread and question navigation inside the one main workspace
- keeping truthful draft autosave and per-question commit behavior
- keeping queued threads visible as secondary, non-answerable work
- targeted browser proof for desktop and narrow-width continuity

### Out Of Scope

- changing the backend prompt-bank contract
- changing websocket/runtime truth
- inventing client-authored prompts or synthetic question text
- broad redesign of unrelated routes

## 5. Contract

- the initial prompt-bank contract from
  [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md)
  remains fixed
- all banked prompt items must be directly inspectable from the first reveal
- the route must not render a second major pane that restates the same question
  content as an artifact projection
- queued work may stay visible, but it must remain clearly non-answerable
- draft save and commit actions remain truthful and recoverable

## 6. Product Decision

### 6.1 Primary workspace model

The future-state workspace is a single question-bank surface.

Required direction:

- one main scrollable workspace
- all banked threads rendered as sections in that workspace
- all banked questions rendered in those sections from the start
- no permanent artifact pane
- no artifact tab fallback on smaller widths

### 6.2 Navigation model

The route should still support fast orientation, but not through a second equal
page.

Required behavior:

- keep a compact thread navigator for local jumps
- clicking a thread jumps to that section immediately on the client
- clicking or focusing a question sets the active task locally for commit and
  keyboard continuity
- the user never lands on a visible thread that lacks its real banked prompts

### 6.3 Question presentation model

Required behavior:

- each banked question renders as an editable operational block
- question text, options, input, save state, and commit affordance live in the
  same block
- the active question may receive restrained emphasis, but inactive questions
  remain fully readable and directly accessible
- the UI must not mirror each prompt into a second “prompt anchor” or “working
  draft note” shell elsewhere on the page

### 6.4 Queued-work model

Required behavior:

- queued threads remain visible below or after the banked sections
- queued rows must be clearly marked as later work, not locally answerable work
- queued rows may provide summary context only; they must not pretend to be
  loaded question blocks

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- `planner-solid/e2e/phase-33-session-workspace-interaction-and-artifact-refinement.spec.ts`

## 8. Acceptance Criteria

1. the session page no longer renders an artifact pane or artifact/interview tab
   split
2. all banked questions for visible answerable threads are rendered in the main
   workspace from first reveal
3. the user can jump locally across threads without route reloads or waiting
   states for already banked work
4. each question remains directly editable with truthful draft-save and commit
   behavior
5. queued work remains visible but visually subordinate and clearly non-
   answerable
6. the redesign removes mirrored prompt-anchor and draft-projection chrome
   instead of merely restyling it

## 9. Verification Plan

- `npm --prefix planner-solid run test -- --run planner-solid/src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid exec playwright test e2e/phase-33-session-workspace-interaction-and-artifact-refinement.spec.ts`
- `npm --prefix planner-solid run build`

## 10. Rollback / Fallback

If the full all-questions-visible pass is too broad in one slice:

- keep the route single-surface
- render all banked thread sections, but allow only one expanded question at a
  time within each section
- do not restore the artifact pane as the fallback

## 11. Open Questions

None block implementation. The user requirement is explicit: all banked
questions must be available from the start, and the current split view is not
the right product shape.
