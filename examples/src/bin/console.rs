// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Console Sink 示例
//!
//! 演示 inklog 控制台输出的三种配置方式：
//!
//! 1. **basic_console**: 基础控制台输出（无颜色）
//! 2. **colored_console**: 彩色日志输出（按级别着色）
//! 3. **stderr_levels**: 错误日志分流到 stderr
//!
//! ## 运行
//!
//! ```bash
//! cargo run --bin console
//! ```
//!
//! ## 核心特性
//!
//! - **颜色支持**：基于环境变量自动检测（NO_COLOR、CLICOLOR_FORCE、TERM）
//! - **stderr 分流**：将 ERROR/WARN 级别的日志输出到 stderr
//! - **数据脱敏**：可选的敏感信息脱敏功能

use chrono::Utc;
use inklog::config::ConsoleSinkConfig;
use inklog::sink::console::ConsoleSink;
use inklog::sink::LogSink;
use inklog::LogRecord;
use inklog_examples::common::{print_section, print_separator};

/// 示例1: 基础控制台输出（无颜色）
///
/// 演示最简单的控制台日志配置，输出到 stdout，不使用颜色。
fn basic_console() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例1: 基础控制台输出（无颜色）");

	// 创建 ConsoleSink（禁用颜色）
	let config = ConsoleSinkConfig {
		enabled: true,
		colored: false,
		stderr_levels: vec![],
		masking_enabled: false,
	};
	let mut sink = ConsoleSink::new(config, inklog::LogTemplate::default());

	// 写入各级别日志
	let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
	for level in levels {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: level.to_string(),
			message: format!("这是一条 {} 级别日志", level),
			target: "console_example::basic".to_string(),
			fields: Default::default(),
			file: None,
			line: None,
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
	}

	sink.flush()?;
	println!("\n✓ 基础控制台输出完成（无颜色）\n");
	Ok(())
}

/// 示例2: 彩色控制台输出
///
/// 演示彩色日志输出，不同级别使用不同颜色：
/// - ERROR: 红色
/// - WARN: 黄色
/// - INFO: 绿色
/// - DEBUG: 蓝色
/// - TRACE: 紫色
///
/// 颜色输出遵循环境变量控制：
/// - `NO_COLOR=1`: 禁用颜色
/// - `CLICOLOR_FORCE=1`: 强制启用颜色
/// - `TERM=dumb`: 禁用颜色
fn colored_console() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例2: 彩色控制台输出");

	// 创建 ConsoleSink（启用颜色）
	let config = ConsoleSinkConfig {
		enabled: true,
		colored: true,
		stderr_levels: vec![],
		masking_enabled: false,
	};
	let mut sink = ConsoleSink::new(config, inklog::LogTemplate::default());

	// 写入各级别日志
	print_section("彩色日志输出");
	let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
	for level in levels {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: level.to_string(),
			message: format!("这是一条 {} 级别日志（彩色）", level),
			target: "console_example::colored".to_string(),
			fields: Default::default(),
			file: None,
			line: None,
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
	}

	sink.flush()?;
	println!("\n✓ 彩色控制台输出完成\n");
	Ok(())
}

/// 示例3: stderr 日志分流
///
/// 演示将 ERROR 和 WARN 级别的日志输出到 stderr，
/// 而 INFO、DEBUG、TRACE 输出到 stdout。
///
/// 这对于日志分离收集非常有用：
/// - **stdout**: 正常日志流（可管道到文件）
/// - **stderr**: 错误日志流（可单独监控）
///
/// ## 使用场景
///
/// ```bash
/// # 将正常日志保存到文件，错误日志显示在终端
/// cargo run --bin console 2>/dev/null > app.log
///
/// # 仅查看错误日志
/// cargo run --bin console 2>&1 >/dev/null
/// ```
fn stderr_levels() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例3: stderr 日志分流");

	println!("配置：ERROR 和 WARN → stderr");
	println!("      INFO、DEBUG、TRACE → stdout\n");

	// 创建带 stderr 分流的 Console Sink
	let config = ConsoleSinkConfig {
		enabled: true,
		colored: true,
		stderr_levels: vec!["error".to_string(), "warn".to_string()],
		masking_enabled: false,
	};
	let mut sink = ConsoleSink::new(config, inklog::LogTemplate::default());

	// 写入不同级别的日志
	let test_cases = [
		("INFO", "这是 INFO 日志（→ stdout）"),
		("DEBUG", "这是 DEBUG 日志（→ stdout）"),
		("TRACE", "这是 TRACE 日志（→ stdout）"),
		("WARN", "这是 WARN 日志（→ stderr）"),
		("ERROR", "这是 ERROR 日志（→ stderr）"),
	];

	for (level, message) in test_cases {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: level.to_string(),
			message: message.to_string(),
			target: "console_example::stderr".to_string(),
			fields: Default::default(),
			file: None,
			line: None,
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
	}

	sink.flush()?;
	println!("\n✓ stderr 分流完成（检查输出流）\n");
	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog Console Sink 示例 ===\n");

	// 示例1：基础控制台输出（无颜色）
	basic_console()?;

	// 示例2：彩色控制台输出
	colored_console()?;

	// 示例3：stderr 日志分流
	stderr_levels()?;

	println!("所有示例完成！");
	Ok(())
}