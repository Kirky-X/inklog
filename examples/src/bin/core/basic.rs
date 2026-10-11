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
use inklog_examples::common::tr;
use std::mem;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-basic-inklog-basic-usage-examples"));

    // 在 main 中创建并持有 logger，确保所有示例的 tracing 调用都有活跃 subscriber
    let _logger = LoggerManager::new().await?;

    show_default_initialization().await?;
    show_log_levels();
    show_structured_logging();
    show_health_verification();
    show_config_initialization().await?;
    show_graceful_shutdown().await?;
    show_best_practices();

    println!("\n{}", tr("example-basic-all-basic-examples-completed"));
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
    println!("{}\n", tr("example-basic-example-1-default-initialization"));

    // logger 已在 main() 中创建，此处展示初始化模式
    println!("{}", tr("example-basic-loggermanager-new-initialized"));
    tracing::info!("{}", tr("example-basic-log-system-started"));
    println!("{}\n", tr("example-basic-logging-works"));

    Ok(())
}

/// 示例 2：不同日志级别
///
/// 演示 5 个标准日志级别的使用场景。
fn show_log_levels() {
    println!("{}\n", tr("example-basic-example-2-log-levels"));

    println!("{}", tr("example-basic-5-standard-log-levels"));
    tracing::trace!("{}", tr("example-basic-trace-most-detailed-diagnostics"));
    tracing::debug!("{}", tr("example-basic-debug-debugging-information-to"));
    tracing::info!("{}", tr("example-basic-info-routine-runtime"));
    tracing::warn!("{}", tr("example-basic-warn-potential-issue-that-does"));
    tracing::error!("{}", tr("example-basic-error-functionality-failure"));

    println!("\n{}", tr("example-basic-level-filtering-rules-default"));
    println!("  {}", tr("example-basic-trace-only-tracing-trace-visible"));
    println!("  {}", tr("example-basic-debug-debug-and-above-visible"));
    println!("  {}", tr("example-basic-info-info-warn-error-visible"));
    println!("  {}", tr("example-basic-warn-warn-error-visible"));
    println!("  {}", tr("example-basic-error-only-error-visible"));
    println!();
}

/// 示例 3：结构化日志
///
/// 演示带结构化字段的日志记录，便于后续查询和分析。
fn show_structured_logging() {
    println!("{}\n", tr("example-basic-example-3-structured-logging"));

    // 用户操作日志
    tracing::info!(
        user_id = 12345,
        action = "login",
        ip = "192.168.1.1",
        "user logged in successfully"
    );
    println!("{}", tr("example-basic-user-operation-logged-user-id"));

    // 请求处理日志
    tracing::info!(
        request_id = "req-abc-123",
        method = "GET",
        path = "/api/users",
        status = 200,
        latency_ms = 42,
        "request completed"
    );
    println!("{}", tr("example-basic-request-logged-request-id"));

    // 错误日志
    tracing::error!(
        error_code = "E001",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "payment processing failed; retry limit reached"
    );
    println!("{}\n", tr("example-basic-error-logged-error-code"));
}

/// 示例 4：健康状态验证
///
/// 演示如何检查 LoggerManager 的运行状态，确认系统健康。
fn show_health_verification() {
    println!("{}\n", tr("example-basic-example-4-health-status-check"));

    println!("{}", tr("example-basic-note-health-status-must-be"));
    println!("{}\n", tr("example-basic-this-example-shows-the-api"));

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
    println!("{}\n", tr("example-basic-example-5-custom-configuration"));

    // 方式 1：修改默认配置（代码展示）
    println!("{}", tr("example-basic-approach-1-modify-the-default"));
    println!(
        r#"  let mut config = InklogConfig::default();
  config.global.level = "debug".to_string();
  let _logger = LoggerManager::with_config(config).await?;"#
    );

    // 方式 2：Builder 模式（代码展示）
    println!("\n{}", tr("example-basic-approach-2-builder-pattern"));
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
    println!("{}\n", tr("example-basic-example-6-graceful-shutdown"));

    // 创建独立 logger 演示关闭流程
    let logger = LoggerManager::new().await?;
    tracing::info!("{}", tr("example-basic-application-started"));
    tracing::info!("{}", tr("example-basic-processing-business-logic"));

    logger.shutdown()?;
    // shutdown 已显式调用，用 mem::forget 阻止 Drop 再次关闭
    mem::forget(logger);

    println!("{}", tr("example-basic-loggermanager-shutdown-completed"));
    println!("  {}", tr("example-basic-all-logs-left-in-the-channel"));
    println!("  {}", tr("example-basic-all-sinks-are-closed"));
    println!("  {}\n", tr("example-basic-worker-threads-stopped"));

    Ok(())
}

/// 最佳实践建议
fn show_best_practices() {
    println!("{}\n", tr("example-basic-best-practices"));

    println!("{}", tr("example-basic-1-where-to-initialize"));
    println!("   {}", tr("example-basic-initialize-loggermanager-as"));
    println!("   {}", tr("example-basic-bind-the-variable-with-let"));

    println!("\n{}", tr("example-basic-2-choosing-the-log-level"));
    println!("   {}", tr("example-basic-development-debug-or-trace"));
    println!("   {}", tr("example-basic-production-info-default-or-warn"));
    println!("   {}", tr("example-basic-troubleshooting-temporarily"));

    println!("\n{}", tr("example-basic-3-structured-fields"));
    println!("   {}", tr("example-basic-use-tracing-structured-fields"));
    println!("   {}", tr("example-basic-avoid-concatenating-json"));
    println!("   {}", tr("example-basic-common-fields-request-id-user"));

    println!("\n{}", tr("example-basic-4-error-logs"));
    println!("   {}", tr("example-basic-error-only-for-failures-that"));
    println!("   {}", tr("example-basic-warn-for-exceptions-that"));
    println!("   {}", tr("example-basic-include-error-code-and"));

    println!("\n{}", tr("example-basic-5-shutdown-sequence"));
    println!("   {}", tr("example-basic-call-logger-shutdown-before-the"));
    println!("   {}", tr("example-basic-call-it-from-signal-handling"));
    println!("   {}", tr("example-basic-shutdown-waits-until-all-logs"));
}
