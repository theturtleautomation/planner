# Phase 11 Socratic Category Replay And Validation Spec

**Status:** Implemented  
**Date:** 2026-03-21  
**Parent:** [Project Plan](/home/thetu/planner/docs/project-plan.md)  
**Source Research:** [Phase 08 Socratic Category Drill-Down Implementation](/home/thetu/planner/docs/phase-08-socratic-category-drilldown-implementation.md)  
**Prior Slice:** [Phase 10 Socratic Category Status And Refresh Spec](/home/thetu/planner/docs/phase-10-socratic-category-status-and-refresh-spec.md)

## Objective

Harden the category-driven Socratic flow so deep navigation remains trustworthy
across reconnects, checkpoint resume, stale category revisions, and repeated
returns to the main category screen.

This slice should turn the category flow from "works in the happy path" into a
feature that remains stable when the session is resumed, when clients reconnect,
or when a user acts on an older category snapshot.

It does **not** introduce new intake behavior beyond the already planned
recursive hierarchy and richer main-screen status semantics.

## User Outcome

After this slice:

- reconnecting to an active interview restores either the current category
  snapshot or the current scoped prompt with the correct breadcrumb path
- resuming from checkpoint does not strand the user in a stale category state
- acting on an outdated category revision refreshes the client cleanly instead
  of producing an ambiguous failure
- deep category navigation remains stable across web and TUI clients

The user still does **not** get multi-user collaborative interview editing or
cross-device merge behavior.

## Locked Decisions For This Slice

These decisions are treated as settled so implementation can stay bounded:

- snapshot revisions remain authoritative for category navigation actions
- stale `enter_category` requests should respond with the latest
  `category_state`, not a silent no-op
- interview replay must restore exactly one active surface at a time:
  category-state or prompt-state
- `done` remains valid only from the main category screen when build-ready is
  true
- validation should focus on targeted runtime and UI coverage, not broad manual
  test scripts as a substitute for automated proof

## Scope

### In scope

- deep-path checkpoint persistence and restore hardening
- stale revision handling for recursive category trees
- reconnect replay correctness for category-state versus prompt-state
- targeted web, server, core, and TUI validation for deep navigation paths
- one bounded manual verification note for the end-to-end category flow

### Out of scope

- collaborative editing or multi-actor conflict resolution
- browser offline mode support
- visual regression automation
- expanding into unrelated pipeline validation

## Current-State Evidence

- Phase 08 added category snapshots, category replay, and stale revision
  handling for the bounded hierarchy
- the original category-drilldown plan explicitly called for stronger checkpoint
  persistence, replay coverage, stale revision handling, and client navigation
  tests
- deeper recursive navigation increases the risk of replay and stale-state drift
  if that hardening remains implicit

## Requirements

### Replay contract

Reconnect and resume must remain unambiguous:

- a resumed session must restore the active category snapshot when the user is
  on a category screen
- a resumed session must restore the active prompt when the user is inside a
  scoped prompt flow
- prompt replay must preserve the full `category_path`

### Stale revision handling

Category actions must remain revision-safe:

- `enter_category` requests using an outdated revision must be rejected with the
  latest authoritative snapshot
- the client should be able to recover from that refresh without losing the
  session
- stale actions must not corrupt checkpoint state

### Build-gating and main-screen safety

The flow should remain safe at the edges:

- `done` or build/start actions inside a category path must remain rejected
- returning to the main category screen must always refresh to the newest build
  gate and category-state truth before allowing completion

### Validation coverage

Automated validation must cover:

- deep category checkpoint resume
- reconnect replay for category-state and prompt-state
- stale revision refresh behavior
- web and TUI navigation through repeated deep `open` and `back` cycles

## Dependencies And Touched Surfaces

This slice should build on the recursive tree work in Phase 09 and the richer
snapshot semantics in Phase 10.

Likely touched surfaces:

- [planner-core/src/pipeline/steps/socratic/socratic_engine.rs](/home/thetu/planner/planner-core/src/pipeline/steps/socratic/socratic_engine.rs)
- [planner-server/src/session.rs](/home/thetu/planner/planner-server/src/session.rs)
- [planner-server/src/ws.rs](/home/thetu/planner/planner-server/src/ws.rs)
- [planner-server/src/ws_socratic.rs](/home/thetu/planner/planner-server/src/ws_socratic.rs)
- [planner-server/tests/server_integration.rs](/home/thetu/planner/planner-server/tests/server_integration.rs)
- [planner-tui/src/app.rs](/home/thetu/planner/planner-tui/src/app.rs)
- [planner-tui/src/pipeline.rs](/home/thetu/planner/planner-tui/src/pipeline.rs)
- [planner-web/src/hooks/useSocraticWebSocket.ts](/home/thetu/planner/planner-web/src/hooks/useSocraticWebSocket.ts)
- [planner-web/src/pages/__tests__/SessionPage.test.tsx](/home/thetu/planner/planner-web/src/pages/__tests__/SessionPage.test.tsx)
- [planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx](/home/thetu/planner/planner-web/src/hooks/__tests__/useSocraticWebSocket.test.tsx)

Implementation should stay bounded to replay correctness and validation. If the
work starts broadening into offline support, collaborative editing, or general
session-restore redesign, stop and split that into a later spec.

## Acceptance Criteria

- reconnect restores the correct active category or prompt surface for deep
  category paths
- checkpoint resume preserves deep category breadcrumbs and prompt context
- stale category revision requests return the latest authoritative snapshot
- `done` remains rejected inside categories and accepted only from the refreshed
  main screen when build-ready
- automated coverage exists for deep navigation replay and stale-state recovery

## Verification Plan

### Core and server

- engine tests proving deep category replay and resume behavior
- websocket tests proving stale revisions refresh correctly and do not corrupt
  checkpoint state
- checkpoint serde and restore tests for deep paths and recursive snapshots

### Web and TUI

- web tests proving reconnect and stale-refresh flows update the UI cleanly
- TUI tests proving repeated deep `open` and `back` cycles preserve the correct
  visible state

### Manual

- one bounded manual check proving the live lobby survives:
  - entering a deep category path
  - answering a prompt
  - reconnecting or refreshing
  - returning to the main category screen and completing the flow safely

## Rollback And Fallback

- if a deep replay edge case remains unstable, prefer forcing a refresh to the
  latest main category snapshot over leaving the client on an ambiguous stale
  path
- if one client surface lags behind, keep the server contract authoritative and
  temporarily degrade that client to a simpler replay path rather than relaxing
  revision safety

## Open Questions

None.

## Implementation Outcome

This slice is implemented.

Delivered behavior:

- prompt-state replay now preserves deep `category_path` breadcrumbs on resume
  and reconnect
- category-state replay restores the latest deep snapshot as the only active
  surface when no prompt is pending
- stale `enter_category` revisions refresh back to the latest authoritative
  `category_state`
- `done` from an active prompt no longer short-circuits convergence; it
  refreshes back to the main category screen so build gating is re-evaluated
- the web session page no longer exposes a prompt-surface build CTA; build
  completion remains on the main category screen
- TUI category refresh handling and build-request wiring now align with the
  main-screen-only completion rule

Verification completed:

- `cargo test -p planner-core done_during_prompt_loop_refreshes_main_category_screen -- --nocapture`
- `cargo test -p planner-core checkpoint_resume_reemits_pending_prompt_with_deep_category_path -- --nocapture`
- `cargo test -p planner-core stale_category_revision_replays_latest_snapshot -- --nocapture`
- `cargo test -p planner-core recursive_category_entry_emits_nested_prompt_path -- --nocapture`
- `cargo test -p planner-server current_interview_replay_message_replays_checkpoint_ -- --nocapture`
- `cargo test -p planner-server build_checkpoint_resume_state_restores_deep_category_snapshot -- --nocapture`
- `cargo test -p planner-server disk_backed_store_persists_deep_category_checkpoint -- --nocapture`
- `cargo test -p planner-tui tick_socratic_category_state_ -- --nocapture`
- `npm test -- --run src/hooks/__tests__/useSocraticWebSocket.test.tsx src/pages/__tests__/SessionPage.test.tsx`

Manual end-to-end browser verification was not run in this delivery slice.
