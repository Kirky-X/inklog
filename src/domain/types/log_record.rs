// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::DataMasker;
use crate::{get_log_record, get_string_buffer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{Event, Level};

/// Represents a single log record with all associated metadata.
///
/// `LogRecord` is the core data structure that captures log events from the tracing
/// ecosystem and stores them in a structured format. It includes the log message,
/// timestamp, severity level, source location, and any additional contextual fields.
///
/// # Fields
///
/// - `timestamp`: UTC timestamp when the log record was created
/// - `level`: Log severity level (e.g., "INFO", "ERROR", "DEBUG")
/// - `target`: The target/module path that emitted this log (e.g., "myapp::handlers")
/// - `message`: The primary log message content
/// - `fields`: Additional structured key-value pairs attached to the log
/// - `file`: Source file path where the log was emitted (optional)
/// - `line`: Line number in the source file (optional)
/// - `thread_id`: ID of the thread that emitted this log
///
/// # Performance
///
/// `LogRecord` instances are pooled using the global thread-local pool (see
/// [`crate::get_log_record`]) to reduce memory allocations in the hot path.
/// Use [`reset()`](Self::reset) to reuse instances.
///
/// # Sensitive Data
///
/// The struct includes built-in support for masking sensitive information through
/// [`mask_sensitive_fields()`](Self::mask_sensitive_fields). This should be called
/// by sinks before persisting logs to external storage.
///
/// # Example
///
/// ```
/// use inklog::log_record::LogRecord;
/// use tracing::Level;
///
/// let record = LogRecord::new(
///     Level::INFO,
///     "myapp::service".to_string(),
///     "User logged in successfully".to_string(),
/// );
///
/// assert_eq!(record.level, "INFO");
/// assert_eq!(record.target, "myapp::service");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// UTC timestamp when this log record was created.
    pub timestamp: DateTime<Utc>,

    /// Log severity level (e.g., "INFO", "ERROR", "DEBUG", "WARN", "TRACE").
    pub level: String,

    /// Target module path that emitted this log (e.g., "myapp::handlers::auth").
    pub target: String,

    /// The primary log message content.
    pub message: String,

    /// Additional structured key-value pairs attached to the log event.
    ///
    /// These fields are extracted from tracing span and event attributes,
    /// supporting types like strings, numbers, booleans, and nested JSON.
    pub fields: HashMap<String, Value>,

    /// Source file path where the log was emitted (e.g., "src/handlers.rs").
    pub file: Option<String>,

    /// Line number in the source file where the log was emitted.
    pub line: Option<u32>,

    /// Thread ID where the log was emitted (e.g., "ThreadId(1)").
    pub thread_id: String,

    /// Distributed trace id, extracted from the current tracing span.
    ///
    /// Format: 32-char lowercase hex (W3C Trace Context compatible width).
    /// When no OpenTelemetry layer is installed the value is derived from the
    /// id of the root span of the current span chain; events emitted outside
    /// any span carry `None`.
    #[serde(default)]
    pub trace_id: Option<String>,

    /// Span id of the span the event was emitted in.
    ///
    /// Format: 16-char lowercase hex; `None` when outside any span.
    #[serde(default)]
    pub span_id: Option<String>,
}

impl Default for LogRecord {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: String::new(),
            message: String::new(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: String::new(),
            trace_id: None,
            span_id: None,
        }
    }
}

impl LogRecord {
    /// Resets this log record to default values for reuse.
    ///
    /// This method is used in conjunction with the object pool to recycle
    /// `LogRecord` instances, avoiding memory allocations in the hot path.
    /// After calling `reset()`, the record will have default values suitable
    /// for populating with new log data.
    ///
    /// # Performance
    ///
    /// This method clears all fields and resets the timestamp to the current time.
    /// It uses `clear()` and `push_str()` on String fields to reuse existing capacity.
    ///
    /// # Example
    ///
    /// ```
    /// use inklog::log_record::LogRecord;
    /// use tracing::Level;
    ///
    /// let mut record = LogRecord::new(
    ///     Level::INFO,
    ///     "module".to_string(),
    ///     "message".to_string(),
    /// );
    ///
    /// record.reset();
    ///
    /// assert_eq!(record.level, "INFO");
    /// assert!(record.target.is_empty());
    /// assert!(record.message.is_empty());
    /// ```
    pub fn reset(&mut self) {
        self.timestamp = Utc::now();
        self.level.clear();
        self.level.push_str("INFO");
        self.target.clear();
        self.message.clear();
        self.fields.clear();
        self.file = None;
        self.line = None;
        self.thread_id.clear();
        self.trace_id = None;
        self.span_id = None;
    }

    /// Creates a new log record with the specified level, target, and message.
    ///
    /// This is a convenience constructor for creating log records programmatically.
    /// The timestamp is set to the current UTC time, and the thread ID is captured
    /// automatically. No source location (file/line) is attached.
    ///
    /// # Arguments
    ///
    /// * `level` - The severity level of the log (e.g., `Level::INFO`)
    /// * `target` - The module path or target that emitted this log
    /// * `message` - The primary log message content
    ///
    /// # Example
    ///
    /// ```
    /// use inklog::log_record::LogRecord;
    /// use tracing::Level;
    ///
    /// let record = LogRecord::new(
    ///     Level::WARN,
    ///     "myapp::cache".to_string(),
    ///     "Cache miss for key: user_123".to_string(),
    /// );
    ///
    /// assert_eq!(record.level, "WARN");
    /// assert_eq!(record.target, "myapp::cache");
    /// assert_eq!(record.message, "Cache miss for key: user_123");
    /// assert!(!record.thread_id.is_empty());
    /// ```
    pub fn new(level: Level, target: String, message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            level: level.to_string(),
            target,
            message,
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: format!("{:?}", std::thread::current().id()),
            trace_id: None,
            span_id: None,
        }
    }

    /// Creates a log record from a tracing event.
    ///
    /// This is the primary method for converting tracing events into the internal
    /// `LogRecord` format. It extracts all relevant metadata from the event,
    /// including level, target, message, fields, source location, and thread ID.
    ///
    /// # Performance
    ///
    /// This method uses object pooling for both the `LogRecord` and the message
    /// string to minimize allocations in the hot path. The returned instance
    /// is taken from the global thread-local pool via [`crate::get_log_record`].
    ///
    /// # Sensitive Data
    ///
    /// For performance reasons, this method does **NOT** automatically mask
    /// sensitive fields. Callers must either call
    /// [`mask_sensitive_fields()`](Self::mask_sensitive_fields) themselves
    /// before persisting the record, or use the masking constructor
    /// [`from_event_masked()`](Self::from_event_masked) instead.
    ///
    /// # Arguments
    ///
    /// * `event` - The tracing event to convert
    ///
    /// # Example
    ///
    /// ```ignore
    /// use tracing::Event;
    /// use inklog::log_record::LogRecord;
    ///
    /// fn process_event(event: &Event) {
    ///     let record = LogRecord::from_event(event);
    ///     // Use record...
    /// }
    /// ```
    pub fn from_event(event: &Event) -> Self {
        let mut record = get_log_record();
        record.reset();

        let mut fields = HashMap::with_capacity(4);
        let mut message = get_string_buffer();
        message.clear();

        let mut visitor = LogVisitor {
            fields: &mut fields,
            message: &mut message,
        };
        event.record(&mut visitor);

        let metadata = event.metadata();

        record.level.clear();
        record.level.push_str(metadata.level().as_str());
        record.target.clear();
        record.target.push_str(metadata.target());
        record.message = message;
        record.fields = fields;
        record.file = metadata.file().map(|s| s.to_string());
        record.line = metadata.line();
        record.thread_id = format!("{:?}", std::thread::current().id());

        // NOTE: mask_sensitive_fields() is NOT called here for performance.
        // Callers who need masking (e.g., FileSink, DatabaseSink) should call
        // record.mask_sensitive_fields() in their write() method, or use
        // from_event_masked() instead.
        // ConsoleSink may optionally call it based on configuration.
        record
    }

    /// Creates a log record from a tracing event with sensitive data masked.
    ///
    /// Convenience constructor equivalent to [`from_event()`](Self::from_event)
    /// followed by [`mask_sensitive_fields()`](Self::mask_sensitive_fields):
    /// email addresses, phone numbers, ID/bank card numbers and values of
    /// sensitive field names (password, token, api_key, …) are masked before
    /// the record is returned.
    ///
    /// # Performance
    ///
    /// This method has the object-pooling behavior of [`from_event()`](Self::from_event)
    /// plus the cost of running the masking pipeline (regex matching over the
    /// message and all field values). Use [`from_event()`](Self::from_event) in
    /// hot paths where masking is applied later at the sink layer.
    ///
    /// # Arguments
    ///
    /// * `event` - The tracing event to convert
    ///
    /// # Example
    ///
    /// ```ignore
    /// use tracing::Event;
    /// use inklog::log_record::LogRecord;
    ///
    /// fn process_event(event: &Event) {
    ///     let record = LogRecord::from_event_masked(event);
    ///     // Sensitive fields are already masked...
    /// }
    /// ```
    pub fn from_event_masked(event: &Event) -> Self {
        let mut record = Self::from_event(event);
        record.mask_sensitive_fields();
        record
    }

    /// Sensitive key tokens: a field name is sensitive when one of its
    /// separator-delimited tokens exactly equals one of these
    /// (case-insensitive, camelCase-aware). Substring matches are
    /// intentionally not counted, so "author" does not trigger "auth"
    /// and "authorizer" does not trigger "auth" either.
    ///
    /// Crate-wide single source of truth for sensitive-key judgment: the
    /// subscriber sanitizer path (`LoggerSubscriber`) shares this table via
    /// [`Self::is_sensitive_key`] instead of keeping its own copy.
    pub(crate) const SENSITIVE_KEY_PATTERNS: &[&str] = &[
        "password",
        "passwd",
        "pwd",
        "token",
        "secret",
        "credential",
        "credentials",
        "auth",
        "oauth",
        "authorization",
    ];

    /// Qualifier tokens that make a compound "…key/…keys" field name sensitive
    /// (e.g. `api_key`, `secret-key`, `accessKey`). Generic qualifiers such as
    /// `primary` or `index` are intentionally absent, so `primary_key` and
    /// `index_key` are not masked.
    ///
    /// Crate-wide single source of truth, shared with the subscriber sanitizer
    /// path via [`Self::is_sensitive_key`].
    pub(crate) const SENSITIVE_KEY_QUALIFIERS: &[&str] = &[
        "api",
        "access",
        "secret",
        "private",
        "public",
        "encryption",
        "decryption",
        "master",
        "session",
        "aws",
        "ssh",
        "auth",
    ];

    /// Splits a key into lowercase alphanumeric tokens, treating separators
    /// (`_`, `-`, `.`, spaces, …) and camelCase humps as token boundaries
    /// (`apiKey` → `api`, `key`).
    fn key_tokens(key: &str) -> Vec<String> {
        let chars: Vec<char> = key.chars().collect();
        let mut normalized = String::with_capacity(key.len() + 4);
        for (i, &c) in chars.iter().enumerate() {
            let next = chars.get(i + 1).copied();
            if c.is_ascii_uppercase()
                && i > 0
                && (chars[i - 1].is_ascii_lowercase()
                    || chars[i - 1].is_ascii_digit()
                    || next.is_some_and(|n| n.is_ascii_lowercase()))
            {
                normalized.push('_');
            }
            normalized.extend(c.to_lowercase());
        }
        normalized
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Checks if a field name is sensitive, using token-boundary-aware matching
    /// (case-insensitive, camelCase-aware) instead of substring matching.
    ///
    /// 例如：
    /// - `"auth_token"` 匹配（`auth` 为完整 token）
    /// - `"author"` 不匹配（`auth` 不是完整 token，避免误判）
    /// - `"api_key"` 匹配（敏感限定词 + `key`）
    /// - `"primary_key"` 不匹配（`primary` 不是敏感限定词）
    ///
    /// Crate-wide single source of truth（`pub(crate)`）：所有需要敏感键判定的
    /// 模块（`mask_sensitive_fields`、`LoggerSubscriber` 的 sanitizer 路径等）
    /// 统一引用本实现，禁止再复制本地副本（DRY，防安全行为分叉）。
    pub(crate) fn is_sensitive_key(key: &str) -> bool {
        // 无分隔符的单段键（如 "PASSWORD"、"pAsSwOrD"、"apiKey"）：
        // 驼峰切分会把交替大小写撕碎，先按小写整体比对
        if !key.chars().any(|c| !c.is_ascii_alphanumeric()) {
            let lowered = key.to_lowercase();
            if Self::SENSITIVE_KEY_PATTERNS.contains(&lowered.as_str())
                || lowered == "key"
                || lowered == "keys"
                || Self::is_glued_sensitive_key(&lowered)
            {
                return true;
            }
        }
        let tokens = Self::key_tokens(key);
        if tokens
            .iter()
            .any(|t| Self::SENSITIVE_KEY_PATTERNS.contains(&t.as_str()))
        {
            return true;
        }
        match tokens.as_slice() {
            // 单 token：确切的 "key"/"keys" 视为敏感；粘连形式（如 "apikey"）
            // 仅当去掉 key 后的前缀是敏感限定词时才视为敏感
            [single] => single == "key" || single == "keys" || Self::is_glued_sensitive_key(single),
            // 多 token：仅当 "key"/"keys" 与敏感限定词相邻时才视为敏感
            _ => tokens.windows(2).any(|w| {
                let (a, b) = (w[0].as_str(), w[1].as_str());
                (a == "key" || a == "keys") && Self::SENSITIVE_KEY_QUALIFIERS.contains(&b)
                    || Self::SENSITIVE_KEY_QUALIFIERS.contains(&a) && (b == "key" || b == "keys")
            }),
        }
    }

    /// 粘连形式（无分隔符）的 "…key/…keys" 判定：去掉 key 后缀的前缀必须是
    /// 敏感限定词（"apikey" → "api"，"primarykey" → "primary" 不匹配）
    fn is_glued_sensitive_key(lowered_token: &str) -> bool {
        lowered_token
            .strip_suffix("keys")
            .or_else(|| lowered_token.strip_suffix("key"))
            .is_some_and(|prefix| Self::SENSITIVE_KEY_QUALIFIERS.contains(&prefix))
    }

    /// Masks sensitive information in the log message and fields.
    ///
    /// This method performs in-place masking of sensitive data such as:
    /// - Email addresses (user@example.com → **@**.***)
    /// - Phone numbers (13812345678 → ***-****-****)
    /// - ID card numbers (110101199001011234 → ******1234)
    /// - Bank card numbers (6222021234567890123 → ****-****-****-0123)
    /// - Field names containing sensitive patterns (password, token, secret, etc.)
    ///
    /// Sensitive field names (case-insensitive matching) are replaced with "***MASKED***".
    /// Field values containing PII are masked according to their type.
    ///
    /// # When to Use
    ///
    /// This method should be called by sinks before persisting logs to external
    /// storage (files, databases). It is intentionally NOT called in
    /// [`from_event()`](Self::from_event) for performance reasons, and because
    /// console output may not require masking.
    ///
    /// # Performance
    ///
    /// This method creates a new `DataMasker` instance and processes both the
    /// message and all field values. For high-throughput scenarios, consider
    /// whether masking is needed for every log or only certain log levels.
    ///
    /// # Example
    ///
    /// ```
    /// use inklog::log_record::LogRecord;
    /// use tracing::Level;
    /// use serde_json::Value;
    ///
    /// let mut record = LogRecord::new(
    ///     Level::INFO,
    ///     "auth".to_string(),
    ///     "Login attempt: user@example.com".to_string(),
    /// );
    /// record.fields.insert(
    ///     "password".to_string(),
    ///     Value::String("secret123".to_string()),
    /// );
    ///
    /// record.mask_sensitive_fields();
    ///
    /// assert_eq!(record.message, "Login attempt: **@**.***");
    /// assert_eq!(
    ///     record.fields.get("password").unwrap(),
    ///     &Value::String("***MASKED***".to_string())
    /// );
    /// ```
    pub fn mask_sensitive_fields(&mut self) {
        let masker = DataMasker::new();
        self.message = masker.mask(&self.message);
        for v in self.fields.values_mut() {
            masker.mask_value(v);
        }
        for (k, v) in self.fields.iter_mut() {
            if Self::is_sensitive_key(k) {
                *v = Value::String("***MASKED***".to_string());
            }
        }
    }
}

struct LogVisitor<'a> {
    fields: &'a mut HashMap<String, Value>,
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for LogVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        if name == "message" {
            *self.message = format!("{:?}", value);
        } else {
            self.fields
                .insert(name.to_string(), Value::String(format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let name = field.name();
        if name == "message" {
            *self.message = value.to_string();
        } else {
            self.fields
                .insert(name.to_string(), Value::String(value.to_string()));
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), Value::Number(n));
        } else {
            // NaN / Infinity cannot be represented in JSON.
            // Store as a string sentinel so the field is not silently dropped,
            // which would create observability gaps in production.
            let sentinel = if value.is_nan() {
                "NaN"
            } else if value.is_infinite() && value.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            };
            // 此处处于事件 record() 访问回调内：再发 tracing 事件会对正在
            // 遍历的订阅者链重入（第三方层若在 on_event 中持锁处理事件，
            // 重入可能死锁），故用标准错误报告而非 tracing。
            eprintln!(
                "inklog: field '{}' is non-finite ({sentinel}), stored as string sentinel",
                field.name()
            );
            self.fields.insert(
                field.name().to_string(),
                Value::String(sentinel.to_string()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "password".to_string(),
            Value::String("secret123".to_string()),
        );
        record
            .fields
            .insert("api_key".to_string(), Value::String("abcdef".to_string()));
        record
            .fields
            .insert("username".to_string(), Value::String("user".to_string()));

        record.mask_sensitive_fields();

        assert_eq!(
            record.fields.get("password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("api_key").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("username").unwrap(),
            &Value::String("user".to_string())
        );
    }

    #[test]
    fn test_is_sensitive_key_token_boundaries() {
        // "author" 含子串 "auth" 但 token 边界不匹配，不应脱敏
        assert!(!LogRecord::is_sensitive_key("author"));
        assert!(!LogRecord::is_sensitive_key("authorizer"));
        assert!(LogRecord::is_sensitive_key("auth"));
        assert!(LogRecord::is_sensitive_key("auth_token"));
        assert!(LogRecord::is_sensitive_key("api_key"));
        // primary_key / index_key 属数据库索引语义，不再脱敏
        assert!(!LogRecord::is_sensitive_key("primary_key"));
        assert!(!LogRecord::is_sensitive_key("index_key"));
        assert!(LogRecord::is_sensitive_key("password"));
        assert!(LogRecord::is_sensitive_key("token"));
    }

    #[test]
    fn test_is_sensitive_key_compound_and_camel_case() {
        assert!(LogRecord::is_sensitive_key("secret_key"));
        assert!(LogRecord::is_sensitive_key("access_key_id"));
        assert!(LogRecord::is_sensitive_key("private-key"));
        // camelCase 归一化为 token 后仍可识别
        assert!(LogRecord::is_sensitive_key("apiKey"));
        assert!(LogRecord::is_sensitive_key("secretKey"));
        assert!(LogRecord::is_sensitive_key("AUTHORIZATION"));
        // 确切 token "key"/"keys" 视为敏感
        assert!(LogRecord::is_sensitive_key("key"));
        assert!(LogRecord::is_sensitive_key("keys"));
        // 非敏感组合词
        assert!(!LogRecord::is_sensitive_key("hotkey"));
        assert!(!LogRecord::is_sensitive_key("keyspace"));
        assert!(!LogRecord::is_sensitive_key("monkeys"));
    }

    #[test]
    fn test_mask_sensitive_fields_respects_token_boundaries() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record
            .fields
            .insert("author".to_string(), Value::String("Alice".to_string()));
        record
            .fields
            .insert("primary_key".to_string(), Value::Number(1.into()));
        record.fields.insert(
            "auth_token".to_string(),
            Value::String("bearer-value".to_string()),
        );

        record.mask_sensitive_fields();

        assert_eq!(
            record.fields.get("author").unwrap(),
            &Value::String("Alice".to_string())
        );
        assert_eq!(
            record.fields.get("primary_key").unwrap(),
            &Value::Number(1.into())
        );
        assert_eq!(
            record.fields.get("auth_token").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
    }

    #[test]
    fn test_mask_email_in_message() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "Contact: user@example.com".to_string(),
        );
        record.mask_sensitive_fields();
        assert_eq!(record.message, "Contact: **@**.***");
    }

    #[test]
    fn test_mask_phone_in_message() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "Call: 13812345678".to_string(),
        );
        record.mask_sensitive_fields();
        assert_eq!(record.message, "Call: ***-****-****");
    }

    #[test]
    fn test_mask_id_card_in_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "id_card".to_string(),
            Value::String("110101199001011234".to_string()),
        );
        record.mask_sensitive_fields();
        assert_eq!(
            record.fields.get("id_card").unwrap(),
            &Value::String("******1234".to_string())
        );
    }

    #[test]
    fn test_mask_bank_card_in_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "card_number".to_string(),
            Value::String("6222021234567890123".to_string()),
        );
        record.mask_sensitive_fields();
        assert_eq!(
            record.fields.get("card_number").unwrap(),
            &Value::String("****-****-****-0123".to_string())
        );
    }

    #[test]
    fn test_mask_nested_json_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "user_info".to_string(),
            Value::Object(serde_json::from_str(r#"{"email":"admin@test.com"}"#).unwrap()),
        );
        record.mask_sensitive_fields();
        let user_info = record.fields.get("user_info").unwrap();
        assert_eq!(user_info["email"], Value::String("**@**.***".to_string()));
    }

    #[test]
    fn test_mask_array_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "contacts".to_string(),
            Value::Array(vec![
                Value::String("test@email.com".to_string()),
                Value::String("13912345678".to_string()),
            ]),
        );
        record.mask_sensitive_fields();
        let contacts = record.fields.get("contacts").unwrap().as_array().unwrap();
        assert_eq!(contacts[0], Value::String("**@**.***".to_string()));
        assert_eq!(contacts[1], Value::String("***-****-****".to_string()));
    }

    #[test]
    fn test_combined_masking() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "User test@example.com called 13812345678".to_string(),
        );
        record.fields.insert(
            "id_card".to_string(),
            Value::String("110101199001011234".to_string()),
        );
        record.fields.insert(
            "password".to_string(),
            Value::String("mypass123".to_string()),
        );

        record.mask_sensitive_fields();

        assert_eq!(record.message, "User **@**.*** called ***-****-****");
        assert_eq!(
            record.fields.get("id_card").unwrap(),
            &Value::String("******1234".to_string())
        );
        assert_eq!(
            record.fields.get("password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
    }

    // === LogRecord Basic Tests ===

    #[test]
    fn test_log_record_default() {
        let record = LogRecord::default();
        assert_eq!(record.level, "INFO");
        assert!(record.target.is_empty());
        assert!(record.message.is_empty());
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
    }

    #[test]
    fn test_log_record_new() {
        let record = LogRecord::new(
            Level::DEBUG,
            "my_target".to_string(),
            "test message".to_string(),
        );
        assert_eq!(record.level, "DEBUG");
        assert_eq!(record.target, "my_target");
        assert_eq!(record.message, "test message");
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
        assert!(!record.thread_id.is_empty());
    }

    #[test]
    fn test_log_record_reset() {
        let mut record = LogRecord::new(Level::INFO, "target".to_string(), "message".to_string());
        record
            .fields
            .insert("key".to_string(), Value::String("value".to_string()));
        record.file = Some("test.rs".to_string());
        record.line = Some(42);

        record.reset();

        assert_eq!(record.level, "INFO");
        assert!(record.target.is_empty());
        assert!(record.message.is_empty());
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
    }

    #[test]
    fn test_log_record_clone() {
        let record = LogRecord::new(
            Level::WARN,
            "clone_test".to_string(),
            "original".to_string(),
        );
        let cloned = record.clone();

        assert_eq!(cloned.level, record.level);
        assert_eq!(cloned.target, record.target);
        assert_eq!(cloned.message, record.message);
        assert_eq!(cloned.timestamp, record.timestamp);
    }

    #[test]
    fn test_log_record_timestamp() {
        use chrono::Utc;

        let before = Utc::now();
        let record = LogRecord::new(Level::INFO, "test".to_string(), "test".to_string());
        let after = Utc::now();

        assert!(record.timestamp >= before);
        assert!(record.timestamp <= after);
    }

    #[test]
    fn test_log_record_thread_id_format() {
        let record = LogRecord::new(Level::INFO, "test".to_string(), "test".to_string());
        // ThreadId format varies by platform but should not be empty
        assert!(!record.thread_id.is_empty());
    }

    // === LogRecord Field Tests ===

    #[test]
    fn test_log_record_with_string_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record
            .fields
            .insert("username".to_string(), Value::String("john".to_string()));
        assert_eq!(record.fields["username"], Value::String("john".to_string()));
    }

    #[test]
    fn test_log_record_with_number_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "count".to_string(),
            Value::Number(serde_json::Number::from(42)),
        );
        assert_eq!(
            record.fields["count"],
            Value::Number(serde_json::Number::from(42))
        );
    }

    #[test]
    fn test_log_record_with_boolean_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record
            .fields
            .insert("active".to_string(), Value::Bool(true));
        assert_eq!(record.fields["active"], Value::Bool(true));
    }

    #[test]
    fn test_log_record_with_null_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert("optional".to_string(), Value::Null);
        assert_eq!(record.fields["optional"], Value::Null);
    }

    #[test]
    fn test_log_record_with_object_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        let obj = Value::Object(serde_json::from_str(r#"{"key":"value"}"#).unwrap());
        record.fields.insert("data".to_string(), obj);
        assert_eq!(
            record.fields["data"]["key"],
            Value::String("value".to_string())
        );
    }

    // === Masking Edge Cases ===

    #[test]
    fn test_mask_empty_message() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "".to_string());
        record.mask_sensitive_fields();
        assert_eq!(record.message, "");
    }

    #[test]
    fn test_mask_no_sensitive_data() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "Hello world".to_string());
        record
            .fields
            .insert("name".to_string(), Value::String("Alice".to_string()));
        record.mask_sensitive_fields();
        assert_eq!(record.message, "Hello world");
        assert_eq!(record.fields["name"], Value::String("Alice".to_string()));
    }

    #[test]
    fn test_mask_case_insensitive_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record
            .fields
            .insert("PASSWORD".to_string(), Value::String("secret".to_string()));
        record
            .fields
            .insert("Password".to_string(), Value::String("secret".to_string()));
        record
            .fields
            .insert("pAsSwOrD".to_string(), Value::String("secret".to_string()));

        record.mask_sensitive_fields();

        assert_eq!(
            record.fields.get("PASSWORD").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("Password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("pAsSwOrD").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
    }

    #[test]
    fn test_mask_email_in_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "email".to_string(),
            Value::String("user@example.org".to_string()),
        );
        record.mask_sensitive_fields();
        assert_eq!(
            record.fields.get("email").unwrap(),
            &Value::String("**@**.***".to_string())
        );
    }

    #[test]
    fn test_mask_phone_in_field() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "phone".to_string(),
            Value::String("13812345678".to_string()),
        );
        record.mask_sensitive_fields();
        // Phone in field is masked by DataMasker.mask_value
        let masked = record.fields.get("phone").unwrap().as_str().unwrap();
        assert!(
            masked.contains("*") || masked.contains("***"),
            "Phone should be masked: {}",
            masked
        );
    }

    #[test]
    fn test_mask_deeply_nested() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        let nested = serde_json::json!({
            "level1": {
                "level2": {
                    "email": "deep@example.com"
                }
            }
        });
        record.fields.insert("data".to_string(), nested);

        record.mask_sensitive_fields();

        // Verify the nested structure still exists
        assert!(record.fields.contains_key("data"));
    }

    #[test]
    fn test_mask_array_of_objects() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        let users = serde_json::json!([
            {"name": "Alice", "email": "alice@test.com"},
            {"name": "Bob", "email": "bob@test.com"}
        ]);
        record.fields.insert("users".to_string(), users);

        record.mask_sensitive_fields();

        let users = record.fields.get("users").unwrap().as_array().unwrap();
        assert_eq!(users[0]["email"], Value::String("**@**.***".to_string()));
        assert_eq!(users[1]["email"], Value::String("**@**.***".to_string()));
    }

    #[test]
    fn test_mask_multiple_phone_formats() {
        // Test standard 11-digit Chinese mobile numbers
        let test_cases = vec![
            ("13812345678", "***-****-****"),
            ("15987654321", "***-****-****"),
        ];

        for (input, expected) in test_cases {
            let mut record = LogRecord::new(Level::INFO, "test".to_string(), input.to_string());
            record.mask_sensitive_fields();
            assert_eq!(record.message, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_mask_multiple_email_formats() {
        let test_cases = vec![
            ("user@example.com", "**@**.***"),
            ("user.name@example.com", "**@**.***"),
            ("admin@sub.domain.example.org", "**@**.***"),
        ];

        for (input, expected) in test_cases {
            let mut record = LogRecord::new(
                Level::INFO,
                "test".to_string(),
                format!("Contact: {}", input),
            );
            record.mask_sensitive_fields();
            assert!(
                record.message.contains(expected),
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_mask_id_card_formats() {
        let test_cases = vec![
            ("110101199001011234", "******1234"),
            ("310105199001012345", "******2345"),
        ];

        for (input, expected) in test_cases {
            let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
            record
                .fields
                .insert("id_card".to_string(), Value::String(input.to_string()));
            record.mask_sensitive_fields();
            assert_eq!(
                record.fields.get("id_card").unwrap(),
                &Value::String(expected.to_string()),
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_mask_bank_card_formats() {
        let test_cases = vec![
            ("6222021234567890123", "****-****-****-0123"),
            ("6222021234567890", "****-****-****-7890"),
        ];

        for (input, expected) in test_cases {
            let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
            record
                .fields
                .insert("card".to_string(), Value::String(input.to_string()));
            record.mask_sensitive_fields();
            assert_eq!(
                record.fields.get("card").unwrap(),
                &Value::String(expected.to_string()),
                "Failed for input: {}",
                input
            );
        }
    }

    // === LogVisitor record_f64 测试 ===
    // 通过 tracing 事件携带 f64 字段，验证 LogRecord::from_event 正确记录 f64 值

    #[test]
    fn test_log_record_from_event_with_f64_field() {
        use std::sync::{Arc, Mutex};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::Context;
        use tracing_subscriber::prelude::*;

        // 自定义捕获层：记录 on_event 产生的 LogRecord
        struct CaptureLayer(Arc<Mutex<Option<LogRecord>>>);

        impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let record = LogRecord::from_event(event);
                *self.0.lock().unwrap() = Some(record);
            }
        }

        let captured: Arc<Mutex<Option<LogRecord>>> = Arc::new(Mutex::new(None));
        let layer = CaptureLayer(captured.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // 发送带有 f64 字段的 tracing 事件
        with_default(registry, || {
            tracing::info!(
                target: "test::f64",
                message = "f64 field test",
                ratio = std::f64::consts::PI,
                count = 42i64,
            );
        });

        let record = captured
            .lock()
            .unwrap()
            .take()
            .expect("should capture record");
        assert_eq!(record.message, "f64 field test");
        assert_eq!(record.target, "test::f64");

        // 验证 f64 字段被正确记录为 Number
        let ratio = record
            .fields
            .get("ratio")
            .expect("ratio field should exist");
        assert_eq!(ratio.as_f64(), Some(std::f64::consts::PI));

        // 验证 i64 字段也被正确记录
        let count = record
            .fields
            .get("count")
            .expect("count field should exist");
        assert_eq!(count.as_i64(), Some(42));
    }

    #[test]
    fn test_log_record_from_event_with_multiple_f64_fields() {
        use std::sync::{Arc, Mutex};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::Context;
        use tracing_subscriber::prelude::*;

        struct CaptureLayer(Arc<Mutex<Option<LogRecord>>>);

        impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let record = LogRecord::from_event(event);
                *self.0.lock().unwrap() = Some(record);
            }
        }

        let captured: Arc<Mutex<Option<LogRecord>>> = Arc::new(Mutex::new(None));
        let layer = CaptureLayer(captured.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(
                target: "test::multi_f64",
                message = "multiple f64 fields",
                temperature = 36.5f64,
                humidity = 0.75f64,
            );
        });

        let record = captured
            .lock()
            .unwrap()
            .take()
            .expect("should capture record");
        assert_eq!(record.level, "WARN");

        let temp = record
            .fields
            .get("temperature")
            .expect("temperature should exist");
        assert_eq!(temp.as_f64(), Some(36.5));

        let humidity = record
            .fields
            .get("humidity")
            .expect("humidity should exist");
        assert_eq!(humidity.as_f64(), Some(0.75));
    }

    #[test]
    fn test_log_record_from_event_with_nan_f64() {
        use std::sync::{Arc, Mutex};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::Context;
        use tracing_subscriber::prelude::*;

        struct CaptureLayer(Arc<Mutex<Option<LogRecord>>>);

        impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let record = LogRecord::from_event(event);
                *self.0.lock().unwrap() = Some(record);
            }
        }

        let captured: Arc<Mutex<Option<LogRecord>>> = Arc::new(Mutex::new(None));
        let layer = CaptureLayer(captured.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(
                target: "test::nan",
                message = "nan test",
                value = f64::NAN,
            );
        });

        let record = captured
            .lock()
            .unwrap()
            .take()
            .expect("should capture record");
        let val = record
            .fields
            .get("value")
            .expect("value field should exist");
        // NaN should be stored as string sentinel
        assert_eq!(val.as_str(), Some("NaN"));
    }

    #[test]
    fn test_log_record_from_event_with_infinity_f64() {
        use std::sync::{Arc, Mutex};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::Context;
        use tracing_subscriber::prelude::*;

        struct CaptureLayer(Arc<Mutex<Option<LogRecord>>>);

        impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let record = LogRecord::from_event(event);
                *self.0.lock().unwrap() = Some(record);
            }
        }

        let captured: Arc<Mutex<Option<LogRecord>>> = Arc::new(Mutex::new(None));
        let layer = CaptureLayer(captured.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(
                target: "test::inf",
                message = "inf test",
                pos_inf = f64::INFINITY,
                neg_inf = f64::NEG_INFINITY,
            );
        });

        let record = captured
            .lock()
            .unwrap()
            .take()
            .expect("should capture record");
        let pos = record
            .fields
            .get("pos_inf")
            .expect("pos_inf field should exist");
        assert_eq!(pos.as_str(), Some("Infinity"));

        let neg = record
            .fields
            .get("neg_inf")
            .expect("neg_inf field should exist");
        assert_eq!(neg.as_str(), Some("-Infinity"));
    }

    #[test]
    fn test_log_record_from_event_masked() {
        use std::sync::{Arc, Mutex};
        use tracing::subscriber::with_default;
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::Context;
        use tracing_subscriber::prelude::*;

        struct CaptureMaskedLayer(Arc<Mutex<Option<LogRecord>>>);

        impl<S: tracing::Subscriber> Layer<S> for CaptureMaskedLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let record = LogRecord::from_event_masked(event);
                *self.0.lock().unwrap() = Some(record);
            }
        }

        let captured: Arc<Mutex<Option<LogRecord>>> = Arc::new(Mutex::new(None));
        let layer = CaptureMaskedLayer(captured.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(
                target: "test::masked",
                message = "Contact: user@example.com",
                password = "secret-value-1",
            );
        });

        let record = captured
            .lock()
            .unwrap()
            .take()
            .expect("should capture record");
        // message 中的 PII 已被掩码
        assert_eq!(record.message, "Contact: **@**.***");
        // 敏感字段名对应的值已被替换为 ***MASKED***
        assert_eq!(
            record.fields.get("password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
    }
}

// ============================================================================
// trace_id/span_id 字段与序列化兼容
// ============================================================================

#[cfg(test)]
mod trace_id_tests {
    use super::*;

    #[test]
    fn test_deserialize_old_json_without_trace_fields() {
        // 旧版本序列化的记录（无 trace_id/span_id）必须可反序列化（serde default）
        let old = r#"{"timestamp":"2026-01-01T00:00:00Z","level":"INFO","target":"a","message":"m","fields":{},"file":null,"line":null,"thread_id":"ThreadId(1)"}"#;
        let record: LogRecord = serde_json::from_str(old).expect("old JSON must deserialize");
        assert!(record.trace_id.is_none());
        assert!(record.span_id.is_none());
    }

    #[test]
    fn test_trace_fields_serialize_roundtrip() {
        let mut record = LogRecord::new(Level::INFO, "t".to_string(), "m".to_string());
        record.trace_id = Some("a".repeat(32));
        record.span_id = Some("b".repeat(16));
        let json = serde_json::to_string(&record).unwrap();
        let back: LogRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.trace_id.as_deref(),
            Some(record.trace_id.as_deref().unwrap())
        );
        assert_eq!(
            back.span_id.as_deref(),
            Some(record.span_id.as_deref().unwrap())
        );
    }

    #[test]
    fn test_new_and_reset_clear_trace_fields() {
        let mut record = LogRecord::new(Level::INFO, "t".to_string(), "m".to_string());
        record.trace_id = Some("x".to_string());
        record.span_id = Some("y".to_string());
        record.reset();
        assert!(record.trace_id.is_none() && record.span_id.is_none());
        let fresh = LogRecord::new(Level::INFO, "t".to_string(), "m".to_string());
        assert!(fresh.trace_id.is_none() && fresh.span_id.is_none());
    }
}
