// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 文件输出示例
//!
//! 演示如何配置和使用文件日志输出。

use inklog::config::FileSinkConfig;
use std::path::PathBuf;

/// 基础文件输出
pub fn basic_file() {
	println!("=== 基础文件输出 ===\n");

	let config = FileSinkConfig {
		enabled: true,
		path: PathBuf::from("logs/app.log"),
		..Default::default()
	};

	println!("配置: {:?}", config);
}

/// 文件轮转配置
pub fn file_rotation() {
	println!("\n=== 文件轮转配置 ===\n");
	println!("支持的轮转策略:");
	println!("  - hourly: 每小时轮转");
	println!("  - daily: 每天轮转");
	println!("  - weekly: 每周轮转");
	println!("  - monthly: 每月轮转");
	println!("  - 按大小轮转: max_size = \"100MB\"");
}

/// 文件保留策略
pub fn file_retention() {
	println!("\n=== 文件保留策略 ===\n");
	println!("配置示例:");
	println!("  retention_days = 30  // 保留30天");
	println!("  max_total_size = \"1GB\"  // 最大总大小");
}

/// 运行所有示例
pub fn run_all() {
	basic_file();
	file_rotation();
	file_retention();
}
