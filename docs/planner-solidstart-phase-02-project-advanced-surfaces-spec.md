# Planner SolidStart Phase 02 Project Advanced Surfaces Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md), [Planner UI Reset Phase 06 Knowledge Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-06-knowledge-workspace-spec.md), [Planner UI Reset Phase 07 Blueprint Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-07-blueprint-workspace-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)

> Planning note (2026-03-24): Phase 01 made the Solid app project-first and
> centered active Socratic analysis. The next bounded widening slice should not
> reintroduce route sprawl. It should bring the first advanced project-local
> tools into the project workspace as secondary, hidden-by-default surfaces.
>
> Implementation sync (2026-03-24): the Solid project workspace now includes a
> closed-by-default advanced reveal with attached `Knowledge` and `Blueprint`
> tabs powered by the existing project-scoped blueprint API filter. The
> default project workspace remains analysis-first; advanced inspection opens
> locally inside the same route and switches instantly once loaded. Verification
> completed with new helper tests, Solid lint/build, and Playwright proof that
> the attached panels stay hidden until requested and do not displace the main
> analysis path.

## 1. Executive Judgment

The next SolidStart widening slice should stay inside the **project
workspace**, not expand the primary navigation again.

The user has already said the product should be:

- clear
- simple
- fast
- centered on deep Socratic analysis

That means advanced project capabilities should arrive as **attached project
surfaces**, not as co-equal top-level destinations.

The selected Phase 02 slice is:

- project-local advanced reveal model
- knowledge as the first inventory-style advanced surface
- blueprint as the first structural advanced surface
- both reachable from the project workspace without displacing the primary
  analysis task

## 2. User Outcome

After Phase 02:

- the project workspace remains the obvious home for active work
- advanced project tools are available when needed, but hidden by default
- users can inspect project knowledge and project structure without leaving the
  project context
- active analysis stays visually dominant even when advanced surfaces are
  opened
- the app feels more capable without becoming noisier

## 3. Locked Decisions

- the next widening slice stays **inside** `/projects/:projectSlug`
- advanced surfaces remain secondary to active analysis
- Knowledge and Blueprint are the first advanced project-local surfaces to land
- these surfaces must default closed or visually demoted
- they may use tabs, segmented controls, drawers, or attached lower panels
- they must not become equal-weight dashboard columns or top-level route
  clutter
- route count must stay simpler than the React-era app

## 4. Scope

### In Scope

- project workspace advanced-reveal model
- project-local knowledge surface
- project-local blueprint surface
- attached navigation/disclosure model for those surfaces
- local caching/loading behavior needed to keep those reveals fast

### Out Of Scope

- full standalone Knowledge route migration
- full standalone Blueprint route migration
- Discovery, Events, or Admin migration
- full import diagnostics migration
- auth or deployment changes

## 5. Product Problem

Phase 01 correctly centered projects and active analysis, but the advanced
system still has a gap:

- important project tools exist conceptually
- they are not yet available in the Solid workspace
- or they would require reintroducing broader route clutter to expose them

That would push the product back toward the old failure mode:

- too many peer destinations
- not enough focus

The next slice should solve that by making advanced tools **attached to the
project**, not detached from it.

## 6. Phase 02 Product Model

The default project workspace remains:

- project identity
- current analysis state
- continue/start analysis path
- recent session context

Phase 02 adds an explicit advanced reveal band below that primary surface.

Selected advanced surfaces:

- `Knowledge`
  - project-scoped inventory/context
- `Blueprint`
  - project-scoped structure/canvas summary

These should feel like:

- "show me more about this project"

not:

- "leave the project workspace and enter a different app mode"

## 7. Advanced Reveal Contract

The project workspace must provide a secondary reveal model such as:

- collapsed advanced tray
- secondary segmented switch
- attached lower workspace region
- side panel with explicit toggle

The chosen implementation must obey:

- closed by default
- zero interference with the main "continue analysis" path
- local-fast open/close for already-known data
- clear distinction between primary task and secondary inspection

Disallowed:

- always-open secondary dashboard columns
- huge tab bars that overpower the workspace
- route transitions that make advanced inspection feel remote

## 8. Knowledge Surface Contract

The first advanced inventory surface should be project-scoped knowledge.

It should provide:

- compact project knowledge summary
- visible inventory list or inventory preview
- attached detail for the selected item if needed

It must not:

- turn the project workspace into a generic library page
- compete visually with active analysis
- expose every filter/control by default on first open

## 9. Blueprint Surface Contract

The first structural advanced surface should be project-scoped blueprint.

It should provide:

- compact blueprint presence/health summary
- a first useful structural view
- selection/detail as an attached supporting surface if needed

It must not:

- become a full graph theater in this slice
- displace the project workspace as the main project home
- require broad command chrome to be visible by default

## 10. Local-Speed Contract

This slice continues the local-speed rule:

- opening an advanced surface should be immediate when its project data is
  already loaded
- switching between Knowledge and Blueprint inside the project workspace should
  not feel like navigation churn
- active analysis context must remain stable while advanced surfaces open or
  close
- data refresh should reconcile in the background where truthful

## 11. Visual-Clarity Contract

The advanced surfaces must remain quieter than the primary project workspace.

Rules:

- denser than route-level dashboard cards
- lower visual emphasis than the project hero and continue-analysis path
- no large explanatory boilerplate
- no repeated section headers that restate the obvious
- clear bordered or pane-based grouping so advanced inspection reads as a tool,
  not a new home screen

## 12. Testing Contract

Phase 02 should extend the Solid verification surface with:

- unit/component tests for advanced reveal state and project-local switching
- browser verification that advanced tools stay hidden until requested
- browser verification that project -> advanced surface -> return to active
  analysis feels local and stable

## 13. Acceptance Criteria

This slice is complete only when:

1. the project workspace still centers active analysis by default
2. project-local advanced tools are available without expanding primary route
   clutter
3. Knowledge is available as a hidden-by-default project-local advanced surface
4. Blueprint is available as a hidden-by-default project-local advanced surface
5. opening advanced surfaces does not displace or confuse the primary analysis
   path
6. advanced-surface open/close behavior feels immediate on local data
7. browser verification proves the intended primary-versus-secondary hierarchy

## 14. Verification Plan

### Unit / component

- advanced reveal state tests
- project-local Knowledge surface tests
- project-local Blueprint surface tests

### Browser

- open project workspace
- open Knowledge
- switch to Blueprint
- return attention to active analysis
- verify advanced surfaces remain secondary and hidden by default

### Build

- Solid app build succeeds
- Rust server handoff continues to serve the widened route set

## 15. Rollback / Fallback

If both advanced surfaces are too large for one bounded slice, the truthful
fallback is:

- land the shared advanced reveal framework
- land Knowledge first
- land Blueprint immediately after as the next bounded child slice

Disallowed fallback:

- making Knowledge or Blueprint top-level primary navigation just because it is
  simpler to expose

## 16. Open Questions

These do not block readiness:

- should Knowledge or Blueprint be the default first advanced tab when the
  reveal opens?
- should advanced reveal state persist per project, or always reopen closed?
- does project-local Discovery review belong in the same reveal family, or stay
  deferred to a later slice?

## 17. Readiness Judgment

This spec is **implemented**.

Delivered outcomes:

- the project workspace remains analysis-first by default
- Knowledge is available as a hidden-by-default attached project-local surface
- Blueprint is available as a hidden-by-default attached project-local surface
- advanced open/close and tab switching remain local inside the project route
- no new primary route clutter was introduced to expose the advanced tools
