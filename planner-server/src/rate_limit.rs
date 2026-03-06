//! # Rate Limiter — In-Memory Per-IP / Per-User Rate Limiting
//!
//! Tracks requests per key (IP address or authenticated user sub) using a
//! sliding window approach.  Keys that exceed the configured limit return
//! `429 Too Many Requests`.
//!
//! ## Design
//! - `parking_lot::Mutex<HashMap<String, Vec<Instant>>>` — simple and fast
//!   for moderate concurrency.
//! - Background eviction task runs every 5 minutes to remove stale entries.
//! - Limit: 100 requests per 60-second window per key (configurable).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Response, StatusCode},
    middleware::Next,
};
use parking_lot::Mutex;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum requests allowed per window.
const MAX_REQUESTS: usize = 100;

/// Length of the sliding window.
const WINDOW_DURATION: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// RateLimiter state
// ---------------------------------------------------------------------------

/// Shared in-memory rate limiter.
///
/// Stores per-key timestamp vectors.  Stale entries (older than the window)
/// are pruned on every check and periodically via the eviction task.
#[derive(Debug, Default)]
pub struct RateLimiter {
    inner: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether `key` is within the rate limit.
    ///
    /// Returns `true` if the request should be **allowed**, `false` if the
    /// limit has been exceeded.  On `true`, the request timestamp is recorded.
    pub fn check_and_record(&self, key: &str) -> bool {
        let now = Instant::now();
        let window_start = now - WINDOW_DURATION;

        let mut map = self.inner.lock();
        let timestamps = map.entry(key.to_owned()).or_default();

        // Evict timestamps outside the current window.
        timestamps.retain(|&ts| ts > window_start);

        if timestamps.len() >= MAX_REQUESTS {
            return false; // Rate limit exceeded
        }

        timestamps.push(now);
        true
    }

    /// Remove all entries whose most-recent request falls outside the window.
    ///
    /// Called by the background eviction task to bound memory usage.
    pub fn evict_stale(&self) {
        let cutoff = Instant::now() - WINDOW_DURATION;
        let mut map = self.inner.lock();
        map.retain(|_, timestamps| {
            timestamps.retain(|&ts| ts > cutoff);
            !timestamps.is_empty()
        });
    }

    /// Return the number of active keys currently being tracked.
    pub fn active_key_count(&self) -> usize {
        self.inner.lock().len()
    }
}

// ---------------------------------------------------------------------------
// Key extraction helpers
// ---------------------------------------------------------------------------

/// Extract a rate-limiting key from request headers.
///
/// Priority:
/// 1. `X-Forwarded-For` (first IP in a proxy chain)
/// 2. `X-Real-IP`
/// 3. Fallback: `"unknown"`
pub fn extract_key(headers: &HeaderMap) -> String {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            // XFF may be a comma-separated list; take the leftmost IP.
            if let Some(first) = val.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_owned();
                }
            }
        }
    }

    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(val) = xri.to_str() {
            return val.trim().to_owned();
        }
    }

    "unknown".to_owned()
}

// ---------------------------------------------------------------------------
// Axum middleware
// ---------------------------------------------------------------------------

/// Axum middleware layer that applies the rate limiter.
///
/// Attach to any `Router` via:
/// ```ignore
/// router.layer(axum::middleware::from_fn_with_state(
///     limiter_arc,
///     rate_limit_middleware,
/// ))
/// ```
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<Arc<RateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let key = extract_key(req.headers());

    if !limiter.check_and_record(&key) {
        tracing::warn!("Rate limit exceeded for key: {}", key);
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", "60")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"error":"Too many requests — try again in 60 seconds"}"#,
            ))
            .expect("static response is always valid");
    }

    next.run(req).await
}

/// Spawn a background task that evicts stale rate-limiter entries every
/// 5 minutes.  Call once at server startup.
pub fn spawn_eviction_task(limiter: Arc<RateLimiter>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let before = limiter.active_key_count();
            limiter.evict_stale();
            let after = limiter.active_key_count();
            tracing::debug!("Rate limiter eviction: {} → {} active keys", before, after,);
        }
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_under_limit() {
        let limiter = RateLimiter::new();
        for _ in 0..MAX_REQUESTS {
            assert!(limiter.check_and_record("test_key"));
        }
    }

    #[test]
    fn blocks_request_over_limit() {
        let limiter = RateLimiter::new();
        for _ in 0..MAX_REQUESTS {
            limiter.check_and_record("overload_key");
        }
        // The next request must be rejected.
        assert!(!limiter.check_and_record("overload_key"));
    }

    #[test]
    fn different_keys_are_independent() {
        let limiter = RateLimiter::new();
        for _ in 0..MAX_REQUESTS {
            limiter.check_and_record("key_a");
        }
        // key_b should still be allowed.
        assert!(limiter.check_and_record("key_b"));
        // key_a should be blocked.
        assert!(!limiter.check_and_record("key_a"));
    }

    #[test]
    fn evict_stale_removes_all_entries_after_window() {
        let limiter = RateLimiter::new();
        // Manually populate with an Instant that is well outside the window.
        {
            let old_ts = Instant::now() - WINDOW_DURATION - Duration::from_secs(1);
            let mut map = limiter.inner.lock();
            map.insert("stale_key".to_owned(), vec![old_ts]);
        }
        assert_eq!(limiter.active_key_count(), 1);
        limiter.evict_stale();
        assert_eq!(limiter.active_key_count(), 0);
    }

    #[test]
    fn extract_key_xff_first_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "10.0.0.1, 172.16.0.1, 192.168.0.1".parse().unwrap(),
        );
        assert_eq!(extract_key(&headers), "10.0.0.1");
    }

    #[test]
    fn extract_key_real_ip_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.5".parse().unwrap());
        assert_eq!(extract_key(&headers), "203.0.113.5");
    }

    #[test]
    fn extract_key_unknown_fallback() {
        let headers = HeaderMap::new();
        assert_eq!(extract_key(&headers), "unknown");
    }
}
