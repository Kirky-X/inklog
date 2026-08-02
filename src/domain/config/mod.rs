// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Domain config module - configuration types for inklog.

#[allow(clippy::module_inception)]
pub mod config;
pub mod console;
pub mod database;
pub mod file_sink;
pub mod global;
pub mod http;
pub mod performance;

pub use config::InklogConfig;
pub use console::ConsoleSinkConfig;
pub use database::{DatabaseDriver, DatabaseSinkConfig, ParquetConfig, PartitionStrategy};
pub use file_sink::FileSinkConfig;
pub use global::GlobalConfig;
pub use http::{HttpAuthConfig, HttpErrorMode, HttpServerConfig};
pub use performance::{ChannelStrategy, PerformanceConfig};
