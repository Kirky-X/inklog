// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Application Container - 应用级依赖注入容器
//!
//! 提供统一的依赖管理，确保组件间共享同一实例。
//!
//! ## 概述
//!
//! `InklogContainer` 是应用级依赖注入容器，统一管理所有底层依赖实例：
//!
//! - **Cache**: 缓存服务（OxCacheAdapter）
//! - **Config**: 配置服务（InklogConfigAdapter）
//! - **Database**: 数据库服务（DbNexusAdapter，需要 `dbnexus` feature）
//!
//! ## 设计原则
//!
//! - **单例共享**: 容器中的依赖实例被所有创建的 LoggerManager 共享
//! - **延迟创建**: 依赖实例在容器创建时初始化，而非首次使用
//!
//! ## 使用示例
//!
//! ### 基础用法
//!
//! ```ignore
//! use inklog::InklogContainer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建默认容器
//!     let container = InklogContainer::new()?;
//!     
//!     // 创建 LoggerManager
//!     let logger = container.create_logger().await?;
//!     
//!     // 使用共享的依赖
//!     let cache = container.cache();
//!     cache.set("key", "value".to_string()).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 从配置创建
//!
//! ```ignore
//! use inklog::{InklogContainer, InklogConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = InklogConfig::default();
//!     let container = InklogContainer::from_config(config)?;
//!     
//!     let logger = container.create_logger().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### 自定义依赖注入
//!
//! ```ignore
//! use inklog::InklogContainer;
//! use inklog::infrastructure::{OxCacheAdapter, InklogConfigAdapter};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let container = InklogContainer::builder()
//!         .cache(Arc::new(OxCacheAdapter::new()?))
//!         .config(Arc::new(InklogConfigAdapter::new()?))
//!         .build()?;
//!     
//!     let logger = container.create_logger().await?;
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use crate::InklogConfig;
use crate::InklogError;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::integrations::infra::Database;
#[cfg(test)]
use crate::integrations::infra::cache::MockCache;
use crate::integrations::infra::{Cache, Config, InklogConfigAdapter, OxCacheAdapter};
use crate::{LoggerDependencies, LoggerManager};

/// 应用级依赖注入容器
///
/// 统一管理所有底层依赖实例，确保组件间共享同一实例。
///
/// # 实例共享
///
/// 通过容器创建的所有 LoggerManager 将共享相同的依赖实例：
///
/// ```ignore
/// let container = InklogContainer::new()?;
///
/// let logger1 = container.create_logger().await?;
/// let logger2 = container.create_logger().await?;
///
/// // logger1 和 logger2 共享相同的 cache、config 实例
/// ```
///
/// # 线程安全
///
/// 容器本身是线程安全的，可以在多个线程间共享：
///
/// ```ignore
/// use std::sync::Arc;
///
/// let container = Arc::new(InklogContainer::new()?);
///
/// let c1 = container.clone();
/// let handle1 = tokio::spawn(async move {
///     c1.create_logger().await
/// });
///
/// let c2 = container.clone();
/// let handle2 = tokio::spawn(async move {
///     c2.create_logger().await
/// });
/// ```
pub struct InklogContainer {
    /// 缓存实例
    cache: Arc<dyn Cache>,

    /// 配置实例
    config: Arc<dyn Config>,

    /// 数据库实例（可选，需要 dbnexus feature）
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    database: Option<Arc<dyn Database>>,
}

impl InklogContainer {
    /// 创建默认容器
    ///
    /// 使用默认配置创建所有依赖实例：
    ///
    /// - **Cache**: `OxCacheAdapter::new()`
    /// - **Config**: `InklogConfigAdapter::new()`（从搜索路径加载配置）
    /// - **Database**: `None`（未配置）
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 错误
    ///
    /// - `InklogError::CacheError` - 缓存初始化失败
    /// - `InklogError::ConfigError` - 配置加载失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use inklog::InklogContainer;
    ///
    /// let container = InklogContainer::new()?;
    /// ```
    pub fn new() -> Result<Self, InklogError> {
        let cache = Arc::new(OxCacheAdapter::new()?);
        let config = Arc::new(InklogConfigAdapter::new()?);

        Ok(Self {
            cache,
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        })
    }

    /// 从配置创建容器
    ///
    /// 使用提供的 `InklogConfig` 创建依赖实例。
    ///
    /// # Arguments
    ///
    /// * `config` - 已加载的配置实例
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use inklog::{InklogContainer, InklogConfig};
    ///
    /// let config = InklogConfig::default();
    /// let container = InklogContainer::from_config(config)?;
    /// ```
    pub fn from_config(config: InklogConfig) -> Result<Self, InklogError> {
        let cache = Arc::new(OxCacheAdapter::new()?);
        let config = Arc::new(InklogConfigAdapter::from_config(config));

        Ok(Self {
            cache,
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        })
    }

    /// 创建容器构建器
    ///
    /// 用于自定义依赖注入的场景，如使用外部库原生类型。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use inklog::InklogContainer;
    /// use inklog::infrastructure::{OxCacheAdapter, InklogConfigAdapter};
    /// use std::sync::Arc;
    ///
    /// let container = InklogContainer::builder()
    ///     .cache(Arc::new(OxCacheAdapter::new()?))
    ///     .config(Arc::new(InklogConfigAdapter::new()?))
    ///     .build()?;
    /// ```
    pub fn builder() -> InklogContainerBuilder {
        InklogContainerBuilder::default()
    }

    /// 创建 LoggerManager，注入共享依赖
    ///
    /// 返回的 LoggerManager 将使用容器中共享的依赖实例。
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(LoggerManager)`，失败返回 `Err(InklogError)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let container = InklogContainer::new()?;
    /// let logger = container.create_logger().await?;
    ///
    /// // logger 使用容器中共享的 cache 和 config
    /// ```
    pub async fn create_logger(&self) -> Result<LoggerManager, InklogError> {
        let deps = LoggerDependencies {
            cache: Some(Arc::clone(&self.cache)),
            config: Some(Arc::clone(&self.config)),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: self.database.clone(),
        };

        LoggerManager::with_dependencies(deps).await
    }

    /// 获取共享的缓存实例
    ///
    /// 返回容器中缓存实例的克隆（`Arc` 克隆，共享底层数据）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let container = InklogContainer::new()?;
    /// let cache = container.cache();
    ///
    /// cache.set("key", "value".to_string()).await?;
    /// ```
    pub fn cache(&self) -> Arc<dyn Cache> {
        Arc::clone(&self.cache)
    }

    /// 获取共享的配置实例
    ///
    /// 返回容器中配置实例的克隆（`Arc` 克隆，共享底层数据）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let container = InklogContainer::new()?;
    /// let config = container.config();
    ///
    /// let level = config.get_string("global.level");
    /// ```
    pub fn config(&self) -> Arc<dyn Config> {
        Arc::clone(&self.config)
    }

    /// 获取共享的数据库实例（需要 dbnexus feature）
    ///
    /// 返回容器中数据库实例的克隆（如果已配置）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let container = InklogContainer::new()?;
    ///
    /// if let Some(db) = container.database() {
    ///     let healthy = db.is_healthy().await;
    /// }
    /// ```
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub fn database(&self) -> Option<Arc<dyn Database>> {
        self.database.clone()
    }

    /// 设置数据库实例（需要 dbnexus feature）
    ///
    /// 用于在容器创建后动态配置数据库连接。
    ///
    /// # Arguments
    ///
    /// * `database` - 数据库实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use inklog::infrastructure::DbNexusAdapter;
    ///
    /// let mut container = InklogContainer::new()?;
    /// let db = DbNexusAdapter::new("postgres://localhost/logs", 10).await?;
    /// container.set_database(Arc::new(db));
    /// ```
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub fn set_database(&mut self, database: Arc<dyn Database>) {
        self.database = Some(database);
    }
}

impl Default for InklogContainer {
    fn default() -> Self {
        Self::new().expect("Failed to create default InklogContainer")
    }
}

impl std::fmt::Debug for InklogContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("InklogContainer");
        builder
            .field("cache", &"Arc<dyn Cache>")
            .field("config", &"Arc<dyn Config>");
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        builder.field(
            "database",
            &self.database.as_ref().map(|_| "Arc<dyn Database>"),
        );
        builder.finish()
    }
}

// ============================================================================
// InklogContainerBuilder - 容器构建器
// ============================================================================

/// 容器构建器
///
/// 用于自定义依赖注入的场景，支持链式配置。
///
/// # 示例
///
/// ```ignore
/// use inklog::InklogContainer;
/// use inklog::infrastructure::{OxCacheAdapter, InklogConfigAdapter};
/// use std::sync::Arc;
///
/// let container = InklogContainer::builder()
///     .cache(Arc::new(OxCacheAdapter::new()?))
///     .config(Arc::new(InklogConfigAdapter::new()?))
///     .build()?;
/// ```
#[derive(Default)]
pub struct InklogContainerBuilder {
    cache: Option<Arc<dyn Cache>>,
    config: Option<Arc<dyn Config>>,
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    database: Option<Arc<dyn Database>>,
}

impl InklogContainerBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置缓存实例
    ///
    /// # Arguments
    ///
    /// * `cache` - 实现 `Cache` trait 的缓存实例
    pub fn cache(mut self, cache: Arc<dyn Cache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// 设置配置实例
    ///
    /// # Arguments
    ///
    /// * `config` - 实现 `Config` trait 的配置实例
    pub fn config(mut self, config: Arc<dyn Config>) -> Self {
        self.config = Some(config);
        self
    }

    /// 设置数据库实例
    ///
    /// # Arguments
    ///
    /// * `database` - 实现 `Database` trait 的数据库实例
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub fn database(mut self, database: Arc<dyn Database>) -> Self {
        self.database = Some(database);
        self
    }

    /// 构建容器
    ///
    /// 未设置的依赖将使用默认实现：
    ///
    /// - **Cache**: `OxCacheAdapter::new()`
    /// - **Config**: `InklogConfigAdapter::new()`
    /// - **Database**: `None`
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(InklogContainer)`，失败返回 `Err(InklogError)`
    pub fn build(self) -> Result<InklogContainer, InklogError> {
        let cache = self.cache.unwrap_or_else(|| {
            Arc::new(OxCacheAdapter::new().expect("Failed to create default cache"))
        });

        let config = self.config.unwrap_or_else(|| {
            Arc::new(InklogConfigAdapter::new().expect("Failed to create default config"))
        });

        Ok(InklogContainer {
            cache,
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: self.database,
        })
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::infra::{InklogConfigAdapter, OxCacheAdapter};
    use serial_test::serial;

    #[test]
    fn test_container_new() {
        let container = InklogContainer::new();
        assert!(container.is_ok());
    }

    #[test]
    fn test_container_default() {
        let container = InklogContainer::default();
        // 验证容器已创建
        let _cache = container.cache();
        let _config = container.config();
    }

    #[test]
    fn test_container_from_config() {
        let config = InklogConfig::default();
        let container = InklogContainer::from_config(config);
        assert!(container.is_ok());
    }

    #[test]
    fn test_container_builder_default() {
        let container = InklogContainer::builder().build();
        assert!(container.is_ok());
    }

    #[test]
    fn test_container_builder_with_adapters() {
        let container = InklogContainer::builder()
            .cache(Arc::new(
                OxCacheAdapter::new().expect("Failed to create cache"),
            ))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build();

        assert!(container.is_ok());
    }

    #[tokio::test]
    async fn test_container_cache_shared() {
        let container = InklogContainer::builder()
            .cache(Arc::new(
                OxCacheAdapter::new().expect("Failed to create cache"),
            ))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        let cache1 = container.cache();
        let cache2 = container.cache();

        // 设置值
        cache1.set("key", "value".to_string()).await.unwrap();

        // 另一个引用应该能看到
        let value = cache2.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[test]
    fn test_container_config_shared() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        let container = InklogContainer::builder()
            .cache(Arc::new(
                OxCacheAdapter::new().expect("Failed to create cache"),
            ))
            .config(Arc::new(adapter))
            .build()
            .unwrap();

        let config1 = container.config();
        let config2 = container.config();

        // 验证两个引用共享同一配置
        assert_eq!(
            config1.get_string("global.level"),
            config2.get_string("global.level")
        );
    }

    /// 测试创建 LoggerManager
    ///
    /// 注意：需要使用 multi_thread flavor 因为 LoggerManager 内部使用 blocking 操作。
    /// 使用 serial 宏确保测试顺序执行，避免全局状态冲突。
    #[tokio::test(flavor = "multi_thread")]
    #[serial]
    async fn test_container_create_logger() {
        let container = InklogContainer::builder()
            .cache(Arc::new(
                OxCacheAdapter::new().expect("Failed to create cache"),
            ))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        let result = container.create_logger().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_debug() {
        let container = InklogContainer::builder()
            .cache(Arc::new(
                OxCacheAdapter::new().expect("Failed to create cache"),
            ))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        let debug_str = format!("{:?}", container);
        assert!(debug_str.contains("InklogContainer"));
        assert!(debug_str.contains("Arc<dyn Cache>"));
        assert!(debug_str.contains("Arc<dyn Config>"));
    }

    #[test]
    fn test_container_with_mock_cache() {
        // 验证 MockCache 可以用于 DI 容器
        let container = InklogContainer::builder()
            .cache(Arc::new(MockCache::new()))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build();

        assert!(container.is_ok());
    }

    #[tokio::test]
    async fn test_container_mock_cache_operations() {
        // 验证 MockCache 在容器中的操作
        let container = InklogContainer::builder()
            .cache(Arc::new(MockCache::new()))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        let cache = container.cache();

        // 测试基本操作
        cache
            .set("test_key", "test_value".to_string())
            .await
            .unwrap();
        let value = cache.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // 测试存在检查
        assert!(cache.exists("test_key").await.unwrap());

        // 测试删除
        assert!(cache.delete("test_key").await.unwrap());
        assert!(!cache.exists("test_key").await.unwrap());
    }

    #[test]
    fn test_container_builder_new() {
        // 直接调用 InklogContainerBuilder::new()（覆盖 new 方法体）
        let builder = InklogContainerBuilder::new();
        let container = builder.build();
        assert!(container.is_ok());
    }

    #[test]
    fn test_container_builder_new_equals_default() {
        // new() 内部调用 default()，两者应等价
        let from_new = InklogContainerBuilder::new();
        let from_default = InklogContainerBuilder::default();

        // 两者都能成功构建
        assert!(from_new.build().is_ok());
        assert!(from_default.build().is_ok());
    }

    // ============================================================================
    // dbnexus feature 下的 database 方法测试
    // ============================================================================

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[tokio::test]
    async fn test_container_database_returns_none_by_default() {
        // 覆盖 database() 方法（行 301-302）
        // 默认创建的容器 database 字段为 None
        let container = InklogContainer::builder()
            .cache(Arc::new(MockCache::new()))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        // 默认未配置数据库，应返回 None
        assert!(container.database().is_none());
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[tokio::test]
    async fn test_container_set_database_and_get() {
        // 覆盖 set_database() 方法（行 323-324）
        use crate::LogRecord;
        use crate::integrations::infra::MockDatabaseAdapter;

        let mut container = InklogContainer::builder()
            .cache(Arc::new(MockCache::new()))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .build()
            .unwrap();

        // 初始状态：database 为 None
        assert!(container.database().is_none());

        // 设置数据库实例（保留原始 Arc<MockDatabaseAdapter> 用于验证共享状态）
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let db: Arc<dyn Database> = Arc::clone(&mock_db) as Arc<dyn Database>;
        container.set_database(Arc::clone(&db));

        // 获取并验证：database 应返回 Some
        let retrieved = container.database();
        assert!(retrieved.is_some());

        // 验证返回的实例健康
        let retrieved_db = retrieved.unwrap();
        assert!(retrieved_db.is_healthy().await);

        // 通过返回的实例插入记录
        let records = vec![LogRecord::new(
            tracing::Level::INFO,
            "test::module".to_string(),
            "test message".to_string(),
        )];
        let count = retrieved_db.insert_batch(&records).await.unwrap();
        assert_eq!(count, 1);

        // 通过原始 Arc<MockDatabaseAdapter> 验证记录已写入（共享底层存储）
        assert_eq!(mock_db.record_count(), 1);
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[tokio::test]
    async fn test_container_builder_database() {
        // 覆盖 builder.database() 方法（行 409-411）
        use crate::integrations::infra::MockDatabaseAdapter;

        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let db: Arc<dyn Database> = Arc::clone(&mock_db) as Arc<dyn Database>;

        // 通过 builder 设置 database
        let container = InklogContainer::builder()
            .cache(Arc::new(MockCache::new()))
            .config(Arc::new(InklogConfigAdapter::from_config(
                InklogConfig::default(),
            )))
            .database(Arc::clone(&db))
            .build()
            .unwrap();

        // 验证 database 已被设置
        let retrieved = container.database();
        assert!(retrieved.is_some());

        // 验证返回的实例可用
        let retrieved_db = retrieved.unwrap();
        assert!(retrieved_db.is_healthy().await);

        // 通过原始 Arc<MockDatabaseAdapter> 验证实例共享（未插入记录时应为 0）
        assert_eq!(mock_db.record_count(), 0);
    }
}
