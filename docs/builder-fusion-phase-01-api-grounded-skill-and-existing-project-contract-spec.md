# Builder Fusion Phase 01 API-Grounded Skill And Existing Project Contract Spec

**Status:** implemented  
**Date:** 2026-03-29  
**Parent Spec:** [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  

## Purpose

Define the first bounded child slice under the Builder Fusion parent capability
plan by translating Builder's published API and project-settings documentation
into a concrete skill-update contract for Planner.

This slice is intentionally narrower than the parent plan. It does not try to
solve every missing Builder capability. It establishes:

- which Builder surfaces are publicly documented and safe to treat as primary
  integration points
- which Fusion project behaviors are documented as settings semantics but not
  documented as a public CRUD API
- how the repo-local `builder-workflow` skill should expose those boundaries
- what a later implementation tranche must preserve when operating on the saved
  Fusion project in `.codex/builder-fusion-project.json`

## Problem

The current Planner Builder skill has two real gaps:

1. it does not clearly distinguish between Builder's documented APIs and the
   reverse-engineered Fusion project endpoint the repo currently uses for
   project creation
2. it does not teach the agent enough about Builder's documented Project
   settings surface to reason about remote runtime configuration in a grounded,
   field-level way

That creates avoidable ambiguity:

- the skill correctly separates Fusion, CMS, and DSI at a high level, but it
  does not yet explain that Builder documents the CMS/Admin APIs directly while
  Fusion project CRUD remains only partially documented through CLI and UI docs
- the repo already needs to manage concrete Fusion project settings such as dev
  command, environment variables, host requirements, execution environment,
  additional repositories, validation commands, browser automation, app
  subpath, main branch, and commit mode, but the skill does not yet encode
  those fields as a first-class contract
- the repo has an existing saved Fusion project
  (`ee5c85a61a1447dbae6b7c7765e80f20`), but the skill does not yet guide an
  agent on how to update that existing project safely instead of recreating
  state or confusing CMS sync with Fusion settings

## Source Analysis

The following Builder docs materially shape this child slice:

- [API Intro](https://www.builder.io/c/docs/api-intro)
- [Admin API Introduction](https://www.builder.io/c/docs/admin-api)
- [Write API](https://www.builder.io/c/docs/write-api)
- [Project Settings](https://www.builder.io/c/docs/fusion-project-settings)
- [Connect a Local Repo to Projects](https://www.builder.io/c/docs/projects-local-repo)
- [Agent Skills](https://www.builder.io/c/docs/skills)

### Builder docs confirm the documented direct API surface

- Builder documents the Admin API as a GraphQL API for administrative tasks,
  with the endpoint `https://cdn.builder.io/api/v2/admin` and authentication
  through Private API Keys.
- Builder documents the Write API for content-entry mutation via
  `PUT`/`PATCH`/`DELETE` on `https://builder.io/api/v1/write/...`, also using
  Private API Key bearer auth.

This means the Planner Builder skill should treat CMS/Admin operations as
documented API work, not as ad hoc fallback behavior.

### Builder docs confirm Fusion project settings semantics

Builder documents Project settings for:

- dependency install script
- dev command
- environment variables
- native application
- host requirements
- Fusion execution environment
- additional repositories
- workspace instructions
- design system intelligence attachment
- validation command
- browser automation
- app subpath
- main branch name
- commit mode
- branch naming behavior

This means the Planner skill can and should reason about remote Fusion project
configuration in these explicit terms instead of only saying "runtime settings."

### Builder docs confirm the supported local-launch shape

Builder documents using `launch --serverUrl <url>` to connect to an already
running server, and the local repo docs also show a config shape with
`command`, `serverUrl`, `commitMode`, `workspace.folders`, and
`allowedCommands`.

This aligns with Planner's same-origin `planner-server` runtime and means the
skill should explicitly model the distinction between:

- local Builder launch against a running Planner server
- remote Fusion project settings persisted in Builder

### Builder docs confirm how Builder wants skills structured

Builder's skill docs emphasize:

- keep skills focused
- use descriptive names
- document capabilities clearly
- include examples
- include safety instructions for destructive operations

The Planner Builder skill is broad by necessity, but this guidance still
applies: the skill should make the capability boundary explicit, especially for
operations that use undocumented or unstable endpoints.

### Important gap: no documented public Fusion project CRUD/settings API found

From the analyzed Builder docs, there is no clearly documented public API for:

- listing Fusion Projects
- reading a Fusion Project by ID
- updating Fusion Project settings by ID
- mutating Project environment variables through a documented HTTP or GraphQL
  endpoint

This is an explicit planning constraint for Planner.

The repo currently has a project-creation helper that uses
`https://api.builder.io/projects`, which appears to mirror Builder dev-tools
behavior, but this endpoint is not established by the analyzed docs as a
stable, public contract. The skill and future helper scripts must treat it as
an internal fallback, not as a documented Builder API.

## User Outcome

A Planner agent using the updated skill should be able to:

1. choose the correct Builder surface using published docs, not guesswork
2. use documented Admin and Write APIs confidently for CMS/content work
3. understand the documented Fusion Project settings vocabulary when reasoning
   about runtime config
4. distinguish local launch state from persisted remote Fusion project state
5. understand that existing-project Fusion settings updates are required for
   Planner, but currently depend on an internal fallback until Builder exposes
   or documents a supported project-management API

## Scope

### In Scope

- tightening the `builder-workflow` skill around documented API surfaces
- adding durable skill/reference guidance for documented Fusion Project
  settings fields
- defining a safe contract for existing Fusion project read/update helpers that
  operate on `.codex/builder-fusion-project.json`
- explicitly classifying undocumented Fusion project endpoints as internal
  fallback behavior
- documenting how Planner runtime profiles map onto Builder Project settings

### Out Of Scope

- implementing the full `builder-get-project.sh` or `builder-update-project.sh`
  helpers in this planning slice
- claiming that Builder publishes a stable Fusion project settings API when the
  analyzed docs do not show one
- broadening this slice into Builder CMS model/content redesign or DSI changes
- changing Planner's runtime architecture beyond the already-established local
  `planner-server` Builder path

## Implementation Slice

This spec is now implementation-ready only for the Builder skill/reference
hardening work described below.

The implementation slice for this spec includes:

- updating `/home/thetu/.codex/skills/builder-workflow/SKILL.md`
- updating `/home/thetu/.codex/skills/builder-workflow/references/capability-matrix.md`
- adding Builder API and Fusion Project settings reference docs under
  `/home/thetu/.codex/skills/builder-workflow/references/`

This spec does not authorize implementing existing-project helper scripts yet.
Those helpers remain a later slice because the public-documentation gap around
Fusion project CRUD/settings still needs either:

- explicit acceptance of the internal-endpoint risk, or
- newly documented Builder support for a stable project-management API

## Proposed Skill Update

This child slice should drive a focused update to
`/home/thetu/.codex/skills/builder-workflow/` with the following changes.

### 1. Strengthen the skill's surface model

Keep the skill focused around Builder operations, but distinguish between:

- documented Fusion CLI and settings semantics
- documented CMS/Admin APIs
- documented DSI tools
- internal Fusion project-management fallback paths

The skill should explicitly say that Fusion project read/update for an existing
project may require an internal fallback because the public docs analyzed here
do not document a supported project CRUD API.

### 2. Add a documented API reference for Builder operations

Add or update a skill reference that states:

- Admin API endpoint and auth mode
- Write API mutation model and auth mode
- when to use CMS/Admin API vs Write API vs Fusion CLI
- which operations are documented vs internal

This reference should become the canonical answer for "what can we do through a
published Builder API vs repo-local fallback?"

### 3. Add a Fusion Project settings reference

Add a skill reference that maps Builder's documented Project settings to the
Planner repo's needs, including:

- install/dependency script
- dev command
- environment variables
- native application
- host requirements
- execution environment
- additional repositories
- workspace instructions
- validation command
- browser automation
- app subpath
- main branch name
- commit mode

This reference should explicitly map Planner's runtime profiles:

- `live`
- `mock-socratic`
- `mock-full-pipeline`

onto the subset of Builder Project settings they affect.

### 4. Define the existing-project helper contract before implementation

The future helper set for existing-project management must follow these rules:

- prefer the saved project identity in `.codex/builder-fusion-project.json`
- support explicit project ID override when the saved state is stale
- never recreate a project during a read/update path
- print whether an operation used a documented Builder surface or an internal
  fallback
- make local-only launch changes vs remote-persisted project changes explicit

### 5. Add safety language and examples

The skill should include examples for:

- launching Planner locally with Builder against a running same-origin server
- updating Builder CMS content through documented API-backed workflows
- reading/updating an existing saved Fusion project with an internal-fallback
  warning

It should also require operator-facing warnings before:

- changing remote project settings
- changing commit mode
- changing main branch/base branch behavior
- switching runtime profiles on the canonical saved project

## Contracts And Touched Surfaces

### Skill Surfaces

- `/home/thetu/.codex/skills/builder-workflow/SKILL.md`
- `/home/thetu/.codex/skills/builder-workflow/references/capability-matrix.md`
- new or updated Builder API and Fusion Project settings references under
  `/home/thetu/.codex/skills/builder-workflow/references/`

### Repo Planning Surfaces

- [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md)
- [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md)
- [Project Plan](/home/thetu/planner/docs/project-plan.md)

### State Surface

- `.codex/builder-fusion-project.json`

This child slice is specifically about operating on the existing saved project
identity rather than treating Builder project creation as the normal flow.

## Acceptance Criteria

This child slice is coherent when all of the following are true:

1. the spec explicitly names the Builder docs that back the skill update
2. the spec captures the documented Admin API and Write API surfaces
3. the spec captures the documented Fusion Project settings vocabulary relevant
   to Planner
4. the spec explicitly states that a documented public Fusion project
   read/update API was not found in the analyzed docs
5. the spec defines how the `builder-workflow` skill should present documented
   capabilities vs internal fallbacks
6. the spec preserves existing-project update of the saved Fusion project as a
   required behavior in later implementation, not an optional stretch goal

## Verification Plan

Planning verification for this slice is:

1. confirm the child spec is linked from the parent Builder capability thread
2. confirm the doc index and project plan both reference the new child spec
3. confirm the child spec cites the Builder docs that justify the boundaries
4. confirm the child spec does not overclaim a documented Fusion project CRUD
   API that the analyzed docs did not establish

## Rollback / Fallback

If later implementation finds Builder has now published a supported Fusion
project management API, this child spec should be tightened rather than
discarded:

- keep the documented API classification work
- replace the "internal fallback" guidance with the newly documented contract
- preserve the existing-project and saved-project-ID safety requirements

If no such API is documented, implementation should proceed cautiously by
keeping internal-endpoint usage narrow, well-labeled, and easy to disable.

## Open Questions

1. Does Builder publish a supported Fusion project-management API anywhere
   outside the analyzed docs, or is CLI mediation the intended public contract?
2. Should Planner split the current `builder-workflow` skill into a more
   focused Fusion-project-management companion skill, or is a stronger
   capability matrix sufficient?
3. Should runtime profile application eventually update only dev/env fields, or
   also manage validation command, browser automation, and additional
   repositories for the canonical Planner project?

## Closeout Judgment

This child spec is now `implemented` for the narrow skill/reference hardening
slice.

Implemented in this slice:

- `/home/thetu/.codex/skills/builder-workflow/SKILL.md`
- `/home/thetu/.codex/skills/builder-workflow/references/capability-matrix.md`
- `/home/thetu/.codex/skills/builder-workflow/references/api-surfaces.md`
- `/home/thetu/.codex/skills/builder-workflow/references/fusion-project-settings.md`

What remains outside this completed slice:

- existing Fusion project helper scripts
- direct remote project read/update helpers for the saved project identity
- explicit implementation of internal-fallback project-management commands

The next valid move is to decide whether to:

- split existing-project helper work into a separate implementation slice, or
- accept the internal-endpoint risk directly and implement that narrower helper
  tranche
