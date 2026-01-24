// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::log_record::LogRecord;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Pool statistics for monitoring
#[derive(Debug, Default)]
pub struct PoolStats {
    pub(crate) total_items: AtomicUsize,
    pub(crate) hits: AtomicUsize,
    pub(crate) misses: AtomicUsize,
    pub(crate) items_created: AtomicUsize,
    pub(crate) items_reused: AtomicUsize,
}

/// A simple thread-safe object pool with warm-up and monitoring support
pub(crate) struct Pool<T: Default> {
    items: Mutex<Vec<T>>,
    max_size: usize,
    warmup_size: usize,
    stats: Arc<PoolStats>,
}

impl<T: Default> Pool<T> {
    /// Create a new pool with specified max size and warmup size
    pub fn new(max_size: usize, warmup_size: usize) -> Self {
        Self {
            items: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
            warmup_size,
            stats: Arc::new(PoolStats::default()),
        }
    }

    /// Pre-warm the pool by creating initial items
    pub fn warmup(&self) {
        let mut items = self.items.lock().unwrap();
        if items.len() < self.warmup_size {
            for _ in 0..self.warmup_size.saturating_sub(items.len()) {
                items.push(T::default());
                self.stats.items_created.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get an item from the pool, or create a new one if pool is empty
    pub fn get(&self) -> T {
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.pop() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            self.stats.items_reused.fetch_add(1, Ordering::Relaxed);
            self.stats.total_items.store(items.len(), Ordering::Relaxed);
            return item;
        }
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.stats.total_items.store(items.len(), Ordering::Relaxed);
        T::default()
    }

    /// Return an item to the pool
    pub fn put(&self, item: T) {
        let mut items = self.items.lock().unwrap();
        if items.len() < self.max_size {
            items.push(item);
            self.stats.total_items.store(items.len(), Ordering::Relaxed);
        }
    }

    /// Get pool statistics
    ///
    /// This method provides detailed metrics for monitoring pool performance.
    /// It is used internally for debugging and can be exposed via metrics endpoints
    /// when the `http` feature is enabled.
    ///
    /// # Panics
    ///
    /// This method will never panic as it uses relaxed atomics.
    #[allow(dead_code)]
    pub fn stats(&self) -> PoolMetrics {
        let total = self.stats.total_items.load(Ordering::Relaxed);
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let misses = self.stats.misses.load(Ordering::Relaxed);
        let created = self.stats.items_created.load(Ordering::Relaxed);
        let reused = self.stats.items_reused.load(Ordering::Relaxed);

        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            (hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        PoolMetrics {
            current_size: total,
            max_size: self.max_size,
            warmup_size: self.warmup_size,
            total_requests,
            hits,
            misses,
            hit_rate,
            items_created: created,
            items_reused: reused,
        }
    }

    /// Get the current pool size
    ///
    /// Returns the number of items currently available in the pool.
    /// This is useful for debugging and monitoring pool utilization.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.items.lock().unwrap().len()
    }

    /// Check if the pool is empty
    ///
    /// Returns `true` if the pool has no items available.
    /// This is useful for debugging and monitoring pool utilization.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.lock().unwrap().is_empty()
    }
}

/// Pool metrics for monitoring and performance analysis
///
/// This struct provides detailed statistics about pool operations:
/// - Pool size and capacity
/// - Hit/miss rates for item reuse
/// - Item creation and reuse counts
///
/// These metrics are useful for:
/// - Debugging pool configuration issues
/// - Monitoring memory allocation patterns
/// - Performance tuning and optimization
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolMetrics {
    pub current_size: usize,
    pub max_size: usize,
    pub warmup_size: usize,
    pub total_requests: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f64,
    pub items_created: usize,
    pub items_reused: usize,
}

/// Global pool for LogRecord to reduce allocations
pub(crate) static LOG_RECORD_POOL: Lazy<Pool<LogRecord>> = Lazy::new(|| {
    let pool = Pool::new(1024, 128); // Max 1024, warmup 128
    pool.warmup();
    pool
});

/// Global pool for String buffers to reduce allocations
pub(crate) static STRING_POOL: Lazy<Pool<String>> = Lazy::new(|| {
    let pool = Pool::new(1024, 64); // Max 1024, warmup 64
    pool.warmup();
    pool
});
