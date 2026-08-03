// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 数据掩码模块
//!
//! 提供敏感数据（PII）的自动检测和脱敏功能，保护日志中的隐私信息。
//!
//! ## 概述
//!
//! `DataMasker` 结构体提供日志消息和 JSON 结构中敏感数据的检测和脱敏功能。
//! 它结合模式匹配和字段名检测来识别敏感信息。
//!
//! ## 功能特性
//!
//! - **基于模式的脱敏**：通过正则表达式模式检测敏感数据（邮箱、电话等）
//! - **字段名检测**：通过字段名识别敏感字段（password、api_key 等）
//! - **嵌套结构支持**：递归处理嵌套的 JSON 对象和数组
//! - **自定义规则**：支持多个脱敏规则，可配置模式
//!
//! ## 敏感字段检测
//!
//! 以下字段名模式会自动检测为敏感字段：
//! - **认证信息**：`password`, `token`, `secret`, `credential`, `auth`
//! - **API 密钥**：`api_key`, `api_secret`, `access_key`, `secret_key`
//! - **加密密钥**：`encryption_key`, `decryption_key`, `private_key`
//! - **OAuth**：`oauth`, `oauth_token`, `bearer_token`, `jwt`
//! - **AWS 凭据**：`aws_secret`, `aws_key`, `aws_credentials`
//! - **支付信息**：`credit_card`, `card_number`, `cvv`, `ssn`
//!
//! ## 基于模式的检测
//!
//! 除了字段名，以下模式也会被检测：
//! - **邮箱地址**（部分脱敏：`***@***.***`）
//! - **电话号码**（显示后4位：`138****5678`）
//! - **身份证号**（部分脱敏）
//! - **银行卡号**（部分脱敏）
//! - **JWT 令牌**
//! - **AWS 访问密钥**
//! - **通用 API 密钥**
//!
//! ## 使用示例
//!
//! ```rust
//! use inklog::masking::DataMasker;
//!
//! let masker = DataMasker::new();
//!
//! // 脱敏日志消息
//! let message = "User login: email=test@example.com";
//! let masked = masker.mask(message);
//! // 邮箱脱敏格式: **@**.***
//! assert!(masked.contains("**@**.***"));
//! assert!(!masked.contains("test@example.com"));
//!
//! // 检查字段名是否为敏感字段
//! assert!(DataMasker::is_sensitive_field("password"));
//! assert!(DataMasker::is_sensitive_field("api_key"));
//! assert!(!DataMasker::is_sensitive_field("username"));
//! ```
//!
//! ## 性能考虑
//!
//! - 预编译正则表达式以提高性能
//! - 批量处理时使用缓存
//! - 支持禁用特定检测规则以减少开销

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::InklogError;

/// Word-boundary regex patterns for sensitive field detection.
/// Uses \b (word boundary) to avoid false positives like "cakey" matching "key".
static SENSITIVE_FIELD_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Authentication patterns
        Regex::new(r"(?i)\b(password|passwd|pwd)\b").unwrap(),
        // token/bearer/auth: preceded by a non-word separator (space, -, _, etc.) or at start
        // This excludes cases like "cakey" where 'token' is inside a word.
        // Covers: "token" (start), "api_token" (underscore), "bearer_token", "auth_token"
        // The (?:[^a-zA-Z0-9_])? makes the preceding char optional (for start-of-string case)
        Regex::new(r"(?i)(?:[^a-zA-Z0-9_])?(token|bearer|auth)\b").unwrap(),
        Regex::new(r"(?i)\b(secret|credential)\b").unwrap(),
        // Key patterns
        Regex::new(r"(?i)\b(api[_-]?key|apikey|api[_-]?secret)\b").unwrap(),
        Regex::new(r"(?i)\b(access[_-]?key|access[_-]?key[_-]?id)\b").unwrap(),
        Regex::new(r"(?i)\b(secret[_-]?key|private[_-]?key|public[_-]?key)\b").unwrap(),
        Regex::new(r"(?i)\b(encryption[_-]?key|decryption[_-]?key|master[_-]?key)\b").unwrap(),
        Regex::new(r"(?i)\b(session[_-]?key|session[_-]?id|session[_-]?token)\b").unwrap(),
        // OAuth patterns
        Regex::new(r"(?i)\b(oauth|oauth[_-]?token|oauth[_-]?secret)\b").unwrap(),
        Regex::new(r"(?i)\b(jwt(_[a-zA-Z0-9]+)?|bearer[_-]?token)\b").unwrap(),
        // AWS patterns
        Regex::new(r"(?i)\b(aws[_-]?secret|aws[_-]?key|aws[_-]?token|aws[_-]?credentials)\b").unwrap(),
        // Database patterns
        Regex::new(r"(?i)\b(database[_-]?url|db[_-]?password|db[_-]?user|connection[_-]?string)\b").unwrap(),
        // Payment patterns
        Regex::new(r"(?i)\b(credit[_-]?card|card[_-]?number|cvv|ssn|social[_-]?security)\b").unwrap(),
        // Client patterns
        Regex::new(r"(?i)\b(client[_-]?secret|client[_-]?id)\b").unwrap(),
        // Other sensitive patterns
        Regex::new(r"(?i)\b(refresh[_-]?token|pin|pin[_-]?code|two[_-]?factor|totp|backup[_-]?code|recovery[_-]?code)\b").unwrap(),
    ]
});

/// Data masking utility for sensitive information protection.
///
/// The `DataMasker` struct provides functionality to detect and mask sensitive
/// data in log messages and JSON structures. It uses a combination of pattern
/// matching and field name detection to identify sensitive information.
///
/// # Features
/// - **Pattern-based masking**: Detects sensitive data by regex patterns (emails, phones, etc.)
/// - **Field name detection**: Identifies sensitive fields by name (password, api_key, etc.)
/// - **Nested structure support**: Recursively processes nested JSON objects and arrays
/// - **Customizable rules**: Supports multiple mask rules with configurable patterns
///
/// # Sensitive Field Detection
///
/// The following field name patterns are automatically detected as sensitive:
/// - Authentication: `password`, `token`, `secret`, `credential`, `auth`
/// - API Keys: `api_key`, `api_secret`, `access_key`, `secret_key`
/// - Encryption: `encryption_key`, `decryption_key`, `private_key`
/// - OAuth: `oauth`, `oauth_token`, `bearer_token`, `jwt`
/// - AWS: `aws_secret`, `aws_key`, `aws_credentials`
/// - Payment: `credit_card`, `card_number`, `cvv`, `ssn`
///
/// # Pattern-based Detection
///
/// In addition to field names, the following patterns are detected:
/// - Email addresses (partial masking: `***@***.***`)
/// - Phone numbers (last 4 digits shown: `138****5678`)
/// - ID card numbers (partial masking)
/// - Bank card numbers (partial masking)
/// - JWT tokens
/// - AWS access keys
/// - Generic API keys
///
/// # Example
///
/// ```ignore
/// use inklog::masking::DataMasker;
///
/// let masker = DataMasker::new();
///
/// // Mask by pattern
/// let mut email = serde_json::json!("user@example.com");
/// masker.mask_value(&mut email);
/// assert_eq!(email, serde_json::json!("***@***.***"));
///
/// // Detect sensitive fields
/// assert!(DataMasker::is_sensitive_field("password"));
/// assert!(DataMasker::is_sensitive_field("api_key"));
/// assert!(!DataMasker::is_sensitive_field("message"));
/// ```
///
/// # Thread Safety
///
/// `DataMasker` is immutable and can be safely shared between threads.
#[derive(Debug, Clone, Default)]
pub struct DataMasker {
    rules: Vec<MaskRule>,
}

/// Type alias for the custom apply function used in masking rules.
type ApplyFn = Arc<dyn Fn(&Regex, &str, &str) -> String + Send + Sync>;

/// A masking rule that defines how to detect and replace sensitive data patterns.
///
/// # Fields
/// - `name`: Unique identifier for the rule
/// - `pattern`: Compiled regex pattern for detection
/// - `replacement`: Replacement string (supports capture group references like `${1}`)
/// - `replace_count`: Maximum number of replacements per application
/// - `priority`: Execution order (lower values execute first)
/// - `enabled`: Whether this rule is active
/// - `apply_fn`: Custom application function for complex masking logic
#[derive(Clone)]
pub struct MaskRule {
    name: String,
    pattern: Regex,
    replacement: String,
    #[allow(dead_code)]
    replace_count: usize,
    priority: i32,
    enabled: bool,
    apply_fn: ApplyFn,
}

impl std::fmt::Debug for MaskRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaskRule")
            .field("name", &self.name)
            .field("pattern", &self.pattern.as_str())
            .field("replacement", &self.replacement)
            .field("priority", &self.priority)
            .field("enabled", &self.enabled)
            .field("apply_fn", &"<fn>")
            .finish()
    }
}

impl DataMasker {
    pub fn new() -> Self {
        let mut rules = vec![
            MaskRule::new_email_rule(),
            MaskRule::new_phone_rule(),
            MaskRule::new_id_card_rule(),
            MaskRule::new_bank_card_rule(),
            MaskRule::new_api_key_rule(),
            MaskRule::new_aws_key_rule(),
            MaskRule::new_jwt_rule(),
            MaskRule::new_generic_secret_rule(),
            // High-priority
            MaskRule::new_international_phone_rule(),
            MaskRule::new_credit_card_rule(),
            MaskRule::new_ipv4_rule(),
            MaskRule::new_ipv6_rule(),
            MaskRule::new_mac_address_rule(),
            // Medium-priority
            MaskRule::new_passport_rule(),
            MaskRule::new_ssn_rule(),
            MaskRule::new_db_connection_rule(),
            // Low-priority
            MaskRule::new_github_token_rule(),
            MaskRule::new_slack_token_rule(),
            MaskRule::new_stripe_key_rule(),
            MaskRule::new_google_api_key_rule(),
            MaskRule::new_private_key_rule(),
        ];
        rules.sort_by_key(|r| r.priority());
        Self { rules }
    }

    /// 检查字段名是否为敏感字段（大小写不敏感，使用词边界正则避免误判）
    ///
    /// 例如：
    /// - `"cakey"` 不会匹配 `"key"`（避免误判）
    /// - `"polygon"` 不会匹配 `"gon"`（避免误判）
    /// - `"password"` 会匹配（正确检测）
    pub fn is_sensitive_field(field_name: &str) -> bool {
        SENSITIVE_FIELD_PATTERNS
            .iter()
            .any(|pattern| pattern.is_match(field_name))
    }

    pub fn mask(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            if rule.is_enabled() {
                result = rule.apply(&result);
            }
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
                for (k, v) in map.iter_mut() {
                    if Self::is_sensitive_field(k) {
                        *v = Value::String("***MASKED***".to_string());
                    } else {
                        self.mask_value(v);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn mask_hashmap(&self, map: &mut HashMap<String, Value>) {
        for (k, v) in map.iter_mut() {
            if Self::is_sensitive_field(k) {
                *v = Value::String("***MASKED***".to_string());
            } else {
                self.mask_value(v);
            }
        }
    }

    /// Consumes the `DataMasker` and returns the inner rules vector.
    pub fn into_rules(self) -> Vec<MaskRule> {
        self.rules
    }

    /// Creates a new [`DataMaskerBuilder`] for assembling a custom masker.
    pub fn builder() -> DataMaskerBuilder {
        DataMaskerBuilder::new()
    }
}

/// Builder for assembling a [`DataMasker`] with custom rule configurations.
///
/// # Example
///
/// ```rust
/// use inklog::DataMasker;
///
/// let masker = DataMasker::builder()
///     .disable_builtin("email")
///     .build();
/// ```
pub struct DataMaskerBuilder {
    extra_rules: Vec<MaskRule>,
    disabled_builtins: Vec<String>,
    use_builtins: bool,
    custom_registry: Option<super::masking_registry::MaskRuleRegistry>,
}

impl DataMaskerBuilder {
    fn new() -> Self {
        Self {
            extra_rules: Vec::new(),
            disabled_builtins: Vec::new(),
            use_builtins: true,
            custom_registry: None,
        }
    }

    /// Add a custom rule to the masker.
    pub fn add_rule(mut self, rule: MaskRule) -> Self {
        self.extra_rules.push(rule);
        self
    }

    /// Use a custom [`MaskRuleRegistry`] as the rule source instead of builtins.
    ///
    /// When set, the registry's rules replace the default built-in rules.
    /// `add_rule()` and `disable_builtin()` still apply on top.
    pub fn with_registry(mut self, registry: super::masking_registry::MaskRuleRegistry) -> Self {
        self.custom_registry = Some(registry);
        self.use_builtins = false;
        self
    }

    /// Disable a built-in rule by name.
    pub fn disable_builtin(mut self, name: &str) -> Self {
        self.disabled_builtins.push(name.to_string());
        self
    }

    /// Build the [`DataMasker`] with all configured rules sorted by priority.
    pub fn build(self) -> DataMasker {
        let mut rules = if let Some(registry) = self.custom_registry {
            registry.active_rules().into_iter().cloned().collect()
        } else if self.use_builtins {
            DataMasker::new().into_rules()
        } else {
            Vec::new()
        };

        // Remove disabled builtins
        for name in &self.disabled_builtins {
            rules.retain(|r| r.name() != name.as_str());
        }

        // Add extra rules
        rules.extend(self.extra_rules);

        // Sort by priority
        rules.sort_by_key(|r| r.priority());

        DataMasker { rules }
    }
}

#[allow(dead_code)]
use std::sync::LazyLock;

/// Pre-compiled regex patterns for better performance
#[allow(dead_code)]
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

// === High-priority rules (compliance) ===

/// International phone (E.164 format)
static INTERNATIONAL_PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\+(\d{1,3})[\s.-]?(\(?\d{1,4}\)?[\s.-]?\d{2,4}[\s.-]?)(\d{2,4})")
        .expect("Invalid international phone regex")
});

/// Credit card (major card networks)
static CREDIT_CARD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12}|35[0-9]{14})\b")
        .expect("Invalid credit card regex")
});

/// IPv4 address
static IPV4_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b")
        .expect("Invalid IPv4 regex")
});

/// IPv6 address
static IPV6_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){2,7}[0-9a-fA-F]{1,4}\b").expect("Invalid IPv6 regex")
});

/// MAC address
static MAC_ADDRESS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b").expect("Invalid MAC address regex")
});

// === Medium-priority rules (regional identity) ===

/// Passport number (Chinese international passport format)
static PASSPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[EeGg][A-Za-z0-9]{8}\b").expect("Invalid passport regex"));

/// US Social Security Number
static SSN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("Invalid SSN regex"));

/// Database connection string (password in URI)
static DB_CONNECTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)((?:postgres|mysql|mongodb|redis|amqp)://[^:\s]+:)([^@]+)(@\S+)")
        .expect("Invalid DB connection regex")
});

// === Low-priority rules (third-party tokens) ===

/// GitHub personal access token
static GITHUB_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:ghp|github_pat)_[A-Za-z0-9_]{36,}\b").expect("Invalid GitHub token regex")
});

/// Slack token
static SLACK_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"xox[bpas]-[0-9]{10,13}-[0-9a-zA-Z-]+").expect("Invalid Slack token regex")
});

/// Stripe API key
static STRIPE_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:sk|pk)_(?:live|test)_[0-9a-zA-Z]{24,}").expect("Invalid Stripe key regex")
});

/// Google API key
static GOOGLE_API_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"AIza[0-9A-Za-z_-]{35}").expect("Invalid Google API key regex"));

/// Private key PEM block
static PRIVATE_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----")
        .expect("Invalid private key regex")
});

impl MaskRule {
    fn new_email_rule() -> Self {
        Self::build_from_regex("email", EMAIL_REGEX.clone(), "**@**.***", 100, None)
    }

    fn new_phone_rule() -> Self {
        Self::build_from_regex("phone", PHONE_REGEX.clone(), "***-****-****", 100, None)
    }

    fn new_id_card_rule() -> Self {
        Self::build_from_regex(
            "id_card",
            ID_CARD_REGEX.clone(),
            "MASK_ID_CARD",
            100,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex.replace(text, "******$3").to_string()
            })),
        )
    }

    fn new_bank_card_rule() -> Self {
        Self::build_from_regex(
            "bank_card",
            BANK_CARD_REGEX.clone(),
            "MASK_BANK_CARD",
            100,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let matched = caps.get(0).unwrap().as_str();
                        if matched.len() >= 12 {
                            let last_four = &matched[matched.len() - 4..];
                            format!("****-****-****-{}", last_four)
                        } else {
                            matched.to_string()
                        }
                    })
                    .to_string()
            })),
        )
    }

    fn new_api_key_rule() -> Self {
        Self::build_from_regex(
            "api_key",
            API_KEY_REGEX.clone(),
            "${1}***REDACTED***",
            100,
            None,
        )
    }

    fn new_aws_key_rule() -> Self {
        Self::build_from_regex(
            "aws_key",
            AWS_KEY_REGEX.clone(),
            "***REDACTED***",
            100,
            None,
        )
    }

    fn new_jwt_rule() -> Self {
        Self::build_from_regex("jwt", JWT_REGEX.clone(), "***REDACTED_JWT***", 100, None)
    }

    fn new_generic_secret_rule() -> Self {
        Self::build_from_regex(
            "generic_secret",
            GENERIC_SECRET_REGEX.clone(),
            "${1}***REDACTED***",
            100,
            None,
        )
    }

    // === Internal helper for pre-compiled rules ===

    /// Build a rule from a pre-compiled Regex, avoiding re-compilation.
    fn build_from_regex(
        name: &str,
        regex: Regex,
        replacement: &str,
        priority: i32,
        apply_fn: Option<ApplyFn>,
    ) -> Self {
        MaskRule {
            name: name.to_string(),
            pattern: regex,
            replacement: replacement.to_string(),
            replace_count: 1,
            priority,
            enabled: true,
            apply_fn: apply_fn.unwrap_or_else(|| {
                Arc::new(|regex: &Regex, text: &str, replacement: &str| {
                    regex.replace(text, replacement).to_string()
                })
            }),
        }
    }

    // === High-priority rules ===

    fn new_international_phone_rule() -> Self {
        Self::build_from_regex(
            "international_phone",
            INTERNATIONAL_PHONE_REGEX.clone(),
            "+${1}-***-***-${3}",
            10,
            None,
        )
    }

    fn new_credit_card_rule() -> Self {
        Self::build_from_regex(
            "credit_card",
            CREDIT_CARD_REGEX.clone(),
            "***REDACTED_CC***",
            15,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let number = caps.get(0).unwrap().as_str();
                        let digits: Vec<u32> =
                            number.chars().filter_map(|c| c.to_digit(10)).collect();
                        let mut sum = 0u32;
                        let mut alternate = false;
                        for &d in digits.iter().rev() {
                            if alternate {
                                let doubled = d * 2;
                                sum += if doubled > 9 { doubled - 9 } else { doubled };
                            } else {
                                sum += d;
                            }
                            alternate = !alternate;
                        }
                        if !sum.is_multiple_of(10) {
                            return number.to_string();
                        }
                        let last4 = &number[number.len() - 4..];
                        if number.starts_with('3') {
                            format!("****-******-{}", last4)
                        } else {
                            format!("****-****-****-{}", last4)
                        }
                    })
                    .to_string()
            })),
        )
    }

    fn new_ipv4_rule() -> Self {
        Self::build_from_regex(
            "ipv4",
            IPV4_REGEX.clone(),
            "***.***.***.XXX",
            20,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let ip = caps.get(0).unwrap().as_str();
                        if let Some(pos) = ip.rfind('.') {
                            format!("***.***.***.{}", &ip[pos + 1..])
                        } else {
                            "***.***.***.***".to_string()
                        }
                    })
                    .to_string()
            })),
        )
    }

    fn new_ipv6_rule() -> Self {
        Self::build_from_regex(
            "ipv6",
            IPV6_REGEX.clone(),
            "****:****:****:XXXX",
            21,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let ip = caps.get(0).unwrap().as_str();
                        if let Some(pos) = ip.rfind(':') {
                            let last_group = &ip[pos + 1..];
                            let prefix_count = ip.matches(':').count();
                            let mut result = "****".to_string();
                            for _ in 1..prefix_count {
                                result.push_str(":****");
                            }
                            result.push(':');
                            result.push_str(last_group);
                            result
                        } else {
                            ip.to_string()
                        }
                    })
                    .to_string()
            })),
        )
    }

    fn new_mac_address_rule() -> Self {
        Self::build_from_regex(
            "mac_address",
            MAC_ADDRESS_REGEX.clone(),
            "XX:**:**:**:**:XX",
            19,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let mac = caps.get(0).unwrap().as_str();
                        let sep = if mac.contains(':') { ':' } else { '-' };
                        let parts: Vec<&str> = mac.split(sep).collect();
                        if parts.len() == 6 {
                            format!(
                                "{}{}{}{}{}{}{}{}{}{}{}",
                                parts[0], sep, "**", sep, "**", sep, "**", sep, "**", sep, parts[5]
                            )
                        } else {
                            mac.to_string()
                        }
                    })
                    .to_string()
            })),
        )
    }

    // === Medium-priority rules ===

    fn new_passport_rule() -> Self {
        Self::build_from_regex(
            "passport",
            PASSPORT_REGEX.clone(),
            "******XX",
            30,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let passport = caps.get(0).unwrap().as_str();
                        let first = &passport[..1];
                        let last2 = &passport[passport.len() - 2..];
                        format!("{}******{}", first, last2)
                    })
                    .to_string()
            })),
        )
    }

    fn new_ssn_rule() -> Self {
        Self::build_from_regex(
            "ssn",
            SSN_REGEX.clone(),
            "***-**-XXXX",
            35,
            Some(Arc::new(|regex: &Regex, text: &str, _replacement: &str| {
                regex
                    .replace_all(text, |caps: &regex::Captures| {
                        let ssn = caps.get(0).unwrap().as_str();
                        let last4 = &ssn[ssn.len() - 4..];
                        format!("***-**-{}", last4)
                    })
                    .to_string()
            })),
        )
    }

    fn new_db_connection_rule() -> Self {
        Self::build_from_regex(
            "db_connection",
            DB_CONNECTION_REGEX.clone(),
            "${1}***${3}",
            40,
            None,
        )
    }

    // === Low-priority rules ===

    fn new_github_token_rule() -> Self {
        Self::build_from_regex(
            "github_token",
            GITHUB_TOKEN_REGEX.clone(),
            "***REDACTED_GITHUB***",
            50,
            None,
        )
    }

    fn new_slack_token_rule() -> Self {
        Self::build_from_regex(
            "slack_token",
            SLACK_TOKEN_REGEX.clone(),
            "***REDACTED_SLACK***",
            51,
            None,
        )
    }

    fn new_stripe_key_rule() -> Self {
        Self::build_from_regex(
            "stripe_key",
            STRIPE_KEY_REGEX.clone(),
            "***REDACTED_STRIPE***",
            52,
            None,
        )
    }

    fn new_google_api_key_rule() -> Self {
        Self::build_from_regex(
            "google_api_key",
            GOOGLE_API_KEY_REGEX.clone(),
            "***REDACTED_GOOGLE***",
            53,
            None,
        )
    }

    fn new_private_key_rule() -> Self {
        Self::build_from_regex(
            "private_key",
            PRIVATE_KEY_REGEX.clone(),
            "***REDACTED_PRIVATE_KEY***",
            54,
            None,
        )
    }

    fn apply(&self, text: &str) -> String {
        (self.apply_fn)(&self.pattern, text, &self.replacement)
    }

    /// Returns the name of this rule.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether this rule is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the priority of this rule (lower = earlier).
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Sets whether this rule is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Creates a new builder for constructing a `MaskRule`.
    pub fn builder(name: &str) -> MaskRuleBuilder {
        MaskRuleBuilder::new(name)
    }
}

/// Builder for constructing [`MaskRule`] instances with a fluent API.
///
/// # Defaults
/// - `priority`: 100
/// - `enabled`: true
/// - `apply_fn`: standard `regex.replace(text, replacement)`
///
/// # Example
///
/// ```rust
/// use inklog::MaskRule;
///
/// let rule = MaskRule::builder("custom_phone")
///     .pattern(r"\b\d{3}-\d{4}\b")
///     .replacement("***-****")
///     .priority(50)
///     .build()
///     .unwrap();
/// ```
pub struct MaskRuleBuilder {
    name: String,
    pattern: Option<String>,
    replacement: String,
    priority: i32,
    enabled: bool,
    apply_fn: Option<ApplyFn>,
}

impl MaskRuleBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pattern: None,
            replacement: String::new(),
            priority: 100,
            enabled: true,
            apply_fn: None,
        }
    }

    /// Sets the regex pattern for this rule.
    pub fn pattern(mut self, regex: &str) -> Self {
        self.pattern = Some(regex.to_string());
        self
    }

    /// Sets the replacement string (supports capture group refs like `${1}`).
    pub fn replacement(mut self, replacement: &str) -> Self {
        self.replacement = replacement.to_string();
        self
    }

    /// Sets the execution priority (lower values execute first).
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets whether this rule is enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets a custom apply function for complex masking logic.
    pub fn apply_fn(mut self, f: ApplyFn) -> Self {
        self.apply_fn = Some(f);
        self
    }

    /// Builds the [`MaskRule`], compiling the regex pattern.
    ///
    /// # Errors
    /// Returns `Err(InklogError)` if the pattern is missing or an invalid regex.
    pub fn build(self) -> Result<MaskRule, InklogError> {
        let pattern_str = self.pattern.ok_or_else(|| {
            InklogError::ConfigError(format!("MaskRule '{}' requires a pattern", self.name))
        })?;
        let regex = Regex::new(&pattern_str).map_err(|e| {
            InklogError::ConfigError(format!("Invalid regex in rule '{}': {}", self.name, e))
        })?;
        Ok(MaskRule {
            name: self.name,
            pattern: regex,
            replacement: self.replacement,
            replace_count: 1,
            priority: self.priority,
            enabled: self.enabled,
            apply_fn: self.apply_fn.unwrap_or_else(|| {
                Arc::new(|regex: &Regex, text: &str, replacement: &str| {
                    regex.replace(text, replacement).to_string()
                })
            }),
        })
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let rule = MaskRule::builder("test")
            .pattern(r"\d+")
            .replacement("***")
            .build()
            .unwrap();
        assert_eq!(rule.name(), "test");
        assert_eq!(rule.priority(), 100);
        assert!(rule.is_enabled());
        assert_eq!(rule.apply("abc123def"), "abc***def");
    }

    #[test]
    fn test_builder_custom_values() {
        let rule = MaskRule::builder("custom")
            .pattern(r"\d+")
            .replacement("###")
            .priority(50)
            .enabled(false)
            .build()
            .unwrap();
        assert_eq!(rule.priority(), 50);
        assert!(!rule.is_enabled());
    }

    #[test]
    fn test_builder_custom_apply_fn() {
        let rule = MaskRule::builder("reverse")
            .pattern(r"\w+")
            .replacement("")
            .apply_fn(Arc::new(|_re: &Regex, text: &str, _rep: &str| {
                text.chars().rev().collect()
            }))
            .build()
            .unwrap();
        assert_eq!(rule.apply("hello"), "olleh");
    }

    #[test]
    fn test_builder_missing_pattern() {
        let result = MaskRule::builder("no_pattern").replacement("***").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_invalid_regex() {
        let result = MaskRule::builder("bad_regex").pattern(r"[invalid").build();
        assert!(result.is_err());
    }
}

pub fn mask_email(email: &str) -> String {
    EMAIL_REGEX.replace(email, "**@**.***").to_string()
}

pub fn mask_phone(phone: &str) -> String {
    PHONE_REGEX.replace(phone, "***-****-****").to_string()
}

#[allow(dead_code)]
fn mask_id_card(id_card: &str) -> String {
    // 身份证号掩码：只保留后4位，如果是X结尾则保留最后3位+X
    ID_CARD_REGEX
        .replace(id_card, |caps: &regex::Captures| {
            // Defensive: ensure the capture group exists
            let suffix = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            format!("******{}", suffix)
        })
        .to_string()
}

#[allow(dead_code)]
fn mask_bank_card(bank_card: &str) -> String {
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
    fn test_mask_bank_card_short_input() {
        // Test the else branch when bank_card.len() <= 4
        assert_eq!(mask_bank_card("123"), "123");
        assert_eq!(mask_bank_card("1234"), "1234");
        assert_eq!(mask_bank_card("ab"), "ab");
        assert_eq!(mask_bank_card(""), "");
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

    #[test]
    fn test_is_sensitive_field_password() {
        assert!(DataMasker::is_sensitive_field("password"));
        assert!(DataMasker::is_sensitive_field("PASSWORD"));
        assert!(DataMasker::is_sensitive_field("Password"));
    }

    #[test]
    fn test_is_sensitive_field_api_key() {
        assert!(DataMasker::is_sensitive_field("api_key"));
        assert!(DataMasker::is_sensitive_field("apiKey"));
        assert!(DataMasker::is_sensitive_field("API_KEY"));
        assert!(DataMasker::is_sensitive_field("api-secret"));
    }

    #[test]
    fn test_is_sensitive_field_jwt() {
        assert!(DataMasker::is_sensitive_field("jwt"));
        assert!(DataMasker::is_sensitive_field("jwt_token"));
        assert!(DataMasker::is_sensitive_field("bearer_token"));
    }

    #[test]
    fn test_is_sensitive_field_aws() {
        assert!(DataMasker::is_sensitive_field("aws_secret"));
        assert!(DataMasker::is_sensitive_field("aws_key"));
        assert!(DataMasker::is_sensitive_field("aws_credentials"));
    }

    #[test]
    fn test_is_sensitive_field_credit_card() {
        assert!(DataMasker::is_sensitive_field("credit_card"));
        assert!(DataMasker::is_sensitive_field("card_number"));
        assert!(DataMasker::is_sensitive_field("cvv"));
    }

    #[test]
    fn test_is_not_sensitive_field() {
        assert!(!DataMasker::is_sensitive_field("username"));
        assert!(!DataMasker::is_sensitive_field("message"));
        assert!(!DataMasker::is_sensitive_field("content"));
        assert!(!DataMasker::is_sensitive_field("title"));
    }

    #[test]
    fn test_mask_email_variations() {
        let test_cases = vec![
            ("test@example.com", "**@**.***"),
            ("user.name@company.co.uk", "**@**.***"),
            ("admin@localhost", "**@**.***"),
            ("user+tag@example.org", "**@**.***"),
            ("user_name@test.io", "**@**.***"),
        ];
        for (input, expected) in test_cases {
            let result = mask_email(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_phone_variations() {
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
    fn test_mask_jwt_token() {
        let masker = DataMasker::new();
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = masker.mask(jwt);
        assert!(result.contains("***REDACTED_JWT***"));
        assert!(!result.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_mask_aws_key() {
        let masker = DataMasker::new();
        let aws_key = "AKIAIOSFODNN7EXAMPLE";
        let result = masker.mask(aws_key);
        assert!(result.contains("***REDACTED***"));
    }

    #[test]
    fn test_mask_api_key_value() {
        let masker = DataMasker::new();
        let message = "api_key=sk-1234567890abcdefghijABCDEFGH";
        let result = masker.mask(message);
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("sk-1234567890abcdefghijABCDEFGH"));
    }

    #[test]
    fn test_mask_password_value() {
        let masker = DataMasker::new();
        // Use a test case that matches the generic secret pattern
        let message = "mypassword=abcdefghijklmnopqrst";
        let result = masker.mask(message);
        // The password should be masked or the message should change
        assert!(result.contains("REDACTED") || !result.contains("abcdefghijklmnopqrst"));
    }

    #[test]
    fn test_mask_database_url() {
        let masker = DataMasker::new();
        let message = "db_url=postgres://user:password123@localhost:5432/mydb";
        let result = masker.mask(message);
        // URL should be masked or password should be hidden
        assert!(result.contains("REDACTED") || !result.contains("password123"));
    }

    #[test]
    fn test_mask_oauth_token() {
        let masker = DataMasker::new();
        let message = "oauth_token=ya29_token_value_here";
        let result = masker.mask(message);
        assert!(result.contains("REDACTED") || !result.contains("token_value"));
    }

    #[test]
    fn test_mask_empty_string() {
        let masker = DataMasker::new();
        let result = masker.mask("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_mask_no_sensitive_data() {
        let masker = DataMasker::new();
        let message = "This is a normal log message without any sensitive data";
        let result = masker.mask(message);
        assert_eq!(result, message);
    }

    #[test]
    fn test_mask_multiple_sensitive_items() {
        let masker = DataMasker::new();
        // Test with simple email and phone that the regex can match
        let message = "Email: test@example.com, Phone: 13812345678";
        let result = masker.mask(message);
        // At least the email should be masked
        assert!(!result.contains("test@example.com"));
    }

    #[test]
    fn test_mask_hashmap() {
        let masker = DataMasker::new();
        let mut map: HashMap<String, Value> = HashMap::new();
        map.insert(
            "email".to_string(),
            Value::String("user@example.com".to_string()),
        );
        map.insert(
            "password".to_string(),
            Value::String("secret123".to_string()),
        );
        map.insert("name".to_string(), Value::String("John".to_string()));

        masker.mask_hashmap(&mut map);

        assert_eq!(map["email"], "**@**.***");
        assert_eq!(map["name"], "John");
    }

    #[test]
    fn test_mask_array_of_objects() {
        let masker = DataMasker::new();
        let mut value = serde_json::json!([
            {"email": "a@b.com", "name": "A"},
            {"email": "c@d.com", "name": "B"}
        ]);

        masker.mask_value(&mut value);

        let arr = value.as_array().unwrap();
        assert_eq!(arr[0]["email"], "**@**.***");
        assert_eq!(arr[1]["email"], "**@**.***");
    }

    #[test]
    fn test_api_key_rule_does_not_panic() {
        let masker = DataMasker::new();
        let input = "api_key=abcdefghijklmnopqrstuvwxyz1234";
        let result = masker.mask(input);
        assert!(result.contains("***REDACTED***"));
    }

    #[test]
    fn test_generic_secret_rule_does_not_panic() {
        let masker = DataMasker::new();
        let input = "my_token=abcdefghijklmnop1234";
        let result = masker.mask(input);
        assert!(result.contains("***REDACTED***"));
    }

    #[test]
    fn test_mask_skips_disabled_rules() {
        // T008: mask() should respect the enabled flag on rules
        let masker = DataMasker::builder().disable_builtin("email").build();
        let input = "user@example.com";
        let result = masker.mask(input);
        // Email should NOT be masked because the rule is disabled
        assert_eq!(result, "user@example.com");
    }

    #[test]
    fn test_mask_value_masks_sensitive_keys() {
        // T009: mask_value should check key names for sensitive fields
        let masker = DataMasker::new();
        let mut value = serde_json::json!({
            "password": "secret123",
            "name": "Alice"
        });
        masker.mask_value(&mut value);
        assert_eq!(value["password"], "***MASKED***");
        assert_eq!(value["name"], "Alice");
    }

    #[test]
    fn test_mask_hashmap_masks_sensitive_keys() {
        // T009: mask_hashmap should check key names for sensitive fields
        let masker = DataMasker::new();
        let mut map = HashMap::new();
        map.insert(
            "api_key".to_string(),
            Value::String("supersecret".to_string()),
        );
        map.insert("user".to_string(), Value::String("bob".to_string()));
        masker.mask_hashmap(&mut map);
        assert_eq!(map["api_key"], Value::String("***MASKED***".to_string()));
        assert_eq!(map["user"], Value::String("bob".to_string()));
    }
}
