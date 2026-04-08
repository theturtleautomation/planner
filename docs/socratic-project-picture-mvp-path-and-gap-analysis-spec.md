# Socratic Project Picture MVP Path And Gap Analysis Spec

**Status:** draft  
**Date:** 2026-04-03  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Related Planning:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening-spec.md), [Blueprint Project Root And CodeGraph Integration](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-04-03 repo-grounded planning pass across the current SolidStart Socratic route, the new project-picture parent brief, and adjacent implemented session-route slices

## 1. Purpose

Log the grounded planning report for the new Socratic / Darkfactory direction
and turn that report into a durable working surface.

This document exists to answer four immediate planning questions:

- what already exists in the repo that advances the future MVP
- what the smallest coherent MVP actually is
- what gaps still block implementation
- how the team should work through those gaps without over-researching or
  reopening already-implemented route work

This is not a ready implementation slice.
It is a draft planning artifact that should drive the next spec-tightening pass.

## 2. Current State

### 2.1 What already exists and should be reused

The repo is not starting from a blank slate.

The following are already materially implemented and should be treated as
baseline infrastructure rather than reopened product questions:

- project-first entry and removal of projectless direct-session creation in
  [Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening-spec.md)
- prompt-bank truth, local-first route continuity, and commit progression in
  [Planner SolidStart Phase 18 Prompt-Bank Conformance And Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-18-prompt-bank-conformance-and-closeout-remediation-spec.md),
  [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md),
  and
  [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md)
- hidden blueprint/project-root structure and curated relationship handling in
  [Blueprint Project Root And CodeGraph Integration](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md)
- earlier local-first master-detail and bounded-DOM interaction lessons in
  [Socratic Lobby Master-Detail Local Workspace Spec](/home/thetu/planner/docs/socratic-lobby-master-detail-local-workspace-spec.md)
  and
  [Socratic Lobby Local-First Browser Architecture Review](/home/thetu/planner/docs/socratic-lobby-local-first-browser-architecture-review.md)

### 2.2 What is only planned

The new product center of gravity is only planned, not implemented.

Specifically:

- [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)
  correctly reframes the future-state route around a project-picture-first
  collaborative workspace
- but that parent brief is still intentionally broad and explicitly not ready
  for bounded implementation

### 2.3 What should not be mistaken for blockers

These are solved enough for MVP planning and should not be reopened unless a
new spec proves they are direct blockers:

- full prompt-bank persistence
- answer-level submission continuity
- ultra-wide command-desk shell behavior
- project-only entry
- existence of a hidden blueprint-like structural layer

## 3. MVP Definition

The smallest coherent MVP is:

- a project-picture-first Socratic workspace where the first thing the user
  sees is a living project picture rather than a blank prompt or a question
  stack
- the picture exposes a small number of major areas, visible pressure, and a
  small number of high-value relationships
- the user can enter one area, understand the current state of that area
  quickly, and shape it through a bounded interaction model
- the existing prompt-bank remains underneath as one shaping mechanism, not as
  the primary product identity
- the system continuously reconverges internally and may update low-risk
  visible signals without silently rewriting user-committed meaning

### 3.1 In MVP

- first screen is the project picture
- small stable major-area model
- visible state language
- selective relationship visibility
- area entry and shaping flow
- global and local idea capture
- low-risk silent updates to visible signals
- whole-project recoverability from inside deep area work

### 3.2 Out of MVP

- raw graph-tool experience
- raw blueprint UI as the primary surface
- full seed-tray or branch-management product systems
- rich under-the-hood overlays
- full provenance inspection
- broad media capture and multimodal inputs
- "finish the whole organism" scope

## 4. Gap Analysis

### 4.1 Missing product decisions

The current planning thread still lacks answers to several questions that
materially change the MVP:

- the exact first visual form of the project picture
- the minimum major-area model the user sees on day one
- the hard line between silent low-risk updates and protected user-committed
  meaning
- the default relationship density and emphasis rules
- how much substructure one area exposes before it becomes recursively complex

### 4.2 Missing planning artifacts

The current parent brief is not enough by itself.

The planning set is missing:

- one bounded MVP execution slice that pulls the above into a coherent first
  delivery target

The first-reveal project-picture child spec now exists as:

- [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md)

The area-workspace child spec now exists as:

- [Socratic Area Workspace And Shaping Contract Spec](/home/thetu/planner/docs/socratic-area-workspace-and-shaping-contract-spec.md)

The autonomy-boundary child spec now exists as:

- [Socratic Convergence Autonomy Boundary Spec](/home/thetu/planner/docs/socratic-convergence-autonomy-boundary-spec.md)

The bounded MVP execution slice now exists as:

- [Socratic Project Picture MVP Slice Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-slice-spec.md)

### 4.3 Missing implementation slices

This gap is now closed.

The bounded execution artifact exists and has now been implemented as:

- [Socratic Project Picture MVP Slice Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-slice-spec.md)

### 4.4 Risks from unresolved ambiguity

If implementation starts from the current parent brief, the likely failure
modes are:

- building a decorative picture layer over the existing prompt flow
- building a graph-brained interface that feels like a tool for architects
  rather than Darkfactory
- reopening prompt-bank or route shell work that is already good enough
- over-specifying secondary overlays and internal systems before the first
  visible product surface is locked

## 5. Required Specs

The minimum planning set before bounded implementation is:

### 5.1 Tighten existing

**Title:** `Socratic Project Picture And Convergence Workspace Spec`  
**Type:** tighten existing  
**Parent:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)  
**Why:** the parent brief still mixes MVP-critical product rules with later
follow-on ambition  
**When:** before implementation

### 5.2 New child spec

**Title:** `Socratic Project Picture First-Reveal Screen Spec`  
**Type:** new  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Why:** the MVP cannot proceed until the actual first visible artifact is
specified concretely  
**When:** drafted on 2026-04-03; ready-state still pending adjacent child-spec tightening

### 5.3 New child spec

**Title:** `Socratic Area Workspace And Shaping Contract Spec`  
**Type:** new  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Why:** the core interaction loop still lacks a bounded definition for area
entry, context leveling, and shaping interactions  
**When:** drafted on 2026-04-03; ready-state still pending autonomy-boundary tightening

### 5.4 New child spec

**Title:** `Socratic Convergence Autonomy Boundary Spec`  
**Type:** new  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Why:** trust depends on a hard line between silent signal updates and
protected user-committed meaning  
**When:** drafted on 2026-04-03; ready-state still pending bounded MVP-slice selection

### 5.5 New execution slice

**Title:** `Socratic Project Picture MVP Slice Spec`  
**Type:** new  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Why:** after the three product child specs are tightened, one bounded slice is
still needed to define the first real build target  
**When:** drafted on 2026-04-03 and now implemented

### 5.6 Later optional follow-on

**Title:** `Socratic Secondary Overlays And Seed Handling Spec`  
**Type:** new  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Why:** useful follow-on, but not required to prove the MVP  
**When:** later

## 6. Outstanding Questions

Ordered by leverage:

### 6.1 What is the first visual form of the project picture?

**Why it matters:** this decides whether the product feels like Darkfactory or
like a graph editor with better copy  
**Recommended answer:** a calm area-based picture with stable zones and
selective tension markers, not a node-edge graph as the dominant visual  
**Current answer:** locked in the parent brief  
**Who must answer:** answered

### 6.2 What are the minimum major areas in the MVP picture?

**Why it matters:** without a bounded area model, the first screen still has no
real information architecture  
**Recommended answer:** start with a small adaptive set anchored by
transformation, actors, and decisive constraints rather than a broad ontology  
**Current answer:** `Transformation`, `Actors`, `Constraints`, `Approach`,
`Pressure`  
**Who must answer:** answered

### 6.3 What exact updates may happen silently?

**Why it matters:** this is the core trust boundary for convergence  
**Recommended answer:** allow silent updates to state, confidence, suggested
labels, and tension markers; do not silently rewrite area identity, accepted
major relationships, or the current north-star definition  
**Current answer:** confirmed as the working MVP boundary and should now be
turned into bounded child-spec wording  
**Who must answer:** answered at product level and now captured in the autonomy
child spec

### 6.4 How much internal substructure should one area expose on day one?

**Why it matters:** too much turns the workspace into recursive complexity; too
little makes it shallow  
**Recommended answer:** expose only the few pressure points that materially
matter right now, not a full sub-map  
**Current answer:** a small number of meaningful pressure points, usually 2 to
4, with one visually dominant  
**Who must answer:** answered

### 6.5 How visible should relationships be by default?

**Why it matters:** this is the difference between useful orientation and
spaghetti  
**Recommended answer:** default to foundational dependencies and critical
conflicts only  
**Current answer:** foundational dependencies and critical conflicts only  
**Who must answer:** answered strongly enough for the MVP slice

### 6.6 Are soft seeds part of MVP or later?

**Why it matters:** seeds easily become clutter and planning debt  
**Recommended answer:** keep soft capture minimal in MVP and defer a richer
seed system  
**Current answer:** later, with only minimal soft capture in MVP  
**Who must answer:** answered strongly enough for the MVP slice

### 6.7 Which overlays are truly essential on day one?

**Why it matters:** too many overlays turn the product into a cockpit  
**Recommended answer:** whole-project recovery plus pressure and next-moves are
enough for the first pass  
**Current answer:** whole-project recovery plus pressure and next-moves are
enough for the first pass  
**Who must answer:** answered strongly enough for the MVP slice

## 7. How To Work Through The Gaps

The right method is **not** “more broad research first.”

The repo already has enough product and implementation context to answer most
questions through bounded design judgment.

Use this decision rule:

### 7.1 Direct Q&A with the user

Use short, one-question-at-a-time product decisions for:

These four are now answered and should be treated as locked unless later
pressure-testing proves them unstable:

- first visual form
- minimum major-area model
- exact silent-update boundary
- area substructure level

These are not research problems.
They are authorship and product-shape decisions.

### 7.2 Repo-grounded synthesis, not user interviews

Use local repo evidence to answer:

- what route substrate can be reused
- what must not be reopened
- what planning containers already exist
- what should remain hidden backbone versus visible product

### 7.3 Targeted research only when it resolves a concrete visual or UX fork

Research is still useful, but only if it is narrowly scoped.

Good reasons to research:

- validating whether a specific relationship-density model is likely to fail
- comparing 2 to 3 concrete area-substructure interaction patterns after the
  first child spec exists
- borrowing clarity patterns from mature high-complexity products

Bad reasons to research:

- restudying AI-native workspaces at a general level
- collecting more architecture-model frameworks
- reading more graph or whiteboard metaphors without a live design fork to
  resolve

### 7.4 Do not prototype before the first screen is named

The first visual form and the major-area model should be decided before any
implementation slice is promoted.

Otherwise the team will prototype the wrong artifact and rationalize it later.

This gate is now satisfied at the parent-brief level, but not yet at the
child-spec level.

## 8. Recommended Sequence

1. tighten the parent brief to cut MVP versus later ambition more sharply
2. draft the first-reveal screen child spec using the now-locked visual form
   and major-area model
3. draft the area-workspace and shaping-contract child spec using the now-
   locked area-substructure rule
4. draft the autonomy-boundary child spec using the now-locked silent-update
   boundary as its starting point
5. draft the bounded MVP slice spec
6. implement only from that bounded slice

## 9. What To Postpone

Postpone these until after the first MVP slice is defined:

- full seed systems
- rich under-the-hood views
- broader overlay families
- architecture-export lenses
- any attempt to make the user experience the blueprint graph directly

## 10. Blunt Assessment

### 10.1 What is solid

- the repo substrate
- the project-first product direction
- the hidden blueprint-like truth layer
- local-first route continuity and prompt-bank handling
- the broad product thesis in the parent brief

### 10.2 What is still half-baked

- later overlay breadth and seed handling ambition
- whether non-default area switching should become a later bounded follow-on
- keeping follow-on work from reopening prompt-bank and route-shell substrate

### 10.3 What would be a mistake next

- coding from the parent brief as if it were ready
- reopening Phase 38 to 40 as if prompt mechanics were still the main blocker
- over-researching broad AI workspace trends instead of deciding the first
  visible product surface

## 11. Readiness Judgment

This planning report is **ready to use as a working artifact**, but it is
**not a ready implementation spec**.

The next move is no longer foundational planning.

The next move should be one of:

- review the implemented slice against broader product expectations and decide
  whether a follow-on for multi-area entry is warranted
- or open a new bounded follow-on only if the next capability is explicit
