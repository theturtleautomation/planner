# Phase 05 Session Lobby And Events Implementation

**Status:** Research complete, ready for implementation  
**Date:** 2026-03-07

## Objective

Separate operational events from the main conversation workspace and rebuild the
session page as a clearer session lobby with an explicit events table.

This phase is complete when the left conversation pane only shows human-facing
messages, the session lobby has a dedicated `Events` review surface, and the
event source of truth is explicit across REST, websocket, persistence, and UI
state.

## Non-Goals

- redesign the project model or project-first route tree from Phases 0 and 1
- redesign the belief-state or speculative-draft workflows themselves
- introduce global log search, saved log views, or admin-style analytics inside
  the session page
- change the `PlannerEvent` schema beyond consuming the fields already emitted
- replace the existing session export format in this phase
- add a separate `/session/:id/events` route or a full-screen event viewer
- perform a destructive backfill that rewrites all historical session message
  transcripts on disk

## Decision Summary

- `PlannerEvent[]` becomes the single operational event stream for session UI,
  persistence, websocket updates, and the new events table.
- Raw `ChatMessage` records with `role: "event"` should stop being emitted by
  `WsSocraticIO` and should stop being treated as a first-class chat surface.
- Checkpoint persistence must be decoupled from `role: "event"` chat message
  serialization. Raw `SocraticEvent` values should update checkpoint state
  directly on the server.
- The session page remains a split lobby, but the right pane becomes an
  explicit three-tab context area:
  - `Belief State`
  - `Draft`
  - `Events`
- The current footer-style `EventLogPanel` should be replaced by a real
  `SessionEventsTable`, not promoted as-is.
- The canonical event review surface should use the existing
  `GET /sessions/:id/events` endpoint for historical hydration and websocket
  `planner_event` messages for live updates.
- Live progress remains visible while the `Events` tab is closed through header
  counts, status color, and unread indicators.

## Current-State Summary

The current session UI already has a structured event log, but it still mixes
operational telemetry into the conversation transport and into the layout.

| Surface | Current behavior | Current issue |
| --- | --- | --- |
| Socratic transport | `WsSocraticIO.send_event()` serializes raw `SocraticEvent` values into `ChatMessage { role: "event" }` | event telemetry still enters the same message stream as user and planner conversation |
| Structured observability | pipeline and runtime instrumentation also emit `planner_event` websocket messages and persist `Session.events` | there are two parallel event channels with different shapes and different UI consumers |
| Client state | `useSocraticWebSocket` keeps both `messages` and `events`, appending chat `message` payloads to one array and `planner_event` payloads to another | the session page has no single declared event source of truth |
| Chat UI | `ChatPanel` contains a dedicated collapsible renderer for `role === "event"` | the main display box still shows operational telemetry that the user explicitly wants moved elsewhere |
| Session layout | `SessionPage` renders `Belief State` and `Draft` in the right pane, then places `EventLogPanel` as a collapsible footer below them | events feel secondary and cramped instead of being a first-class review surface |
| Resume path | `handle_resume_ws()` streams new chat messages and stage updates, but does not stream incremental `planner_event` updates after attach | refreshed or resumed pipeline pages cannot rely on the live event log staying current |
| REST event API | `GET /sessions/:id/events` already supports `level`, `source`, `limit`, and `offset` | the session page does not use the dedicated event API it already has |
| Tests | web and server tests explicitly encode `role: "event"` behavior | the current duplication is locked into the test suite |

### Current code anchors

- `planner-server/src/ws_socratic.rs`
- `planner-server/src/ws.rs`
- `planner-server/src/api.rs`
- `planner-server/src/session.rs`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/components/ChatPanel.tsx`
- `planner-web/src/components/EventLogPanel.tsx`
- `planner-web/src/components/SessionStatusHeader.tsx`
- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/components/__tests__/ChatPanel.test.tsx`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`

### Code findings that make the current model unsafe to keep

- `WsSocraticIO.send_event()` currently emits contradiction and other Socratic
  events as raw chat JSON with `role: "event"`.
- `apply_checkpoint_from_server_message()` only updates checkpoint state by
  parsing those serialized chat events, which means the checkpoint path is
  incorrectly coupled to a chat-only transport detail.
- `Session.record_event()` already updates `current_step`, `error_message`, and
  `events`, so the backend already has a better operational source than chat.
- `get_session_events()` already exposes structured event filtering and
  pagination, but the frontend still hydrates events indirectly via the full
  session snapshot.
- `handle_resume_ws()` does not mirror incremental `planner_event` streaming,
  which is acceptable for a footer preview but not acceptable for a canonical
  live events table.

### Visual audit findings from 2026-03-07

- The current waiting state still inherits dashboard-era chrome such as
  `Back to Dashboard`, which reinforces that the session page is treated as a
  detached workflow rather than a project-scoped lobby.
- A browser replay of the current `SessionPage` using interview-state data
  shows `role: "event"` messages appearing inline in the left transcript as
  gold event rows while the right pane still reserves space for a footer event
  strip below `Belief State` or `Draft`.
- That layout creates three competing operational channels at once: the status
  header, inline chat events, and footer events. The proposed `Events` tab is
  therefore an information architecture correction, not just a component swap.

## Proposed Behavior

### Session Lobby Information Architecture

Treat `/session/:id` as a session lobby with three stable zones once the
session has moved past the initial waiting form:

1. Header and status strip.
2. Left conversation workspace.
3. Right context workspace with explicit tabs.

The waiting state can keep its current description-first layout. The lobby
changes apply to `interviewing`, `pipeline_running`, `complete`, and `error`
states.

### Lobby layout

| Region | Content | Rules |
| --- | --- | --- |
| Header | `SessionStatusHeader`, actions, convergence or pipeline bar | always visible once the session has started or has resumable state |
| Left pane | conversation transcript plus `MessageInput` | conversation-only surface; no operational event rows |
| Right pane | `Belief State`, `Draft`, `Events` tabs | one active tab at a time; no footer-style event panel |

### Tab rules

- `Belief State` remains the default tab.
- `Draft` keeps its current availability rules and disabled state when no draft
  exists.
- `Events` should always be visible once the session has entered the lobby,
  even when the table is empty.
- Opening `Events` does not pause live updates.
- If new events arrive while another tab is active, the `Events` tab should
  show an unread indicator until the user opens it.

### Conversation, Notices, And Events

The lobby should make the difference between user-facing dialogue and
operational telemetry explicit.

### Conversation transcript

The main chat pane should contain only:

- `user` messages
- `planner` messages
- `system` notices

### `planner` messages

Keep `planner` messages for content intentionally written to the user, such as:

- interview questions
- classification summaries
- speculative draft prompts
- convergence or pipeline-complete narration

### `system` notices

Keep `system` messages for sparse, user-impacting notices rather than
continuous telemetry, for example:

- contradiction notices
- connection loss or reconnect failure
- explicit restart or retry failures
- user shortcut echoes such as skip or done

These may still have corresponding `PlannerEvent` records in the event table.
That duplication is acceptable because those notices are user-facing outcomes,
not raw step-by-step instrumentation.

### Operational events

Operational events should live only in `PlannerEvent[]` and in the `Events`
tab. They should not be rendered inline in the chat transcript.

### Legacy compatibility rule

Historical sessions may already contain `role: "event"` chat messages on disk.
Phase 5 should hide those from the conversation UI even before any cleanup
backfill runs.

Recommended rule:

- stop writing new `role: "event"` messages immediately
- ignore legacy `role: "event"` messages on read in the session page or websocket
  hook
- remove the `event` chat renderer after the migration window

### Events Table Design

Replace the current `EventLogPanel` footer with a new table-oriented component,
for example:

- new: `planner-web/src/components/SessionEventsTable.tsx`

The old `EventLogPanel` should not remain as the main session event surface.
At most, small pieces of row-formatting logic may be extracted and reused.

### Table columns

The events table should use these columns in this order:

| Column | Purpose |
| --- | --- |
| `Time` | local wall-clock timestamp for quick review; full timestamp on hover |
| `Level` | `info`, `warn`, `error` |
| `Source` | `socratic_engine`, `llm_router`, `pipeline`, `factory`, `system` |
| `Step` | current step or event step, when present |
| `Message` | primary event summary text |
| `Duration` | duration badge or blank when unavailable |

### Sort order

- default sort should be newest first
- Phase 5 does not need arbitrary column sorting
- websocket updates should insert new rows at the top when the default sort is
  active

### Filters

Phase 5 only needs lightweight operational filters:

- level filter:
  - `All`
  - `Errors`
  - `Warnings`
- source filter:
  - `All`
  - `Socratic`
  - `LLM`
  - `Pipeline`
  - `Factory`
  - `System`

Do not add full-text search, saved filters, or date-range pickers in this
phase.

### Expansion behavior

- rows with metadata or duration details are expandable
- expanding a row reveals:
  - full message text
  - formatted metadata
  - duration, if present
- keep the default row height compact enough that the table still reads as a
  table rather than a card list

### Empty and live states

- empty state text: `No events yet. Live session activity will appear here.`
- while live updates are arriving, the table should update in place without
  forcing the tab to open
- the header and tab label should continue to show warning and error counts
  even when the user is reading another tab

### Event Data Flow And Source Of Truth

Phase 5 needs one explicit event path on both server and client.

### Backend event source of truth

The canonical operational source should be:

1. raw runtime instrumentation and Socratic events
2. `Session.record_event()` persistence into `Session.events`
3. `GET /sessions/:id/events` for historical reads
4. websocket `planner_event` messages for incremental live updates

### Remove raw event-chat transport

`WsSocraticIO.send_event()` should stop serializing raw `SocraticEvent` values
into chat messages for the client.

Recommended replacement:

- keep the typed websocket messages already used by the client:
  - `classified`
  - `belief_state_update`
  - `question`
  - `speculative_draft`
  - `converged`
  - `contradiction_detected`
- keep `planner_event` as the operational stream
- do not emit `message` payloads whose role is `event`

### Checkpoint update refactor

Checkpoint logic should no longer depend on parsing chat JSON back into
`SocraticEvent`.

Recommended implementation shape:

- introduce a direct checkpoint projector for raw `SocraticEvent`
- call it from the server runtime path before or while dispatching typed
  websocket messages
- delete the dependency on `apply_checkpoint_from_server_message()` for
  `role: "event"` chat payloads

This is the most important backend change in Phase 5. Without it, removing
chat events would silently break interview resume and checkpoint persistence.

### Resume websocket parity

`handle_resume_ws()` must start streaming incremental `planner_event` updates,
not only chat messages and stage changes.

Recommended parity rule:

- live runtime attach and resumed pipeline attach should both forward new
  `planner_event` records
- both paths should track a `last_event_count` the same way they already track
  `last_msg_count`

Without this change, the `Events` tab will look correct on first load but stop
updating after refresh or resume.

### Frontend hydration model

The session lobby should stop treating `initialSession.events` as the canonical
event load path.

Recommended model:

1. `GET /sessions/:id` hydrates core session state and conversation.
2. `GET /sessions/:id/events` hydrates the initial event table.
3. websocket `planner_event` payloads append live updates.
4. client-side dedupe by `PlannerEvent.id` prevents duplicate rows after
   reconnect or endpoint refetch.

Keeping `session.events` in the session payload is acceptable during the
migration window, but the new table should not depend on it once the explicit
event query exists on the client.

## Impacted Files And Modules

### Backend event transport and checkpointing

- `planner-server/src/ws_socratic.rs`
- `planner-server/src/api.rs`
- `planner-server/src/session.rs`

### Frontend session lobby

- `planner-web/src/pages/SessionPage.tsx`
- `planner-web/src/components/SessionStatusHeader.tsx`
- new: `planner-web/src/components/SessionEventsTable.tsx`
- `planner-web/src/components/ChatPanel.tsx`
- optional delete or retire: `planner-web/src/components/EventLogPanel.tsx`
- `planner-web/src/hooks/useSocraticWebSocket.ts`
- `planner-web/src/api/client.ts`
- `planner-web/src/types.ts`
- `planner-web/src/index.css`

### Tests

- `planner-server/src/ws_socratic.rs`
- `planner-server/src/api.rs`
- `planner-web/src/components/__tests__/ChatPanel.test.tsx`
- new: `planner-web/src/components/__tests__/SessionEventsTable.test.tsx`
- `planner-web/src/pages/__tests__/SessionPage.test.tsx`

## API And Data Model Changes

Phase 5 does not require a brand-new backend route, but it does require the
current event APIs and message contracts to become explicit.

### Backend changes

- keep `GET /sessions/:id/events`
- keep websocket `planner_event`
- stop emitting websocket `message` payloads with `role: "event"`
- keep `PlannerEvent` as the structured event payload for persistence and UI

### Frontend API changes

Add client support for the existing session event endpoint, for example:

```ts
interface SessionEventsResponse {
  session_id: string;
  events: PlannerEvent[];
  count: number;
}
```

Add a client method such as:

- `getSessionEvents(id, params?)`

Supported params should mirror the current server query contract:

- `level`
- `source`
- `limit`
- `offset`

### Message-role migration

Short term:

- keep the frontend type system tolerant of legacy `event` roles so old data
  can still decode cleanly
- filter that role out before rendering the conversation pane

Long term:

- remove `event` from the web chat-role union once new sessions no longer
  persist it and any compatibility window has ended

## UI And Routing Changes

### Session route scope

No route changes are required in Phase 5.

The canonical route remains:

- `/session/:id`

The `Events` view is a tab inside the session lobby, not a separate route or
subview in this phase.

### Session page layout changes

Apply these changes to the lobby states:

- replace the right-pane footer `EventLogPanel` with a third `Events` tab
- keep `Belief State` and `Draft` as sibling tabs
- keep `MessageInput` below the conversation pane
- remove inline operational events from `ChatPanel`

### Header behavior

`SessionStatusHeader` should continue to show:

- health color
- current step
- elapsed time
- LLM call count
- workflow actions

It should also expose event awareness more explicitly:

- total event count
- error and warning counts
- clickable event summary or button that opens the `Events` tab

### Responsive behavior

The right-pane tab model should remain stable across sizes.

Recommended behavior:

- desktop: standard split-pane layout with three right-pane tabs
- narrow widths: keep the tab model, but let the tab bar scroll horizontally if
  needed
- do not fall back to a footer event tray on smaller screens

## Migration And Backfill Plan

### Step 1: Decouple checkpoints from chat events

- move checkpoint projection to raw `SocraticEvent` handling on the server
- verify checkpoint resume before touching the chat renderer

### Step 2: Restore websocket parity for resumed sessions

- add incremental `planner_event` forwarding to `handle_resume_ws()`
- dedupe event delivery across resume and reconnect flows

### Step 3: Promote the explicit events API in the client

- add `getSessionEvents()` to the web API client
- hydrate the new `SessionEventsTable` from the event endpoint
- append websocket `planner_event` updates by ID

### Step 4: Remove event rows from the conversation UI

- stop writing new `role: "event"` messages
- ignore legacy `role: "event"` messages in the lobby UI
- delete the event-specific branch from `ChatPanel` once the compatibility path
  is no longer needed

### Historical data handling

No destructive session-data migration is required for the first cut.

Historical sessions may keep legacy `role: "event"` messages on disk, but the
Phase 5 UI should treat them as hidden legacy data rather than user-visible
conversation.

## Tests To Add Or Update

### Backend tests

- update `ws_socratic_io_send_event_contradiction` to stop expecting a second
  `ChatMessage` with `role: "event"`
- add a test proving checkpoint state still updates when raw `SocraticEvent`
  values are projected directly
- add a resume-path test proving `handle_resume_ws()` streams new
  `planner_event` records after attach
- keep `get_session_events` coverage for filtering and pagination

### Frontend component tests

- remove or replace `ChatPanel` tests that expect event messages to render in
  the chat pane
- add `SessionEventsTable` tests for:
  - newest-first ordering
  - level and source filters
  - row expansion for metadata
  - empty state rendering

### Session page tests

- verify the right pane exposes `Belief State`, `Draft`, and `Events`
- verify clicking the header event affordance opens the `Events` tab
- verify legacy `role: "event"` messages do not appear in the conversation pane
- verify live `planner_event` updates appear in the events tab without opening
  the tab automatically
- verify unread indicator behavior when new events arrive off-tab

## Risks, Dependencies, And Rollout Order

### Main risks

- checkpoint resume regression if the event-chat coupling is removed without a
  direct checkpoint projector
- event duplication after reconnect if endpoint hydration and websocket updates
  are not deduped by event ID
- false confidence from the current resume path, which does not yet stream live
  `planner_event` updates
- UI crowding in the right pane if the table is implemented as oversized cards
  instead of a dense table

### Dependencies

- no blocking dependency on Phases 0 through 4
- Phase 5 can land independently on the current `/session/:id` route
- it does depend on preserving the existing `PlannerEvent` instrumentation and
  session persistence path

### Recommended rollout order

1. Backend checkpoint decoupling and resume-path event streaming.
2. Frontend event endpoint client and `SessionEventsTable`.
3. `SessionPage` tab integration and header affordance.
4. Legacy `role: "event"` filtering and chat cleanup.
5. Test updates across server and web.

## Unresolved Questions

- Should the `Events` tab auto-open when the first error-level event arrives,
  or should Phase 5 keep that as a passive badge-only signal?
- Should contradiction notices remain in the chat transcript long term, or
  eventually move entirely into `Belief State` plus `Events`?
- Once the explicit event endpoint is in use, should `GET /sessions/:id` keep
  embedding the full `events` array or eventually drop it to reduce payload
  size?
