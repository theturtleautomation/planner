# Planner SolidStart Phase 23 Session Live Artifact Split Spec

**Status:** implemented  
**Date:** 2026-03-25  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 22 Session Workspace Master-Detail Density And Autosave Spec](/home/thetu/planner/docs/planner-solidstart-phase-22-session-workspace-master-detail-density-and-autosave-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Socratic Lobby Live Virtualized Document Spec](/home/thetu/planner/docs/socratic-lobby-live-virtualized-document-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Audit:** user-provided redesign diagnosis and direction on 2026-03-25 plus direct repo review of the current Solid session route, Phase 22 master-detail implementation, and prior Socratic document-workspace planning

**Implementation Update (2026-03-26):**
- the Solid session route, stylesheet, workspace helpers, and targeted browser
  coverage are now implemented on the Phase 23 artifact-first shape
- the client-side maximum-call-stack loop was removed by stopping the
  prompt-bank merge effect from tracking and rewriting `promptBankGraph`
  recursively during route hydration
- commit-and-advance now progresses truthfully through both the button and
  `Cmd+Enter` path, preserves draft-save truth, and restores focus continuity
- targeted browser verification now passes for desktop progression, saved-draft
  restoration, and the sub-`1024px` tabbed fallback

## 1. Executive Judgment

The current desktop session route is still solving the wrong primary problem.

Phase 22 improved the route by making draft persistence truthful and by
removing the most obvious survey-form failure modes, but it still frames the
experience around **answering prompts** rather than **co-authoring an artifact**.

That is now the critical product correction:

- the user should no longer type into a void
- the generated specification should become the dominant visual object
- the Socratic questioning flow should read like an input mechanism for a live
  blueprint, not like a task list or a survey

The selected next slice is therefore a **live artifact split**, not a deeper
investment in the current master-detail map/canvas shape.

Phase 22 remains useful as a precursor because it already established the
needed draft-save truth contract. It no longer defines the preferred final
desktop interaction model.

## 2. User Outcome

After Phase 23:

- the session route feels like a generative planning workspace rather than a
  questionnaire
- the user sees the evolving specification while answering prompts
- committing an answer immediately updates the visible artifact instead of
  disappearing into a hidden backend process
- the left pane becomes a lightweight Socratic feed, not a heavy dashboard
- the right pane becomes the star of the route: a live, readable spec document
- the route keeps a stable desktop shell while making the payoff visible
- multimodal answers can be inserted without breaking the structural layout

## 3. Problems To Solve

### 3.1 Survey anti-pattern

The route still asks the user to do difficult thinking without showing the
artifact that this work is supposed to create.

### 3.2 Output invisibility

The actual specification draft is not visually present in the active workspace,
so the route does not communicate progress or value while the user is working.

### 3.3 Transactional interaction framing

Even with autosave, the route still reads like a prompt-handling desk with a
completion action instead of a co-authoring surface where each answer shapes an
artifact.

### 3.4 Overweighted chrome

Too much visual importance is still given to route actions, thread framing, and
question containers compared with the artifact being created.

### 3.5 Layout instability risk

Future rich inputs must not stretch the workspace vertically or collapse the
underlying grid. The route needs stable lanes for both input and output.

## 4. Product Decision

The selected future-state for the Solid session route is:

- **desktop layout:** fixed 40/60 live artifact split
- **left pane:** compact Socratic feed for active questioning and short-range
  queue context
- **right pane:** live specification document
- **primary interaction:** `Cmd+Enter` commits the current answer into the live
  artifact and advances to the next task
- **draft behavior:** autosave still exists, but it is subordinate to visible
  artifact creation
- **progress telemetry:** subtle and shape-driven rather than large thread
  headings and explicit index chrome

This slice explicitly does **not** select:

- a permanent question-map-first desktop model as the preferred future-state
- a giant inline document full of dozens of open ghost prompts
- a floating composer widget as the default production desktop pattern
- a return to submit-button-based batching

## 5. Scope

### In Scope

- `/sessions/:sessionId` route restructuring in `planner-solid`
- replacing the desktop-first master-detail emphasis with a live artifact split
- rendering a live, readable specification document alongside the Socratic feed
- optimistic artifact updates on answer commit
- autosave reuse from Phase 22 where needed for truthful recovery
- `Cmd+Enter` answer commit and next-task advancement
- subtle section synchronization between active question and artifact section
- bounded multimodal answer slots that do not alter the core split
- planning/doc synchronization needed to make this the new selected direction

### Out Of Scope

- a full general-purpose Notion-style editor
- rewriting unrelated Planner routes
- introducing full file upload/storage or media processing infrastructure
- requiring a backend LLM rewrite before the UI can show a live artifact
- replacing the existing Socratic prompt-bank truth contract

## 6. Product And Technical Contract

### 6.1 Viewport and split-shell contract

The route remains a desktop-style fixed-height workspace:

- outer document does not scroll
- left and right panes own their own vertical scroll
- desktop uses a stable 40/60 split with the artifact on the larger side
- desktop keeps both lanes visible at once
- on viewports under `1024px`, the split must collapse into a tabbed model or
  a drawer-overlay model such as a Socratic feed bottom sheet over the
  artifact
- the responsive fallback must **not** use a vertically stacked layout of two
  independent scrollable panes because that creates a scroll-trap failure mode

### 6.2 Socratic feed contract

The left lane becomes a restrained interviewer surface:

- compact, border-light question feed
- active question visually indicated without oversized cards
- category structure shown through subtle sticky feed markers, not loud route
  headings
- suggestion pills remain available and should gain numeric keyboard affordance
  where practical
- draft-save state stays visible but tertiary

The feed is no longer the star of the page. It is the control surface for the
artifact.

### 6.3 Live artifact contract

The right lane must show a real, structured spec document while the user works.

Minimum required behavior:

- render a readable specification outline with real section headings
- map active prompt threads into corresponding artifact sections
- show section placeholders or processing states when content is not yet
  committed
- visibly update the relevant section when the user commits an answer
- keep the artifact readable even when only some sections are filled
- on initial load, render a skeletal outline of the known categories or
  sections even when their content is still empty so the user always sees the
  structural blueprint they are filling in

This artifact may begin as a **truthful local projection** built from:

- prompt-bank thread metadata
- answered draft content
- committed answer content
- existing checkpoint/category context

When backend-authored synthesized section text becomes available, it may replace
or refine the local projection. The UI must not require that richer backend
path to exist before Phase 23 can land.

### 6.4 Commit-and-advance contract

The route must remove the big explicit submit model entirely.

Selected behavior:

- answer edits autosave in the background for recovery
- `Cmd+Enter` is the primary commit interaction
- committing an answer immediately:
  - saves the latest draft
  - projects the answer into the artifact
  - marks the current question as processed
  - advances focus to the next active prompt
- after `Cmd+Enter`, DOM focus must programmatically snap into the input area
  of the next active prompt so the user can continue typing without touching
  the mouse or using `Tab`
- a small pointer-visible action may exist for discoverability, but not as a
  dominant page CTA

### 6.5 Artifact synchronization contract

The interface must connect question focus to document focus.

Minimum required behavior:

- when the active prompt changes, the right document scrolls or anchors to the
  relevant section
- the relevant section receives a subtle transient highlight
- switching prompts must never feel like the artifact is unrelated to the
  current question
- automatic artifact alignment must use smooth scrolling rather than hard jump
  motion
- manual scrolling in the right artifact pane must not steal typing focus from
  the active left-pane input or override the user's active editing flow while
  they are composing an answer

This can be implemented with route-local section ids and an Intersection
Observer or equivalent scroll-spy mechanism.

### 6.6 Stable multimodal-slot contract

The input lane must stay structurally stable when richer answers arrive.

Required behavior:

- image/audio/code attachments appear inside bounded insertion rows or trays
- the left feed width does not balloon or collapse because of one rich answer
- the right artifact does not shift horizontally when attachments are added
- rich input affordances stay compact until invoked

### 6.7 Truth contract for the artifact

The route must not fake document progress.

Allowed:

- optimistic local section content clearly based on the user's committed answer
- lightweight processing states while richer synthesis catches up
- placeholder text for unanswered sections
- a distinct draft visual treatment for optimistic user-projected text such as
  muted color, italic styling, dashed rule treatment, shimmer, or equivalent
  "ghost draft" affordance so the user understands that the section is saved
  but not yet fully synthesized

Not allowed:

- showing polished final-spec prose that does not correspond to known draft or
  synthesized state
- pretending a section is complete when only a transient unsaved draft exists

## 7. Dependencies And Reuse

Phase 23 should reuse, not discard, the durable work from Phase 22:

- backend prompt-draft persistence
- `saved_drafts` prompt-bank response support
- client-side draft recovery helpers
- truthful startup/status behavior from Phase 21

Phase 23 supersedes the Phase 22 desktop interaction model, but it depends on
the Phase 22 persistence groundwork.

## 8. Touched Surfaces

Expected touched surfaces include:

- `planner-solid/src/routes/sessions/[sessionId].tsx`
- `planner-solid/src/app.css`
- `planner-solid/src/lib/prompt-bank.ts`
- `planner-solid/src/lib/workspace.ts`
- `planner-solid/src/lib/session-status.ts`
- new `planner-solid/src/components/session-artifact/*` primitives if extracted
- backend/session types only if additional artifact-projection fields are
  needed for truthfulness
- targeted e2e coverage for split behavior, commit flow, and artifact updates

## 9. Acceptance Criteria

This phase is complete only when:

1. the session route presents a desktop 40/60 live artifact split with no
   document-level scroll
2. the artifact document is visually dominant and clearly represents the live
   payoff of the Socratic workflow
3. the left pane no longer reads like a survey card stack or dashboard shell
4. `Cmd+Enter` commits the current answer, updates the artifact, and advances
   DOM focus to the next task's input without manual pointer or tabbing work
5. answer drafts remain recoverable across refresh/reconnect
6. the route no longer relies on a large explicit submit button
7. active question focus and artifact section focus stay visibly synchronized
   with smooth automatic alignment and without right-pane manual reading
   stealing left-pane typing focus
8. the artifact remains truthful when content is optimistic, processing, or
   fully committed, and optimistic user-projected text is visibly distinct from
   finalized synthesized prose
9. the artifact pane renders a skeletal outline of the known document structure
   before sections are filled so the route never cold-starts into a blank void
10. multimodal inputs can appear without breaking the split or creating major
    layout shift
11. smaller-width layouts degrade cleanly without making either the feed or the
    artifact inaccessible, and do not use a vertically stacked two-scroll-pane
    layout

## 10. Verification Plan

### Automated

- targeted Solid tests for:
  - artifact-section projection from committed answers
  - commit-and-advance behavior
  - focus handoff to the next prompt input after `Cmd+Enter`
  - section highlight / active-section synchronization logic
  - placeholder versus committed artifact rendering
  - ghost-draft versus synthesized artifact rendering
- targeted e2e coverage for:
  - desktop split rendering
  - `Cmd+Enter` commit and next-task advancement
  - artifact update visibility
  - smooth section alignment without focus theft during active typing
  - smaller-width fallback behavior
- run `npm test` inside `planner-solid`
- run `npm run lint` inside `planner-solid`
- run `npm run build` inside `planner-solid`

### Browser

- verify that answering a question visibly updates the corresponding artifact
  section
- verify that `Cmd+Enter` moves the user forward without needing a large CTA
- verify that the right pane subtly and smoothly tracks the active left-pane
  section
- verify that the typing cursor lands in the next prompt input immediately
  after commit
- verify that optimistic artifact text uses a distinct draft treatment before
  synthesis completes
- refresh mid-session and confirm the saved draft / committed artifact state is
  truthful
- open the route on a smaller laptop width and confirm the responsive fallback
  uses tabs or an overlay drawer rather than a vertically stacked two-pane
  scroll trap

## 11. Rollback And Fallback

- if the live artifact split lands cleanly before richer synthesized prose
  exists, ship the truthful local projection rather than waiting for a full
  backend rewrite
- if the desktop split feels too compressed on narrower widths, collapse only
  the smaller layouts rather than abandoning the artifact-first direction
- if `Cmd+Enter` is implemented before visible artifact updates, do not ship
  that shortcut as the primary interaction yet

## 12. Open Questions

Non-blocking follow-on questions:

- whether the artifact should support inline code-block previews in the first
  slice or defer to later richer formatting
- whether the left feed should keep a tiny mini-map for queued later work once
  the artifact becomes the main orientation surface
- whether future multimodal insertion should use slash commands, inline trays,
  or a small floating action strip

These do not block readiness because the core direction is now clear:

- artifact-first desktop split
- live visible specification
- `Cmd+Enter` commit and advance
- autosave as support system, not primary interaction framing

## 13. Readiness Judgment

This spec is **implemented and verified**.

The user intent is now coherent and bounded:

- stop treating the route as a questionnaire
- make the live spec document the star
- preserve truthful draft persistence from Phase 22
- ship a stable artifact-first workspace before attempting broader AI-editor
  ambitions
