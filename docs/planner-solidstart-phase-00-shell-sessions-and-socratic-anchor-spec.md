# Planner SolidStart Phase 00 Shell, Sessions, And Socratic Anchor Spec

**Status:** implemented  
**Date:** 2026-03-24  
**Parent:** [Planner SolidStart Platform Direction Spec](/home/thetu/planner/docs/planner-solidstart-platform-direction-spec.md)  
**Related Planning:** [Socratic SolidStart Greenfield Platform Spec](/home/thetu/planner/docs/socratic-solidstart-greenfield-platform-spec.md), [Socratic Initial Prompt Bank And Dynamic Hydration Spec](/home/thetu/planner/docs/socratic-initial-prompt-bank-and-dynamic-hydration-spec.md), [Planner UI Reset Phase 00 Shell Navigation And Auth Spec](/home/thetu/planner/docs/planner-ui-reset-phase-00-shell-navigation-and-auth-spec.md), [Planner UI Reset Phase 04 Sessions Queue Spec](/home/thetu/planner/docs/planner-ui-reset-phase-04-sessions-queue-spec.md), [Session Workflow Web UI Implementation Plan](/home/thetu/planner/docs/session-workflow-webui-plan.md)

> Planning note (2026-03-24): this is the first bounded delivery slice under
> the Planner-wide SolidStart direction. It does not attempt full product
> parity. It establishes the new SolidStart shell, a simplified route spine,
> the sessions queue as the first non-Socratic validation route, and the
> Socratic workspace as the first anchor route under the prompt-bank contract.
>
> Implementation sync (2026-03-24): the new `planner-solid/` app now exists,
> root frontend scripts point at it, `/`, `/sessions`, `/sessions/new`, and
> `/sessions/:sessionId` are implemented, and the backend now exposes a
> truthful `/sessions/{id}/prompt-bank` endpoint that separates banked prompts
> from queued prompt-ready threads. `planner-server` now serves the Solid app's
> static export by default from `./planner-solid/dist/static`, and browser
> verification covers the shell, queue, creation flow, and bank-aware Socratic
> anchor route against the Rust server handoff.

## 1. Executive Judgment

The first SolidStart slice must prove the platform on the routes that matter
most to Planner's current product loop:

- getting into work
- seeing session work clearly
- opening the Socratic workspace

That means the first slice should not try to port every React route.

The selected bounded slice is:

- SolidStart app shell
- simplified route spine
- local-only runtime assumptions
- sessions queue as the first non-Socratic route
- Socratic workspace as the anchor route

This is the smallest slice that can validate:

- the platform decision
- the route simplification posture
- the local-speed requirement
- the prompt-bank workspace contract

## 2. User Outcome

After Phase 00:

- Planner boots into a new SolidStart shell with a simpler route model
- the main navigation is quieter and easier to scan than the current React app
- users can open the sessions queue and understand active work quickly
- users can enter the Socratic workspace through the new shell
- the Socratic route is shaped around the prompt-bank contract rather than the
  old one-prompt-plus-shells behavior
- the app already feels more local and visually coherent even before the rest
  of Planner is migrated

## 3. Locked Decisions

- this phase is a new SolidStart app, not a React retrofit
- this phase assumes local-only runtime posture; Auth0 is out of scope
- the first non-Socratic validation route is `/sessions`
- the first anchor route is the Socratic workspace
- route cleanup is allowed and expected
- same-path parity with the current React app is not required
- visual clarity and local-speed outrank rote route preservation
- the first implementation may leave Projects, Knowledge, Blueprint, Discovery,
  Events, and Admin outside the migrated slice

## 4. Scope

### In Scope

- scaffold the new SolidStart app shell and route layout
- define the simplified Phase 00 route map
- implement the first non-Socratic route: sessions queue
- implement the Socratic workspace route shell and prompt-bank-aware data
  contract
- establish local-only runtime assumptions in the app shell
- establish the first Solid-native testing harness for the new app

### Out Of Scope

- full Planner route parity
- Auth0 parity
- migrating every current route into SolidStart in this phase
- product redesign of unrelated route families beyond what the new shell needs

## 5. Phase 00 Route Model

The Phase 00 route map should be intentionally smaller than the current React
route tree.

Selected initial routes:

- `/`:
  - local-first home or work entry shell
- `/sessions`:
  - cross-project sessions queue
- `/sessions/new`:
  - create/start a new session flow
- `/sessions/:sessionId`:
  - session workspace, with Socratic as the anchor experience

Allowed simplifications:

- no separate singular `/session/:id` versus plural queue split
- no duplicate project redirect routes in this phase
- no utility-route carryover just because they exist in the React app

## 6. Shell Contract

The SolidStart shell must establish the new platform direction immediately:

- fixed app shell
- simplified primary navigation
- local-first product posture
- no legacy React-era sidebar clutter
- one shared visual language across entry, queue, and session workspace

The shell must also prove that route cleanup is real, not theoretical.

That means:

- fewer primary destinations
- no utility inventory posing as main navigation
- no shell chrome that competes with active work

## 7. Sessions Queue Contract

The first non-Socratic validation route is the sessions queue because it
already embodies Planner's work-selection model.

The SolidStart `/sessions` route must preserve the useful product semantics
from the current implemented queue while simplifying the presentation:

- rows are the primary object
- urgency and next action are readable inline
- project/session context remains visible
- the queue is scannable and calm

The queue must also be fast:

- list navigation feels immediate
- no heavy dashboard framing
- no unnecessary route or shell churn around row interaction

## 8. Socratic Anchor Contract

The session workspace route in this phase must be anchored on the already
selected Socratic product contract:

- master-detail local workspace
- initial prompt bank before first reveal
- dynamic later updates
- no fake locally answerable preview-shell rows

This phase does not need every possible Socratic refinement from day one, but
it must establish the correct platform/runtime shape:

- local graph state in Solid-native primitives
- prompt-bank-aware first reveal
- local thread switching among banked threads
- isolated typing under background updates

## 9. Runtime Contract

Phase 00 assumes local-only operation for the frontend platform move.

That means:

- no Auth0 dependency in the initial SolidStart shell
- session creation, queue loading, and workspace entry can assume local runtime
  configuration
- backend/session/websocket contract changes are allowed where needed to serve
  the prompt-bank model cleanly

The explicit future auth model remains a later planning concern, not a blocker
for this slice.

## 10. Testing Contract

Phase 00 also establishes the initial future app test shape.

Selected first-wave stack:

- unit/component: Vitest
- browser verification: Playwright

The phase must add enough coverage to prove:

- shell route loading
- queue rendering and row action behavior
- Socratic first reveal and local thread switching on banked work

## 11. Acceptance Criteria

This slice is complete only when:

1. a new SolidStart app shell exists and boots as the active Phase 00 frontend
   target
2. the initial route map is simplified to the bounded Phase 00 set
3. `/sessions` is implemented as the first non-Socratic validation route
4. `/sessions/:sessionId` establishes the Socratic anchor route on the
   prompt-bank contract
5. the app behaves as a local-first tool without Auth0 dependency in this phase
6. local-speed and visual clarity are materially better than the current React
   route accumulation
7. the test harness proves shell, queue, and Socratic anchor behavior

## 12. Verification Plan

### Unit / component

- shell route and nav tests
- sessions queue tests for row semantics and primary actions
- Socratic route tests for first reveal, banked-thread switching, and truthful
  non-banked handling

### Browser

- root entry to sessions flow
- sessions queue to session workspace flow
- prompt-bank first reveal behavior in the Socratic workspace
- local thread switching without per-thread loading on banked content

### Build

- SolidStart build succeeds
- production routing works for the bounded Phase 00 route set

## 13. Rollback / Fallback

If Phase 00 cannot yet replace the current frontend in one move, the truthful
fallback is:

- keep the new SolidStart app as the active migration target
- do not pretend the current React shell is the future-state architecture
- reduce Phase 00 route scope further before widening it

Disallowed fallback:

- broadening the route set to chase parity before the shell, queue, and
  Socratic anchor are solid

## 14. Remaining Open Questions

These do not block Phase 00 readiness:

- the next widening route family after Phase 00 is now closed as the
  projects/work-entry slice in
  [Planner SolidStart Phase 01 Projects And Guided Work Entry Spec](/home/thetu/planner/docs/planner-solidstart-phase-01-projects-and-guided-work-entry-spec.md)
- session creation ergonomics may still move later from `/sessions/new` into a
  more project-first flow if Phase 01 makes that cleaner

## 15. Readiness Judgment

This spec is **implemented**.

Delivered:

- one platform
- one shell
- one validation route outside Socratic
- one anchor route inside Socratic

Verification completed:

- `cargo build -p planner-server`
- `cargo test -p planner-server tier2_session_prompt_bank_reports_banked_and_queued_threads_truthfully -- --nocapture`
- `npm test`
- `npm run build`
- `cd planner-solid && npm run lint`
- `cd planner-solid && npm run test:e2e`

That is enough to prove the platform without exploding into a whole-app rewrite
in one pass.
