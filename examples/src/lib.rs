// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! inklog 示例库
//!
//! 本 crate 提供 inklog 库的使用示例。
//!
//! ## 运行示例
//!
//! ```bash
//! # 运行基础示例
//! cargo run --bin basic
//!
//! # 运行生产环境示例
//! cargo run --bin production
//!
//! # 运行完整功能演示
//! cargo run --bin all_features
//! ```
//!
//! ## 模块说明
//!
//! - `builder`: Builder 模式配置
//! - `console`: 控制台输出
//! - `file`: 文件输出
//! - `database`: 数据库输出
//! - `encryption`: 加密功能
//! - `compression`: 压缩功能
//! - `masking`: 数据脱敏
//! - `http`: HTTP 健康监控
//! - `s3_archive`: S3 归档
//! - `fallback`: 降级机制
//! - `performance`: 性能配置
//! - `template`: 日志模板

pub mod modules;

pub use modules::*;
