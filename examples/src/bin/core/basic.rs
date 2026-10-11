// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 基础用法示例
//!
//! 演示 inklog 的最基本使用方式，涵盖初始化、日志记录、验证和关闭全流程。
//!
//! # 内容
//!
//! 1. `LoggerManager::new()` 默认初始化
//! 2. 不同日志级别的使用（trace/debug/info/warn/error）
//! 3. 结构化日志（带字段）
//! 4. 健康状态验证
//! 5. 优雅关闭
//! 6. 最佳实践建议
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin basic
//! ```

use inklog::LoggerManager;
use std::mem;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog basic usage examples ===\n");

    // 在 main 中创建并持有 logger，确保所有示例的 tracing 调用都有活跃 subscriber
    let _logger = LoggerManager::new().await?;

    show_default_initialization().await?;
    show_log_levels();
    show_structured_logging();
    show_health_verification();
    show_config_initialization().await?;
    show_graceful_shutdown().await?;
    show_best_practices();

    println!("\nAll basic examples completed.");
    Ok(())
}

/// 示例 1：默认初始化
///
/// 使用 `LoggerManager::new()` 以默认配置启动日志系统。
/// 默认配置仅启用 Console Sink，日志级别为 info。
///
/// 注意：本示例的 logger 在 main() 中创建并保持存活，
/// 确保后续示例的 tracing 调用有活跃的 subscriber。
async fn show_default_initialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: default initialization ---\n");

    // logger 已在 main() 中创建，此处展示初始化模式
    println!("✓ LoggerManager::new() initialized (logger is created in main)");
    tracing::info!("log system started");
    println!("✓ logging works\n");

    Ok(())
}

/// 示例 2：不同日志级别
///
/// 演示 5 个标准日志级别的使用场景。
fn show_log_levels() {
    println!("--- Example 2: log levels ---\n");

    println!("5 standard log levels:");
    tracing::trace!("TRACE - most detailed diagnostics; development only");
    tracing::debug!("DEBUG - debugging information to help locate problems");
    tracing::info!("INFO - routine runtime information marking important events");
    tracing::warn!("WARN - potential issue that does not affect normal operation");
    tracing::error!("ERROR - functionality failure that needs attention");

    println!("\nLevel filtering rules (default info):");
    println!("  trace → only tracing::trace! visible");
    println!("  debug → debug and above visible");
    println!("  info  → info/warn/error visible (default)");
    println!("  warn  → warn/error visible");
    println!("  error → only error visible");
    println!();
}

/// 示例 3：结构化日志
///
/// 演示带结构化字段的日志记录，便于后续查询和分析。
fn show_structured_logging() {
    println!("--- Example 3: structured logging ---\n");

    // 用户操作日志
    tracing::info!(
        user_id = 12345,
        action = "login",
        ip = "192.168.1.1",
        "user logged in successfully"
    );
    println!("✓ user operation logged (user_id, action, ip)");

    // 请求处理日志
    tracing::info!(
        request_id = "req-abc-123",
        method = "GET",
        path = "/api/users",
        status = 200,
        latency_ms = 42,
        "request completed"
    );
    println!("✓ request logged (request_id, method, path, status, latency_ms)");

    // 错误日志
    tracing::error!(
        error_code = "E001",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "payment processing failed; retry limit reached"
    );
    println!("✓ error logged (error_code, component, retry_count)\n");
}

/// 示例 4：健康状态验证
///
/// 演示如何检查 LoggerManager 的运行状态，确认系统健康。
fn show_health_verification() {
    println!("--- Example 4: health status check ---\n");

    println!("Note: health status must be checked on a LoggerManager instance.");
    println!("This example shows the API usage (it needs to hold a logger reference):\n");

    println!(
        r#"  let logger = LoggerManager::new().await?;
  let health = logger.get_health_status();

  // Verify the overall status
  assert!(health.overall_status.is_operational());

  // Inspect channel usage
  let capacity = logger.effective_channel_capacity();
  let queued = logger.channel_len();
  println!("Channel: {{}}/{{}}", queued, capacity);

  // Iterate over each sink status
  for (name, sink) in &health.sinks {{
      println!("  Sink '{{}}': {{:?}}", name, sink.status);
  }}"#
    );
    println!();
}

/// 示例 5：自定义配置初始化
///
/// 演示使用 `InklogConfig` 自定义配置初始化日志系统。
///
/// 注意：由于 tracing 全局 subscriber 已在 main() 中设置，
/// 此处展示配置模式代码，不再创建新的 LoggerManager。
async fn show_config_initialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 5: custom configuration initialization ---\n");

    // 方式 1：修改默认配置（代码展示）
    println!("Approach 1: modify the default config");
    println!(
        r#"  let mut config = InklogConfig::default();
  config.global.level = "debug".to_string();
  let _logger = LoggerManager::with_config(config).await?;"#
    );

    // 方式 2：Builder 模式（代码展示）
    println!("\nApproach 2: Builder pattern");
    println!(
        r#"  let _logger = LoggerManager::builder()
      .level("info")
      .console(true)
      .file("logs/app.log")
      .build()
      .await?;"#
    );
    println!();

    Ok(())
}

/// 示例 6：优雅关闭
///
/// 演示应用退出前如何正确关闭日志系统。
async fn show_graceful_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 6: graceful shutdown ---\n");

    // 创建独立 logger 演示关闭流程
    let logger = LoggerManager::new().await?;
    tracing::info!("application started");
    tracing::info!("processing business logic...");

    logger.shutdown()?;
    // shutdown 已显式调用，用 mem::forget 阻止 Drop 再次关闭
    mem::forget(logger);

    println!("✓ LoggerManager::shutdown() completed");
    println!("  - All logs left in the channel have been written to sinks");
    println!("  - All sinks are closed");
    println!("  - Worker threads stopped\n");

    Ok(())
}

/// 最佳实践建议
fn show_best_practices() {
    println!("--- Best practices ---\n");

    println!("1. Where to initialize:");
    println!("   - Initialize LoggerManager as early as possible in main()");
    println!("   - Bind the variable with `let _logger = ...` so Drop does not close it early");

    println!("\n2. Choosing the log level:");
    println!("   - Development: debug or trace");
    println!("   - Production: info (default) or warn");
    println!("   - Troubleshooting: temporarily raise to debug, then restore");

    println!("\n3. Structured fields:");
    println!("   - Use tracing structured fields (key = value)");
    println!("   - Avoid concatenating JSON strings into the message");
    println!("   - Common fields: request_id, user_id, component, latency_ms");

    println!("\n4. Error logs:");
    println!("   - error! only for failures that need human intervention");
    println!("   - warn!  for exceptions that recover automatically");
    println!("   - Include error_code and component for easier triage");

    println!("\n5. Shutdown sequence:");
    println!("   - Call logger.shutdown() before the application exits");
    println!("   - Call it from signal handling (SIGTERM / Ctrl+C)");
    println!("   - shutdown() waits until all logs in the channel are written");
}
