// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试模块
//!
//! 测试 inklog 系统的完整功能集成，包括：
//! - 归档调度测试
//! - 自动恢复测试
//! - 批量写入测试
//! - 配置环境测试
//! - HTTP 服务器测试
//! - Parquet 测试
//! - 稳定性测试
//! - 验证测试

mod archive;
mod batch;
mod config;
mod http;
mod parquet;
mod recovery;
mod stability;
mod verification;
