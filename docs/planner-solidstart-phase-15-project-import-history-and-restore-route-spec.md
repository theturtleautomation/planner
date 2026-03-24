# Planner SolidStart Phase 15 Project Import History And Restore Route Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md), [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md), [Planner SolidStart Phase 14 Project Import Review Route Spec](/home/thetu/planner/docs/planner-solidstart-phase-14-project-import-review-route-spec.md), [Planner SolidStart Phase 16 Project Import Comparison And Selection Summary Spec](/home/thetu/planner/docs/planner-solidstart-phase-16-project-import-comparison-and-selection-summary-spec.md)

> Planning note (2026-03-24): after the live import review desk, the next
> import-family move should expose project-local import history and restore
> actions. Historical restore and restore-for-review should stay in the project
> context, not force a jump back into a generic utility surface.
>
> Implementation sync (2026-03-24): the Solid app now keeps import history,
> restore, restore-for-review, and review-draft recovery attached directly to
> `/projects/:projectSlug/import`. Historical entries are visible in the same
> project-local desk as the live import review, so the user no longer has to
> leave project context to compare or restore old imports.

## 1. Executive Judgment

The next SolidStart widening slice should add a **project-local import history
and restore** route or attached workspace.

This route should answer:

- what import history exists for the project
- which historical entry is safe to compare or restore
- whether the next action should be direct restore, restore-for-review, or
  review-draft recovery

## 2. User Outcome

After Phase 15:

- project import history is available in SolidStart
- historical entries are legible as project decisions, not raw logs
- restore and review-recovery actions remain project-local
- current pending review state still outranks historical restore actions

## 3. Locked Decisions

- this stays project-local, not a top-level history page
- restore actions remain subordinate to current pending review safety rules
- backend history and restore semantics remain unchanged in this slice

## 4. Acceptance Criteria

This slice is complete only when:

1. project import history is visible in SolidStart
2. restore-eligible entries are clear
3. restore and restore-for-review actions are available in-route
4. browser verification proves the route keeps current project context primary

## 5. Readiness Judgment

This spec is **implemented**.
