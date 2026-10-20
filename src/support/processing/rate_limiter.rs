// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Token bucket rate limiter for log throughput control.
//!
//! Provides a thread-safe rate limiter using the token bucket algorithm
//! to prevent log flooding and protect downstream systems.
//!
//! 算法内核委托 limiteron [`TokenBucketLimiter`]（手写 `Mutex<f64>` 桶与
//! 补充公式已删除）：`new(rate)` 映射 `TokenBucketLimiter::new(rate, rate)`
//! （桶满 rate、每秒补 rate）；同步 `try_acquire()` 经
//! `futures::executor::block_on` 桥接异步 `allow(1)`；拒绝计数
//! [`RateLimiter::dropped_count`] 由 inklog 自持（limiteron 无诊断字段）。

use std::sync::atomic::{AtomicU64, Ordering};

use limiteron::limiters::{Limiter, TokenBucketLimiter};

/// Token bucket rate limiter for controlling log throughput.
///
/// 容量 = 速率 = `rate`：桶满 `rate`、每秒补 `rate`（与既有
/// `Mutex<f64>` 实现语义等价）。`try_acquire()` 为 O(1) 同步决策点。
pub struct RateLimiter {
    limiter: TokenBucketLimiter,
    dropped_count: AtomicU64,
}

impl RateLimiter {
    /// Create a new rate limiter with the given tokens-per-second limit.
    ///
    /// The bucket starts full (tokens = max_tokens = rate).
    /// `refill_rate` equals `rate` — tokens replenish at the same rate they consume.
    pub fn new(rate: u64) -> Self {
        Self {
            limiter: TokenBucketLimiter::new(rate, rate),
            dropped_count: AtomicU64::new(0),
        }
    }

    /// 时钟注入构造（单元测试确定性时间；生产走 [`Self::new`] 系统时钟）。
    #[cfg(test)]
    pub(crate) fn with_clock(rate: u64, clock: std::sync::Arc<dyn limiteron::Clock>) -> Self {
        Self {
            limiter: TokenBucketLimiter::with_clock(rate, rate, clock),
            dropped_count: AtomicU64::new(0),
        }
    }

    /// Attempt to acquire a token. Returns `true` if allowed, `false` if rate-limited.
    ///
    /// On rejection, increments the internal dropped counter.
    pub fn try_acquire(&self) -> bool {
        // 同步决策点桥接 limiteron 异步判定；cost=1 恒合法，错误路径
        // fail-closed（拒绝并计数）不放大写入压力
        if futures::executor::block_on(self.limiter.allow(1)).unwrap_or(false) {
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
    use super::*;
    use limiteron::MockClock;
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
