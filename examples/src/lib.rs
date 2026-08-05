// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! inklog 示例库
//!
//! 本 crate 提供 inklog 库的使用示例，按功能域模块化组织：
//!
//! ## 模块结构
//!
//! - `core/` — 核心用法：basic, builder, template, i18n, production, all_features
//! - `config/` — 配置管理：config_file, config_inspect, env_overrides
//! - `sinks/` — 输出 Sink：console, file, rotation, compression, ring_buffered_file, parquet_archive, partition_strategy
//! - `database/` — 数据库：database, database_pg_mysql, di_example
//! - `network/` — 网络服务：http, http_auth, tls_config
//! - `infra/` — 基础设施：fallback, circuit_breaker, performance, runtime_ops, channel_strategy, object_pool, metrics, log_level, log_adapter
//! - `security/` — 安全：masking, encryption, log_sanitizer, path_validator
//!
//! ## 运行示例
//!
//! ```bash
//! # 核心用法
//! cargo run --bin basic
//! cargo run --bin builder
//! cargo run --bin template
//! cargo run --bin i18n
//! cargo run --bin production
//! cargo run --bin all_features
//!
//! # 配置管理
//! cargo run --bin config_file
//! cargo run --bin config_inspect
//! cargo run --bin env_overrides
//!
//! # 输出 Sink
//! cargo run --bin console
//! cargo run --bin file
//! cargo run --bin rotation
//! cargo run --bin compression --features compression
//! cargo run --bin ring_buffered_file
//! cargo run --bin parquet_archive --features sqlite
//! cargo run --bin partition_strategy
//!
//! # 数据库
//! cargo run --bin database --features sqlite
//! cargo run --bin database_pg_mysql
//! cargo run --bin di_example --features sqlite
//!
//! # 网络服务
//! cargo run --bin http
//! cargo run --bin http_auth
//! cargo run --bin tls_config
//!
//! # 基础设施
//! cargo run --bin fallback
//! cargo run --bin circuit_breaker
//! cargo run --bin performance
//! cargo run --bin runtime_ops
//! cargo run --bin channel_strategy
//! cargo run --bin object_pool
//! cargo run --bin metrics
//! cargo run --bin log_level
//! cargo run --bin log_adapter
//!
//! # 安全
//! cargo run --bin masking
//! cargo run --bin encryption
//! cargo run --bin log_sanitizer
//! cargo run --bin path_validator
//! ```

/// 共享辅助函数（可选）
pub mod common;
pub mod console_ops;
pub mod crypto_ops;
pub mod file_ops;
pub mod perf_ops;
pub mod template_ops;

pub use common::*;
