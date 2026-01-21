// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod archive;
pub mod config;
mod config_validator;
mod error;
pub mod log_adapter;
pub mod log_record;
mod manager;
pub mod masking;
pub mod metrics;
mod pool;
pub mod sink;
pub mod subscriber;
pub mod template;

pub use config::{
    ChannelStrategy, ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, InklogConfig,
    PerformanceConfig,
};
pub use error::InklogError;
pub use log_adapter::{LogAdapter, LogLogger};
pub use manager::{LoggerBuilder, LoggerManager};
pub use metrics::{
    FallbackAction, FallbackConfig, FallbackState, FallbackStats, HealthStatus, Metrics,
    SinkHealthMonitor, SinkStatus,
};

#[cfg(feature = "aws")]
pub use archive::{ArchiveService, ArchiveServiceBuilder, S3ArchiveConfig, S3ArchiveManager};
