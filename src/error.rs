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
