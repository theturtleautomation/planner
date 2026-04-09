# Socratic Project Picture And Convergence Workspace Spec

**Status:** Implemented  
**Date:** 2026-04-09  
**Parent:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)  
**Related Planning:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md), [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md), [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md), [Socratic Area Workspace And Shaping Contract Spec](/home/thetu/planner/docs/socratic-area-workspace-and-shaping-contract-spec.md), [Socratic Convergence Autonomy Boundary Spec](/home/thetu/planner/docs/socratic-convergence-autonomy-boundary-spec.md), [Socratic Project Picture MVP Slice Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-slice-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)

## Purpose

Define the canonical broad thesis for the Socratic/project-picture direction.

This artifact exists to state the whole top-level product direction cleanly,
without forcing the reader to reconstruct it from several narrower child specs
or from an older future-state brainstorm.

It is a planning artifact, not an implementation slice. Its job is to provide a
stable parent surface from which later branch work can proceed.

## Canonical Thesis

The product should become a **project-picture-first collaborative workspace**.

That means:
- the user first meets a living project picture rather than a blank prompt,
- the project picture becomes the primary orientation and truth surface,
- questions become one shaping tool rather than the product's center of gravity,
- a hidden truth model keeps the system rigorous underneath the humane visible
  surface,
- and continuous convergence keeps the project coherent without dissolving user
  authorship.

In this model, the system's intelligence is primarily proven by the quality of
its evolving project picture, not by prompt handling, graph literalism, or
self-explanatory system chatter.

## Core Product Position

### 1. The main thing is the project picture

The primary visible artifact is a living project picture.

The product should not feel centered on:
- a blank prompt box,
- a question stack,
- a document editor,
- or a generic architecture/graph tool.

The user should be able to understand the current state of the project through a
calm, human-readable picture of what matters, where pressure exists, and what
kind of shaping is needed next.

### 2. The visible picture is humane, but grounded in hidden rigor

The visible surface is not the raw blueprint.

A hidden truth model may rigorously track:
- what areas exist,
- which parts are foundational or downstream,
- which tensions are real,
- and what likely matters next.

But the user-facing product must remain humane and project-shaped rather than
forcing the user to inhabit a raw internal graph.

### 3. Questions are a tool, not the center

Questioning and prompting remain useful, but they are subordinate to the larger
job of helping the user preserve whole-project meaning.

The product's center of gravity is:
- whole-project understanding first,
- then area shaping,
- then question-led refinement where needed.

### 4. Convergence is necessary, but trust is load-bearing

Background convergence is part of the thesis.

Without it, detail drift wins and the picture becomes stale. But if visible
meaning changes too aggressively, the system becomes slippery and untrustworthy.

So the product must support:
- active internal convergence,
- low-risk visible tightening,
- and explicit protection of user-committed meaning.

### 5. The user still owns authorship

The system should behave like a sharp colleague:
- it may challenge,
- synthesize,
- sharpen,
- and suggest better structures,
- but it must not silently replace the user's intended project with its own.

## Broad User Outcome

When the thesis is realized, a user should be able to tell quickly:
- what this project currently is,
- what parts are load-bearing,
- what parts are defined or weak,
- where pressure or contradiction exists,
- where deeper exploration lives,
- and what the best next moves are.

The user should be able to:
- enter directly into the project picture,
- open one area and shape it without losing the whole,
- add ideas globally or locally,
- understand tension without being interrupted constantly,
- and recover whole-project orientation even after deep local work.

## Parent-Level Product Model

At the broad thesis level, the dominant loop is:
- **project picture -> enter area -> shape area -> picture updates**

The top-level artifact should remain:
- calm,
- spatially legible,
- behavior-first rather than structure-first,
- and oriented around project meaning rather than tooling theater.

The broad thesis also locks these parent-level truths:
- the project picture is the primary visible truth surface,
- the product is not graph-first,
- the system should stay recoverable rather than making everything always
  visible,
- and guidance should create visible pressure and direction without turning the
  workspace into either chat drift or dashboard clutter.

## Existing Child Planning Surfaces

The broad thesis is no longer standing alone. These child artifacts already
exist beneath it and should be treated as subordinate planning surfaces rather
than as still-missing prerequisites.

### First-reveal child surface
- [Socratic Project Picture First-Reveal Screen Spec](/home/thetu/planner/docs/socratic-project-picture-first-reveal-screen-spec.md)
- Covers the first visible hierarchy and the initial picture-first screen
  contract.

### Area-workspace child surface
- [Socratic Area Workspace And Shaping Contract Spec](/home/thetu/planner/docs/socratic-area-workspace-and-shaping-contract-spec.md)
- Covers what happens after entering one area, including shaping behavior and
  area-level interaction boundaries.

### Autonomy-boundary child surface
- [Socratic Convergence Autonomy Boundary Spec](/home/thetu/planner/docs/socratic-convergence-autonomy-boundary-spec.md)
- Covers the trust boundary between convergence, silent visible updates, and
  pending revisions.

### MVP planning and execution surfaces
- [Socratic Project Picture MVP Path And Gap Analysis Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-path-and-gap-analysis-spec.md)
- [Socratic Project Picture MVP Slice Spec](/home/thetu/planner/docs/socratic-project-picture-mvp-slice-spec.md)
- These surfaces describe the bounded MVP framing and execution slice beneath
  the broader thesis.

## What This Thesis Now Resolves

This pass resolves the parent-level ambiguity about what the broad direction is.

It makes explicit that the Socratic/project-picture direction is:
- a project-picture-first workspace,
- grounded in hidden structure but not graph-literal as a UX,
- shaped one area at a time,
- supported by convergence with trust boundaries,
- and intended to preserve whole-project meaning while AI accelerates local
  detail.

## What Remains Deferred

Finishing the broad thesis does **not** mean every branch under it is now
resolved.

The following remain deferred to later dedicated passes:
- hidden truth-model / blueprint relationship as an active structural concern,
- whole-project recoverability beyond the current shell model,
- richer overlay / reorientation modeling,
- provenance / change-inspection UX,
- preview hierarchy refinement,
- and any later follow-on branch work that depends on those concerns.

These branches may be acknowledged by the thesis, but they are not resolved
here.

## Non-goals Of This Pass

This pass does **not**:
- implement code,
- reopen already-implemented MVP work,
- collapse all child artifacts back into the parent,
- or resolve structural concern branches inside the same rewrite.

## Outcome Of This Pass

The broad Socratic/project-picture direction now has one canonical top-level
artifact that cleanly states the thesis.

That means later work should be able to proceed by:
- inheriting from this thesis,
- approaching unresolved branches separately,
- and avoiding confusion about whether the broad direction itself is still an
  unresolved brainstorm.

## Sync impact

Updated truth surfaces in this pass:
- `docs/socratic-project-picture-and-convergence-workspace-spec.md`

Unchanged routing / summary truth surfaces:
- `.omx/ledger/planner-ledger.json`
- `.omx/ledger/project-plan.md`
- `.omx/ledger/current-status.md` *(routing/summary sections unchanged)*

Regenerated maintenance surfaces after final sync:
- `.omx/ledger/current-status.md` *(maintenance signal only)*
- `.omx/ledger/automation-trace.json`
- `.omx/ledger/automation-report.md`

Routing result after sync:
- `workstream:socratic-project-picture` remains `active`
- `routing_state` remains `needs_deep_interview`

Reason:
- the broad thesis is now explicit as a canonical top-level surface,
- but structural concern branches and other later follow-ons remain intentionally
  deferred,
- and `npm run project:ledger:auto` only needs to refresh maintenance metadata
  unless later routing truth changes.
