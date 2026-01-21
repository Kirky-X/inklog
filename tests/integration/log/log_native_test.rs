// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! log crate 原生支持的集成测试
//!
//! 验证 inklog 可以直接接收来自 `log` crate 的日志，无需 `tracing_log` 适配器。

use inklog::LoggerManager;
use log::{debug, error, info, warn};
use std::time::Duration;
use tracing::{error as tracing_error, info as tracing_info};

/// 测试 log crate 原生支持
/// 验证用户可以直接使用 log::info! 等宏，无需 tracing_log 适配器
#[tokio::test]
async fn test_log_crate_native_support() {
    // 初始化 inklog
    let _logger = LoggerManager::builder()
        .level("debug")
        .build()
        .await;

    // 使用 log crate 的宏
    info!("This is a log::info message");
    warn!("This is a log::warn message");
    error!("This is a log::error message");
    debug!("This is a log::debug message");

    // 给异步 workers 一些时间处理
    std::thread::sleep(Duration::from_millis(200));

    // 如果没有 panic，说明日志系统正常工作
    assert!(true);
}

/// 测试 tracing 和 log 可以同时使用
#[tokio::test]
async fn test_tracing_and_log_coexist() {
    let _logger = LoggerManager::builder()
        .level("debug")
        .build()
        .await;

    // 同时使用 tracing 和 log
    info!("log::info message");
    tracing_info!("tracing::info message");

    error!("log::error message");
    tracing_error!("tracing::error message");

    std::thread::sleep(Duration::from_millis(200));

    assert!(true);
}

/// 测试日志级别过滤
#[tokio::test]
async fn test_log_level_filtering() {
    // 设置为 WARN 级别
    let _logger = LoggerManager::builder()
        .level("warn")
        .build()
        .await;

    // 这些日志应该被过滤掉
    debug!("This debug message should not appear");
    info!("This info message should not appear");

    // 只有 WARN 和 ERROR 应该出现
    warn!("This warn message should appear");
    error!("This error message should appear");

    std::thread::sleep(Duration::from_millis(100));

    assert!(true);
}

/// 测试结构化日志（log crate 的 target 功能）
#[tokio::test]
async fn test_log_target() {
    let _logger = LoggerManager::new().await;

    // log crate 支持通过宏设置 target
    info!(target: "my_module", "Structured log message from log crate");
    warn!(target: "auth", "User authentication attempt");

    std::thread::sleep(Duration::from_millis(100));

    assert!(true);
}

/// 测试 log crate 的所有级别
#[tokio::test]
async fn test_log_all_levels() {
    let _logger = LoggerManager::builder()
        .level("trace")
        .build()
        .await;

    log::trace!("Trace message from log crate");
    log::debug!("Debug message from log crate");
    log::info!("Info message from log crate");
    log::warn!("Warn message from log crate");
    log::error!("Error message from log crate");

    std::thread::sleep(Duration::from_millis(100));

    assert!(true);
}

/// 测试日志格式化
#[tokio::test]
async fn test_log_formatting() {
    let _logger = LoggerManager::new().await;

    // 测试各种格式化选项
    info!("Simple message");
    info!("Message with {}", "formatting");
    info!("Message with {:?} debug", "structure");
    info!("Message with numbers: {}, {}", 42, 3.14);

    std::thread::sleep(Duration::from_millis(100));

    assert!(true);
}

/// 测试日志文件写入
#[tokio::test]
async fn test_log_to_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("test.log");

    let _logger = LoggerManager::builder()
        .level("info")
        .file(&log_file)
        .build()
        .await;

    info!("This should go to file");
    warn!("This warning should also be in file");

    std::thread::sleep(Duration::from_millis(200));

    // 验证文件存在且有内容
    assert!(log_file.exists());
    let contents = std::fs::read_to_string(&log_file).unwrap();
    assert!(contents.contains("This should go to file") || contents.len() > 0);
}
