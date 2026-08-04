// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Token bucket rate limiter for log throughput control.
//!
//! Provides a thread-safe rate limiter using the token bucket algorithm
//! to prevent log flooding and protect downstream systems.

use parking_lot::Mutex;
use std::time::Instant;

/// Token bucket rate limiter for controlling log throughput.
///
/// Uses `parking_lot::Mutex` for low-contention locking (~20ns per acquire).
/// The `try_acquire()` method is O(1): refill tokens based on elapsed time,
/// then attempt to consume one token.
pub struct RateLimiter {
    inner: Mutex<RateLimiterInner>,
}

struct RateLimiterInner {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
    dropped_count: u64,
}

impl RateLimiter {
    /// Create a new rate limiter with the given tokens-per-second limit.
    ///
    /// The bucket starts full (tokens = max_tokens = rate).
    /// `refill_rate` equals `rate` — tokens replenish at the same rate they consume.
    pub fn new(rate: u64) -> Self {
        let rate_f = rate as f64;
        Self {
            inner: Mutex::new(RateLimiterInner {
                tokens: rate_f,
                max_tokens: rate_f,
                refill_rate: rate_f,
                last_refill: Instant::now(),
                dropped_count: 0,
            }),
        }
    }

    /// Attempt to acquire a token. Returns `true` if allowed, `false` if rate-limited.
    ///
    /// On rejection, increments the internal dropped counter.
    pub fn try_acquire(&self) -> bool {
        let mut inner = self.inner.lock();
        inner.refill();
        if inner.tokens >= 1.0 {
            inner.tokens -= 1.0;
            true
        } else {
            inner.dropped_count += 1;
            false
        }
    }

    /// Total number of logs dropped due to rate limiting.
    pub fn dropped_count(&self) -> u64 {
        self.inner.lock().dropped_count
    }
}

impl RateLimiterInner {
    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
            self.last_refill = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(1000);
        // Should allow many acquires within the limit
        for _ in 0..100 {
            assert!(limiter.try_acquire());
        }
    }

    #[test]
    fn test_rate_limiter_rejects_when_exhausted() {
        // Very low rate: 2 tokens/sec
        let limiter = RateLimiter::new(2);
        // Consume both tokens
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        // Third should be rejected (no time for refill)
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_dropped_count_increments() {
        let limiter = RateLimiter::new(1);
        // Consume the single token
        assert!(limiter.try_acquire());
        // These should be dropped
        assert!(!limiter.try_acquire());
        assert!(!limiter.try_acquire());
        assert_eq!(limiter.dropped_count(), 2);
    }

    #[test]
    fn test_rate_limiter_refills_over_time() {
        let limiter = RateLimiter::new(1000);
        // Exhaust all tokens
        for _ in 0..1000 {
            limiter.try_acquire();
        }
        // Wait a bit for refill (100ms should give ~100 tokens at 1000/sec)
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Should be able to acquire again
        assert!(limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_zero_rate() {
        // Rate of 0 means no tokens available
        let limiter = RateLimiter::new(0);
        assert!(!limiter.try_acquire());
        assert_eq!(limiter.dropped_count(), 1);
    }
}
