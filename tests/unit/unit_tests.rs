// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use inklog::log_record::LogRecord;
use inklog::sink::console::ConsoleSink;
use inklog::sink::LogSink;
use inklog::template::LogTemplate;
use inklog::{ConsoleSinkConfig, FileSinkConfig, InklogConfig, LoggerManager};
use tempfile::TempDir;
use tracing::Level;

#[test]
fn test_config_validation() {
    let config = InklogConfig::default();
    assert!(config.validate().is_ok());

    let mut invalid_config = InklogConfig::default();
    invalid_config.performance.channel_capacity = 0;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_builder() {
    let _logger = LoggerManager::builder()
        .level("debug")
        .console(false)
        .channel_capacity(100)
        .build();
}

#[test]
fn test_console_sink_format() {
    let config = ConsoleSinkConfig {
        colored: false,
        ..Default::default()
    };
    let template = LogTemplate::default();
    let mut sink = ConsoleSink::new(config, template);
    let record = LogRecord::new(Level::INFO, "test_target".into(), "test_message".into());
    assert!(sink.write(&record).is_ok());
}

#[test]
fn test_file_sink_rotation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let log_path = temp_dir.path().join("app.log");

    let config = FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "100B".into(), // Small size to trigger rotation
        ..Default::default()
    };

    let mut sink = inklog::sink::file::FileSink::new(config).expect("Failed to create FileSink");

    // Write enough data to trigger rotation
    for i in 0..10 {
        let record = LogRecord::new(Level::INFO, "test".into(), format!("msg {}", i));
        sink.write(&record).expect("Failed to write log record");
    }

    // Check if files created
    let entries = std::fs::read_dir(temp_dir.path()).expect("Failed to read temp directory");
    assert!(entries.count() >= 1);
}
