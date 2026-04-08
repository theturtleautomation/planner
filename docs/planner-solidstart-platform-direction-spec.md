# Planner SolidStart Platform Direction Spec

**Status:** active  
**Date:** 2026-03-24  
**Parent:** [Planner OMX Project Plan](/home/thetu/planner/.omx/ledger/project-plan.md)  
**Source Research:** official Solid and SolidStart docs reviewed on 2026-03-24, plus current Planner planning artifacts  
**Related Planning:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Planner UI Reset Route-By-Route Spec Queue](/home/thetu/planner/docs/planner-ui-reset-route-by-route-spec-queue.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)

> Planning note (2026-03-24): the user explicitly rejected a split-framework
> future. This spec widens the SolidStart platform decision from a
> Socratic-lobby-only fork to a whole-frontend greenfield direction for
> Planner.
>
> Status sync note (2026-03-30): this is the active platform-direction
> container, not a bounded delivery-ready slice.

## 1. Executive Judgment

Planner should not adopt separate frontend frameworks by route.

If the greenfield future-state platform is being reconsidered, it should be
reconsidered for the full Planner web product, not just for the Socratic lobby.

The selected greenfield direction is therefore:

- **entire Planner frontend future-state platform:** SolidStart
- **current React app:** live baseline, migration source, and requirements
  reference

This keeps the long-term platform coherent:

- one routing/runtime model
- one reactive UI model
- one shared component and shell system
- one test/tooling direction

The selected migration posture is also explicit now:

- full frontend replacement
- no intended split-framework steady state
- route cleanup and simplification are allowed during the rewrite
- local-speed and visual clarity are first-class product requirements, not
  secondary polish

## 2. Problem

The narrower Socratic-only SolidStart fork would create the wrong long-term
shape:

- separate framework mental models inside one product
- duplicated app shell/auth/routing concerns
- fragmented design system implementation
- higher maintenance cost for tests, deployment, and onboarding

That is not a serious platform decision. It is a local optimization that would
become architectural drag.

## 3. User Outcome

After this direction is fully realized:

- Planner uses one frontend platform across the product
- the Socratic lobby gets the fine-grained, native-feeling behavior it needs
  without becoming a special-case framework island
- the broader Planner routes can share the same shell, state patterns, routing
  model, design system, and deployment/runtime assumptions
- future route work happens on one coherent platform instead of straddling two

## 4. Product Decision

The greenfield future-state platform for Planner is **SolidStart**.

This does not mean immediate rewrite.

It means:

- React remains the live production baseline
- SolidStart becomes the selected target platform for greenfield frontend work
- future route/platform planning should assume one eventual SolidStart app, not
  mixed React plus SolidStart surfaces

The migration shape is now closed:

- the intended end-state is a full replacement of `planner-web`
- the current React app may be frozen and discarded rather than carried as a
  long-lived compatibility surface
- the new SolidStart app may simplify routes and information architecture where
  that improves usability and clarity
- deployment is direct replacement rather than permanent side-by-side operation
- a maintenance window is acceptable during cutover

## 5. Scope Boundaries

### In Scope

- setting the whole-frontend greenfield platform direction
- defining the platform-level implications of moving Planner to SolidStart
- identifying the migration boundary between current React and future
  SolidStart
- reframing the Socratic SolidStart spec as a route-specific child of a broader
  platform decision

### Out Of Scope

- immediate implementation
- committing to a big-bang rewrite schedule
- route-by-route migration sequencing beyond what is needed to bound the
  platform decision
- backend rewrites unrelated to frontend platform selection

## 6. Platform Contract

The future-state Planner frontend should converge on:

- **framework:** SolidStart
- **reactivity:** Solid-native fine-grained primitives
- **routing/layout:** one shared SolidStart app shell
- **design system:** one shared cross-route component/token layer
- **session/runtime integration:** one shared websocket/auth/session strategy
- **testing:** one coherent browser/component/unit strategy for the future app

Additional locked platform requirements:

- **local-speed:** visible navigation and editing must feel point-and-click
  immediate for already-known content
- **visual clarity:** route count, shell hierarchy, and page structure may be
  simplified aggressively to reduce cognitive noise
- **local-first development reality:** Auth0 continuity is not a blocker for
  the first SolidStart platform move because Planner is currently being treated
  as a local-first tool
- **backend freedom:** frontend migration may reshape session and websocket
  payloads where needed; compatibility with the current React contract is not a
  hard constraint

The Socratic lobby remains the strongest motivating route because it stresses:

- dense interactive state
- prompt-bank hydration
- background graph mutation
- local-fast editing

But the decision is no longer limited to Socratic.

## 7. Relationship To Existing Specs

This spec becomes the broader platform parent for:

- [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md)

The existing React-era route specs remain valuable as:

- product requirements source
- UX and verification source
- migration reference

They are no longer treated as final platform architecture.

## 8. Migration Principles

The future platform direction must obey these rules:

1. no mixed-framework steady state as the intended end-state
2. full replacement is preferred over indefinite coexistence
3. no Socratic-only island that duplicates shell/auth/runtime permanently
4. route requirements should migrate from current React specs into
   SolidStart-native implementation specs
5. the design system should not fork by framework ideology
6. route simplification is allowed when it improves clarity and usability
7. local-speed and visual clarity outrank rote parity with the current route
   map
8. the React app remains the truth source for current behavior until a future
   SolidStart replacement actually exists

## 9. Closed Decisions

The following migration-shape decisions are now closed:

- full frontend replacement instead of a permanent mixed-framework approach
- route cleanup is allowed during the rewrite
- same-route parity is not required where simplification improves usability
- the current React app does not need to remain a long-lived sibling product
- auth continuity is not a first-wave blocker for the platform move because the
  tool is currently local-first
- backend payload shapes may change where the SolidStart app needs a cleaner
  contract
- direct replacement deployment is acceptable
- maintenance windows are acceptable during cutover

## 10. Remaining Open Questions

These no longer block platform direction, but they still need follow-on
planning before delivery:

- the first widening route family after Phase 00 is now closed as the
  projects/work-entry family in
  [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md)
- what is the explicit future auth model once Planner moves beyond the current
  local-first posture?
- what testing stack and CI shape should be canonical for the future SolidStart
  app once the route family broadens beyond the current local app proof?

## 11. Readiness Judgment

This spec is now **ready for implementation**.

The platform decision is clear and the migration shape is closed enough to
bound real delivery:

- one future frontend framework
- full replacement
- route cleanup allowed
- backend contract freedom allowed
- local-speed and clarity treated as hard product requirements
