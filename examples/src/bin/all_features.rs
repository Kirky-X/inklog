// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 完整功能演示
//!
//! 演示 inklog 的所有主要功能。
//!
//! # 运行
//! ```bash
//! cargo run --bin all_features
//! ```

use inklog::{LoggerManager, masking::DataMasker, template::LogTemplate};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 完整功能演示 ===\n");

	// 1. 数据脱敏演示
	println!("1. 数据脱敏功能:");
	let masker = DataMasker::new();
	let sensitive_message = "用户登录: email=test@example.com, phone=13812345678, password=secret123";
	let masked = masker.mask(sensitive_message);
	println!("   原始: {}", sensitive_message);
	println!("   脱敏: {}\n", masked);

	// 2. 自定义模板演示
	println!("2. 自定义日志模板:");
	let _template = LogTemplate::new("{timestamp} | {level} | {target} | {message}");
	println!("   模板已创建\n");

	// 3. 配置日志系统
	let _logger = LoggerManager::builder()
		.level("debug")
		.format("{timestamp} [{level}] {target} - {message}")
		.console(true)
		.console_colored(true)
		.file("logs/all_features.log")
		.file_max_size("50MB")
		.file_compress(true)
		.file_rotation_time("hourly")
		.channel_capacity(5000)
		.build()
		.await?;

	tracing::info!("日志系统已启动，所有功能已就绪");

	// 4. 不同级别的日志
	println!("3. 日志级别演示:");
	tracing::trace!("TRACE 级别 - 最详细的调试信息");
	tracing::debug!("DEBUG 级别 - 调试信息");
	tracing::info!("INFO 级别 - 常规信息");
	tracing::warn!("WARN 级别 - 警告信息");
	tracing::error!("ERROR 级别 - 错误信息");

	// 5. 结构化日志
	println!("\n4. 结构化日志:");
	tracing::info!(
		user_id = 12345,
		action = "purchase",
		amount = 99.99,
		currency = "USD",
		"用户购买商品"
	);

	// 6. 错误处理
	println!("\n5. 错误处理:");
	tracing::error!(
		error_code = "E001",
		component = "payment",
		retry_count = 3,
		"支付处理失败"
	);

	println!("\n演示完成！日志已输出到控制台和文件 (logs/all_features.log)");
	println!("按 Ctrl+C 退出...");

	tokio::signal::ctrl_c().await?;

	Ok(())
}
