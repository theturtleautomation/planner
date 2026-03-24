# Socratic Lobby Master-Detail Local Workspace Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Related Planning:** [Socratic Lobby Live Virtualized Document Spec](/home/thetu/planner/docs/socratic-lobby-live-virtualized-document-spec.md), [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md), [Socratic Lobby First-Reveal Preload Gate Spec](/home/thetu/planner/docs/socratic-lobby-first-reveal-preload-gate-spec.md), [Socratic Lobby Document Chrome And Scroll De-escalation Spec](/home/thetu/planner/docs/socratic-lobby-document-chrome-and-scroll-de-escalation-spec.md), [Phase 12 Socratic Live Question Workspace Spec](/home/thetu/planner/docs/phase-12-socratic-live-question-workspace-spec.md)

> Planning note (2026-03-24): this spec supersedes the live virtualized
> document model as the selected future product target for the Socratic Lobby.
> The currently implemented split-pane shell, Jotai document graph, preload
> gate, and local-fast fixes remain useful migration primitives, but the final
> target is no longer one continuously rendered document. This document now
> remains primarily as the record of the implemented React baseline route. The
> selected greenfield future-state now sits under
> [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)
> and
> [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md).

## Implementation Sync

The bounded master-detail redesign landed on `planner-web` on 2026-03-24.

Delivered across the completed slices:

- replaced continuous right-desk document rendering with active-thread-only
  workspace rendering in `SocraticWorkspace`
- kept the normalized Socratic graph as the source of truth for known thread
  state while mounting only the selected thread's workspace content
- changed left-index interaction to local client-side thread selection instead
  of document jump behavior
- added intentional right-pane scroll reset on thread change
- updated question text entry to use local component state with debounce/blur
  synchronization and unmount flush protection
- preserved immediate active-row telemetry updates on the empty/non-empty
  answer threshold while keeping full text syncing buffered
- kept manual thread selection and active editing ownership from being
  overwritten by later server-focus changes
- removed the generic workspace chrome and normalized the active-thread desk to
  a compact sans-serif operational hierarchy
- tightened spacing and question-surface treatment so the route reads like a
  dense master-detail productivity tool instead of a continuous document
- reduced empty-thread boilerplate to a minimal `Awaiting questions...` state
- updated route-level Playwright coverage to prove the active-thread workspace
  model instead of the superseded continuous-document desk

Verification completed for the delivered work:

- `npm --prefix planner-web test -- src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
- `npm --prefix planner-web run build`
- `cd planner-web && ./node_modules/.bin/playwright test e2e/session-ethereal.spec.ts`

Status note:

- no open implementation delta remains against this spec's bounded contract
- future work on the Socratic lobby should be treated as optional follow-on
  refinement rather than unfinished master-detail migration work

## 1. Executive Judgment

The current session page requires a fundamental architectural redesign.

The implemented continuous-document route proved that the product can hold
dynamic categories and locally retained question state, but it also confirmed
the wrong primary abstraction: an interactive planning workspace should not
behave like a continuous markdown preview.

The selected replacement direction is now explicit:

- keep dynamic server-authored categories and questions
- keep as much known data preloaded locally as possible
- stop rendering the entire hierarchy as one long desk document
- pivot to a strict master-detail workspace model

This spec defines that replacement contract.

## 2. Problem & Structural Diagnosis

The current route flattens a hierarchical, multi-threaded planning structure
into one continuous scrolling feed. That creates the wrong user experience and
the wrong rendering shape:

- it destroys spatial memory because the user must scroll to relocate work
- it blurs navigation and editing into one monolithic DOM tree
- it overuses large display typography and repeated boilerplate states
- it turns passive reading patterns into the dominant UI metaphor for an
  operational editing tool

The correct diagnosis is:

- the thread hierarchy belongs in the persistent index
- the active editing surface belongs in an isolated right-side workspace
- browsing must feel local and synchronous for already-known content
- typing must stay isolated from global navigation and non-active content

## 3. User Outcome

After this redesign is delivered:

- the page behaves like a dense desktop app, not a long generated article
- the thread index remains pinned and continuously readable
- the workspace renders only the currently active thread
- switching threads feels immediate and local for already-known content
- known questions and drafts stay preloaded in client state even when their
  thread is not mounted
- question blocks are visually distinct, dense, and easy to scan
- empty states become quiet and rare instead of dominating the page
- typing remains fluid regardless of overall session size

## 4. Product Decision

### Selected future-state model

The Socratic Lobby must converge to a **Master-Detail Local Workspace**:

- the left side owns hierarchy, telemetry, and thread selection
- the right side owns only the active thread's editing context
- the document body does not contain inactive sibling threads
- category switching is a local content swap, not anchor scrolling through one
  giant document

### Explicitly rejected end-states

- one continuous rendered Socratic document as the primary steady-state model
- anchor-scroll navigation as the main way to move between threads
- oversized editorial or blog-post typography
- repeated paragraph-style waiting copy per empty thread
- one giant mounted React tree that keeps all inactive thread UI in the DOM

## 5. Scope Boundaries

### In Scope

- replacing the continuous right-desk document with an active-thread workspace
- preserving dynamic category/question generation while keeping known data
  locally preloaded
- locking the route to a true `100vh` desktop-app shell with independent pane
  scroll
- dense thread-index navigation and progress telemetry
- dense workspace question rendering with explicit interactive surfaces
- local-first navigation behavior for already-known content
- input-state isolation so typing does not fan out through the full page
- bounded preload behavior that improves the first view without forcing the
  continuous-document architecture to remain the final target

### Out Of Scope

- changing backend question-authoring semantics beyond what is required to keep
  known thread content preloaded and browsable locally
- redesigning unrelated Planner routes
- introducing a spreadsheet or grid-first interaction model
- reopening the selected dark operational palette for this route

## 6. Macro Architecture

### Shell contract

- route shell: `width: 100vw; height: 100vh; overflow: hidden`
- top chrome remains fixed
- main content is a stable horizontal multi-pane desktop layout
- document/body scroll must remain locked

### Pane model

- left project/session chrome remains outside the Socratic work area
- thread index is a dedicated scrollable pane
- workspace is a separate dedicated scrollable pane
- scrolling one pane must not move the other
- there is no global page-level rubber-banding on desktop

### Navigation model

- clicking a thread row updates `activeThreadId` immediately on the client
- the right workspace swaps to the selected thread instantly if its content is
  already known locally
- server focus synchronization may continue in the background, but it must not
  block local inspection of known content
- only truly unknown content may show a waiting state
- local thread switching must never discard unsaved in-progress answer text

## 7. Rendering Model

### Left thread index

The thread index owns:

- thread hierarchy
- selection state
- progress fractions
- pending/loading indicators
- keyboard traversal

The thread index does **not** repeat its structure as large headings in the
workspace.

### Right workspace

The right workspace renders only:

- the active thread title
- compact machine/context metadata
- the active thread's currently known questions
- the active thread's answer inputs or truthful empty state

All inactive threads must be unmounted from the workspace DOM.

### Dynamic generation contract

Dynamic generation remains a key product requirement:

- categories may continue to appear over time
- questions may continue to appear or update over time
- all known categories/questions must stay normalized in client state
- only the active thread's content is mounted in the workspace

This means:

- **preloaded in state**: yes
- **rendered in the DOM all at once**: no

## 8. Design Direction

### Typography

- drop serif headings for this route
- use a strict sans-serif operational scale throughout
- base text should land in the `13px` to `14px` range
- active thread title must cap at `16px` to `18px`, semibold
- hierarchy must come from alignment, weight, and contrast rather than large
  type jumps

### Spacing

- use a rigid `4px` / `8px` spacing grid
- avoid vertical gaps larger than `16px` inside the active workspace
- the page should feel dense, calm, and local-fast rather than theatrical

### Surfaces

- question blocks must be distinct interactive surfaces
- subtle border or low-contrast surface separation is required
- actionable inputs must be visually separable from passive metadata
- repeated prose empty states are not allowed
- if the active thread has no known question yet, use one concise muted empty
  state such as `Awaiting questions...`

### Palette

- preserve the restrained dark palette already in use
- shell background should remain darker than the thread index
- active question surfaces may use a slightly elevated dark surface, but
  without glass, glow, or oversized card theatrics

## 9. State & Performance Contract

### Global state

The route must keep a normalized model of known Socratic state, including:

- `activeThreadId`
- `threadsById`
- `threadOrder`
- `questionsById`
- `questionIdsByThread`
- `draftsByQuestionId`
- `threadTelemetry`
- `generationStateByThread`

### Local typing isolation

Typing must not drive full-workspace rerenders.

Required contract:

- the workspace subscribes only to the active thread
- question input components isolate keystrokes with local state or equivalent
  fine-grained selector boundaries
- syncing to the shared store/backend happens on blur, submit, or debounce
- inactive threads do not rerender during active typing
- unmounting the active thread must synchronously flush any dirty local input
  state to the shared draft store before the active workspace subtree is
  removed

### Local-fast interaction

The route must feel immediate for already-known content:

- switching to a known thread is point-and-click fast
- no spinner is allowed when the selected thread is already locally known
- sidebar progress may update optimistically when a local draft or answer is
  submitted

### Subscription granularity

The normalized state contract must not be defeated by coarse subscriptions.

Required contract:

- the parent workspace subscribes only to the active thread id and the ordered
  question ids for that thread
- the parent workspace must not subscribe to the entire
  `draftsByQuestionId`, `questionsById`, or equivalent large dictionaries
- each `QuestionBlock` or equivalent child component subscribes only to its own
  question record and draft record
- typing in one answer must not rerender sibling question blocks unless a
  sibling's own subscribed data changed

### Scroll ownership and reset behavior

Changing `activeThreadId` must not bleed scroll position between threads.

Required contract:

- the right workspace scroll container resets to the top when the user switches
  to a different thread unless explicit per-thread scroll restoration has been
  implemented
- if per-thread scroll restoration is later added, it must be keyed by thread
  id and restored intentionally rather than through incidental DOM reuse

### Server focus protection

The user’s local workspace ownership takes priority over background server
focus changes.

Required contract:

- incoming server-focus updates must not yank the workspace away from a thread
  the user manually selected
- while an input is actively focused or a thread has been manually selected,
  competing server-focus changes must be ignored or queued until they can be
  applied safely
- background server focus may update non-destructive telemetry, but it must not
  steal the active editing surface mid-keystroke

## 10. Touched Surfaces

Expected primary surfaces:

- [planner-web/src/pages/SessionPage.tsx](/home/thetu/planner/planner-web/src/pages/SessionPage.tsx)
- [planner-web/src/components/SocraticWorkspace.tsx](/home/thetu/planner/planner-web/src/components/SocraticWorkspace.tsx)
- [planner-web/src/components/QuestionCanvas.tsx](/home/thetu/planner/planner-web/src/components/QuestionCanvas.tsx)
- [planner-web/src/components/QuestionBlock.tsx](/home/thetu/planner/planner-web/src/components/QuestionBlock.tsx)
- [planner-web/src/components/SeamlessInput.tsx](/home/thetu/planner/planner-web/src/components/SeamlessInput.tsx)
- [planner-web/src/stores/socraticDocumentStore.ts](/home/thetu/planner/planner-web/src/stores/socraticDocumentStore.ts)
- [planner-web/src/index.css](/home/thetu/planner/planner-web/src/index.css)

Potential supporting backend surfaces, only if required to preserve local-fast
truth:

- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)

## 11. Acceptance Criteria

1. The Socratic workspace no longer renders inactive sibling threads in the
   right workspace.
2. Clicking a known thread row swaps the right workspace immediately without
   anchor scrolling through a continuous document.
3. The thread hierarchy lives in the left index, not as repeated large section
   headings in the workspace.
4. The route remains locked to `100vh` with independent thread-index and
   workspace scroll containers.
5. The active thread title is capped to a compact operational scale and uses
   sans-serif typography.
6. Question inputs are visually distinct from passive metadata and empty-state
   prose is reduced to a minimal truthful message.
7. Typing into an active question does not cause visible lag or obvious
   rerender churn across inactive content.
8. Already-known threads remain locally browsable without waiting for a server
   focus acknowledgment.
9. Progress telemetry in the left index remains truthful and does not invent
   loaded content for shell-only threads.
10. Switching threads while an answer is dirty does not lose the unsynced text.
11. Switching from one thread to another does not inherit the prior thread's
    scroll position accidentally.
12. Manual thread selection is not overridden mid-edit by background
    server-focus updates.

## 12. Verification Plan

### Automated

- add or update frontend tests proving the workspace mounts only the active
  thread
- add or update frontend tests proving thread clicks immediately swap known
  local content without waiting on server focus
- add or update tests proving shell-only threads do not present fake loaded
  telemetry
- add or update tests proving question input updates do not fan out through
  non-active thread rendering contracts
- add or update tests proving dirty local input state is flushed when the user
  switches threads before blur
- add or update tests proving the workspace scroll container resets or restores
  intentionally on thread change instead of bleeding prior scroll position
- add or update tests proving manual thread selection is not overridden by a
  competing server-focus update while the user is editing
- rerun at minimum:
  - `npm --prefix planner-web test -- src/components/__tests__/SocraticWorkspace.test.tsx src/pages/__tests__/SessionPage.test.tsx src/hooks/__tests__/useSocraticWebSocket.test.tsx`
  - `npm --prefix planner-web run build`

### Manual

- verify desktop behavior with enough generated threads to ensure the index and
  workspace scroll independently
- verify selecting multiple known threads feels immediate and local
- verify the workspace no longer resembles one long generated article
- verify the active question area remains dense and readable at short viewport
  heights
- verify the index remains readable and truthful when some threads are shells
  and one thread is currently answerable
- verify switching threads mid-typing preserves the just-entered draft text
- verify switching threads does not land the next thread at an inherited scroll
  offset
- verify background server updates do not steal the active thread while the
  user is typing

## 13. Rollback & Migration Notes

- the existing continuous-document implementation remains a migration baseline
  and can continue to ship while this redesign lands in bounded slices
- the normalized Jotai graph, preload gate, and local-fast fixes should be
  reused rather than deleted blindly
- if a bounded slice cannot yet remove continuous-document rendering
  completely, it must still move the route toward active-thread-only workspace
  rendering instead of deepening the document model further

## 14. Open Questions

- none blocking spec readiness; the product direction is explicit enough to
  start bounded implementation

## 15. Implementation Guardrails

These constraints are mandatory during delivery and are not optional polish:

- dirty local input must flush on unmount or thread switch before the active
  workspace subtree is discarded
- workspace scroll position must reset or restore intentionally per thread;
  incidental DOM reuse is not acceptable
- parent components must subscribe narrowly enough that keystrokes do not
  rerender the full active thread tree
- server focus must never steal the active editing surface from the user while
  they are manually navigating or typing

## 16. Readiness Judgment

This spec is ready for implementation.

The structural diagnosis, target interaction model, and design constraints are
all explicit. The new direction intentionally replaces the current
continuous-document future-state contract, and the repo already contains enough
migration primitives to begin bounded delivery without reopening discovery. The
implementation guardrails above close the main execution trapdoors without
changing the bounded product contract.
