# Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Planner SolidStart Phase 32 Work Entry IA And Session Route Topology Spec](/home/thetu/planner/docs/planner-solidstart-phase-32-work-entry-ia-and-session-route-topology-spec.md), [Planner SolidStart Phase 34 Session Question-Bank Workspace Reset Spec](/home/thetu/planner/docs/planner-solidstart-phase-34-session-question-bank-workspace-reset-spec.md), [Planner SolidStart Phase 35.8 Backendless Mock Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-8-backendless-mock-closeout-remediation-spec.md), [Planner SolidStart Phase 35.9 Backendless Mock Residual Cleanup Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-9-backendless-mock-residual-cleanup-spec.md), [Planner SolidStart Phase 35.10 Builder Frontend Mock Runtime Alignment Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md), [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Builder Fusion Project Management And Runtime Sync Spec](/home/thetu/planner/docs/builder-fusion-project-management-and-runtime-sync-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/lib/api.ts`, the Solid route controllers, the `planner-solid/e2e` route-interception helpers, and `planner-server/src/e2e_mock_llm.rs`

## 1. Executive Judgment

The actual requirement is simpler than a pipeline/runtime mock:

- Builder needs to open the app with a specific frontend env/profile
- that profile must expose mock data for the route family
- a designer should be able to click around manually and view the pages that
  would normally appear during a real session/project lifecycle
- this must work without depending on a functioning backend or a real pipeline

The repo already proves route feasibility through Playwright interception, but
that coverage is test-local and not usable as a manual browsing mode.

So the bounded capability for this phase is now explicit:

- add an app-owned frontend mock mode for `planner-solid`
- make that mode selectable through a documented frontend env/config contract
- let Builder use that mode to browse the main route family without
  `planner-server`

## 2. User Outcome

After this phase:

- a repo user or Builder can run `planner-solid` in a frontend mock mode and
  browse the main routes without a working backend
- `/sessions/:sessionId` still supports deterministic startup, prompt-bank, and
  draft-save behavior under mock control instead of collapsing on missing REST
  and websocket infrastructure
- `/`, `/projects`, `/projects/:projectSlug`, `/projects/:projectSlug/import`,
  `/sessions`, `/sessions/new`, `/knowledge`, `/blueprint`, `/events`,
  `/discovery`, and `/admin` can all be viewed from shared mock scenarios
- the mock mode is explicitly for route viewing, UI design, and click-through
  exploration, not backend-truth verification

## 3. Problems To Solve

- `planner-solid/src/lib/api.ts` is currently a direct `fetch` client with no
  provider seam for a frontend-owned mock mode
- the session workspace opens a real websocket directly, so the session route
  cannot be browsed backendlessly without an additional transport seam
- route fixture knowledge is scattered across Playwright specs instead of
  living in one reusable scenario registry
- the current Builder/local-workflow docs are centered on `planner-server`
  runtime truth, not frontend-only browsing for UI design

## 4. Scope

### In Scope

- a frontend-owned mock mode for `planner-solid`
- a shared mock data/scenario registry that can drive the major route family
- an API provider seam behind the current `~/lib/api` surface
- a websocket/session-transport seam for the session workspace
- route coverage for:
  - `/`
  - `/projects`
  - `/projects/new`
  - `/projects/:projectSlug`
  - `/projects/:projectSlug/import`
  - `/sessions`
  - `/sessions/new`
  - `/sessions/:sessionId`
  - `/knowledge`
  - `/blueprint`
  - `/events`
  - `/discovery`
  - `/admin`
- reuse of the shared scenarios in at least targeted frontend/browser proof

### Out Of Scope

- backend/runtime pipeline simulation unless a later slice needs it
- claiming frontend mock mode as proof of real backend truth
- inventing new product behavior not grounded in the existing API types and
  route contracts
- reworking unrelated product IA or route content as a pretext for mock-mode
  work

## 5. Product And Architecture Decisions

### 5.1 Frontend mock mode is the primary requirement

This slice should not assume a pipeline mock is required.

The primary requirement is:

- frontend-only route browsing with mock data
- manual click-through support for Builder/UI design work
- enough deterministic state to represent the pages a normal workflow would
  expose

Server-backed runtime mock may continue to exist separately, but it is not the
goal of this slice and should not shape the first implementation.

### 5.2 Add a client seam instead of more ad hoc interception

The frontend should not continue scaling route coverage through scattered
`page.route(...)` helpers alone.

Required direction:

- keep the public `~/lib/api` function surface stable where practical
- move request execution behind a provider/resolver seam
- move session transport creation behind a dedicated wrapper instead of direct
  `new WebSocket(...)`
- let mock mode resolve those seams to deterministic in-process handlers

This keeps route code stable while making mock mode reusable outside tests.

### 5.3 Drive mock mode from scenario packs

The backendless mock must not be one giant hard-coded payload.

Required direction:

- define named scenario packs that describe coherent multi-route state
- let one scenario pack provide shared project/session/import/admin/blueprint
  fixtures together
- allow route-specific deep links without forcing every page to own its own
  local fake data

Minimum scenario coverage should include:

- `default`
  - one active project, one session-ready route, normal summaries
- `empty`
  - no projects, no sessions, empty knowledge/blueprint/timeline surfaces
- `session-workspace`
  - one question-bank session with deterministic prompt-bank and draft-save
    behavior
- `import-review`
  - one project with pending import review and attached history
- `ops-attention`
  - degraded admin posture plus populated events/discovery surfaces

### 5.4 Make session behavior explicit

The session page is the hardest route because it currently needs both REST and
live transport behavior.

Required direction:

- support a mock transport layer that can simulate:
  - initial startup/open
  - prompt-bank arrival
  - prompt submission completion
  - convergence or pipeline-complete refresh points
- keep the current bank-first and saved-brief startup contract shape
- do not silently special-case the session page into a dead static screenshot

The session route should remain interactable under mock mode, not just visible.

### 5.5 Reuse the scenario registry in tests

The existing Playwright specs prove route feasibility, but they duplicate mock
payload setup across many files.

Required direction:

- frontend/browser tests should be able to reuse the shared scenario registry
  for at least the core route-family proof
- per-test overrides may remain where a spec needs narrow behavior, but the
  default route payloads should stop being copy-pasted across the suite

### 5.6 Deliver this as route-family slices, not one broad pass

This capability should be delivered as individual major route slices.

Reasoning:

- the route family already breaks naturally along distinct data contracts
- `/sessions/:sessionId` is materially more complex than the list/detail routes
- Builder/UI browsing value starts early if the primary routes land first
- later advanced surfaces can reuse the same mock provider/scenario foundation
  without blocking the first useful pass

The parent Phase 35 artifact should therefore remain the umbrella capability
spec, and implementation should promote child route slices under it.

## 6. Planned Child Slices

### Slice 35.1: [Shared frontend mock foundation](/home/thetu/planner/docs/planner-solidstart-phase-35-1-shared-frontend-mock-foundation-spec.md)

Status:

- implemented

Purpose:

- create the provider seam behind `~/lib/api`
- define frontend mock activation
- define the shared scenario registry and route-fixture ownership model

Primary touched surfaces:

- `planner-solid/src/lib/api.ts`
- new frontend mock provider/runtime modules
- frontend mock activation/config wiring

Why this comes first:

- every route slice depends on the same provider/scenario contract
- this slice should land once instead of being reimplemented in each route spec

### Slice 35.2: [Work-entry and queue routes](/home/thetu/planner/docs/planner-solidstart-phase-35-2-work-entry-and-queue-routes-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/`
- `/projects`
- `/projects/new`
- `/sessions`
- `/sessions/new`

Purpose:

- make the primary entry/navigation surfaces browsable in Builder
- cover the main list/create flows with deterministic mock state

Primary data shape:

- project list
- session list
- create-project
- create-session

Why this is the first route slice:

- it gives immediate click-through value
- it establishes navigation continuity for the rest of the route family
- it is simpler than session workspace and project advanced surfaces

### Slice 35.3: [Session workspace route](/home/thetu/planner/docs/planner-solidstart-phase-35-3-session-workspace-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/sessions/:sessionId`

Purpose:

- make the question-bank session page interactable under frontend mock mode

Primary data shape:

- session detail
- prompt bank
- prompt drafts
- mock session transport events

Why this is isolated:

- it is the hardest route because it needs live-like progression semantics
- it should not block the simpler route-list browsing slices

### Slice 35.4: [Project workspace route](/home/thetu/planner/docs/planner-solidstart-phase-35-4-project-workspace-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/projects/:projectSlug`

Purpose:

- make the main project workspace browsable with attached advanced summaries

Primary data shape:

- project detail
- project sessions
- prompt bank summary
- import-state/import-review summary
- runs/events/export-history summary
- blueprint summary

Why this is separate from session:

- it is complex, but the complexity is broad aggregate state rather than live
  transport behavior
- it should reuse the mock foundation and the mocked session summary contract

### Slice 35.5: [Import review route](/home/thetu/planner/docs/planner-solidstart-phase-35-5-import-review-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/projects/:projectSlug/import`

Purpose:

- make import review/history/compare/restore browsing possible in Builder

Primary data shape:

- project detail
- import review
- import state
- import history
- comparison payloads

Why this is separate:

- it has a distinct workflow and payload family
- it should not widen the first project-workspace slice unnecessarily

### Slice 35.6: [Knowledge and blueprint routes](/home/thetu/planner/docs/planner-solidstart-phase-35-6-knowledge-and-blueprint-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/knowledge`
- `/blueprint`

Purpose:

- make the graph/inventory exploration surfaces browsable from shared mock
  blueprint state

Primary data shape:

- project list
- project blueprint payload

Why these pair together:

- both are read-heavy views over the same blueprint/node graph contract

### Slice 35.7: [Events, discovery, and admin routes](/home/thetu/planner/docs/planner-solidstart-phase-35-7-events-discovery-and-admin-frontend-mock-spec.md)

Status:

- implemented

Routes:

- `/events`
- `/discovery`
- `/admin`

Purpose:

- make the operational/triage surfaces browsable from deterministic mock
  telemetry and proposal state

Primary data shape:

- blueprint events
- snapshot history
- discovery proposals and scan results
- admin status and admin events

Why these pair together:

- all three are operational surfaces rather than core work-entry/workspace
  routes
- they can share the same mock "ops attention" and "quiet system" scenarios

### Follow-on 35.10: [Builder frontend mock runtime alignment](/home/thetu/planner/docs/planner-solidstart-phase-35-10-builder-frontend-mock-runtime-alignment-spec.md)

Status:

- implemented

Purpose:

- correct the Builder-facing setup contract so the canonical UI-review project
  targets the frontend-only mock runtime on `3000` rather than the server-backed
  `4174` path
- make the exact Builder project env/command/URL settings explicit
- re-prove top-level route navigation under the actual Builder-targeted
  frontend mock runtime
## 7. Touched Surfaces

- `planner-solid/src/lib/api.ts`
- new provider-backed API client and mock-runtime modules under
  `planner-solid/src/lib/`
- session transport creation used by
  `planner-solid/src/routes/sessions/session-workspace-controller.ts`
- route-facing scenario and fixture modules used by the Solid route family
- targeted `planner-solid/e2e` and/or unit proof that can consume the shared
  mock scenarios
- docs that explain the frontend mock contract for Builder/UI design work

## 8. Acceptance Criteria

1. `planner-solid` can run in a documented frontend mock mode and still render
   the main route family without `planner-server`
2. the session workspace remains interactable in that mode, including prompt
   visibility, draft save behavior, and deterministic progress events
3. the route family uses one shared mock scenario registry rather than
   scattering the default payloads across individual browser tests
4. the UI clearly communicates when frontend mock mode is active so it is not
   mistaken for live backend truth
5. the mock mode is activatable through a clear frontend contract, for example
   an env such as `frontend_mock=true` or equivalent documented profile

## 9. Verification Plan

- targeted unit tests for API-provider resolution and mock scenario mutation
  behavior
- targeted session-route tests proving the mock transport can drive startup and
  prompt-bank progression without a real websocket backend
- browser proof against a backendless frontend run, for example `vite dev`
  with frontend mock mode enabled, covering:
  - `/`
  - `/projects`
  - `/projects/:projectSlug`
  - `/projects/:projectSlug/import`
  - `/sessions`
  - `/sessions/new`
  - `/sessions/:sessionId`
  - `/knowledge`
  - `/blueprint`
  - `/events`
  - `/discovery`
  - `/admin`
- browser proof should use the same frontend mock contract that Builder will
  use for route browsing

## 10. Rollback / Fallback

If full route-family coverage is too broad in one pass:

- land the provider seam and shared scenario registry first
- cover the work-entry, project, and session route family first
- defer knowledge, blueprint, events, discovery, and admin scenario packs to
  follow-on slices
- do not fallback to adding more isolated ad hoc Playwright interceptors as the
  main product path

## 11. Open Questions

None block readiness.

## 12. Implementation Outcome

Implemented on 2026-03-30.

This umbrella tranche now delivers the Builder-facing capability it was created
to close:

- `planner-solid` now supports documented frontend-only route browsing through
  `VITE_PLANNER_FRONTEND_MOCK=1`
- the route family shares one app-owned scenario registry and in-memory mock
  store rather than scattered browser-test interceptors
- `/`, `/projects`, `/projects/new`, `/projects/:projectSlug`,
  `/projects/:projectSlug/import`, `/sessions`, `/sessions/new`,
  `/sessions/:sessionId`, `/knowledge`, `/blueprint`, `/events`, `/discovery`,
  and `/admin` are all browseable without `planner-server`
- the session workspace remains interactable under mock transport rather than
  collapsing into a static shell
- route continuity preserves the active mock scenario across the main click
  paths Builder uses for UI review

Verification evidence:

- targeted browser proof covered every route family named in this umbrella,
  including session progression, import apply, knowledge project switching,
  snapshot creation, and discovery triage
- `npm --prefix planner-solid run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts src/routes/projects/project-workspace-controller.test.ts src/routes/sessions/session-workspace-view.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Residual note:

- the pre-existing Nitro `"send"` warning during build remains, but the build
  exits successfully and this tranche did not widen that issue
- implementation review after closeout found one bounded follow-on remediation
  thread:
  - [Planner SolidStart Phase 35.8 Backendless Mock Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-8-backendless-mock-closeout-remediation-spec.md)
  - that follow-on is now implemented and closes the remaining discovery
    refresh, shared-scenario E2E proof, and Builder-facing documentation gaps
- a separate optional residual-cleanup follow-on is now captured in
  [Planner SolidStart Phase 35.9 Backendless Mock Residual Cleanup Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-9-backendless-mock-residual-cleanup-spec.md)
  for the lingering Nitro build warning, broader Playwright fixture-ownership
  migration, and bounded scenario-pack polish without reopening the tranche
- that residual cleanup slice is now implemented, so the current documented
  Phase 35 follow-ons are closed again
