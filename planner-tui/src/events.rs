//! # Event Handler — Terminal Input Events
//!
//! Crossterm-based async event handler with tick rate for periodic updates.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Application events.
pub enum Event {
    /// Terminal key press.
    Key(KeyEvent),
    /// Periodic tick (for async operations).
    Tick,
    /// Terminal resize.
    /// Width and height used when resize handling is implemented.
    #[allow(dead_code)]
    Resize(u16, u16),
}

/// Async event handler that polls terminal events with a tick rate.
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate in milliseconds.
    pub fn new(tick_rate_ms: u64) -> Self {
        EventHandler {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Wait for the next event.
    pub async fn next(&self) -> Event {
        // Use tokio to poll crossterm events without blocking the async runtime
        loop {
            if event::poll(self.tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key)) => return Event::Key(key),
                    Ok(CrosstermEvent::Resize(w, h)) => return Event::Resize(w, h),
                    _ => {}
                }
            } else {
                return Event::Tick;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_handler_creation() {
        let handler = EventHandler::new(100);
        assert_eq!(handler.tick_rate, Duration::from_millis(100));
    }
}
