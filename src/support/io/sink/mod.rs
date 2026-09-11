// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
pub mod circuit_breaker;
pub mod compression;
pub mod console;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub mod database;
pub mod encryption;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub mod entity;
pub mod file;
pub mod registry;
pub mod ring_buffered_file;
pub mod rotation;
pub mod sampling;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
#[cfg(feature = "compression")]
pub use compression::ZstdCompression;
#[cfg(feature = "gzip")]
pub use compression::GzipCompression;
pub use compression::{CompressionStrategy, NoCompression};
pub use console::ConsoleSink;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub use database::DatabaseSink;
pub use file::FileSink;
pub use registry::{FileSinkFactory, SinkFactory, SinkMetadata, SinkRegistry};
pub use rotation::{
    CompositeRotation, RotationContext, RotationResult, RotationStrategy, SizeBasedRotation,
    TimeBasedRotation,
};
pub use sampling::{Sampler, SamplingSink};

use crate::InklogError;
use crate::LogRecord;
use async_trait::async_trait;

/// Async sink registration port (T501: dynamic sink registration).
///
/// Third-party sinks implement [`LogSink`] (all methods async) and are
/// registered via [`crate::LoggerBuilder::add_sink`] as `Arc<dyn AsyncSink>`.
/// Every `LogSink` implementor automatically implements `AsyncSink` through a
/// blanket impl, so third-party sinks need zero core changes to plug in.
pub trait AsyncSink: LogSink {}

impl<T: LogSink + ?Sized> AsyncSink for T {}

/// Log sink trait for writing log records to various destinations.
///
/// All methods use `&self` instead of `&mut self` to support interior mutability
/// and dependency injection patterns. Implementations should use `Mutex` or `RwLock`
/// for mutable state.
///
/// # Trait Isolation
///
/// Optional capabilities are split into separate traits:
/// - [`Rotatable`]: File rotation support (implemented by [`FileSink`], extensible)
/// - [`DiskCheckable`]: Disk space checking (implemented by [`FileSink`], extensible)
#[async_trait]
pub trait LogSink: Send + Sync {
    /// Write a log record to the sink.
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;

    /// Flush any buffered data to the underlying storage.
    async fn flush(&self) -> Result<(), InklogError>;

    /// Check if the sink is healthy and operational.
    ///
    /// # Default Implementation
    ///
    /// The default implementation **always returns `true`** so that sinks
    /// which do not track health keep working unchanged. Implementations
    /// SHOULD override this method to reflect their real health state.
    fn is_healthy(&self) -> bool {
        true
    }

    /// Gracefully shutdown the sink, flushing any remaining data.
    async fn shutdown(&self) -> Result<(), InklogError>;
}

/// Trait for sinks that support log file rotation.
///
/// Implemented by [`FileSink`]; the trait is `pub`, so custom sinks may
/// implement it too. Separated from [`LogSink`] to keep the core trait
/// minimal for sinks that don't need rotation.
pub trait Rotatable {
    /// Start rotation timer (for file-based sinks with time-based rotation).
    fn start_rotation_timer(&self);

    /// Stop rotation timer.
    fn stop_rotation_timer(&self);
}

/// Trait for sinks that can check disk space before writing.
///
/// Implemented by [`FileSink`]; the trait is `pub`, so custom sinks may
/// implement it too. Separated from [`LogSink`] to keep the core trait
/// minimal for sinks that don't write to disk.
pub trait DiskCheckable {
    /// Check if there is sufficient disk space for writing.
    fn check_disk_space(&self) -> Result<bool, InklogError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test struct that uses default trait method implementations
    struct DummySink;

    #[async_trait]
    impl LogSink for DummySink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[test]
    fn test_default_is_healthy() {
        let sink = DummySink;
        assert!(sink.is_healthy());
    }

    /// Test that Rotatable and DiskCheckable are separate traits
    struct RotatableDiskSink;

    #[async_trait]
    impl LogSink for RotatableDiskSink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    impl Rotatable for RotatableDiskSink {
        fn start_rotation_timer(&self) {}
        fn stop_rotation_timer(&self) {}
    }

    impl DiskCheckable for RotatableDiskSink {
        fn check_disk_space(&self) -> Result<bool, InklogError> {
            Ok(true)
        }
    }

    #[test]
    fn test_rotatable_trait() {
        let sink = RotatableDiskSink;
        // Verify Rotatable trait works
        sink.start_rotation_timer();
        sink.stop_rotation_timer();
    }

    #[test]
    fn test_disk_checkable_trait() {
        let sink = RotatableDiskSink;
        let result = sink.check_disk_space();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_log_sink_does_not_have_rotation_or_disk() {
        // Verify LogSink trait no longer has rotation/disk methods
        // DummySink only implements LogSink, not Rotatable or DiskCheckable
        let sink = DummySink;
        assert!(sink.is_healthy());
        // sink.start_rotation_timer(); // This would not compile - correct!
        // sink.check_disk_space(); // This would not compile - correct!
    }
}
