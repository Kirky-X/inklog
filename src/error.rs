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

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

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
