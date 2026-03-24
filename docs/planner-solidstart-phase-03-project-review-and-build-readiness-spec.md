# Planner SolidStart Phase 03 Project Review And Build Readiness Spec

**Status:** ready for implementation  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 02 Project Advanced Surfaces Spec](/home/thetu/planner/docs/planner-solidstart-phase-02-project-advanced-surfaces-spec.md), [Planner UI Reset Phase 08 Discovery Review Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-08-discovery-review-workspace-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md)

> Planning note (2026-03-24): after Phase 02 added hidden-by-default Knowledge
> and Blueprint inside the project workspace, the next widening slice should
> continue to stay project-local. The most aligned next capability family is
> project review and build readiness: turning deep Socratic analysis into
> explicit reviewable changes and an obvious "ready to build or not" project
> posture.

## 1. Executive Judgment

The next SolidStart widening slice should keep the project workspace as the
main home for work and introduce the first **review/build readiness** surfaces
inside it.

The user’s main goal is not just browsing knowledge or structure. It is:

- run deep Socratic analysis of an idea
- shape that analysis into buildable truth
- understand when the project is ready for the next automated step

The selected Phase 03 slice is therefore:

- project-local discovery/review surface
- project-local build-readiness surface
- both secondary to active analysis and hidden until needed

## 2. User Outcome

After Phase 03:

- the project workspace can show whether the project is still discovering,
  needs review, or is ready to build
- proposal/review work becomes reachable without leaving the project context
- users can inspect what still needs decision or acceptance before build
- build readiness reads as an explicit project state, not a vague implication
- the app stays focused because review/build surfaces remain secondary to the
  primary analysis path

## 3. Locked Decisions

- the next widening slice stays inside `/projects/:projectSlug`
- review and build-readiness remain project-local attached surfaces
- active Socratic analysis stays the primary default surface
- discovery review is the next attached capability family
- build readiness must be visible as a project posture, not buried in logs or
  route trivia
- no new top-level primary nav destinations are introduced in this phase

## 4. Scope

### In Scope

- project-local review/reconciliation surface
- project-local build-readiness summary surface
- attached reveal model for those surfaces
- local-fast switching between analysis, knowledge, blueprint, review, and
  build-readiness states where data is already known

### Out Of Scope

- full standalone Discovery route migration
- full build execution UI
- admin/ops route migration
- auth or deployment changes

## 5. Product Problem

The project workspace is now project-first and has attached knowledge and
blueprint inspection, but it still lacks the key bridge between analysis and
automation:

- what still needs review
- what was proposed by the system
- whether the project is actually ready to move into the build path

Without that bridge, the workspace still risks feeling like:

- a place to think

instead of:

- a place to think, review, decide, and advance toward build

## 6. Phase 03 Product Model

The default project workspace remains:

- project identity
- active analysis
- continue/start analysis path
- recent sessions

Phase 03 adds two more attached secondary surfaces:

- `Review`
  - proposal triage and acceptance/rejection state
- `Build readiness`
  - explicit project readiness summary and blockers

These must remain secondary and hidden by default.

## 7. Review Surface Contract

The project-local review surface should behave like a compact review desk:

- pending review work first
- accepted/rejected work secondary
- quiet controls
- direct action affordances

It must not:

- become a generic utility page
- compete with active analysis for top-of-page dominance
- require leaving the project workspace to understand the review state

## 8. Build-Readiness Contract

The project workspace should gain an explicit build-readiness surface that
answers:

- is this project ready to build?
- what is missing?
- what is blocked?
- what was recently completed?

This surface should be concise and legible:

- readiness state
- key blockers
- key confirmations
- clear next action

It must not become:

- a log viewer
- a deployment dashboard
- a noisy checklist wall

## 9. Local-Speed Contract

This slice continues the local-speed rule:

- opening Review or Build readiness should be immediate when data is already
  loaded
- switching among attached surfaces should not feel like route churn
- active analysis context must remain stable while these panels open or close
- build-readiness state should update truthfully without forcing a disruptive
  workspace reset

## 10. Visual-Clarity Contract

These new surfaces must remain subordinate to the project hero and primary
analysis task.

Rules:

- clear but compact state communication
- pending work more visible than completed/reviewed work
- no giant banners unless the project is truly blocked or build-ready
- no repeated explanatory boilerplate
- the workspace should still read as one project home, not several mini-apps

## 11. Testing Contract

Phase 03 should extend the Solid verification surface with:

- unit/component tests for project-local review grouping and build-readiness
  derivation
- browser verification that the review/build surfaces remain hidden until
  requested
- browser verification that pending review and readiness state are easy to
  distinguish once opened

## 12. Acceptance Criteria

This slice is complete only when:

1. the project workspace still centers active analysis by default
2. project-local Review is available as a hidden-by-default attached surface
3. project-local Build readiness is available as a hidden-by-default attached
   surface
4. pending review work is easier to find than already-reviewed work
5. build readiness is explicit and understandable within a few seconds
6. opening these surfaces does not reintroduce route clutter or workflow
   confusion
7. browser verification proves the intended primary-versus-secondary hierarchy

## 13. Verification Plan

### Unit / component

- review-surface grouping tests
- build-readiness summary tests
- attached-surface switching tests

### Browser

- open project workspace
- open Review
- inspect pending vs reviewed work
- open Build readiness
- verify readiness summary and blockers remain secondary to active analysis

### Build

- Solid app build succeeds
- Rust server handoff continues to serve the widened route set

## 14. Rollback / Fallback

If both attached surfaces are too large for one bounded slice, the truthful
fallback is:

- land the build-readiness posture first
- then land the review desk immediately after as the next bounded child slice

Disallowed fallback:

- turning review or readiness into top-level route clutter just because it is
  easier to expose

## 15. Open Questions

These do not block readiness:

- should Review and Build readiness join the existing attached advanced panel,
  or become a neighboring attached reveal family?
- should build-readiness state persist in the project header even when the
  attached panel is closed?
- should project-local session events remain deferred until after review and
  readiness are visible?

## 16. Readiness Judgment

This spec is **ready for implementation**.

The next widening direction is bounded and aligned with the product:

- stay project-first
- keep active analysis primary
- add review and build-readiness as attached project-local capabilities
- move the workspace closer to the real goal: analysis that meaningfully
  shapes the automated build platform
