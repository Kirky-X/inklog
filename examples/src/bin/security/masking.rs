// Copyright (c) 2026 Kirky.X🌠
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
    println!("=== inklog data masking examples ===\n");

    let masker = DataMasker::new();

    print_separator("1. Basic masking");

    print_section("1.1 Email masking");
    let email = "user@example.com";
    let masked_email = masker.mask(email);
    println!("Raw email: {}", email);
    println!("Masked:    {}", masked_email);
    assert_eq!(masked_email, "**@**.***");

    print_section("1.2 Phone number masking");
    let phone = "13812345678";
    let masked_phone = masker.mask(phone);
    println!("Raw phone: {}", phone);
    println!("Masked:    {}", masked_phone);
    assert_eq!(masked_phone, "***-****-****");

    print_section("1.3 National ID masking");
    let id_card = "110101199001011234";
    let masked_id = masker.mask(id_card);
    println!("Raw ID: {}", id_card);
    println!("Masked:     {}", masked_id);
    assert_eq!(masked_id, "******1234");

    print_section("1.4 Bank card masking");
    // 注意：此处为演示刻意打印原始卡号以对照脱敏效果，使用的是公开测试号。
    // 生产代码严禁将真实 PII 原文写入日志或 stdout。
    let bank_card = "6222021234567890123";
    let masked_card = masker.mask(bank_card);
    println!("Raw bank card: {}", bank_card);
    println!("Masked:     {}", masked_card);

    print_separator("2. Mixed sensitive data");

    let mixed_text = "user info: email=test@example.com, phone=13812345678, note=contact support";
    let masked_text = masker.mask(mixed_text);
    println!("Raw text: {}", mixed_text);
    println!("Masked:   {}", masked_text);
    assert!(!masked_text.contains("test@example.com"));
    assert!(!masked_text.contains("13812345678"));

    print_separator("3. JSON data masking");

    let mut user_data = json!({
        "name": "John Doe",
        "email": "zhangsan@company.com",
        "phone": "13912345678",
        "age": 28,
        "address": "Chaoyang District, Beijing"
    });

    println!("Raw JSON:");
    println!("{}", serde_json::to_string_pretty(&user_data)?);

    masker.mask_value(&mut user_data);

    println!("\nMasked JSON:");
    println!("{}", serde_json::to_string_pretty(&user_data)?);

    assert_eq!(user_data["email"], "**@**.***");
    assert_eq!(user_data["phone"], "***-****-****");
    assert_eq!(user_data["name"], "John Doe"); // non-sensitive fields stay unchanged

    print_separator("4. Nested JSON masking");

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

    println!("Raw nested JSON:");
    println!("{}", serde_json::to_string_pretty(&nested_data)?);

    masker.mask_value(&mut nested_data);

    println!("\nMasked nested JSON:");
    println!("{}", serde_json::to_string_pretty(&nested_data)?);

    print_separator("5. Sensitive field detection");

    print_section("5.1 Sensitive field examples");
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

    println!("These fields are detected as sensitive:");
    for field in &sensitive_fields {
        assert!(DataMasker::is_sensitive_field(field));
        println!("  ✓ {}", field);
    }

    print_section("5.2 Non-sensitive field examples");
    let normal_fields = vec!["username", "message", "content", "title", "description"];

    println!("These fields are not sensitive:");
    for field in &normal_fields {
        assert!(!DataMasker::is_sensitive_field(field));
        println!("  ✓ {}", field);
    }

    print_separator("6. HashMap data masking");

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
        serde_json::Value::String("Jane Doe".to_string()),
    );
    user_map.insert("age".to_string(), serde_json::Value::Number(30.into()));

    println!("Raw HashMap:");
    for (key, value) in &user_map {
        println!("  {}: {}", key, value);
    }

    masker.mask_hashmap(&mut user_map);

    println!("\nMasked HashMap:");
    for (key, value) in &user_map {
        println!("  {}: {}", key, value);
    }

    assert_eq!(user_map["email"], "**@**.***");
    assert_eq!(user_map["phone"], "***-****-****");
    assert_eq!(user_map["name"], "Jane Doe");

    print_separator("7. Advanced sensitive data");

    print_section("7.1 JWT token masking");
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let masked_jwt = masker.mask(jwt);
    println!("Raw JWT: {}...", &jwt[..50]);
    println!("Masked:   {}", masked_jwt);
    assert!(masked_jwt.contains("***REDACTED_JWT***"));

    print_section("7.2 AWS access key masking");
    let aws_key = "AKIAIOSFODNN7EXAMPLE";
    let masked_aws = masker.mask(aws_key);
    println!("Raw AWS key: {}", aws_key);
    println!("Masked:       {}", masked_aws);
    assert!(masked_aws.contains("***REDACTED***"));

    print_section("7.4 Credit card masking (Luhn)");
    let visa = "4111111111111111";
    let masked_visa = masker.mask(visa);
    println!("Raw Visa:  {}", visa);
    println!("Masked:     {}", masked_visa);

    let amex = "378282246310005";
    let masked_amex = masker.mask(amex);
    println!("Raw Amex:  {}", amex);
    println!("Masked:     {}", masked_amex);

    print_section("7.5 International phone masking");
    let intl_phone = "+1-202-555-0123";
    let masked_intl = masker.mask(intl_phone);
    println!("Raw international phone: {}", intl_phone);
    println!("Masked:       {}", masked_intl);

    print_section("7.6 IP/MAC address masking");
    let ipv4 = "Server at 192.168.1.100 port 8080";
    let ipv6 = "IPv6 addr: 2001:0db8:85a3:0000:0000:8a2e:0370:7334";
    let mac = "Device MAC: 00:1A:2B:3C:4D:5E";
    println!("IPv4: {} → {}", ipv4, masker.mask(ipv4));
    println!("IPv6: {} → {}", ipv6, masker.mask(ipv6));
    println!("MAC:  {} → {}", mac, masker.mask(mac));

    print_section("7.7 Third-party token masking");
    let github_token = "ghp_1234567890abcdefghij1234567890abcdef";
    let slack_token = "xoxb-1234567890-1234567890-abcDEF123";
    let stripe_key = "sk_test_1234567890abcdefghij";
    println!("GitHub: {} → {}", github_token, masker.mask(github_token));
    println!("Slack:  {} → {}", slack_token, masker.mask(slack_token));
    println!("Stripe: {} → {}", stripe_key, masker.mask(stripe_key));

    print_separator("8. Real-world scenarios");

    print_section("8.1 Masking log messages");

    let log_message = r#"User logged in successfully:
   Email: user@example.com
   Phone: 13812345678
   IP address: 192.168.1.1
   Login time: 2026-03-19 10:30:00"#;

    let masked_log = masker.mask(log_message);
    println!("Raw log:\n{}", log_message);
    println!("\nMasked log:\n{}", masked_log);

    print_section("8.2 Masking API responses");

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

    println!("Raw API response:");
    println!("{}", serde_json::to_string_pretty(&api_response)?);

    masker.mask_value(&mut api_response);

    println!("\nMasked API response:");
    println!("{}", serde_json::to_string_pretty(&api_response)?);

    print_separator("9. Custom rules");

    print_section("9.1 MaskRuleBuilder");
    let custom_rule = MaskRule::builder("employee_id")
        .pattern(r"\bEMP-\d{6}\b")
        .replacement("EMP-***")
        .priority(30)
        .build()
        .expect("Invalid pattern");
    println!(
        "Custom rule: {} (priority={})",
        custom_rule.name(),
        custom_rule.priority()
    );

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
    println!("Raw: {}", emp_text);
    println!("Masked: {}", custom_masker.mask(emp_text));

    print_section("9.3 MaskRuleRegistry");
    let mut registry = MaskRuleRegistry::with_builtins();
    println!("Built-in rule count: {}", registry.len());
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
    println!("After registering the custom rule: {}", registry.len());
    println!("Active rule count: {}", registry.active_rules().len());

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
    println!("Loaded {} custom rules from TOML", toml_rules.len());
    for rule in &toml_rules {
        println!(
            "  Rule: {} (priority={}, enabled={})",
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
    println!("Raw: {}", order_text);
    println!("Masked: {}", toml_masker.mask(order_text));

    print_separator("10. Performance notes");

    println!("DataMasker performance notes:");
    println!("  1. Uses pre-compiled LazyLock regular expressions");
    println!("  2. Handles HashMap and JSON in bulk");
    println!("  3. Recurses into nested structures");
    println!("  4. Field name detection is case-insensitive");
    println!("  5. Rules run in priority order");
    println!("  6. Rules can be managed dynamically through the registry");

    println!("\n✓ all examples completed");
    println!("\nPress Ctrl+C to exit...");

    inklog_examples::wait_for_ctrl_c().await?;

    Ok(())
}
