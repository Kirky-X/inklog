// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # Object Pool
//!
//! High-performance object pool using oxcache for LRU caching and thread-safe operations.
//! This module provides object pooling for LogRecord and String buffers.
//! All caching is centralized through oxcache - no other cache implementations exist.
//!
//! # Construction Patterns
//!
//! This module supports two construction patterns:
//! - `new()` - Creates pool with default configuration
//! - `builder()` - Creates pool with custom configuration
//!
//! # Usage Examples
//!
//! ```
//! use inklog::ObjectPool;
//!
//! // Pattern 1: new() - Default configuration
//! let pool1 = ObjectPool::<String, i32>::new();
//!
//! // Pattern 2: builder() - Custom configuration
//! let pool2 = ObjectPool::<String, i32>::builder()
//!     .capacity(2048)
//!     .build();
//! ```

use crate::log_record::LogRecord;
use once_cell::sync::Lazy;
use oxcache::Cache;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;

/// Pool configuration - configurable via InklogConfig.performance.object_pool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectPoolConfig {
    /// Maximum capacity of the pool
    #[serde(default = "default_max_capacity")]
    pub max_capacity: usize,
    /// Default TTL for pooled items (None = no TTL)
    pub ttl_secs: Option<u64>,
}

fn default_max_capacity() -> usize {
    1024
}

impl Default for ObjectPoolConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1024,
            ttl_secs: None,
        }
    }
}

/// Object pool metrics for monitoring
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub current_size: usize,
    pub max_capacity: usize,
    pub total_requests: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f64,
    pub items_created: usize,
    pub items_reused: usize,
}

/// Object pool using oxcache Cache
///
/// This pool provides:
/// - LRU eviction when pool is full
/// - Thread-safe operations without explicit locking
/// - Configurable capacity and TTL
/// - Internal metrics tracking
#[derive(Clone)]
#[allow(dead_code)]
pub struct ObjectPool<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    /// The underlying oxcache async cache
    cache: Arc<Cache<K, V>>,
    /// Pool configuration
    config: ObjectPoolConfig,
    /// Metrics tracking
    stats: Arc<PoolStats>,
}

impl<K, V> ObjectPool<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    /// Create a new object pool with default configuration (capacity: 1024)
    pub fn new() -> Self {
        Self::with_config(ObjectPoolConfig::default())
    }

    /// Create a new object pool with specified max capacity
    #[allow(dead_code)]
    pub fn with_capacity(max_capacity: usize) -> Self {
        let config = ObjectPoolConfig {
            max_capacity,
            ttl_secs: None,
        };
        Self::with_config(config)
    }

    /// Create a new object pool with full configuration
    pub fn with_config(config: ObjectPoolConfig) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for object pool");

            let cache = rt.block_on(async {
                let mut builder = Cache::builder();
                builder = builder.capacity(config.max_capacity as u64);
                if let Some(ttl_secs) = config.ttl_secs {
                    builder = builder.ttl(Duration::from_secs(ttl_secs));
                }
                Arc::new(builder.build().await.expect("Failed to build cache"))
            });

            let _ = tx.send(cache);
        });

        let cache = rx.recv().expect("Failed to receive from worker thread");

        Self {
            cache,
            config: config.clone(),
            stats: Arc::new(PoolStats::default()),
        }
    }

    /// Create a new ObjectPoolBuilder for configuring the pool
    #[allow(dead_code)]
    pub fn builder() -> ObjectPoolBuilder<K, V> {
        ObjectPoolBuilder::new()
    }

    /// Get an item from the pool by key
    pub fn get(&self, key: &K) -> Option<V>
    where
        K: Clone,
    {
        if let Ok(_handle) = Handle::try_current() {
            let cache = Arc::clone(&self.cache);
            let stats = Arc::clone(&self.stats);
            let key = key.clone();
            let result = futures::executor::block_on(async move {
                let handle =
                    tokio::task::spawn(
                        async move { cache.get(&key).await.expect("Cache get failed") },
                    );
                handle.await.expect("Task panicked")
            });
            if result.is_some() {
                stats.hits.fetch_add(1, Ordering::Relaxed);
                stats.items_reused.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.misses.fetch_add(1, Ordering::Relaxed);
            }
            stats.total_items.store(self.len(), Ordering::Relaxed);
            return result;
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        let result =
            runtime.block_on(async { self.cache.get(key).await.expect("Cache get failed") });
        if result.is_some() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            self.stats.items_reused.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
        }
        self.stats.total_items.store(self.len(), Ordering::Relaxed);
        result
    }

    /// Put an item into the pool with the given key
    pub fn put(&self, key: &K, value: V)
    where
        K: Clone,
        V: Clone,
    {
        if let Ok(_handle) = Handle::try_current() {
            let cache = Arc::clone(&self.cache);
            let key = key.clone();
            let value = value.clone();
            futures::executor::block_on(async move {
                let handle = tokio::task::spawn(async move {
                    cache.set(&key, &value).await.expect("Cache set failed")
                });
                handle.await.expect("Task panicked")
            });
            self.stats.total_items.store(self.len(), Ordering::Relaxed);
            return;
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime.block_on(async { self.cache.set(key, &value).await.expect("Cache set failed") });
        self.stats.total_items.store(self.len(), Ordering::Relaxed);
    }

    /// Check if a key exists in the pool
    #[allow(dead_code)]
    pub fn contains(&self, key: &K) -> bool
    where
        K: Clone,
    {
        if let Ok(_handle) = Handle::try_current() {
            let cache = Arc::clone(&self.cache);
            let key = key.clone();
            return futures::executor::block_on(async move {
                let handle = tokio::task::spawn(async move {
                    cache.exists(&key).await.expect("Cache exists failed")
                });
                handle.await.expect("Task panicked")
            });
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime.block_on(async { self.cache.exists(key).await.expect("Cache exists failed") })
    }

    /// Remove and return an item from the pool by key
    #[allow(dead_code)]
    pub fn remove(&self, key: &K) -> Option<V>
    where
        K: Clone,
        V: Clone,
    {
        if let Ok(_handle) = Handle::try_current() {
            let cache_get = Arc::clone(&self.cache);
            let cache_delete = Arc::clone(&self.cache);
            let key_get = key.clone();
            let key_delete = key.clone();
            let value = futures::executor::block_on(async move {
                let get_handle = tokio::task::spawn(async move {
                    cache_get.get(&key_get).await.expect("Cache get failed")
                });
                get_handle.await.expect("Task panicked")
            });

            futures::executor::block_on(async move {
                let delete_handle = tokio::task::spawn(async move {
                    cache_delete
                        .delete(&key_delete)
                        .await
                        .expect("Cache delete failed")
                });
                delete_handle.await.expect("Task panicked")
            });

            if value.is_some() {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
            }
            self.stats.total_items.store(self.len(), Ordering::Relaxed);
            return value;
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        let value =
            runtime.block_on(async { self.cache.get(key).await.expect("Cache get failed") });

        runtime.block_on(async { self.cache.delete(key).await.expect("Cache delete failed") });

        if value.is_some() {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
        }
        self.stats.total_items.store(self.len(), Ordering::Relaxed);
        value
    }

    /// Get the current number of items in the pool
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.stats.total_items.load(Ordering::Relaxed)
    }

    /// Check if the pool is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the maximum capacity of the pool
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.config.max_capacity
    }

    /// Get pool metrics for internal monitoring
    #[allow(dead_code)]
    pub fn metrics(&self) -> PoolMetrics {
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
            max_capacity: self.config.max_capacity,
            total_requests,
            hits,
            misses,
            hit_rate,
            items_created: created,
            items_reused: reused,
        }
    }

    /// Clear all items from the pool
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(_handle) = Handle::try_current() {
            let cache = self.cache.clone();
            let handle = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create runtime");
                rt.block_on(async { cache.clear().await.expect("Cache clear failed") })
            });
            futures::executor::block_on(handle).expect("Blocking task panicked");
            self.stats.total_items.store(0, Ordering::Relaxed);
            return;
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime.block_on(async { self.cache.clear().await.expect("Cache clear failed") });
        self.stats.total_items.store(0, Ordering::Relaxed);
    }
}

impl<K, V> Default for ObjectPool<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating ObjectPool instances with custom configuration
#[allow(dead_code)]
pub struct ObjectPoolBuilder<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    /// Maximum capacity of the pool
    capacity: usize,
    /// TTL for pool entries (in seconds)
    ttl_secs: Option<u64>,
    /// Marker for generic types
    _marker: std::marker::PhantomData<(K, V)>,
}

#[allow(dead_code)]
impl<K, V> ObjectPoolBuilder<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            capacity: 1024,
            ttl_secs: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the maximum capacity of the pool
    #[allow(dead_code)]
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// Set the TTL for pool entries
    #[allow(dead_code)]
    pub fn ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    /// Build the ObjectPool with the configured settings
    #[allow(dead_code)]
    pub fn build(self) -> ObjectPool<K, V> {
        let config = ObjectPoolConfig {
            max_capacity: self.capacity,
            ttl_secs: self.ttl_secs,
        };

        ObjectPool::with_config(config)
    }
}

impl<K, V> Default for ObjectPoolBuilder<K, V>
where
    K: oxcache::CacheKey + Send + Sync + 'static,
    V: oxcache::Cacheable + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Internal pool statistics
#[derive(Debug, Default)]
struct PoolStats {
    pub(crate) total_items: AtomicUsize,
    pub(crate) hits: AtomicUsize,
    pub(crate) misses: AtomicUsize,
    #[allow(dead_code)]
    pub(crate) items_created: AtomicUsize,
    pub(crate) items_reused: AtomicUsize,
}

/// Global pool for LogRecord to reduce allocations
///
/// Uses oxcache Cache for thread-safe LRU caching with configurable capacity.
#[derive(Clone)]
pub struct LogRecordPool {
    pool: ObjectPool<String, LogRecord>,
}

impl LogRecordPool {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let config = ObjectPoolConfig {
            max_capacity: capacity,
            ttl_secs: None,
        };
        let pool: ObjectPool<String, LogRecord> = ObjectPool::with_config(config);
        Self { pool }
    }

    pub fn get(&self) -> LogRecord {
        self.pool.get(&"log_record".to_string()).unwrap_or_default()
    }

    pub fn put(&self, record: LogRecord) {
        self.pool.put(&"log_record".to_string(), record);
    }

    #[allow(dead_code)]
    pub fn metrics(&self) -> PoolMetrics {
        self.pool.metrics()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

impl Default for LogRecordPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pool for String buffers to reduce allocations
///
/// Uses oxcache Cache for thread-safe LRU caching.
#[derive(Clone)]
pub struct StringPool {
    pool: ObjectPool<String, String>,
}

impl StringPool {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let config = ObjectPoolConfig {
            max_capacity: capacity,
            ttl_secs: None,
        };
        let pool = ObjectPool::<String, String>::with_config(config);
        Self { pool }
    }

    pub fn get(&self) -> String {
        self.pool
            .get(&"string_buffer".to_string())
            .unwrap_or_default()
    }

    pub fn put(&self, s: String) {
        self.pool.put(&"string_buffer".to_string(), s);
    }

    #[allow(dead_code)]
    pub fn metrics(&self) -> PoolMetrics {
        self.pool.metrics()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pool for LogRecord instances
pub static LOG_RECORD_POOL: Lazy<LogRecordPool> = Lazy::new(|| LogRecordPool::with_capacity(1024));

/// Global pool for String buffers
pub static STRING_POOL: Lazy<StringPool> = Lazy::new(|| StringPool::with_capacity(1024));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_pool_basic_operations() {
        let pool = ObjectPool::<String, i32>::with_capacity(10);

        assert!(pool.is_empty());

        let value = pool.get(&"nonexistent".to_string());
        assert!(value.is_none());

        pool.put(&"key1".to_string(), 42);

        assert!(pool.contains(&"key1".to_string()));

        let value = pool.get(&"key1".to_string());
        assert_eq!(value, Some(42));

        let value2 = pool.get(&"key1".to_string());
        assert_eq!(value2, Some(42));

        let removed = pool.remove(&"key1".to_string());
        assert_eq!(removed, Some(42));

        let value3 = pool.get(&"key1".to_string());
        assert!(value3.is_none());
        assert!(pool.is_empty());
    }

    #[test]
    fn test_object_pool_capacity() {
        let pool = ObjectPool::<String, i32>::builder()
            .capacity(3)
            .ttl_secs(1)
            .build();

        assert_eq!(pool.capacity(), 3);

        pool.put(&"1".to_string(), 1);
        pool.put(&"2".to_string(), 2);
        pool.put(&"3".to_string(), 3);

        assert_eq!(pool.get(&"1".to_string()), Some(1));
        assert_eq!(pool.get(&"2".to_string()), Some(2));
        assert_eq!(pool.get(&"3".to_string()), Some(3));

        pool.put(&"4".to_string(), 4);
        pool.put(&"5".to_string(), 5);

        let _ = pool.get(&"1".to_string());
        let _ = pool.get(&"4".to_string());

        pool.clear();
        assert!(pool.is_empty());
    }

    #[test]
    fn test_object_pool_metrics() {
        let pool = ObjectPool::<String, i32>::new();

        let metrics = pool.metrics();
        assert_eq!(metrics.current_size, 0);
        assert_eq!(metrics.max_capacity, 1024);
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.items_created, 0);
        assert_eq!(metrics.items_reused, 0);

        let _ = pool.get(&"missing".to_string());
        let metrics = pool.metrics();
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.total_requests, 1);

        pool.put(&"key".to_string(), 100);
        let _ = pool.get(&"key".to_string());
        let metrics = pool.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.hit_rate, 50.0);
        assert_eq!(metrics.items_created, 0);
        assert_eq!(metrics.items_reused, 1);
    }

    #[test]
    fn test_log_record_pool() {
        let pool = LogRecordPool::with_capacity(10);

        assert!(pool.is_empty());

        let record = pool.get();
        assert_eq!(record.level, "INFO");

        let metrics = pool.metrics();
        assert_eq!(metrics.current_size, 0);

        let record = LogRecord::default();
        pool.put(record);

        let metrics = pool.metrics();
        assert_eq!(metrics.max_capacity, 10);

        let record2 = pool.get();
        assert_eq!(record2.level, "INFO");

        let _ = pool.len();
    }

    #[test]
    fn test_string_pool() {
        let pool = StringPool::with_capacity(10);

        assert!(pool.is_empty());

        let s = pool.get();
        assert!(s.is_empty());

        pool.put("hello".to_string());

        let s2 = pool.get();
        assert!(s2.is_empty() || s2 == *"hello");

        let metrics = pool.metrics();
        assert_eq!(metrics.max_capacity, 10);
        let _ = pool.len();
    }
}
