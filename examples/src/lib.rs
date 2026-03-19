// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
//!
//! ## Layer 1 - 本地资源示例（自动清理）
//!
//! - `file`: 文件输出、轮转、压缩
//! - `encryption`: 加密功能
//! - `performance`: 性能测试
//!
//! ## Layer 2 - 外部服务示例（可选依赖）
//!
//! - `database`: 数据库输出
//! - `http`: HTTP 健康监控
//! - `fallback`: 降级机制
//! - `s3_archive`: S3 归档
//!
//! ## 运行示例
//!
//! ```bash
//! # Layer 0 示例
//! cargo run --bin console
//! cargo run --bin template
//! cargo run --bin builder
//! cargo run --bin masking
//!
//! # Layer 1 示例
//! cargo run --bin file
//! cargo run --bin encryption
//! cargo run --bin performance
//!
//! # Layer 2 示例
//! cargo run --bin database
//! cargo run --bin http
//! cargo run --bin fallback
//! cargo run --bin s3_archive
//! ```
//!
//! ## 现有 binary（保留）
//!
//! - `basic`: 基础用法
//! - `production`: 生产环境配置
//! - `all_features`: 完整功能演示

/// 共享辅助函数（可选）
pub mod common;

pub use common::*;
