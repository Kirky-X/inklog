// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Domain config module - configuration types for inklog.

#[allow(clippy::module_inception)]
pub mod config;

pub use config::{
    ChannelStrategy, ConsoleSinkConfig, DatabaseDriver, DatabaseSinkConfig, FileSinkConfig,
    GlobalConfig, HttpAuthConfig, HttpErrorMode, HttpServerConfig, InklogConfig, ParquetConfig,
    PartitionStrategy, PerformanceConfig,
};
