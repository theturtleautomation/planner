# Socratic Lobby Live Virtualized Document Spec

**Status:** implemented precursor  
**Date:** 2026-03-23  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md), [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Socratic Hybrid Question Routing And Latency Spec](/home/thetu/planner/docs/socratic-hybrid-question-routing-and-latency-spec.md), [Socratic Question Canvas Alignment And Visual Refinement Spec](/home/thetu/planner/docs/socratic-question-canvas-alignment-and-visual-refinement-spec.md)

> Planning note (2026-03-24): this spec remains the record of the delivered
> continuous-document route, but it no longer defines the selected future
> product target. The successor is
> [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md),
> which replaced the continuously rendered document model with an active-thread
> workspace in the live React route. The broader greenfield future-state now
> lives under
> [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md).
> master-detail workspace while reusing the delivered split-pane shell,
> normalized state, and local-fast browsing primitives as migration inputs.

## Implementation Sync

The first four bounded delivery slices landed on `planner-web` on 2026-03-23,
the browser verification surface was extended in the follow-on verification
pass, and the final manual verification checklist was rerun on 2026-03-24.

Delivered across these slices:

- added a Jotai-backed Socratic document graph to hold normalized category,
  question, metadata, and retained draft state across live workspace updates
- hydrated that graph from the current websocket-driven workspace and prompt
  payloads instead of keeping route knowledge trapped in the active prompt
  renderer
- switched the split-pane consultant shell to read category telemetry and
  retained thread-question preview state from the document graph
- seeded returning prompts from retained document drafts so previously known
  answers can repopulate when a prompt revisits the desk
- replaced right-pane prompt swapping with a continuous category document that
  renders active prompt sections, retained question sections, preview sections,
  and preparing states in one reading surface
- wired left-index interaction to jump the desk to the requested category while
  only sending server focus changes when the user selects a different live
  thread
- introduced TanStack Virtual-based section virtualization with a bounded
  static fallback for first render and non-measured environments, preserving
  prompt submission semantics and scroll ownership
- deleted the remaining `PromptBatchPanel` and `PromptCard` compatibility shims
  and moved the remaining prompt surface imports onto `QuestionCanvas` /
  `QuestionBlock`
- added selector-isolation proof for draft updates, a seeded 15-category /
  120-question typing guardrail, and live-insertion anchor tests for the
  virtualized document and document graph
- completed left-index keyboard navigation so arrow traversal previews document
  sections, Enter jumps and focuses the first answerable item when it exists,
  and active-row state stays pinned to the currently edited section
- added browser-level Playwright proof that keyboard-driven deep jumps still
  land on the correct section after a live category insertion, using a
  dev-only test hook into the live Jotai document graph rather than a duplicate
  browser module instance
- reran the manual verification checklist against the live route in a real
  browser:
  - desktop deep-scroll pass confirmed the right desk remains the primary
    vertical scroll owner while the document stays stable as the live question
    hydrates into the document
  - keyboard-only traversal from the left index into the live answer field and
    back to the index completed successfully once the prompt became answerable
  - short-viewport validation at `1280x600` confirmed the desk can scroll the
    live prompt until `Submit answered items` is visible and enabled
  - tablet and mobile spot-checks at `768x1024` and `390x844` confirmed
    containment without horizontal overflow, while preserving readable access
    to the live prompt and submit action

Verification completed for these slices:

- `npm --prefix planner-web test -- src/stores/__tests__/socraticDocumentStore.test.ts src/components/__tests__/QuestionCanvas.test.tsx src/components/__tests__/VirtualizedCategoryDocument.test.tsx src/components/__tests__/SocraticWorkspace.performance.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm --prefix planner-web test -- src/components/__tests__/SocraticWorkspace.test.tsx src/stores/__tests__/socraticDocumentStore.test.ts`
- `npm --prefix planner-web run build`
- `cd planner-web && ./node_modules/.bin/playwright test e2e/session-ethereal.spec.ts`

## 1. Problem & Decision

The current lobby implementation still forces a false choice:

- dynamic server-authored categories and prompts, or
- all planning data visible in one continuous workspace

The product direction is now explicitly both:

- categories and questions must continue to appear dynamically as the system
  learns from the session
- all currently known Socratic data must be loaded into one client-side
  workspace at once
- the user must be able to navigate and edit that workspace as one continuous
  planning document without page transitions or active-pane swapping

The critical architectural clarification is:

- **loaded at once** means all currently known categories, questions, drafts,
  and AI context are resident in normalized client state
- **rendered at once** does **not** mean every question must stay mounted in
  the DOM at all times
- the correct render strategy is a continuous virtualized document so the user
  experiences one workspace while the browser only mounts the visible window

This spec selects that architecture as the authoritative replacement model.

## 2. User Outcome

After this spec is delivered:

- the left pane remains a dense persistent index of all known categories
- the right pane becomes one continuous consultant document, not a prompt swap
  area
- clicking a category jumps the document to that section instead of replacing
  the desk body
- newly generated categories and questions insert live into the document
  without route changes or modal drill-down
- the user can treat the lobby as one working paper while still benefiting
  from live AI generation
- typing remains responsive even when the workspace contains the full current
  question set

## 3. Product Contract

### Final interaction model

- the route remains a `100vw` by `100vh` split-pane application shell
- the left index is persistent, dense, and keyboard navigable
- the right desk is the primary vertical scroll container
- the right desk contains all currently known category sections in one
  continuous reading and editing document
- each category section may be collapsed, expanded, or partially virtualized,
  but its data remains loaded in memory
- focus and navigation move within one document; the user is not pushed
  between separate prompt pages

### Explicitly rejected end-states

- a single active-category detail pane that swaps content on row click
- a monolithic always-mounted 120-input DOM tree without virtualization
- a spreadsheet-first grid as the default interaction model
- a classic HTML form mental model with one final submit gate as the primary
  framing

## 4. Scope Boundaries

### In Scope

- replacing the active-category detail pane with a continuous document desk
- maintaining live server-authored category and question insertion
- loading all currently known Socratic data into normalized client state
- virtualizing category sections and question blocks in the right desk
- dense left-index telemetry derived from normalized state
- scroll anchoring, jump-to-section behavior, and focus restoration
- store architecture changes required to support the document model
- verification of typing latency, scroll ownership, and live insertion

### Out Of Scope

- changing the Socratic product from dynamic generation to a fixed static
  questionnaire
- redesigning other Planner routes
- revisiting the selected model-family routing in the backend unless a later
  delivery slice proves it is required
- converting the lobby into a spreadsheet or AG Grid product surface

## 5. Architecture Overview

### Shell model

- root shell remains `100vw`, `100vh`, `overflow: hidden`
- top chrome remains fixed
- main frame remains split-pane
- left index remains fixed-width
- right desk remains the only primary vertical scroll owner

### Data model

The lobby must keep a normalized entity graph for all known Socratic content:

- `categoriesById`
- `categoryOrder`
- `questionsById`
- `questionOrderByCategory`
- `answersByQuestionId`
- `llmContextByCategoryId`
- `generationStateByCategoryId`
- `viewportState`
- `selectionState`

All currently known categories, questions, metadata, and drafts must be kept in
that graph even when not currently visible.

### Render model

The right desk must render a single continuous document using virtualization:

- top-level virtual units are category sections
- category sections may contain measured question blocks
- the virtualizer owns DOM mount/unmount based on visibility
- a category row click scrolls to the section instead of swapping desk content
- section headers may be sticky within the right desk to preserve orientation

### Synchronization model

Server updates must mutate the normalized graph incrementally:

- new category appears -> insert category into order and create its section
- new question appears -> insert into that category section in document order
- question removed or revised -> patch the entity graph without losing unrelated
  drafts
- AI context changes -> update the category metadata line without re-rendering
  the entire document

The UI must treat these as live document deltas, not as prompts that replace
the entire right pane.

## 6. State Management Decision

### Canonical target

The final target store architecture is **Jotai plus TanStack Virtual**.

Reasons:

- Jotai is better suited to a large, changing entity graph where each category,
  question, answer, and derived telemetry value should be observed
  independently
- Jotai's `selectAtom`, `focusAtom`, and `splitAtom` patterns are a strong fit
  for a live-generated question tree with fine-grained rerender isolation
- TanStack Virtual is a strong fit for one continuous document with dynamic
  section heights and measured question blocks

### Transitional allowance

The currently delivered `zustand` store may remain during migration, but it is
not the final state contract for this spec. New architecture work must target
the Jotai-based entity graph rather than deepening the current active-prompt
store into another temporary local maximum.

### React primitives

Implementation should use modern React prioritization intentionally:

- `useSyncExternalStore` semantics through the selected state library
- `useDeferredValue` for non-urgent derived views when input responsiveness must
  take priority
- transitions for non-urgent insertion, filter, and jump feedback where
  appropriate

## 7. Component Model

The final route should converge toward this hierarchy:

```text
<SessionPage>
  <SocraticLobbyDocumentShell>
    <SocraticLobbyTopBar />
    <SocraticLobbyMainFrame>
      <SocraticThreadIndex />
      <SocraticDocumentDesk>
        <VirtualizedCategoryDocument />
          <CategorySection />
            <CategorySectionHeader />
            <CategoryContextMeta />
            <QuestionBlock />
              <QuestionPrompt />
              <SeamlessInput />
```

Rules:

- `SocraticThreadIndex` never owns the question content
- `SocraticDocumentDesk` owns scroll, anchoring, and virtualization
- `QuestionBlock` remains flat; card metaphors must not return
- compatibility shims for `PromptBatchPanel` and `PromptCard` should be deleted
  once no longer required by remaining imports

## 8. Interaction Rules

### Category navigation

- click on a left-index row scrolls to the matching section
- arrow-key traversal in the left index updates active selection and optionally
  previews jump targets
- Enter on an index row scrolls and focuses the first answerable item in that
  category when one exists

### Document behavior

- sections currently in view determine the active row highlight
- if the user is editing in a section, that section remains active even if new
  content is inserted elsewhere
- new categories or questions may appear with restrained insertion affordances,
  but they must not steal focus

### Input behavior

- answers remain local-first and immediate
- draft edits update category telemetry without whole-document rerenders
- submission remains question- or batch-aware based on existing Socratic
  semantics, but the document itself must not behave like one giant classic
  form

## 9. Visual Contract

The consultant-desk visual language remains valid, but the right pane changes
from one active batch to one continuous document.

Non-negotiable rules:

- dark split-pane shell remains
- left index remains `32px` row height
- serif title treatment remains limited to section titles, not oversized hero
  prompts
- AI metadata remains mono and subordinate
- question blocks stay flat on the canvas with hairline separation
- footer actions must remain reachable on short viewports
- document density must support 15 categories and up to 120 currently known
  answerable items without feeling theatrical or oversized

## 10. Performance Contract

### Hard requirements

- a single keystroke in one answer must not trigger a whole-document rerender
- the document must remain responsive with a seeded 15-category, 120-input
  dataset
- left-index telemetry updates must remain perceptibly immediate during typing
- section virtualization must preserve draft state and cursor continuity
- scroll jumps must land on the correct category even after live insertions

### Evidence threshold

Implementation is not complete until the repo contains proof for:

- seeded 15-category / 120-input typing smoke
- scroll anchoring after live category/question insertion
- document-root scroll lock with right-desk ownership
- selector or atom isolation tests showing local updates do not repaint the
  entire desk

## 11. Verification Plan

### Automated

- unit tests for normalized category, question, and answer graph reducers
- unit tests for derived telemetry selectors or atoms
- component tests for section rendering, jump-to-section behavior, and answer
  persistence
- Playwright coverage for scroll ownership, deep document jumps, and live
  insertion stability
- a seeded performance harness that records typing responsiveness against a
  full 15x8 fixture

### Manual

- desktop pass with deep document scrolling and live insertion
- keyboard-only pass from left index into question editing and back
- short-viewport pass confirming actions remain reachable
- mobile and tablet spot-checks for containment and readability

## 12. Migration Plan

The next delivery work should be executed as three bounded slices:

1. **Entity Graph & Store Migration**
   - introduce the Jotai entity graph and adapters for current websocket data
   - preserve the current consultant shell while moving data ownership out of
     the active-prompt store
2. **Continuous Document & Virtualization**
   - replace right-pane prompt swapping with a virtualized category document
   - wire row clicks to jump-to-section behavior
3. **Cleanup & Proof**
   - delete compatibility shims
   - land performance proof and live-insertion verification

## 13. Rollback & Fallback

- if the Jotai migration lands but the continuous document does not, the route
  may temporarily keep the current shell while the new entity graph is hidden
  behind the existing render path
- if continuous rendering lands but live insertion causes instability, the UI
  may temporarily stage new sections behind restrained reveal affordances rather
  than immediate auto-insertion
- falling back to the old active-prompt desk should be treated as temporary,
  not as the target contract

## 14. Open Questions

None material enough to block bounded implementation.

The main product decision has been made:

- the lobby must remain dynamically generated
- the workspace must behave like one loaded planning document
- virtualization is the chosen way to satisfy both requirements without paying
  the cost of a permanently mounted full DOM tree

## 15. Readiness Judgment

This spec is **in progress and ready for the next delivery slice**.

Why:

- the product direction is explicit and the selected architecture is now partly
  delivered
- the remaining work is bounded to final verification evidence rather than new
  structural implementation
- the migration relationship to the currently delivered consultant shell is
  clear
- the remaining verification burden is specific and testable

Next valid move:

- execute the next bounded delivery slice for **Final Verification Closeout**
  against this spec, using the landed Jotai document graph and virtualized
  consultant document as the new source of truth
