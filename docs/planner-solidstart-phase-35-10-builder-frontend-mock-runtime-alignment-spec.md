# Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.9 Backendless Mock Residual Cleanup Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-9-backendless-mock-residual-cleanup-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 operator correction after direct Builder setup/testing showed that the repo’s current Builder project/runtime guidance still steers UI-review work onto the server-backed `4174` path instead of the intended frontend-only mock runtime

## 1. Executive Judgment

Phase 35 successfully implemented the frontend-only mock runtime, but the
Builder-facing setup contract is still ambiguous enough to cause the wrong
project configuration in practice.

The route-browsing capability the user actually wants is:

- one Builder project for UI review
- one frontend-only mock runtime
- one port and env contract
- no dependency on `planner-server`
- no dependency on `PLANNER_LLM_MOCK=full_pipeline`

Today the repo still privileges the server-backed `4174` Builder path through
`builder.config.json` and the Builder wrappers, which is the correct contract
for backend-integrated Fusion work but the wrong contract for the current UI
review use case.

So the bounded follow-on is:

- align the canonical Builder UI-review project to the frontend mock runtime
- make the exact Builder settings explicit and repo-native
- verify that the top-level navigation surfaces actually work under that mode

## 2. User Outcome

After this phase:

- a repo user can create or recreate the canonical Builder UI-review project
  without guessing between `3000` and `4174`
- the Builder UI-review project uses the frontend mock runtime only:
  - URL: `http://127.0.0.1:3000`
  - command: `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid`
  - no `PLANNER_LLM_MOCK`
- the frontend mock runtime defaults to the `default` scenario without needing
  a query-string override at startup
- top-level shell navigation and the major Phase 35 route family are proven to
  work from the Builder-targeted frontend mock runtime
- the repo docs and Builder wrappers no longer imply that the canonical
  Builder UI-review project should point at `planner-server` on `4174`

## 3. Problem

Planner currently has two different runtime contracts that are both truthful in
their own context:

- server-backed Builder/Fusion work through `planner-server` on `4174`
- frontend-only route browsing through `planner-solid` mock mode on `3000`

The problem is not that both exist. The problem is that the repo’s current
Builder project setup path still defaults to the wrong one for UI-review work.

That drift has already produced concrete operator failure:

- a Builder project was created on `4174`
- `PLANNER_LLM_MOCK=full_pipeline` was treated as if it enabled the Phase 35
  frontend route-browsing mock
- top navigation expectations for `Home`, `Knowledge`, `Blueprint`, and the
  other route-family surfaces were evaluated against the wrong runtime

The repo now needs a bounded correction that makes the Builder UI-review path
singular and explicit.

## 4. Scope

### In Scope

- the canonical Builder UI-review runtime contract for the frontend mock mode
- repo-native Builder project creation/update guidance for that UI-review
  contract
- exact env, port, command, and URL settings for the UI-review project
- removal of unnecessary or misleading mock/runtime settings from that
  UI-review path
- targeted route-navigation verification for the top-level shell and main
  frontend mock route family
- planning and documentation sync so the repo reflects the corrected Builder
  contract honestly

### Out Of Scope

- reopening the implemented Phase 35 route-family work itself
- removing the server-backed `4174` path from the repo entirely
- claiming the frontend mock runtime replaces backend-integrated Fusion proof
- broad Builder CMS or DSI workflow redesign
- new product behavior unrelated to runtime alignment and route verification

## 5. Contracts

### 5.1 Canonical Builder UI-review project settings

The Builder project used for frontend mock UI review must use:

- `Dev server URL`: `http://127.0.0.1:3000`
- `Dev server command`:
  `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid`
- no `PLANNER_LLM_MOCK` environment variable

The UI-review project must not require:

- `planner-server`
- `npm run build --prefix planner-solid`
- `cargo run -p planner-server ...`
- `PLANNER_LLM_MOCK=full_pipeline`

### 5.2 Scenario startup contract

The frontend mock runtime already defaults to the `default` scenario when no
`mockScenario` query parameter is present.

Required result:

- the Builder UI-review project can start at `http://127.0.0.1:3000`
- optional deep links such as `?mockScenario=ops-history` may still be used
  for targeted review
- the startup contract must not imply that `?mockScenario=default` is required
  just to make the app work

### 5.3 Navigation truth contract

The Builder-facing frontend mock runtime must support truthful click-through
navigation for the top-level route family it claims to cover.

At minimum, verification must cover navigation from the shell into:

- `/`
- `/projects`
- `/sessions`
- `/knowledge`
- `/blueprint`
- `/events`
- `/discovery`
- `/admin`

And at least one continuity path into:

- one project workspace route
- one session workspace route
- one import route

If a route remains unsupported in frontend mock mode, the implementation must
either:

- fix the route, or
- explicitly remove/disable the misleading navigation expectation

The repo must not keep claiming route-family browseability while shipping dead
or misleading shell links.

### 5.4 Shared frontend surface contract

The frontend mock runtime must not become a mock-only UI fork.

Required architectural truth:

- Builder edits made against the frontend mock runtime must change the same
  `planner-solid` route and component surfaces that are later built and served
  by `planner-server`
- frontend mock mode may swap data providers, transport behavior, and scenario
  selection, but it must not swap in a separate page tree, alternate shell, or
  Builder-only layout implementation
- if Builder updates the shell, navigation, route layout, or workspace
  composition while using the frontend mock runtime, those changes must flow
  back into the real app automatically because the runtime is shared

What this means in practice:

- the mock layer owns data and transport seams
- the real route components remain the only design/edit target
- any Builder-oriented shortcut that would edit mock-only pages is out of scope
  for this slice

### 5.5 Repo-native Builder helper contract

The repo’s Builder project setup path for UI review must stop defaulting to the
server-backed `4174` runtime.

Acceptable bounded outcomes:

- a dedicated Builder mock-project create/update path that applies the frontend
  mock contract above, or
- a corrected default Builder project path for the current UI-review use case,
  with the server-backed path retained only as an explicit alternate workflow

What is not acceptable:

- continuing to create the canonical Builder UI-review project on `4174`
- continuing to imply that `PLANNER_LLM_MOCK=full_pipeline` is the same as the
  frontend-only Phase 35 mock mode

## 6. Product Decisions

### 6.1 Favor one Builder UI-review path

For the current user need, there should be one canonical Builder project path:
frontend mock on `3000`.

This does not erase the server-backed path from the repo. It only stops using
that path as the default answer to the wrong question.

### 6.2 Keep the server-backed path explicit but secondary

`4174` remains valid for backend-integrated Builder/Fusion work.

But the repo must distinguish it clearly as:

- server-backed integration/runtime verification

not:

- the default Builder project for Phase 35 route browsing

### 6.3 Verify navigation, not just startup

The next proof must not stop at “the page loads.”

The actual contract for this follow-on is:

- the Builder-targeted frontend mock runtime starts correctly
- the shell links work
- the route family is traversable enough for real UI review
- Builder-driven layout/design edits under mock mode still target the same
  shared frontend surfaces that `planner-server` serves later in the real app

## 7. Candidate Touched Surfaces

- [builder.config.json](/home/thetu/planner/builder.config.json) if the repo
  chooses to make frontend mock the Builder-project default for this workflow
- `scripts/builder-create-project.sh`
- `scripts/builder-update-project.sh`
- `scripts/builder-launch.sh` if the repo chooses a dedicated mock-profile
  entrypoint
- shared `planner-solid` route/component surfaces rather than mock-only page
  clones
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
- [README.md](/home/thetu/planner/README.md) if Builder guidance needs a brief
  correction there too
- targeted `planner-solid/e2e` or browser proof for shell-navigation and route
  continuity under frontend mock mode
- parent Phase 35 planning docs for honest follow-on tracking

## 8. Acceptance Criteria

1. The spec-defined Builder UI-review runtime is explicit and singular:
   `http://127.0.0.1:3000` with
   `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid`.
2. The Builder UI-review path does not require `PLANNER_LLM_MOCK`.
3. Repo-native Builder setup guidance or wrappers no longer default the
   canonical UI-review project to `4174`.
4. The frontend mock startup contract documents that the app defaults to the
   `default` scenario without requiring a query parameter.
5. Browser proof demonstrates working shell navigation into the top-level route
   family the frontend mock runtime claims to support.
6. Any remaining unsupported shell route under frontend mock mode is either
   remediated or explicitly removed from the Builder UI-review expectation.
7. The implementation documents and preserves that frontend mock mode edits the
   same `planner-solid` route/component surfaces that `planner-server` serves,
   rather than introducing a mock-only UI fork.

## 9. Verification Plan

- review the repo-native Builder setup path and confirm it produces the
  frontend mock contract above for UI-review work
- browser proof under the frontend mock runtime covering:
  - shell navigation to `/`, `/projects`, `/sessions`, `/knowledge`,
    `/blueprint`, `/events`, `/discovery`, `/admin`
  - one project workspace continuity path
  - one session workspace continuity path
  - one import continuity path
- implementation review confirming that mock-mode Builder changes flow through
  shared `planner-solid` route/component surfaces instead of mock-only view
  files
- targeted E2E/browser proof if automated verification is widened
- documentation review confirming that Builder guidance no longer conflates:
  - `VITE_PLANNER_FRONTEND_MOCK=1`
  - `PLANNER_LLM_MOCK=full_pipeline`
  - `3000`
  - `4174`

## 10. Rollback / Fallback

If the repo cannot safely change the default Builder project path in one pass:

- add a dedicated mock-specific Builder create/update workflow instead
- make that path the only documented UI-review setup
- keep the server-backed path documented as a separate advanced/integration
  workflow

If route verification finds unsupported shell links:

- do not hand-wave them as “probably fine”
- either narrow the documented route-coverage claim or create a direct bounded
  remediation list in the implementation outcome

## 11. Open Questions

1. Should the repo make the Builder UI-review project the new default in
   `builder.config.json`, or should it introduce a separate mock-specific
   Builder project config/profile?
2. Should repo-native Builder helper output explicitly print whether a project
   is using the `frontend-mock-ui-review` contract versus the server-backed
   integration contract?

## 12. Implementation Outcome

Implemented on 2026-03-30.

This slice corrected the remaining Builder-facing drift without reopening the
Phase 35 route-family implementation itself:

- the repo's default Builder config now targets the canonical frontend mock
  UI-review runtime on `3000`
- the server-backed `planner-server` path remains available through an explicit
  alternate config on `4174`
- the repo-native Builder wrappers now read the selected config profile instead
  of blindly forcing the server-backed runtime and `PLANNER_LLM_MOCK`
- frontend-mock browser proof now runs against the actual `vite dev` runtime
  and exercises shell navigation plus project/session/import continuity
- docs now state clearly that Builder-driven design work under frontend mock
  mode edits the same `planner-solid` route/component surfaces later served by
  `planner-server`

Primary implementation surfaces:

- [builder.config.json](/home/thetu/planner/builder.config.json)
- [builder.server.config.json](/home/thetu/planner/builder.server.config.json)
- [builder-launch.sh](/home/thetu/planner/scripts/builder-launch.sh)
- [builder-create-project.sh](/home/thetu/planner/scripts/builder-create-project.sh)
- [builder-update-project.sh](/home/thetu/planner/scripts/builder-update-project.sh)
- [Makefile](/home/thetu/planner/Makefile)
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
- [README.md](/home/thetu/planner/README.md)
- [playwright.frontend-mock.config.ts](/home/thetu/planner/planner-solid/playwright.frontend-mock.config.ts)
- [phase-35-frontend-mock.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-35-frontend-mock.spec.ts)

Verification evidence:

- `bash -n scripts/builder-launch.sh scripts/builder-create-project.sh scripts/builder-update-project.sh`
- `jq . builder.config.json >/dev/null && jq . builder.server.config.json >/dev/null`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
- `./scripts/builder-create-project.sh --dryrun`
- `BUILDER_PROJECT_CONFIG_PATH=./builder.server.config.json ./scripts/builder-create-project.sh --dryrun`

Resulting contract:

- canonical Builder UI-review project:
  - `http://127.0.0.1:3000`
  - `VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid -- --host 127.0.0.1 --port 3000 --strictPort`
  - no `PLANNER_LLM_MOCK`
- explicit server-backed alternate:
  - `http://127.0.0.1:4174`
  - `npm run build --prefix planner-solid && cargo run -p planner-server -- --port 4174 --static-dir ./planner-solid/dist/static`
  - optional `PLANNER_BUILDER_LLM_MOCK_MODE` mapping to `PLANNER_LLM_MOCK`

Residual note:

- `npm --prefix planner-solid run build` still emits the known Nitro `"send"`
  warning, but the build exits successfully and this slice did not widen that
  contained dependency-version limitation
