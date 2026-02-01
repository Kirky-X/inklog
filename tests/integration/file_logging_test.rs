// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! File sink integration tests
//!
//! These tests verify file logging functionality and can be run independently
//! to avoid conflicts with other tests that may initialize the global logger.

use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_log_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("test.log");

    // Create a unique logger for this test
    let config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_file.clone(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let logger = LoggerManager::with_config(config).await.unwrap();

    log::info!("This should go to file");
    log::warn!("This warning should also be in file");

    // Wait for async worker to process
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify file exists
    assert!(log_file.exists(), "Log file should exist");

    // Verify file has content
    let contents = std::fs::read_to_string(&log_file).unwrap_or_default();
    assert!(
        !contents.is_empty(),
        "Log file should have content"
    );
    assert!(
        contents.contains("This should go to file"),
        "Log should contain the info message"
    );

    let _ = logger.shutdown();
}

#[tokio::test]
async fn test_log_to_file_with_rotation() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("rotating.log");

    let config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_file.clone(),
            max_size: "1KB".into(),
            keep_files: 3,
            ..Default::default()
        }),
        ..Default::default()
    };

    let logger = LoggerManager::with_config(config).await.unwrap();

    // Write enough logs to trigger rotation
    for i in 0..100 {
        log::info!("Log message number {}", i);
    }

    // Wait for async worker to process
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Verify main file exists
    assert!(log_file.exists(), "Main log file should exist");

    let _ = logger.shutdown();
}
