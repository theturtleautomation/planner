# Socratic Lobby Local-First Browser Architecture Review

**Status:** active  
**Date:** 2026-03-24  
**Parent:** [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md)  
**Related Planning:** [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md), [Socratic Lobby Consultant Desk Spec](/home/thetu/planner/docs/socratic-lobby-consultant-desk-spec.md), [Socratic Hybrid Question Routing And Latency Spec](/home/thetu/planner/docs/socratic-hybrid-question-routing-and-latency-spec.md)

## Purpose

This document is a complementary architecture review behind the current
Socratic prompt-bank direction.

It records the design tension and the selected browser/runtime strategy:

- the lobby should feel like a premium continuous consultant desk
- the browser must not pay the cost of mounting the full generated graph as one
  large interactive DOM tree
- local thread switching and local editing must remain instantaneous
- dynamic background updates must continue after first reveal

This note exists to support the planning decision, not to replace the delivery
spec.

## Executive Judgment

The strongest browser architecture for the Socratic lobby is a hybrid:

- keep all known prompt-bank and thread data loaded in normalized client state
- keep the visible workspace dense and local-feeling
- bound the live DOM strictly to the active workspace surface and visible UI
- let later generated content merge into local state without blocking the user

The key insight is correct:

- a premium consultant-desk experience cannot feel like a wizard or route swap
- a naive "mount everything" document is also wrong because it creates typing
  and reconciliation risk as the generated graph grows

The correct 2026 browser pattern is:

- all known data in local state
- only actively needed data in the DOM

## Core Architectural Tension

The product wants two things at once:

1. completeness at first reveal
2. native-feeling editing and navigation after reveal

Those goals fight each other if implemented naively:

- a thin server payload plus preview shells feels incomplete and deceptive
- a giant always-mounted question document risks input lag and layout churn

The correct compromise is not to weaken either goal. It is to separate:

- data completeness
- DOM completeness

For this route:

- data should be complete for the initial bank
- DOM should stay intentionally bounded

## Review Of The Recommended Direction

### 1. The core insight is correct

The consultant-desk goal requires spatial permanence and local continuity.

Users should be able to:

- trust that visible answerable work is actually present
- switch among known threads with no spinner
- keep their cursor and context stable while background generation continues

That does not require one giant mounted DOM. It requires one giant **known data
graph** plus a bounded rendered surface.

### 2. Jotai is a better fit than a broad mutable store

This route is not a static form. It is a dynamic graph of:

- threads
- prompt envelopes
- question items
- drafts
- derived telemetry
- background insertions

The benefit of Jotai for this shape is subscription granularity.

The useful unit of update here is not "the whole Socratic store." It is:

- one thread
- one prompt bank entry
- one question
- one derived progress counter

That aligns naturally with atomized state.

Zustand can still work, but it tends to push more normalization and selector
discipline onto the implementation. For this specific route, Jotai is the
cleaner state primitive for the growing graph problem.

### 3. Virtualization is the right safety valve when the product wants
continuity

If the product later chooses a larger continuous review surface, virtualization
is the safe way to preserve that feeling without unbounded DOM cost.

TanStack Virtual is the most relevant current tool because it supports:

- dynamic measurement
- overscan
- stable range control
- scroll correction when content changes size

That matters because Socratic content is not fixed-height:

- textareas expand
- categories may gain new prompt items
- machine context blocks may change size

### 4. Live insertion must stabilize the user’s viewport

Dynamic updates cannot be allowed to steal the user’s cursor or shift their
active editing surface.

The right model is:

- append or revise known data in normalized client state
- keep the active workspace mounted and stable
- only materialize new UI where it belongs
- avoid re-rendering unrelated active controls

This prevents the "remote app" feel where the page seems to move under the
user.

### 5. This complements the active prompt-bank direction

The active prompt-bank spec solves the biggest truth problem:

- first reveal must contain real prompts, not shells

This review complements that with the browser/runtime decision:

- local-first state and bounded rendering are the right way to preserve native
  feel once that bank exists

## Technology Review

### React 19 remains viable

The current repo already uses React 19, Jotai, Zustand, and TanStack Virtual.
That means the core performance strategy can be achieved without a framework
rewrite.

React is not the blocker here. The blocker is the runtime/data contract:

- one prompt versus a prompt bank
- shell rows versus truthful local-ready rows

### Recommended browser/runtime stack

For Planner, the strongest current direction is:

- React 19 for the route shell
- Jotai for atomized lobby graph state
- TanStack Virtual where the rendered review surface needs bounded DOM
- a real local-first prompt-bank model in client state
- server-authored dynamic updates merged incrementally after first reveal

### When local client storage becomes important

If the route evolves toward stronger offline or resumable local behavior, the
best next evaluation targets are:

- Dexie for a mature browser-local database layer
- TanStack DB for a more ambitious reactive client-first data layer
- PowerSync or Electric-style sync if the app later wants true synced local
  database semantics

Those are future-facing complements, not blockers for the current prompt-bank
fix.

## Decision

Approve the browser architecture recommendation entirely.

The selected direction is:

- master-detail shell stays
- initial prompt bank becomes the truth contract for first reveal
- Jotai remains the preferred client-state direction for the growing graph
- TanStack Virtual remains the preferred rendering safety valve where continuity
  would otherwise create DOM pressure
- the system should behave like a live local workspace, not a prompt wizard and
  not a monolithic rendered document

## Implications For The Active Spec

The active prompt-bank spec should be read with this complementary decision in
mind:

- "full bank on load" means full bank in known local state for the initial
  derivable prompt set
- "dynamic updates" means later server-authored inserts or replacements that do
  not destabilize the active workspace
- "native feel" means local switching, isolated typing, bounded DOM, and stable
  viewport ownership

## Handoff

This document does not add a separate implementation slice.

It strengthens the architectural basis for the next delivery cycle:

- backend: initial prompt-bank assembly and transport
- frontend: truthful first reveal and local-ready thread semantics
- runtime: preserve native-feeling responsiveness through atomized state and
  bounded rendering
