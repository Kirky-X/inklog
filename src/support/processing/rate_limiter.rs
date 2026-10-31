// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Token bucket rate limiter for log throughput control.
//!
//! Provides a thread-safe rate limiter using the token bucket algorithm
//! to prevent log flooding and protect downstream systems.
//!
//! 算法内核为仓内自持令牌桶（`TokenBucket`）：`new(rate)` 映射「桶满 rate、
//! 每秒补 rate」；决策点同步 O(1)，不经异步桥接（避免在 async 写路径上
//! 阻塞）；拒绝计数 [`RateLimiter::dropped_count`] 由 inklog 自持。

use std::sync::atomic::{AtomicU64, Ordering};

use super::token_bucket::TokenBucket;

/// Token bucket rate limiter for controlling log throughput.
///
/// 容量 = 速率 = `rate`：桶满 `rate`、每秒补 `rate`（与既有
/// `Mutex<f64>` 实现语义等价）。`try_acquire()` 为 O(1) 同步决策点。
pub struct RateLimiter {
    limiter: TokenBucket,
    dropped_count: AtomicU64,
}

impl RateLimiter {
    /// Create a new rate limiter with the given tokens-per-second limit.
    ///
    /// The bucket starts full (tokens = max_tokens = rate).
    /// `refill_rate` equals `rate` — tokens replenish at the same rate they consume.
    pub fn new(rate: u64) -> Self {
        Self {
            limiter: TokenBucket::new(rate),
            dropped_count: AtomicU64::new(0),
        }
    }

    /// 时钟注入构造（单元测试确定性时间；生产走 [`Self::new`] 系统时钟）。
    #[cfg(test)]
    pub(crate) fn with_clock(
        rate: u64,
        clock: std::sync::Arc<dyn super::token_bucket::Clock>,
    ) -> Self {
        Self {
            limiter: TokenBucket::with_clock(rate, clock),
            dropped_count: AtomicU64::new(0),
        }
    }

    /// Attempt to acquire a token. Returns `true` if allowed, `false` if rate-limited.
    ///
    /// On rejection, increments the internal dropped counter.
    pub fn try_acquire(&self) -> bool {
        // 同步 O(1) 决策点：余额不足即拒绝并计数（fail-closed），不阻塞写路径
        if self.limiter.try_acquire() {
            true
        } else {
            self.dropped_count.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Total number of logs dropped due to rate limiting.
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::super::token_bucket::MockClock;
    use super::*;
    use std::sync::Arc;

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
        // 冻结时钟（MockClock 不推进）→ 无时间补充，拒绝与真实耗时无关
        let clock = Arc::new(MockClock::new());
        let limiter = RateLimiter::with_clock(2, clock);
        // Consume both tokens
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        // Third must be rejected without any time-based flakiness
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_dropped_count_increments() {
        let clock = Arc::new(MockClock::new());
        let limiter = RateLimiter::with_clock(1, clock);
        // Consume the single token
        assert!(limiter.try_acquire());
        // These should be dropped
        assert!(!limiter.try_acquire());
        assert!(!limiter.try_acquire());
        assert_eq!(limiter.dropped_count(), 2);
    }

    #[test]
    fn test_rate_limiter_refill_is_time_driven() {
        // 冻结时钟耗尽后推进 1s：恰补充 rate 个令牌（limiteron 时间积分）
        let clock = Arc::new(MockClock::new());
        let limiter = RateLimiter::with_clock(2, clock.clone());
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
        clock.advance(std::time::Duration::from_secs(1));
        assert!(limiter.try_acquire(), "1s at 2/s must refill 2 tokens");
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
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
