// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! I/O module - log output adapters and sink implementations.

pub mod log_adapter;
pub mod sink;

pub use log_adapter::{LogAdapter, LogLogger};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
pub use sink::DatabaseSink;
pub use sink::{CircuitBreaker, ConsoleSink, FileSink, LogSink};
