// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Infrastructure module
//!
//! 提供基础设施层的抽象 trait 接口和适配器实现，用于依赖注入。
//!
//! ## 概述
//!
//! 本模块定义了三个核心 trait 和对应的适配器实现：
//!
//! | Trait | 描述 | 适配器 | Mock 实现 |
//! |-------|------|--------|-----------|
//! | [`Cache`] | 缓存操作 | `OxCacheAdapter` | `MockCache` |
//! | [`Config`] | 配置访问 | `InklogConfigAdapter` | `MockConfig` |
//! | [`Database`] | 数据库操作 | `DbNexusAdapter` | `MockDatabaseAdapter` |
//!
//! ## 设计原则
//!
//! - **依赖倒置**：高层模块依赖抽象接口，而非具体实现
//! - **可替换性**：实现可运行时切换，无需修改调用代码
//!
//! ## 使用示例
//!
//! ```ignore
//! use inklog::infrastructure::{Cache, Config, Database};
//! use inklog::infrastructure::{OxCacheAdapter, InklogConfigAdapter, DbNexusAdapter};
//!
//! async fn process_logs(
//!     cache: &dyn Cache,
//!     config: &dyn Config,
//!     db: &dyn Database,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // 从配置获取参数
//!     let batch_size = config.get_int("batch.size").unwrap_or(100);
//!     
//!     // 检查数据库健康状态
//!     if !db.is_healthy().await {
//!         return Err("Database unhealthy".into());
//!     }
//!     
//!     // 使用缓存
//!     if let Some(cached) = cache.get("last_sync").await {
//!         println!("Last sync: {}", cached);
//!     }
//!     
//!     Ok(())
//! }
//!
//! // 使用具体实现
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cache = OxCacheAdapter::new()?;
//!     let config = InklogConfigAdapter::new()?;
//!     let db = DbNexusAdapter::new("postgres://localhost/logs", 10).await?;
//!     
//!     process_logs(&cache, &config, &db).await
//! }
//! ```
//!
//! ## 条件编译
//!
//! - `DbNexusAdapter` 需要启用 `dbnexus` feature
//! - `OxCacheAdapter`、`InklogConfigAdapter` 默认可用

pub mod cache;
pub mod config;
pub mod database;

// Trait 导出
pub use cache::Cache;
pub use config::Config;
pub use database::Database;

// 适配器导出
pub use cache::OxCacheAdapter;
pub use config::InklogConfigAdapter;

#[cfg(feature = "dbnexus")]
pub use database::DbNexusAdapter;

// ============================================================================
// 构建器导出
// ============================================================================
pub use cache::OxCacheAdapterBuilder;

// ============================================================================
// Mock 实现导出（用于测试）
// ============================================================================
pub use cache::MockCache;
pub use config::MockConfig;
pub use database::MockDatabaseAdapter;
