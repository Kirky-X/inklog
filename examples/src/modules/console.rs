// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 控制台输出示例
//!
//! 演示如何配置和使用控制台日志输出。

use inklog::config::ConsoleSinkConfig;

/// 基础控制台输出
pub fn basic_console() {
	println!("=== 基础控制台输出 ===\n");

	let config = ConsoleSinkConfig {
		enabled: true,
		colored: true,
		..Default::default()
	};

	println!("配置: {:?}", config);
}

/// 彩色控制台输出
pub fn colored_console() {
	println!("\n=== 彩色控制台输出 ===\n");
	println!("启用彩色输出后，不同日志级别会显示不同颜色:");
	println!("  TRACE - 灰色");
	println!("  DEBUG - 蓝色");
	println!("  INFO  - 绿色");
	println!("  WARN  - 黄色");
	println!("  ERROR - 红色");
}

/// stderr 级别配置
pub fn stderr_levels() {
	println!("\n=== stderr 级别配置 ===\n");
	println!("可以将特定级别的日志输出到 stderr:");
	println!("  stderr_levels = [\"error\", \"warn\"]");
}

/// 运行所有示例
pub fn run_all() {
	basic_console();
	colored_console();
	stderr_levels();
}
