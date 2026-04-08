# Planner SolidStart Phase 37 Session Workspace Command Rail Hierarchy Spec

**Status:** implemented  
**Date:** 2026-04-01  
**Parent:** [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md)  
**Related Planning:** [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 35.3 Session Workspace Frontend Mock Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-3-session-workspace-frontend-mock-spec.md), [Planner SolidStart Phase 29 Work Entry Summary Truth And Workflow Continuity Spec](/home/thetu/planner/docs/planner-solidstart-phase-29-work-entry-summary-truth-and-workflow-continuity-spec.md), [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-01 direct inspection of `planner-solid/src/routes/sessions/session-workspace-screen.tsx`, `session-workspace-controller.ts`, `session-workspace-view.ts`, `planner-solid/src/app.css`, the frontend shell contract in `planner-solid/src/app.tsx`, and a design-only review of the active session route against the current Builder-facing session workflow goal
**Implementation Update (2026-04-01):** the session route now uses a compact header, sticky command rail, and single active-thread work area in `planner-solid`, while preserving the existing controller/runtime contracts. Inactive question cards are collapsed to preview state, queued threads moved into subordinate rail disclosure, and frontend-mock proof now covers local rail thread switching through a dedicated multi-thread session scenario.

## 1. Executive Judgment

Phase 34 corrected the wrong artifact-first model, but the current question-bank
workspace still says the same thing too many times.

The implemented route currently duplicates orientation and progress across:

- the session header
- the summary-pill strip
- the sticky jump bar
- per-thread section headers
- per-question local chrome

That leaves the page with the right data and the wrong hierarchy. The user is
here to work one thread and one question at a time, but the page still behaves
like a dashboard summary stacked on top of a question bank.

The bounded correction is now explicit:

- keep the bank-first session truth
- keep all banked work available locally from first reveal
- replace the current stacked summary-plus-jump-plus-thread-shell with a
  command rail and a single active-thread work area
- demote repeated progress, explanatory copy, and inactive-thread chrome

## 2. User Outcome

After this phase:

- `/sessions/:sessionId` has one clear primary work surface instead of several
  competing orientation surfaces
- the user can still move instantly across any banked thread without a route
  change or a new server wait
- the active thread dominates the page while inactive threads remain locally
  reachable through a compact rail
- queued later work remains available for context, but no longer competes with
  answerable work
- the route stays truthful in canonical and frontend-mock runtimes, so Builder
  edits still apply to the same real session surface

## 3. Problems To Solve

- the header is carrying identity, status, explanation, summary metrics, and
  secondary actions before the work begins
- progress is currently repeated in the header pills, thread chips, and thread
  headers
- explanatory copy is duplicated between the header intro and the jump-bar
  copy
- every question card repeats local chrome that only matters for the active
  question
- queued threads are given too much visual weight for content that is not yet
  answerable

## 4. Scope

### In Scope

- session-workspace hierarchy and layout in `planner-solid`
- replacing the current jump-band plus stacked-thread presentation with a
  command-rail layout
- compacting the session header and action chrome
- making one active thread the dominant work area on desktop
- demoting queued-later work into secondary disclosure or rail context
- preserving truthful local navigation, draft-save, commit, retry, restart,
  export, duplicate, and return-target behavior
- targeted browser proof for the Builder-facing frontend-mock runtime and the
  real route contract

### Out Of Scope

- changing the backend prompt-bank contract
- changing websocket/runtime truth or startup truth
- changing route topology or shell navigation
- inventing new session actions or backend capabilities
- broad redesign of unrelated project, queue, or knowledge routes

## 5. Contract

- the bank-first runtime truth from
  [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md)
  remains fixed at the data level: all banked threads and their prompt items
  are already available locally at first reveal
- this phase explicitly revises only the presentation rule from Phase 34 that
  required all banked questions to remain visibly rendered in one long primary
  scroll surface at the same time
- after this phase, every banked question must still be directly reachable
  from first reveal without a new server fetch or route transition
- the route must still present one truthful session workspace, not a mirrored
  second pane or a mock-only fork
- the command rail is navigation and context, not a second equal workspace
- draft save, commit, retry, restart, duplicate, export, and return navigation
  remain truthful and recoverable

## 6. Product Decision

### 6.1 Primary layout model

The selected future-state for the session route is a command-rail workspace.

Required direction:

- a compact session header at the top
- a sticky left rail on desktop for thread selection and light metadata
- one active-thread work area as the dominant surface
- no separate summary-pill band
- no separate jump-bar explanation band
- no second large panel that mirrors or competes with the active work area

### 6.2 Header requirements

The header must become identity and status chrome, not a mini dashboard.

Required behavior:

- keep the return action, session title, and one concise session-status line
- keep progress available, but collapse it to one restrained summary expression
- move non-core actions such as duplicate, export, project import, restart, and
  retry into compact secondary chrome such as an overflow group
- remove long explanatory copy when the same concept is already taught by the
  workspace itself

### 6.3 Rail requirements

The rail is the one place where inactive-thread context should live.

Required behavior:

- list answerable threads with a strong active selection state
- show concise per-thread metadata only once in the rail, not again in a
  top-level jump band and then again in a section header
- allow immediate local switching to any banked thread
- support narrow-width fallback without inventing a second route or losing
  truthful access to banked work
- keep queued-later work in a subordinate rail disclosure, appendix, or footer
  treatment instead of a peer panel in the main work stack

### 6.4 Active-thread work-area requirements

The work area is where the session should feel decisive.

Required behavior:

- show one active thread as the main workspace surface
- keep the active thread title, minimal context, and editable question blocks
  together
- preserve the current answer model: options, freeform input, draft save, and
  commit-and-advance remain intact
- reduce repeated local chrome so only the active question carries full
  emphasis
- inactive threads may be collapsed out of the main work area as long as they
  remain instantly reachable locally through the rail

### 6.5 Question-block guidelines

Question blocks should look operational, not ceremonial.

Required behavior:

- keep one clear emphasis treatment for the active question
- remove or demote duplicate save-state and hint copy that becomes repetitive
  when shown on every card
- do not require the user to interpret several status badges before answering
- preserve keyboard continuity and commit behavior from the current controller

### 6.6 Queued-work guidelines

Queued-later work must remain legible but quiet.

Required behavior:

- queued work stays present for planning context
- queued work must not take equal visual weight with active answerable work
- queued rows may show summary context only
- queued content must not appear answerable until the runtime truth actually
  changes

## 7. Touched Surfaces

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- `planner-solid/e2e/phase-35-frontend-mock.spec.ts`
- targeted session-route browser proof or route-level Playwright coverage for
  the canonical runtime if needed

## 8. Acceptance Criteria

1. the session route uses a compact header plus a command rail plus one
   dominant active-thread work area on desktop
2. the route no longer renders a summary-pill strip and a separate jump-bar
   explanation band above the work area
3. all banked threads remain directly reachable from first reveal without a
   route reload or new fetch, even if inactive threads are not simultaneously
   expanded in the main work area
4. selecting a thread from the rail updates the active work area locally and
   truthfully
5. the active question remains directly editable with the existing truthful
   draft-save and commit behavior
6. queued-later work remains visible but clearly subordinate
7. Builder-facing frontend-mock review and the real route continue to use the
   same session surface rather than diverging into mock-only layout behavior

## 9. Verification Plan

- targeted route or helper tests if any new view helpers are extracted
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
- targeted browser or Playwright proof that:
  - the session header is compacted
  - the summary-pill strip and jump-band copy are gone
  - the rail can switch active threads locally
  - queued work is still present but subordinate
  - draft save and commit-and-advance still work from the active thread surface

## 10. Rollback / Fallback

If the full active-thread-only main surface is too broad in one delivery slice:

- keep the command rail and compact header changes
- keep all thread sections mounted in the main area, but visually collapse
  inactive sections and remove the repeated top-of-page summary/jump chrome
- do not restore the old stacked summary strip plus jump band plus equal-weight
  queued panel as the fallback

## 11. Open Questions

None block readiness.

The main product decision is now explicit:

- preserve bank-first local availability
- simplify the hierarchy around one active thread
- stop treating session orientation as four separate visual systems

## 12. Implementation Outcome

Implemented on 2026-04-01 as a bounded hierarchy correction on top of the
Phase 34 question-bank workspace.

Phase 37 landed without reopening the session controller or backend contracts:

- `session-workspace-screen.tsx` now renders one compact header, a command rail
  for thread selection, and one active-thread work surface
- the summary-pill strip and jump-bar explanation band are gone
- inactive question cards now collapse to preview state so only the active
  question carries full answering chrome
- queued-later work now lives in subordinate rail disclosure instead of a
  peer main-surface panel
- frontend-mock proof now includes a dedicated multi-thread session scenario to
  verify local rail switching truthfully

Verification:

- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
