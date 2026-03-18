//! # Socratic Runtime Registry
//!
//! Keeps live interview runtimes available across short websocket disconnects.
//! A runtime owns:
//! - an inbound user-input channel for the Socratic engine
//! - a broadcast channel for outbound server messages
//! - attachment metadata used for lease-based expiry

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use tokio::sync::{broadcast, mpsc, watch, Mutex as AsyncMutex};
use uuid::Uuid;

use planner_schemas::PromptResponse;

use crate::ws::ServerMessage;

const DEFAULT_BROADCAST_BUFFER: usize = 128;

#[derive(Debug, Clone)]
pub enum SocraticRuntimeInput {
    PromptResponse(PromptResponse),
    Done,
    DimensionEdit {
        dimension: String,
        new_value: String,
    },
}

pub struct RuntimeAttachment {
    pub input_tx: mpsc::UnboundedSender<SocraticRuntimeInput>,
    pub outbound_rx: broadcast::Receiver<ServerMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachError {
    AlreadyAttached,
    Closed,
}

#[derive(Debug)]
struct RuntimeLeaseState {
    attached: bool,
    detached_at: Option<Instant>,
}

pub struct SessionRuntime {
    input_tx: Mutex<Option<mpsc::UnboundedSender<SocraticRuntimeInput>>>,
    outbound_tx: broadcast::Sender<ServerMessage>,
    shutdown_tx: watch::Sender<bool>,
    lease_state: Mutex<RuntimeLeaseState>,
}

impl SessionRuntime {
    pub fn new() -> (
        Arc<Self>,
        Arc<AsyncMutex<mpsc::UnboundedReceiver<SocraticRuntimeInput>>>,
    ) {
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (outbound_tx, _) = broadcast::channel(DEFAULT_BROADCAST_BUFFER);
        let (shutdown_tx, _) = watch::channel(false);
        let runtime = Arc::new(Self {
            input_tx: Mutex::new(Some(input_tx)),
            outbound_tx,
            shutdown_tx,
            lease_state: Mutex::new(RuntimeLeaseState {
                attached: false,
                detached_at: None,
            }),
        });

        (runtime, Arc::new(AsyncMutex::new(input_rx)))
    }

    pub fn try_attach(&self) -> Result<RuntimeAttachment, AttachError> {
        let input_tx = self
            .input_tx
            .lock()
            .as_ref()
            .cloned()
            .ok_or(AttachError::Closed)?;

        let mut lease_state = self.lease_state.lock();
        if lease_state.attached {
            return Err(AttachError::AlreadyAttached);
        }

        lease_state.attached = true;
        lease_state.detached_at = None;

        Ok(RuntimeAttachment {
            input_tx,
            outbound_rx: self.outbound_tx.subscribe(),
        })
    }

    pub fn mark_detached(&self) {
        let mut lease_state = self.lease_state.lock();
        lease_state.attached = false;
        lease_state.detached_at = Some(Instant::now());
    }

    pub fn close_input(&self) {
        self.input_tx.lock().take();
    }

    pub fn publish(&self, msg: ServerMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    pub fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown_tx.subscribe()
    }

    pub fn signal_closed(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub fn is_detached_expired(&self, now: Instant, lease_duration: Duration) -> bool {
        let lease_state = self.lease_state.lock();
        if lease_state.attached {
            return false;
        }

        lease_state
            .detached_at
            .map(|detached_at| now.duration_since(detached_at) >= lease_duration)
            .unwrap_or(false)
    }
}

pub struct SessionRuntimeRegistry {
    runtimes: RwLock<HashMap<Uuid, Arc<SessionRuntime>>>,
    lease_duration: Duration,
}

impl SessionRuntimeRegistry {
    pub fn new(lease_duration: Duration) -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
            lease_duration,
        }
    }

    pub fn lease_duration(&self) -> Duration {
        self.lease_duration
    }

    pub fn get(&self, session_id: Uuid) -> Option<Arc<SessionRuntime>> {
        self.runtimes.read().get(&session_id).cloned()
    }

    pub fn try_insert(
        &self,
        session_id: Uuid,
        runtime: Arc<SessionRuntime>,
    ) -> Result<(), Arc<SessionRuntime>> {
        let mut runtimes = self.runtimes.write();
        if let Some(existing) = runtimes.get(&session_id) {
            return Err(existing.clone());
        }
        runtimes.insert(session_id, runtime);
        Ok(())
    }

    pub fn remove(&self, session_id: Uuid) -> Option<Arc<SessionRuntime>> {
        self.runtimes.write().remove(&session_id)
    }

    pub fn expire_detached(&self) -> Vec<(Uuid, Arc<SessionRuntime>)> {
        let now = Instant::now();
        let mut runtimes = self.runtimes.write();
        let expired_ids: Vec<Uuid> = runtimes
            .iter()
            .filter_map(|(session_id, runtime)| {
                runtime
                    .is_detached_expired(now, self.lease_duration)
                    .then_some(*session_id)
            })
            .collect();

        expired_ids
            .into_iter()
            .filter_map(|session_id| {
                runtimes
                    .remove(&session_id)
                    .map(|runtime| (session_id, runtime))
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct SessionPipelineRuntime {
    shutdown_tx: watch::Sender<bool>,
    abort_handle: Mutex<Option<tokio::task::AbortHandle>>,
}

impl SessionPipelineRuntime {
    pub fn new() -> (Arc<Self>, watch::Receiver<bool>) {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let runtime = Arc::new(Self {
            shutdown_tx,
            abort_handle: Mutex::new(None),
        });
        (runtime, shutdown_rx)
    }

    pub fn set_abort_handle(&self, abort_handle: tokio::task::AbortHandle) {
        *self.abort_handle.lock() = Some(abort_handle);
    }

    pub fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown_tx.subscribe()
    }

    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub fn abort(&self) {
        if let Some(handle) = self.abort_handle.lock().take() {
            handle.abort();
        }
    }
}

pub struct SessionPipelineRegistry {
    runtimes: RwLock<HashMap<Uuid, Arc<SessionPipelineRuntime>>>,
}

impl SessionPipelineRegistry {
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, session_id: Uuid) -> Option<Arc<SessionPipelineRuntime>> {
        self.runtimes.read().get(&session_id).cloned()
    }

    pub fn insert(
        &self,
        session_id: Uuid,
        runtime: Arc<SessionPipelineRuntime>,
    ) -> Result<(), Arc<SessionPipelineRuntime>> {
        let mut runtimes = self.runtimes.write();
        if let Some(existing) = runtimes.get(&session_id) {
            return Err(existing.clone());
        }
        runtimes.insert(session_id, runtime);
        Ok(())
    }

    pub fn remove(&self, session_id: Uuid) -> Option<Arc<SessionPipelineRuntime>> {
        self.runtimes.write().remove(&session_id)
    }

    pub fn stop(&self, session_id: Uuid) -> Option<Arc<SessionPipelineRuntime>> {
        let runtime = self.remove(session_id)?;
        runtime.signal_shutdown();
        runtime.abort();
        Some(runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pipeline_registry_insert_and_get() {
        let registry = SessionPipelineRegistry::new();
        let session_id = Uuid::new_v4();
        let (runtime, _shutdown_rx) = SessionPipelineRuntime::new();

        registry.insert(session_id, runtime.clone()).unwrap();
        let loaded = registry.get(session_id).unwrap();

        assert!(Arc::ptr_eq(&runtime, &loaded));
    }

    #[tokio::test]
    async fn pipeline_registry_remove_returns_handle() {
        let registry = SessionPipelineRegistry::new();
        let session_id = Uuid::new_v4();
        let (runtime, _shutdown_rx) = SessionPipelineRuntime::new();

        registry.insert(session_id, runtime.clone()).unwrap();
        let removed = registry.remove(session_id).unwrap();

        assert!(Arc::ptr_eq(&runtime, &removed));
        assert!(registry.get(session_id).is_none());
    }

    #[tokio::test]
    async fn pipeline_registry_stop_signals_shutdown() {
        let registry = SessionPipelineRegistry::new();
        let session_id = Uuid::new_v4();
        let (runtime, mut shutdown_rx) = SessionPipelineRuntime::new();

        let join_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        runtime.set_abort_handle(join_handle.abort_handle());

        registry.insert(session_id, runtime).unwrap();
        let stopped = registry.stop(session_id);
        assert!(stopped.is_some());
        assert!(registry.get(session_id).is_none());

        shutdown_rx.changed().await.unwrap();
        assert!(*shutdown_rx.borrow());
        let err = join_handle.await.unwrap_err();
        assert!(err.is_cancelled());
    }

    #[tokio::test]
    async fn pipeline_registry_rejects_duplicate_session_registration() {
        let registry = SessionPipelineRegistry::new();
        let session_id = Uuid::new_v4();
        let (first_runtime, _first_rx) = SessionPipelineRuntime::new();
        let (second_runtime, _second_rx) = SessionPipelineRuntime::new();

        registry.insert(session_id, first_runtime.clone()).unwrap();
        let duplicate = registry.insert(session_id, second_runtime).unwrap_err();

        assert!(Arc::ptr_eq(&first_runtime, &duplicate));
    }
}
