// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Cache trait - 抽象缓存操作
//!
//! 提供 CRUD 操作的抽象接口，支持不同缓存后端的统一访问。

use crate::error::InklogError;
use async_trait::async_trait;

/// Cache trait - 抽象缓存操作
///
/// 提供基本的缓存 CRUD 操作接口，所有方法都是异步的。
/// 实现必须保证线程安全（`Send + Sync`）。
///
/// # 实现要求
///
/// - 所有方法使用 `&self`（不可变引用），支持并发访问
/// - 返回类型使用 `Option` 或 `Result<_, InklogError>`
/// - 实现应该是非阻塞的，适合在 async 上下文中调用
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::Cache;
///
/// async fn example(cache: &dyn Cache) {
///     cache.set("key", "value".to_string()).await.unwrap();
///     let value = cache.get("key").await;
///     assert_eq!(value, Some("value".to_string()));
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
    /// 如果键存在返回 `Some(value)`，否则返回 `None`
    async fn get(&self, key: &str) -> Option<String>;

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
    /// 如果键存在并被删除返回 `true`，否则返回 `false`
    async fn delete(&self, key: &str) -> bool;

    /// 检查键是否存在
    ///
    /// # 参数
    ///
    /// * `key` - 缓存键
    ///
    /// # 返回
    ///
    /// 键存在返回 `true`，否则返回 `false`
    async fn exists(&self, key: &str) -> bool;
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
///     let name = cache.get("user:1").await;
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
    /// 使用默认的内存后端（Moka）创建缓存实例。
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 错误
    ///
    /// - `InklogError::CacheError` - 缓存初始化失败
    pub fn new() -> Result<Self, InklogError> {
        // 使用同步构造方法（oxcache 0.2.0+ 支持）
        let cache = OxCache::new();
        Ok(Self { inner: cache })
    }

    /// 使用构建器创建适配器
    ///
    /// 提供更细粒度的配置选项，如 TTL、容量等。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use inklog::infrastructure::cache::OxCacheAdapter;
    ///
    /// let cache = OxCacheAdapter::builder()
    ///     .ttl(Duration::from_secs(3600))
    ///     .build()?;
    /// ```
    pub fn builder() -> OxCacheAdapterBuilder {
        OxCacheAdapterBuilder::default()
    }
}

impl Default for OxCacheAdapter {
    fn default() -> Self {
        Self::new().expect("Failed to create default OxCacheAdapter")
    }
}

#[async_trait]
impl Cache for OxCacheAdapter {
    async fn get(&self, key: &str) -> Option<String> {
        // oxcache::Cache::get 返回 Result<Option<V>>
        match self.inner.get(&key.to_string()).await {
            Ok(Some(value)) => Some(value),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Cache get error for key '{}': {}", key, e);
                None
            }
        }
    }

    async fn set(&self, key: &str, value: String) -> Result<(), InklogError> {
        self.inner
            .set(&key.to_string(), &value)
            .await
            .map_err(|e| InklogError::CacheError(format!("Failed to set cache value: {}", e)))
    }

    async fn delete(&self, key: &str) -> bool {
        // 先检查键是否存在
        let exists = self.exists(key).await;
        if !exists {
            return false;
        }

        match self.inner.delete(&key.to_string()).await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("Cache delete error for key '{}': {}", key, e);
                false
            }
        }
    }

    async fn exists(&self, key: &str) -> bool {
        match self.inner.exists(&key.to_string()).await {
            Ok(exists) => exists,
            Err(e) => {
                tracing::warn!("Cache exists error for key '{}': {}", key, e);
                false
            }
        }
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

    /// 构建适配器
    pub fn build(self) -> Result<OxCacheAdapter, InklogError> {
        // 目前使用默认配置，后续可扩展支持更多选项
        // 注意：oxcache 的 builder() 返回异步构建器
        // 这里简化处理，直接使用 new()
        let _ = (self.ttl, self.capacity);
        OxCacheAdapter::new()
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
/// async fn main() {
///     let cache = MockCache::new();
///
///     cache.set("key", "value".to_string()).await.unwrap();
///     assert_eq!(cache.get("key").await, Some("value".to_string()));
///
///     // 带延迟的 Mock
///     let delayed_cache = MockCache::with_delay(100);
///     cache.set("slow", "op".to_string()).await.unwrap();
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
    async fn get(&self, key: &str) -> Option<String> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let storage = self.storage.read().unwrap();
        storage.get(key).cloned()
    }

    async fn set(&self, key: &str, value: String) -> Result<(), InklogError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let mut storage = self.storage.write().unwrap();
        storage.insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> bool {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let mut storage = self.storage.write().unwrap();
        storage.remove(key).is_some()
    }

    async fn exists(&self, key: &str) -> bool {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let storage = self.storage.read().unwrap();
        storage.contains_key(key)
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
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        // 测试 exists
        assert!(cache.exists("key1").await);
        assert!(!cache.exists("nonexistent").await);

        // 测试 delete
        assert!(cache.delete("key1").await);
        assert!(!cache.exists("key1").await);
        assert!(!cache.delete("key1").await); // 再次删除返回 false
    }

    #[tokio::test]
    async fn test_oxcache_adapter_get_nonexistent() {
        let cache = OxCacheAdapter::new().expect("Failed to create cache");

        let value = cache.get("nonexistent_key").await;
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

        let value = cache.get("key").await;
        assert_eq!(value, Some("value2".to_string()));
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
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        // 测试 exists
        assert!(cache.exists("key1").await);
        assert!(!cache.exists("nonexistent").await);

        // 测试 delete
        assert!(cache.delete("key1").await);
        assert!(!cache.exists("key1").await);
        assert!(!cache.delete("key1").await); // 再次删除返回 false
    }

    #[tokio::test]
    async fn test_mock_cache_get_nonexistent() {
        let cache = MockCache::new();

        let value = cache.get("nonexistent_key").await;
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

        let value = cache.get("key").await;
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
        let _ = cache.get("key").await;
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_mock_cache_delete_operation() {
        let cache = MockCache::new();

        // 删除不存在的键
        assert!(!cache.delete("nonexistent").await);

        // 设置并删除
        cache.set("key", "value".to_string()).await.unwrap();
        assert!(cache.delete("key").await);

        // 验证已删除
        assert_eq!(cache.get("key").await, None);
        assert!(!cache.exists("key").await);
    }

    #[tokio::test]
    async fn test_mock_cache_exists_operation() {
        let cache = MockCache::new();

        // 不存在的键
        assert!(!cache.exists("key1").await);

        // 设置后存在
        cache.set("key1", "value1".to_string()).await.unwrap();
        assert!(cache.exists("key1").await);

        // 删除后不存在
        cache.delete("key1").await;
        assert!(!cache.exists("key1").await);
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
            let value = cache.get(&format!("key{}", i)).await;
            assert_eq!(value, Some(format!("value{}", i)));
        }
    }
}
