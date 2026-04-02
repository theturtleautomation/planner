# Builder Local Workflow

This document defines the truthful local Builder.io workflow for Planner.

Planner now has two distinct Builder-relevant runtimes, and they solve
different problems:

1. the canonical Builder UI-review project
2. the server-backed integration runtime

The canonical Builder project for Phase 35 route browsing is now the
frontend-only mock runtime on `3000`.

The server-backed `planner-server` runtime on `4174` still exists, but it is a
separate advanced workflow for backend-integrated verification rather than the
default Builder UI-review path.

## Workflow Split

Keep these concerns separate:

1. **Builder UI review**
   - Run `planner-solid` in frontend mock mode.
   - Point the canonical Builder project at `http://127.0.0.1:3000`.
   - Use this for layout, navigation, and route-surface design work.
2. **Server-backed integration work**
   - Build `planner-solid`.
   - Run `planner-server`.
   - Open the app on the same origin that serves `/api`.
   - Use this when backend/runtime truth matters.
3. **Builder MCP for Codex**
   - This repo ships repo-local Codex plugins for Builder CMS and Builder DSI.
   - CMS and DSI are different Builder surfaces and should stay separate in
     workflow and mental model.

## Canonical Builder UI-Review Project

The default repo-native Builder config now targets the frontend mock runtime:

- config file: [builder.config.json](/home/thetu/planner/builder.config.json)
- URL: `http://127.0.0.1:3000`
- command:
  `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort`

From the repo root:

```bash
make builder-launch
```

Or run the app directly:

```bash
VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort
```

Then open:

```text
http://127.0.0.1:3000
```

Scenario behavior:

- the app defaults to the `default` scenario without any query string
- optional deep links such as `?mockScenario=ops-history` or
  `?mockScenario=ops-attention` remain available for targeted review

Important boundaries:

- this mode is frontend-only and does not require `planner-server`
- this mode is the canonical Builder project path for UI design and
  click-through review
- this mode uses the same `planner-solid` route/component surfaces that
  `planner-server` later serves in the real app
- the mock layer changes data and transport seams only; it must not become a
  mock-only UI fork

## Server-Backed Integration Runtime

This is the explicit alternate workflow when Builder/Fusion work needs the real
same-origin frontend plus `/api`.

From the repo root:

```bash
npm run build --prefix planner-solid
cargo run -p planner-server -- --port 4174 --static-dir ./planner-solid/dist/static
```

Then open:

```text
http://127.0.0.1:4174
```

Health check:

```bash
curl http://127.0.0.1:4174/api/health
```

Notes:

- `planner-server` is the real app runtime for Builder/Fusion work.
- `./planner-solid/dist/static` must exist before starting the server.
- Port `3100` is the default server port, but any free port works as long as
  the Builder launch port and `planner-server --port` value match.
- If Auth0 environment variables are unset, Planner runs in local dev mode with
  no login gate.

Known build note:

- `npm run build --prefix planner-solid` still emits a Nitro warning about
  `"send"` not being exported by `h3/dist/_entries/node.mjs`
- current containment judgment: this matches the present SolidStart alpha /
  Nitro `h3` version split, the build still exits successfully, and the
  generated output remains usable for local Builder review

## Builder Fusion / Local Project Connection

Verify Builder CLI auth first:

```bash
npx @builder.io/dev-tools@latest auth status
```

If auth is missing, a human must run the interactive login flow:

```bash
npx @builder.io/dev-tools@latest auth login
```

Canonical Builder UI-review launch:

```bash
make builder-launch
```

This uses [builder.config.json](/home/thetu/planner/builder.config.json), which
now points at the frontend mock runtime on `3000`.

If you want to recreate the canonical Builder UI-review project:

```bash
make builder-create-project
```

If you want to sync the saved Builder project back to the canonical UI-review
runtime settings:

```bash
make builder-update-project
make builder-verify-sync
```

Server-backed alternate launch:

```bash
make builder-server-launch
```

The explicit alternate config file for that path is
[builder.server.config.json](/home/thetu/planner/builder.server.config.json).

If you need a server-backed Builder project or want to resync a saved project
to that path explicitly:

```bash
make builder-server-create-project
make builder-server-update-project
make builder-server-verify-sync
```

Server-backed mock mode values:

- `phase26_live` - mocks only the early Socratic startup/question flow
- `full_pipeline` - mocks Socratic plus the full intake/compile/review/validate/
  telemetry pipeline and swaps the factory worker to a deterministic mock

`PLANNER_BUILDER_LLM_MOCK_MODE` is only relevant to the server-backed path.

Example:

```bash
PLANNER_BUILDER_LLM_MOCK_MODE=full_pipeline make builder-server-launch
```

To opt back into live provider CLIs there:

```bash
PLANNER_BUILDER_LLM_MOCK_MODE=disabled make builder-server-launch
```

What this does:

- `make builder-launch` starts Builder against the frontend mock UI-review
  runtime on `3000`
- `make builder-server-launch` starts Builder against `planner-server` on
  `4174`

Important caveats:

- the canonical Builder UI-review project should not be configured with
  `PLANNER_LLM_MOCK=full_pipeline`
- frontend mock UI review uses `3000`; server-backed integration uses `4174`
- `planner-server` serves built static assets and does not hot-reload frontend
  code the way `vite dev` does
- if you change frontend code while using the server-backed path, rebuild
  `planner-solid` and restart `planner-server`
- `full_pipeline` remains a server-backed runtime mock, not the same thing as
  the frontend-only Phase 35 mock mode

## Documented Repo Builder Config

Planner now commits the Builder-facing repo config artifacts described in
Builder's developer docs:

- [builder.config.json](/home/thetu/planner/builder.config.json)
- [builder.server.config.json](/home/thetu/planner/builder.server.config.json)
- [.builderrules](/home/thetu/planner/.builderrules)

Current repo contract:

- `builder.config.json` is the canonical Builder UI-review config:
  - profile: frontend mock
  - runtime command
  - runtime URL
  - workspace folders
  - allowed shell commands
  - commit mode
- `builder.server.config.json` is the explicit alternate config for
  server-backed integration work
- `.builderrules` is the Builder-specific AI instruction layer for this repo

Planner's repo-native Builder wrappers now inherit runtime command and URL
defaults from the selected config file instead of hardcoding a separate runtime
contract.

That means:

- `make builder-launch`, `make builder-create-project`, and
  `make builder-update-project` now target the canonical frontend mock
  UI-review contract from `builder.config.json`
- `make builder-server-launch`, `make builder-server-create-project`, and
  `make builder-server-update-project` explicitly target the server-backed
  integration contract from `builder.server.config.json`
- `PLANNER_BUILDER_LLM_MOCK_MODE` only applies to the server-backed contract

## Builder MCP for Codex

Planner now ships a repo-local Codex plugin for Builder CMS:

- plugin manifest: `plugins/planner-builder-cms/.codex-plugin/plugin.json`
- plugin MCP config: `plugins/planner-builder-cms/.mcp.json`
- repo marketplace: `.agents/plugins/marketplace.json`

That plugin defines a repo-local HTTP MCP entry named
`planner-builder-cms` pointing at:

```text
https://cdn.builder.io/api/v1/mcp/builder-content
```

It reads authentication from:

```text
BUILDER_PRIVATE_API_KEY
```

Notes:

- This avoids depending on a user-global `builder-cms` entry in
  `~/.codex/config.toml`.
- Existing Codex sessions may need a restart before newly added repo-local
  plugins are discovered.

## Builder DSI Repo-Local Plugin

Planner now also ships a repo-local Codex plugin for Builder DSI:

- plugin manifest: `plugins/planner-builder-dsi/.codex-plugin/plugin.json`
- plugin MCP config: `plugins/planner-builder-dsi/.mcp.json`
- repo marketplace: `.agents/plugins/marketplace.json`

This MCP config runs:

```text
npx -y @builder.io/dev-tools@latest dsi-mcp
```

Use Builder DSI for:

- design-system discovery
- design-system docs
- design-aware planning
- design-system-aware implementation

Do not use Builder DSI as if it were:

- Fusion runtime/project configuration
- Builder CMS content mutation
- a requirement for all Builder work

Verify the repo-local DSI setup with:

```bash
make builder-dsi-status
```

What this check covers:

- repo-local DSI plugin files exist and parse
- `.agents/plugins/marketplace.json` advertises the local DSI plugin
- `node` and `npx` are installed
- Node.js is v20+
- `npx @builder.io/dev-tools@latest dsi-mcp --help` is runnable when
  `timeout` is available

What it does not prove:

- that an already-running Codex session has hot-discovered the new plugin
- that Builder account auth is active for interactive DSI work

## Environment Variables

Do not assume values in repo-local `.env` files are automatically loaded into
your shell or into Codex.

For Builder CMS MCP, `BUILDER_PRIVATE_API_KEY` must be exported in the shell
that launches Codex. One local option from the repo root is:

```bash
set -a
source ./.env
set +a
```

That only exports values into the current shell session. It does not make them
globally available, and it should not be treated as proof that another shell,
terminal tab, or editor session has the variable loaded.

Builder DSI does not use `BUILDER_PRIVATE_API_KEY` in the same way as the CMS
plugin. Its repo-local requirement is the `npx @builder.io/dev-tools@latest
dsi-mcp` command path plus Builder account access when you actually use the
DSI surface.

## Known Limitations

- Planner's canonical Builder path is build-first, not HMR-first.
- `planner-server` uses an explicit `--port` flag. If you change the app port,
  update both the Fusion `-p` argument and the `planner-server --port` value.
- Builder CLI auth is interactive and cannot be fully verified in a non-
  interactive repo setup.
- Repo-local Builder CMS wiring still depends on
  `BUILDER_PRIVATE_API_KEY` being exported in the shell that launches Codex.
- Repo-local Builder DSI still depends on `node`, `npx`, Node.js v20+, and a
  Codex session restart if the plugin was added after the session started.

## Codex Fallback Skill

If a Codex session does not expose usable `builder-cms` tools even though the
endpoint and API key are valid, use the global Builder skill at:

```text
/home/thetu/.codex/skills/builder-workflow
```

That skill covers:

- Builder Fusion / local runtime launch
- repo connect and index flows
- Builder CMS direct fallback via JSON-RPC
- Builder DSI mental model and references

Useful commands:

```bash
/home/thetu/.codex/skills/builder-workflow/scripts/builder-auth-status.sh
/home/thetu/.codex/skills/builder-workflow/scripts/builder-cms-rpc.sh tools
/home/thetu/.codex/skills/builder-workflow/scripts/upsert-project-entry.sh \
  --path . \
  --runtime-url http://127.0.0.1:4174 \
  --proxy-url http://127.0.0.1:48752
```

Planner also exposes a repo-native wrapper:

```bash
make builder-auth-status
make builder-print-config
make builder-validate-config
make builder-verify-sync
make builder-diagnose-project-visibility
make builder-dsi-status
make builder-launch
make builder-create-project
make builder-list-projects
make builder-get-project
make builder-update-project
make builder-server-print-config
make builder-server-validate-config
make builder-server-verify-sync
make builder-connect-repo
make builder-connect-repo-dryrun
make builder-index-repo
make builder-code
make builder-figma-generate
make builder-figma-publish
make builder-figma-migrate
make builder-sync-project
```

These targets delegate to repo-local wrapper scripts, which call the global
Builder skill scripts.

Inspection and validation:

```bash
make builder-print-config
make builder-validate-config
make builder-verify-sync
make builder-diagnose-project-visibility
make builder-server-print-config
make builder-server-validate-config
make builder-server-verify-sync
```

What these commands now tell you explicitly:

- which config file is active
- whether you are on the default frontend-mock UI-review path or the alternate
  server-backed path
- the resolved runtime URL and command
- the remote Builder project profile that `create` or `update` will use
- whether `PLANNER_BUILDER_LLM_MOCK_MODE` matters for the selected path
- whether the saved Fusion project is visible and aligned or blocked/drifted
  for the selected path
- why the saved Fusion project is blocked when visibility fails, including the
  current auth context, read-surface evidence, and visibility classification

The launch/create/update wrappers now print the same resolved contract before
doing any work, so wrapper output and docs tell the same runtime story.

The verify-sync wrappers are read-only. They compare:

- the active local config contract
- the saved project in `.codex/builder-fusion-project.json`
- the visible remote Fusion project settings when the current auth context can
  see that project

If the saved remote project is not visible, the verify-sync command reports a
truthful blocked state instead of pretending comparison succeeded.

If the saved project is visible only on Builder's branch surface, the
verification command now reports a truthful partial state instead:

- `status: visibility_partial`
- `visibility.state: branch_visible_only`

That means the saved Fusion project is still live enough for branch truth, but
the current metadata read surfaces do not expose its project settings.

Use the dedicated diagnosis target when you need the underlying evidence:

```bash
make builder-diagnose-project-visibility
```

The diagnosis output now proves:

- the current Builder CLI/env user and space
- whether `org-tree` and `projects?apiKey=...&userId=...` disagree
- whether direct project read fails with and without `userId`
- whether the Builder branch surface returns live branches for the saved
  project ID
- whether the saved project state has enough context to classify the failure as
  stale, mismatched, branch-visible-only, API-surface-specific, or still
  `undetermined`

Older saved Fusion state files may lack `spaceId` and `userId`. In that case,
the repo still reports the visibility state honestly instead of guessing stale
state or auth drift. When branch truth is available, the repo now classifies
that state as `branch_visible_only`; otherwise it may remain `undetermined`.
Future `make builder-create-project` flows persist richer saved-state context
automatically so this diagnosis is more precise going forward.

The existing-project helper targets use documented Builder Project settings
semantics with an internal fallback transport. They are intended to operate on
the saved project in `.codex/builder-fusion-project.json`, warn before remote
mutation, and avoid recreating a project during read/update flows.

If Builder does not return the saved Fusion project in the current project-list
query, the outcome now depends on the other Builder surfaces:

- if the branch surface also fails, `make builder-get-project` returns a
  truthful `not_found` payload
- if the branch surface succeeds, `make builder-get-project` returns
  `status: partial` and the repo reports the project as `branch_visible_only`
- in both cases, `make builder-update-project` still blocks live mutation when
  metadata settings remain unreadable, while still supporting `--dryrun` for
  payload inspection

Do not validate `api.builder.io` behavior by opening an API URL directly in a
browser tab. Builder serves the web-app shell there, and the in-page runtime
rewrites `api.builder.io` requests toward the main app host. Use one of these
instead:

- the browser network panel on an authenticated Builder session
- repo-native scripts such as `make builder-diagnose-project-visibility`
- authenticated direct probes from the repo shell

Pass CLI flags through `make` with `ARGS="..."`:

```bash
make builder-connect-repo-dryrun ARGS="--spaceId c302669d31c74e7fa80574973c437cfa"
make builder-create-project
make builder-index-repo ARGS="--spaceId c302669d31c74e7fa80574973c437cfa --force"
make builder-code ARGS="--url https://www.figma.com/design/... --prompt 'Implement this exactly'"
make builder-figma-publish ARGS="--spaceId c302669d31c74e7fa80574973c437cfa --dryrun --yes"
```

`make builder-create-project` persists the chosen Fusion project in:

```text
.codex/builder-fusion-project.json
```

That avoids duplicate project creation when Builder's project list API is not
returning reliable results for this auth context.

The repo wrapper now creates Fusion projects with Planner-specific defaults:

- `needSetup: false`
- `settings.installCommand = npm install --prefix planner-solid`
- `settings.setupDependencies` includes `node`, `npm`, `pnpm`, and `rust`
- `settings.devServerCommand` and `settings.devServerUrl` inherit from the
  selected Builder config file
- default config: frontend mock runtime on `3000`
- alternate config: server-backed runtime on `4174`
- `settings.mainBranchName = main`
- `settings.environmentVariables = []`

Override runtime/proxy endpoints with:

```bash
BUILDER_PROJECT_RUNTIME_URL=http://127.0.0.1:4174 \
BUILDER_PROJECT_PROXY_URL=http://127.0.0.1:48752 \
make builder-sync-project
```

This does not repair the underlying Codex-side MCP bootstrap issue. It gives
future sessions a stable Builder CMS path that works as long as
`BUILDER_PRIVATE_API_KEY` is exported.

Operational notes:

- `make builder-connect-repo` can create remote Builder Fusion project state.
- `make builder-connect-repo-dryrun` is the safer preview path, but in the
  current CLI version it can still prompt interactively for values like project
  name.
- `make builder-index-repo` can update Builder's indexed view of the repo.
- `make builder-code` expects Builder CLI arguments like `--url` and usually a
  prompt or URL-driven workflow.
- `make builder-figma-generate`, `make builder-figma-publish`, and
  `make builder-figma-migrate` are thin wrappers over the corresponding
  Builder CLI flows and may require additional interactive input or flags.
