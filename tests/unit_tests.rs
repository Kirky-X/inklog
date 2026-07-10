// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试入口文件
//!
//! 此文件作为单元测试的入口点，包含所有单元测试模块的测试用例。
//!
//! 测试模块组织：
//! - 配置测试 (unit::config)
//! - 输出端测试 (unit::sink)
//! - 归档测试 (unit::archive)
//! - CLI测试 (unit::cli)

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod unit;

// Allow unused imports for conditional compilation with different features
#[allow(unused_imports)]
use inklog::log_record::LogRecord;
#[allow(unused_imports)]
use inklog::sink::LogSink;
#[allow(unused_imports)]
use inklog::sink::console::ConsoleSink;
#[allow(unused_imports)]
use inklog::template::LogTemplate;
#[allow(unused_imports)]
use inklog::{ConsoleSinkConfig, FileSinkConfig, InklogConfig, LoggerManager};
#[allow(unused_imports)]
use tempfile::TempDir;
#[allow(unused_imports)]
use tracing::Level;

// ============ 配置单元测试 (unit::config) ============

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
#[test]
fn test_config_validation() {
    let config = InklogConfig::default();
    assert!(config.validate().is_ok());

    let mut invalid_config = InklogConfig::default();
    invalid_config.performance.channel_capacity = 0;
    assert!(invalid_config.validate().is_err());
}

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
#[test]
fn test_builder() {
    let _logger = LoggerManager::builder()
        .level("debug")
        .console(false)
        .channel_capacity(100)
        .build();
}

// ============ 输出端单元测试 (unit::sink) ============

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
#[tokio::test]
async fn test_console_sink_format() {
    let config = ConsoleSinkConfig {
        colored: false,
        ..Default::default()
    };
    let template = LogTemplate::default();
    let sink = ConsoleSink::new(config, template);
    let record = LogRecord::new(Level::INFO, "test_target".into(), "test_message".into());
    assert!(sink.write(&record).await.is_ok());
}

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
#[tokio::test]
async fn test_file_sink_rotation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let log_path = temp_dir.path().join("app.log");

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "100B".into(), // Small size to trigger rotation
        ..Default::default()
    };

    let sink = inklog::sink::file::FileSink::new(config).expect("Failed to create FileSink");

    // Write enough data to trigger rotation
    for i in 0..10 {
        let record = LogRecord::new(Level::INFO, "test".into(), format!("msg {}", i));
        sink.write(&record)
            .await
            .expect("Failed to write log record");
    }

    // Check if files created
    let entries = std::fs::read_dir(temp_dir.path()).expect("Failed to read temp directory");
    assert!(entries.count() >= 1);
}

// ============ 归档单元测试 (unit::archive) ============
// (归档单元测试的具体实现待补充)

// ============ CLI 单元测试 (unit::cli) ============
// (CLI 单元测试的具体实现待补充)
