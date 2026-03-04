# Admin & Observability Plan — Revised v3

## Design Principle

Observability lives **inside the session**, not in a separate admin ghetto.
When you click into a session, you should immediately see: what phase it's in,
what's running right now, what failed, and how to drill into the details.
A global admin page exists too, but it's a summary — the session is where
you debug.

---

## 1. Server: Structured Event System

### 1a. PlannerEvent type (planner-core)

Lives in `planner-core` — not planner-server — because the engine, LLM router,
and pipeline steps all emit events from core code. The server and TUI each
create an `EventSink` and inject it into the call chain.

```rust
pub struct PlannerEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub level: EventLevel,        // Info, Warn, Error
    pub source: EventSource,      // SocraticEngine, LlmRouter, Factory, Pipeline, System
    pub session_id: Option<Uuid>,
    pub step: Option<String>,     // "classify_domain", "verify_and_update", "Compile", etc.
    pub message: String,
    pub duration_ms: Option<u64>, // for timed operations (LLM calls, steps)
    pub metadata: serde_json::Value, // model, tokens_in, tokens_out, exit_code, etc.
}

pub enum EventLevel { Info, Warn, Error }
pub enum EventSource { SocraticEngine, LlmRouter, Factory, Pipeline, System }
```

### 1b. EventSink trait (planner-core)

```rust
pub trait EventSink: Send + Sync {
    fn emit(&self, event: PlannerEvent);
}
```

A concrete `NoopEventSink` for tests and a `ChannelEventSink` (wraps
`mpsc::UnboundedSender<PlannerEvent>`) for real use.

**No EventBus struct in planner-server for Phase A.** Events attach to the
Session as `Vec<PlannerEvent>`. The global ring buffer is deferred to Phase B
when the admin page needs it.

### 1c. Session model additions

Add to `Session` struct (planner-server/session.rs):
```rust
pub events: Vec<PlannerEvent>,         // full event log for this session
pub current_step: Option<String>,      // what's executing right now
pub error_message: Option<String>,     // last error (for quick display)
```

Counters like `llm_calls_count` and `llm_total_latency_ms` are **derived** from
the event log via methods — not stored as separate fields.

### 1d. New REST endpoints (Phase A)

```
GET /api/sessions/:id/events              — session event log (paginated)
GET /api/sessions/:id/events?level=error  — filtered by level
```

### 1e. WebSocket: ServerMessage::Event variant

Events flow through the **existing** WebSocket channel. No separate broadcast
infrastructure. A new `ServerMessage::Event` variant carries `PlannerEvent`
data to the client in real time.

---

## 2. Web UI: Session-Level Observability

### 2a. SessionStatusHeader (above existing bars, not replacing them)

A thin bar above ConvergenceBar/PipelineBar that shows live operational status:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ● classify_domain (2.3s)  │  3 LLM calls  │  [Logs]                   │
└──────────────────────────────────────────────────────────────────────────┘
```

In error state:
```
┌──────────────────────────────────────────────────────────────────────────┐
│  ✕ classify_domain failed: CLI binary not found: claude  │  [Logs]      │
└──────────────────────────────────────────────────────────────────────────┘
```

Key elements:
- **Current step** with elapsed time (detects hangs visually)
- **LLM call counter** (know if any calls are happening at all)
- **[Logs] button** that toggles the right panel to event log view
- **Does NOT replace** ConvergenceBar or PipelineBar — sits above them

### 2b. Right Panel: Logs Tab

Add a third mode to the right panel: **EventLogPanel**.

Toggle tabs at the top of the right panel:
```
  [Belief State]  [Draft]  [Logs]
```

EventLogPanel shows:
- Scrollable list of PlannerEvents for this session
- Color-coded by level (info=dim, warn=yellow, error=red)
- Expandable rows for metadata (LLM call details, error stack)
- Filter chips: All | Errors | LLM Calls | State Changes
- Timestamps as relative ("2s ago", "1m ago") with hover for absolute

### 2c. Dashboard Session List Enhancement (Phase B)

Add columns:
- **Status** — colored dot + phase label
- **Current Step** — what's executing right now (or last completed)
- **Errors** — error count badge (red)
- **Duration** — total session time

---

## 3. TUI: Matching Observability

### 3a. Status Bar Enhancement

Current bottom status bar gets enriched:
```
[phase: INTERVIEWING]  [step: classify_domain 2.3s]  [llm: 3 calls]  [L: logs]
```

### 3b. Logs Panel (Tab toggle)

Tab key cycles: Chat | BeliefState | Logs

Logs panel in TUI shows the same event stream as the web EventLogPanel,
rendered as ratatui List with colored items. j/k to scroll, f to filter.

### 3c. Provider Status on Startup (Phase B)

TUI startup screen shows provider detection results before entering
the interview:
```
  Planner v2 — Socratic Planning Engine

  LLM Providers:
    ✓ anthropic (claude v1.0.34)
    ✗ google (gemini not found)
    ✓ openai (codex v0.104.0)

  Press Enter to start...
```

---

## 4. Implementation Phases

### Phase A — Event Foundation + Session Status (build first)

1. **PlannerEvent struct + EventLevel + EventSource** in planner-core
2. **EventSink trait + NoopEventSink + ChannelEventSink** in planner-core
3. **Vec<PlannerEvent> on Session** + current_step + error_message fields
4. **Instrument run_cli()**: wrap with timing events via EventSink
5. **Instrument Socratic engine**: emit events at each phase transition
6. **Instrument pipeline stages**: emit events at start/complete/fail
7. **`GET /api/sessions/:id/events`** endpoint
8. **ServerMessage::Event** variant on the existing WebSocket
9. **Web: SessionStatusHeader** component (above existing bars)
10. **Web: EventLogPanel** as third right-panel tab
11. **TUI: enhanced status bar** with current step + timing
12. **TUI: Logs panel** (Tab toggle)

### Phase B — Dashboard + Persistence

13. **Dashboard enhancements** — status dots, error badges, duration column
14. **Event persistence to disk** — MessagePack, alongside CXDB data
15. **TUI: startup provider check screen**
16. **Global EventBus** with ring buffer (for admin page in Phase C)

### Phase C — Admin Page + Live Streaming

17. **`GET /api/admin/status`** with provider versions, uptime, memory
18. **Web: /admin page** with system health + provider cards
19. **Live event streaming** if polling proves insufficient
20. **`/api/admin/events/ws`** WebSocket for global log tail

---

## 5. Event Instrumentation Points

### LLM Router (run_cli wrapper)
```
llm.call.start    { model, provider, prompt_len }
llm.call.complete { model, provider, duration_ms, tokens_in, tokens_out }
llm.call.error    { model, provider, duration_ms, exit_code, stderr_preview }
llm.call.timeout  { model, provider, timeout_secs }
```

### Socratic Engine
```
socratic.classify.start     { description_len }
socratic.classify.complete  { project_type, complexity, question_budget, duration_ms }
socratic.verify.start       { turn_number, input_len }
socratic.verify.complete    { slots_updated: [...], duration_ms }
socratic.question.generated { dimension, question_text }
socratic.draft.triggered    { convergence_pct }
socratic.converged          { reason, convergence_pct, total_turns }
socratic.error              { step, error }
```

### Pipeline Steps
```
pipeline.stage.start    { stage_name }
pipeline.stage.complete { stage_name, duration_ms }
pipeline.stage.error    { stage_name, error }
pipeline.complete       { total_duration_ms, stages_completed }
```

### System
```
system.startup          { version, providers: [...], hostname, pid }
system.provider.check   { provider, available, binary_path, version, error }
system.session.created  { session_id, user_id }
system.session.error    { session_id, error }
```

---

## 6. Design Constraints

- No external dependencies (no Prometheus, Grafana, ELK)
- Events stored on Session in memory (Phase A), persisted to disk (Phase B)
- Both Web and TUI get the same data, just rendered differently
- All endpoints behind the same auth as session endpoints
- No stubs — every endpoint returns real data from day one
- Session is the primary debugging surface, admin page is the overview
- EventSink lives in planner-core; server and TUI inject their own implementations
