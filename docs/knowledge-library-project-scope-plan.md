# Knowledge Library Project Scope Plan

This document defines the product plan for evolving the Knowledge Library from
a global architecture graph into a project-scoped knowledge hub.

The emphasis is goal and result, not implementation detail. The next step after
this document is to break the work into data model, API, UX, and delivery
epics.

## Objective

Turn Knowledge Library into a project-scoped knowledge hub so users enter from
the project they are working on, stay inside that context by default, and only
expand to global knowledge intentionally.

## Current Problem

The current library is global and type-first:

- users land in one mixed library view
- the primary navigation model is node type, not project context
- there is no first-class project scope
- there is no scoped landing experience
- there is no durable filter model beyond local text search and type tabs
- deep links from other product surfaces cannot preserve working context

This creates a mismatch for feature-oriented work. If a user is building a task
tracker widget, they should not need to enter the library and scan unrelated
knowledge across every project.

## Target Result

A user working on a task tracker widget should be able to:

- open Knowledge Library and immediately see the correct software project
- view only the knowledge tied to that project by default
- narrow further to the widget, feature, artifact, or component they are
  working on
- broaden scope deliberately when they want related or shared knowledge
- create new knowledge that inherits the correct scope automatically

## Product Principles

- Project-first, global-second.
- Scope must always be visible.
- Context should be inherited automatically where possible.
- Shared knowledge should be reusable but clearly labeled.
- Scoped knowledge should be manageable, portable, and branchable without
  leaving project context.
- Inventory and rationale should be separated in the UI, not mixed in one flat
  global view.
- Unscoped knowledge should be an exception that is visible and managed.
- Software projects are scope containers and entry points, not archiveable,
  copyable, or branchable knowledge records.

## Management Scope Decisions

- Archive, restore, export, and duplicate or branch actions apply to knowledge
  records and scoped knowledge views.
- Archived knowledge is hidden by default inside project views and can be
  revealed intentionally.
- Export supports both single-record export and current scoped-view export with
  active project and filter context preserved.
- Duplicate or branch preserves project scope and any active secondary context
  by default, with lineage back to the source record or scoped slice.
- Software project cards and project routes remain navigation and grouping
  constructs, not managed knowledge objects.

## Product Shape

### Primary entry points

- `/knowledge`
  - project landing page with software project cards
- `/knowledge/projects/:projectId`
  - project-scoped knowledge view
- `/knowledge/projects/:projectId?...`
  - deep-linked contextual view with preselected feature, widget, artifact, or
    component filters
- `/knowledge/all`
  - explicit global view for cross-project exploration

### User journey

1. User enters Knowledge Library from nav or from a project surface.
2. If entering from nav, user sees project cards first.
3. If entering from a project surface, user lands directly inside that project.
4. User can refine scope with persistent filters.
5. User can inspect, create, or edit knowledge without losing project context.
6. User can intentionally broaden to global knowledge if needed.

## Success Outcomes

- Users reach relevant project knowledge in 1-2 clicks.
- Most library visits begin in a project-scoped view, not a global view.
- New knowledge created from a project surface inherits scope automatically.
- Unscoped knowledge becomes a managed exception, not the default state.
- The library reads as project knowledge first and architecture rationale second.
- Users can archive, export, or branch scoped knowledge without falling back to
  a separate global management surface.

## Phase Plan

## Phase 1: Establish The Scoping Model

### Goal

Make project scope a first-class part of the knowledge model.

### Result

Every knowledge record can answer:

- which software project it belongs to
- whether it is shared or project-specific
- what narrower working context it belongs to, if any

### Deliverables

- Add a primary project scope field to all knowledge records.
- Add optional secondary scope references for:
  - feature
  - widget
  - artifact
  - component
- Define explicit scope classes:
  - global
  - project
  - project-contextual
- Define an explicit `unscoped` state for legacy or ambiguous records.
- Define the rules for shared knowledge:
  - linked into projects
  - inherited into project views
  - visually distinct from project-local records

### Done when

- A record can be assigned to a project without inference.
- A record can be shown as shared, project-local, or unscoped.
- Scope can be filtered and displayed consistently.

## Phase 2: Redesign The Landing Page Around Projects

### Goal

Make the first screen a project chooser, not a global mixed table.

### Result

Users start with software projects and drill into relevant knowledge from there.

### Deliverables

- Replace the global landing table with project cards.
- Each project card should show:
  - project name
  - short description
  - owner or owning team
  - total knowledge count
  - counts by major knowledge category
  - stale item count
  - last activity
  - a health or completeness summary
- Add search, sort, and favorites.
- Add an explicit `All Knowledge` entry point for cross-project browsing.

### Done when

- A user can identify the correct project from the landing page without opening
  the full library table.

## Phase 3: Build A Scoped Project Knowledge View

### Goal

Keep users firmly inside project context once they enter a project.

### Result

The main working view for the library becomes a project page, not a global node
table.

### Deliverables

- Add a persistent project scope header.
- Show the active project and active filters at all times.
- Add persistent filter chips for:
  - knowledge type
  - widget
  - feature
  - artifact
  - component
  - tag
  - owner
  - status or lifecycle
  - stale
  - orphan
  - documentation presence
  - updated date
- Add clear actions:
  - clear filters
  - reset to project scope
  - broaden to all project knowledge
  - open global view
  - archive selected knowledge
  - restore archived knowledge
  - export the current scoped view
  - duplicate or branch a record or scoped subset

### Done when

- Users never wonder whether they are seeing project-scoped or global knowledge.
- Filters persist while moving around inside the same project context.
- Management actions on records or scoped views preserve the active project
  context.

## Phase 4: Add Contextual Deep Links From Product Surfaces

### Goal

Let users arrive in the library from where they are already working.

### Result

Feature work opens directly into the relevant knowledge slice.

### Deliverables

- Define deep-link parameters for:
  - project
  - feature
  - widget
  - artifact
  - component
- Add a standard `View related knowledge` action on project surfaces.
- Support back-navigation to the originating surface.
- Ensure contextual entry points prefill scope and filters instead of opening
  the generic library view.

### Done when

- Opening the library from a task tracker widget lands inside the correct
  project and relevant subset of knowledge.

## Phase 5: Refactor The Information Architecture Inside Each Project

### Goal

Make the project view read like a software knowledge hub rather than a raw
architecture graph.

### Result

Users can move from inventory to architecture rationale naturally.

### Deliverables

- Split the project page into clearer sections:
  - Overview
  - Inventory
  - Architecture
  - Quality
  - Activity
- Position content as follows:
  - Inventory:
    - services or components
    - APIs or resources
    - artifacts
    - technologies
  - Architecture:
    - decisions
    - constraints
    - patterns
  - Quality:
    - quality requirements
    - knowledge health and completeness
  - Activity:
    - recent edits
    - node history
    - review queues
    - export history and branch lineage

### Done when

- Users can browse from "what exists" to "why it exists" without cognitive
  friction.

## Phase 6: Define Shared Vs Project-Specific Knowledge Rules

### Goal

Prevent confusion between reusable standards and project-local decisions.

### Result

Users understand whether they are looking at local knowledge or inherited shared
guidance.

### Deliverables

- Define shared knowledge classes and display rules.
- Add visible badges for scope class on every record.
- Allow filtering for:
  - project only
  - inherited shared knowledge
  - all visible in scope
- Define whether shared knowledge can be overridden locally and how that is
  represented.

### Done when

- Users can tell immediately whether a technology recommendation or pattern is
  global guidance or local project knowledge.

## Phase 7: Improve Quality Signals For Scoped Knowledge

### Goal

Make scoped knowledge trustworthy and maintainable.

### Result

Project owners can see where knowledge quality is strong or decaying.

### Deliverables

- Replace approximate completeness with type-aware completeness logic.
- Define explicit lifecycle states for knowledge records:
  - active
  - archived
- Add project-level health metrics for:
  - stale records
  - orphaned records
  - missing scope
  - missing docs
  - archived records
  - recently changed knowledge
- Default project views to active records, with an explicit archived reveal
  filter.
- Add history and audit visibility for:
  - archive and restore actions
  - exports
  - duplicate or branch actions
- Label branched or copied knowledge with source lineage.
- Add review queues for:
  - unscoped records
  - stale records
  - orphan records
  - archived records pending review

### Done when

- Project owners can identify cleanup work without manually inspecting every
  record.
- Users can tell what is active, archived, exported, or branched from the same
  project-scoped working surface.

## Phase 8: Define Creation And Editing Behavior Around Scope

### Goal

Ensure new knowledge is scoped correctly at the moment of creation.

### Result

Correct scoping becomes the default behavior in normal workflows.

### Deliverables

- Creating from a project page auto-fills project scope.
- Creating from a widget or artifact page auto-fills:
  - project scope
  - secondary contextual scope
- Creating from a global entry point requires explicit scope selection unless
  the user is creating shared knowledge.
- Moving knowledge between scopes requires review and confirmation.
- Duplicating or branching a record preserves project scope and active
  contextual scope by default.
- Restoring archived knowledge or moving it between scopes requires review and
  confirmation.

### Done when

- New records stop appearing unscoped unless the user explicitly chooses that
  path.
- Branching and restore flows preserve scope rather than forcing users to
  recreate it manually.

## Phase 9: Roll Out In Small Product Slices

### Goal

Reduce adoption risk and ship useful value early.

### Result

The system improves incrementally without waiting for a single large redesign.

### Rollout sequence

- Phase 9.1 result:
  - project landing page and project routes exist
- Phase 9.2 result:
  - project-scoped filtering works
- Phase 9.3 result:
  - deep links from widgets and artifacts work
- Phase 9.4 result:
  - project IA is split into inventory, architecture, quality, and activity
- Phase 9.5 result:
  - scope governance, archive and restore flows, export, and branch flows are
    live

### Done when

- The team can ship usable slices that improve relevance without needing the
  full end-state first.

## Recommended MVP

### Goal

Solve the biggest usability failure first: users land in an overwhelming global
library instead of a project context.

### MVP result

Users start from projects and can stay scoped while browsing or creating
knowledge.

### MVP contents

- Project landing page with project cards
- Project-scoped route
- Persistent project scope header
- Basic filters for:
  - knowledge type
  - tag
  - component or artifact
  - stale
  - documentation presence
  - updated date
- Deep-link support from at least one project surface, such as the task tracker
  widget
- Explicit `All Knowledge` global route
- Project cards remain entry points only; archive, export, and branch actions
  apply to knowledge records and scoped views rather than to projects

### MVP done when

- Users no longer need to start from a cross-project table to find relevant
  knowledge for active work.

## Acceptance Criteria

- Opening Knowledge Library from the main nav shows project cards first.
- Opening the library from a task tracker widget lands in the correct project
  and relevant scoped subset.
- Every visible knowledge item shows its scope clearly.
- Global view remains available but is not the default for project work.
- Users can filter inside a project without losing project context.
- Shared knowledge is visibly distinct from project-specific knowledge.
- New records created from project context inherit scope automatically.
- Users can archive and restore knowledge records without leaving project
  context.
- Users can export either a single knowledge record or the current scoped view.
- Users can duplicate or branch knowledge while preserving project and
  contextual scope lineage.
- Project cards are not treated as archiveable or branchable knowledge objects.

## Product Metrics

- Percentage of library sessions that begin in a project-scoped view
- Median time to first relevant knowledge item
- Percentage of knowledge items with project scope assigned
- Percentage of visits that use contextual deep links
- Count of stale, orphan, and unscoped items by project
- User-reported confidence that the library matches the project they are
  working on

## Risks To Manage

- Over-scoping all knowledge and making shared knowledge hard to reuse
- Adding too many filters and recreating complexity in another form
- Treating project as the only meaningful scope when widget, artifact, or
  component context also matters
- Migrating legacy records without a clear review path for ambiguous scope
- Building a project landing page without enough summary quality to help users
  choose the correct project quickly

## Decision Summary

The Knowledge Library should evolve from:

- one global architecture table

to:

- a project-first knowledge hub with contextual entry points, scoped filtering,
  and a clear distinction between local and shared knowledge

This gives users a better default experience for active work while preserving
the value of cross-project knowledge exploration.
