// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 数据脱敏示例
//!
//! 演示如何使用 DataMasker 对敏感信息进行脱敏处理。
//!
//! # 功能演示
//!
//! - 基础脱敏：邮箱、电话、身份证、银行卡等
//! - 新增规则：信用卡(Luhn)、国际电话、IPv4/IPv6、MAC、护照、SSN、DB连接串、第三方令牌
//! - JSON 数据脱敏
//! - 敏感字段检测
//! - HashMap 脱敏
//! - 自定义规则 (MaskRuleBuilder / DataMaskerBuilder)
//! - 规则注册中心 (MaskRuleRegistry)
//! - 日志中的脱敏应用
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin masking
//! ```

use inklog::DataMasker;
use inklog::{MaskRule, MaskRuleRegistry};
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
    user_map.insert("age".to_string(), serde_json::Value::Number(30.into()));

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

    // 7.4 信用卡脱敏 (Luhn 校验)
    print_section("7.4 信用卡脱敏 (Luhn 校验)");
    let visa = "4111111111111111";
    let masked_visa = masker.mask(visa);
    println!("原始 Visa:  {}", visa);
    println!("脱敏后:     {}", masked_visa);

    let amex = "378282246310005";
    let masked_amex = masker.mask(amex);
    println!("原始 Amex:  {}", amex);
    println!("脱敏后:     {}", masked_amex);

    // 7.5 国际电话号码脱敏
    print_section("7.5 国际电话号码脱敏");
    let intl_phone = "+1-202-555-0123";
    let masked_intl = masker.mask(intl_phone);
    println!("原始国际电话: {}", intl_phone);
    println!("脱敏后:       {}", masked_intl);

    // 7.6 IP 地址和 MAC 地址脱敏
    print_section("7.6 IP/MAC 地址脱敏");
    let ipv4 = "Server at 192.168.1.100 port 8080";
    let ipv6 = "IPv6 addr: 2001:0db8:85a3:0000:0000:8a2e:0370:7334";
    let mac = "Device MAC: 00:1A:2B:3C:4D:5E";
    println!("IPv4: {} → {}", ipv4, masker.mask(ipv4));
    println!("IPv6: {} → {}", ipv6, masker.mask(ipv6));
    println!("MAC:  {} → {}", mac, masker.mask(mac));

    // 7.7 第三方令牌脱敏
    print_section("7.7 第三方令牌脱敏");
    let github_token = "ghp_1234567890abcdefghij1234567890abcdef";
    let slack_token = "xoxb-1234567890-1234567890-abcDEF123";
    let stripe_key = "sk_test_1234567890abcdefghij";
    println!("GitHub: {} → {}", github_token, masker.mask(github_token));
    println!("Slack:  {} → {}", slack_token, masker.mask(slack_token));
    println!("Stripe: {} → {}", stripe_key, masker.mask(stripe_key));

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

    // 9. 自定义规则
    print_separator("9. 自定义规则");

    // 9.1 MaskRuleBuilder 创建自定义规则
    print_section("9.1 MaskRuleBuilder");
    let custom_rule = MaskRule::builder("employee_id")
        .pattern(r"\bEMP-\d{6}\b")
        .replacement("EMP-***")
        .priority(30)
        .build()
        .expect("Invalid pattern");
    println!(
        "自定义规则: {} (priority={})",
        custom_rule.name(),
        custom_rule.priority()
    );

    // 9.2 DataMaskerBuilder 组装脱敏器
    print_section("9.2 DataMaskerBuilder");
    let custom_masker = DataMasker::builder()
        .add_rule(
            MaskRule::builder("employee_id")
                .pattern(r"\bEMP-\d{6}\b")
                .replacement("EMP-***")
                .priority(30)
                .build()
                .unwrap(),
        )
        .disable_builtin("bank_card")
        .build();
    let emp_text = "Employee EMP-123456 logged in";
    println!("原始: {}", emp_text);
    println!("脱敏: {}", custom_masker.mask(emp_text));

    // 9.3 MaskRuleRegistry 规则注册中心
    print_section("9.3 MaskRuleRegistry");
    let mut registry = MaskRuleRegistry::with_builtins();
    println!("内置规则数: {}", registry.len());
    registry
        .register(
            MaskRule::builder("project_code")
                .pattern(r"PRJ-[A-Z]{3}-\d{4}")
                .replacement("PRJ-***")
                .priority(35)
                .build()
                .unwrap(),
        )
        .unwrap();
    println!("注册自定义规则后: {}", registry.len());
    println!("活跃规则数: {}", registry.active_rules().len());

    // 9.4 TOML 配置加载
    print_section("9.4 MaskRuleRegistry::load_from_toml");
    let toml_config = r#"
[[masking_rules]]
name = "order_id"
pattern = "\\bORD-\\d{8}\\b"
replacement = "ORD-***"
priority = 60
enabled = true

[[masking_rules]]
name = "trace_id"
pattern = "\\bTRACE-[A-F0-9]{16}\\b"
replacement = "***TRACE***"
"#;
    let toml_rules = MaskRuleRegistry::load_from_toml(toml_config).expect("Invalid TOML");
    println!("从 TOML 加载了 {} 条自定义规则", toml_rules.len());
    for rule in &toml_rules {
        println!(
            "  规则: {} (priority={}, enabled={})",
            rule.name(),
            rule.priority(),
            rule.is_enabled()
        );
    }
    // 将 TOML 规则注册到注册中心并构建 DataMasker
    let mut toml_registry = MaskRuleRegistry::with_builtins();
    for rule in toml_rules {
        let _ = toml_registry.register(rule);
    }
    let toml_masker = DataMasker::builder().with_registry(toml_registry).build();
    let order_text = "Order ORD-20260803 processed, trace: TRACE-0123456789ABCDEF";
    println!("原始: {}", order_text);
    println!("脱敏: {}", toml_masker.mask(order_text));

    // 10. 性能提示
    print_separator("10. 性能提示");

    println!("DataMasker 性能优化要点:");
    println!("  1. 使用预编译的 LazyLock 正则表达式");
    println!("  2. 支持批量处理 HashMap 和 JSON");
    println!("  3. 递归处理嵌套结构");
    println!("  4. 大小写不敏感的字段名检测");
    println!("  5. 规则按优先级排序执行");
    println!("  6. 支持通过 Registry 动态管理规则");

    // 完成
    println!("\n✓ 所有示例演示完成");
    println!("\n按 Ctrl+C 退出...");

    inklog_examples::wait_for_ctrl_c().await?;

    Ok(())
}
