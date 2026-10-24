// Copyright (c) 2025-2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 自持令牌桶：容量 = 速率，每秒补充速率个令牌。
//!
//! 语义等价 limiteron `TokenBucketLimiter::new(rate, rate)`（桶满 rate、每秒补
//! rate），但决策点是同步 O(1)：不再经 `futures::executor::block_on` 桥接异步
//! `allow(1)`——原实现把同步阻塞留在 async 写路径上，每条日志阻塞一次。
//!
//! 定点表示：一个令牌 = `UNITS_PER_TOKEN` 单位，补充量按「经过纳秒 × 每秒
//! 补充数」累加，避免浮点误差。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 一个令牌对应的定点单位数
const UNITS_PER_TOKEN: u64 = 1_000_000_000;

/// 可注入时钟（测试确定性；生产走系统时钟）
pub(crate) trait Clock: Send + Sync {
    /// 自 Unix 纪元起的纳秒数
    fn unix_timestamp_nanos(&self) -> u64;
}

/// 系统时钟
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn unix_timestamp_nanos(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}

/// 令牌桶：容量与补充速率同参（每秒补 `capacity` 个令牌）
pub(crate) struct TokenBucket {
    /// 容量上限（定点单位）
    capacity_units: u64,
    /// 每秒补充的令牌数
    refill_per_sec: u64,
    /// 当前余额（定点单位）
    balance: Mutex<u64>,
    /// 上次补充计算的时刻（纳秒）
    last_refill_nanos: AtomicU64,
    clock: Arc<dyn Clock>,
}

impl TokenBucket {
    /// 桶满 `capacity`，每秒补 `capacity` 个令牌
    pub(crate) fn new(capacity: u64) -> Self {
        Self::with_clock(capacity, Arc::new(SystemClock))
    }

    /// 以注入时钟创建（测试确定性）
    pub(crate) fn with_clock(capacity: u64, clock: Arc<dyn Clock>) -> Self {
        let now = clock.unix_timestamp_nanos();
        let initial = capacity.saturating_mul(UNITS_PER_TOKEN);
        Self {
            capacity_units: initial,
            refill_per_sec: capacity,
            balance: Mutex::new(initial),
            last_refill_nanos: AtomicU64::new(now),
            clock,
        }
    }

    /// 取用一枚令牌：余额充足返回 true，否则 false（不阻塞、不上抛）
    pub(crate) fn try_acquire(&self) -> bool {
        let mut balance = match self.balance.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.refill(&mut balance);
        if *balance >= UNITS_PER_TOKEN {
            *balance -= UNITS_PER_TOKEN;
            true
        } else {
            false
        }
    }

    /// 按经过时间补充余额（封顶容量）
    fn refill(&self, balance: &mut u64) {
        let now = self.clock.unix_timestamp_nanos();
        let last = self.last_refill_nanos.load(Ordering::Acquire);
        if now <= last {
            return;
        }
        self.last_refill_nanos.store(now, Ordering::Release);
        // 补充单位数 = 经过纳秒 × 每秒补充数（1 token = 1e9 单位）
        let added = (now - last) as u128 * self.refill_per_sec as u128;
        if added == 0 {
            return;
        }
        *balance = balance
            .saturating_add(added.min(u64::MAX as u128) as u64)
            .min(self.capacity_units);
    }
}

/// 冻结时钟（单元测试确定性）
#[cfg(test)]
pub(crate) struct MockClock {
    nanos: AtomicU64,
}

#[cfg(test)]
impl MockClock {
    pub(crate) fn new() -> Self {
        Self {
            nanos: AtomicU64::new(0),
        }
    }

    /// 推进时钟
    pub(crate) fn advance(&self, delta: std::time::Duration) {
        self.nanos
            .fetch_add(delta.as_nanos() as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
impl Clock for MockClock {
    fn unix_timestamp_nanos(&self) -> u64 {
        self.nanos.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn bucket_allows_up_to_capacity_then_rejects() {
        let bucket = TokenBucket::new(2);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn bucket_refills_at_configured_rate() {
        let clock = Arc::new(MockClock::new());
        let bucket = TokenBucket::with_clock(2, clock.clone());
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
        clock.advance(Duration::from_secs(1));
        assert!(bucket.try_acquire(), "1s at 2/s must refill 2 tokens");
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn zero_capacity_never_allows() {
        let bucket = TokenBucket::new(0);
        assert!(!bucket.try_acquire());
    }
}
