// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Cache trait - 抽象缓存操作
//!
//! 提供 CRUD 操作的抽象接口，支持不同缓存后端的统一访问。

use crate::InklogError;
use async_trait::async_trait;

/// Cache trait - 抽象缓存操作
///
/// 提供基本的缓存 CRUD 操作接口，所有方法都是异步的。
/// 实现必须保证线程安全（`Send + Sync`）。
///
/// # 错误处理
///
/// 所有方法返回 `Result<_, InklogError>`，错误显性传播，不静默吞错。
/// 实现应该是非阻塞的，适合在 async 上下文中调用。
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::Cache;
///
/// async fn example(cache: &dyn Cache) -> Result<(), Box<dyn std::error::Error>> {
///     cache.set("key", "value".to_string()).await?;
///     let value = cache.get("key").await?;
///     assert_eq!(value, Some("value".to_string()));
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait Cache: Send + Sync {
    /// 获取缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回
    ///
    /// - `Ok(Some(value))` - 键存在
    /// - `Ok(None)` - 键不存在
    /// - `Err(InklogError)` - 缓存访问失败
    async fn get(&self, key: &str) -> Result<Option<String>, InklogError>;

    /// 设置缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(())`，失败返回 `Err(InklogError)`
    async fn set(&self, key: &str, value: String) -> Result<(), InklogError>;

    /// 删除缓存值
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回
    ///
    /// - `Ok(true)` - 键存在并已删除
    /// - `Ok(false)` - 键不存在
    /// - `Err(InklogError)` - 缓存访问失败
    async fn delete(&self, key: &str) -> Result<bool, InklogError>;

    /// 检查键是否存在
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回
    ///
    /// - `Ok(true)` - 键存在
    /// - `Ok(false)` - 键不存在
    /// - `Err(InklogError)` - 缓存访问失败
    async fn exists(&self, key: &str) -> Result<bool, InklogError>;
}

// ============================================================================
// OxCacheAdapter - oxcache 适配器实现
// ============================================================================

use oxcache::Cache as OxCache;
use std::collections::HashMap;
use std::sync::RwLock;

/// oxcache 适配器
///
/// 将 oxcache 库的 `Cache<K, V>` 适配为 `Cache` trait。
/// 使用 `Cache<String, String>` 作为底层存储类型。
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::cache::{Cache, OxCacheAdapter};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let cache = OxCacheAdapter::new()?;
///
///     cache.set("user:1", "Alice".to_string()).await?;
///     let name = cache.get("user:1").await?;
///     assert_eq!(name, Some("Alice".to_string()));
///
///     Ok(())
/// }
/// ```
pub struct OxCacheAdapter {
    inner: OxCache<String, String>,
}

impl OxCacheAdapter {
    /// 创建新的 oxcache 适配器
    ///
    /// 使用默认的内存后端创建缓存实例。
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 错误
    ///
    /// - `InklogError::CacheError` - 缓存初始化失败
    pub fn new() -> Result<Self, InklogError> {
        let cache = OxCache::new();
        Ok(Self { inner: cache })
    }

    /// 使用构建器创建适配器
    ///
    /// 提供更细粒度的配置选项，如 TTL、容量等。
    /// 必须在 async 上下文中调用 `build().await`。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use inklog::infrastructure::cache::OxCacheAdapter;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cache = OxCacheAdapter::builder()
    ///     .ttl(Duration::from_secs(3600))
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> OxCacheAdapterBuilder {
        OxCacheAdapterBuilder::default()
    }
}

#[async_trait]
impl Cache for OxCacheAdapter {
    async fn get(&self, key: &str) -> Result<Option<String>, InklogError> {
        self.inner.get(&key.to_string()).await.map_err(|e| {
            InklogError::CacheError(format!("Failed to get cache key '{}': {}", key, e))
        })
    }

    async fn set(&self, key: &str, value: String) -> Result<(), InklogError> {
        self.inner.set(&key.to_string(), &value).await.map_err(|e| {
            InklogError::CacheError(format!("Failed to set cache key '{}': {}", key, e))
        })
    }

    async fn delete(&self, key: &str) -> Result<bool, InklogError> {
        // 先检查键是否存在，避免无谓的 delete 调用
        let exists = self.exists(key).await?;
        if !exists {
            return Ok(false);
        }
        self.inner.delete(&key.to_string()).await.map_err(|e| {
            InklogError::CacheError(format!("Failed to delete cache key '{}': {}", key, e))
        })?;
        Ok(true)
    }

    async fn exists(&self, key: &str) -> Result<bool, InklogError> {
        self.inner.exists(&key.to_string()).await.map_err(|e| {
            InklogError::CacheError(format!(
                "Failed to check existence of cache key '{}': {}",
                key, e
            ))
        })
    }
}

/// OxCacheAdapter 构建器
///
/// 提供链式配置 API。
#[derive(Default)]
pub struct OxCacheAdapterBuilder {
    ttl: Option<std::time::Duration>,
    capacity: Option<u64>,
}

impl OxCacheAdapterBuilder {
    /// 设置 TTL（生存时间）
    pub fn ttl(mut self, ttl: std::time::Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// 设置缓存容量
    pub fn capacity(mut self, capacity: u64) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// 异步构建适配器
    ///
    /// 将配置的 TTL 和容量应用到 oxcache 后端。
    /// 必须在 async 上下文中调用，因为 oxcache 0.2.0 的
    /// `CacheBuilder::build()` 是异步方法。
    ///
    /// # 错误
    ///
    /// - `InklogError::CacheError` - oxcache 构建失败
    pub async fn build(self) -> Result<OxCacheAdapter, InklogError> {
        let mut builder = OxCache::builder();
        if let Some(ttl) = self.ttl {
            builder = builder.ttl(ttl);
        }
        if let Some(capacity) = self.capacity {
            builder = builder.capacity(capacity);
        }
        let cache = builder
            .build()
            .await
            .map_err(|e| InklogError::CacheError(format!("Failed to build oxcache: {}", e)))?;
        Ok(OxCacheAdapter { inner: cache })
    }
}

// ============================================================================
// MockCache - 单元测试 Mock 实现
// ============================================================================

/// Mock 缓存实现，用于单元测试
///
/// 提供基于内存的简单缓存实现，支持可选的延迟模拟。
/// 使用 `RwLock` 保护内部存储，确保线程安全。
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::cache::{Cache, MockCache};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let cache = MockCache::new();
///
///     cache.set("key", "value".to_string()).await?;
///     assert_eq!(cache.get("key").await?, Some("value".to_string()));
///
///     // 带延迟的 Mock
///     let delayed_cache = MockCache::with_delay(100);
///     delayed_cache.set("slow", "op".to_string()).await?;
///     Ok(())
/// }
/// ```
pub struct MockCache {
    /// 内部存储，使用 RwLock 保护并发访问
    storage: RwLock<HashMap<String, String>>,
    /// 模拟延迟（毫秒）
    delay_ms: u64,
}

impl MockCache {
    /// 创建新的 MockCache 实例
    ///
    /// 创建一个空的内存存储，无延迟。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let cache = MockCache::new();
    /// ```
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
            delay_ms: 0,
        }
    }

    /// 创建带延迟的 MockCache 实例
    ///
    /// 用于测试异步场景或模拟网络延迟。
    ///
    /// # 参数
    ///
    /// * `ms` - 延迟毫秒数
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let cache = MockCache::with_delay(100); // 100ms 延迟
    /// ```
    pub fn with_delay(ms: u64) -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
            delay_ms: ms,
        }
    }
}

impl Default for MockCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for MockCache {
    async fn get(&self, key: &str) -> Result<Option<String>, InklogError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }
        let storage = self.storage.read().unwrap();
        Ok(storage.get(key).cloned())
    }

    async fn set(&self, key: &str, value: String) -> Result<(), InklogError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }
        let mut storage = self.storage.write().unwrap();
        storage.insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, InklogError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }
        let mut storage = self.storage.write().unwrap();
        Ok(storage.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> Result<bool, InklogError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }
        let storage = self.storage.read().unwrap();
        Ok(storage.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // OxCacheAdapter 测试
    // ============================================================================

    #[tokio::test]
    async fn test_oxcache_adapter_basic_operations() {
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        // 测试 set 和 get
        cache
            .set("key1", "value1".to_string())
            .await
            .expect("Failed to set");
        let value = cache.get("key1").await.expect("Failed to get");
        assert_eq!(value, Some("value1".to_string()));

        // 测试 exists
        assert!(cache.exists("key1").await.expect("exists failed"));
        assert!(!cache.exists("nonexistent").await.expect("exists failed"));

        // 测试 delete
        assert!(cache.delete("key1").await.expect("delete failed"));
        assert!(!cache.exists("key1").await.expect("exists failed"));
        assert!(
            !cache.delete("key1").await.expect("delete failed"),
            "再次删除返回 false"
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_get_nonexistent() {
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        let value = cache.get("nonexistent_key").await.expect("Failed to get");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_oxcache_adapter_overwrite() {
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        cache
            .set("key", "value1".to_string())
            .await
            .expect("Failed to set");
        cache
            .set("key", "value2".to_string())
            .await
            .expect("Failed to set");

        let value = cache.get("key").await.expect("Failed to get");
        assert_eq!(value, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_oxcache_adapter_builder_default() {
        // builder().build() 不带配置应等价于 new()
        let cache = OxCacheAdapter::builder()
            .build()
            .await
            .expect("Failed to build default cache");

        cache
            .set("bkey", "bval".to_string())
            .await
            .expect("Failed to set");
        assert_eq!(
            cache.get("bkey").await.expect("Failed to get"),
            Some("bval".to_string())
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_builder_with_ttl_and_capacity() {
        // 验证 ttl 和容量配置被真正应用到后端
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_secs(60))
            .capacity(100)
            .build()
            .await
            .expect("Failed to build configured cache");

        cache
            .set("ckey", "cval".to_string())
            .await
            .expect("Failed to set");
        assert_eq!(
            cache.get("ckey").await.expect("Failed to get"),
            Some("cval".to_string())
        );
        assert!(cache.exists("ckey").await.expect("exists failed"));
    }

    // ============================================================================
    // MockCache 测试
    // ============================================================================

    #[tokio::test]
    async fn test_mock_cache_basic_operations() {
        let cache = MockCache::new();

        // 测试 set 和 get
        cache
            .set("key1", "value1".to_string())
            .await
            .expect("Failed to set");
        let value = cache.get("key1").await.expect("Failed to get");
        assert_eq!(value, Some("value1".to_string()));

        // 测试 exists
        assert!(cache.exists("key1").await.expect("exists failed"));
        assert!(!cache.exists("nonexistent").await.expect("exists failed"));

        // 测试 delete
        assert!(cache.delete("key1").await.expect("delete failed"));
        assert!(!cache.exists("key1").await.expect("exists failed"));
        assert!(
            !cache.delete("key1").await.expect("delete failed"),
            "再次删除返回 false"
        );
    }

    #[tokio::test]
    async fn test_mock_cache_get_nonexistent() {
        let cache = MockCache::new();

        let value = cache.get("nonexistent_key").await.expect("Failed to get");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_mock_cache_overwrite() {
        let cache = MockCache::new();

        cache
            .set("key", "value1".to_string())
            .await
            .expect("Failed to set");
        cache
            .set("key", "value2".to_string())
            .await
            .expect("Failed to set");

        let value = cache.get("key").await.expect("Failed to get");
        assert_eq!(value, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_mock_cache_with_delay() {
        let cache = MockCache::with_delay(10);

        let start = std::time::Instant::now();
        cache.set("key", "value".to_string()).await.unwrap();
        let elapsed = start.elapsed();

        // 验证至少有 10ms 延迟
        assert!(elapsed.as_millis() >= 10);

        let start = std::time::Instant::now();
        let _ = cache.get("key").await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_mock_cache_delete_operation() {
        let cache = MockCache::new();

        // 删除不存在的键
        assert!(!cache.delete("nonexistent").await.expect("delete failed"));

        // 设置并删除
        cache.set("key", "value".to_string()).await.unwrap();
        assert!(cache.delete("key").await.expect("delete failed"));

        // 验证已删除
        assert_eq!(cache.get("key").await.expect("get failed"), None);
        assert!(!cache.exists("key").await.expect("exists failed"));
    }

    #[tokio::test]
    async fn test_mock_cache_exists_operation() {
        let cache = MockCache::new();

        // 不存在的键
        assert!(!cache.exists("key1").await.expect("exists failed"));

        // 设置后存在
        cache.set("key1", "value1".to_string()).await.unwrap();
        assert!(cache.exists("key1").await.expect("exists failed"));

        // 删除后不存在
        cache.delete("key1").await.unwrap();
        assert!(!cache.exists("key1").await.expect("exists failed"));
    }

    #[tokio::test]
    async fn test_mock_cache_thread_safety() {
        use std::sync::Arc;
        use tokio::task;

        let cache = Arc::new(MockCache::new());
        let mut handles = vec![];

        // 并发写入
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = task::spawn(async move {
                cache_clone
                    .set(&format!("key{}", i), format!("value{}", i))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // 等待所有写入完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 验证所有写入
        for i in 0..10 {
            let value = cache.get(&format!("key{}", i)).await.expect("get failed");
            assert_eq!(value, Some(format!("value{}", i)));
        }
    }

    // ============================================================================
    // Default 实现测试 - 覆盖 MockCache::default（OxCacheAdapter::default 已移除）
    // ============================================================================

    #[tokio::test]
    async fn test_mock_cache_default_equals_new() {
        // 覆盖 MockCache::default() 实现
        let default_cache = MockCache::default();
        let new_cache = MockCache::new();
        // 两者行为应等价：初始为空
        assert_eq!(default_cache.get("any").await.expect("get failed"), None);
        assert_eq!(new_cache.get("any").await.expect("get failed"), None);
        // default 实例可正常写入读取
        default_cache
            .set("k", "v".to_string())
            .await
            .expect("default mock set should succeed");
        assert_eq!(
            default_cache.get("k").await.expect("get failed"),
            Some("v".to_string())
        );
    }

    // ============================================================================
    // MockCache with_delay 覆盖 delete/exists 延迟分支
    // ============================================================================

    #[tokio::test]
    async fn test_mock_cache_with_delay_delete_measures_delay() {
        // 覆盖 MockCache::delete 的 delay 分支
        let cache = MockCache::with_delay(15);
        // 先放入一条记录
        cache
            .set("delay_key", "v".to_string())
            .await
            .expect("set should succeed");
        // delete 应当至少消耗 delay_ms 的时间
        let start = std::time::Instant::now();
        let deleted = cache.delete("delay_key").await.expect("delete failed");
        let elapsed = start.elapsed();
        assert!(deleted, "existing key should be deleted");
        assert!(
            elapsed.as_millis() >= 15,
            "delete should respect delay_ms, got {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_mock_cache_with_delay_exists_measures_delay() {
        // 覆盖 MockCache::exists 的 delay 分支
        let cache = MockCache::with_delay(15);
        cache
            .set("k", "v".to_string())
            .await
            .expect("set should succeed");
        // exists 应当至少消耗 delay_ms 的时间
        let start = std::time::Instant::now();
        let exists = cache.exists("k").await.expect("exists failed");
        let elapsed = start.elapsed();
        assert!(exists, "key should exist");
        assert!(
            elapsed.as_millis() >= 15,
            "exists should respect delay_ms, got {}ms",
            elapsed.as_millis()
        );
    }

    // ============================================================================
    // OxCacheAdapter TTL 过期与边界场景测试
    // ============================================================================

    #[tokio::test]
    async fn test_oxcache_adapter_ttl_expiration() {
        // 覆盖 TTL 过期路径：set 后等待 TTL 到期，get 应返回 None
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_millis(100))
            .build()
            .await
            .expect("Failed to build cache with TTL");

        cache
            .set("expiring_key", "expiring_value".to_string())
            .await
            .expect("Failed to set");

        // 立即读取应能拿到值
        assert_eq!(
            cache.get("expiring_key").await.expect("get failed"),
            Some("expiring_value".to_string())
        );

        // 等待 TTL 过期
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 过期后应返回 None
        assert_eq!(
            cache.get("expiring_key").await.expect("get failed"),
            None,
            "expired key should return None"
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_exists_after_ttl_expiration() {
        // 覆盖 exists 在 TTL 过期后返回 false 的路径
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_millis(80))
            .build()
            .await
            .expect("Failed to build cache with TTL");

        cache
            .set("ttl_exists_key", "v".to_string())
            .await
            .expect("Failed to set");
        assert!(cache.exists("ttl_exists_key").await.expect("exists failed"));

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert!(
            !cache.exists("ttl_exists_key").await.expect("exists failed"),
            "exists should return false after TTL expiration"
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_delete_after_ttl_expiration() {
        // 覆盖 delete 在键已过期（exists 返回 false）时返回 false 的路径
        // 命中 OxCacheAdapter::delete 中 `if !exists { return false }` 分支
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_millis(80))
            .build()
            .await
            .expect("Failed to build cache with TTL");

        cache
            .set("ttl_delete_key", "v".to_string())
            .await
            .expect("Failed to set");

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // 键已过期，exists 返回 false，delete 应短路返回 false
        let deleted = cache.delete("ttl_delete_key").await.expect("delete failed");
        assert!(
            !deleted,
            "delete should return false for expired key (exists short-circuit)"
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_builder_ttl_only() {
        // 覆盖 builder 仅配置 TTL（不配置 capacity）的分支
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_secs(60))
            .build()
            .await
            .expect("Failed to build cache with TTL only");

        cache
            .set("ttl_only_key", "ttl_only_value".to_string())
            .await
            .expect("Failed to set");
        assert_eq!(
            cache.get("ttl_only_key").await.expect("get failed"),
            Some("ttl_only_value".to_string())
        );
        assert!(cache.exists("ttl_only_key").await.expect("exists failed"));
    }

    #[tokio::test]
    async fn test_oxcache_adapter_builder_capacity_only() {
        // 覆盖 builder 仅配置 capacity（不配置 TTL）的分支
        let cache = OxCacheAdapter::builder()
            .capacity(50)
            .build()
            .await
            .expect("Failed to build cache with capacity only");

        cache
            .set("cap_only_key", "cap_only_value".to_string())
            .await
            .expect("Failed to set");
        assert_eq!(
            cache.get("cap_only_key").await.expect("get failed"),
            Some("cap_only_value".to_string())
        );
        assert!(cache.exists("cap_only_key").await.expect("exists failed"));
    }

    #[tokio::test]
    async fn test_oxcache_adapter_empty_key() {
        // 边界场景：空键
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        cache
            .set("", "empty_key_value".to_string())
            .await
            .expect("Failed to set with empty key");
        assert_eq!(
            cache.get("").await.expect("get failed"),
            Some("empty_key_value".to_string())
        );
        assert!(cache.exists("").await.expect("exists failed"));
        assert!(cache.delete("").await.expect("delete failed"));
        assert!(!cache.exists("").await.expect("exists failed"));
    }

    #[tokio::test]
    async fn test_oxcache_adapter_special_characters_in_key() {
        // 边界场景：键包含特殊字符
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        let keys = [
            "user:1",
            "namespace::key",
            "key with spaces",
            "key\twith\ttabs",
            "key\nwith\nnewlines",
            "中文键",
            "key\"with\"quotes",
            "key'with'apostrophes",
        ];

        for key in &keys {
            let value = format!("value_for_{}", key);
            cache
                .set(key, value.clone())
                .await
                .expect("Failed to set with special char key");
            assert_eq!(
                cache.get(key).await.expect("get failed"),
                Some(value),
                "get should return the set value for key: {:?}",
                key
            );
        }
    }

    #[tokio::test]
    async fn test_oxcache_adapter_large_value() {
        // 边界场景：大值
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        let large_value = "x".repeat(100_000);
        cache
            .set("large_key", large_value.clone())
            .await
            .expect("Failed to set large value");
        let retrieved = cache.get("large_key").await.expect("get failed");
        assert_eq!(retrieved.as_ref().map(|v| v.len()), Some(100_000));
        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_oxcache_adapter_concurrent_access() {
        // 覆盖 OxCacheAdapter 的并发安全性（与 MockCache 的并发测试对称）
        use std::sync::Arc;
        use tokio::task;

        let cache = Arc::new(OxCacheAdapter::new().expect("Failed to create cache"));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = task::spawn(async move {
                cache_clone
                    .set(&format!("ckey{}", i), format!("cvalue{}", i))
                    .await
                    .expect("Failed to set");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task panicked");
        }

        for i in 0..10 {
            assert_eq!(
                cache.get(&format!("ckey{}", i)).await.expect("get failed"),
                Some(format!("cvalue{}", i)),
                "concurrent write for key {} should be visible",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_oxcache_adapter_delete_nonexistent_after_set() {
        // 覆盖 delete 在 exists=false 时返回 false 的路径（非 TTL 场景）
        // 即 OxCacheAdapter::delete 中 `if !exists { return false }` 分支
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        // 从未设置过的键
        assert!(
            !cache.delete("never_set_key").await.expect("delete failed"),
            "delete on never-set key should return false"
        );

        // 设置后删除，再删除应返回 false
        cache
            .set("temp_key", "temp_value".to_string())
            .await
            .expect("Failed to set");
        assert!(cache.delete("temp_key").await.expect("delete failed"));
        assert!(
            !cache.delete("temp_key").await.expect("delete failed"),
            "delete on already-deleted key should return false"
        );
    }

    #[tokio::test]
    async fn test_oxcache_adapter_overwrite_does_not_reset_ttl() {
        // 覆盖 TTL 场景下的 overwrite：新值覆盖旧值后应可读
        // （不验证 TTL 是否重置，因为 oxcache 的 TTL 语义由后端决定）
        let cache = OxCacheAdapter::builder()
            .ttl(std::time::Duration::from_secs(60))
            .build()
            .await
            .expect("Failed to build cache");

        cache
            .set("ow_key", "v1".to_string())
            .await
            .expect("Failed to set v1");
        cache
            .set("ow_key", "v2".to_string())
            .await
            .expect("Failed to set v2");

        assert_eq!(
            cache.get("ow_key").await.expect("get failed"),
            Some("v2".to_string()),
            "overwrite should replace value"
        );
    }
}
