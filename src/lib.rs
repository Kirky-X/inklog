pub mod archive;
pub mod config;
mod error;
pub mod log_record;
mod manager;
mod masking;
mod metrics;
mod pool;
pub mod sink;
pub mod subscriber;
pub mod template;

pub use config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig};
pub use error::InklogError;
pub use manager::{LoggerBuilder, LoggerManager};
pub use metrics::{HealthStatus, Metrics, SinkStatus};

#[cfg(feature = "aws")]
pub use archive::{S3ArchiveConfig, S3ArchiveManager};
