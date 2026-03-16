// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
//! - **S3 归档**: 支持 AWS S3 云存储归档
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
//! use inklog::LoggerManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

pub mod archive;
pub mod cache;
pub mod config;
mod error;
pub mod infrastructure;
pub mod log_adapter;
pub mod log_record;
mod manager;
pub mod masking;
pub mod metrics;
mod object_pool;
pub mod sink;
pub mod subscriber;
pub mod template;

pub use config::{
    ChannelStrategy, ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig,
    PerformanceConfig,
};

pub use error::InklogError;
pub use log_adapter::{LogAdapter, LogLogger};
pub use log_record::LogRecord;
pub use manager::{LoggerBuilder, LoggerManager};
pub use metrics::{
    FallbackAction, FallbackConfig, FallbackState, FallbackStats, HealthStatus, Metrics,
    SinkHealthMonitor, SinkStatus,
};
pub use object_pool::ObjectPool;
pub use template::LogTemplate;

#[cfg(feature = "aws")]
pub use archive::{ArchiveService, ArchiveServiceBuilder, S3ArchiveConfig, S3ArchiveManager};
