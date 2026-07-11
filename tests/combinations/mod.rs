// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 特性组合测试模块
//!
//! 测试 inklog 多个特性组合使用的场景，包括：
//! - 加密 + 压缩
//! - 多 Sink 降级 + 监控
//! - 数据掩码 + 格式化 + 多 Sink

mod encryption_file_test;
mod multi_sink_fallback_test;
mod complex_features_test;  // 新增：复杂特性组合测试
