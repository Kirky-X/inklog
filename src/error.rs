// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 错误类型模块
//!
//! 定义 Inklog 项目中使用的所有错误类型。
//!
//! ## 概述
//!
//! 使用 `thiserror` 派生实现的错误枚举，提供类型安全且用户友好的错误消息。
//!
//! ## 错误类型
//!
//! | 变体 | 描述 |
//! |------|------|
//! | `ConfigError` | 配置相关错误 |
//! | `IoError` | I/O 操作错误 |
//! | `SerializationError` | JSON/TOML 序列化错误 |
//! | `DatabaseError` | 数据库操作错误 |
//! | `EncryptionError` | 加密/解密错误 |
//! | `Shutdown` | 关闭过程中的错误 |
//! | `ChannelError` | 通道通信错误 |
//! | `CompressionError` | 压缩/解压错误 |
//! | `RuntimeError` | 运行时错误 |
//! | `HttpServerError` | HTTP 服务器错误 |
//! | `Unknown` | 未知错误 |
//!
//! ## 使用示例
//!
//! ```rust
//! use inklog::InklogError;
//!
//! fn example() -> Result<(), InklogError> {
//!     // 配置错误
//!     Err(InklogError::ConfigError("Invalid log level".to_string()))
//! }
//!
//! // 使用 ? 操作符传播错误
//! fn read_config() -> Result<(), InklogError> {
//!     let content = std::fs::read_to_string("config.toml")?;
//!     Ok(())
//! }
//! ```

use std::sync::LazyLock;
use thiserror::Error;

/// Sensitive pattern redaction rules for error messages.
/// Each tuple contains (pattern, replacement).
const SENSITIVE_PATTERNS: &[(&str, &str)] = &[
    // AWS Access Key ID pattern (20 characters, starts with AKIA, ABIA, ACCA, ASIA)
    (
        "(?i)(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\\b",
        "[AWS_ACCESS_KEY_ID]",
    ),
    // AWS Secret Key pattern (40 characters, base64-like with word boundary)
    // Tightened to require adjacent AWS context to reduce false positives on generic 40-char tokens.
    (
        "(?i)(?:aws[_-]?(?:secret[_-]?access[_-]?key|secret)[\"'\\s:=]+)[0-9a-zA-Z+/]{40}={0,2}\\b",
        "[AWS_SECRET_ACCESS_KEY]",
    ),
    // JWT Token pattern (with word boundaries)
    (
        "\\beyJ[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]+\\b",
        "[JWT_TOKEN]",
    ),
    // Database connection strings (postgres, mysql, sqlite)
    ("(?i)(postgres|postgresql)://[^@]+:[^@]+@", "$1://***:***@"),
    ("(?i)mysql://[^@]+:[^@]+@", "mysql://***:***@"),
    // SQLite connection strings (matches both path-based and query-parameter URIs)
    ("(?i)sqlite://[^\\s]+", "sqlite://***"),
    // API keys (generic pattern)
    (
        "(?i)(api[_-]?key|access[_-]?key|secret[_-]?key)[\"']?\\s*[=:]\\s*[\"']?[a-zA-Z0-9_\\-]{20,}",
        "$1=***REDACTED***",
    ),
    // Bearer tokens
    (
        "(?i)(bearer|authorization)\\s*:\\s*[a-zA-Z0-9_\\-\\.]+",
        "$1: ***REDACTED***",
    ),
    // Sensitive paths (cross-platform)
    ("/home/[a-zA-Z0-9_-]+/", "[USER_HOME_PATH]"),
    ("/Users/[a-zA-Z0-9_-]+/", "[USER_HOME_PATH]"),
    (
        "(?i)[A-Z]:[/\\\\]Users[/\\\\][a-zA-Z0-9_-]+[/\\\\]",
        "[USER_HOME_PATH]",
    ),
    ("/etc/inklog/", "[CONFIG_PATH]"),
    ("/run/secrets/", "[SECRETS_PATH]"),
    // Passwords in URLs
    ("(?i)(password|passwd|pwd)=[^&\\s]+", "$1=***"),
    // Email addresses
    (
        "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}",
        "***@***.***",
    ),
    // Phone numbers (Chinese)
    ("\\b1[3-9]\\d{9}\\b", "***-****-****"),
    // Credit card numbers (basic pattern)
    (
        "\\b\\d{4}[ -]?\\d{4}[ -]?\\d{4}[ -]?\\d{4}\\b",
        "****-****-****-****",
    ),
];

/// Pre-compiled regex patterns for efficient repeated sanitization.
/// Compiled once on first access via `LazyLock` instead of on every call.
static COMPILED_PATTERNS: LazyLock<Vec<(regex::Regex, &'static str)>> = LazyLock::new(|| {
    SENSITIVE_PATTERNS
        .iter()
        .filter_map(|(pattern, replacement)| match regex::Regex::new(pattern) {
            Ok(re) => Some((re, *replacement)),
            Err(e) => {
                // Use eprintln during static init (tracing may not be set up yet)
                // but prefix with clear warning marker
                eprintln!(
                    "[inklog] WARNING: failed to compile sensitive pattern '{}': {}",
                    pattern, e
                );
                None
            }
        })
        .collect()
});

/// Sanitizes a message by removing sensitive information.
/// Uses pre-compiled regex patterns for optimal performance under high-frequency logging.
/// Returns a borrowed reference when no patterns match (zero-allocation fast path).
fn sanitize_message(msg: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    // Fast path: check if any pattern matches before allocating
    let mut has_match = false;
    for (re, _) in COMPILED_PATTERNS.iter() {
        if re.is_match(msg) {
            has_match = true;
            break;
        }
    }
    if !has_match {
        return Cow::Borrowed(msg);
    }
    // Slow path: apply all replacements
    let mut result = msg.to_string();
    for (re, replacement) in COMPILED_PATTERNS.iter() {
        result = re.replace_all(&result, *replacement).into_owned();
    }
    Cow::Owned(result)
}

#[derive(Error, Debug)]
pub enum InklogError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {message}")]
    DatabaseError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Encryption error: {message}")]
    EncryptionError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("HTTP server error: {0}")]
    HttpServerError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<toml::de::Error> for InklogError {
    fn from(err: toml::de::Error) -> Self {
        InklogError::ConfigError(err.to_string())
    }
}

impl InklogError {
    /// Create a `DatabaseError` with a message only (no source chain).
    pub fn database_error(message: impl Into<String>) -> Self {
        InklogError::DatabaseError {
            message: message.into(),
            source: None,
        }
    }

    /// Create a `DatabaseError` with a message and source error chain.
    pub fn database_error_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        InklogError::DatabaseError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an `EncryptionError` with a message only (no source chain).
    pub fn encryption_error(message: impl Into<String>) -> Self {
        InklogError::EncryptionError {
            message: message.into(),
            source: None,
        }
    }

    /// Create an `EncryptionError` with a message and source error chain.
    pub fn encryption_error_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        InklogError::EncryptionError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Returns a sanitized error message that does not contain sensitive information.
    ///
    /// This method is useful for logging and displaying errors to users
    /// where sensitive data (like passwords, keys, paths) should not be exposed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inklog::InklogError;
    ///
    /// let error = InklogError::ConfigError(
    ///     "Failed to load AKIA1234567890EXAMPLE from /home/user/.aws/credentials".to_string()
    /// );
    /// let safe = error.safe_message();
    /// // Returns: "Configuration error: Failed to load [AWS_ACCESS_KEY_ID] from [USER_HOME_PATH]/.aws/credentials"
    /// ```
    pub fn safe_message(&self) -> String {
        // Note: source error chains (e.g. DatabaseError.source) are intentionally
        // NOT included in safe_message output. Only the top-level message field
        // is sanitized and displayed. This prevents sensitive data in wrapped
        // errors from leaking through the error chain.
        match self {
            InklogError::ConfigError(msg) => {
                format!("Configuration error: {}", sanitize_message(msg))
            }
            InklogError::IoError(e) => {
                format!("IO error: {}", sanitize_message(&e.to_string()))
            }
            InklogError::SerializationError(e) => {
                format!("Serialization error: {}", sanitize_message(&e.to_string()))
            }
            InklogError::DatabaseError { message, .. } => {
                format!("Database error: {}", sanitize_message(message))
            }
            InklogError::CacheError(msg) => {
                format!("Cache error: {}", sanitize_message(msg))
            }
            InklogError::EncryptionError { message, .. } => {
                format!("Encryption error: {}", sanitize_message(message))
            }
            InklogError::Shutdown(msg) => {
                format!("Shutdown error: {}", sanitize_message(msg))
            }
            InklogError::ChannelError(msg) => {
                format!("Channel error: {}", sanitize_message(msg))
            }
            InklogError::CompressionError(msg) => {
                format!("Compression error: {}", sanitize_message(msg))
            }
            InklogError::RuntimeError(msg) => {
                format!("Runtime error: {}", sanitize_message(msg))
            }
            InklogError::HttpServerError(msg) => {
                format!("HTTP server error: {}", sanitize_message(msg))
            }
            InklogError::Unknown(msg) => {
                format!("Unknown error: {}", sanitize_message(msg))
            }
        }
    }
}

/// Convenience `Result` type alias using [`InklogError`] as the error type.
pub type InklogResult<T> = std::result::Result<T, InklogError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_message_redacts_aws_keys() {
        let error = InklogError::ConfigError(
            "Failed to load AKIAIOSFODNN7EXAMPLE from credentials".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[AWS_ACCESS_KEY_ID]") || msg.contains("***"),
            "Message: {}",
            msg
        );
        assert!(!msg.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_safe_message_redacts_jwt_tokens() {
        let error = InklogError::ConfigError(
            "prefix.eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.suffix".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[JWT_TOKEN]") || msg.contains("***"),
            "Message: {}",
            msg
        );
    }

    #[test]
    fn test_safe_message_redacts_database_urls() {
        let error = InklogError::ConfigError(
            "Connection failed: postgres://user:secret@localhost:5432/db".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("***") || !msg.contains("secret"),
            "Message: {}",
            msg
        );
    }

    #[test]
    fn test_safe_message_redacts_user_paths() {
        let error = InklogError::ConfigError(
            "Config not found at /home/user/.config/inklog.yaml".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[USER_HOME_PATH]") || msg.contains("***"),
            "Message: {}",
            msg
        );
        assert!(!msg.contains("/home/user/"));
    }

    #[test]
    fn test_safe_message_redacts_macos_user_paths() {
        let error = InklogError::ConfigError(
            "Config not found at /Users/john/.config/inklog.yaml".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[USER_HOME_PATH]"),
            "macOS path should be redacted. Message: {}",
            msg
        );
        assert!(!msg.contains("/Users/john/"));
    }

    #[test]
    fn test_safe_message_redacts_windows_user_paths() {
        let error = InklogError::ConfigError(
            "Config not found at C:\\Users\\Admin\\.config\\inklog.yaml".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[USER_HOME_PATH]"),
            "Windows path should be redacted. Message: {}",
            msg
        );
        assert!(!msg.contains("C:\\Users\\Admin\\"));
    }

    #[test]
    fn test_safe_message_redacts_windows_forward_slash_paths() {
        let error = InklogError::ConfigError(
            "Config not found at C:/Users/Admin/.config/inklog.yaml".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("[USER_HOME_PATH]"),
            "Windows forward-slash path should be redacted. Message: {}",
            msg
        );
        assert!(!msg.contains("C:/Users/Admin/"));
    }

    #[test]
    fn test_safe_message_redacts_bearer_tokens() {
        let error = InklogError::HttpServerError(
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0".to_string(),
        );
        let msg = error.safe_message();
        assert!(
            msg.contains("REDACTED") || msg.contains("***"),
            "Message: {}",
            msg
        );
    }

    #[test]
    fn test_safe_message_preserves_non_sensitive() {
        let error = InklogError::ConfigError("Configuration file not found".to_string());
        let msg = error.safe_message();
        assert!(msg.contains("Configuration file not found"));
    }

    #[test]
    fn test_safe_message_redacts_passwords() {
        let error =
            InklogError::ConfigError("Failed to connect: password=mysecretpassword".to_string());
        let msg = error.safe_message();
        assert!(
            !msg.contains("mysecretpassword") || msg.contains("***"),
            "Message: {}",
            msg
        );
    }

    #[test]
    fn test_safe_message_all_variants() {
        // 验证所有错误变体的 safe_message() 都返回正确前缀
        assert!(
            InklogError::ConfigError("x".into())
                .safe_message()
                .contains("Configuration error:")
        );
        assert!(
            InklogError::DatabaseError {
                message: "x".into(),
                source: None
            }
            .safe_message()
            .contains("Database error:")
        );
        assert!(
            InklogError::CacheError("x".into())
                .safe_message()
                .contains("Cache error:")
        );
        assert!(
            InklogError::EncryptionError {
                message: "x".into(),
                source: None
            }
            .safe_message()
            .contains("Encryption error:")
        );
        assert!(
            InklogError::Shutdown("x".into())
                .safe_message()
                .contains("Shutdown error:")
        );
        assert!(
            InklogError::ChannelError("x".into())
                .safe_message()
                .contains("Channel error:")
        );
        assert!(
            InklogError::CompressionError("x".into())
                .safe_message()
                .contains("Compression error:")
        );
        assert!(
            InklogError::RuntimeError("x".into())
                .safe_message()
                .contains("Runtime error:")
        );
        assert!(
            InklogError::Unknown("x".into())
                .safe_message()
                .contains("Unknown error:")
        );
    }

    #[test]
    fn test_safe_message_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = InklogError::IoError(io_err);
        let msg = error.safe_message();
        assert!(msg.contains("IO error:"));
        assert!(msg.contains("file missing"));
    }

    #[test]
    fn test_safe_message_serialization_error() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let error = InklogError::SerializationError(json_err);
        let msg = error.safe_message();
        assert!(msg.contains("Serialization error:"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let inklog_err: InklogError = io_err.into();
        assert!(matches!(inklog_err, InklogError::IoError(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let inklog_err: InklogError = json_err.into();
        assert!(matches!(inklog_err, InklogError::SerializationError(_)));
    }

    #[test]
    fn test_from_toml_de_error() {
        let toml_err: toml::de::Error =
            toml::from_str::<toml::Value>("invalid = = toml").unwrap_err();
        let inklog_err: InklogError = toml_err.into();
        assert!(matches!(inklog_err, InklogError::ConfigError(_)));
    }

    #[test]
    fn test_safe_message_redacts_email() {
        let error = InklogError::ConfigError("Contact admin@example.com for help".to_string());
        let msg = error.safe_message();
        assert!(!msg.contains("admin@example.com"));
    }

    #[test]
    fn test_safe_message_redacts_phone() {
        let error = InklogError::ConfigError("Call 13812345678 for support".to_string());
        let msg = error.safe_message();
        assert!(!msg.contains("13812345678"));
    }

    #[test]
    fn test_safe_message_redacts_credit_card() {
        let error = InklogError::ConfigError("Card: 4111111111111111".to_string());
        let msg = error.safe_message();
        assert!(!msg.contains("4111111111111111"));
    }

    #[test]
    fn test_error_display_format() {
        // 验证 Display trait 实现
        assert_eq!(
            InklogError::ConfigError("test".into()).to_string(),
            "Configuration error: test"
        );
        assert_eq!(
            InklogError::ChannelError("closed".into()).to_string(),
            "Channel error: closed"
        );
    }

    #[test]
    fn test_database_error_constructor() {
        let err = InklogError::database_error("connection refused");
        assert!(matches!(
            err,
            InklogError::DatabaseError { source: None, .. }
        ));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_database_error_with_source_constructor() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = InklogError::database_error_with_source("db init failed", io_err);
        match &err {
            InklogError::DatabaseError { message, source } => {
                assert_eq!(message, "db init failed");
                assert!(source.is_some());
            }
            _ => panic!("expected DatabaseError"),
        }
        assert!(err.to_string().contains("db init failed"));
    }

    #[test]
    fn test_encryption_error_constructor() {
        let err = InklogError::encryption_error("invalid key");
        assert!(matches!(
            err,
            InklogError::EncryptionError { source: None, .. }
        ));
        assert!(err.to_string().contains("invalid key"));
    }

    #[test]
    fn test_encryption_error_with_source_constructor() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = InklogError::encryption_error_with_source("key load failed", io_err);
        match &err {
            InklogError::EncryptionError { message, source } => {
                assert_eq!(message, "key load failed");
                assert!(source.is_some());
            }
            _ => panic!("expected EncryptionError"),
        }
    }

    #[test]
    fn test_inklog_result_type_alias() {
        let ok: InklogResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
        let err: InklogResult<i32> = Err(InklogError::Unknown("test".into()));
        assert!(err.is_err());
    }

    #[test]
    fn test_safe_message_cache_error() {
        let err = InklogError::CacheError("connection lost".into());
        let msg = err.safe_message();
        assert!(msg.contains("Cache error:"));
        assert!(msg.contains("connection lost"));
    }

    #[test]
    fn test_safe_message_encryption_error() {
        let err = InklogError::encryption_error("key expired");
        let msg = err.safe_message();
        assert!(msg.contains("Encryption error:"));
        assert!(msg.contains("key expired"));
    }

    #[test]
    fn test_safe_message_shutdown() {
        let err = InklogError::Shutdown("timeout".into());
        let msg = err.safe_message();
        assert!(msg.contains("Shutdown error:"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_safe_message_channel_error() {
        let err = InklogError::ChannelError("disconnected".into());
        let msg = err.safe_message();
        assert!(msg.contains("Channel error:"));
        assert!(msg.contains("disconnected"));
    }

    #[test]
    fn test_safe_message_compression_error() {
        let err = InklogError::CompressionError("zlib failed".into());
        let msg = err.safe_message();
        assert!(msg.contains("Compression error:"));
        assert!(msg.contains("zlib failed"));
    }

    #[test]
    fn test_safe_message_runtime_error() {
        let err = InklogError::RuntimeError("panic".into());
        let msg = err.safe_message();
        assert!(msg.contains("Runtime error:"));
        assert!(msg.contains("panic"));
    }

    #[test]
    fn test_safe_message_http_server_error() {
        let err = InklogError::HttpServerError("bind failed".into());
        let msg = err.safe_message();
        assert!(msg.contains("HTTP server error:"));
        assert!(msg.contains("bind failed"));
    }

    #[test]
    fn test_safe_message_unknown() {
        let err = InklogError::Unknown("something".into());
        let msg = err.safe_message();
        assert!(msg.contains("Unknown error:"));
        assert!(msg.contains("something"));
    }

    #[test]
    fn test_safe_message_database_error() {
        let err = InklogError::database_error("conn refused");
        let msg = err.safe_message();
        assert!(msg.contains("Database error:"));
        assert!(msg.contains("conn refused"));
    }
}
