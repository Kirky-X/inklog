// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 数据脱敏示例
//!
//! 演示如何使用 DataMasker 对敏感信息进行脱敏处理。
//!
//! # 功能演示
//!
//! - 基础脱敏：邮箱、电话、身份证、银行卡等
//! - JSON 数据脱敏
//! - 敏感字段检测
//! - HashMap 脱敏
//! - 日志中的脱敏应用
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin masking
//! ```

use inklog::masking::DataMasker;
use inklog_examples::common::{print_section, print_separator};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 数据脱敏示例 ===\n");

	// 创建 DataMasker 实例
	let masker = DataMasker::new();

	// 1. 基础脱敏功能
	print_separator("1. 基础脱敏功能");

	// 1.1 邮箱脱敏
	print_section("1.1 邮箱脱敏");
	let email = "user@example.com";
	let masked_email = masker.mask(email);
	println!("原始邮箱: {}", email);
	println!("脱敏后:    {}", masked_email);
	assert_eq!(masked_email, "**@**.***");

	// 1.2 电话脱敏
	print_section("1.2 电话号码脱敏");
	let phone = "13812345678";
	let masked_phone = masker.mask(phone);
	println!("原始电话: {}", phone);
	println!("脱敏后:    {}", masked_phone);
	assert_eq!(masked_phone, "***-****-****");

	// 1.3 身份证号脱敏
	print_section("1.3 身份证号脱敏");
	let id_card = "110101199001011234";
	let masked_id = masker.mask(id_card);
	println!("原始身份证: {}", id_card);
	println!("脱敏后:     {}", masked_id);
	assert_eq!(masked_id, "******1234");

	// 1.4 银行卡号脱敏
	print_section("1.4 银行卡号脱敏");
	let bank_card = "6222021234567890123";
	let masked_card = masker.mask(bank_card);
	println!("原始银行卡: {}", bank_card);
	println!("脱敏后:     {}", masked_card);

	// 2. 多种敏感数据混合脱敏
	print_separator("2. 混合敏感数据脱敏");

	let mixed_text = "用户信息：邮箱=test@example.com，电话=13812345678，备注=联系客服";
	let masked_text = masker.mask(mixed_text);
	println!("原始文本: {}", mixed_text);
	println!("脱敏后:   {}", masked_text);
	assert!(!masked_text.contains("test@example.com"));
	assert!(!masked_text.contains("13812345678"));

	// 3. JSON 数据脱敏
	print_separator("3. JSON 数据脱敏");

	let mut user_data = json!({
		"name": "张三",
		"email": "zhangsan@company.com",
		"phone": "13912345678",
		"age": 28,
		"address": "北京市朝阳区"
	});

	println!("原始 JSON:");
	println!("{}", serde_json::to_string_pretty(&user_data)?);

	masker.mask_value(&mut user_data);

	println!("\n脱敏后 JSON:");
	println!("{}", serde_json::to_string_pretty(&user_data)?);

	assert_eq!(user_data["email"], "**@**.***");
	assert_eq!(user_data["phone"], "***-****-****");
	assert_eq!(user_data["name"], "张三"); // 非敏感字段保持不变

	// 4. 嵌套 JSON 结构脱敏
	print_separator("4. 嵌套 JSON 结构脱敏");

	let mut nested_data = json!({
		"user": {
			"profile": {
				"email": "admin@company.org",
				"phone": "18655556666"
			},
			"contacts": [
				"friend@email.com",
				"13811112222"
			]
		},
		"metadata": {
			"created_at": "2026-01-01"
		}
	});

	println!("原始嵌套 JSON:");
	println!("{}", serde_json::to_string_pretty(&nested_data)?);

	masker.mask_value(&mut nested_data);

	println!("\n脱敏后嵌套 JSON:");
	println!("{}", serde_json::to_string_pretty(&nested_data)?);

	// 5. 敏感字段检测
	print_separator("5. 敏感字段检测");

	print_section("5.1 敏感字段示例");
	let sensitive_fields = vec![
		"password",
		"api_key",
		"token",
		"secret",
		"aws_key",
		"jwt",
		"credit_card",
		"cvv",
	];

	println!("以下字段被识别为敏感字段:");
	for field in &sensitive_fields {
		assert!(DataMasker::is_sensitive_field(field));
		println!("  ✓ {}", field);
	}

	print_section("5.2 非敏感字段示例");
	let normal_fields = vec!["username", "message", "content", "title", "description"];

	println!("以下字段为非敏感字段:");
	for field in &normal_fields {
		assert!(!DataMasker::is_sensitive_field(field));
		println!("  ✓ {}", field);
	}

	// 6. HashMap 脱敏
	print_separator("6. HashMap 数据脱敏");

	let mut user_map: HashMap<String, serde_json::Value> = HashMap::new();
	user_map.insert(
		"email".to_string(),
		serde_json::Value::String("user@example.com".to_string()),
	);
	user_map.insert(
		"phone".to_string(),
		serde_json::Value::String("13812345678".to_string()),
	);
	user_map.insert(
		"name".to_string(),
		serde_json::Value::String("李四".to_string()),
	);
	user_map.insert(
		"age".to_string(),
		serde_json::Value::Number(30.into()),
	);

	println!("原始 HashMap:");
	for (key, value) in &user_map {
		println!("  {}: {}", key, value);
	}

	masker.mask_hashmap(&mut user_map);

	println!("\n脱敏后 HashMap:");
	for (key, value) in &user_map {
		println!("  {}: {}", key, value);
	}

	assert_eq!(user_map["email"], "**@**.***");
	assert_eq!(user_map["phone"], "***-****-****");
	assert_eq!(user_map["name"], "李四");

	// 7. 高级敏感数据脱敏
	print_separator("7. 高级敏感数据脱敏");

	// 7.1 JWT Token 脱敏
	print_section("7.1 JWT Token 脱敏");
	let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
	let masked_jwt = masker.mask(jwt);
	println!("原始 JWT: {}...", &jwt[..50]);
	println!("脱敏后:   {}", masked_jwt);
	assert!(masked_jwt.contains("***REDACTED_JWT***"));

	// 7.2 AWS Access Key 脱敏
	print_section("7.2 AWS Access Key 脱敏");
	let aws_key = "AKIAIOSFODNN7EXAMPLE";
	let masked_aws = masker.mask(aws_key);
	println!("原始 AWS Key: {}", aws_key);
	println!("脱敏后:       {}", masked_aws);
	assert!(masked_aws.contains("***REDACTED***"));

	// 7.3 API Key 脱敏
	print_section("7.3 API Key 脱敏");
	let api_key_text = "api_key=sk-1234567890abcdefghijABCDEFGH";
	let masked_api = masker.mask(api_key_text);
	println!("原始文本: {}", api_key_text);
	println!("脱敏后:   {}", masked_api);
	assert!(masked_api.contains("***REDACTED***"));

	// 8. 实际应用场景
	print_separator("8. 实际应用场景");

	// 8.1 日志消息脱敏
	print_section("8.1 日志消息脱敏");

	let log_message = r#"用户登录成功：
  用户邮箱: user@example.com
  手机号: 13812345678
  IP地址: 192.168.1.1
  登录时间: 2026-03-19 10:30:00"#;

	let masked_log = masker.mask(log_message);
	println!("原始日志:\n{}", log_message);
	println!("\n脱敏后日志:\n{}", masked_log);

	// 8.2 API 响应数据脱敏
	print_section("8.2 API 响应数据脱敏");

	let mut api_response = json!({
		"code": 200,
		"message": "success",
		"data": {
			"user_id": 12345,
			"username": "zhang_san",
			"email": "zhangsan@example.com",
			"phone": "13912345678",
			"token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test"
		}
	});

	println!("原始 API 响应:");
	println!("{}", serde_json::to_string_pretty(&api_response)?);

	masker.mask_value(&mut api_response);

	println!("\n脱敏后 API 响应:");
	println!("{}", serde_json::to_string_pretty(&api_response)?);

	// 9. 性能提示
	print_separator("9. 性能提示");

	println!("DataMasker 性能优化要点:");
	println!("  1. 使用预编译的正则表达式");
	println!("  2. 支持批量处理 HashMap 和 JSON");
	println!("  3. 递归处理嵌套结构");
	println!("  4. 大小写不敏感的字段名检测");

	// 完成
	println!("\n✓ 所有示例演示完成");
	println!("\n按 Ctrl+C 退出...");

	inklog_examples::wait_for_ctrl_c().await?;

	Ok(())
}