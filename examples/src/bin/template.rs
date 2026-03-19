// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 日志模板示例
//!
//! 演示 LogTemplate 的基本用法、自定义占位符和不同格式效果。
//!
//! # 运行
//! ```bash
//! cargo run --bin template
//! ```
//!
//! # 功能
//! - 基础模板渲染：演示默认模板和自定义模板
//! - 占位符展示：输出不同占位符的效果（timestamp、level、message 等）
//! - 格式对比：展示多种模板格式的渲染效果

use inklog::{LogRecord, LogTemplate};
use serde_json::Value;
use std::collections::HashMap;
use tracing::Level;

fn main() {
	println!("=== inklog 日志模板示例 ===\n");

	// 示例 1: 默认模板
	default_template();

	// 示例 2: 自定义简单模板
	custom_simple_template();

	// 示例 3: 完整占位符模板
	full_placeholders_template();

	// 示例 4: 字段占位符演示
	fields_placeholder();

	// 示例 5: 多种格式对比
	multiple_formats_comparison();

	println!("\n=== 示例运行完成 ===");
}

/// 示例 1: 默认模板
fn default_template() {
	println!("--- 示例 1: 默认模板 ---");

	// 使用默认模板：{timestamp} [{level}] {target} - {message}
	let template = LogTemplate::default();
	let record = create_sample_record();

	println!("模板格式: {{timestamp}} [{{level}}] {{target}} - {{message}}");
	println!("渲染结果: {}", template.render(&record));
	println!();
}

/// 示例 2: 自定义简单模板
fn custom_simple_template() {
	println!("--- 示例 2: 自定义简单模板 ---");

	// 创建简洁的日志格式
	let template = LogTemplate::new("[{level}] {message}");
	let record = create_sample_record();

	println!("模板格式: [{{level}}] {{message}}");
	println!("渲染结果: {}", template.render(&record));
	println!();

	// 创建详细的日志格式
	let template = LogTemplate::new("{timestamp} | {level} | {target} | {message}");
	let record = create_sample_record();

	println!("模板格式: {{timestamp}} | {{level}} | {{target}} | {{message}}");
	println!("渲染结果: {}", template.render(&record));
	println!();
}

/// 示例 3: 完整占位符模板
fn full_placeholders_template() {
	println!("--- 示例 3: 完整占位符模板 ---");

	// 使用所有可用占位符
	let template = LogTemplate::new(
		"[{timestamp}] [{level}] [{thread_id}] {target} - {message} ({file}:{line})",
	);
	let record = create_sample_record();

	println!("模板格式: [{{timestamp}}] [{{level}}] [{{thread_id}}] {{target}} - {{message}} ({{file}}:{{line}})");
	println!("渲染结果: {}", template.render(&record));
	println!();
}

/// 示例 4: 字段占位符演示
fn fields_placeholder() {
	println!("--- 示例 4: 字段占位符演示 ---");

	// 创建包含结构化字段的日志记录
	let mut record = create_sample_record();
	record.fields = HashMap::from([
		("user_id".to_string(), Value::Number(12345.into())),
		(
			"action".to_string(),
			Value::String("user_login".to_string()),
		),
		("success".to_string(), Value::Bool(true)),
	]);

	// 使用 {fields} 占位符输出所有字段
	let template = LogTemplate::new("{timestamp} [{level}] {message} | {fields}");
	println!("模板格式: {{timestamp}} [{{level}}] {{message}} | {{fields}}");
	println!("渲染结果: {}", template.render(&record));
	println!();

	// 仅输出消息和字段
	let template = LogTemplate::new("{message} {fields}");
	println!("模板格式: {{message}} {{fields}}");
	println!("渲染结果: {}", template.render(&record));
	println!();
}

/// 示例 5: 多种格式对比
fn multiple_formats_comparison() {
	println!("--- 示例 5: 多种格式对比 ---");

	let record = create_sample_record();

	let formats = vec![
		("简洁格式", "[{level}] {message}"),
		("标准格式", "{timestamp} [{level}] {target} - {message}"),
		("详细格式", "[{timestamp}] [{level}] [{thread_id}] {file}:{line} - {message}"),
		("JSON 风格", "{{\"time\":\"{timestamp}\",\"level\":\"{level}\",\"msg\":\"{message}\"}}"),
		("自定义分隔符", "{timestamp}>>>[{level}]>>> {message}"),
	];

	for (name, format) in formats {
		let template = LogTemplate::new(format);
		println!("{}: {}", name, template.render(&record));
	}
	println!();
}

/// 创建示例日志记录
fn create_sample_record() -> LogRecord {
	let mut record = LogRecord::new(
		Level::INFO,
		"my_app::module".to_string(),
		"用户登录成功".to_string(),
	);

	// 添加可选字段
	record.file = Some("src/main.rs".to_string());
	record.line = Some(42);

	record
}