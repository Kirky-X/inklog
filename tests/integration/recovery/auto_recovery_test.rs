// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use inklog::LoggerManager;
use std::fs;
use std::thread;
use std::time::Duration;

#[tokio::test]
async fn test_file_sink_auto_recovery() {
    // Create a test directory
    let test_dir = "tests/temp_recovery";
    let _ = fs::create_dir_all(test_dir);

    // Create a logger with file sink
    let log_file = format!("{}/test_recovery.log", test_dir);
    let manager = LoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log some messages
    tracing::info!("Test message before failure");
    thread::sleep(Duration::from_millis(100));

    // Simulate file sink failure by removing the log file
    let _ = fs::remove_file(&log_file);

    // Log more messages (these should fail and trigger recovery)
    for i in 0..10 {
        tracing::info!("Test message during failure {}", i);
        thread::sleep(Duration::from_millis(50));
    }

    // Wait for auto-recovery to trigger
    thread::sleep(Duration::from_secs(2));

    // Log messages after potential recovery
    tracing::info!("Test message after recovery");
    thread::sleep(Duration::from_millis(100));

    // Check health status
    let health = manager.get_health_status();
    println!("Health status: {:?}", health);

    // Clean up
    let _ = fs::remove_dir_all(test_dir);
}

#[tokio::test]
async fn test_manual_sink_recovery() {
    let test_dir = "tests/temp_manual_recovery";
    let _ = fs::create_dir_all(test_dir);

    let log_file = format!("{}/test_manual_recovery.log", test_dir);
    let manager = LoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log initial message
    tracing::info!("Initial test message");
    thread::sleep(Duration::from_millis(100));

    // Simulate failure by removing file
    let _ = fs::remove_file(&log_file);

    // Log during failure
    tracing::info!("Message during failure");
    thread::sleep(Duration::from_millis(100));

    // Trigger manual recovery
    let recovery_result = manager.recover_sink("file");
    println!("Manual recovery result: {:?}", recovery_result);

    // Wait for recovery
    thread::sleep(Duration::from_millis(500));

    // Log after manual recovery
    tracing::info!("Message after manual recovery");
    thread::sleep(Duration::from_millis(100));

    // Clean up
    let _ = fs::remove_dir_all(test_dir);

    assert!(recovery_result.is_ok());
}

#[tokio::test]
async fn test_bulk_recovery_for_unhealthy_sinks() {
    let test_dir = "tests/temp_bulk_recovery";
    let _ = fs::create_dir_all(test_dir);

    let log_file = format!("{}/test_bulk_recovery.log", test_dir);
    let manager = LoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log initial message
    tracing::info!("Initial test message");
    thread::sleep(Duration::from_millis(100));

    // Simulate failure
    let _ = fs::remove_file(&log_file);

    // Log during failure to make sink unhealthy
    for i in 0..5 {
        tracing::info!("Message during failure {}", i);
        thread::sleep(Duration::from_millis(50));
    }

    // Trigger bulk recovery
    let recovery_result = manager.trigger_recovery_for_unhealthy_sinks();
    println!("Bulk recovery result: {:?}", recovery_result);

    // Wait for recovery
    thread::sleep(Duration::from_millis(500));

    // Log after bulk recovery
    tracing::info!("Message after bulk recovery");
    thread::sleep(Duration::from_millis(100));

    // Clean up
    let _ = fs::remove_dir_all(test_dir);

    assert!(recovery_result.is_ok());
}
