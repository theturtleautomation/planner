# Planner SolidStart Phase 01 Projects And Guided Work Entry Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 00 Shell, Sessions, And Socratic Anchor Spec](/home/thetu/planner/docs/planner-solidstart-phase-00-shell-sessions-and-socratic-anchor-spec.md), [Planner UI Reset Phase 02 Projects Directory Spec](/home/thetu/planner/docs/planner-ui-reset-phase-02-projects-directory-spec.md), [Planner UI Reset Phase 03 Project Workspace Spec](/home/thetu/planner/docs/planner-ui-reset-phase-03-project-workspace-spec.md), [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)

> Planning note (2026-03-24): after Phase 00 proved the Solid shell, sessions
> queue, and Socratic anchor route, the next widening slice should not chase
> broad parity. The user explicitly prioritized clarity, simplicity, local
> speed, hidden-by-default advanced controls, and a workflow centered on deep
> Socratic analysis of an idea that ultimately shapes the automated build
> platform.
>
> Implementation sync (2026-03-24): the Solid app now exposes a guided work
> entry root, a project-first `/projects` directory, lightweight
> `/projects/new` creation, and a primary `/projects/:projectSlug` workspace
> that centers current analysis and hides advanced detail behind disclosure.
> The widened slice reorders the shell around work and projects, while keeping
> `/sessions` as a secondary route. Verification completed with Solid unit
> tests, lint/build, and Playwright proof for the root -> project -> session
> flow plus hidden-advanced/default behavior.

## 1. Executive Judgment

The next SolidStart slice should widen into the **projects and work-entry
family**, but it must not simply port the React-era route tree.

The current product still reads too much like an inventory of surfaces:

- sessions
- projects
- blueprint
- knowledge
- discovery
- admin

That is not the right first impression for Planner's actual purpose.

The product's main job is:

- take an idea
- run deep Socratic analysis on it
- shape that analysis into the build platform's working truth

The next phase should therefore simplify the route model around that outcome:

- make projects the primary work container
- make the root entry point a guided work-entry surface
- make "continue analysis" and "start new analysis" obvious
- keep advanced route families available but visually demoted or hidden until
  needed

## 2. User Outcome

After Phase 01:

- the app no longer feels sessions-first or route-inventory-first
- `/` behaves like a calm work-entry surface, not a generic shell placeholder
- `/projects` becomes the primary directory for active work
- `/projects/:projectSlug` becomes the dominant project work surface
- users can quickly tell how to:
  - start a new project
  - continue active Socratic analysis
  - resume the most relevant project work
- advanced surfaces remain reachable but do not crowd the default experience
- the flow feels local, direct, and visually simple

## 3. Locked Decisions

- the next widening slice is the **projects route family**
- the root route may be rethought and simplified
- the product should become **project-first**, not sessions-first
- visual clarity and operational simplicity outrank parity with the React route
  map
- advanced capabilities should remain available but hidden behind secondary
  navigation, disclosure, or attached panels
- the primary project workspace should emphasize active Socratic analysis and
  immediate next work, not equal-weight exposure of every subsystem
- backend and route contracts may change to support the simpler shape

## 4. Scope

### In Scope

- SolidStart root work-entry surface
- SolidStart `/projects` route
- SolidStart project creation entry flow if needed for the clearer work-entry
  model
- SolidStart `/projects/:projectSlug` primary project workspace
- project-local routing or secondary navigation needed to expose advanced tools
  without crowding the primary workspace
- route simplification needed to make projects and Socratic analysis the
  obvious product center

### Out Of Scope

- full parity for Blueprint, Knowledge, Discovery, Events, or Admin routes
- final auth model
- full import-history migration if it would crowd this slice
- unrelated backend refactors that do not support project-first work entry

## 5. Product Problem

Phase 00 proved the shell and the Socratic anchor, but it still leaves the app
too close to a sessions-first mental model:

- `/` is still a generic shell entry
- `/sessions` remains the clearest work surface
- projects are not yet the obvious primary container
- advanced product areas are not yet intentionally tucked behind a simpler main
  path

That is misaligned with the intended product story.

The user should not have to think:

- "Which route family am I supposed to use?"

They should instead feel:

- "Open the project, continue the analysis, and shape the build."

## 6. Phase 01 Route Model

The selected route family for this slice is:

- `/`
  - guided work-entry surface
- `/projects`
  - active projects directory
- `/projects/new`
  - lightweight project creation path if the work-entry surface does not embed
    creation directly
- `/projects/:projectSlug`
  - primary project workspace

Allowed simplifications:

- `/` may route users directly into project selection or recent work rather
  than acting as a generic landing page
- `/sessions` may become a secondary route rather than the primary way into
  work
- project-local tabs may be reduced, hidden, or collapsed into "advanced"
  reveals if that improves clarity
- project creation may move into the directory or root entry if that shortens
  the path to real work

## 7. Work-Entry Contract

The root entry surface must answer only a few high-value questions:

- what should I work on now?
- how do I start a new project or analysis?
- where is my most recent or most urgent work?

It must not become:

- a dashboard of competing metrics
- a giant navigation catalog
- a route index for every Planner subsystem

Required characteristics:

- one dominant next action
- one compact recent-work list or project list
- one clear way to create or continue
- no advanced route clutter above the fold

## 8. Projects Directory Contract

`/projects` becomes the primary directory for Planner work.

The route should feel like an operating directory, not a dashboard:

- dense rows
- one clear primary action per project
- freshness and active-analysis status readable at a glance
- quick distinction between active, quiet, and blocked projects

The directory must emphasize current work, not archival breadth.

Default view should favor:

- active projects
- recent project work
- projects with resumable or active Socratic sessions

Advanced or quieter data should be secondary:

- archive state
- detailed import history
- low-value metadata

## 9. Project Workspace Contract

`/projects/:projectSlug` must become the true project work surface.

Its purpose is to make the current project truth legible quickly:

- what idea is being shaped?
- what analysis is active?
- what is the next meaningful move?
- what advanced artifacts exist if needed?

### Primary workspace behavior

The primary surface should emphasize:

- current project identity
- the active or most relevant Socratic analysis path
- recent session history only as supporting context
- one obvious "continue" or "start analysis" path

### Advanced content behavior

Advanced items must be available but hidden by default.

Examples:

- knowledge
- blueprint
- discovery review
- event history
- import diagnostics

Allowed patterns:

- secondary nav tucked below the primary work header
- collapsed side inspector
- explicit "Advanced" reveal
- attached lower panel that is closed by default

Disallowed patterns:

- equal-weight multi-panel dashboards
- permanently visible low-signal technical surfaces
- top-level clutter that competes with active analysis

## 10. Local-Speed Contract

This slice must continue the local-speed rule from Phase 00.

Requirements:

- project switching is immediate for already-known project data
- entering a project workspace feels synchronous
- opening a banked Socratic thread from inside a project does not wait on a
  route spinner when local state already exists
- advanced panels/reveals must not block the main project workspace from
  feeling fast
- the UI must prefer local cached project/session state and reconcile in the
  background where truthful

## 11. Visual-Clarity Contract

This slice must aggressively reduce visual noise.

Rules:

- no giant editorial headings
- no summary-card mosaics
- no dashboard rows competing with the actual work object
- no repeated explanatory prose where a terse label is enough
- hierarchy should come from spacing, alignment, and weight rather than volume

The visual center should be:

- the project as the work container
- the active Socratic/build-shaping path as the primary task

## 12. Data / Runtime Contract

This phase may reshape backend contracts to support a cleaner project-first
entry model.

The frontend should be able to load:

- recent projects
- active/resumable Socratic sessions per project
- enough project summary state to render the work-entry and directory views
  without route thrash

The project workspace should be able to derive:

- most relevant session
- resumable analysis path
- whether advanced artifacts exist
- whether the next action is analysis, review, or quiet maintenance

## 13. Testing Contract

Phase 01 should extend the SolidStart test stack established in Phase 00:

- unit/component: Vitest
- browser: Playwright

Minimum proof required:

- `/` behaves as a guided work-entry surface rather than a generic placeholder
- `/projects` renders a dense project directory with one clear primary action
- `/projects/:projectSlug` prioritizes active work over advanced clutter
- advanced surfaces are reachable but hidden by default
- project-to-workspace navigation feels immediate on already-known data

## 14. Acceptance Criteria

This slice is complete only when:

1. the root route is simplified into a guided work-entry surface
2. projects become the primary visible work container in the Solid app
3. `/projects` reads as an active work directory, not a dashboard
4. `/projects/:projectSlug` makes active analysis or the next meaningful move
   obvious within a few seconds
5. advanced project surfaces remain available but hidden by default
6. the user can continue active Socratic analysis without being routed through
   avoidable route clutter
7. the new flow feels simpler and faster than the current sessions-first shell
8. browser verification proves the intended work-entry and project-workflow
   path

## 15. Verification Plan

### Unit / component

- root work-entry route tests
- project directory tests
- project workspace tests for primary-surface selection and advanced reveal
  hiding

### Browser

- `/` to `/projects` to `/projects/:projectSlug`
- continue active analysis from the project workspace
- hidden advanced surfaces stay out of the way until requested
- project switching and project-local work opening feel immediate on local data

### Build

- Solid app build succeeds
- Rust server handoff continues to serve the widened route set

## 16. Rollback / Fallback

If the full project workspace proves too large for one bounded slice, the
truthful fallback is:

- land the root work-entry plus `/projects` directory first
- keep `/projects/:projectSlug` narrower around "continue analysis" and recent
  sessions
- defer deeper project-local advanced reveals to the next slice

Disallowed fallback:

- reintroducing a sessions-first shell because it is already implemented
- exposing every advanced route by default just to preserve parity

## 17. Open Questions

These do not block readiness, but they should be closed during delivery:

- should project creation live on `/projects/new`, inline in `/projects`, or
  directly on `/`?
- should `/sessions` remain user-visible in primary nav or move behind the
  project-first flow?
- which advanced project area should be the first revealed secondary surface:
  knowledge, blueprint, or discovery review?

## 18. Readiness Judgment

This spec is **implemented**.

Delivered outcomes:

- project-first work entry is now the dominant SolidStart posture
- `/projects` and `/projects/:projectSlug` are real routes in the active app
- project creation is lightweight and routes directly into the project
  workspace
- active Socratic analysis is the primary project task surface
- advanced detail is hidden by default behind an attached reveal instead of
  crowding the workspace
