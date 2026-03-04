//! # Observability — Structured Event System
//!
//! Provides `PlannerEvent`, `EventSink`, and concrete sink implementations.
//! All pipeline components emit events through `EventSink`; the server and
//! TUI inject their own implementations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// PlannerEvent
// ---------------------------------------------------------------------------

/// A structured event emitted by any Planner component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerEvent {
    /// Unique event ID.
    pub id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Severity level.
    pub level: EventLevel,
    /// Which component emitted this event.
    pub source: EventSource,
    /// Session ID, if the event is scoped to a session.
    pub session_id: Option<Uuid>,
    /// The step or operation that produced this event.
    pub step: Option<String>,
    /// Human-readable event description.
    pub message: String,
    /// Duration in milliseconds (for timed operations like LLM calls).
    pub duration_ms: Option<u64>,
    /// Arbitrary structured metadata (model, tokens, exit_code, etc.).
    pub metadata: serde_json::Value,
}

/// Event severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Info,
    Warn,
    Error,
}

/// Which subsystem emitted the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    SocraticEngine,
    LlmRouter,
    Factory,
    Pipeline,
    System,
}

impl PlannerEvent {
    /// Create a new Info-level event.
    pub fn info(source: EventSource, step: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: EventLevel::Info,
            source,
            session_id: None,
            step: Some(step.into()),
            message: message.into(),
            duration_ms: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a new Warn-level event.
    pub fn warn(source: EventSource, step: impl Into<String>, message: impl Into<String>) -> Self {
        let mut e = Self::info(source, step, message);
        e.level = EventLevel::Warn;
        e
    }

    /// Create a new Error-level event.
    pub fn error(source: EventSource, step: impl Into<String>, message: impl Into<String>) -> Self {
        let mut e = Self::info(source, step, message);
        e.level = EventLevel::Error;
        e
    }

    /// Set session_id.
    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set duration_ms.
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

impl std::fmt::Display for EventLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventLevel::Info => write!(f, "INFO"),
            EventLevel::Warn => write!(f, "WARN"),
            EventLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl std::fmt::Display for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSource::SocraticEngine => write!(f, "socratic"),
            EventSource::LlmRouter => write!(f, "llm_router"),
            EventSource::Factory => write!(f, "factory"),
            EventSource::Pipeline => write!(f, "pipeline"),
            EventSource::System => write!(f, "system"),
        }
    }
}

// ---------------------------------------------------------------------------
// EventSink trait
// ---------------------------------------------------------------------------

/// Trait for receiving structured events from pipeline components.
///
/// Both the server and TUI inject their own implementations. Components
/// receive `&dyn EventSink` (or `Arc<dyn EventSink>`) to emit events.
pub trait EventSink: Send + Sync {
    /// Emit a single event.
    fn emit(&self, event: PlannerEvent);
}

// ---------------------------------------------------------------------------
// Concrete sinks
// ---------------------------------------------------------------------------

/// A no-op sink that discards all events. Used in tests and when
/// observability is not needed.
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn emit(&self, _event: PlannerEvent) {}
}

/// A sink that forwards events through a tokio mpsc channel.
/// The receiver end collects events into the session event log.
pub struct ChannelEventSink {
    tx: tokio::sync::mpsc::UnboundedSender<PlannerEvent>,
}

impl ChannelEventSink {
    /// Create a new channel sink and its corresponding receiver.
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<PlannerEvent>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    /// Create a sink from an existing sender (when the receiver is managed externally).
    pub fn from_sender(tx: tokio::sync::mpsc::UnboundedSender<PlannerEvent>) -> Self {
        Self { tx }
    }
}

impl EventSink for ChannelEventSink {
    fn emit(&self, event: PlannerEvent) {
        let _ = self.tx.send(event);
    }
}

/// A sink that collects events into a Vec behind a Mutex.
/// Useful for tests where you want to inspect emitted events.
pub struct CollectorEventSink {
    events: std::sync::Mutex<Vec<PlannerEvent>>,
}

impl CollectorEventSink {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Get a snapshot of all collected events.
    pub fn events(&self) -> Vec<PlannerEvent> {
        self.events.lock().unwrap().clone()
    }

    /// How many events have been collected.
    pub fn count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl EventSink for CollectorEventSink {
    fn emit(&self, event: PlannerEvent) {
        self.events.lock().unwrap().push(event);
    }
}

// ---------------------------------------------------------------------------
// Event Persistence
// ---------------------------------------------------------------------------

/// Filesystem-backed event store. Persists session events as MessagePack.
pub struct EventStore {
    events_dir: std::path::PathBuf,
}

impl EventStore {
    /// Create a new EventStore, creating the events directory if needed.
    pub fn new(data_dir: &std::path::Path) -> std::io::Result<Self> {
        let events_dir = data_dir.join("events");
        std::fs::create_dir_all(&events_dir)?;
        Ok(Self { events_dir })
    }

    /// Persist all events for a session.
    pub fn save_session_events(&self, session_id: Uuid, events: &[PlannerEvent]) -> std::io::Result<()> {
        let path = self.events_dir.join(format!("{}.msgpack", session_id));
        let bytes = rmp_serde::to_vec(events).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("MessagePack encode: {}", e))
        })?;
        std::fs::write(&path, &bytes)
    }

    /// Load events for a session. Returns empty vec if file doesn't exist.
    pub fn load_session_events(&self, session_id: Uuid) -> std::io::Result<Vec<PlannerEvent>> {
        let path = self.events_dir.join(format!("{}.msgpack", session_id));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read(&path)?;
        rmp_serde::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("MessagePack decode: {}", e))
        })
    }

    /// List all session IDs that have persisted events.
    pub fn list_sessions(&self) -> std::io::Result<Vec<Uuid>> {
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.events_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id_str) = name.strip_suffix(".msgpack") {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }

    /// Delete events for a session.
    pub fn delete_session_events(&self, session_id: Uuid) -> std::io::Result<()> {
        let path = self.events_dir.join(format!("{}.msgpack", session_id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_event_info() {
        let event = PlannerEvent::info(
            EventSource::LlmRouter,
            "llm.call.start",
            "Starting LLM call to claude-opus-4-6",
        );
        assert_eq!(event.level, EventLevel::Info);
        assert_eq!(event.source, EventSource::LlmRouter);
        assert_eq!(event.step.as_deref(), Some("llm.call.start"));
        assert!(event.duration_ms.is_none());
    }

    #[test]
    fn planner_event_builder() {
        let sid = Uuid::new_v4();
        let event = PlannerEvent::info(EventSource::Pipeline, "compile", "Compiling NLSpec")
            .with_session(sid)
            .with_duration(1234)
            .with_metadata(serde_json::json!({"model": "claude-opus-4-6"}));

        assert_eq!(event.session_id, Some(sid));
        assert_eq!(event.duration_ms, Some(1234));
        assert_eq!(event.metadata["model"], "claude-opus-4-6");
    }

    #[test]
    fn planner_event_serialization() {
        let event = PlannerEvent::warn(EventSource::System, "system.startup", "No providers found");
        let json = serde_json::to_string(&event).unwrap();
        let parsed: PlannerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.level, EventLevel::Warn);
        assert_eq!(parsed.source, EventSource::System);
        assert_eq!(parsed.message, "No providers found");
    }

    #[test]
    fn noop_sink_does_not_panic() {
        let sink = NoopEventSink;
        sink.emit(PlannerEvent::info(EventSource::System, "test", "test event"));
    }

    #[test]
    fn collector_sink_collects() {
        let sink = CollectorEventSink::new();
        sink.emit(PlannerEvent::info(EventSource::LlmRouter, "a", "first"));
        sink.emit(PlannerEvent::error(EventSource::Pipeline, "b", "second"));
        assert_eq!(sink.count(), 2);
        let events = sink.events();
        assert_eq!(events[0].level, EventLevel::Info);
        assert_eq!(events[1].level, EventLevel::Error);
    }

    #[tokio::test]
    async fn channel_sink_sends() {
        let (sink, mut rx) = ChannelEventSink::new();
        sink.emit(PlannerEvent::info(EventSource::SocraticEngine, "test", "hello"));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.message, "hello");
        assert_eq!(event.source, EventSource::SocraticEngine);
    }

    #[test]
    fn event_level_display() {
        assert_eq!(format!("{}", EventLevel::Info), "INFO");
        assert_eq!(format!("{}", EventLevel::Warn), "WARN");
        assert_eq!(format!("{}", EventLevel::Error), "ERROR");
    }

    #[test]
    fn event_source_display() {
        assert_eq!(format!("{}", EventSource::LlmRouter), "llm_router");
        assert_eq!(format!("{}", EventSource::SocraticEngine), "socratic");
        assert_eq!(format!("{}", EventSource::Pipeline), "pipeline");
    }

    #[test]
    fn event_store_save_and_load() {
        let dir = std::env::temp_dir().join(format!("planner_test_{}", Uuid::new_v4()));
        let store = EventStore::new(&dir).unwrap();

        let sid = Uuid::new_v4();
        let events = vec![
            PlannerEvent::info(EventSource::System, "test.start", "Started"),
            PlannerEvent::error(EventSource::LlmRouter, "test.fail", "Failed"),
        ];

        store.save_session_events(sid, &events).unwrap();

        let loaded = store.load_session_events(sid).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].message, "Started");
        assert_eq!(loaded[1].level, EventLevel::Error);

        let ids = store.list_sessions().unwrap();
        assert!(ids.contains(&sid));

        store.delete_session_events(sid).unwrap();
        assert!(store.load_session_events(sid).unwrap().is_empty());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn event_store_load_missing_returns_empty() {
        let dir = std::env::temp_dir().join(format!("planner_test_{}", Uuid::new_v4()));
        let store = EventStore::new(&dir).unwrap();
        let events = store.load_session_events(Uuid::new_v4()).unwrap();
        assert!(events.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
