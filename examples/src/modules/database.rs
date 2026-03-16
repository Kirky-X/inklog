// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 数据库输出示例
//!
//! 演示如何配置和使用数据库日志输出。

/// 数据库配置示例
pub fn database_config() {
	println!("=== 数据库输出配置 ===\n");
	println!("支持的数据库:");
	println!("  - SQLite");
	println!("  - PostgreSQL");
	println!("  - MySQL");
	println!("\n配置示例:");
	println!("  [database_sink]");
	println!("  enabled = true");
	println!("  url = \"sqlite://logs.db\"");
	println!("  pool_size = 10");
	println!("  batch_size = 100");
}

/// 批量写入配置
pub fn batch_write() {
	println!("\n=== 批量写入配置 ===\n");
	println!("配置参数:");
	println!("  batch_size = 100  // 每批写入100条");
	println!("  flush_interval_ms = 500  // 500ms刷新间隔");
}

/// 运行所有示例
pub fn run_all() {
	database_config();
	batch_write();
}
