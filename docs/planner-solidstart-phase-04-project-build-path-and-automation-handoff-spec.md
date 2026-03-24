# Planner SolidStart Phase 04 Project Build Path And Automation Handoff Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 03 Project Review And Build Readiness Spec](/home/thetu/planner/docs/planner-solidstart-phase-03-project-review-and-build-readiness-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md)

> Planning note (2026-03-24): after Phase 03 added project-local review and
> build-readiness surfaces, the next widening slice should stop short of full
> build execution while still making the build path explicit. The right next
> step is a project-local automation handoff surface that turns readiness into
> an obvious next move instead of leaving it as a passive status readout.
>
> Implementation sync (2026-03-24): the Solid project workspace now includes an
> attached `Build path` surface inside the same hidden-by-default project
> reveal. The new surface translates build readiness into an explicit
> automation-handoff summary with a compact handoff target, blockers,
> confirmations, and one clear next action. Verification completed with helper
> tests, Solid lint/build, and Playwright proof that the handoff surface
> remains secondary to active analysis while staying local-fast once opened.

## 1. Executive Judgment

The next SolidStart widening slice should add a **project-local build path**
surface inside the project workspace.

The app now lets the user:

- shape a project
- continue active Socratic analysis
- inspect knowledge and blueprint
- review reconciliation work
- understand build readiness

But it still lacks the next operational bridge:

- what exactly happens when the project is ready
- what should the user do next
- what handoff state will the automated build platform consume

Phase 04 should solve that inside the project workspace, not by adding another
top-level route.

## 2. User Outcome

After Phase 04:

- users can open an attached build-path surface from the project workspace
- build readiness turns into an explicit next-step handoff, not a passive label
- blocked projects explain what must happen before build handoff
- ready projects make the build path obvious within a few seconds
- the project route continues to feel like one coherent workspace, not a route
  maze

## 3. Locked Decisions

- the next widening slice stays inside `/projects/:projectSlug`
- the build path remains a hidden-by-default attached surface
- active Socratic analysis stays the primary default view
- this slice is about **handoff clarity**, not full build execution
- no new top-level build dashboard is introduced in this phase

## 4. Scope

### In Scope

- project-local `Build path` attached surface
- explicit automation handoff summary
- readiness-to-handoff translation
- concise prerequisites, blockers, and next action
- local-fast switching between analysis, review, readiness, and build path when
  data is already known

### Out Of Scope

- actual build job execution UI
- long-running build logs
- deployment surfaces
- admin/ops route migration

## 5. Product Problem

The user’s main goal is not to stop at analysis or even at readiness. It is to
shape an idea until it can move into the automated build platform.

Right now the project workspace can say:

- this needs review
- this is still in progress
- this is ready

But it still does not say:

- what the build handoff consists of
- what the user is handing off
- what remains missing in build terms

That creates a dead-end feeling just when the workspace should become the most
confident.

## 6. Phase 04 Product Model

The default project workspace remains:

- project identity
- active analysis
- recent sessions
- attached review/readiness/knowledge/blueprint surfaces

Phase 04 adds:

- `Build path`
  - project-local automation handoff summary
  - explicit readiness-to-build interpretation
  - concise next action

This surface should read like:

- "here is what the system knows"
- "here is what is still missing"
- "here is the next build-facing move"

It must not read like:

- a generic dashboard
- a raw log viewer
- a pseudo-terminal

## 7. Build Path Contract

The build-path surface should answer:

- what artifact or project state is being handed to the automated build system?
- is the project blocked, review-gated, or ready?
- what are the top missing prerequisites?
- what is the single clearest next action?

The surface should present:

- handoff posture
- concise project snapshot
- blockers
- confirmations
- handoff-ready summary when appropriate

## 8. Local-Speed Contract

Phase 04 continues the local-speed rule:

- opening Build path should be immediate when readiness data is already loaded
- switching among attached surfaces should remain local and synchronous
- build-path inspection must not reset the active project workspace
- the surface must feel like attached project context, not a route transition

## 9. Visual-Clarity Contract

The build-path surface must stay compact and operational:

- clear state first
- one strong next action
- short lists instead of narrative paragraphs
- no giant hero takeover
- no repeated explanation of what the project route already shows elsewhere

## 10. Testing Contract

Phase 04 should extend the Solid verification surface with:

- unit tests for build-path handoff derivation
- browser proof that the build-path surface is hidden by default
- browser proof that blocked versus ready handoff states are easy to distinguish

## 11. Acceptance Criteria

This slice is complete only when:

1. the project workspace still centers active analysis by default
2. `Build path` is available as a hidden-by-default attached surface
3. the surface explains build posture and next action quickly
4. blocked projects show concise handoff blockers
5. ready projects show a clear handoff-ready state
6. the build-path surface does not introduce new route clutter
7. browser verification proves it remains secondary to active analysis

## 12. Verification Plan

### Unit / component

- build-path summary tests
- readiness-to-handoff derivation tests

### Browser

- open project workspace
- open Build path
- verify blocked handoff state
- verify ready handoff state
- confirm the project route still reads as analysis-first

### Build

- Solid app build succeeds
- Rust server continues to serve the widened project route

## 13. Rollback / Fallback

If a full build-path surface is too large for one bounded slice, the truthful
fallback is:

- land the handoff summary and next-action path first
- defer richer build-state detail to the next bounded phase

Disallowed fallback:

- adding a new top-level Build route just to expose the surface quickly

## 14. Readiness Judgment

This spec is **ready for implementation**.

The next bounded widening move is clear:

- stay project-first
- keep analysis primary
- turn readiness into explicit automation handoff
- keep the whole experience project-local and fast
