// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 降级机制示例
//!
//! 演示如何配置和使用日志降级功能。

/// 降级策略
pub fn fallback_strategy() {
	println!("=== 降级策略 ===\n");
	println!("三级降级机制:");
	println!("  1. Database -> File (数据库不可用时)");
	println!("  2. File -> Console (文件不可用时)");
	println!("  3. Console (最终降级)");
}

/// 降级配置
pub fn fallback_config() {
	println!("\n=== 降级配置 ===\n");
	println!("配置示例:");
	println!("  [fallback]");
	println!("  enabled = true");
	println!("  max_failures = 3");
	println!("  reset_timeout_secs = 30");
}

/// 健康检查
pub fn health_check() {
	println!("\n=== 健康检查 ===\n");
	println!("自动健康检查:");
	println!("  - 定期检查 sink 可用性");
	println!("  - 自动切换到备用 sink");
	println!("  - 自动恢复主 sink");
}

/// 运行所有示例
pub fn run_all() {
	fallback_strategy();
	fallback_config();
	health_check();
}
