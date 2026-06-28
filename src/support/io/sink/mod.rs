// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod circuit_breaker;
pub mod compression;
pub mod console;
#[cfg(feature = "dbnexus")]
pub mod database;
pub mod encryption;
#[cfg(feature = "dbnexus")]
pub mod entity;
pub mod file;
pub mod registry;
pub mod ring_buffered_file;
pub mod rotation;

pub use compression::{CompressionStrategy, GzipCompression, NoCompression, ZstdCompression};
pub use registry::{FileSinkFactory, SinkFactory, SinkMetadata, SinkRegistry};
pub use rotation::{
    CompositeRotation, RotationContext, RotationResult, RotationStrategy, SizeBasedRotation,
    TimeBasedRotation,
};

use crate::InklogError;
use crate::LogRecord;

/// Log sink trait for writing log records to various destinations.
///
/// All methods use `&self` instead of `&mut self` to support interior mutability
/// and dependency injection patterns. Implementations should use `Mutex` or `RwLock`
/// for mutable state.
pub trait LogSink: Send + Sync {
    /// Write a log record to the sink.
    fn write(&self, record: &LogRecord) -> Result<(), InklogError>;

    /// Flush any buffered data to the underlying storage.
    fn flush(&self) -> Result<(), InklogError>;

    /// Check if the sink is healthy and operational.
    fn is_healthy(&self) -> bool {
        true
    }

    /// Gracefully shutdown the sink, flushing any remaining data.
    fn shutdown(&self) -> Result<(), InklogError>;

    /// Start rotation timer (for file-based sinks with time-based rotation).
    fn start_rotation_timer(&self) {
        // 默认空实现
    }

    /// Stop rotation timer.
    fn stop_rotation_timer(&self) {
        // 默认空实现
    }

    /// Check if there is sufficient disk space for writing.
    fn check_disk_space(&self) -> Result<bool, InklogError> {
        Ok(true) // 默认返回有足够空间
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test struct that uses default trait method implementations
    struct DummySink;

    impl LogSink for DummySink {
        fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Ok(())
        }
        fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[test]
    fn test_default_is_healthy() {
        let sink = DummySink;
        assert!(sink.is_healthy());
    }

    #[test]
    fn test_default_start_rotation_timer() {
        let sink = DummySink;
        // Just verify it doesn't panic
        sink.start_rotation_timer();
    }

    #[test]
    fn test_default_stop_rotation_timer() {
        let sink = DummySink;
        // Just verify it doesn't panic
        sink.stop_rotation_timer();
    }

    #[test]
    fn test_default_check_disk_space() {
        let sink = DummySink;
        let result = sink.check_disk_space();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
