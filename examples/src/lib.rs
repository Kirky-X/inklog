// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! inklog 示例库
//!
//! 本 crate 提供 inklog 库的使用示例，按分层架构组织：
//!
//! ## Layer 0 - 零依赖示例（开箱即运行）
//!
//! - `console`: 控制台输出示例
//! - `template`: 日志模板示例
//! - `builder`: Builder 模式配置
//! - `masking`: 数据脱敏示例
//! - `object_pool`: 对象池使用示例
//! - `path_validator`: 路径验证示例
//! - `log_sanitizer`: 日志净化示例
//! - `log_adapter`: log crate 兼容适配器示例
//!
//! ## Layer 1 - 本地资源示例（自动清理）
//!
//! - `file`: 文件输出、轮转、压缩
//! - `encryption`: 加密功能
//! - `performance`: 性能测试
//! - `compression`: Zstd 压缩/解压缩示例
//! - `rotation`: 日志轮转示例（按大小/按时间）
//! - `ring_buffered_file`: 环形缓冲文件 Sink 示例
//! - `config_file`: 从 TOML 配置文件加载示例
//!
//! ## Layer 2 - 外部服务示例（可选依赖）
//!
//! - `database`: 数据库输出
//! - `http`: HTTP 健康监控
//! - `fallback`: 降级机制
//! - `metrics`: 指标收集与健康监控示例
//! - `circuit_breaker`: 熔断器示例
//!
//! ## Layer 2 - 公共 API 补全示例
//!
//! - `log_level`: LogLevel 类型解析/比较/Display 示例
//! - `channel_strategy`: 自适应 Channel 策略（ChannelStrategy::Adaptive + 阈值参数）
//! - `http_auth`: HTTP 认证/IP 白名单（HttpAuthConfig, ip_whitelist）
//! - `env_overrides`: 环境变量覆盖加载（load_with_env_overrides()）
//! - `config_inspect`: 配置检查（sinks_enabled()、LoggerManager::load()）
//! - `parquet_archive`: Parquet 归档（ParquetConfig + convert_logs_to_parquet()）
//! - `database_pg_mysql`: PostgreSQL/MySQL 数据库驱动配置示例
//!
//! ## 运行示例
//!
//! ```bash
//! # Layer 0 示例
//! cargo run --bin console
//! cargo run --bin template
//! cargo run --bin builder
//! cargo run --bin masking
//! cargo run --bin object_pool
//! cargo run --bin path_validator
//! cargo run --bin log_sanitizer
//! cargo run --bin log_adapter
//!
//! # Layer 1 示例
//! cargo run --bin file
//! cargo run --bin encryption
//! cargo run --bin performance
//! cargo run --bin compression
//! cargo run --bin rotation
//! cargo run --bin ring_buffered_file
//! cargo run --bin config_file
//!
//! # Layer 2 示例
//! cargo run --bin database
//! cargo run --bin http
//! cargo run --bin fallback
//! cargo run --bin metrics
//! cargo run --bin circuit_breaker
//!
//! # Layer 2 公共 API 补全示例
//! cargo run --bin log_level
//! cargo run --bin channel_strategy
//! cargo run --bin http_auth
//! cargo run --bin env_overrides
//! cargo run --bin config_inspect
//! cargo run --bin database_pg_mysql
//! # Parquet 归档示例需要 sqlite feature
//! cargo run --bin parquet_archive --features sqlite
//! ```
//!
//! ## 现有 binary（保留）
//!
//! - `basic`: 基础用法
//! - `production`: 生产环境配置
//! - `all_features`: 完整功能演示
//! - `di_example`: 依赖注入示例

/// 共享辅助函数（可选）
pub mod common;
pub mod console_ops;
pub mod crypto_ops;
pub mod file_ops;
pub mod perf_ops;
pub mod template_ops;

pub use common::*;
