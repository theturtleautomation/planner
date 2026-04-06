# Socratic Project Picture First-Reveal Screen Spec

**Status:** draft  
**Date:** 2026-04-03  
**Parent:** [Socratic Project Picture And Convergence Workspace Spec](/home/thetu/planner/docs/socratic-project-picture-and-convergence-workspace-spec.md)  
**Related Planning:** [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 38 Socratic Multimodal Command Desk Spec](/home/thetu/planner/docs/planner-solidstart-phase-38-socratic-multimodal-command-desk-spec.md), [Planner SolidStart Phase 39 Session Commit Continuity And Prompt-Bank Merge Spec](/home/thetu/planner/docs/planner-solidstart-phase-39-session-commit-continuity-and-prompt-bank-merge-spec.md), [Planner SolidStart Phase 40 Project-Only Entry And Stale-Draft Hardening Spec](/home/thetu/planner/docs/planner-solidstart-phase-40-project-only-entry-and-stale-draft-hardening-spec.md), [Blueprint Project Root And CodeGraph Integration](/home/thetu/planner/docs/blueprint-project-root-codegraph-integration.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Review:** 2026-04-03 bounded planning pass after the project-picture parent brief and MVP gap-analysis report locked the first visual form, the MVP top-level area model, and the silent-update boundary

## 1. Purpose

Define the first visible screen of the project-picture-first Socratic MVP.

This spec exists to answer a narrower question than the parent brief:

- what should the user actually see on first reveal?

It does **not** define the full area-workspace contract after a user enters one
area. That remains a separate child-spec concern.

## 2. Problem

The current implemented SolidStart route still reveals the Socratic experience
primarily as a prompt-bank workspace.

That route may already be truthful about:

- prompt-bank persistence
- local answer continuity
- answer-level progression
- three-surface desktop layout

But it still makes the user orient around prompt handling rather than around
the project itself.

Without a bounded first-reveal screen spec, the likely failure mode is one of
two bad outcomes:

- a decorative project picture layered over the existing prompt-first route
- a graph-brained interface that feels like an architecture tool instead of
  Darkfactory

## 3. User Outcome

On first reveal, the user should be able to understand in one pass:

- what this project currently is
- what the main parts of the project are
- which areas are under pressure
- what is already defined versus still weak
- where the most important next move lives

The user should feel that they have entered:

- a living project
- not a blank prompt
- not a question stack
- not a graph editor
- not a mirrored blueprint console

## 4. Scope

### In Scope

- the first visible hierarchy of the project-picture-first screen
- the visible top-level area model
- the default relationship density on first reveal
- what supporting elements are visible immediately versus deferred
- the role of global idea capture on first reveal
- what the first-reveal screen must not become

### Out Of Scope

- detailed area-workspace behavior after the user enters an area
- exact substructure inside an area
- detailed overlay system design
- full convergence-autonomy rules beyond what first reveal requires
- implementation-level component or transport decomposition

## 5. Locked Inputs From Parent Planning

This child spec assumes the following are already decided:

- the first visual form is a calm area-based project picture
- the visible top-level areas are:
  - `Transformation`
  - `Actors`
  - `Constraints`
  - `Approach`
  - `Pressure`
- the product must remain behavior-first, not structure-first
- the project picture must not default to a node-edge graph
- the picture is the primary truth surface
- low-risk silent updates may affect state, confidence, suggested labels,
  tension markers, and suggested relationships, but may not silently rewrite
  user-committed meaning

## 6. First-Reveal Screen Contract

### 6.1 Primary screen hierarchy

The first reveal must be dominated by the project picture.

Required hierarchy:

- one compact project identity and north-star framing layer
- one dominant project picture surface
- one visible but secondary next-move support layer
- one subordinate global idea-capture affordance

The first screen must **not** behave like:

- a blank prompt screen
- a question list with an attached summary
- an equal-weight three-column dashboard
- a graph canvas that asks the user to decode system topology first

### 6.2 Project identity layer

The first screen should include a compact framing layer that reminds the user:

- what this project is
- what it is trying to change

This layer should be concise.
It should not become a large hero, long brief, or document preamble.

### 6.3 The project picture itself

The dominant surface should be an area-based picture composed of five visible
zones:

- `Transformation`
- `Actors`
- `Constraints`
- `Approach`
- `Pressure`

These zones should feel like parts of one coherent project, not isolated
widgets.

They should remain spatially stable enough for user memory to form.

### 6.4 Zone behavior on first reveal

Each visible zone should expose only enough information to support orientation.

At minimum, each zone should show:

- its name
- its visible state
- a compact current read or summary
- visible pressure when present

The first-reveal screen should not require a user to open the area before
understanding whether it is:

- defined
- incomplete
- unclear
- conflicted

### 6.5 Relationship display on first reveal

Relationships must be first-class but tightly bounded.

Default first-reveal behavior:

- show only foundational dependencies
- show only critical conflicts
- keep all other relationship detail out of the default view

The first-reveal screen must not attempt to visualize every influence path or
every latent dependency.

### 6.6 Next-move support on first reveal

The first screen should make the next move visible, but not let the next-move
surface compete with the picture itself.

Required behavior:

- one obvious current next move must be visible
- additional guidance may exist, but must remain secondary
- the screen should not read like a task feed

This means:

- the picture remains primary
- the next-move surface supports interpretation and action
- the user should not have to open an overlay to know where to start

### 6.7 Global idea capture on first reveal

The first reveal should expose a visible but subordinate global capture path.

It should exist because the user may have a thought before entering any one
area.

It should stay subordinate because otherwise the product collapses back toward
prompt-first interaction.

### 6.8 What first reveal must not promise

The first screen must not imply:

- that every visible relationship is being fully visualized
- that every visible area is deeply explorable already
- that the system has finished converging the project
- that a graph-tool mental model is required to use the product

## 7. Design Direction

The first reveal should feel:

- calm
- spatial
- coherent
- product-first
- low-noise

It should not feel:

- theatrical
- diagrammatic
- academic
- over-instrumented
- like a control room

The intended visual tone is closer to a premium product workspace than to a
whiteboard, a graph tool, or a dashboard.

## 8. Touched Surfaces

The likely primary implementation surfaces for this slice are:

- `planner-solid/src/routes/sessions/session-workspace-screen.tsx`
- `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- `planner-solid/src/routes/sessions/session-workspace-view.ts`
- `planner-solid/src/app.css`
- route-level tests and browser proof for the session workspace

This child spec should avoid reopening backend prompt-bank or continuity logic
unless the first-reveal screen exposes a concrete blocker in those already-
implemented contracts.

## 9. Acceptance Criteria

1. the first visible screen is clearly the project picture, not a prompt stack
2. the five MVP top-level areas are visible on first reveal
3. the screen makes state and pressure legible without requiring the user to
   enter an area first
4. the default screen shows only bounded, high-value relationships
5. one obvious next move is visible without overpowering the picture
6. a global capture path exists but remains subordinate to the main picture
7. the screen cannot be mistaken for a graph editor or architecture diagram

## 10. Verification Plan

Before this child spec is promoted or implemented, it should be checked against:

- the parent brief for scope drift
- the MVP path doc for boundedness
- a product walkthrough proving the screen reads as project-first rather than
  prompt-first
- future browser proof once a bounded slice is selected

## 11. Rollback / Fallback

If this first-reveal screen proves too broad to implement in one pass, the
fallback is:

- keep the same five-area first-reveal hierarchy
- reduce relationship detail further
- reduce visible support layers further

The fallback is **not**:

- reverting to a prompt-first first screen
- exposing the blueprint graph directly
- making the picture decorative only

## 12. Open Questions

The main remaining blocker for the next child spec is:

- how much substructure one area should expose on day one

Secondary non-blocking follow-ons:

- which visible causes of confidence loss belong on the main screen
- whether first reveal should show one next move or a very small grouped set
- how research-heavy areas should read without becoming dashboard-like

## 13. Readiness Judgment

This spec is **draft but bounded**.

It is strong enough to anchor the next child spec and to keep future work from
drifting back into graph-first or prompt-first UX.

It is not yet promoted to ready because the adjacent area-workspace child spec
still needs the unresolved day-one substructure decision.
