// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 基础用法示例
//!
//! 演示 inklog 的最简单使用方式。
//!
//! # 运行
//! ```bash
//! cargo run --bin basic
//! ```

use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 基础用法示例 ===\n");

	// 使用默认配置初始化日志系统
	let _logger = LoggerManager::new().await?;

	// 使用 tracing 宏记录日志
	tracing::info!("日志系统已启动");

	// 不同级别的日志
	tracing::trace!("这是 TRACE 级别日志");
	tracing::debug!("这是 DEBUG 级别日志");
	tracing::info!("这是 INFO 级别日志");
	tracing::warn!("这是 WARN 级别日志");
	tracing::error!("这是 ERROR 级别日志");

	// 结构化日志
	tracing::info!(
		user_id = 12345,
		action = "login",
		ip = "192.168.1.1",
		"用户登录成功"
	);

	println!("\n日志已输出到控制台");
	println!("按 Ctrl+C 退出...");

	// 保持程序运行
	tokio::signal::ctrl_c().await?;

	Ok(())
}
