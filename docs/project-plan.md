# Project Plan

**Status:** Active  
**Date:** 2026-03-22

## Purpose

This is a lightweight project planning index for the Planner repo.

It is not meant to replace feature-specific planning docs. It exists to give a
simple top-level view of:

- the main planning documents already in this repo
- the current active planning thread
- the next expected move before implementation

## Current Planning Spine

These are the main planning documents currently shaping the repo:

- [Project-First UI Research Sessions](/home/thetu/planner/docs/project-first-ui-research-sessions.md)
- [Phase 00 Project Ownership Implementation](/home/thetu/planner/docs/phase-00-project-ownership-implementation.md)
- [Phase 01 Root Landing And Navigation Implementation](/home/thetu/planner/docs/phase-01-root-landing-implementation.md)
- [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)
- [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md)
- [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)
- [Phase 09 Socratic Recursive Category Synthesis Spec](/home/thetu/planner/docs/phase-09-socratic-recursive-category-synthesis-spec.md)
- [Phase 10 Socratic Category Status And Refresh Spec](/home/thetu/planner/docs/phase-10-socratic-category-status-and-refresh-spec.md)
- [Phase 11 Socratic Category Replay And Validation Spec](/home/thetu/planner/docs/phase-11-socratic-category-replay-and-validation-spec.md)
- [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)
- [Planner Design System Phase 1 Tonal Foundation Spec](/home/thetu/planner/docs/planner-design-system-phase-1-tonal-foundation-spec.md)
- [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md)
- [Planner Design System Phase 3 Overlay Depth And Restrained Glass Spec](/home/thetu/planner/docs/planner-design-system-phase-3-overlay-depth-and-restrained-glass-spec.md)
- [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md)
- [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)
- [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)
- [Planner Design System Phase 7 Knowledge Inventory And Context Spec](/home/thetu/planner/docs/planner-design-system-phase-7-knowledge-inventory-and-context-spec.md)
- [Planner Design System Phase 8 Blueprint Command Chrome And Inspector Spec](/home/thetu/planner/docs/planner-design-system-phase-8-blueprint-command-chrome-and-inspector-spec.md)
- [Knowledge Library Project Scope Plan](/home/thetu/planner/docs/knowledge-library-project-scope-plan.md)
- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)
- [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)
- [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md)
- [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)
- [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)
- [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md)
- [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md)
- [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md)
- [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md)
- [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md)
- [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md)
- [Import Existing Project Phase 11 Selective Apply Merge Controls Spec](/home/thetu/planner/docs/import-existing-project-phase-11-selective-apply-merge-controls-spec.md)
- [Import Existing Project Phase 12 Historical Applied Restore For Review Spec](/home/thetu/planner/docs/import-existing-project-phase-12-historical-applied-restore-for-review-spec.md)
- [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md)
- [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md)
- [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md)
- [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md)
- [Planning Status Audit Remediation Spec](/home/thetu/planner/docs/planning-status-audit-remediation-spec.md)

## Current Active Thread

### Socratic Category Drill-Down

Goal:

- replace the flat multi-area Socratic lobby batch with a category-first intake
  flow
- let users enter a category, answer scoped prompts, and explicitly return to a
  refreshed main category screen
- preserve dimension-based convergence and draft review as the hidden truth
  model behind the new navigation layer

Canonical planning doc:

- [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)

Supporting research doc:

- [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md)

Current completed slice:

- prompt-envelope based Socratic intake is implemented and verified in
  [Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md)
- category-driven interview navigation is implemented and verified in
  [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)
- recursive category synthesis and deep breadcrumb navigation are implemented
  and verified in
  [Phase 09 Socratic Recursive Category Synthesis Spec](/home/thetu/planner/docs/phase-09-socratic-recursive-category-synthesis-spec.md)
- category status semantics, refresh cues, and build-gating explanation are
  implemented and verified in
  [Phase 10 Socratic Category Status And Refresh Spec](/home/thetu/planner/docs/phase-10-socratic-category-status-and-refresh-spec.md)
- replay hardening, stale-revision refresh, and main-screen-only build
  completion are implemented and verified in
  [Phase 11 Socratic Category Replay And Validation Spec](/home/thetu/planner/docs/phase-11-socratic-category-replay-and-validation-spec.md)

Current ready follow-on specs:

- none queued yet

Current agreed product constraints:

- categories are synthesized server-side from the current interview state
- clients render the latest category snapshot and do not invent hierarchy
- recursive category paths are now supported beyond one root and one leaf level
- users explicitly choose when to return to the main category screen
- build/start remains valid only from the main category screen when the
  underlying belief state is build-ready
- category snapshots now carry server-authored status, new-category, and
  build-guidance metadata for the current screen
- draft review remains a separate later prompt flow, not a category
- replay and validation hardening are now part of the implemented category flow

Current next bounded slice:

- no additional bounded Socratic follow-on spec is queued yet; the next move is
  either manual confidence verification of the live lobby or a new spec for the
  next product change

### Planner Visual System Refresh

Goal:

- restyle the Planner React SPA into a calmer command-center visual system
  grounded in tonal layering instead of border-heavy chrome
- establish a transferable surface, spacing, and hierarchy foundation without
  turning the work into an unbounded full-app redesign
- phase typography, CTA treatment, and overlay depth so performance-sensitive
  surfaces stay protected

Canonical planning doc:

- [Planner Design System Command Center Plan](/home/thetu/planner/docs/planner-design-system-command-center-plan.md)

Current completed slice:

- tonal shell and border-removal foundation is implemented and verified in
  [Planner Design System Phase 1 Tonal Foundation Spec](/home/thetu/planner/docs/planner-design-system-phase-1-tonal-foundation-spec.md)
- editorial typography, CTA hierarchy, and in-scope empty-state refinement are
  implemented and verified in
  [Planner Design System Phase 2 Editorial Typography And CTA Spec](/home/thetu/planner/docs/planner-design-system-phase-2-editorial-typography-and-cta-spec.md)
- overlay depth, restrained glass, and shared modal/drawer normalization are
  implemented and verified in
  [Planner Design System Phase 3 Overlay Depth And Restrained Glass Spec](/home/thetu/planner/docs/planner-design-system-phase-3-overlay-depth-and-restrained-glass-spec.md)
- utility-route command-center consistency cleanup is implemented and verified
  in
  [Planner Design System Phase 4 Utility Route Consistency Spec](/home/thetu/planner/docs/planner-design-system-phase-4-utility-route-consistency-spec.md)
- route hierarchy and operational density follow-on work is implemented and
  verified in
  [Planner Design System Phase 5 Route Hierarchy And Operational Density Spec](/home/thetu/planner/docs/planner-design-system-phase-5-route-hierarchy-and-operational-density-spec.md)
- operational surfaces and event-density follow-on work is implemented and
  verified in
  [Planner Design System Phase 6 Operational Surfaces And Event Density Spec](/home/thetu/planner/docs/planner-design-system-phase-6-operational-surfaces-and-event-density-spec.md)
- knowledge inventory and context follow-on work is implemented and verified in
  [Planner Design System Phase 7 Knowledge Inventory And Context Spec](/home/thetu/planner/docs/planner-design-system-phase-7-knowledge-inventory-and-context-spec.md)
- Blueprint command chrome and inspector follow-on work is implemented and
  verified in
  [Planner Design System Phase 8 Blueprint Command Chrome And Inspector Spec](/home/thetu/planner/docs/planner-design-system-phase-8-blueprint-command-chrome-and-inspector-spec.md)

Current ready slice:

- no additional visual-system follow-on slice is queued yet

Current agreed product constraints:

- phase 1 stays focused on tonal sectioning, shell treatment, and border removal
  across the highest-value shared frontend surfaces
- phase 2 kept display typography and CTA emphasis scoped to high-traffic
  shell, project, session-entry, and related modal/input surfaces
- phase 3 kept blur and translucency restricted to transient modal and drawer
  overlays only
- graph-heavy pages are not part of the first restyle slice

Current next bounded slice:

- no additional visual-system slice is queued; any later restyle work should be
  opened as a new bounded spec rather than extending the completed Phase 5-8
  queue

### Import Existing Project

Status:

- implemented through the currently tracked history/review/reconciliation
  slices

Canonical planning doc:

- [Import Existing Project Plan](/home/thetu/planner/docs/import-existing-project-plan.md)

Most recent completed slices include:

- queued import domain skeleton is implemented and verified in
  [Import Existing Project Phase 1 Domain Skeleton Spec](/home/thetu/planner/docs/import-existing-project-phase-1-domain-skeleton-spec.md)
- public GitHub acquisition and managed checkout are implemented and verified in
  [Import Existing Project Phase 2 GitHub Acquisition Spec](/home/thetu/planner/docs/import-existing-project-phase-2-github-acquisition-spec.md)
- project-scoped import draft generation and seeded-session handoff are
  implemented and verified in
  [Import Existing Project Phase 3 Analysis Draft And Socratic Handoff Spec](/home/thetu/planner/docs/import-existing-project-phase-3-analysis-draft-and-socratic-handoff-spec.md)
- explicit review/apply promotion into canonical project blueprint knowledge is
  implemented and verified in
  [Import Existing Project Phase 4 Review Apply Spec](/home/thetu/planner/docs/import-existing-project-phase-4-review-apply-spec.md)
- local repo provider parity is implemented and verified in
  [Import Existing Project Phase 5 Local Provider Spec](/home/thetu/planner/docs/import-existing-project-phase-5-local-provider-spec.md)
- re-import, duplicate-source protection, and import-owned lifecycle cleanup
  are implemented and verified in
  [Import Existing Project Phase 6 Reimport And Lifecycle Cleanup Spec](/home/thetu/planner/docs/import-existing-project-phase-6-reimport-and-lifecycle-cleanup-spec.md)
- project-scoped import history and lightweight draft-vs-last-applied diffing
  are implemented and verified in
  [Import Existing Project Phase 7 History And Draft Diff Spec](/home/thetu/planner/docs/import-existing-project-phase-7-history-and-draft-diff-spec.md)
- apply-time canonical reconciliation for import-owned project-local blueprint
  state is implemented and verified in
  [Import Existing Project Phase 8 Canonical Reconciliation Spec](/home/thetu/planner/docs/import-existing-project-phase-8-canonical-reconciliation-spec.md)
- project-scoped restore to a historical applied import is implemented and
  verified in
  [Import Existing Project Phase 9 Historical Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-9-historical-restore-spec.md)
- reopening an older historical `review_pending` draft into the current review
  slot is implemented and verified in
  [Import Existing Project Phase 10 Historical Review Draft Restore Spec](/home/thetu/planner/docs/import-existing-project-phase-10-historical-review-draft-restore-spec.md)
- selective apply merge controls on the current review draft are implemented
  and verified in
  [Import Existing Project Phase 11 Selective Apply Merge Controls Spec](/home/thetu/planner/docs/import-existing-project-phase-11-selective-apply-merge-controls-spec.md)
- historical applied import restore-for-review is implemented and verified in
  [Import Existing Project Phase 12 Historical Applied Restore For Review Spec](/home/thetu/planner/docs/import-existing-project-phase-12-historical-applied-restore-for-review-spec.md)
- selected-entry historical comparison is implemented and verified in
  [Import Existing Project Phase 13 Historical Entry Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-13-historical-entry-comparison-spec.md)
- arbitrary two-entry historical comparison is implemented and verified in
  [Import Existing Project Phase 14 Arbitrary History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-14-arbitrary-history-comparison-spec.md)
- selection-aware history comparison is implemented and verified in
  [Import Existing Project Phase 15 Selection-Aware History Comparison Spec](/home/thetu/planner/docs/import-existing-project-phase-15-selection-aware-history-comparison-spec.md)
- history selection summaries are implemented and verified in
  [Import Existing Project Phase 16 History Selection Summary Spec](/home/thetu/planner/docs/import-existing-project-phase-16-history-selection-summary-spec.md)

## Immediate Bounded Closeout Slice

The remaining planning-status drift surfaced by the 2026-03-19 audit is now
closed in:

- [Planning Status Audit Remediation Spec](/home/thetu/planner/docs/planning-status-audit-remediation-spec.md)

This was a cross-cutting documentation and verification slice. It did not
replace the active product thread.

## Current Manual Verification Checkpoint

If manual product verification is resumed for the active Socratic thread, the
next bounded check should be:

- open the Socratic lobby
- verify the main category list renders before a scoped prompt batch
- enter a deep category path, answer at least one prompt, then return with
  `Back`
- confirm that the refreshed main category list and build gating update from
  the latest interview state

This is a manual confidence check only. It does not create a new implementation
slice by itself.

## Working Rule

Keep planning and implementation aligned to artifact state:

1. Define or update the feature plan.
2. Turn the feature into a concrete spec/backlog item.
3. Only implement once the relevant spec is ready.
4. After implementation, review against the spec and sync docs.

## Next Expected Move

For the design-system thread, the initial four-phase command-center refresh and
the bounded follow-on queue are implemented and verified through Phase 8.

The next move is:

- treat any further visual work as a fresh bounded spec rather than reopening
  the completed design-system queue
- keep the manual Socratic lobby confidence check as the current non-coding
  verification checkpoint before promoting any new follow-on slice

If Socratic work resumes in parallel, keep using the live lobby verification
checkpoint above or add a new bounded Socratic spec rather than reopening the
completed Phase 11 slice.

Use
[Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)
plus
[Phase 07 Socratic Prompt Protocol Redesign Implementation](/home/thetu/planner/docs/phase-07-socratic-prompt-protocol-redesign-implementation.md)
plus
[Phase 09 Socratic Recursive Category Synthesis Spec](/home/thetu/planner/docs/phase-09-socratic-recursive-category-synthesis-spec.md)
as the source planning spine for later Socratic follow-on work.
