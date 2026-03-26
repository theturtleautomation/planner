# Planner SolidStart Phase 22 Session Workspace Master-Detail Density And Autosave Spec

**Status:** implemented as precursor groundwork  
**Date:** 2026-03-25  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 21 Session Startup Truth And Status Clarity Spec](/home/thetu/planner/docs/planner-solidstart-phase-21-session-startup-truth-and-status-clarity-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md), [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Audit:** user-provided live route diagnosis and reference patterns on 2026-03-25 plus direct inspection of the current `planner-solid` session workspace, prompt-bank contract, and Socratic backend prompt assembly
**Planning Note (2026-03-25):** [Planner SolidStart Phase 23 Session Live Artifact Split Spec](/home/thetu/planner/docs/planner-solidstart-phase-23-session-live-artifact-split-spec.md) now supersedes the master-detail split as the preferred future-state desktop interaction model. Phase 22 remains the record of the autosave, prompt-draft persistence, and workspace cleanup precursor work.
**Implementation Note (2026-03-26):** The draft-save backend contract, dense master-detail workspace layout, responsive map sheet, and compact thread-complete action are implemented and were reused by Phase 23. The original keyboard-only traversal closeout for the master-detail model is no longer the active target because the artifact-first Phase 23 surface superseded this desktop interaction direction.

## 1. Executive Judgment

Phase 21 corrected startup truth, but it did not correct the active session
workspace itself.

The current Solid session route still behaves too much like a thin web-form
wrapper around prompt payloads:

- global actions and oversized headings consume too much of the viewport
- the active question is trapped in a bloated single-card composition
- the left thread index is too weak to carry true macro context
- answer progression still depends on a large transactional submit action
- the route does not yet feel like a dense local desktop tool

The next bounded slice should therefore be a **workspace-structure and answer
commit slice**, not another generic styling pass.

The selected direction is a **dense master-detail split**:

- permanent question map on desktop
- focused active-thread canvas on the right
- no document-level scroll
- compact typography and restrained chrome
- autosaved answer drafts
- a small explicit "done with thread" progression action instead of a large
  survey-style submit block

## 2. User Outcome

After Phase 22:

- the session workspace reads like a premium desktop planning app rather than a
  tall web form
- users can see the full currently answerable thread bank while focusing on one
  thread at a time
- the route keeps macro context and micro task context visible at the same time
- typing no longer feels staged behind a giant submit action
- drafts save quietly and truthfully while the user works
- keyboard traversal across ready threads is first-class
- queued later work remains visible without pretending it is answerable now
- the layout can later host richer multimodal answer blocks without collapsing
  into vertical card sprawl

## 3. Problems To Solve

### 3.1 Inverted information hierarchy

The current route burns too much vertical space on route-level actions, large
headings, and duplicated title structures before the actual working surface
begins.

### 3.2 Weak macro-context surface

The thread list currently behaves more like a minimal selector than a true
question map. It does not yet communicate the full answerable bank, queued
later work, or stable progress structure strongly enough to orient the user.

### 3.3 Transactional answer model

The existing `Submit answered items` action pushes the route toward a survey
mindset. That is the wrong interaction model for a dense continuous planning
workspace.

### 3.4 Canvas density failure

The active question area uses too much padding, too much visual container
weight, and too little fine-grained hierarchy. One question can consume most of
the viewport while still feeling empty.

### 3.5 Layout instability risk for richer inputs

If the route later needs inline images, audio, attachments, or structured
answer blocks, the current card-heavy stack will become unstable and
scroll-fatiguing.

## 4. Product Decision

The selected future-state for the Solid session workspace is:

- **desktop layout:** permanent two-pane master-detail split
- **left pane:** dense question map and thread telemetry
- **right pane:** focused active-thread canvas
- **context shelf:** hidden by default behind an explicit trigger
- **draft behavior:** quiet autosave, not large manual submit
- **progression action:** compact explicit thread-complete/continue action when
  the user wants the backend to adjudicate the saved answers

This slice does **not** select:

- a centered continuous ledger as the primary desktop model
- a permanent three-pane IDE layout
- a route with all answer blocks mounted in one giant scrolling document
- fake autosave copy without a real draft persistence contract

## 5. Scope

### In Scope

- `/sessions/:sessionId` workspace structure and interaction model in
  `planner-solid`
- desktop-first permanent question map plus focused canvas layout
- responsive collapse of the map/context surfaces for smaller widths
- compact chrome and strict typography hierarchy for the active session route
- truthfully showing the full current prompt bank and queued later work
- keyboard-first thread traversal
- draft autosave contract for in-progress answers
- compact progression affordance replacing the current oversized submit block
- touched backend/session contracts required to support truthful autosave and
  progression
- verification for viewport locking, keyboard traversal, autosave truth, and
  macro/micro context preservation

### Out Of Scope

- changing the upstream Socratic reasoning model beyond what the route needs to
  save drafts and present the current prompt bank truthfully
- introducing a full Notion-style general block editor in this slice
- generic file storage or rich media processing beyond the route-level answer
  surface contract
- redesigning unrelated Planner routes
- reopening the Phase 21 startup/status truth contract except where the new
  workspace chrome consumes it

## 6. Product And Technical Contract

### 6.1 Viewport-lock contract

The session workspace must behave like a desktop application shell:

- the browser document must not own the main vertical scroll
- the route shell must stay locked to `100vh`
- the question map and active canvas may scroll independently
- switching threads must never reset the outer page position because there is
  no outer page scroll to reset

### 6.2 Question-map contract

The left pane becomes the always-available map for answerable work on desktop.

Required behavior:

- render all currently banked answerable threads in a dense list
- group or separate queued later work so it is visible but not presented as
  answerable
- show compact per-thread telemetry:
  - title
  - short summary
  - question count
  - ready/queued state
  - answered-progress cue where draft state is known
- keep the active thread unmistakably selected
- keep the map visible at standard desktop widths instead of hiding it behind a
  default overlay

Narrow-width behavior:

- tablet and mobile may collapse the map into an explicit sheet or drawer
- the collapsed state must remain easy to reopen and must preserve the same
  thread order and state cues

### 6.3 Focused-canvas contract

The right pane is the active-thread desk.

Required behavior:

- remove oversized repeated route headings from the main workspace
- use compact typography and spacing
- mount only the selected thread's active question content in the main canvas
- avoid large card shells around every question block
- present question text, quick options, answer input, and local save state as
  dense operational blocks rather than theatrical forms
- preserve truthful empty states for threads that are selected but not yet
  answerable

### 6.4 Answer commit contract

The route must stop depending on a giant transactional submit button.

Selected model:

- answer edits persist as **draft saves** automatically on debounce and blur
- draft saves must be real and recoverable, not cosmetic local-only toasts
- draft save success/failure must be visible in a restrained status indicator
- sending a draft save must not itself adjudicate the thread or mutate the
  global prompt bank
- the route keeps one compact explicit progression action such as
  `Done with thread` or `Continue synthesis`
- that progression action tells the backend to adjudicate the already saved
  answers for the current thread

This preserves the required user control over "I am finished with this thread"
without forcing survey-style bulk submit behavior.

### 6.5 Prompt-bank visibility contract

This slice depends on the existing first-reveal bank truth from Phase 18 and
extends the presentation contract:

- the user must be able to see the full currently banked answerable thread set
  from the map
- the map must not collapse the workspace down to a misleading one-thread-only
  mental model when more banked work exists
- queued later work may be visible, but it must be clearly marked as queued,
  blocked, or awaiting refresh
- the active canvas must render exactly the selected thread's real prompt items
  and draft state

This slice does not require rendering every banked thread's question blocks in
the DOM at once. It preserves the selected master-detail model:

- all known work visible in the map
- one active thread mounted in the canvas

### 6.6 Keyboard traversal contract

Keyboard navigation is first-class.

Minimum required behavior:

- up/down arrows move thread selection inside the question map
- `j` / `k` may mirror the same behavior if added consistently
- `Enter` or `Space` opens the selected thread when focus is on the map
- keyboard traversal must not trigger route navigation or full resource reloads
- focus movement between the map and the active answer field must remain
  obvious and recoverable

### 6.7 Multimodal readiness contract

This slice must not trap the route in a text-area-only layout shape.

Required behavior:

- the answer blocks must allow future insertion of attachment rows, preview
  blocks, or richer editors without forcing giant nested card stacks
- the resting UI should stay quiet when only text is present
- any attachment affordance added in this slice must be compact and
  subordinate

This slice may keep true image/audio persistence as a later follow-on, but the
layout must be shaped so that adding those blocks later does not require
another root-layout rewrite.

## 7. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/lib/types.ts`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/*` for autosave/status/keyboard helpers
- new `planner-solid/src/components/session-workspace/*` primitives if extracted
- `planner-server/src/session.rs`
- `planner-server/src/api.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/ws_socratic.rs`
- `planner-schemas/src/artifacts/socratic.rs`
- `planner-solid/e2e/*`
- targeted Rust/server tests for any draft-save or thread-complete contract

## 8. Acceptance Criteria

This phase is complete only when:

1. the session route uses a fixed-height desktop shell with no document-level
   scroll during the active workspace state
2. the left question map is permanently visible on desktop and renders all
   currently banked answerable threads plus queued later work with truthful
   state cues
3. the active canvas uses compact typography and spacing and no longer relies
   on one oversized padded prompt card as the dominant UI shape
4. the route no longer uses a large `Submit answered items` action as the
   primary answer workflow
5. answer drafts save automatically on debounce and blur and the saved state is
   recoverable across refresh/reconnect
6. draft saving does not accidentally trigger thread adjudication or prompt-bank
   mutation
7. the route provides one compact explicit progression action for "finished
   with this thread"
8. keyboard-only traversal across the question map works without route reloads
   and without losing the active editing surface unexpectedly
9. the route remains truthful on tablet/mobile by collapsing the map into an
   explicit reveal surface instead of forcing an unusable permanent split
10. the session workspace is structurally ready for future richer multimodal
    answer blocks without another root-layout rewrite

## 9. Verification Plan

### Automated

- targeted Solid unit/component tests for:
  - question-map rendering and selection
  - keyboard traversal
  - autosave status mapping
  - compact progression action visibility/rules
- targeted Rust/server tests for:
  - draft-save persistence
  - reconnect/reload recovery of saved drafts
  - distinction between draft save and thread-complete adjudication
- run `npm test` inside `planner-solid`
- run `npm run lint` inside `planner-solid`
- run `npm run build` inside `planner-solid`

### Browser

- open the session workspace at desktop width and confirm the page remains
  viewport-locked while map and canvas scroll independently
- verify the question map is visible by default on desktop and shows both
  banked and queued work clearly
- answer a thread, pause typing, refresh, and confirm the saved draft returns
- use keyboard-only traversal to move between ready threads and into the active
  answer surface
- confirm that finishing a thread uses the compact progression control rather
  than a large bulk-submit block
- open the route at tablet/mobile width and confirm the map collapses into an
  explicit reveal surface without losing state

## 10. Rollback And Fallback

- if the desktop shell/layout lands cleanly before the autosave backend
  contract, do **not** ship fake autosave copy on top of local-only draft
  state
- the truthful fallback is to keep the current explicit submit model until real
  draft persistence exists
- if the desktop map density works but permanent visibility is too tight on
  narrower widths, collapse only the smaller-width layouts rather than backing
  out the desktop master-detail direction

## 11. Open Questions

Non-blocking follow-on questions:

- whether future multimodal insertion should use a slash-command menu, a compact
  floating action rail, or both
- whether the question map should eventually support inline per-thread preview
  snippets beyond title, summary, and counts

These do not block implementation readiness for this slice because the current
phase only needs the structural contract, autosave truth, and compact
master-detail interaction model.

## 12. Readiness Judgment

This spec is **ready for implementation**.

The route-level product choice is clear:

- dense master-detail split
- permanent desktop question map
- compact active-thread canvas
- no giant submit action
- real autosave drafts plus explicit compact progression

The main remaining work is delivery, not planning.
