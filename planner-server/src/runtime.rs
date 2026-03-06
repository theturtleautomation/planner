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

use crate::ws::ServerMessage;

const DEFAULT_BROADCAST_BUFFER: usize = 128;

pub struct RuntimeAttachment {
    pub input_tx: mpsc::UnboundedSender<String>,
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
    input_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    outbound_tx: broadcast::Sender<ServerMessage>,
    shutdown_tx: watch::Sender<bool>,
    lease_state: Mutex<RuntimeLeaseState>,
}

impl SessionRuntime {
    pub fn new() -> (
        Arc<Self>,
        Arc<AsyncMutex<mpsc::UnboundedReceiver<String>>>,
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
            .filter_map(|session_id| runtimes.remove(&session_id).map(|runtime| (session_id, runtime)))
            .collect()
    }
}
