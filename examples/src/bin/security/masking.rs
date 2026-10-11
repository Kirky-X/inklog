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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-masking-inklog-data-masking-examples"));

    let masker = DataMasker::new();

    print_separator(&tr("example-masking-1-basic-masking"));

    print_section(&tr("example-masking-1-1-email-masking"));
    let email = "user@example.com";
    let masked_email = masker.mask(email);
    println!(
        "{}",
        tr_args("example-masking-raw-email", &[("email", &email)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked", &[("masked_email", &masked_email)])
    );
    assert_eq!(masked_email, "**@**.***");

    print_section(&tr("example-masking-1-2-phone-number-masking"));
    let phone = "13812345678";
    let masked_phone = masker.mask(phone);
    println!(
        "{}",
        tr_args("example-masking-raw-phone", &[("phone", &phone)])
    );
    println!(
        "{}",
        tr_args(
            "example-masking-masked-2",
            &[("masked_phone", &masked_phone)]
        )
    );
    assert_eq!(masked_phone, "***-****-****");

    print_section(&tr("example-masking-1-3-national-id-masking"));
    let id_card = "110101199001011234";
    let masked_id = masker.mask(id_card);
    println!(
        "{}",
        tr_args("example-masking-raw-id", &[("id_card", &id_card)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-3", &[("masked_id", &masked_id)])
    );
    assert_eq!(masked_id, "******1234");

    print_section(&tr("example-masking-1-4-bank-card-masking"));
    // 注意：此处为演示刻意打印原始卡号以对照脱敏效果，使用的是公开测试号。
    // 生产代码严禁将真实 PII 原文写入日志或 stdout。
    let bank_card = "6222021234567890123";
    let masked_card = masker.mask(bank_card);
    println!(
        "{}",
        tr_args(
            "example-masking-raw-bank-card",
            &[("bank_card", &bank_card)]
        )
    );
    println!(
        "{}",
        tr_args("example-masking-masked-4", &[("masked_card", &masked_card)])
    );

    print_separator(&tr("example-masking-2-mixed-sensitive-data"));

    let mixed_text = "user info: email=test@example.com, phone=13812345678, note=contact support";
    let masked_text = masker.mask(mixed_text);
    println!(
        "{}",
        tr_args("example-masking-raw-text", &[("mixed_text", &mixed_text)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-5", &[("masked_text", &masked_text)])
    );
    assert!(!masked_text.contains("test@example.com"));
    assert!(!masked_text.contains("13812345678"));

    print_separator(&tr("example-masking-3-json-data-masking"));

    let mut user_data = json!({
        "name": "John Doe",
        "email": "zhangsan@company.com",
        "phone": "13912345678",
        "age": 28,
        "address": "Chaoyang District, Beijing"
    });

    println!("{}", tr("example-masking-raw-json"));
    println!("{}", serde_json::to_string_pretty(&user_data)?);

    masker.mask_value(&mut user_data);

    println!("\n{}", tr("example-masking-masked-json"));
    println!("{}", serde_json::to_string_pretty(&user_data)?);

    assert_eq!(user_data["email"], "**@**.***");
    assert_eq!(user_data["phone"], "***-****-****");
    assert_eq!(user_data["name"], "John Doe"); // non-sensitive fields stay unchanged

    print_separator(&tr("example-masking-4-nested-json-masking"));

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

    println!("{}", tr("example-masking-raw-nested-json"));
    println!("{}", serde_json::to_string_pretty(&nested_data)?);

    masker.mask_value(&mut nested_data);

    println!("\n{}", tr("example-masking-masked-nested-json"));
    println!("{}", serde_json::to_string_pretty(&nested_data)?);

    print_separator(&tr("example-masking-5-sensitive-field-detection"));

    print_section(&tr("example-masking-5-1-sensitive-field-examples"));
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

    println!("{}", tr("example-masking-these-fields-are-detected-as"));
    for field in &sensitive_fields {
        assert!(DataMasker::is_sensitive_field(field));
        println!("  ✓ {}", field);
    }

    print_section(&tr("example-masking-5-2-non-sensitive-field-examples"));
    let normal_fields = vec!["username", "message", "content", "title", "description"];

    println!("{}", tr("example-masking-these-fields-are-not-sensitive"));
    for field in &normal_fields {
        assert!(!DataMasker::is_sensitive_field(field));
        println!("  ✓ {}", field);
    }

    print_separator(&tr("example-masking-6-hashmap-data-masking"));

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

    println!("{}", tr("example-masking-raw-hashmap"));
    for (key, value) in &user_map {
        println!("  {}: {}", key, value);
    }

    masker.mask_hashmap(&mut user_map);

    println!("\n{}", tr("example-masking-masked-hashmap"));
    for (key, value) in &user_map {
        println!("  {}: {}", key, value);
    }

    assert_eq!(user_map["email"], "**@**.***");
    assert_eq!(user_map["phone"], "***-****-****");
    assert_eq!(user_map["name"], "Jane Doe");

    print_separator(&tr("example-masking-7-advanced-sensitive-data"));

    print_section(&tr("example-masking-7-1-jwt-token-masking"));
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let masked_jwt = masker.mask(jwt);
    println!(
        "{}",
        tr_args("example-masking-raw-jwt", &[("value", &(&jwt[..50]))])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-6", &[("masked_jwt", &masked_jwt)])
    );
    assert!(masked_jwt.contains("***REDACTED_JWT***"));

    print_section(&tr("example-masking-7-2-aws-access-key-masking"));
    let aws_key = "AKIAIOSFODNN7EXAMPLE";
    let masked_aws = masker.mask(aws_key);
    println!(
        "{}",
        tr_args("example-masking-raw-aws-key", &[("aws_key", &aws_key)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-7", &[("masked_aws", &masked_aws)])
    );
    assert!(masked_aws.contains("***REDACTED***"));

    print_section(&tr("example-masking-7-4-credit-card-masking-luhn"));
    let visa = "4111111111111111";
    let masked_visa = masker.mask(visa);
    println!(
        "{}",
        tr_args("example-masking-raw-visa", &[("visa", &visa)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-8", &[("masked_visa", &masked_visa)])
    );

    let amex = "378282246310005";
    let masked_amex = masker.mask(amex);
    println!(
        "{}",
        tr_args("example-masking-raw-amex", &[("amex", &amex)])
    );
    println!(
        "{}",
        tr_args("example-masking-masked-9", &[("masked_amex", &masked_amex)])
    );

    print_section(&tr("example-masking-7-5-international-phone-masking"));
    let intl_phone = "+1-202-555-0123";
    let masked_intl = masker.mask(intl_phone);
    println!(
        "{}",
        tr_args(
            "example-masking-raw-international-phone",
            &[("intl_phone", &intl_phone)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-masked-10",
            &[("masked_intl", &masked_intl)]
        )
    );

    print_section(&tr("example-masking-7-6-ip-mac-address-masking"));
    let ipv4 = "Server at 192.168.1.100 port 8080";
    let ipv6 = "IPv6 addr: 2001:0db8:85a3:0000:0000:8a2e:0370:7334";
    let mac = "Device MAC: 00:1A:2B:3C:4D:5E";
    println!(
        "{}",
        tr_args(
            "example-masking-ipv4",
            &[("ipv4", &ipv4), ("value", &(masker.mask(ipv4)))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-ipv6",
            &[("ipv6", &ipv6), ("value", &(masker.mask(ipv6)))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-mac",
            &[("mac", &mac), ("value", &(masker.mask(mac)))]
        )
    );

    print_section(&tr("example-masking-7-7-third-party-token-masking"));
    let github_token = "ghp_1234567890abcdefghij1234567890abcdef";
    let slack_token = "xoxb-1234567890-1234567890-abcDEF123";
    let stripe_key = "sk_test_1234567890abcdefghij";
    println!(
        "{}",
        tr_args(
            "example-masking-github",
            &[
                ("github_token", &github_token),
                ("value", &(masker.mask(github_token)))
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-slack",
            &[
                ("slack_token", &slack_token),
                ("value", &(masker.mask(slack_token)))
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-stripe",
            &[
                ("stripe_key", &stripe_key),
                ("value", &(masker.mask(stripe_key)))
            ]
        )
    );

    print_separator(&tr("example-masking-8-real-world-scenarios"));

    print_section(&tr("example-masking-8-1-masking-log-messages"));

    let log_message = r#"User logged in successfully:
   Email: user@example.com
   Phone: 13812345678
   IP address: 192.168.1.1
   Login time: 2026-03-19 10:30:00"#;

    let masked_log = masker.mask(log_message);
    println!(
        "{}",
        tr_args("example-masking-raw-log", &[("log_message", &log_message)])
    );
    println!(
        "\n{}",
        tr_args("example-masking-masked-log", &[("masked_log", &masked_log)])
    );

    print_section(&tr("example-masking-8-2-masking-api-responses"));

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

    println!("{}", tr("example-masking-raw-api-response"));
    println!("{}", serde_json::to_string_pretty(&api_response)?);

    masker.mask_value(&mut api_response);

    println!("\n{}", tr("example-masking-masked-api-response"));
    println!("{}", serde_json::to_string_pretty(&api_response)?);

    print_separator(&tr("example-masking-9-custom-rules"));

    print_section(&tr("example-masking-9-1-maskrulebuilder"));
    let custom_rule = MaskRule::builder("employee_id")
        .pattern(r"\bEMP-\d{6}\b")
        .replacement("EMP-***")
        .priority(30)
        .build()
        .expect("Invalid pattern");
    println!(
        "{}",
        tr_args(
            "example-masking-custom-rule-priority",
            &[
                ("value", &(custom_rule.name())),
                ("value2", &(custom_rule.priority()))
            ]
        )
    );

    print_section(&tr("example-masking-9-2-datamaskerbuilder"));
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
    println!(
        "{}",
        tr_args("example-masking-raw", &[("emp_text", &emp_text)])
    );
    println!(
        "{}",
        tr_args(
            "example-masking-masked-11",
            &[("value", &(custom_masker.mask(emp_text)))]
        )
    );

    print_section(&tr("example-masking-9-3-maskruleregistry"));
    let mut registry = MaskRuleRegistry::with_builtins();
    println!(
        "{}",
        tr_args(
            "example-masking-built-in-rule-count",
            &[("value", &(registry.len()))]
        )
    );
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
    println!(
        "{}",
        tr_args(
            "example-masking-after-registering-the-custom",
            &[("value", &(registry.len()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-masking-active-rule-count",
            &[("value", &(registry.active_rules().len()))]
        )
    );

    print_section(&tr("example-masking-9-4-maskruleregistry-load-from"));
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
    println!(
        "{}",
        tr_args(
            "example-masking-loaded-custom-rules-from-toml",
            &[("value", &(toml_rules.len()))]
        )
    );
    for rule in &toml_rules {
        println!(
            "  {}",
            tr_args(
                "example-masking-rule-priority-enabled",
                &[
                    ("value", &(rule.name())),
                    ("value2", &(rule.priority())),
                    ("value3", &(rule.is_enabled()))
                ]
            )
        );
    }
    // 将 TOML 规则注册到注册中心并构建 DataMasker
    let mut toml_registry = MaskRuleRegistry::with_builtins();
    for rule in toml_rules {
        let _ = toml_registry.register(rule);
    }
    let toml_masker = DataMasker::builder().with_registry(toml_registry).build();
    let order_text = "Order ORD-20260803 processed, trace: TRACE-0123456789ABCDEF";
    println!(
        "{}",
        tr_args("example-masking-raw-2", &[("order_text", &order_text)])
    );
    println!(
        "{}",
        tr_args(
            "example-masking-masked-11",
            &[("value", &(toml_masker.mask(order_text)))]
        )
    );

    print_separator(&tr("example-masking-10-performance-notes"));

    println!("{}", tr("example-masking-datamasker-performance-notes"));
    println!("  {}", tr("example-masking-1-uses-pre-compiled-lazylock"));
    println!("  {}", tr("example-masking-2-handles-hashmap-and-json-in"));
    println!("  {}", tr("example-masking-3-recurses-into-nested"));
    println!("  {}", tr("example-masking-4-field-name-detection-is-case"));
    println!("  {}", tr("example-masking-5-rules-run-in-priority-order"));
    println!("  {}", tr("example-masking-6-rules-can-be-managed"));

    println!("\n{}", tr("example-masking-all-examples-completed"));
    println!("\n{}", tr("example-masking-press-ctrl-c-to-exit"));

    inklog_examples::wait_for_ctrl_c().await?;

    Ok(())
}
