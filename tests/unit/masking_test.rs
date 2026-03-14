// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 数据掩码功能测试
// 测试 PII 自动检测与脱敏功能，确保合规性

#[cfg(test)]
mod masking_test {
    use inklog::masking::DataMasker;
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
        assert!(!result.contains("110101199001011234"));
        assert!(!result.contains("6222021234567890123"));

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
        let message = "Contact: test@example.com (email) or 138-1234-5678 (phone)!";
        let result = masker.mask(message);

        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("138-1234-5678"));
    }
}
