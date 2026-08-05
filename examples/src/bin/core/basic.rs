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
    println!("=== inklog 基础用法示例 ===\n");

    // 在 main 中创建并持有 logger，确保所有示例的 tracing 调用都有活跃 subscriber
    let _logger = LoggerManager::new().await?;

    show_default_initialization().await?;
    show_log_levels();
    show_structured_logging();
    show_health_verification();
    show_config_initialization().await?;
    show_graceful_shutdown().await?;
    show_best_practices();

    println!("\n所有基础示例演示完毕。");
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
    println!("--- 示例 1：默认初始化 ---\n");

    // logger 已在 main() 中创建，此处展示初始化模式
    println!("✓ LoggerManager::new() 初始化成功（logger 在 main 中创建）");
    tracing::info!("日志系统已启动");
    println!("✓ 日志记录正常\n");

    Ok(())
}

/// 示例 2：不同日志级别
///
/// 演示 5 个标准日志级别的使用场景。
fn show_log_levels() {
    println!("--- 示例 2：日志级别 ---\n");

    println!("5 个标准日志级别：");
    tracing::trace!("TRACE - 最详细的诊断信息，仅开发环境启用");
    tracing::debug!("DEBUG - 调试信息，帮助定位问题");
    tracing::info!("INFO - 常规运行信息，标记重要事件");
    tracing::warn!("WARN - 潜在问题，不影响正常运行");
    tracing::error!("ERROR - 功能失败，需要关注");

    println!("\n级别过滤规则（默认 info）：");
    println!("  trace → 仅 tracing::trace! 可见");
    println!("  debug → debug 及以上可见");
    println!("  info  → info/warn/error 可见（默认）");
    println!("  warn  → warn/error 可见");
    println!("  error → 仅 error 可见");
    println!();
}

/// 示例 3：结构化日志
///
/// 演示带结构化字段的日志记录，便于后续查询和分析。
fn show_structured_logging() {
    println!("--- 示例 3：结构化日志 ---\n");

    // 用户操作日志
    tracing::info!(
        user_id = 12345,
        action = "login",
        ip = "192.168.1.1",
        "用户登录成功"
    );
    println!("✓ 用户操作日志已记录（user_id, action, ip）");

    // 请求处理日志
    tracing::info!(
        request_id = "req-abc-123",
        method = "GET",
        path = "/api/users",
        status = 200,
        latency_ms = 42,
        "请求处理完成"
    );
    println!("✓ 请求日志已记录（request_id, method, path, status, latency_ms）");

    // 错误日志
    tracing::error!(
        error_code = "E001",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "支付处理失败，已达重试上限"
    );
    println!("✓ 错误日志已记录（error_code, component, retry_count）\n");
}

/// 示例 4：健康状态验证
///
/// 演示如何检查 LoggerManager 的运行状态，确认系统健康。
fn show_health_verification() {
    println!("--- 示例 4：健康状态验证 ---\n");

    println!("注意：健康状态需在 LoggerManager 实例上调用。");
    println!("此示例展示 API 用法（需持有 logger 引用）：\n");

    println!(
        r#"  let logger = LoggerManager::new().await?;
  let health = logger.get_health_status();
  
  // 验证整体状态
  assert!(health.overall_status.is_operational());
  
  // 查看 Channel 使用情况
  let capacity = logger.effective_channel_capacity();
  let queued = logger.channel_len();
  println!("Channel: {{}}/{{}}", queued, capacity);
  
  // 遍历各 Sink 状态
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
    println!("--- 示例 5：自定义配置初始化 ---\n");

    // 方式 1：修改默认配置（代码展示）
    println!("方式 1：修改默认配置");
    println!(
        r#"  let mut config = InklogConfig::default();
  config.global.level = "debug".to_string();
  let _logger = LoggerManager::with_config(config).await?;"#
    );

    // 方式 2：Builder 模式（代码展示）
    println!("\n方式 2：Builder 模式");
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
    println!("--- 示例 6：优雅关闭 ---\n");

    // 创建独立 logger 演示关闭流程
    let logger = LoggerManager::new().await?;
    tracing::info!("应用启动");
    tracing::info!("处理业务逻辑...");

    // 应用退出前优雅关闭
    logger.shutdown()?;
    // shutdown 已显式调用，用 mem::forget 阻止 Drop 再次关闭
    mem::forget(logger);

    println!("✓ LoggerManager::shutdown() 完成");
    println!("  - Channel 中剩余日志已全部写入 Sink");
    println!("  - 所有 Sink 已关闭");
    println!("  - Worker 线程已停止\n");

    Ok(())
}

/// 最佳实践建议
fn show_best_practices() {
    println!("--- 最佳实践 ---\n");

    println!("1. 初始化位置：");
    println!("   - 在 main() 函数最早处初始化 LoggerManager");
    println!("   - 使用 let _logger = ... 绑定变量，防止 Drop 提前关闭");

    println!("\n2. 日志级别选择：");
    println!("   - 开发环境: debug 或 trace");
    println!("   - 生产环境: info（默认）或 warn");
    println!("   - 故障排查: 临时调至 debug，排查后恢复");

    println!("\n3. 结构化字段：");
    println!("   - 使用 tracing 的结构化字段（key = value）");
    println!("   - 避免在 message 中拼接 JSON 字符串");
    println!("   - 常用字段: request_id, user_id, component, latency_ms");

    println!("\n4. 错误日志：");
    println!("   - error! 仅用于需要人工介入的失败");
    println!("   - warn!  用于可自动恢复的异常");
    println!("   - 包含 error_code 和 component 便于定位");

    println!("\n5. 关闭流程：");
    println!("   - 应用退出前调用 logger.shutdown()");
    println!("   - 在信号处理中调用（SIGTERM / Ctrl+C）");
    println!("   - shutdown() 会等待 Channel 中的日志全部写入");
}
