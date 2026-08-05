// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! I/O module - log output adapters and sink implementations.

pub mod log_adapter;
pub mod sink;

pub use log_adapter::{LogAdapter, LogLogger};
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub use sink::DatabaseSink;
pub use sink::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, ConsoleSink, DiskCheckable, FileSink,
    LogSink, Rotatable,
};
