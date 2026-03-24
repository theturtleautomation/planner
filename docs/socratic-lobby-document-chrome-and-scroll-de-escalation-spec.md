# Socratic Lobby Document Chrome And Scroll De-escalation Spec

**Status:** implemented precursor  
**Date:** 2026-03-24  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Socratic Lobby Live Virtualized Document Spec](/home/thetu/planner/docs/socratic-lobby-live-virtualized-document-spec.md), [Socratic Question Canvas Alignment And Visual Refinement Spec](/home/thetu/planner/docs/socratic-question-canvas-alignment-and-visual-refinement-spec.md), [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md)

## Problem & Intent

> Planning note (2026-03-24): this slice remains the record of bounded cleanup
> work that landed on the continuous-document route. It no longer defines an
> active future-state slice after the pivot to
> [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md).

The live virtualized Socratic lobby is structurally correct, but the current
document chrome is still over-signaling the active question surface in ways
that hurt readability:

- section titles are still too large for a dense working document
- the repeated `Thread` / `Live question` eyebrow labels add noise without
  helping orientation
- the active live-question surface visually bubbles above the rest of the
  document while scrolling, which makes it feel like a floating overlay instead
  of one section in a continuous consultant paper

This slice is a bounded cleanup of hierarchy, labeling, and scroll behavior in
the implemented live document. It does not reopen the underlying split-pane,
virtualized-document, or dynamic-generation architecture.

It also hardens one interaction expectation that is currently under-specified:

- browsing already-known categories and questions must feel local and immediate
  rather than server-latent

## User Outcome

After this slice:

- category section headings read like compact document subheads, not hero lines
- the document stops repeating `Thread` as generic filler text
- the active live question remains clearly identifiable without being visually
  louder than the rest of the document
- scrolling the document no longer makes the active question appear to float
  over or detach from neighboring sections
- any sticky affordance that remains is visibly bounded to the answer actions,
  not the whole active section
- clicking a known category or question region feels point-and-click fast,
  because local document navigation does not wait on a server round trip just
  to let the user look around

## Current-State Evidence

- section chrome is still explicitly rendering `Thread` / `Live question` /
  `Preparing` in
  [planner-web/src/components/SocraticDocumentSection.tsx](/home/thetu/planner/planner-web/src/components/SocraticDocumentSection.tsx)
- section titles are still using a relatively large editorial clamp in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- the question canvas footer remains sticky in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- live user feedback on 2026-03-24 called out three concrete failures:
  - headings are too large
  - `thread` adds no value
  - the live question block floats above the rest of the items during scroll

## In Scope

- reduce the visual scale and emphasis of category section titles
- remove generic `Thread` labeling from the document chrome
- tighten or replace `Live question` labeling so it only appears when it adds
  real value and does not duplicate the section title
- de-escalate the live-question visual treatment so it stays in-flow while
  scrolling
- enforce a local-first browsing contract for already-known document sections
  so category/question inspection feels immediate
- constrain sticky behavior so only the minimum action affordance remains
  sticky, if any
- adjust related microcopy only where needed to support the new hierarchy
- update the targeted frontend verification surface for this behavior

## Out Of Scope

- changing Socratic question routing or generation timing
- changing the left-index IA or the continuous-document architecture
- redesigning unrelated session route surfaces
- broad token or design-system expansion outside this Socratic document slice

The slice may refine when server focus changes are dispatched, but it does not
reopen backend question-generation strategy.

## Product Contract

### 1. Section heading scale

- `.socratic-document-section__title` must be reduced to a compact document
  subhead scale
- the title must remain editorial, but it must not dominate the desk
- the active section may receive a subtle color or weight distinction, but not
  a second-level hero treatment

### 2. Redundant chrome removal

- generic `Thread` labels must be removed from section chrome
- the document must not repeat both a state label and the same category title in
  a way that adds no new information
- if a live-state indicator remains, it must be brief, secondary, and only
  present when materially useful

### 3. In-flow live question behavior

- the live question section must remain visually part of the continuous
  document while scrolling
- the active section must not appear to hover above retained or preview
  sections
- section headers and bodies must remain in normal reading flow

### 4. Local-fast browsing contract

- clicking a known category row in the left index must update the user-visible
  desk location immediately on the client
- scrolling or jumping to already-known sections must not wait on a websocket
  or server acknowledgement before the user can inspect the section
- reading previously known question content must remain local-first even when a
  separate authoritative server-focus message is still in flight
- only genuinely unknown content, such as a newly requested live prompt that
  has not been generated yet, may present a waiting state
- the UI must not label a locally available section as `Preparing` merely
  because the authoritative server focus has not caught up yet

### 5. Sticky containment

- sticky behavior must not apply to the full live-question block
- if the footer action row stays sticky, it must be bounded to the active
  answer region and must not visually read as a floating card
- sticky gradients, blur, or z-index treatment must be reduced if they create a
  detached overlay effect

## Design Constraints

- preserve the dense consultant-desk aesthetic rather than returning to sparse
  card UI
- prefer subtraction over adding new chrome
- keep the section distinction semantic and typographic before using borders,
  glows, or decorative surfaces
- preserve accessible contrast and focus visibility
- preserve the already-delivered local-preview behavior; any server sync that
  remains must stay behind the immediate client response

## Touched Surfaces

Expected primary files:

- [planner-web/src/components/SocraticDocumentSection.tsx](/home/thetu/planner/planner-web/src/components/SocraticDocumentSection.tsx)
- [planner-web/src/components/QuestionCanvas.tsx](/home/thetu/planner/planner-web/src/components/QuestionCanvas.tsx)
- [planner-web/src/components/VirtualizedCategoryDocument.tsx](/home/thetu/planner/planner-web/src/components/VirtualizedCategoryDocument.tsx)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

Expected supporting tests:

- [planner-web/src/components/__tests__/SocraticWorkspace.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/SocraticWorkspace.test.tsx)
- [planner-web/src/components/__tests__/QuestionCanvas.test.tsx](/home/thetu/planner/planner-web/src/components/__tests__/QuestionCanvas.test.tsx)
- new targeted component or Playwright coverage only if needed to prove the
  floating/scroll regression is closed

## Acceptance Criteria

1. Default category sections no longer render the literal label `Thread`.
2. The live-question section no longer duplicates the category title with a
   heavy `Live question` chrome treatment.
3. Category section headings are visibly smaller and calmer than the current
   implementation.
4. Clicking any already-known category or question area repositions or reveals
   the relevant desk content immediately, without waiting for server round-trip
   latency.
5. Scrolling the right desk keeps the live question visually in-flow with the
   rest of the document; it does not read as a floating overlay.
6. Any sticky action row that remains is visually restrained and bounded to the
   answer controls.
7. The document remains dense, readable, and keyboard-safe after the cleanup.

## Verification Plan

### Automated

- add or update targeted frontend tests for the section labels and any DOM/CSS
  contract changes around sticky action containment
- add or update targeted tests proving that index clicks and known-section
  jumps update the visible desk immediately without requiring server focus to
  complete first
- rerun:
  - `npm --prefix planner-web test -- src/components/__tests__/QuestionCanvas.test.tsx src/components/__tests__/SocraticWorkspace.test.tsx`
  - `npm --prefix planner-web run build`

### Manual

- verify the live document at desktop width with an active prompt near the top
  and retained sections below it
- verify that clicking a known category row moves the desk immediately even if
  the authoritative live-question state changes later
- scroll through the desk and confirm the live question no longer floats above
  the rest of the document
- verify the section heading scale feels subordinate to the overall desk
  hierarchy
- verify the route no longer shows generic `Thread` chrome
- verify short-viewport behavior still keeps answer actions reachable

## Rollback & Fallback

- if full sticky removal makes short-viewport action access worse, retain a
  minimal sticky footer but reduce the visual treatment until it no longer
  reads as a floating block
- if removing live-state chrome harms orientation, replace it with a smaller
  inline status treatment rather than restoring the old eyebrow hierarchy

## Open Questions

- none blocking this slice; the user feedback and current code surface are
  specific enough to bound implementation

## Readiness Judgment

This spec is ready for implementation.

The problems are concrete, reproducible on the live route, and tightly bounded
to existing Socratic document surfaces. No product-architecture decision is
pending, and this slice does not require backend contract changes.

## Implementation Sync

Implemented on `planner-web` on 2026-03-24.

Delivered in this slice:

- removed generic `Thread` chrome from category sections and replaced the
  heavy live-question eyebrow treatment with a quieter inline state treatment
  in
  [planner-web/src/components/SocraticDocumentSection.tsx](/home/thetu/planner/planner-web/src/components/SocraticDocumentSection.tsx)
- changed preparing-state gating so sections with locally known preview or
  retained content no longer collapse into a false `Preparing` state while
  authoritative server focus catches up
- de-escalated the top desk chrome from `Live question` / `Active thread` into
  a calmer workspace-level label and suppressed desk-level `Preparing` when
  the currently displayed section is already known locally in
  [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- removed sticky section headers, reduced section-title scale, and stripped the
  floating footer treatment so the active question stays visually in-flow while
  scrolling in
  [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)
- kept the prompt footer action row intact as a normal bounded action region in
  [planner-web/src/components/QuestionCanvas.tsx](/home/thetu/planner/planner-web/src/components/QuestionCanvas.tsx)

Automated verification completed:

- `npm --prefix planner-web test -- src/components/__tests__/SocraticWorkspace.test.tsx src/components/__tests__/QuestionCanvas.test.tsx`
- `npm --prefix planner-web run build`

Not yet completed in this slice:

- the manual browser checks from this spec were not rerun in this pass, so the
  remaining open work is manual confirmation that the live question no longer
  reads as a floating overlay at real desktop and short-viewport sizes
