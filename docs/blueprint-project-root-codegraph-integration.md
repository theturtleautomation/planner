# Blueprint Project Root And CodeGraph Integration

**Status:** Implemented  
**Date:** 2026-03-15

> Verification follow-up (2026-03-18):
> 1. Reconcile the factory project-identity regression test with the current
>    factory naming contract.
> 2. Re-validate that factory-emitted blueprint nodes are project-scoped and
>    attached to the project root when the pipeline supplies project identity.
> 3. Narrow the remaining limitation text so it reflects the actual gap:
>    `FactoryOutputV1` is still not self-describing, but current pipeline
>    emission already attaches factory output to the project root.

## Objective

Make the blueprint graph structurally useful without turning it into a raw code
graph.

This implementation is complete when Planner:

- treats the project itself as a first-class blueprint node
- attaches project-scoped blueprint items to that root with explicit
  membership edges
- accepts code-derived relationship hints through a reviewable proposal
  contract instead of mutating the blueprint directly
- isolates CodeGraphContext usage behind a dedicated Gemini profile so the
  normal Planner Gemini runtime remains tool-restricted

## Non-Goals

- replace the canonical project record stored by the server with the blueprint
  `project` node
- mirror raw source-level call graphs or import graphs into the blueprint
  without review
- infer semantic edges such as `affects`, `constrains`, or `satisfies` purely
  from static analysis
- make the graph visualization the primary blueprint UX again
- automate a full CGC worker loop in this phase
- redesign the full discovery review UI beyond supporting the new edge
  proposal contract

## Decision Summary

- Add a blueprint node type `project` and edge type `contains`.
- Use the `project` node as the blueprint-local root for project-scoped
  records.
- Backfill legacy stored blueprints on load so the upgrade applies to existing
  data, not only future pipeline runs.
- Keep the blueprint as curated architectural knowledge, not as a direct code
  graph.
- Introduce a separate imported edge-proposal workflow for code-graph tools.
- Allow only a narrow first-pass set of code-derived edges:
  - `contains`
  - `depends_on`
  - `uses`
- Resolve imported edge endpoints through stable selectors rather than forcing
  external tools to know concrete blueprint node IDs ahead of time.
- Install a dedicated `gemini-cgc` wrapper with its own Gemini home and
  settings so CodeGraphContext is isolated from the normal Planner Gemini
  runtime.

## Why This Exists

Before this change, many projects produced a flat blueprint:

- there was no blueprint root node representing the project itself
- many projects had zero explicit edges
- graph views degraded into disconnected cards or grid fallbacks
- code-structure tools had no clean integration point except direct blueprint
  mutation

That blurred two different concerns:

- blueprint graph: durable product and architecture knowledge
- code graph: implementation structure and static relationships

The result was a graph surface that often looked empty or arbitrary and an
integration story that encouraged polluting the blueprint with unreviewed
static analysis output.

## Current-State Summary

The implementation now separates these concerns cleanly.

| Surface | Implemented behavior | Why it matters |
| --- | --- | --- |
| Blueprint schema | includes `project` nodes and `contains` edges | every project now has a graph-local root |
| Blueprint store | backfills missing project roots and membership edges on load | legacy stored projects upgrade automatically |
| Pipeline emitter | attaches intake/spec/AR output to the project root | new pipeline output arrives already rooted |
| Discovery contract | supports imported pending edge proposals | code-graph tools can propose relationships without direct mutation |
| API | exposes edge-proposal list/import/accept/reject endpoints | review flow matches node proposals |
| Frontend types/UI | understands `project`, `contains`, and `code_graph_context` | overview, inventory, and graph surfaces stay type-safe |
| Gemini runtime | adds `gemini-cgc` isolated wrapper | CodeGraphContext can be used without weakening the standard Gemini profile |

## Architecture Model

### Blueprint layer

The blueprint remains the curated architecture graph. Nodes here represent
durable system facts such as:

- project scope
- decisions
- components
- constraints
- technologies
- quality requirements

Relationships in this layer should be semantically meaningful and stable enough
to survive beyond a single implementation detail.

### Code-graph layer

CodeGraphContext and similar tools operate on source structure:

- imports
- module dependencies
- symbol relationships
- caller/callee paths
- type hierarchies

That information is useful, but it should enter Planner as evidence or pending
structural proposals, not as automatic architecture truth.

### Boundary between the two

Good candidates for code-derived blueprint proposals:

- `project contains component`
- `component depends_on component`
- `component uses technology`

Poor candidates for automatic inference from code alone:

- `decision affects component`
- `constraint constrains component`
- `quality requirement satisfies component`

Those higher-level relationships usually require pipeline semantics or human
review.

## Schema And Data Model Changes

### Blueprint schema

`planner-schemas/src/artifacts/blueprint.rs` now defines:

- node type: `project`
- edge type: `contains`

The `project` blueprint node is not a replacement for the canonical server-side
project record. It is a graph anchor for project-scoped blueprint knowledge.

### Membership semantics

`contains` means:

- the source must be the project root node
- the target is a project-scoped blueprint node
- the relationship is about blueprint membership, not ownership of every
  server-side resource

Typical shape:

- `Project` -> `contains` -> `Decision`
- `Project` -> `contains` -> `Component`
- `Project` -> `contains` -> `Constraint`
- `Project` -> `contains` -> `Technology`
- `Project` -> `contains` -> `Quality Requirement`

### Imported edge proposal types

`planner-core/src/discovery.rs` adds:

- `ProposedEdge`
- `ProposalNodeRef`
- `ImportedEdgeProposal`
- `EdgeImportResult`
- `DiscoverySource::CodeGraphContext`

The imported edge proposal contract is intentionally narrow. Accepted imported
edge types today:

- `contains`
- `depends_on`
- `uses`

Any other edge type is rejected during validation.

## Detailed Implementation

### 1. Blueprint store backfill

`planner-core/src/blueprint.rs` now owns the persistence-side upgrade behavior.

Implemented responsibilities:

- derive a deterministic blueprint root ID from the canonical `project_id`
- detect project-scoped legacy nodes that are missing a project root
- create the missing `project` node
- attach legacy nodes to that root with `contains`
- deduplicate identical `source/target/edge_type` edges
- flush migrated state so existing stored projects stay upgraded after reload

This means the change applies to current stored data without requiring a manual
rebuild of every project.

### 2. Pipeline emission

`planner-core/src/pipeline/blueprint_emitter.rs` now ensures project-root
attachment at emission time.

Implemented paths:

- intake output
- spec output
- adversarial review output

The emitter creates or reuses the project root and then attaches emitted nodes
with `contains` wherever the source artifact has explicit project scope.

Current limitation:

- `FactoryOutputV1` does not yet carry embedded project identity, so standalone
  consumers of the artifact cannot infer project scope from the artifact alone
- current pipeline emission already supplies project identity explicitly, so
  factory-emitted blueprint records are project-scoped and attached in the
  active runtime path

### 3. Discovery edge-proposal contract

`planner-core/src/discovery.rs` adds a separate proposal store path for edges.

Persistence layout:

- node proposals: `data/blueprint/proposals.msgpack`
- edge proposals: `data/blueprint/edge_proposals.msgpack`

For system installs, these live under `/opt/planner/data/blueprint/`.

Imported edge proposals are validated before insertion:

- `confidence` must be within `0.0..=1.0`
- both endpoints must resolve against the current blueprint snapshot
- `contains` must originate from a `project` node
- duplicates are skipped

### 4. Endpoint resolution for imported edges

External code-graph tooling does not have to know raw blueprint node IDs ahead
of time.

`ProposalNodeRef` supports these endpoint selectors:

- `node_id`
- `component_origin_key`
- `technology_name`
- `project_id`

This keeps the import contract stable across reruns and lets external tools
resolve against Planner’s current blueprint state instead of hardcoding node
IDs.

### 5. Server API

`planner-server/src/api.rs` now exposes dedicated edge-proposal endpoints:

- `GET /api/blueprint/discovery/edge-proposals`
- `POST /api/blueprint/discovery/edge-proposals/import`
- `POST /api/blueprint/discovery/edge-proposals/{id}/accept`
- `POST /api/blueprint/discovery/edge-proposals/{id}/reject`

Behavior:

- `import` validates and stores pending proposals
- `accept` materializes the edge into the blueprint and marks the proposal
  merged
- `reject` marks the proposal rejected without mutating the blueprint

This mirrors the node-proposal workflow instead of bypassing it.

### 6. Frontend type and UI support

The frontend was updated so the new model is visible and safe across all major
surfaces.

Files updated include:

- `planner-web/src/types/blueprint.ts`
- `planner-web/src/api/client.ts`
- `planner-web/src/components/BlueprintOverview.tsx`
- `planner-web/src/components/BlueprintGraph.tsx`
- `planner-web/src/components/NodeDetailPanel.tsx`
- `planner-web/src/components/AddEdgeModal.tsx`
- `planner-web/src/components/CreateNodeModal.tsx`
- `planner-web/src/components/TableView.tsx`
- `planner-web/src/pages/BlueprintPage.tsx`
- `planner-web/src/pages/DiscoveryPage.tsx`
- `planner-web/src/pages/KnowledgeLibraryPage.tsx`

Implemented frontend support:

- `project` appears as a first-class node type
- `contains` appears as a supported relationship type
- the discovery source taxonomy includes `code_graph_context`
- overview and inventory surfaces can render project nodes without falling back
  to unknown-type behavior

### 7. Dedicated Gemini CodeGraph profile

`deploy/install.sh` now installs a second Gemini wrapper:

- binary: `/opt/planner/bin/gemini-cgc`
- home: `/opt/planner/cli-home/gemini-codegraph`
- settings: `/opt/planner/cli-home/gemini-codegraph/settings.json`

This profile is intentionally separate from the normal Planner Gemini runtime:

- standard Planner Gemini remains tool-restricted
- `gemini-cgc` loads only the CodeGraphContext MCP profile

The codegraph profile reuses Planner Gemini auth via symlinked OAuth files, so
it does not require a second Google login.

Optional installer inputs:

- `PLANNER_CGC_COMMAND`
- `PLANNER_CGC_NEO4J_URI`
- `PLANNER_CGC_NEO4J_USERNAME`
- `PLANNER_CGC_NEO4J_PASSWORD`

## API Contract Details

### Discovery scan task for CGC ingestion

`POST /api/blueprint/discovery/scan` now accepts scanner
`"code_graph_context"` and can import edge proposals in one step.

Request shape:

```json
{
  "scanners": ["code_graph_context"],
  "root_path": "/path/to/project"
}
```

Runtime behavior:

- scanner prefers direct `cgc` CLI access via `PLANNER_CGC_COMMAND` (or
  `/opt/planner/bin/cgc`)
- it ensures the repo is indexed, inspects indexed workspace packages, and
  derives deterministic `contains`, `depends_on`, and `uses` proposals from
  manifest/package relationships
- imported proposals are then validated and persisted as pending edge
  proposals
- `PLANNER_CGC_SCAN_COMMAND` remains as a legacy fallback only when direct CGC
  CLI access is unavailable

Operational flow:

1. Run `directory_structure` to create filesystem-backed component proposals
   with stable `path:*` origin keys.
2. Accept the relevant component proposals into the blueprint.
3. Run `code_graph_context` so CGC can map workspace packages onto those
   components and propose `contains`, `depends_on`, and `uses` edges.
4. Review and merge the pending edge proposals separately.

When `scanners` includes `"all"`, `code_graph_context` is included
automatically only when CodeGraphContext is available.

### Daily scheduler

The server supports a background daily CGC proposal-ingestion task when
CodeGraphContext is available.

Supported environment variables:

- `PLANNER_CGC_DAILY_SCAN_ENABLED` (default `true`)
- `PLANNER_CGC_DAILY_SCAN_INTERVAL_SECS` (default `86400`)
- `PLANNER_CGC_DAILY_SCAN_ROOT` (default `/opt/planner`)
- `PLANNER_CGC_DAILY_SCAN_RUN_ON_STARTUP` (default `false`)

This scheduler follows the same proposal path as manual discovery scans:
it inserts pending edge proposals and validation errors, but it does not
auto-merge any edges into the blueprint.

### Imported edge proposal payload

Example payload for `POST /api/blueprint/discovery/edge-proposals/import`:

```json
{
  "proposals": [
    {
      "edge_type": "depends_on",
      "source": {
        "component_origin_key": "path:src/review-controls"
      },
      "target": {
        "component_origin_key": "path:src/task-list"
      },
      "reason": "Review controls imports task-list reorder helpers",
      "confidence": 0.91,
      "metadata": "module dependency from CGC",
      "source_artifact": "codegraph:task-widget"
    }
  ]
}
```

Expected result shape:

```json
{
  "inserted": 1,
  "skipped": 0,
  "errors": []
}
```

### Accept/reject semantics

- accept:
  - edge is added to the blueprint if both nodes still exist
  - proposal status becomes `merged`
- reject:
  - proposal status becomes `rejected`
  - blueprint remains unchanged

## Validation Completed

The shipped implementation was validated with:

- `cargo test -p planner-core discovery -- --nocapture`
- `cargo test -p planner-core emit_from_spec_rerun_uses_origin_key_and_preserves_manual_component_names -- --nocapture`
- `cargo test -p planner-core store_persist_and_reload -- --nocapture`
- `cargo test -p planner-server test_create_blueprint_node -- --nocapture`
- `cargo test -p planner-tui --no-run`
- `npm --prefix planner-web test -- BlueprintPage DiscoveryPage`
- `npm --prefix planner-web run build`
- `bash -n deploy/install.sh`
- `sudo ./deploy/install.sh --update`

Live verification also confirmed that existing project data was backfilled with
one `project` node and `contains` edges for existing scoped nodes.

## Current Limitations

- Daily scheduling currently runs at a fixed interval from server start rather
  than at a wall-clock UTC time.
- Imported relationships are still limited to `contains`, `depends_on`, and
  `uses`.
- CGC edge import requires accepted filesystem-backed blueprint components
  (`path:*` origin keys). If the blueprint only contains spec/factory-generated
  components, the scanner now returns an explicit diagnostic rather than
  silently yielding zero usable edges.
- Factory output does not yet attach to the project root because project scope
  is missing in the current artifact payload.
- The relationships view is still secondary. It becomes useful only when the
  blueprint has enough explicit edges to justify graph exploration.
- This phase documents the dedicated `gemini-cgc` wrapper, but it does not yet
  define a model-routing policy that auto-selects it inside the Planner
  pipeline.

## Implementation Progress

- Discovery UI now supports a unified review workflow for both node proposals
  and edge proposals in one page, including accept/reject actions for pending
  edges.
- Discovery scan now supports scanner `code_graph_context`, which runs a
  direct CGC CLI path first and imports pending edge proposals automatically
  via the existing import contract.
- Server now supports a daily background CGC proposal loop using configurable
  scan interval and root, with no auto-merge step.

## Recommended Next Steps

1. Define scoring and filtering heuristics for CGC-derived edges before they
   enter the proposal queue.
2. Add project scope to factory output if factory-emitted blueprint items
   should attach directly to the project root.
3. Revisit whether additional edge types deserve import support once the first
   three have enough production usage.

## Code Anchors

- `planner-schemas/src/artifacts/blueprint.rs`
- `planner-core/src/blueprint.rs`
- `planner-core/src/discovery.rs`
- `planner-core/src/pipeline/blueprint_emitter.rs`
- `planner-server/src/api.rs`
- `planner-web/src/types/blueprint.ts`
- `planner-web/src/api/client.ts`
- `planner-web/src/components/BlueprintOverview.tsx`
- `planner-web/src/components/BlueprintGraph.tsx`
- `planner-web/src/pages/BlueprintPage.tsx`
- `planner-web/src/pages/DiscoveryPage.tsx`
- `deploy/install.sh`
- `deploy/planner.env`
