// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 生产环境配置示例
//!
//! 演示如何在生产环境中配置 inklog。
//!
//! # 运行
//! ```bash
//! cargo run --bin production
//! ```

use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 生产环境配置示例 ===\n");

	// 使用 Builder 模式配置日志系统
	let _logger = LoggerManager::builder()
		.level("info")
		.format("{timestamp} [{level}] {target} - {message}")
		.console(true)
		.console_colored(true)
		.file("logs/production.log")
		.file_max_size("100MB")
		.file_compress(true)
		.file_rotation_time("daily")
		.file_keep_files(30)
		.channel_capacity(10000)
		.build()
		.await?;

	tracing::info!("生产环境日志系统已启动");

	// 模拟应用日志
	for i in 1..=5 {
		tracing::info!(
			request_id = %uuid::Uuid::new_v4(),
			iteration = i,
			"处理请求"
		);
	}

	tracing::warn!(
		component = "database",
		latency_ms = 150,
		"数据库查询延迟较高"
	);

	tracing::error!(
		error_code = "E001",
		component = "cache",
		"缓存连接失败"
	);

	println!("\n日志已输出到控制台和文件 (logs/production.log)");
	println!("按 Ctrl+C 退出...");

	tokio::signal::ctrl_c().await?;

	Ok(())
}
