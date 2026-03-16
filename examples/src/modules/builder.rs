// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Builder 模式示例
//!
//! 演示如何使用 Builder 模式配置日志系统。

/// 基础 Builder 用法
pub fn basic_builder() {
	println!("=== 基础 Builder 用法 ===\n");
	println!("let logger = LoggerManager::builder()");
	println!("    .level(\"info\")");
	println!("    .console(true)");
	println!("    .build().await?;");
}

/// 完整配置
pub fn full_builder() {
	println!("\n=== 完整 Builder 配置 ===\n");
	println!("let logger = LoggerManager::builder()");
	println!("    .level(\"debug\")");
	println!("    .format(\"{{timestamp}} [{{level}}] {{target}} - {{message}}\")");
	println!("    .console(true)");
	println!("    .console_colored(true)");
	println!("    .file(\"logs/app.log\")");
	println!("    .file_max_size(\"100MB\")");
	println!("    .file_compress(true)");
	println!("    .file_rotation_time(\"daily\")");
	println!("    .channel_capacity(10000)");
	println!("    .worker_threads(4)");
	println!("    .build().await?;");
}

/// 链式调用
pub fn method_chaining() {
	println!("\n=== 链式调用 ===\n");
	println!("Builder 支持链式调用:");
	println!("  .level(\"info\")");
	println!("  .console(true)");
	println!("  .file(\"app.log\")");
	println!("  .build()");
}

/// 运行所有示例
pub fn run_all() {
	basic_builder();
	full_builder();
	method_chaining();
}
