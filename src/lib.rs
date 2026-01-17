// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod archive;
pub mod config;
mod config_validator;
mod error;
pub mod log_record;
mod manager;
pub mod masking;
pub mod metrics;
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
