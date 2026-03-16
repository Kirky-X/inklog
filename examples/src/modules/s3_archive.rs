// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! S3 归档示例
//!
//! 演示如何配置和使用 S3 归档功能。

/// S3 归档配置
pub fn s3_config() {
	println!("=== S3 归档配置 ===\n");
	println!("配置示例:");
	println!("  [s3_archive]");
	println!("  enabled = true");
	println!("  bucket = \"your-log-bucket\"");
	println!("  region = \"us-east-1\"");
	println!("  prefix = \"logs/\"");
}

/// 归档策略
pub fn archive_strategy() {
	println!("\n=== 归档策略 ===\n");
	println!("配置参数:");
	println!("  archive_after_days = 30  // 30天后归档");
	println!("  archive_format = \"json\"  // 支持 json, parquet");
}

/// 与数据库配合
pub fn database_integration() {
	println!("\n=== 与数据库配合 ===\n");
	println!("可以将数据库日志归档到 S3:");
	println!("  [database_sink]");
	println!("  archive_to_s3 = true");
	println!("  archive_after_days = 30");
	println!("  s3_bucket = \"your-log-bucket\"");
}

/// 运行所有示例
pub fn run_all() {
	s3_config();
	archive_strategy();
	database_integration();
}
