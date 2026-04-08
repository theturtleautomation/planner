# Socratic Lobby Consultant Desk Spec

**Status:** implemented precursor  
**Date:** 2026-03-23  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Related Planning:** [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md), [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md), [Socratic Ethereal Cascade Redesign Spec](/home/thetu/planner/docs/socratic-ethereal-cascade-redesign-spec.md), [Socratic Question Canvas Alignment And Visual Refinement Spec](/home/thetu/planner/docs/socratic-question-canvas-alignment-and-visual-refinement-spec.md), [Planner UI Reset Phase 14: Socratic Pro Max Redesign Spec](/home/thetu/planner/docs/planner-ui-reset-phase-14-socratic-pro-max-redesign-spec.md)

> Planning note (2026-03-24): this document remains the record of the first
> delivered replacement slices: permanent split-pane shell, flattened prompt
> rendering, and initial normalized desk state. It is no longer the final
> future-state contract. The selected successor is
> [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md),
> which supersedes both the active-category detail-pane end-state described
> here and the later continuous-document future-state contract.

## Implementation Sync

The second bounded delivery slice landed on `planner-web` on 2026-03-23.

Delivered across the first two slices:

- replaced the focused single-column workspace with a permanent split-pane
  consultant-desk shell on the active interview route
- rendered the left thread index from the live server-authored
  `category_snapshot.nodes` payload instead of a hidden overlay map
- moved right-pane ownership to a stable desk that mounts only the currently
  active prompt, preview, preparing, or build-ready state
- introduced a normalized `zustand` draft store for mounted prompt answers,
  with selector-based subscriptions instead of callback-owned parent draft
  state
- replaced the consultant-desk prompt render path with flat
  `QuestionCanvas` / `QuestionBlock` / `SeamlessInput` components while
  keeping `PromptBatchPanel` and `PromptCard` only as compatibility shims for
  older import sites
- kept live sidebar telemetry wired to the active prompt while removing
  keystroke-driven whole-desk draft ownership from `SocraticWorkspace`
- added route-level Playwright verification that the document root remains
  scroll-locked while `.socratic-desk__body` owns the primary scroll region

Verification completed for the delivered slices:

- `npm --prefix planner-web test -- src/components/__tests__/PromptBatchPanel.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm --prefix planner-web run build`
- `cd planner-web && ./node_modules/.bin/playwright test e2e/session-ethereal.spec.ts`

Still open against the full spec:

- the route intentionally behaves as a server-authored dynamic menu rather than
  a continuously mounted 120-input local form tree
- the spec's seeded 15-category / 120-input performance-smoke coverage is not
  yet implemented
- if the continuously mounted 120-input canvas is still desired, it requires a
  new bounded delivery slice; if not, the remaining body text in this spec
  should be narrowed to the selected dynamic-menu contract

### Next Bounded Slice (Ready for Delivery)

The next immediate delivery cycle must execute **Performance Proof & Contract Cleanup**:

1. **Performance Smoke**: Add a seeded consultant-desk performance harness that
   exercises 15 categories and 120 answerable inputs against the normalized
   store so typing latency and selector isolation are actually measured.
2. **Spec Contract Cleanup**: Reconcile the remaining “single mounted 120-input
   form” language in this spec with the selected server-authored dynamic-menu
   behavior if that is now the final product decision.
3. **Legacy Compatibility Review**: Decide whether the `PromptBatchPanel` /
   `PromptCard` compatibility shims should be deleted after the remaining
   import sites migrate, or left in place as stable aliases.

## 1. Problem & Intent

The current Socratic Lobby product line was optimized for focused reading and
progressive drill-down. That direction reduced dashboard noise, but it is the
wrong interaction model if the active payload becomes a fixed high-density
consultant desk with:

- exactly 15 categories
- up to 8 scaffold questions per category
- up to 120 answerable text inputs on one route

At that density, the current drill-down and typography-led cascade patterns
cost too much context. They force users to traverse depth, move between states,
and repeatedly rebuild their macro mental model.

This spec defines a competing lobby paradigm:

- one route
- zero reloads
- zero document-level scrolling
- persistent macro index on the left
- dense active execution canvas on the right
- keyboard-first editing and scanning

The target feel is a premium dark-mode consultant desk: dense, restrained,
ultra-readable, and operational rather than theatrical.

## 2. System Overview

The Socratic Lobby becomes a **Split-Pane Consultant Desk**.

### UX paradigm

- the left pane is a permanent master index of all 15 dimensions
- the right pane is the active detail canvas for the selected dimension
- the user never loses macro position while answering micro questions
- the route consumes `100vw` and `100vh`
- the browser document must not vertically scroll
- only the right canvas may independently scroll

### Interaction contract

- changing categories is instant and local; it must not feel like navigation to
  another page
- answering a question updates both the local field state and the category
  telemetry without perceptible typing lag
- the left index remains dense and fully visible on a standard 1080p display
- the right canvas shows up to 8 questions for the active category at once

## 3. Product Replacement Decision

This direction materially conflicts with current implemented planning:

- [Phase 13 Socratic Focused Question Lobby Reset Spec](/home/thetu/planner/docs/phase-13-socratic-realtime-workspace-deltas-and-warm-prompt-library-spec.md)
  explicitly removed the persistent split-pane lobby
- [Socratic Ethereal Cascade Redesign Spec](/home/thetu/planner/docs/socratic-ethereal-cascade-redesign-spec.md)
  selected a typography-first single-column cascade instead of a permanent
  sidebar map
- [Planner UI Reset Phase 14: Socratic Pro Max Redesign Spec](/home/thetu/planner/docs/planner-ui-reset-phase-14-socratic-pro-max-redesign-spec.md)
  was deferred specifically because a permanent left sidebar conflicted with the
  implemented focused-lobby model

This spec intentionally supersedes that future direction. The existing
Ethereal Cascade remains implemented code, but it is no longer the chosen next
product target.

## 4. User Outcome

After this direction is implemented:

- all 15 dimensions remain visible at once in the left index
- users can answer up to 8 questions in the active dimension without leaving
  macro context
- the page feels denser and more authoritative than the current editorial
  cascade
- the system reads like a planning workstation, not a wizard or page stack
- high input density remains fast enough for sustained keyboard-driven work

## 5. Scope Boundaries

### In Scope

- a permanent left index / right canvas split for the Socratic Lobby route
- zero-reload category switching
- dense telemetry for category completion
- question canvas density rules for a maximum of 8 visible active questions
- dark-mode consultant-desk token system for this route
- normalized frontend state model for 120 active inputs
- React component hierarchy and state strategy for performant synchronized
  editing
- keyboard-first interaction rules for navigation and submission affordances

### Out Of Scope

- backend changes to category synthesis, scaffold generation, or prompt wording
- changes to websocket semantics unless a later implementation pass proves they
  are required
- redesign of other Planner routes
- generic design-system modernization beyond this route
- re-litigating whether the consultant-desk model is the replacement direction

## 6. Macro Architecture

### Root shell

- root container: `width: 100vw; height: 100vh; overflow: hidden;`
- top frame: fixed-height application chrome row
- main frame: horizontal flex container, no wrap

### Left Index: The Map

- fixed width: `clamp(240px, 18vw, 280px)`
- height: `100%` of main frame
- overflow: hidden by default; no vertical scrollbar at the target 1080p
  density contract
- role: persistent master index of dimensions

### Right Canvas: The Desk

- `flex: 1 1 auto`
- min-width: `0`
- height: `100%`
- overflow-x: hidden
- overflow-y: auto
- role: stable reading and writing zone for the selected category

### Non-negotiable layout rules

- no document-level vertical scroll
- no route transition on category selection
- no nested floating card stacks inside the active question canvas
- no modal category drill-down for desktop as the primary interaction

## 7. Density & Layout Mandates

### Left Index rules

- exactly one row per category
- row height: `32px`
- internal vertical padding: `0`
- horizontal padding budget: `10px 12px`
- all 15 rows must fit within a standard 1080p viewport without a sidebar
  scrollbar
- status telemetry sits at the right edge of each row, never below the label

### Right Canvas rules

- canvas title block remains visible at the top of the scrolling region
- question list is vertically segmented by hairline separators only
- the active category may show all 8 questions in one continuous form stack
- answer inputs are borderless or near-borderless, auto-expanding, and visually
  embedded into the page background

### Wireframe baseline

Implementation must preserve this spatial relationship:

```text
+-------------------------------------------------------------------------------------------------+
| ≡ PLANNER   personal calendar > socratic workspace                   [Refine All] [Commit Plan] |
+---------------------------+---------------------------------------------------------------------+
| THREAD INDEX              |  Verify Stakeholders                                                |
|                           |  Current assumption: "Single user (personal use)" (85%)             |
| [●] Stakeholders    [8/8] |  ────────────────────────────────────────────────────────────────── |
| [◐] Core Features   [4/8] |                                                                     |
| [○] Success Crit.   [0/8] |  Q1. Who is the primary end-user?                                   |
| [○] User Flows      [0/8] |  [ Me, solo dev. I need this to manage daily tasks.               ] |
| [○] Data Privacy    [0/8] |                                                                     |
| [○] Monetization    [0/8] |  Q2. Are there any secondary stakeholders?                          |
| [○] Integrations    [0/8] |  [ My spouse might need read-only access eventually.              ] |
| [○] Accessibility   [0/8] |                                                                     |
| [○] Edge Cases      [0/8] |  Q3. Do we need an admin role?                                      |
| [○] Security        [0/8] |  [ No, standard access is sufficient for V1.                      ] |
| [○] UI Theme        [0/8] |                                                                     |
| [○] Offline Mode    [0/8] |  Q4. What external actors exist?                                    |
| [○] Notifications   [0/8] |  [ Calendar APIs (Google), maybe Notion later.                    ] |
| [○] Error Handling  [0/8] |                                                                     |
| [○] Deployment      [0/8] |  Q5. Guest access rules?                                            |
|                           |  [ ______________________________________________________________ ] |
+---------------------------+---------------------------------------------------------------------+
```

## 8. Design Tokens (Tailwind / CSS Guide)

### Color tokens

```css
:root {
  --sl-bg-app: #0b0d10;
  --sl-bg-panel: #111318;
  --sl-bg-canvas: #0f1115;
  --sl-bg-input: #1a1c20;
  --sl-bg-active-row: rgba(255, 255, 255, 0.05);
  --sl-bg-hover-row: rgba(255, 255, 255, 0.03);

  --sl-border-hairline: rgba(255, 255, 255, 0.06);
  --sl-border-strong: rgba(255, 255, 255, 0.12);

  --sl-text-primary: #f5f7fb;
  --sl-text-secondary: #b6beca;
  --sl-text-tertiary: #7f8896;
  --sl-text-disabled: #56606d;

  --sl-accent-blue: #7cb7ff;
  --sl-accent-green: #67c587;
  --sl-accent-amber: #d7a756;
  --sl-accent-red: #df6d6d;
}
```

### Tri-font system

```css
:root {
  --sl-font-sans: "Inter", "Geist", "SF Pro Text", system-ui, sans-serif;
  --sl-font-serif: "Ivar Text", "Canela", "Georgia", serif;
  --sl-font-mono: "JetBrains Mono", "SF Mono", "IBM Plex Mono", monospace;
}
```

### Typography assignments

- app chrome, index rows, question labels, buttons: `--sl-font-sans`
- active category title only: `--sl-font-serif`
- AI assumptions, counters, telemetry, confidence strings: `--sl-font-mono`

### Type scale

- sidebar row label: `12px` to `13px`
- sidebar telemetry: `11px` to `12px` mono
- canvas category title: `28px` to `36px`
- AI context metadata: `11px` to `12px` mono
- question label: `13px`
- answer input text: `14px`

### Spacing constraints

```css
:root {
  --sl-space-1: 4px;
  --sl-space-2: 8px;
  --sl-space-3: 12px;
  --sl-space-4: 16px;
  --sl-space-5: 20px;
  --sl-space-6: 24px;
}
```

Rules:

- no question block may exceed `24px` internal vertical spacing
- index rows do not use vertical padding
- category header to metadata gap: `8px`
- question separator to question label gap: `12px`
- question label to input gap: `8px`

### Tailwind mapping guidance

- app background: `bg-[var(--sl-bg-app)]`
- sidebar: `bg-[var(--sl-bg-panel)]`
- canvas: `bg-[var(--sl-bg-canvas)]`
- hairlines: `border-white/6`
- active row: `bg-white/5`
- mono metadata: `font-mono text-[11px] text-[var(--sl-text-tertiary)]`

## 9. Data Models (TypeScript)

```ts
export type CategoryStatus = "unstarted" | "in_progress" | "complete" | "blocked";
export type QuestionStatus = "unanswered" | "draft" | "answered";

export interface LlmAssumption {
  id: string;
  label: string;
  value: string;
  confidence: number; // 0..1
  evidence?: string[];
  updatedAt: string;
}

export interface SocraticQuestion {
  id: string;
  categoryId: string;
  order: number; // 0..7
  prompt: string;
  placeholder?: string;
  helpText?: string;
  status: QuestionStatus;
  answer: string;
  lastEditedAt?: string;
}

export interface SocraticCategory {
  id: string;
  slug: string;
  title: string;
  shortLabel: string;
  order: number; // 0..14
  status: CategoryStatus;
  answeredCount: number;
  totalQuestions: number; // max 8
  assumptions: LlmAssumption[];
  questions: SocraticQuestion[];
}

export interface SocraticLobbyToolbarState {
  canRefineAll: boolean;
  canCommitPlan: boolean;
  isRefiningAll: boolean;
  isCommittingPlan: boolean;
}

export interface SocraticLobbyViewState {
  activeCategoryId: string;
  orderedCategoryIds: string[];
  categoriesById: Record<string, SocraticCategory>;
  toolbar: SocraticLobbyToolbarState;
}
```

### Validation invariants

- `orderedCategoryIds.length === 15`
- every category has `0 <= totalQuestions <= 8`
- every question belongs to exactly one category
- `answeredCount` must be derivable from `questions`, not manually trusted from
  stale UI state

## 10. Component Hierarchy

```text
<SocraticLobbyShell>
  <SocraticLobbyTopBar>
    <AppBreadcrumbs />
    <ToolbarActions />
  </SocraticLobbyTopBar>

  <SocraticLobbyBody>
    <SidebarMap>
      <SidebarMapHeader />
      <CategoryRow />
      <CategoryRow />
      ...
    </SidebarMap>

    <ConsultantDesk>
      <DeskHeader>
        <ActiveCategoryTitle />
        <LlmContextMeta />
      </DeskHeader>

      <QuestionStack>
        <QuestionBlock>
          <QuestionLabel />
          <SeamlessInput />
        </QuestionBlock>
        ...
      </QuestionStack>
    </ConsultantDesk>
  </SocraticLobbyBody>
</SocraticLobbyShell>
```

### Component responsibilities

- `<SocraticLobbyShell>`: owns viewport lock, top-level layout, and route-level
  event wiring
- `<SidebarMap>`: renders all categories, status rows, and active selection
- `<CategoryRow>`: renders label, progress telemetry, active/complete state,
  keyboard focus, and click target
- `<ConsultantDesk>`: owns independent right-pane scrolling and active category
  content
- `<DeskHeader>`: renders serif title and mono machine context
- `<QuestionBlock>`: one question separator, label, and input
- `<SeamlessInput>`: auto-expanding borderless text surface with IME-safe local
  editing behavior

## 11. Interaction & Accessibility Rules

### Left index

- click selects category immediately
- `ArrowUp` / `ArrowDown` move active row focus
- `Enter` or `Space` opens the focused category in the desk
- active row requires a visible but restrained contrast cue

### Right canvas

- `Tab` order follows question order
- answer inputs auto-expand vertically without layout thrash
- separators remain visible while scrolling
- no floating card shell should intercept keyboard focus

### Accessibility

- color alone must not carry completion state
- sidebar telemetry must have text or ARIA label equivalents
- active row uses `aria-current="true"` or equivalent selection semantics
- each seamless input needs an explicit associated label

## 12. State Management Strategy

Use a normalized **Zustand** store with selector-based subscriptions.

### Required store shape

```ts
interface SocraticLobbyStore {
  activeCategoryId: string;
  orderedCategoryIds: string[];
  categoriesById: Record<string, SocraticCategory>;

  setActiveCategory: (categoryId: string) => void;
  setAnswerDraft: (questionId: string, nextValue: string) => void;
  commitAnswer: (questionId: string) => void;
}
```

### Performance strategy

- normalize state by `categoryId` and `questionId`
- each `<QuestionBlock>` subscribes only to its own answer string and status
- each `<CategoryRow>` subscribes only to its own derived `answeredCount`,
  `totalQuestions`, and `status`
- `<SidebarMap>` does not subscribe to all answer strings at once
- use memoized selectors and shallow comparison for category row telemetry
- avoid one giant controlled form object flowing through React props

### Why this is the chosen approach

For up to 120 active inputs, typing performance will degrade if:

- every keystroke rerenders the entire desk
- every keystroke rerenders all 15 sidebar rows
- the full store is passed down through a broad React Context tree

With normalized Zustand selectors:

- the edited `SeamlessInput` rerenders
- the matching `QuestionBlock` rerenders
- the matching `CategoryRow` rerenders if its completion count changes
- the rest of the view remains stable

### Input synchronization model

- `SeamlessInput` may keep an immediate local value for IME and cursor stability
- keystrokes sync to the store on change
- sidebar completion telemetry derives from current store state, not from
  debounced submit events
- autosave or server sync can be layered later without changing the local store
  contract

## 13. Visual Telemetry Rules

### Category completion

Allowed right-edge telemetry:

- muted monospace `[ 3/8 ]`
- compact radial progress ring
- compact dot or half-ring state plus count

Disallowed:

- verbose strings such as `2 of 8 answered`
- secondary subtitle rows
- badges that increase row height beyond `32px`

### Completion semantics

- `0/8`: muted empty state
- partial: low-contrast active telemetry
- `8/8`: complete state with restrained accent, never bright celebratory color

## 14. Contracts & Touched Surfaces

Expected primary implementation surfaces:

- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/components/SocraticWorkspace.tsx`
- `planner-web/src/components/PromptBatchPanel.tsx`
- `planner-web/src/index.css`
- `planner-web/src/types.ts`

Expected likely new components:

- `planner-web/src/components/socratic/SidebarMap.tsx`
- `planner-web/src/components/socratic/CategoryRow.tsx`
- `planner-web/src/components/socratic/ConsultantDesk.tsx`
- `planner-web/src/components/socratic/QuestionBlock.tsx`
- `planner-web/src/components/socratic/SeamlessInput.tsx`

Potential optional support surfaces:

- `planner-web/src/store/useSocraticLobbyStore.ts`
- route-level tests and interaction tests for the dense lobby

## 15. Acceptance Criteria

1. The Socratic Lobby occupies `100vw` and `100vh` with no document-level
   vertical scrolling.
2. All 15 categories fit in the left index on a standard 1080p viewport
   without sidebar scrolling.
3. Category rows use a strict `32px` row height and remain readable at `12px`
   to `13px`.
4. Clicking a category updates the right desk instantly without route
   navigation or page reload.
5. The right desk can display up to 8 active questions for the selected
   category with only hairline separators and seamless inputs.
6. The active category title uses the reserved serif face; LLM assumptions and
   confidence values use mono styling.
7. Typing into one answer does not cause perceptible lag across the rest of the
   120-input interface.
8. Sidebar telemetry updates live from answer changes without full-layout
   rerender churn.
9. The visual language remains premium dark-mode and dense rather than bulky,
   card-heavy, or wizard-like.

## 16. Verification Plan

### Automated

- unit test normalized state selectors so editing one question updates only the
  expected category telemetry
- component tests for:
  - `CategoryRow` active state and progress rendering
  - `ConsultantDesk` category switch behavior
  - `SeamlessInput` auto-expansion and label association
- integration tests proving the route does not document-scroll and the right
  desk owns the primary scroll container
- interaction tests proving sidebar progress updates while typing
- performance smoke test on a seeded 15-category / 120-input fixture

### Manual

- verify at 1920x1080 that all 15 rows fit without sidebar scrolling
- verify the right desk scrolls independently while the app shell stays locked
- verify rapid typing in a late question does not visibly lag the sidebar
- verify the dense layout still reads clearly at laptop widths
- verify keyboard-only traversal across sidebar selection and question editing

## 17. Rollback / Fallback

- if the consultant-desk direction is rejected, retain the currently
  implemented focused-lobby / Ethereal Cascade model and close this spec as
  superseded
- if the always-visible left index proves too dense for smaller widths, desktop
  may keep the split-pane while smaller widths switch to a drawer or overlay
  index, but only after a follow-on bounded spec
- if live telemetry updates create typing lag, prioritize selector granularity
  and derived-state optimization before broadening the architecture

## 18. Open Questions

- Is the 15-category / 8-question payload a hard backend contract or a current
  product target that may vary by session?
- Inactive categories may remain unmounted while the lobby behaves as a dynamic
  server-authored menu. A single mounted 120-input tree is not required for the
  current delivery path.
- Should the left index remain permanently visible only above a specific desktop
  breakpoint?
- This consultant-desk model intentionally replaces the Ethereal Cascade as the
  selected future direction.

## 19. Readiness Judgment

This spec is now **in progress** and **ready for the next delivery slice**.

Readiness basis:

- the product decision is closed: this consultant-desk model is the intended
  replacement direction
- the first two bounded delivery slices successfully established the split-pane
  macro layout, normalized draft state, flat question components, and route-level
  scroll-ownership verification
- the remaining work is now narrow and well-bounded: seeded performance proof
  plus contract cleanup around the intended dynamic-menu behavior

The next valid move is to execute the "Performance Proof & Contract Cleanup"
slice so the consultant-desk spec either closes honestly around the dynamic
menu now implemented, or clearly scopes the additional work needed for a true
continuously mounted 120-input canvas.
