// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 突发配额计数器（无时钟、无补充、无速率语义）。
//!
//! 定位：inklog 默认限速路径的突发配额内核——初始 `capacity` 枚配额，
//! 每次取用递减，归零后拒绝。行为等价于 limiteron
//! `TokenBucketLimiter::new(capacity, 0)`（`refill_rate = 0`：无时间补充），
//! 但不引入时钟依赖，也不承担速率/peek/按 key 维度语义——那些语义仍归
//! limiteron 拥有，需按目标/按速率分组的限流走 `limiteron-rate-limit`
//! feature 的注入路径。
//!
//! 并发语义：CAS 递减，无超卖——并发争抢下总放行数恰为 `capacity`。

use std::sync::atomic::{AtomicU64, Ordering};

/// 突发配额计数器：满员配额池，取用即消费，耗尽即拒绝。
pub struct BurstLimiter {
    /// 剩余配额（枚）。
    remaining: AtomicU64,
    /// 配额上限（枚），创建时满员。
    capacity: u64,
}

impl BurstLimiter {
    /// 以固定容量创建（初始满员：`capacity` 枚配额）。
    pub fn new(capacity: u64) -> Self {
        Self {
            remaining: AtomicU64::new(capacity),
            capacity,
        }
    }

    /// 取用一枚配额；配额耗尽返回 `false`（不放行）。
    ///
    /// 并发安全：CAS 递减保证总放行数不超过初始容量。
    pub fn try_acquire(&self) -> bool {
        let mut current = self.remaining.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return false;
            }
            match self.remaining.compare_exchange_weak(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// 剩余配额（枚）。
    pub fn available(&self) -> u64 {
        self.remaining.load(Ordering::Relaxed)
    }

    /// 配额上限（枚）。
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_burst_limiter_allows_capacity_then_rejects() {
        let limiter = BurstLimiter::new(3);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert_eq!(limiter.available(), 0);
        assert!(
            !limiter.try_acquire(),
            "the capacity+1 acquire must be rejected"
        );
        assert_eq!(limiter.capacity(), 3);
    }

    #[test]
    fn test_burst_limiter_zero_capacity_rejects_immediately() {
        let limiter = BurstLimiter::new(0);
        assert!(!limiter.try_acquire());
        assert_eq!(limiter.available(), 0);
        assert_eq!(limiter.capacity(), 0);
    }

    #[test]
    fn test_burst_limiter_concurrent_acquires_never_oversell() {
        // 并发争抢同一配额池：CAS 冲突重试后总放行数必须恰等于容量（无超卖）
        let limiter = std::sync::Arc::new(BurstLimiter::new(64));
        let granted = std::sync::Arc::new(AtomicU64::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (limiter, granted) = (limiter.clone(), granted.clone());
                scope.spawn(move || {
                    for _ in 0..64 {
                        if limiter.try_acquire() {
                            granted.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });
        assert_eq!(
            granted.load(Ordering::Relaxed),
            64,
            "concurrent acquires must never oversell the budget"
        );
        assert_eq!(limiter.available(), 0);
    }
}
