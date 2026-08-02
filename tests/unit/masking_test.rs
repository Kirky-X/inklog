// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 数据掩码功能测试
// 测试 PII 自动检测与脱敏功能，确保合规性

#[cfg(test)]
mod masking_test {
    use inklog::masking::DataMasker;
    use inklog::{DataMaskerBuilder, MaskRule, MaskRuleBuilder, MaskRuleRegistry};
    use serde_json::json;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    // === 邮箱脱敏测试 ===

    #[test]
    fn test_email_masking() {
        let masker = DataMasker::new();

        let test_cases = vec![
            ("test@example.com", "**@**.***"),
            ("user.name@company.co.uk", "**@**.***"),
            ("admin@localhost", "**@**.***"),
            ("user+tag@example.org", "**@**.***"),
            ("user_name@test.io", "**@**.***"),
            ("a@b.c", "**@**.***"),
        ];

        for (input, expected) in test_cases {
            let result = masker.mask(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_email_in_message() {
        let masker = DataMasker::new();

        let message = "Contact user at test@example.com for verification";
        let result = masker.mask(message);

        assert!(!result.contains("test@example.com"));
        assert!(result.contains("**@**.***"));
    }

    // === 电话脱敏测试 ===

    #[test]
    fn test_phone_masking() {
        let masker = DataMasker::new();

        let test_cases = vec![
            ("13812345678", "***-****-****"),
            ("15987654321", "***-****-****"),
            ("18655556666", "***-****-****"),
        ];

        for (input, expected) in test_cases {
            let result = masker.mask(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_phone_in_message() {
        let masker = DataMasker::new();

        let message = "Contact: 13812345678 for support";
        let result = masker.mask(message);

        assert!(!result.contains("13812345678"));
        assert!(result.contains("***-****-****"));
    }

    // === 身份证脱敏测试 ===

    #[test]
    fn test_id_card_masking() {
        let masker = DataMasker::new();

        let test_cases = vec![
            ("110101199001011234", "******1234"),
            ("31011519880530218X", "******218X"),
        ];

        for (input, expected) in test_cases {
            let result = masker.mask(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    // === 银行卡脱敏测试 ===

    #[test]
    fn test_bank_card_masking() {
        let masker = DataMasker::new();

        let test_cases = vec![
            ("6222021234567890123", "****-****-****-0123"),
            ("4567890123456789", "****-****-****-6789"),
        ];

        for (input, expected) in test_cases {
            let result = masker.mask(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    // === JWT 令牌脱敏测试 ===

    #[test]
    fn test_jwt_masking() {
        let masker = DataMasker::new();

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = masker.mask(jwt);

        assert!(!result.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(result.contains("***REDACTED_JWT***"));
    }

    // === AWS 密钥脱敏测试 ===

    #[test]
    fn test_aws_key_masking() {
        let masker = DataMasker::new();

        let aws_keys = vec![
            "AKIAIOSFODNN7EXAMPLE",
            "AKIA1234567890ABCDEF",
            "ABIA1234567890ABCDEF",
            "ASIA1234567890ABCDEF",
        ];

        for key in aws_keys {
            let result = masker.mask(key);
            assert!(result.contains("***REDACTED***"), "Failed for: {}", key);
            assert!(!result.contains(key), "Failed for: {}", key);
        }
    }

    // === API 密钥脱敏测试 ===

    #[test]
    fn test_api_key_masking() {
        let masker = DataMasker::new();

        let messages = vec![
            "api_key=sk-1234567890abcdefghijABCDEFGH",
            "api_key=sk_test_1234567890abcdef",
            "API_KEY=pk_live_1234567890abcdefghij",
        ];

        for message in messages {
            let result = masker.mask(&message);
            assert!(result.contains("***REDACTED***"), "Failed for: {}", message);
        }
    }

    // === 敏感字段名检测测试 ===

    #[test]
    fn test_sensitive_field_detection() {
        let sensitive_fields = vec![
            "password",
            "PASSWORD",
            "Password",
            "api_key",
            "apiKey",
            "API_KEY",
            "api-secret",
            "token",
            "TOKEN",
            "jwt_token",
            "bearer_token",
            "secret",
            "SECRET",
            "credential",
            "aws_secret",
            "aws_key",
            "credit_card",
            "card_number",
            "cvv",
            "ssn",
        ];

        for field in sensitive_fields {
            assert!(
                DataMasker::is_sensitive_field(field),
                "Failed to detect sensitive field: {}",
                field
            );
        }
    }

    #[test]
    fn test_non_sensitive_field_detection() {
        let non_sensitive_fields = vec![
            "username",
            "message",
            "content",
            "title",
            "email", // 邮箱地址是敏感值，但字段名本身不是
            "phone", // 电话号码是敏感值，但字段名本身不是
            "name",
            "timestamp",
            "level",
            "target",
        ];

        for field in non_sensitive_fields {
            assert!(
                !DataMasker::is_sensitive_field(field),
                "Incorrectly detected non-sensitive field as sensitive: {}",
                field
            );
        }
    }

    // === JSON 值脱敏测试 ===

    #[test]
    fn test_json_value_masking() {
        let masker = DataMasker::new();

        let mut value = json!({
            "email": "user@example.com",
            "phone": "13812345678",
            "name": "John Doe"
        });

        masker.mask_value(&mut value);

        assert_eq!(value["email"], "**@**.***");
        assert_eq!(value["phone"], "***-****-****");
        assert_eq!(value["name"], "John Doe");
    }

    #[test]
    fn test_nested_json_masking() {
        let masker = DataMasker::new();

        let mut value = json!({
            "user": {
                "email": "admin@company.org",
                "contacts": [
                    {"email": "test@email.com", "phone": "13811112222"}
                ]
            }
        });

        masker.mask_value(&mut value);

        let user = &value["user"];
        assert_eq!(user["email"], "**@**.***");

        let contacts = user["contacts"].as_array().unwrap();
        assert_eq!(contacts[0]["email"], "**@**.***");
        assert_eq!(contacts[0]["phone"], "***-****-****");
    }

    #[test]
    fn test_array_masking() {
        let masker = DataMasker::new();

        let mut value = json!([
            {"email": "a@b.com", "name": "A"},
            {"email": "c@d.com", "name": "B"}
        ]);

        masker.mask_value(&mut value);

        let arr = value.as_array().unwrap();
        assert_eq!(arr[0]["email"], "**@**.***");
        assert_eq!(arr[1]["email"], "**@**.***");
    }

    // === HashMap 脱敏测试 ===

    #[test]
    fn test_hashmap_masking() {
        let masker = DataMasker::new();
        let mut map: HashMap<String, serde_json::Value> = HashMap::new();

        map.insert("email".to_string(), json!("user@example.com"));
        map.insert("password".to_string(), json!("secret123"));
        map.insert("name".to_string(), json!("John"));

        masker.mask_hashmap(&mut map);

        assert_eq!(map["email"], "**@**.***");
        assert_eq!(map["name"], "John");
    }

    // === 组合脱敏测试 ===

    #[test]
    fn test_multiple_sensitive_data_types() {
        let masker = DataMasker::new();

        let message = "User email: test@example.com, phone: 13812345678, SSN: 110101199001011234, card: 6222021234567890123";
        let result = masker.mask(message);

        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("13812345678"));
        // 18-digit ID number is matched by bank_card rule (any 8+ digit sequence)
        // and formatted as ****-****-****-XXXX
        assert!(
            !result.contains("110101199001011234"),
            "ID should be masked"
        );
        assert!(
            !result.contains("6222021234567890123"),
            "Card should be masked"
        );

        // 验证脱敏标记存在
        assert!(result.contains("**@**.***") || result.contains("***REDACTED***"));
    }

    // === 性能测试 ===

    #[test]
    fn test_masking_performance() {
        use std::time::Instant;

        let masker = DataMasker::new();

        // 测试大量脱敏操作的性能
        let iterations = 10000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = masker.mask("Email: test@example.com, Phone: 13812345678");
        }

        let elapsed = start.elapsed();

        // 10000 次脱敏应该在合理时间内完成
        assert!(elapsed.as_secs() < 5, "Masking too slow: {:?}", elapsed);
    }

    // === 边界条件测试 ===

    #[test]
    fn test_empty_string_masking() {
        let masker = DataMasker::new();

        let result = masker.mask("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_sensitive_data() {
        let masker = DataMasker::new();

        let message = "This is a normal log message without any sensitive data";
        let result = masker.mask(message);

        assert_eq!(result, message);
    }

    #[test]
    fn test_only_sensitive_data() {
        let masker = DataMasker::new();

        // 只有邮箱
        let result = masker.mask("test@example.com");
        assert_eq!(result, "**@**.***");

        // 只有电话
        let result = masker.mask("13812345678");
        assert_eq!(result, "***-****-****");
    }

    #[test]
    fn test_unicode_sensitive_data() {
        let masker = DataMasker::new();

        // 包含 Unicode 字符的消息
        let message = "用户邮箱: test@example.com, 电话: 13812345678";
        let result = masker.mask(message);

        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("13812345678"));
        assert!(result.contains("用户邮箱:"));
    }

    #[test]
    fn test_special_characters_in_message() {
        let masker = DataMasker::new();

        // 包含特殊字符的消息
        let message = "Contact: test@example.com (email) or 13812345678 (phone)!";
        let result = masker.mask(message);

        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("13812345678"));
    }

    // === 国际电话号码脱敏测试 (T003) ===

    #[test]
    fn test_international_phone_masking() {
        let masker = DataMasker::new();

        // E.164 format
        let result = masker.mask("+1-202-555-0123");
        assert!(!result.contains("202-555"), "Should mask middle digits");
        assert!(result.contains("+1"), "Should preserve country code");
        assert!(result.contains("23"), "Should preserve last 2 digits");
    }

    #[test]
    fn test_international_phone_in_json() {
        let masker = DataMasker::new();
        let mut value = json!("Call +44 20 7946 0958 now");
        masker.mask_value(&mut value);
        let s = value.as_str().unwrap();
        assert!(!s.contains("7946"), "Should mask middle of intl phone");
    }

    // === 信用卡脱敏测试 (T004) ===

    #[test]
    fn test_credit_card_visa() {
        let masker = DataMasker::new();
        // Visa with valid Luhn
        let result = masker.mask("4111111111111111");
        assert!(result.contains("****-****-****-1111"), "Visa: {}", result);
    }

    #[test]
    fn test_credit_card_amex() {
        let masker = DataMasker::new();
        // Amex with valid Luhn
        let result = masker.mask("378282246310005");
        assert!(result.contains("****-******-0005"), "Amex: {}", result);
    }

    #[test]
    fn test_credit_card_invalid_luhn() {
        let masker = DataMasker::new();
        // Invalid Luhn checksum (sum=32, not divisible by 10)
        let result = masker.mask("4111111111111114");
        assert!(
            result.contains("4111111111111114"),
            "Invalid Luhn should be unchanged: {}",
            result
        );
    }

    #[test]
    fn test_credit_card_in_message() {
        let masker = DataMasker::new();
        let result = masker.mask("Card: 4111111111111111 charged");
        assert!(!result.contains("4111111111111111"));
        assert!(result.contains("****-****-****-1111"));
    }

    // === IPv4 脱敏测试 (T005) ===

    #[test]
    fn test_ipv4_masking() {
        let masker = DataMasker::new();
        let result = masker.mask("Server at 192.168.1.100");
        assert!(result.contains("***.***.***.100"), "IPv4: {}", result);
        assert!(!result.contains("192.168.1.100"));
    }

    #[test]
    fn test_ipv4_in_json() {
        let masker = DataMasker::new();
        let mut value = json!({"ip": "10.0.0.1"});
        masker.mask_value(&mut value);
        assert_eq!(value["ip"], "***.***.***.1");
    }

    // === IPv6 脱敏测试 (T005) ===

    #[test]
    fn test_ipv6_masking() {
        let masker = DataMasker::new();
        let result = masker.mask("2001:0db8:85a3:0000:0000:8a2e:0370:7334");
        assert!(
            result.contains("7334"),
            "Should preserve last group: {}",
            result
        );
        assert!(!result.contains("2001:0db8:85a3"));
    }

    // === MAC 地址脱敏测试 (T006) ===

    #[test]
    fn test_mac_address_colon() {
        let masker = DataMasker::new();
        let result = masker.mask("Device 00:1A:2B:3C:4D:5E connected");
        assert!(
            result.contains("00:"),
            "Should preserve first octet: {}",
            result
        );
        assert!(
            result.contains(":5E"),
            "Should preserve last octet: {}",
            result
        );
        assert!(result.contains("**"), "Should mask middle octets");
    }

    #[test]
    fn test_mac_address_dash() {
        let masker = DataMasker::new();
        let result = masker.mask("MAC: AA-BB-CC-DD-EE-FF");
        assert!(
            result.contains("AA-"),
            "Should preserve first octet: {}",
            result
        );
        assert!(
            result.contains("-FF"),
            "Should preserve last octet: {}",
            result
        );
    }

    // === 护照号脱敏测试 (T007) ===

    #[test]
    fn test_passport_masking() {
        let masker = DataMasker::new();
        let result = masker.mask("Passport: E12345678");
        assert!(result.contains("E******78"), "Passport: {}", result);
    }

    #[test]
    fn test_passport_g_prefix() {
        let masker = DataMasker::new();
        let result = masker.mask("G98765432");
        assert!(result.contains("G******32"), "Passport G: {}", result);
    }

    // === SSN 脱敏测试 (T008) ===

    #[test]
    fn test_ssn_masking() {
        let masker = DataMasker::new();
        let result = masker.mask("SSN: 123-45-6789");
        assert!(result.contains("***-**-6789"), "SSN: {}", result);
        assert!(!result.contains("123-45-6789"));
    }

    // === 数据库连接串脱敏测试 (T009) ===

    #[test]
    fn test_db_connection_masking() {
        let masker = DataMasker::new();
        let result = masker.mask("postgres://user:secret@localhost/db");
        assert!(
            result.contains("postgres://user:"),
            "Should preserve scheme+user: {}",
            result
        );
        assert!(result.contains("***"), "Should mask password: {}", result);
        assert!(
            result.contains("@localhost"),
            "Should preserve host: {}",
            result
        );
        assert!(!result.contains("secret"));
    }

    // === GitHub Token 脱敏测试 (T010) ===

    #[test]
    fn test_github_token_masking() {
        let masker = DataMasker::new();
        let token = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let result = masker.mask(token);
        assert!(
            result.contains("***REDACTED_GITHUB***"),
            "GitHub: {}",
            result
        );
        assert!(!result.contains(token));
    }

    // === Slack Token 脱敏测试 (T010) ===

    #[test]
    fn test_slack_token_masking() {
        let masker = DataMasker::new();
        let token = "xoxb-1234567890-1234567890123-ABCDEF";
        let result = masker.mask(token);
        assert!(result.contains("***REDACTED_SLACK***"), "Slack: {}", result);
    }

    // === Stripe Key 脱敏测试 (T010) ===

    #[test]
    fn test_stripe_key_masking() {
        let masker = DataMasker::new();
        let key = "sk_test_abcdefghijklmnopqrstuvwxyz1234";
        let result = masker.mask(key);
        assert!(
            result.contains("***REDACTED_STRIPE***"),
            "Stripe: {}",
            result
        );
    }

    // === Google API Key 脱敏测试 (T010) ===

    #[test]
    fn test_google_api_key_masking() {
        let masker = DataMasker::new();
        let key = "AIzaSyTESTKEY1234567890abcdefghijklmnopqrs";
        let result = masker.mask(key);
        assert!(
            result.contains("***REDACTED_GOOGLE***"),
            "Google: {}",
            result
        );
    }

    // === Private Key 脱敏测试 (T010) ===

    #[test]
    fn test_private_key_masking() {
        let masker = DataMasker::new();
        let key = "-----BEGIN RSA PRIVATE KEY-----\nMIIBogIBAAJ...\n-----END RSA PRIVATE KEY-----";
        let result = masker.mask(key);
        assert!(
            result.contains("***REDACTED_PRIVATE_KEY***"),
            "Private key: {}",
            result
        );
    }

    // === MaskRuleRegistry 测试 ===

    #[test]
    fn test_registry_with_builtins() {
        let registry = MaskRuleRegistry::with_builtins();
        assert!(
            registry.len() >= 21,
            "Should have at least 21 built-in rules"
        );
        let active = registry.active_rules();
        assert_eq!(
            active.len(),
            registry.len(),
            "All builtins should be enabled"
        );
    }

    #[test]
    fn test_registry_register_and_remove() {
        let mut registry = MaskRuleRegistry::default();
        let rule = MaskRule::builder("custom")
            .pattern(r"\bSECRET-\d+\b")
            .replacement("***SECRET***")
            .build()
            .unwrap();
        assert!(registry.register(rule).is_ok());
        assert_eq!(registry.len(), 1);

        // Duplicate
        let dup = MaskRule::builder("custom").pattern(r"\d+").build().unwrap();
        assert!(registry.register(dup).is_err());

        // Remove
        let removed = registry.remove("custom");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_enable_disable() {
        let mut registry = MaskRuleRegistry::with_builtins();
        let total = registry.len();

        registry.set_enabled("email", false);
        assert_eq!(registry.active_rules().len(), total - 1);

        registry.set_enabled("email", true);
        assert_eq!(registry.active_rules().len(), total);
    }

    #[test]
    fn test_registry_priority_ordering() {
        let registry = MaskRuleRegistry::with_builtins();
        let active = registry.active_rules();
        for w in active.windows(2) {
            assert!(
                w[0].priority() <= w[1].priority(),
                "Rules should be sorted by priority"
            );
        }
    }

    #[test]
    fn test_registry_load_from_toml() {
        let toml_str = r#"
[[masking_rules]]
name = "custom_id"
pattern = "\\bCUSTOM-\\d{6}\\b"
replacement = "***CUSTOM***"
priority = 100
enabled = true

[[masking_rules]]
name = "another"
pattern = "\\bTEST\\b"
"#;
        let rules = MaskRuleRegistry::load_from_toml(toml_str).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name(), "custom_id");
        assert_eq!(rules[1].name(), "another");
        assert_eq!(rules[1].priority(), 100); // default
    }

    // === DataMaskerBuilder 测试 ===

    #[test]
    fn test_builder_default() {
        let masker = DataMasker::builder().build();
        // Should behave like DataMasker::new()
        let result = masker.mask("test@example.com");
        assert_eq!(result, "**@**.***");
    }

    #[test]
    fn test_builder_disable_builtin() {
        let masker = DataMasker::builder().disable_builtin("email").build();
        // Email should NOT be masked
        let result = masker.mask("test@example.com");
        assert_eq!(result, "test@example.com");
        // Phone should still be masked
        let result = masker.mask("13812345678");
        assert_eq!(result, "***-****-****");
    }

    #[test]
    fn test_builder_add_custom_rule() {
        let custom = MaskRule::builder("custom_tag")
            .pattern(r"\bTAG-\d{4}\b")
            .replacement("***TAGGED***")
            .priority(200)
            .build()
            .unwrap();
        let masker = DataMasker::builder().add_rule(custom).build();
        let result = masker.mask("Order TAG-1234 shipped");
        assert!(result.contains("***TAGGED***"), "Custom rule: {}", result);
    }

    #[test]
    fn test_builder_combined() {
        let custom = MaskRule::builder("hex_color")
            .pattern(r"#[0-9a-fA-F]{6}\b")
            .replacement("#******")
            .priority(150)
            .build()
            .unwrap();
        let masker = DataMasker::builder()
            .disable_builtin("phone")
            .add_rule(custom)
            .build();

        // Phone NOT masked
        assert_eq!(masker.mask("13812345678"), "13812345678");
        // Email still masked
        assert_eq!(masker.mask("a@b.com"), "**@**.***");
        // Custom rule works
        assert_eq!(masker.mask("#FF0000"), "#******");
    }
}
