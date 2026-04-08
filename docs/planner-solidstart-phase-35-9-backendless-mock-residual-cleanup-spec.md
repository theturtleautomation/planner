# Planner SolidStart Phase 35.9 Backendless Mock Residual Cleanup Spec

**Status:** implemented  
**Date:** 2026-03-30  
**Parent:** [Planner SolidStart Phase 35 Backendless Mock Route Coverage Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-backendless-mock-route-coverage-spec.md)  
**Depends On:** [Planner SolidStart Phase 35.8 Backendless Mock Closeout Remediation Spec](/home/thetu/planner/docs/planner-solidstart-phase-35-8-backendless-mock-closeout-remediation-spec.md)  
**Related Planning:** [Builder Local Workflow](/home/thetu/planner/docs/builder-local-workflow.md), [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md), [README.md](/home/thetu/planner/README.md)  
**Source Review:** 2026-03-30 review of the completed Phase 35 tranche, `planner-solid/e2e`, frontend mock scenario modules, and the remaining `planner-solid` build warning

## 1. Executive Judgment

Phase 35 is already implemented and honestly closed for its required route
coverage outcome.

There are still three worthwhile residual cleanup items:

- the pre-existing Nitro build warning around `"send"` remains unresolved
- most older Playwright specs still own their route payloads through local
  `page.route(...)` interception instead of the app-owned frontend mock runtime
- the scenario registry now covers the route family, but it is still thin in
  places where additional variant polish would improve Builder browsing and UI
  review

These are not reasons to reopen Phase 35 itself.

They are a bounded quality and maintainability follow-on, so they should live
in one explicit residual cleanup slice.

## 2. User Outcome

After this phase:

- `planner-solid` build output is either free of the known Nitro `"send"`
  warning or explicitly documented as a pinned upstream/tooling limitation with
  a verified containment note
- more of the Playwright suite proves route behavior through the shared
  frontend mock runtime instead of hand-owned route interception
- Builder and local UI review can select from a slightly richer set of
  coherent frontend mock scenarios without inventing backend-truth claims

## 3. Scope

### In Scope

- bounded investigation and remediation of the remaining Nitro build warning
- migration of a targeted additional set of Playwright specs away from
  `page.route(...)` where the shared frontend mock runtime can now own the
  route payload contract
- additive polish to the existing frontend mock scenario packs
- planning and documentation sync that keeps Phase 35 and Phase 35.8 closed as
  implemented while introducing this follow-on honestly

### Out Of Scope

- reopening the implemented Phase 35 route-family tranche
- claiming the frontend mock runtime replaces backend-integrated verification
- migrating the entire historical Playwright suite in one pass
- turning scenario polish into a new product or IA redesign

## 4. Contract

### 4.1 Build-warning remediation contract

Required outcome:

- the current Nitro build warning about `"send"` must be classified as one of:
  - fixed in repo code or config
  - fixed through dependency or build-tool adjustment
  - still present but explicitly documented as an upstream/tooling limitation
    with proof that the build remains successful and functionally unaffected

This slice should not hand-wave the warning as acceptable without recording why
it still exists if it cannot be removed.

### 4.2 E2E ownership migration contract

Required outcome:

- at least a targeted additional set of legacy Playwright route specs should
  stop rebuilding their core payload family through local interception when the
  frontend mock runtime can now provide the same state more truthfully
- migrated specs should prefer scenario selection and app-owned state over
  duplicate fixture payloads
- per-test overrides may remain where they express behavior the scenario
  registry does not and should not own

The goal is not zero `page.route(...)` usage. The goal is to reduce avoidable
duplication and make the frontend mock runtime a broader proof surface.

### 4.3 Scenario polish contract

Required outcome:

- the existing scenario registry should gain a small number of additional
  route-coherent variants where current browsing still feels thin
- scenario additions must remain deterministic and named, not ad hoc
- new variants should improve design-review coverage for high-value states such
  as richer empty states, alternate operational density, or a second believable
  project/session posture

Scenario polish must strengthen browsing fidelity without drifting into backend
simulation or uncontrolled fixture growth.

## 5. Product Decisions

### 5.1 Keep the tranche closed

Phase 35 and Phase 35.8 stay implemented.

This slice exists because the remaining work is quality debt and maintainability
debt, not because the main frontend mock route-coverage capability failed.

### 5.2 Prioritize leverage over breadth

The best follow-on is not "migrate everything."

It is:

- remove or explain the lingering build warning
- migrate the highest-value duplicate Playwright surfaces first
- add only the scenario variants that materially improve manual Builder review

### 5.3 Preserve mock-mode truthfulness

Scenario polish should stay honest about what frontend mock mode is for:

- UI review
- click-through browsing
- deterministic route-state proof

It should not blur into claims of real backend or full pipeline fidelity.

## 6. Candidate Touched Surfaces

- `planner-solid` build config and dependencies if the Nitro warning is
  removable
- `planner-solid/e2e/*.spec.ts` files that still overuse `page.route(...)`
- [playwright.frontend-mock.config.ts](/home/thetu/planner/planner-solid/playwright.frontend-mock.config.ts)
- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [store.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.ts) if new
  scenario variants need bounded mutation support
- [README.md](/home/thetu/planner/README.md) or
  [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)
  only if build-warning or scenario guidance needs durable clarification

## 7. Acceptance Criteria

1. The remaining Nitro `"send"` build warning is either removed or documented
   with a concrete containment judgment and verification evidence.
2. At least one additional meaningful Playwright slice beyond the existing
   Phase 35 frontend-mock proof now uses the shared frontend mock runtime
   instead of rebuilding its state via broad `page.route(...)` interception.
3. The frontend mock scenario registry gains bounded, deterministic variant
   coverage that improves manual browsing for at least one high-value route
   family.
4. Phase 35 and Phase 35.8 remain recorded as implemented rather than being
   silently downgraded or reopened.

## 8. Verification Plan

- targeted build verification:
  - `npm --prefix planner-solid run build`
- targeted frontend mock/browser verification for any newly migrated Playwright
  slice
- test review confirming the chosen migrated specs no longer duplicate route
  payload ownership unnecessarily
- targeted unit tests if scenario-polish work widens mock store behavior
- `npm --prefix planner-solid run lint`

## 9. Rollback / Fallback

If the Nitro warning cannot be removed safely in one bounded pass:

- document the exact cause, current impact, and containment judgment
- do not block the broader cleanup slice on a speculative bundler chase

If broad Playwright migration proves too large:

- migrate one or two high-value specs only
- capture the remaining migration candidates explicitly in implementation
  notes instead of pretending the whole suite was cleaned up

If scenario polish starts drifting into mock sprawl:

- cut new variants back to the smallest named set that materially improves
  Builder browsing

## 10. Open Questions

None block readiness.

## 11. Implementation Outcome

Implemented on 2026-03-30.

This slice kept Phase 35 closed while resolving the remaining bounded quality
items that were worth carrying forward:

- the frontend mock runtime now includes a richer operational-history scenario
  through `ops-history`
- the legacy Phase 08 and Phase 12 Playwright route proofs now run against the
  shared frontend mock runtime instead of route-local `page.route(...)`
  payload ownership
- the lingering Nitro `"send"` warning is now documented as a contained
  dependency-version limitation rather than an unexplained repo warning

Primary implementation surfaces:

- [runtime.ts](/home/thetu/planner/planner-solid/src/lib/mock/runtime.ts)
- [scenarios.ts](/home/thetu/planner/planner-solid/src/lib/mock/scenarios.ts)
- [runtime.test.ts](/home/thetu/planner/planner-solid/src/lib/mock/runtime.test.ts)
- [store.test.ts](/home/thetu/planner/planner-solid/src/lib/mock/store.test.ts)
- [phase-08-events.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-08-events.spec.ts)
- [phase-12-discovery.spec.ts](/home/thetu/planner/planner-solid/e2e/phase-12-discovery.spec.ts)
- [playwright.frontend-mock.config.ts](/home/thetu/planner/planner-solid/playwright.frontend-mock.config.ts)
- [package.json](/home/thetu/planner/planner-solid/package.json)
- [README.md](/home/thetu/planner/README.md)
- [builder-local-workflow.md](/home/thetu/planner/docs/builder-local-workflow.md)

Build-warning containment judgment:

- `planner-solid` still builds successfully while Nitro reports
  `"send" is not exported by h3/dist/_entries/node.mjs`
- the current repo evidence points to a dependency-version split between
  `@solidjs/start@2.0.0-alpha.2` pulling `h3@2.0.1-rc.4` and Nitro's current
  `h3@1.15.10` path
- this slice did not claim the warning was fixed; it documented the limitation
  and reverified that build output remains usable

Verification evidence:

- `npm --prefix planner-solid run test -- --run src/lib/mock/runtime.test.ts src/lib/mock/store.test.ts`
- `npm --prefix planner-solid run test:e2e:frontend-mock`
- `npm --prefix planner-solid run lint`
- `npm --prefix planner-solid run build`
