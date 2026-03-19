// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! File Sink 示例
//!
//! 演示 inklog 文件输出的三种核心功能：
//!
//! 1. **basic_file**: 基础文件写入
//! 2. **file_rotation**: 文件轮转（按大小）
//! 3. **file_compression**: Zstd 压缩
//!
//! ## 运行
//!
//! ```bash
//! cargo run --bin file
//! ```
//!
//! ## 核心特性
//!
//! - **文件轮转**: 按 size/time 自动轮转，支持保留文件数量控制
//! - **压缩**: 支持 ZSTD、GZIP、Brotli 压缩算法
//! - **加密**: 支持 AES-256-GCM 加密（见 encryption 示例）
//! - **清理**: 自动清理过期日志文件（retention_days）
//!
//! ## 文件命名规则
//!
//! - 当前日志: `app.log`
//! - 轮转日志: `app.log.1`, `app.log.2`, ...
//! - 压缩日志: `app.log.1.zst`, `app.log.2.zst`, ...

use chrono::Utc;
use inklog::config::FileSinkConfig;
use inklog::sink::file::FileSink;
use inklog::sink::LogSink;
use inklog::LogRecord;
use inklog_examples::common::{print_section, print_separator, temp_file_path};
use std::fs;
use std::path::PathBuf;

/// 示例1: 基础文件写入
///
/// 演示最基本的文件日志写入功能：
/// - 创建 FileSink 配置
/// - 写入多级别日志
/// - 查看文件内容
fn basic_file() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例1: 基础文件写入");

	// 生成临时文件路径
	let log_path = temp_file_path("basic");
	println!("日志文件路径: {}", log_path);

	// 创建 FileSink 配置
	let config = FileSinkConfig {
		enabled: true,
		path: PathBuf::from(&log_path),
		max_size: "10MB".to_string(),      // 单个文件最大 10MB
		rotation_time: "daily".to_string(), // 每天轮转
		keep_files: 10,                    // 保留 10 个历史文件
		compress: false,                   // 不启用压缩
		compression_level: 3,
		encrypt: false,
		encryption_key_env: None,
		retention_days: 7,
		max_total_size: "1GB".to_string(),
		cleanup_interval_minutes: 60,
		batch_size: 100,
		flush_interval_ms: 1000,
		masking_enabled: false,
	};

	// 创建 FileSink
	let mut sink = FileSink::new(config)?;

	// 写入各级别日志
	print_section("写入日志");
	let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
	for level in levels {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: level.to_string(),
			message: format!("这是一条 {} 级别日志", level),
			target: "file_example::basic".to_string(),
			fields: Default::default(),
			file: Some(file!().to_string()),
			line: Some(line!()),
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
	}

	sink.flush()?;

	// 读取并展示文件内容
	print_section("文件内容");
	let content = fs::read_to_string(&log_path)?;
	println!("{}", content);

	// 清理临时文件
	cleanup_files(&log_path, "inklog_example_basic")?;
	println!("\n✓ 基础文件写入完成\n");

	Ok(())
}

/// 示例2: 文件轮转
///
/// 演示文件轮转功能：
/// - 配置小文件大小（100字节）触发轮转
/// - 写入大量日志触发多次轮转
/// - 展示轮转文件命名规则
fn file_rotation() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例2: 文件轮转");

	// 生成临时文件路径
	let log_path = temp_file_path("rotation");
	println!("日志文件路径: {}", log_path);

	// 创建 FileSink 配置（小文件触发轮转）
	let config = FileSinkConfig {
		enabled: true,
		path: PathBuf::from(&log_path),
		max_size: "100".to_string(),        // 100 字节触发轮转（非常小的值）
		rotation_time: "daily".to_string(),
		keep_files: 10,                     // 保留 10 个历史文件
		compress: false,                    // 暂不压缩（下个示例演示）
		compression_level: 3,
		encrypt: false,
		encryption_key_env: None,
		retention_days: 7,
		max_total_size: "1GB".to_string(),
		cleanup_interval_minutes: 60,
		batch_size: 1,                      // 每条日志立即 flush
		flush_interval_ms: 10,              // 10ms flush 间隔
		masking_enabled: false,
	};

	// 创建 FileSink
	let mut sink = FileSink::new(config)?;

	// 写入大量日志触发轮转
	print_section("写入日志（触发轮转）");
	for i in 1..=50 {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: "INFO".to_string(),
			message: format!(
				"日志消息 #{:03} - 这是一条较长的日志消息用于触发轮转功能测试",
				i
			),
			target: "file_example::rotation".to_string(),
			fields: Default::default(),
			file: Some(file!().to_string()),
			line: Some(line!()),
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
		// 立即 flush 以触发轮转检查
		sink.flush()?;
	}

	// 展示轮转后的文件列表
	print_section("轮转文件列表");
	let log_dir = PathBuf::from(&log_path).parent().unwrap().to_path_buf();

	println!("目录: {}", log_dir.display());
	println!("\n文件列表:");

	// 列出目录中所有文件
	let mut all_files: Vec<_> = fs::read_dir(&log_dir)?
		.filter_map(|entry| entry.ok())
		.collect();

	all_files.sort_by_key(|entry| entry.file_name());

	let mut rotation_count = 0;
	for file in &all_files {
		let file_name = file.file_name().to_str().unwrap().to_string();
		if file_name.contains("inklog_example_rotation") {
			let metadata = file.metadata()?;
			let size = metadata.len();
			let marker = if file_name.contains("_2026") {
				rotation_count += 1;
				" [轮转]"
			} else {
				" [当前]"
			};
			println!("  {} ({} bytes){}", file_name, size, marker);
		}
	}

	println!("\n轮转次数: {}", rotation_count);

	// 清理临时文件
	cleanup_files(&log_path, "inklog_example_rotation")?;
	println!("\n✓ 文件轮转演示完成\n");

	Ok(())
}

/// 示例3: Zstd 压缩
///
/// 演示日志文件压缩功能：
/// - 启用 Zstd 压缩
/// - 配置小文件触发轮转
/// - 展示压缩后的 .zst 文件
fn file_compression() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("示例3: Zstd 压缩");

	// 生成临时文件路径
	let log_path = temp_file_path("compression");
	println!("日志文件路径: {}", log_path);

	// 创建 FileSink 配置（启用压缩）
	let config = FileSinkConfig {
		enabled: true,
		path: PathBuf::from(&log_path),
		max_size: "100".to_string(),        // 100 字节触发轮转
		rotation_time: "daily".to_string(),
		keep_files: 10,
		compress: true,                     // 启用压缩
		compression_level: 3,               // Zstd 压缩级别（1-21，默认 3）
		encrypt: false,
		encryption_key_env: None,
		retention_days: 7,
		max_total_size: "1GB".to_string(),
		cleanup_interval_minutes: 60,
		batch_size: 1,                      // 每条日志立即 flush
		flush_interval_ms: 10,              // 10ms flush 间隔
		masking_enabled: false,
	};

	// 创建 FileSink
	let mut sink = FileSink::new(config)?;

	// 写入大量日志触发轮转和压缩
	print_section("写入日志（触发轮转和压缩）");
	for i in 1..=50 {
		let record = LogRecord {
			timestamp: Utc::now(),
			level: "INFO".to_string(),
			message: format!(
				"日志消息 #{:03} - 重复内容可以更好地展示压缩效果（AAABBBCCCDDDEEEFFF）",
				i
			),
			target: "file_example::compression".to_string(),
			fields: Default::default(),
			file: Some(file!().to_string()),
			line: Some(line!()),
			thread_id: "main".to_string(),
		};
		sink.write(&record)?;
		// 立即 flush 以触发轮转检查
		sink.flush()?;
	}

	// 等待压缩完成
	print_section("等待压缩完成");
	std::thread::sleep(std::time::Duration::from_millis(500));

	// 展示压缩文件列表
	print_section("压缩文件列表");
	let log_dir = PathBuf::from(&log_path).parent().unwrap().to_path_buf();

	println!("目录: {}", log_dir.display());
	println!("\n文件列表:");

	// 列出目录中所有文件
	let mut all_files: Vec<_> = fs::read_dir(&log_dir)?
		.filter_map(|entry| entry.ok())
		.collect();

	all_files.sort_by_key(|entry| entry.file_name());

	let mut compression_count = 0;
	for file in &all_files {
		let file_name = file.file_name().to_str().unwrap().to_string();
		if file_name.contains("inklog_example_compression") {
			let metadata = file.metadata()?;
			let size = metadata.len();

			let marker = if file_name.ends_with(".zst") {
				compression_count += 1;
				" [压缩]"
			} else {
				" [当前]"
			};

			println!("  {} ({} bytes){}", file_name, size, marker);
		}
	}

	println!("\n压缩文件数: {}", compression_count);

	// 清理临时文件
	cleanup_files(&log_path, "inklog_example_compression")?;
	println!("\n✓ Zstd 压缩演示完成\n");

	Ok(())
}

/// 清理临时文件（包括轮转文件）
///
/// 删除指定路径相关的所有文件（包括轮转文件和压缩文件）
fn cleanup_files(log_path: &str, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
	print_section("清理临时文件");

	let log_dir = PathBuf::from(log_path).parent().unwrap().to_path_buf();

	let mut deleted_count = 0;

	for entry in fs::read_dir(&log_dir)? {
		let entry = entry?;
		let file_name = entry.file_name().to_str().unwrap().to_string();

		// 只删除当前示例相关的文件
		if file_name.contains(prefix) {
			fs::remove_file(entry.path())?;
			println!("  删除: {}", file_name);
			deleted_count += 1;
		}
	}

	if deleted_count == 0 {
		println!("  没有找到需要清理的文件");
	} else {
		println!("\n  共删除 {} 个文件", deleted_count);
	}

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog File Sink 示例 ===\n");

	// 示例1：基础文件写入
	basic_file()?;

	// 示例2：文件轮转
	file_rotation()?;

	// 示例3：Zstd 压缩
	file_compression()?;

	println!("所有示例完成！");
	Ok(())
}