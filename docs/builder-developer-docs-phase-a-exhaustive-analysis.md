# Builder Developer Docs Phase A Exhaustive Analysis

**Status:** draft  
**Date:** 2026-03-30  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md), [Project Plan](/home/thetu/planner/docs/project-plan.md)  

## Purpose

Convert Builder's developer documentation into a repo-grounded interaction map
for Planner.

This note is intentionally exhaustive for Phase A. It does not attempt to
implement the whole Builder improvement plan. It answers a narrower planning
question:

- what Builder surfaces are actually documented for developers
- how those surfaces fit together for a local-repo Fusion workflow
- which config files and APIs are officially described
- what is still undocumented or operationally unclear for Planner

## Scope Of This Phase A Pass

This pass focuses on the Builder developer surfaces most relevant to Planner:

1. Fusion Projects developer workflow
2. local-repo launch and configuration files
3. AI instruction/config surfaces used by Builder code generation
4. Desktop App runtime and privacy controls
5. Builder CMS MCP and Builder DSI MCP
6. documented Builder APIs

This pass does **not** try to prove every Builder UI behavior, nor does it
claim that undocumented internal endpoints are stable integration contracts.

## Source Set

Primary source index:

- [Builder Developer Docs](https://www.builder.io/c/docs/developers)

Projects and local-repo workflow:

- [Fusion Projects Overview](https://www.builder.io/c/docs/fusion-projects-overview)
- [Project Setup Overview](https://www.builder.io/c/docs/fusion-projects-setup-overview)
- [Connect a local repo to Projects](https://www.builder.io/c/docs/projects-local-repo)
- [Projects Configuration Files](https://www.builder.io/c/docs/projects-configuration-files)
- [Builder CLI API](https://www.builder.io/c/docs/builder-cli-api)
- [Project settings](https://www.builder.io/c/docs/fusion-project-settings)
- [Multiple repositories in Projects](https://www.builder.io/c/docs/multiple-repositories-in-projects)
- [Projects best practices](https://www.builder.io/c/docs/projects-best-practices)
- [Project previews](https://www.builder.io/c/docs/projects-previews)
- [Projects dashboard](https://www.builder.io/c/docs/projects-dashboard)
- [Connect with VS Code Extension](https://www.builder.io/c/docs/projects-vscode)

AI instruction and code-generation configuration:

- [AGENTS.md](https://www.builder.io/c/docs/agents-md)
- [Builder Rules](https://www.builder.io/c/docs/configuration-builder-rules)
- [Agent Skills](https://www.builder.io/c/docs/skills)
- [Subagents](https://www.builder.io/c/docs/subagents)
- [AI Instruction Best Practices](https://www.builder.io/c/docs/ai-instruction-best-practices)

Desktop runtime and privacy:

- [Execution environments](https://www.builder.io/c/docs/desktop-app-execution-environments)
- [Set host requirements](https://www.builder.io/c/docs/desktop-app-set-host-requirements)
- [Privacy mode](https://www.builder.io/c/docs/privacy-mode)

MCP and design system surfaces:

- [Builder CMS MCP server](https://www.builder.io/c/docs/mcp-builder-server)
- [Builder DSI MCP](https://www.builder.io/c/docs/builder-dsi-mcp)
- [Design System Indexing](https://www.builder.io/c/docs/component-indexing)
- [Builder Design System Intelligence overview](https://www.builder.io/c/docs/fusion-design-system-intelligence)

Documented APIs:

- [API Intro](https://www.builder.io/c/docs/api-intro)
- [Admin API](https://www.builder.io/c/docs/admin-api)
- [Write API](https://www.builder.io/c/docs/write-api)

## Executive Summary

The Builder docs describe a coherent developer stack, but it is split across
multiple surfaces:

1. **Fusion Projects**
   - local repo connection
   - launch and repo indexing
   - project settings
   - Builder-controlled code-generation workflow
2. **Project instruction/config files**
   - `builder.config.json`
   - `AGENTS.md`
   - `.builderrules`
   - `.builder/rules/*`
   - skills and subagents
3. **Desktop runtime controls**
   - execution environment
   - host requirements
   - privacy mode
4. **MCP surfaces**
   - Builder CMS MCP for Publish/Hybrid content work
   - Builder DSI MCP for design-system-aware coding
5. **Documented HTTP APIs**
   - Content API
   - Admin API
   - Write API
   - Upload, Image, GraphQL Content APIs

For Planner, the main conclusion is:

- the **supported** Builder interaction model is local-repo launch plus
  config/instruction files plus documented CMS APIs/MCP
- Builder's docs clearly describe **Project settings semantics**
- Builder's docs do **not** clearly establish a documented public API for
  **existing Fusion project CRUD/list/get/update by project ID**

That last point remains the main integration gap we hit in practice.

## Builder Surface Model

### 1. Fusion Projects is the primary app-development surface

Builder's developer docs position Fusion Projects as the app-facing workflow
for local repositories, generated code, branch flows, previews, and repo-aware
development.

For Planner, this means Fusion should be treated as the main Builder surface
for:

- connecting the local repository
- launching Builder against the Planner runtime
- indexing the repo or design system
- applying Builder-specific workspace and AI instruction configuration

It should **not** be conflated with:

- Builder Publish CMS content management
- Builder DSI design-system documentation
- Builder CMS MCP content mutation

### 2. Publish/CMS is a separate documented API and MCP surface

Builder's API intro documents the Content API, HTML API, GraphQL Content API,
Write API, Upload API, Admin API, and Image API.

The docs make a clean distinction:

- **Admin API** is the documented GraphQL administrative surface for trusted
  back-end or partner use
- **Write API** is the documented HTTP mutation surface for Builder content
  entries
- **Builder CMS MCP** is a content-oriented MCP surface for Publish and Hybrid
  spaces

This means CMS/Admin/MCP work should be described as documented Builder
content-management behavior, not as Fusion project management.

### 3. DSI is a separate design-system surface

Builder DSI MCP and the design-system indexing docs position DSI as the design
system and token/component context layer for AI-assisted code generation.

For Planner, DSI is relevant when:

- Builder needs design-system documentation
- we want code generation aligned with a shared component library
- we want to improve design-system-aware generation rather than project runtime
  config

DSI is not the right surface for Fusion project lifecycle management.

## Supported Local Repo Interaction Model

The local-repo docs and CLI docs point to a supported interaction model with
three main pieces.

### A. Launch a local repository through Builder CLI

The Builder CLI docs explicitly document:

- `auth`
- `auth status`
- `index-repo`
- `launch`

The local-repo docs also document `launch --serverUrl <url>` as a supported
way to connect Builder to an already running dev server.

For Planner, this is the strongest documented match for the actual app shape:

- run `planner-server`
- point Builder at the live same-origin app URL
- use Builder launch semantics instead of inventing a separate unsupported
  runtime contract

### B. Encode repo behavior in `builder.config.json`

The local-repo docs and config-file docs describe `builder.config.json` as the
main per-project Builder configuration file.

The extracted example includes these fields:

- `command`
- `serverUrl`
- `commitMode`
- `workspace`
- `allowedCommands`
- `repoIndexingConfig`

The local-repo docs also show `workspace.folders` and `allowedCommands` as
first-class configuration concepts.

Planner implication:

- `builder.config.json` should likely become the canonical documented Builder
  config surface for Planner's local repo behavior
- our current wrapper scripts are useful, but they are repo-native helpers, not
  a replacement for the documented Builder config file model

### C. Encode AI behavior in instruction files, not only in prompts

Builder documents multiple project-side AI instruction surfaces:

- `AGENTS.md`
- `.builderrules`
- `.builder/rules/*`
- skills
- subagents

These are not all interchangeable.

Builder's docs consistently treat them as structured, repo-committed guidance
for code generation and agent behavior.

Planner implication:

- if we want Builder to behave consistently, we should prefer committed
  configuration files over relying on manual prompts or ad hoc chat context

## Configuration Files And Instruction Surfaces

### `builder.config.json`

Documented purpose:

- launch/runtime config
- server URL
- allowed shell command boundaries
- workspace folder layout
- repo indexing preferences
- commit behavior

Practical Planner use:

- declare the canonical Planner runtime
- constrain Builder shell access to a truthful allowlist
- encode multi-folder workspace context where relevant
- express repo indexing preferences explicitly

Important implication:

- Builder's own docs imply this file should be preferred over hidden or purely
  UI-managed local settings when a repo wants durable, reviewable behavior

### `AGENTS.md`

Builder documents `AGENTS.md` as a project instruction file that can describe:

- dev environment
- setup/install/run commands
- testing instructions
- PR creation workflow
- conventions and expectations

The docs also note that `AGENTS.md` can exist in multiple directories and
should be actionable, focused, precise, and scoped.

Planner implication:

- our repo already has a strong `AGENTS.md`
- Builder should be aligned with that file rather than configured through a
  parallel undocumented instruction channel
- if Builder-specific guidance differs from general repo guidance, the docs
  suggest using scoped files rather than making one global file overbroad

### `.builderrules` and `.builder/rules/*`

Builder's config docs describe `.builderrules` as Builder-specific AI
instruction files, including directory-scoped variants.

The docs explicitly show:

- root `.builderrules`
- nested `.builderrules` in subdirectories
- segmented architecture guidance through directory scoping

Planner implication:

- `.builderrules` is likely the most Builder-native place to encode
  repo-specific Fusion coding instructions that should not leak into all other
  tools
- we should use it for Builder-specific expectations instead of stuffing all
  Builder behavior into wrapper scripts

### Skills

Builder's Agent Skills docs frame skills as reusable, named capability units
with a clear purpose and examples.

The docs emphasize:

- descriptive names
- narrow, purposeful scope
- explicit instructions
- examples

Planner implication:

- the repo can mirror this by treating Builder-facing skill docs as modular
  units instead of one monolithic Builder prompt
- our current `builder-workflow` skill is useful, but Builder's own guidance
  suggests additional narrowing may improve long-term clarity

### Subagents

Builder's docs expose subagents as a first-class advanced code-generation and
delegation surface.

Planner implication:

- if we want Builder to perform larger or multi-stage repo tasks reliably, we
  should assume its best path is structured delegation through documented
  subagent/skill/instruction mechanisms, not one oversized root prompt

### AI Instruction Best Practices

Builder's best-practices doc emphasizes:

- define safety and permissions
- provide project structure hints
- make instructions actionable and scoped
- use the available file types strategically

Planner implication:

- this directly supports our repo's current preference for explicit,
  filesystem-backed configuration
- it also supports a future Builder-specific allowlist/approval model encoded
  in config rather than inferred from chat

## Project Settings Semantics

Builder's Project settings docs are strong on **settings vocabulary** even
though they are weak on **documented project CRUD APIs**.

The settings docs describe categories including:

- installation/runtime dependencies
- development command
- environment variables
- execution environment
- additional repositories
- workspace instructions
- validation command
- browser automation
- app subpath
- main branch
- commit behavior

For Planner, this is important because it means we can speak precisely about
what a Fusion project should contain:

- install command
- dev command
- runtime URL or connected server
- `PLANNER_LLM_MOCK` profile value
- execution environment
- additional repos if needed
- branch and commit behavior

This is a strong documented semantics layer, but not a strong documented
remote-management API layer.

## Desktop Runtime And Privacy Controls

### Execution environments

Builder documents execution environments as a real project-level concern and
ties them to Desktop App usage.

The host-requirements docs explicitly say host requirements apply to projects
configured to run on a **Local Machine**.

Planner implication:

- Builder expects some projects to run locally, not only in Builder-managed
  cloud execution
- this aligns well with Planner's same-origin local runtime model

### Host requirements

Builder documents host requirements as checks for tools installed on the host
platform and ties them to local-machine execution.

Planner implication:

- if we want Builder to launch Planner reliably on developer machines, we
  should likely encode expected host requirements for:
  - Rust toolchain
  - Node/npm
  - any repo-specific launch prerequisites

### Privacy mode

Builder documents a `privacyMode` configuration interface including:

- `encrypt`
- `encryptKey`
- `redactUserMessages`
- `redactLLMMessages`
- `mcpServers`

The docs also show privacy mode as project configuration plus Desktop App UI
settings.

Planner implication:

- this is the documented place to reason about code/privacy constraints in
  Builder itself
- if Planner wants stricter Builder behavior, the supported path is likely
  privacy mode plus instruction/config files, not ad hoc “please don't send
  code” guidance

## MCP Surfaces

### Builder CMS MCP

The Builder CMS MCP docs are very explicit:

- it connects AI tools to **Publish** spaces
- it uses MCP client config with bearer auth through a **Private API Key**
- it is content-oriented
- it is not a Fusion project-management transport

The docs explicitly note that the Builder CMS MCP server only connects to
**Publish and Hybrid** spaces.

Planner implication:

- our repo-local Builder CMS plugin is aligned with a documented Builder
  surface
- it should continue to be treated as CMS/content tooling
- it should not be treated as evidence that Fusion project configuration is
  remotely manageable through the same channel

### Builder DSI MCP

The Builder DSI MCP docs state:

- enterprise plans
- requires a Builder account
- requires Node.js v20+
- setup uses `npx @builder.io/dev-tools@latest dsi-mcp`
- config can live in a global client file or a repo-local `.mcp.json`

Planner implication:

- DSI MCP is a supported way to bring Builder's design-system intelligence into
  coding assistants
- repo-local DSI setup is plausible and documented
- DSI remains separate from Fusion project lifecycle and CMS content mutation

### Design system indexing

The design-system indexing docs tie Builder's DSI and repo indexing work to
the `index-repo` and design-system configuration path.

Planner implication:

- repo indexing and DSI are part of the supported Builder workflow
- when Builder indexing fails, we should think in terms of documented indexing
  and DSI surfaces, not in terms of hidden project CRUD APIs

## Documented API Surface

Builder's API intro clearly documents these families:

- Content API
- HTML API
- Write API
- GraphQL Content API
- Upload API
- Admin API
- Image API

### Admin API

Builder's docs describe the Admin API as a GraphQL API for:

- administrative tasks
- back-end servers
- trusted partners

Planner implication:

- when we need space/model/folder/asset administration, this is the documented
  server-side API family to prefer

### Write API

Builder's docs describe the Write API as HTTP mutation endpoints using:

- `POST`
- `PATCH`
- `PUT`
- `DELETE`

and explicitly position it as separate from the Admin API.

Planner implication:

- content-entry mutation is documented and supported
- it is not evidence of a documented Fusion project settings mutation API

## Explicit Documentation Gaps

After this Phase A pass, the most important unresolved gap remains the same:

### No clearly documented existing Fusion project CRUD API found

Across the analyzed developer docs, I did **not** find a clearly documented
public API contract for:

- listing Fusion projects by space
- reading a Fusion project by project ID
- updating Fusion project settings by project ID
- mutating Fusion project env vars by project ID

This does **not** prove Builder has no such internal endpoint.

It does mean:

- Planner should not treat the current inferred `/projects` endpoints as a
  documented public integration contract
- any helper that uses them must continue to label them as internal fallback
- our trouble reading back freshly created projects is consistent with the docs
  gap, not just with operator error

### The docs are stronger on settings semantics than on remote management

Builder documents:

- what project settings mean
- how to configure repo-local Builder behavior
- how to launch and index repos
- how to use CMS APIs and MCP

Builder does **not** equivalently document:

- how to manage existing Fusion projects programmatically after creation

That asymmetry matters for Planner.

## Implications For Planner

### What we should treat as supported

1. `planner-server` as the canonical launched runtime for Builder local work
2. `launch` and `--serverUrl` as the documented attach/connect path
3. `builder.config.json` as the canonical durable Builder project config file
4. `AGENTS.md`, `.builderrules`, and Builder skill files as the canonical AI
   instruction surfaces
5. Builder CMS MCP only for Publish/Hybrid content work
6. Builder DSI MCP only for design-system-aware development
7. Admin API and Write API only for documented content/admin use cases

### What we should stop overclaiming

1. that existing Fusion project read/update/list is a documented public API
2. that Builder CMS sync implies Fusion runtime sync
3. that Builder CMS MCP is a project-management surface
4. that ad hoc wrapper scripts are enough without aligning to
   `builder.config.json` and committed instruction files

### What our current work suggests

The recreated-project problem we hit is consistent with Builder's documentation
shape:

- project creation appears possible
- project settings vocabulary is documented
- repo-local config and launch are documented
- project readback/update by ID is not clearly documented

This means the safest Planner strategy is:

1. maximize repo-local Builder config
2. maximize create-time project settings
3. keep internal-fallback helpers narrow and explicitly labeled
4. avoid pretending remote project readback is a supported guaranteed contract

## Recommended Follow-On Work

### Phase B: Repo alignment to documented Builder config

Highest-value next move:

- add a repo-local `builder.config.json`
- encode the canonical Planner runtime there
- encode the Builder shell allowlist there
- encode workspace folder structure there if needed
- align local launch docs and scripts to that file

### Phase C: Builder instruction surface hardening

- decide what belongs in `AGENTS.md`
- decide what belongs in `.builderrules`
- add scoped Builder rules where Planner's directory structure justifies it
- document why Builder-specific instruction files exist separately from general
  repo instructions

### Phase D: Internal-fallback minimization

- keep `builder-get-project`, `builder-list-projects`, and
  `builder-update-project` labeled as internal fallback
- prefer create-time profile/env injection over post-create remote mutation
- add diagnostics instead of broadening unsupported assumptions

### Phase E: CMS/DSI split hardening

- keep Builder CMS MCP as a content-only surface
- evaluate whether repo-local DSI MCP config is worth adding
- tighten docs so Fusion, CMS, and DSI are never presented as one system

## Planner-Specific Decisions Emerging From Phase A

These are the strongest repo-level conclusions from this analysis:

1. Planner should treat `builder.config.json` as the next most important
   documented Builder artifact to add.
2. Planner should treat Builder AI instruction files as first-class repo
   configuration, not optional polish.
3. Planner should continue to treat existing Fusion project CRUD/readback as an
   internal fallback area until Builder documents a supported public contract.
4. Planner should continue to separate:
   - Fusion runtime/project behavior
   - CMS content behavior
   - DSI design-system behavior
5. Planner should prefer create-time project settings and local config truth
   over post-hoc remote project mutation when Builder readback is unreliable.

## Open Questions After Phase A

1. Which documented `builder.config.json` fields should Planner commit now,
   versus leaving to local overrides?
2. Should Planner add a Builder-specific `.builderrules` file at the repo root,
   or should the repo keep all instruction gravity in `AGENTS.md`?
3. Should Planner add repo-local Builder DSI MCP config, or keep DSI user-level
   until there is stronger evidence of value?
4. Can Builder's documented config files cover enough of our workflow that the
   existing internal-fallback project-update helpers become low-value?

## Proposed Next Move

The next bounded and high-confidence move after this Phase A pass is:

- create a follow-on planning slice for documented Builder repo configuration
  alignment centered on:
  - `builder.config.json`
  - Builder instruction files
  - launch/index/runtime alignment
  - explicit separation from undocumented project CRUD assumptions

That is the cleanest path to improving Builder reliability without pretending
that the missing existing-project API gap has already been solved.
