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
//! | `S3Error` | AWS S3 操作错误 |
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

use thiserror::Error;

/// Sensitive pattern redaction rules for error messages.
/// Each tuple contains (pattern, replacement).
const SENSITIVE_PATTERNS: &[(&str, &str)] = &[
    // AWS Access Key ID pattern (20 characters, starts with AKIA)
    ("AKIA[0-9A-Z]{16}", "[AWS_ACCESS_KEY_ID]"),
    // AWS Secret Key pattern (40 characters, base64-like)
    ("[0-9a-zA-Z+/]{40}=?", "[AWS_SECRET_ACCESS_KEY]"),
    // JWT Token pattern
    (
        "eyJ[a-zA-Z0-9_-]*\\.eyJ[a-zA-Z0-9_-]*\\.[a-zA-Z0-9_-]*",
        "[JWT_TOKEN]",
    ),
    // Database connection strings
    ("postgres://[^:]+:[^@]+@[^/]+/[^?]*", "[DB_CONNECTION]"),
    ("mysql://[^:]+:[^@]+@[^/]+/[^?]*", "[DB_CONNECTION]"),
    ("sqlite://[^?]*\\?[^&]*", "[DB_CONNECTION]"),
    // Sensitive paths
    ("/home/[a-zA-Z0-9_-]+/", "[USER_HOME_PATH]"),
    ("/etc/inklog/", "[CONFIG_PATH]"),
    ("/run/secrets/", "[SECRETS_PATH]"),
];

/// Sanitizes a message by removing sensitive information.
/// Uses pattern matching to detect and redact common sensitive patterns.
fn sanitize_message(msg: &str) -> String {
    let mut result = msg.to_string();
    for (pattern, replacement) in SENSITIVE_PATTERNS {
        result = result.replace(pattern, replacement);
    }
    result
}

#[derive(Error, Debug)]
pub enum InklogError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("HTTP server error: {0}")]
    HttpServerError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[cfg(feature = "confers")]
impl From<confers::error::ConfigError> for InklogError {
    fn from(err: confers::error::ConfigError) -> Self {
        InklogError::ConfigError(err.to_string())
    }
}

#[cfg(feature = "confers")]
impl From<toml::de::Error> for InklogError {
    fn from(err: toml::de::Error) -> Self {
        InklogError::ConfigError(err.to_string())
    }
}

#[cfg(feature = "aws")]
impl From<tokio_cron_scheduler::JobSchedulerError> for InklogError {
    fn from(err: tokio_cron_scheduler::JobSchedulerError) -> Self {
        InklogError::ConfigError(format!("Scheduler error: {}", err))
    }
}

impl InklogError {
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
            InklogError::DatabaseError(msg) => {
                format!("Database error: {}", sanitize_message(msg))
            }
            InklogError::EncryptionError(msg) => {
                format!("Encryption error: {}", sanitize_message(msg))
            }
            InklogError::Shutdown(msg) => {
                format!("Shutdown error: {}", sanitize_message(msg))
            }
            InklogError::ChannelError(msg) => {
                format!("Channel error: {}", sanitize_message(msg))
            }
            InklogError::S3Error(msg) => {
                format!("S3 error: {}", sanitize_message(msg))
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
