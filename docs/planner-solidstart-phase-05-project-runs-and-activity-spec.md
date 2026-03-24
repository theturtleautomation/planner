# Planner SolidStart Phase 05 Project Runs And Activity Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 04 Project Build Path And Automation Handoff Spec](/home/thetu/planner/docs/planner-solidstart-phase-04-project-build-path-and-automation-handoff-spec.md), [Planner UI Reset Phase 09 Events Timeline Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-09-events-timeline-workspace-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)

> Planning note (2026-03-24): after Phase 04 made the build path explicit, the
> next bounded widening slice should make project-local activity and build/run
> progress inspectable without expanding route clutter. The next project-local
> capability family is an attached runs/activity surface.
>
> Implementation sync (2026-03-24): the Solid project workspace now includes an
> attached `Activity` surface inside the same hidden-by-default reveal family.
> It derives a concise project-local stream from active sessions, import state,
> queued Socratic follow-ups, and build-path posture, keeping operational
> visibility attached to the project without introducing a separate monitoring
> route. Verification completed with helper tests, Solid lint/build, and
> Playwright proof that Activity remains secondary to active analysis.

## 1. Executive Judgment

The next SolidStart widening slice should add a **project-local runs/activity**
surface inside the project workspace.

The project route now lets the user:

- start and continue analysis
- inspect knowledge and blueprint
- review pending reconciliation
- understand build readiness
- understand the build handoff

The next gap is operational visibility:

- what changed recently
- what build/run work is active, blocked, or complete
- how the project is moving after handoff

Phase 05 should solve that inside the project workspace and keep it secondary
to active analysis.

## 2. User Outcome

After Phase 05:

- the user can inspect recent project activity without leaving the project
  route
- build/run state is visible as a project-local attached surface
- recent events feel concise and operational, not like a log dump
- the project workspace becomes a stable home for analysis, review, handoff,
  and near-term execution visibility

## 3. Locked Decisions

- the next widening slice stays inside `/projects/:projectSlug`
- runs/activity remains a hidden-by-default attached surface
- active analysis stays the primary default view
- no top-level events or runs destination is introduced in this phase
- the surface is about concise project-local visibility, not full admin ops

## 4. Scope

### In Scope

- attached `Activity` or `Runs` surface inside the project workspace
- concise recent-event timeline or run list
- build/run state summaries when available
- local-fast switching among attached project surfaces

### Out Of Scope

- full admin operations migration
- full deployment dashboard
- long-form historical audit views
- auth or deployment changes

## 5. Product Problem

The workspace is now strong from analysis through build handoff, but there is
still a visibility gap immediately after that point:

- users need a local project view of what is happening
- they should not have to leave the project route to inspect recent movement
- they should not have to parse raw logs to understand the project state

## 6. Product Model

The default project workspace remains analysis-first.

Phase 05 adds one more attached project-local surface:

- `Activity` or `Runs`
  - concise recent events
  - concise build/run state
  - project-local progress visibility

This surface should feel like:

- "what is happening in this project right now?"

not:

- "open another monitoring app"

## 7. Testing Contract

Phase 05 should extend the Solid verification surface with:

- helper tests for run/activity grouping
- browser proof that the surface is hidden by default
- browser proof that recent activity remains secondary to active analysis

## 8. Acceptance Criteria

This slice is complete only when:

1. the project workspace still centers active analysis by default
2. the new runs/activity surface is hidden by default and attached locally
3. recent project movement is understandable without parsing logs
4. the surface does not reintroduce route clutter
5. browser verification proves the intended primary-versus-secondary hierarchy

## 9. Readiness Judgment

This spec is **ready for implementation**.

The next bounded move remains consistent:

- stay project-first
- keep analysis primary
- widen the project route with concise local execution visibility
