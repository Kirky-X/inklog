// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 日志模板示例
//!
//! 演示如何自定义日志输出格式。

use inklog::template::LogTemplate;

/// 基础模板
pub fn basic_template() {
	println!("=== 基础日志模板 ===\n");

	let _template = LogTemplate::new("{timestamp} [{level}] {target} - {message}");
	println!("模板已创建");
}

/// 可用占位符
pub fn placeholders() {
	println!("\n=== 可用占位符 ===\n");
	println!("支持的占位符:");
	println!("  {{timestamp}}  - 时间戳");
	println!("  {{level}}      - 日志级别");
	println!("  {{target}}     - 目标模块");
	println!("  {{message}}    - 日志消息");
	println!("  {{fields}}     - 结构化字段");
	println!("  {{file}}       - 源文件");
	println!("  {{line}}       - 行号");
	println!("  {{thread_id}}  - 线程ID");
}

/// 自定义模板示例
pub fn custom_templates() {
	println!("\n=== 自定义模板示例 ===\n");

	let templates = [
		"{timestamp} | {level} | {message}",
		"[{level}] {target}: {message}",
		"{timestamp} [{level}] {target} ({file}:{line}) - {message}",
		"{level:5} {message} {fields}",
	];

	for t in &templates {
		println!("  {}", t);
	}
}

/// 运行所有示例
pub fn run_all() {
	basic_template();
	placeholders();
	custom_templates();
}
