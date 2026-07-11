// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 输出端单元测试模块
//!
//! 测试各种日志输出端（文件、控制台、数据库等）

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod entity_test;
