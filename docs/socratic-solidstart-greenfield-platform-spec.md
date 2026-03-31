# Socratic SolidStart Greenfield Platform Spec

**Status:** active  
**Date:** 2026-03-24  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** official Solid and SolidStart docs reviewed on 2026-03-24, plus prior local planning artifacts  
**Related Planning:** [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Socratic Lobby Local-First Browser Architecture Review](/home/thetu/planner/docs/socratic-lobby-local-first-browser-architecture-review.md), [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md), [Socratic Hybrid Question Routing And Latency Spec](/home/thetu/planner/docs/socratic-hybrid-question-routing-and-latency-spec.md)

> Planning note (2026-03-24): this route-level spec now sits under the broader
> [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md).
> The current React implementation remains the live product baseline, but the
> route-specific future-state platform is no longer treated as an isolated
> Socratic-only fork. The broader migration shape is now closed: full frontend
> replacement, route cleanup allowed, direct replacement deployment, and no
> intended split-framework end-state.
>
> Status sync note (2026-03-30): this remains an active route-level future-state
> planning artifact, not a bounded delivery-ready implementation slice.

## 1. Executive Judgment

If this surface were being designed greenfield for the user's actual goals, the
selected platform would not be React. It would be **SolidStart**.

The deciding factors are:

- the product is a dense, long-lived, locally reactive workspace
- the page must feel native under constant background mutation
- the user wants a fully banked first reveal plus dynamic later updates
- the main performance risk is not initial HTML delivery alone; it is
  fine-grained interactivity and background graph churn over time

Official Solid docs explicitly position Solid around fine-grained reactivity and
targeted updates rather than broader component re-execution. That is the best
fit for this product shape.

This spec therefore defines the greenfield SolidStart future-state for the
Socratic lobby.

## 2. User Outcome

After this greenfield direction is fully realized:

- the Socratic lobby loads as a fully banked first-view workspace for the
  currently derivable prompt set
- visible answerable threads always feel local and instantly navigable
- typing remains fluid while websocket-driven updates continue in the
  background
- local browsing of banked work feels immediate and native
- route structure and page shape become simpler and clearer rather than merely
  porting React-era complexity
- the page behaves like a premium desktop planning tool rather than a chat
  wrapper, route wizard, or markdown feed
- the platform is chosen to match that behavior by default, not by layering
  optimizations on top of a less natural rendering model

## 3. Why SolidStart Is Selected

### Solid's runtime model fits the product directly

Official Solid docs state that Solid uses fine-grained reactivity and performs
highly targeted updates instead of the broader component work common in
replay-style frameworks.

That directly maps to this route's hardest requirements:

- a keystroke in one answer should not disturb unrelated active UI
- one thread updating should not force broad page reconciliation
- dynamic prompt-bank insertions should merge into state without shaking the
  active workspace

### Solid stores and resources match the graph problem

Official Solid docs describe stores as maintaining fine-grained reactivity for
complex nested state, and resources as embedding async request state into the
reactive system.

That makes Solid a strong fit for:

- prompt-bank by thread
- question graph by id
- local draft state
- websocket-driven category or prompt inserts
- derived telemetry without broad recomputation

### SolidStart is the correct meta-framework layer

Official SolidStart docs describe it as the framework layer for routing,
building, deployment presets, and server integration on top of Solid.

That gives the greenfield route a coherent platform for:

- app routing
- SSR/streaming where helpful
- session-aware initial prompt-bank delivery
- server functions and route structure

## 4. Comparison Judgment

### React 19

React 19 remains viable and the current repo proves it can be pushed far.

But it still requires more explicit discipline to achieve what Solid aligns
with naturally:

- compiler setup
- external-store subscription hygiene
- careful rerender isolation
- more architectural work to preserve native feel under graph churn

React is the strongest incremental path. It is not the strongest greenfield
path for this specific product.

### Angular

Angular is the strongest enterprise-platform runner-up because its current
signals and zoneless direction materially improve its reactive fit.

But for this route, Solid is still the cleaner choice:

- less platform overhead
- more direct fine-grained mental model
- better fit for a dense local-first editing surface rather than a broad
  batteries-included app framework

### Qwik

Qwik is strongest on startup/hydration concerns through resumability.

That is valuable, but it is not the dominant problem for this route. This
product's harder problem is sustained interactive responsiveness after the app
is already live, banked, and editing.

### Vue and other signal-style alternatives

They remain credible platforms, but none beat Solid on direct fit for a
fine-grained, graph-heavy, native-feeling workspace.

## 5. Greenfield Product Decision

The selected greenfield platform is:

- **framework**: SolidStart
- **reactivity model**: Solid signals, stores, memos, and resources
- **workspace model**: master-detail local workspace
- **startup contract**: initial prompt bank before first reveal
- **dynamic update model**: websocket-driven graph mutation merged into local
  state
- **rendering discipline**: bounded DOM, no giant mounted document

The active React implementation is therefore demoted from "future-state
platform" to "live baseline and requirements source."

Additional locked route-level requirements:

- **local-speed** is mandatory for banked content
- **visual clarity** outranks rote parity with the current route structure
- route simplification is allowed if it produces a clearer, lower-noise
  workspace

## 6. Scope Boundaries

### In Scope

- defining the greenfield Socratic lobby future-state on SolidStart
- carrying forward the selected prompt-bank, local-first, and master-detail
  product requirements into a Solid-native platform contract
- identifying the platform surfaces that must exist to support that product
  model
- documenting what parts of the current React route become migration inputs
  rather than final-state architecture

### Out Of Scope

- immediate implementation inside the existing React repo
- route-by-route rewrite planning for the entire Planner application beyond what
  this Socratic child spec needs
- speculative backend rewrites unrelated to the Socratic lobby contract
- pretending that a complete migration plan is already implementation-ready

## 7. Greenfield Architecture Contract

### 7.1 Route shell

The SolidStart Socratic route must preserve the selected product shell:

- fixed-height desktop workspace
- pinned thread index
- isolated active-thread workspace
- no document-level scroll
- instant local thread switching among banked threads
- a clearer and simpler route/workspace model than the current React-era
  accumulation when that improves usability

### 7.2 Initial prompt bank

The SolidStart route must still depend on the product truth already captured in
the prompt-bank spec:

- first reveal waits for a real prompt bank
- every visible answerable thread at first reveal has a real prompt behind it
- later dynamic prompt-bank additions may appear incrementally

SolidStart does not change that requirement. It becomes the platform for
implementing it more naturally.

### 7.3 Local graph state

The Solid route must model the Socratic lobby as a local reactive graph with at
least:

- `activeThreadId`
- `threadsById`
- `threadOrder`
- `promptBankByThreadId`
- `questionsById`
- `questionIdsByThread`
- `draftsByQuestionId`
- `queuedThreadIds`
- `workspaceSyncState`

This graph should be owned by Solid-native reactive primitives rather than by a
React external-store adaptation.

### 7.4 Input isolation

Typing must remain local and immediate.

The Solid implementation must make sure:

- editing one answer does not disturb unrelated question blocks
- dirty local answer state is flushed safely on thread switch and unmount
- server focus or incoming updates never steal the cursor from the user
- banked-thread switching never waits on a round trip just to inspect known
  content

### 7.5 Dynamic updates

The route must support:

- websocket-driven insertion of new threads
- websocket-driven replacement of banked prompts
- truthful queued-to-banked transitions
- stable active workspace ownership while updates occur
- no fake preview-shell rows that imply local answerability without a banked
  prompt

## 8. Styling & UX Direction

The Solid greenfield route carries forward the selected UX direction:

- dense sans-serif operational typography
- no giant editorial headings
- no theatrical waiting prose
- crisp, bordered, legible interactive surfaces
- local-fast thread browsing
- a premium desktop-workspace feel

This spec does not reopen the earlier rejected continuous-document feed model.
It also does not require preserving the current route tree if a simpler route
shape improves clarity.

## 9. Touched Surfaces In A Future Greenfield Build

Expected primary surfaces in a SolidStart implementation:

- SolidStart route and layout definitions for the Socratic workspace
- Solid-native graph store for prompt bank, threads, and drafts
- websocket/session integration layer for prompt-bank hydration and live updates
- active-thread workspace components
- thread-index and telemetry components
- test harness replacement for route, state, and browser verification

This is intentionally framework-native. The goal is not to port React patterns
mechanically; it is to implement the product model in the platform that fits it
best.

## 10. Acceptance Criteria

This greenfield direction is only fulfilled if:

1. the framework/platform choice for the future-state Socratic lobby is
   explicitly SolidStart rather than React
2. the product requirements from the prompt-bank spec remain preserved
3. the route uses Solid-native fine-grained state rather than recreating a
   React-like rerender architecture in Solid
4. banked thread switching is instant and local
5. typing remains isolated under live background updates
6. the page preserves the dense master-detail desktop model instead of falling
   back to a document feed
7. visual clarity improves through simplification rather than cargo-cult route
   parity

## 11. Verification Expectations

Before any future implementation can be called complete, verification must
cover:

- banked first reveal behavior
- thread-switch immediacy
- dirty-input retention on thread switch
- background insertion stability
- active workspace ownership under websocket churn
- browser-level proof of native-feeling interaction on desktop and short
  viewports

## 12. Rollback / Fallback

If this direction is not selected for buildout, the fallback is not to pretend
React is equally ideal.

The truthful fallback is:

- keep the current React route as the live baseline
- continue delivering the prompt-bank contract there if needed
- treat React as a compromise path, not as the chosen greenfield platform

## 13. Open Questions

These are still material and prevent promotion to ready:

- is the SolidStart direction intended only for the Socratic lobby or for a
  larger future Planner surface area?
- will the SolidStart build coexist beside the current React app during
  migration, or replace it as a new frontend root?
- how should Auth0, routing, and current websocket/session glue be packaged for
  a Solid-native app shell?
- what is the intended greenfield test stack replacement strategy for the
  current Vitest/Playwright route harness?

## 14. Readiness Judgment

This spec is now **ready for implementation**.

The earlier blocker on migration boundary is closed in the broader
[Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md).

What remains for this document is route-level delivery planning against the now
locked platform direction, not another round of platform indecision.
