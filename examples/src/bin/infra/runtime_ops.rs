// SPDX-License-Identifier: MIT
//! LoggerManager 运行时操作 API 示例
//!
//! 演示 `LoggerManager` 的运行时监控和运维操作：
//!
//! 1. `get_health_status()` - 获取整体健康状态
//! 2. `effective_channel_capacity()` - 查看有效 Channel 容量
//! 3. `channel_len()` - 查看当前 Channel 队列长度
//! 4. `recover_sink()` - 手动恢复故障 Sink
//! 5. `trigger_recovery_for_unhealthy_sinks()` - 批量恢复不健康 Sink
//! 6. `shutdown()` - 优雅关闭
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin runtime_ops
//! ```

use inklog::InklogConfig;
use inklog_examples::common::{print_section, print_separator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("inklog LoggerManager 运行时操作 API 示例");

    show_health_status();
    show_channel_info();
    show_sink_recovery();
    show_shutdown();

    println!("\n所有运行时操作 API 示例展示完毕。");
    Ok(())
}

/// 演示 get_health_status() 获取健康状态
fn show_health_status() {
    print_section("示例 1：get_health_status() 健康状态");

    println!("API 签名：");
    println!("  pub fn get_health_status(&self) -> HealthStatus\n");

    println!("HealthStatus 字段：");
    println!("  overall_status:      SinkStatus (Healthy/Degraded/Unhealthy/NotStarted)");
    println!("  sinks:               HashMap<String, SinkHealth>");
    println!("  channel_usage:       f64 (0.0 ~ 1.0)");
    println!("  uptime_seconds:      u64");
    println!("  metrics:             MetricsSnapshot");
    println!("  pool_stats:          Option<PoolStats>");
    println!("  encryption_key_valid: bool");

    println!("\nSinkStatus 枚举值：");
    println!("  Healthy             - Sink 正常运行");
    println!("  Degraded {{ reason }}  - Sink 降级但仍可用");
    println!("  Unhealthy {{ error }}  - Sink 故障不可用");
    println!("  NotStarted          - Sink 尚未启动");

    println!("\n使用场景：");
    println!("  - 定期轮询健康状态（如每 30 秒）");
    println!("  - 集成到 /health HTTP 端点");
    println!("  - 触发告警通知");

    println!("\n代码示例：");
    println!(
        r#"  let logger = LoggerManager::new().await?;
  let health = logger.get_health_status();
  println!("整体状态: {{:?}}", health.overall_status);
  println!("Channel 使用率: {{:.1}}%", health.channel_usage * 100.0);
  println!("运行时间: {{}}s", health.uptime_seconds);
  for (name, sink_health) in &health.sinks {{
      println!("  Sink '{{}}': {{:?}}", name, sink_health.status);
  }}"#
    );
}

/// 演示 Channel 容量和队列长度查询
fn show_channel_info() {
    print_section("示例 2：Channel 容量与队列长度");

    println!("API 签名：");
    println!("  pub fn effective_channel_capacity(&self) -> usize");
    println!("  pub fn channel_len(&self) -> usize\n");

    println!("配置默认值：");
    let config = InklogConfig::default();
    println!(
        "  performance.channel_capacity = {} (默认)",
        config.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads   = {} (默认)",
        config.performance.worker_threads
    );

    println!("\n使用场景：");
    println!("  - 监控 Channel 使用率，预警背压");
    println!("  - channel_len / effective_channel_capacity > 0.8 → 告警");
    println!("  - 动态调整 channel_capacity（需重建 LoggerManager）");

    println!("\n代码示例：");
    println!(
        r#"  let logger = LoggerManager::new().await?;
  let capacity = logger.effective_channel_capacity();
  let queued = logger.channel_len();
  let usage = queued as f64 / capacity as f64;
  println!("Channel: {{}}/{{}} ({{:.1}}%)", queued, capacity, usage * 100.0);
  if usage > 0.8 {{
      eprintln!("警告: Channel 使用率超过 80%!");
  }}"#
    );
}

/// 演示 Sink 恢复操作
fn show_sink_recovery() {
    print_section("示例 3：Sink 恢复操作");

    println!("API 签名：");
    println!("  pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError>");
    println!(
        "  pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError>\n"
    );

    println!("recover_sink() 用法：");
    println!("  - 手动恢复指定名称的 Sink");
    println!("  - Sink 名称来自配置（如 \"console\", \"file\", \"database\"）");
    println!("  - 恢复失败返回 Err(InklogError)");

    println!("\ntrigger_recovery_for_unhealthy_sinks() 用法：");
    println!("  - 自动扫描所有不健康的 Sink");
    println!("  - 尝试恢复每个不健康的 Sink");
    println!("  - 返回成功恢复的 Sink 名称列表");

    println!("\n代码示例：");
    println!(
        r#"  let logger = LoggerManager::new().await?;

  // 方式 1：手动恢复指定 Sink
  if let Err(e) = logger.recover_sink("database") {{
      eprintln!("恢复 database Sink 失败: {{}}", e);
  }}

  // 方式 2：批量恢复所有不健康 Sink
  match logger.trigger_recovery_for_unhealthy_sinks() {{
      Ok(recovered) => {{
          if recovered.is_empty() {{
              println!("所有 Sink 均健康，无需恢复");
          }} else {{
              println!("已恢复 Sink: {{:?}}", recovered);
          }}
      }}
      Err(e) => eprintln!("恢复操作失败: {{}}", e),
  }}"#
    );

    println!("\n恢复策略：");
    println!("  - Console Sink: 通常不需要恢复（直接写 stdout）");
    println!("  - File Sink: 重新打开文件句柄");
    println!("  - Database Sink: 重新建立连接池");
    println!("  - 降级机制: 若 auto_fallback=true，自动降级到下一优先级 Sink");
}

/// 演示优雅关闭
fn show_shutdown() {
    print_section("示例 4：shutdown() 优雅关闭");

    println!("API 签名：");
    println!("  pub fn shutdown(&self) -> Result<(), InklogError>\n");

    println!("shutdown() 行为：");
    println!("  1. 停止接收新的日志记录");
    println!("  2. 等待 Channel 中的日志全部写入 Sink");
    println!("  3. 关闭所有 Sink（文件句柄、数据库连接等）");
    println!("  4. 停止 HTTP 服务器（如已启用）");
    println!("  5. 停止所有 Worker 线程");

    println!("\n代码示例：");
    println!(
        r#"  let logger = LoggerManager::new().await?;

  // 记录一些日志
  tracing::info!("应用启动");
  tracing::info!("处理请求...");

  // 应用退出前优雅关闭
  logger.shutdown()?;
  println!("Logger 已优雅关闭");"#
    );

    println!("\n注意事项：");
    println!("  - shutdown() 是同步操作，会阻塞直到所有日志写入完成");
    println!("  - 如果 Channel 中有大量未处理日志，shutdown 可能耗时较长");
    println!("  - 建议在应用退出信号处理中调用 shutdown()");
    println!("  - LoggerManager 的 Drop 实现也会尝试关闭，但不保证完成");

    println!("\n完整生命周期示例：");
    println!(
        r#"  // 1. 初始化
  let logger = LoggerManager::new().await?;

  // 2. 运行（记录日志）
  tracing::info!("应用运行中");

  // 3. 监控
  let health = logger.get_health_status();
  assert!(health.overall_status.is_operational());

  // 4. 关闭
  logger.shutdown()?;"#
    );
}
