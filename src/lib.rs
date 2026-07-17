// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
#![doc(html_root_url = "https://docs.rs/inklog/0.1.7")]

//! # inklog - 企业级 Rust 日志基础设施
//!
//! inklog 是一个高性能、可扩展的日志库，专为生产环境设计。
//!
//! ## 功能特性
//!
//! - **多输出目标**: 支持 Console、File、Database 三种输出通道
//! - **日志轮转**: 支持按大小和按时间轮转
//! - **压缩与加密**: 支持 Zstandard 压缩和 AES-256-GCM 加密
//! - **批量写入**: 数据库批量写入，可配置批次大小和刷新间隔
//! - **降级机制**: DB → File → Console 三级降级
//! - **健康监控**: HTTP 端点暴露健康状态和 Prometheus 指标
//!
//! ## 快速开始
//!
//! ### 基础用法
//!
//! ```rust,no_run
//! use inklog::{LoggerManager, InklogConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 使用默认配置初始化
//!     let _logger = LoggerManager::new().await?;
//!     
//!     // 使用 tracing 宏记录日志
//!     tracing::info!("Hello, inklog!");
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 使用 Builder 模式配置
//!
//! ```rust,no_run
//! # #[cfg(not(feature = "http"))]
//! # fn main() {}
//! # #[cfg(feature = "http")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     use inklog::LoggerManager;
//!
//!     let _logger = LoggerManager::builder()
//!         .level("debug")
//!         .console(true)
//!         .file("logs/app.log")
//!         .enable_http_server(true)
//!         .http_port(9090)
//!         .build()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 从配置文件加载
//!
//! ```rust,no_run
//! use inklog::LoggerManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 从指定文件加载
//!     let _logger = LoggerManager::from_file("config.toml").await?;
//!     
//!     // 或自动搜索配置文件
//!     // let _logger = LoggerManager::load().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 使用依赖注入模式
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use inklog::{LoggerManager, LoggerDependencies, InklogContainer};
//! use inklog::infrastructure::{OxCacheAdapter, InklogConfigAdapter};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 方式 1: 使用依赖注入容器
//!     let container = InklogContainer::new()?;
//!     let logger = container.create_logger().await?;
//!     
//!     // 方式 2: 使用 Builder 模式注入依赖
//!     let logger = LoggerManager::builder()
//!         .cache(Arc::new(OxCacheAdapter::new()?))
//!         .config(Arc::new(InklogConfigAdapter::new()?))
//!         .build().await?;
//!     
//!     // 方式 3: 使用 with_dependencies
//!     let deps = LoggerDependencies {
//!         cache: Some(Arc::new(OxCacheAdapter::new()?)),
//!         config: Some(Arc::new(InklogConfigAdapter::new()?)),
//!         ..Default::default()
//!     };
//!     let logger = LoggerManager::with_dependencies(deps).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## 配置文件示例 (TOML)
//!
//! ```toml
//! [global]
//! level = "info"
//!
//! [console_sink]
//! enabled = true
//!
//! [file_sink]
//! enabled = true
//! path = "logs/app.log"
//! max_size = "100MB"
//! rotation = "daily"
//!
//! [http_server]
//! enabled = true
//! host = "127.0.0.1"
//! port = 8080
//! ```

mod error;
mod log_level;
mod validation;

// ICU4X-backed internationalization (optional, enabled via `i18n` feature)
#[cfg(feature = "i18n")]
pub mod i18n;

// Backwards compatibility - expose modules at root level
pub use domain::config;
pub use domain::types::log_record;
pub use support::io::sink;
pub use support::processing::template;

// Domain layer
pub mod domain;

// Support layer
pub mod support;

// Re-export masking for benchmarks
pub use support::processing::masking;

// Integrations layer
pub mod integrations;

// Re-export types from domain layer for backwards compatibility
pub use domain::config::{
    ChannelStrategy, ConsoleSinkConfig, DatabaseDriver, DatabaseSinkConfig, FileSinkConfig,
    GlobalConfig, HttpAuthConfig, HttpErrorMode, HttpServerConfig, InklogConfig, ParquetConfig,
    PartitionStrategy, PerformanceConfig,
};
pub use domain::db_provider::LogDbProvider;
pub use domain::types::log_record::LogRecord;
pub use error::InklogError;
pub use error::InklogResult;
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub use integrations::DbNexusLogDbAdapter;
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub use integrations::InklogModule;

pub use domain::core::{
    InklogContainer, InklogContainerBuilder, LoggerBuilder, LoggerDependencies, LoggerManager,
};

pub use log_level::{LogLevel, LogLevelParseError};
pub use support::io::{LogAdapter, LogLogger};
pub use support::observability::{
    FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealthMonitor, SinkStatus,
};
pub use support::processing::{
    DataMasker, LogTemplate, ObjectPool, ObjectPoolConfig, get_log_record, get_string_buffer,
    put_log_record, put_string_buffer,
};
pub use validation::{
    EscapeMode, LogSanitizer, PathValidator, PathValidatorConfig, SanitizerConfig, ValidationResult,
};

// Re-export underlying dependencies used in public API type signatures.
//
// Scope: only type references (L1) — e.g. `use inklog::tracing::Level`,
// `use inklog::chrono::DateTime`. Macro attributes (L2, `#[tokio::main]`)
// and macro invocations (L3, `tracing::info!`) reference absolute crate
// paths at expansion time and cannot be routed through a re-export alias;
// downstream crates must still declare direct dependencies for those uses.
// This re-export therefore narrows the direct-dependency surface to the
// macro path only; it does not eliminate it.
pub use chrono;
pub use serde;
pub use tokio;
pub use tracing;
