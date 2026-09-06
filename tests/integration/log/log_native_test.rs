// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! log crate 原生支持的集成测试
//!
//! 验证 inklog 可以直接接收来自 `log` crate 的日志，无需 `tracing_log` 适配器。

use inklog::LoggerManager;
use inklog::tracing::{error as tracing_error, info as tracing_info};
use log::{debug, error, info, warn};
use std::time::Duration;

/// 测试 log crate 原生支持
/// 验证用户可以直接使用 log::info! 等宏，无需 tracing_log 适配器
#[tokio::test]
async fn test_log_crate_native_support() {
    // 初始化 inklog（build 返回 Result，unwrap 确认配置有效）
    let _logger = LoggerManager::builder()
        .level("debug")
        .build()
        .await
        .unwrap();

    // 使用 log crate 的宏
    info!("This is a log::info message");
    warn!("This is a log::warn message");
    error!("This is a log::error message");
    debug!("This is a log::debug message");

    // 给异步 workers 一些时间处理；宏调用无 panic + shutdown 成功即 smoke 通过
    std::thread::sleep(Duration::from_millis(200));

    let _ = _logger.shutdown();
}

/// 测试 tracing 和 log 可以同时使用
#[tokio::test]
async fn test_tracing_and_log_coexist() {
    let _logger = LoggerManager::builder()
        .level("debug")
        .build()
        .await
        .unwrap();

    // 同时使用 tracing 和 log
    info!("log::info message");
    tracing_info!("tracing::info message");

    error!("log::error message");
    tracing_error!("tracing::error message");

    std::thread::sleep(Duration::from_millis(200));

    let _ = _logger.shutdown();
}

/// 测试日志级别过滤
#[tokio::test]
async fn test_log_level_filtering() {
    // 设置为 WARN 级别
    let _logger = LoggerManager::builder()
        .level("warn")
        .build()
        .await
        .unwrap();

    // 这些日志应该被过滤掉
    debug!("This debug message should not appear");
    info!("This info message should not appear");

    // 只有 WARN 和 ERROR 应该出现
    warn!("This warn message should appear");
    error!("This error message should appear");

    std::thread::sleep(Duration::from_millis(100));

    let _ = _logger.shutdown();
}

/// 测试结构化日志（log crate 的 target 功能）
#[tokio::test]
async fn test_log_target() {
    let _logger = LoggerManager::new().await.unwrap();

    // log crate 支持通过宏设置 target
    info!(target: "my_module", "Structured log message from log crate");
    warn!(target: "auth", "User authentication attempt");

    std::thread::sleep(Duration::from_millis(100));

    let _ = _logger.shutdown();
}

/// 测试 log crate 的所有级别
#[tokio::test]
async fn test_log_all_levels() {
    let _logger = LoggerManager::builder()
        .level("trace")
        .build()
        .await
        .unwrap();

    log::trace!("Trace message from log crate");
    log::debug!("Debug message from log crate");
    log::info!("Info message from log crate");
    log::warn!("Warn message from log crate");
    log::error!("Error message from log crate");

    std::thread::sleep(Duration::from_millis(100));

    let _ = _logger.shutdown();
}

/// 测试日志格式化
#[tokio::test]
async fn test_log_formatting() {
    let _logger = LoggerManager::new().await.unwrap();

    // 测试各种格式化选项
    info!("Simple message");
    info!("Message with {}", "formatting");
    info!("Message with {:?} debug", "structure");
    info!("Message with numbers: {}, {}", 42, std::f64::consts::PI);

    std::thread::sleep(Duration::from_millis(100));

    let _ = _logger.shutdown();
}

/// 测试日志文件写入
///
/// 核正：log::! 宏唯一入口是进程级全局 logger（manager 构建时 LogLogger::install，
/// log::set_boxed_logger 仅首次成功且不可更换）——cargo test 并发下无法保证本用例
/// 赢得绑定，log 记录可能流向其他用例的 logger（noop/转发路径由库单测
/// log_adapter::tests 覆盖）。文件落盘验证改按本仓口径（build_detached +
/// 线程级 set_default + tracing 宏）固化；log 宏调用保留以验证两路径均不 panic。
#[tokio::test]
async fn test_log_to_file() {
    use inklog::InklogConfig;
    use inklog::config::FileSinkConfig;
    use tracing_subscriber::layer::SubscriberExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("test.log");

    // log 宏路径：无全局绑定保证，仅验证调用不 panic
    let _smoke = LoggerManager::builder().level("info").build().await;
    info!("log macro path (no binding guarantee)");
    warn!("log macro path (no binding guarantee)");

    // tracing 路径：真实落盘验证
    let config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_file.clone(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let (logger, subscriber, filter) = LoggerManager::build_detached(
        config,
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        None,
    )
    .await
    .unwrap();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(subscriber).with(filter),
    );

    tracing::info!("This should go to file (tracing path)");
    tracing::warn!("This warning should also be in file (tracing path)");

    let _ = logger.shutdown();

    // 验证文件存在且有内容
    assert!(log_file.exists());
    let contents = std::fs::read_to_string(&log_file).unwrap();
    assert!(
        contents.contains("This should go to file"),
        "tracing 路径日志应落盘"
    );
}
