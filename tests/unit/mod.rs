// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试模块
//!
//! 测试 inklog 各个组件的独立功能

mod config;
mod sink;
mod archive;
mod cli;
mod concurrent;       // 新增：并发安全测试
mod memory;           // 新增：内存泄漏测试

mod unit_tests;
