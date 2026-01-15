use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// 扩展敏感字段检测列表 - 包含常见敏感字段名称模式
static SENSITIVE_FIELDS: &[&str] = &[
    // 基础敏感字段
    "password",
    "token",
    "secret",
    "key",
    "credential",
    "auth",
    // API 相关
    "api_key",
    "api_key_id",
    "api_secret",
    "access_key",
    "access_key_id",
    "secret_key",
    "private_key",
    "public_key",
    "encryption_key",
    "decryption_key",
    "master_key",
    "session_key",
    "oauth",
    "oauth_token",
    "oauth_secret",
    "bearer",
    "bearer_token",
    "jwt",
    "session_id",
    "session_token",
    // AWS 相关
    "aws_secret",
    "aws_key",
    "aws_token",
    "aws_credentials",
    // 数据库相关
    "database_url",
    "db_password",
    "db_user",
    "connection_string",
    // 支付相关
    "credit_card",
    "card_number",
    "cvv",
    "ssn",
    "social_security",
    // 其他敏感
    "client_secret",
    "client_id",
    "refresh_token",
    "pin",
    "pin_code",
    "two_factor",
    "totp",
    "backup_code",
    "recovery_code",
];

#[derive(Debug, Clone, Default)]
pub struct DataMasker {
    rules: Vec<MaskRule>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MaskRule {
    name: String,
    pattern: Regex,
    replacement: String,
    replace_count: usize,
}

impl DataMasker {
    pub fn new() -> Self {
        let rules = vec![
            MaskRule::new_email_rule(),
            MaskRule::new_phone_rule(),
            MaskRule::new_id_card_rule(),
            MaskRule::new_bank_card_rule(),
            MaskRule::new_api_key_rule(),
            MaskRule::new_aws_key_rule(),
            MaskRule::new_jwt_rule(),
            MaskRule::new_generic_secret_rule(),
        ];

        Self { rules }
    }

    /// 检查字段名是否为敏感字段（大小写不敏感）
    pub fn is_sensitive_field(field_name: &str) -> bool {
        let lower_name = field_name.to_lowercase();
        SENSITIVE_FIELDS
            .iter()
            .any(|sensitive| lower_name.contains(*sensitive))
    }

    pub fn mask(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            result = rule.apply(&result);
        }
        result
    }

    pub fn mask_value(&self, value: &mut Value) {
        match value {
            Value::String(s) => {
                *s = self.mask(s);
            }
            Value::Array(arr) => {
                for item in arr {
                    self.mask_value(item);
                }
            }
            Value::Object(map) => {
                for (_, v) in map {
                    self.mask_value(v);
                }
            }
            _ => {}
        }
    }

    pub fn mask_hashmap(&self, map: &mut HashMap<String, Value>) {
        for (_, v) in map.iter_mut() {
            self.mask_value(v);
        }
    }
}

use std::sync::LazyLock;

/// Pre-compiled regex patterns for better performance
static EMAIL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+").expect("Invalid email regex"));

static PHONE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b1[3-9]\d{9}\b").expect("Invalid phone regex"));

static ID_CARD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{6})(\d{8})(\d{3}[\dX])$").expect("Invalid ID card regex"));

static BANK_CARD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})(\d+)(\d{4})").expect("Invalid bank card regex"));

/// API Key 模式 - 匹配常见的 API key 格式
static API_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(api[_-]?key[^\s:=]*\s*[=:]\s*[a-zA-Z0-9_-]{20,})")
        .expect("Invalid API key regex")
});

/// AWS Access Key 模式 - 匹配 AKIA 开头的 AWS 密钥
static AWS_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}").expect("Invalid AWS key regex")
});

/// JWT Token 模式 - 匹配 JWT 格式
static JWT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*")
        .expect("Invalid JWT regex")
});

/// 通用密钥/密码模式 - 匹配 key=value 或 "key": "value" 中的敏感值
static GENERIC_SECRET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([^\s:=]*(?:token|secret|key|password|passwd|pwd|credential)s?[^\s:=]*\s*[=:]\s*)([a-zA-Z0-9_\-\+]{16,})")
        .expect("Invalid generic secret regex")
});

impl MaskRule {
    fn new_email_rule() -> Self {
        Self {
            name: "email".to_string(),
            pattern: EMAIL_REGEX.clone(),
            replacement: "**@**.***".to_string(),
            replace_count: 1,
        }
    }

    fn new_phone_rule() -> Self {
        Self {
            name: "phone".to_string(),
            pattern: PHONE_REGEX.clone(),
            replacement: "***-****-****".to_string(),
            replace_count: 1,
        }
    }

    fn new_id_card_rule() -> Self {
        Self {
            name: "id_card".to_string(),
            pattern: ID_CARD_REGEX.clone(),
            replacement: "MASK_ID_CARD".to_string(), // Special marker for custom handling
            replace_count: 1,
        }
    }

    fn new_bank_card_rule() -> Self {
        Self {
            name: "bank_card".to_string(),
            pattern: BANK_CARD_REGEX.clone(),
            replacement: "MASK_BANK_CARD".to_string(), // Special marker for custom handling
            replace_count: 1,
        }
    }

    fn new_api_key_rule() -> Self {
        Self {
            name: "api_key".to_string(),
            pattern: API_KEY_REGEX.clone(),
            replacement: "${1}***REDACTED***${3}".to_string(),
            replace_count: 1,
        }
    }

    fn new_aws_key_rule() -> Self {
        Self {
            name: "aws_key".to_string(),
            pattern: AWS_KEY_REGEX.clone(),
            replacement: "***REDACTED***".to_string(),
            replace_count: 1,
        }
    }

    fn new_jwt_rule() -> Self {
        Self {
            name: "jwt".to_string(),
            pattern: JWT_REGEX.clone(),
            replacement: "***REDACTED_JWT***".to_string(),
            replace_count: 1,
        }
    }

    fn new_generic_secret_rule() -> Self {
        Self {
            name: "generic_secret".to_string(),
            pattern: GENERIC_SECRET_REGEX.clone(),
            replacement: "${1}***REDACTED***${3}".to_string(),
            replace_count: 1,
        }
    }

    fn apply(&self, text: &str) -> String {
        if self.name == "id_card" {
            // ID card: mask all but last 4 digits
            self.pattern.replace(text, "******$3").to_string()
        } else if self.name == "bank_card" {
            // Bank card: check if it looks like a bank card number (all digits, >= 12 chars)
            if text.len() >= 12 && text.chars().all(|c| c.is_ascii_digit()) {
                let last_four = &text[text.len() - 4..];
                format!("****-****-****-{}", last_four)
            } else {
                text.to_string()
            }
        } else if self.name == "api_key" || self.name == "generic_secret" {
            // For patterns with capture groups, use the replacement with group references
            self.pattern
                .replace(text, self.replacement.as_str())
                .to_string()
        } else {
            // For email and phone, use the standard replacement
            self.pattern
                .replace(text, self.replacement.as_str())
                .to_string()
        }
    }
}

pub fn mask_email(email: &str) -> String {
    EMAIL_REGEX.replace(email, "**@**.***").to_string()
}

pub fn mask_phone(phone: &str) -> String {
    PHONE_REGEX.replace(phone, "***-****-****").to_string()
}

pub fn mask_id_card(id_card: &str) -> String {
    // 身份证号掩码：只保留后4位，如果是X结尾则保留最后3位+X
    ID_CARD_REGEX
        .replace(id_card, |caps: &regex::Captures| {
            let suffix = caps.get(3).unwrap().as_str();
            format!("******{}", suffix)
        })
        .to_string()
}

pub fn mask_bank_card(bank_card: &str) -> String {
    // 银行卡号掩码：只保留后4位，支持16位和19位卡号
    if bank_card.len() > 4 {
        let last_four = &bank_card[bank_card.len() - 4..];
        format!("****-****-****-{}", last_four)
    } else {
        bank_card.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email() {
        let test_cases = vec![
            ("test@example.com", "**@**.***"),
            ("user.name@company.co.uk", "**@**.***"),
            ("admin@localhost", "**@**.***"),
        ];

        for (input, expected) in test_cases {
            let result = mask_email(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_phone() {
        let test_cases = vec![
            ("13812345678", "***-****-****"),
            ("15987654321", "***-****-****"),
            ("Contact: 18655556666 now", "Contact: ***-****-**** now"),
        ];

        for (input, expected) in test_cases {
            let result = mask_phone(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_id_card() {
        let test_cases = vec![
            ("110101199001011234", "******1234"),
            ("31011519880530218X", "******218X"),
        ];

        for (input, expected) in test_cases {
            let result = mask_id_card(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_bank_card() {
        let test_cases = vec![
            ("6222021234567890123", "****-****-****-0123"),
            ("4567890123456789", "****-****-****-6789"),
        ];

        for (input, expected) in test_cases {
            let result = mask_bank_card(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_data_masker() {
        let masker = DataMasker::new();

        let test_email = "user@example.com";
        assert_eq!(masker.mask(test_email), "**@**.***");

        let test_phone = "13912345678";
        assert_eq!(masker.mask(test_phone), "***-****-****");

        let mixed = "Contact user at test@example.com, phone: 13812345678";
        let result = masker.mask(mixed);
        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("13812345678"));
    }

    #[test]
    fn test_mask_value() {
        let masker = DataMasker::new();

        let mut value = serde_json::json!({
            "email": "user@example.com",
            "phone": "13712345678",
            "name": "John"
        });

        masker.mask_value(&mut value);

        assert_eq!(value["email"], "**@**.***");
        assert_eq!(value["phone"], "***-****-****");
        assert_eq!(value["name"], "John");
    }

    #[test]
    fn test_mask_nested_value() {
        let masker = DataMasker::new();

        let mut value = serde_json::json!({
            "user": {
                "email": "admin@company.org",
                "contacts": ["test@email.com", "13811112222"]
            }
        });

        masker.mask_value(&mut value);

        let user = &value["user"];
        assert_eq!(user["email"], "**@**.***");

        let contacts = user["contacts"]
            .as_array()
            .expect("contacts should be an array");
        assert_eq!(contacts[0], "**@**.***");
        assert_eq!(contacts[1], "***-****-****");
    }
}
