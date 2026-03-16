// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 加密功能示例
//!
//! 演示如何配置和使用日志加密功能。

/// 加密配置示例
pub fn encryption_config() {
	println!("=== 日志加密配置 ===\n");
	println!("支持的加密算法:");
	println!("  - AES-256-GCM");
	println!("\n配置步骤:");
	println!("  1. 生成加密密钥:");
	println!("     openssl rand -base64 32");
	println!("  2. 设置环境变量:");
	println!("     export INKLOG_ENCRYPTION_KEY=\"your-base64-key\"");
	println!("  3. 配置文件:");
	println!("     [file_sink]");
	println!("     encryption_key_env = \"INKLOG_ENCRYPTION_KEY\"");
}

/// 密钥管理
pub fn key_management() {
	println!("\n=== 密钥管理 ===\n");
	println!("最佳实践:");
	println!("  - 使用环境变量存储密钥");
	println!("  - 不要将密钥提交到版本控制");
	println!("  - 定期轮换密钥");
	println!("  - 使用密钥管理服务 (如 AWS KMS)");
}

/// 运行所有示例
pub fn run_all() {
	encryption_config();
	key_management();
}
