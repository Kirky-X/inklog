// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 数据脱敏示例
//!
//! 演示如何配置和使用日志脱敏功能。

use inklog::masking::DataMasker;

/// 基础脱敏
pub fn basic_masking() {
	println!("=== 基础数据脱敏 ===\n");

	let masker = DataMasker::new();

	let message = "用户登录: email=test@example.com, password=secret123";
	let masked = masker.mask(message);

	println!("原始: {}", message);
	println!("脱敏: {}", masked);
}

/// 敏感字段检测
pub fn sensitive_fields() {
	println!("\n=== 敏感字段检测 ===\n");

	let sensitive = ["password", "api_key", "secret", "token", "credit_card"];
	let normal = ["username", "email", "name", "age"];

	println!("敏感字段:");
	for field in &sensitive {
		println!("  {} -> {}", field, DataMasker::is_sensitive_field(field));
	}

	println!("\n普通字段:");
	for field in &normal {
		println!("  {} -> {}", field, DataMasker::is_sensitive_field(field));
	}
}

/// 脱敏规则
pub fn masking_rules() {
	println!("\n=== 脱敏规则 ===\n");
	println!("自动脱敏的数据类型:");
	println!("  - 邮箱地址: test@example.com -> ***@***.***");
	println!("  - 电话号码: 13812345678 -> 138****5678");
	println!("  - 身份证号: 110101199001011234 -> 110101********1234");
	println!("  - 银行卡号: 6222021234567890 -> 6222************7890");
	println!("  - JWT令牌: 自动截断");
}

/// 运行所有示例
pub fn run_all() {
	basic_masking();
	sensitive_fields();
	masking_rules();
}
