// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Zstd 压缩/解压缩示例（Layer 1 本地资源）
//!
//! 演示 inklog 的日志压缩能力，使用临时目录自动清理：
//! - `ZstdCompression` 压缩/解压内存数据
//! - `compress_file` 压缩整个文件（原文件会被删除）
//! - 不同压缩级别对压缩比的影响
//! - `GzipCompression` 与 `NoCompression` 策略对比
//!
//! # 运行
//! ```bash
//! cargo run --bin compression
//! ```

use inklog::sink::compression::{
	compress_data, compress_file, CompressionStrategy, GzipCompression, NoCompression,
	ZstdCompression,
};
use inklog_examples::common::{print_section, print_separator};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog Zstd 压缩示例 ===\n");

	show_in_memory_compression();
	show_file_compression().await?;
	show_compression_levels_comparison();
	show_strategy_variants();

	println!("\n✓ 所有压缩示例演示完成");
	Ok(())
}

/// 展示内存数据压缩/解压
fn show_in_memory_compression() {
	print_separator("1. 内存数据 Zstd 压缩/解压");

	print_section("1.1 ZstdCompression::default() (level=3)");
	let strategy = ZstdCompression::default();
	println!("算法名称: {}", strategy.name());
	println!("扩展名  : {}", strategy.extension());
	assert_eq!(strategy.name(), "zstd");
	assert_eq!(strategy.extension(), "zst");

	print_section("1.2 compress() 压缩字符串");
	let original = "inklog 企业级日志基础设施 — 这是一条用于测试压缩效果的日志消息。".repeat(8);
	let compressed = strategy.compress(original.as_bytes()).expect("压缩失败");
	println!("原始大小 : {} 字节", original.len());
	println!("压缩大小 : {} 字节", compressed.len());
	assert!(!compressed.is_empty());

	print_section("1.3 decompress() 解压还原");
	let decompressed = strategy.decompress(&compressed).expect("解压失败");
	println!("解压大小 : {} 字节", decompressed.len());
	assert_eq!(decompressed, original.as_bytes());
	println!("✓ 解压数据与原文一致");

	print_section("1.4 compress_data() 便捷函数");
	let compressed2 =
		compress_data(b"compress_data convenience function", 3).expect("便捷函数压缩失败");
	println!("便捷函数压缩结果大小: {} 字节", compressed2.len());
	assert!(!compressed2.is_empty());
}

/// 展示文件级压缩
async fn show_file_compression() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("2. 文件级 Zstd 压缩");

	let temp_dir = TempDir::new()?;
	let log_path = temp_dir.path().join("app.log");

	print_section("2.1 准备原始日志文件");
	let mut file = fs::File::create(&log_path)?;
	for i in 0..50 {
		writeln!(
			file,
			"2026-06-30 INFO  inklog::sink - 日志消息 #{:03} 演示文件压缩",
			i
		)?;
	}
	drop(file);
	let original_size = fs::metadata(&log_path)?.len();
	println!("原始文件: {}", log_path.display());
	println!("原始大小: {} 字节", original_size);

	print_section("2.2 compress_file() 压缩文件");
	let compressed_path = compress_file(&log_path, 3)?;
	println!("压缩文件: {}", compressed_path.display());
	println!("压缩大小: {} 字节", fs::metadata(&compressed_path)?.len());
	assert_eq!(compressed_path.extension().unwrap(), "zst");
	assert!(!log_path.exists(), "compress_file 应删除原始文件");
	assert!(compressed_path.exists(), "压缩文件应存在");

	print_section("2.3 验证压缩文件可还原");
	let compressed_bytes = fs::read(&compressed_path)?;
	// 使用 ZstdCompression::decompress 还原，避免示例直接依赖 zstd crate
	let strategy = ZstdCompression::default();
	let decompressed = strategy.decompress(&compressed_bytes)?;
	let text = String::from_utf8(decompressed)?;
	assert!(text.contains("日志消息 #000"));
	assert!(text.contains("日志消息 #049"));
	println!("✓ 解压内容包含所有 50 条日志");

	Ok(())
}

/// 展示不同压缩级别的压缩比对比
fn show_compression_levels_comparison() {
	print_separator("3. 压缩级别对压缩比的影响");

	// 重复内容占比越高，压缩比越好
	let data: Vec<u8> = (0..2000).map(|i| (i % 64) as u8).collect();
	let original_size = data.len();
	println!("原始数据: {} 字节（高度重复）\n", original_size);

	println!("{:<10}{:<15}{:<15}", "级别", "压缩大小(B)", "压缩比");
	println!("{}", "-".repeat(40));

	for level in [1, 3, 9, 19, 22] {
		let strategy = ZstdCompression::new(level);
		let compressed = strategy.compress(&data).expect("压缩失败");
		let ratio = compressed.len() as f64 / original_size as f64;
		println!("{:<10}{:<15}{:<15.4}", level, compressed.len(), ratio);
	}

	println!("\n说明: 级别越高压缩比越好，但 CPU 开销越大。生产环境推荐 3-9。");
}

/// 展示不同压缩策略对比
fn show_strategy_variants() {
	print_separator("4. 压缩策略对比（Zstd / Gzip / NoCompression）");

	let data = b"inklog compression strategy comparison test data for zstd and gzip.";

	print_section("4.1 ZstdCompression (level=3)");
	let zstd = ZstdCompression::new(3);
	let zstd_compressed = zstd.compress(data).unwrap();
	println!(
		"扩展名: {}, 压缩大小: {} 字节",
		zstd.extension(),
		zstd_compressed.len()
	);
	assert_eq!(zstd.decompress(&zstd_compressed).unwrap(), data);

	print_section("4.2 GzipCompression (level=6)");
	let gzip = GzipCompression::new(6);
	let gzip_compressed = gzip.compress(data).unwrap();
	println!(
		"扩展名: {}, 压缩大小: {} 字节",
		gzip.extension(),
		gzip_compressed.len()
	);
	assert_eq!(gzip.decompress(&gzip_compressed).unwrap(), data);

	print_section("4.3 NoCompression（透传）");
	let none = NoCompression;
	let none_compressed = none.compress(data).unwrap();
	println!(
		"扩展名: {:?}, 输出大小: {} 字节",
		none.extension(),
		none_compressed.len()
	);
	assert_eq!(none_compressed.as_slice(), data);
	assert_eq!(none.decompress(&none_compressed).unwrap(), data);
}
