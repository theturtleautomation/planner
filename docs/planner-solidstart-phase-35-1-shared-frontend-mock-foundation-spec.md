# Planner SolidStart Phase 35.1 Shared Frontend Mock Foundation Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner SolidStart Phase 31 Session Workspace Route Family Decomposition Spec](/home/thetu/planner/docs/planner-solidstart-phase-31-session-workspace-route-family-decomposition-spec.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Review:** 2026-03-30 direct inspection of `planner-solid/src/lib/api.ts`, `planner-solid/src/routes/sessions/session-workspace-controller.ts`, `planner-solid/src/app.tsx`, `planner-solid/vite.config.ts`, and the existing `planner-solid/e2e` route-interception helpers

## 1. Executive Judgment

Every route slice under Phase 35 depends on one shared contract:

- a frontend mock activation mode
- a provider seam behind `~/lib/api`
- a shared scenario registry
- a session transport seam for routes that currently assume a real websocket

That foundation must land first or the route slices will duplicate mock
plumbing and drift immediately.

## 2. User Outcome

After this phase:

- `planner-solid` can run in a documented frontend mock mode without
  `planner-server`
- route code reads data through the same API facade, but that facade can now
  resolve to either live fetch or frontend mock handlers
- mock scenarios are shared repo assets rather than scattered ad hoc test data
- the app visibly indicates when frontend mock mode is active

## 3. Scope

### In Scope

- frontend mock activation contract
- API provider seam behind `planner-solid/src/lib/api.ts`
- shared scenario registry and in-memory mock store
- session transport factory abstraction for session-route use
- global mock-mode UI affordance or badge

### Out Of Scope

- route-specific scenario completion beyond what is needed to prove the
  foundation
- broad route polish or IA changes
- backend runtime mocks or `planner-server` changes

## 4. Contract

### 4.1 Activation

The frontend mock mode should use a Vite-compatible build/runtime contract:

- build/env gate: `VITE_PLANNER_FRONTEND_MOCK=1`
- optional runtime scenario override: `?mockScenario=<scenario-key>`

Rules:

- if `VITE_PLANNER_FRONTEND_MOCK` is not enabled, the app uses the live API
  client
- if `VITE_PLANNER_FRONTEND_MOCK` is enabled, the app uses the frontend mock
  provider and must not require `/api` reachability
- `mockScenario` is honored only when frontend mock mode is enabled

Builder-facing example:

```bash
VITE_PLANNER_FRONTEND_MOCK=1 npm run dev --prefix planner-solid
```

Then browse:

```text
http://127.0.0.1:3000/?mockScenario=default
```

### 4.2 Provider seam

The current exported API surface should remain stable where practical, but the
implementation should route through a provider interface rather than direct
`fetch` calls only.

Minimum capability:

- GET/POST style request handlers for the current frontend route needs
- cache invalidation compatible with the current `cachedGet` semantics where
  still useful
- in-memory mutations for create/update flows in mock mode

Required file-level direction:

- keep `planner-solid/src/lib/api.ts` as the public facade used by routes
- add an internal provider selector module, for example
  `planner-solid/src/lib/api-provider.ts`
- add a live provider implementation for the current fetch-based behavior
- add a mock provider implementation backed by the scenario registry and
  in-memory store

### 4.3 Session transport seam

The session route must stop constructing `new WebSocket(...)` directly.

Required direction:

- introduce a transport factory
- live mode returns the current websocket transport
- frontend mock mode returns an in-process deterministic transport

Required file-level direction:

- the session controller should depend on a route-agnostic transport adapter
- the transport seam should live under `planner-solid/src/lib/`, not as a
  session-route-only inline helper, so later route slices can reuse the same
  mock/runtime selection pattern

### 4.4 Scenario registry

The foundation must define named scenario packs at minimum for:

- `default`
- `empty`
- `session-workspace`
- `import-review`
- `ops-attention`

The registry should be able to expose:

- projects
- sessions
- project detail and aggregate state
- prompt-bank/session state
- blueprint state
- import state/history
- timeline/admin/discovery state

Required file-level direction:

- scenario definitions should live under a dedicated mock namespace, for
  example `planner-solid/src/lib/mock/`
- the in-memory mutable state should be separate from the immutable scenario
  baselines so route actions can mutate a running session without corrupting
  the scenario source data

## 5. Product Decisions

### 5.1 Mock mode is for browsing, not truth claims

The UI must not present frontend mock mode as backend truth.

Required behavior:

- visible shell-level mock badge in the main app header from
  `planner-solid/src/app.tsx`
- copy should describe the mode as frontend mock browsing/design mode
- docs must not describe this mode as integration proof

### 5.2 First-pass persistence

The first pass should use in-memory state only.

Required behavior:

- navigation within the running app preserves mock mutations
- full page reload may reset to the selected scenario baseline

This keeps the foundation bounded while still supporting click-through design
work.

### 5.3 Keep the public API surface function-based

The route layer already consumes plain exported functions from `~/lib/api`.

Decision:

- keep that public function-based surface for bounded adoption
- let `api.ts` delegate internally to the selected provider
- do not force a broad route refactor to injected client objects in this slice

This minimizes route churn while still creating the required provider seam.

## 6. Implementation Plan

1. Add frontend mock env detection and scenario selection helpers.
2. Split the current fetch logic behind an internal provider interface while
   preserving the `~/lib/api` exports.
3. Add the initial mock namespace with immutable scenario baselines and an
   in-memory mutable store.
4. Add a session transport abstraction and replace direct websocket
   construction in the session controller.
5. Add a shell-level mock-mode badge in `app.tsx`.
6. Seed only the minimum `default` and `empty` scenarios needed to prove the
   foundation, leaving richer route scenarios to later child slices.

## 7. Touched Surfaces

- [api.ts](/home/thetu/planner/planner-solid/src/lib/api.ts)
- [session-workspace-controller.ts](/home/thetu/planner/planner-solid/src/routes/sessions/session-workspace-controller.ts)
- [app.tsx](/home/thetu/planner/planner-solid/src/app.tsx)
- [vite.config.ts](/home/thetu/planner/planner-solid/vite.config.ts)
- new frontend mock provider/runtime modules under `planner-solid/src/lib/`
- shell-level UI for mock mode state

## 8. Acceptance Criteria

1. the app can switch between live and frontend mock providers through a
   documented contract
2. the session route no longer depends on direct websocket construction
3. scenario packs are defined in shared modules rather than embedded in route
   tests
4. the UI clearly indicates when frontend mock mode is active
5. frontend mock mode does not require the Vite `/api` proxy target to be
   reachable
6. route slices can build on this foundation without redefining provider or
   scenario ownership

## 9. Verification Plan

- unit tests for provider selection and scenario resolution
- unit tests for in-memory mock mutation behavior
- targeted session transport tests proving the mock transport can open, send,
  and emit deterministic events
- targeted mock-runtime tests covering frontend mock activation and badge copy
  behavior
- frontend build/lint verification

Suggested verification commands after implementation:

- `npm --prefix planner-solid run test`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

## 10. Rollback / Fallback

If the full scenario registry is too broad in one pass:

- land provider selection and session transport abstraction first
- ship a minimal `default` and `empty` scenario set
- add the remaining named scenarios in the following route slices

## 11. Open Questions

None block readiness.

## 12. Implementation Outcome

Implemented on 2026-03-30.

This slice landed the bounded frontend mock foundation without widening into
route-family completion:

- `VITE_PLANNER_FRONTEND_MOCK=1` now activates a frontend-only mock mode for
  `planner-solid`
- `planner-solid/src/lib/api.ts` remains the public facade while delegating
  through an internal provider seam
- a shared mock runtime/store now owns minimal scenario baselines plus in-memory
  mutation state
- the session route now depends on a shared transport abstraction instead of
  direct websocket construction
- the app shell now shows a frontend mock badge in the header when mock mode is
  active

Verification evidence:

- `npm run test -- --run src/lib/api.test.ts src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts src/lib/session-transport.test.ts`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`

Residual verification note:

- the build still emits the pre-existing Nitro warning about `"send"` from
  `h3/dist/_entries/node.mjs`, but the command exits successfully and this
  slice did not introduce a new build failure.
