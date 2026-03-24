# Planner SolidStart Phase 06 Project Build Execution And Run Detail Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 05 Project Runs And Activity Spec](/home/thetu/planner/docs/planner-solidstart-phase-05-project-runs-and-activity-spec.md), [Planner SolidStart Phase 04 Project Build Path And Automation Handoff Spec](/home/thetu/planner/docs/planner-solidstart-phase-04-project-build-path-and-automation-handoff-spec.md)

> Planning note (2026-03-24): after Phase 05 attached project-local activity,
> the next bounded move should turn build handoff into real project-local build
> execution visibility. The next slice is not a global ops dashboard. It is a
> compact project-local build execution surface.
>
> Implementation sync (2026-03-24): the Solid project workspace now includes an
> attached `Build execution` surface that combines the active project session,
> session run IDs, and pipeline event trail into a compact project-local run
> view. Execution posture, latest run, current step, and recent pipeline events
> are now visible without leaving the project workspace. Verification completed
> with helper tests, Solid lint/build, and Playwright proof that the execution
> surface remains secondary to active analysis.

## 1. Executive Judgment

The next SolidStart widening slice should add a **project-local build
execution** surface inside the project workspace.

The workspace now reaches:

- analysis
- review
- readiness
- build handoff
- recent activity

The next operational gap is explicit run execution detail:

- what build is active
- what step it is in
- whether it succeeded or failed
- what the user should do next

## 2. User Outcome

After Phase 06:

- the project workspace can show the active or latest build run locally
- users can inspect run posture without leaving the project route
- execution status stays concise and readable
- the app still avoids route clutter and admin-panel sprawl

## 3. Locked Decisions

- the next widening slice stays inside `/projects/:projectSlug`
- build execution remains an attached secondary surface
- active analysis stays the primary default view
- the surface is project-local, not a global operations route

## 4. Scope

### In Scope

- attached build execution surface
- active/latest run summary
- concise step/state rendering
- clear next action on success/failure/in-progress

### Out Of Scope

- full admin operations migration
- deep deployment tooling
- multi-project run orchestration

## 5. Acceptance Criteria

This slice is complete only when:

1. the project workspace still centers active analysis by default
2. build execution detail is hidden by default and attached locally
3. active/latest run state is understandable quickly
4. failure or blocked run state is actionable
5. browser verification proves the surface remains secondary to active analysis

## 6. Readiness Judgment

This spec is **ready for implementation**.
