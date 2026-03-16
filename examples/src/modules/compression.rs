// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 压缩功能示例
//!
//! 演示如何配置和使用日志压缩功能。

/// 压缩算法配置
pub fn compression_config() {
	println!("=== 日志压缩配置 ===\n");
	println!("支持的压缩算法:");
	println!("  - zstd: 高压缩比，快速解压 (推荐)");
	println!("  - gzip: 广泛支持，兼容性好");
	println!("  - brotli: 最高压缩比，较慢");
	println!("\n配置示例:");
	println!("  [file_sink]");
	println!("  compress = true");
	println!("  compression_method = \"zstd\"");
}

/// 压缩级别
pub fn compression_level() {
	println!("\n=== 压缩级别 ===\n");
	println!("不同算法的压缩级别:");
	println!("  zstd: 1-22 (默认3)");
	println!("  gzip: 1-9 (默认6)");
	println!("  brotli: 0-11 (默认6)");
}

/// 运行所有示例
pub fn run_all() {
	compression_config();
	compression_level();
}
