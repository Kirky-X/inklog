// Copyright (c) 2026 Kirky.X🌠
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
    print_separator("inklog LoggerManager runtime operations API example");

    show_health_status();
    show_channel_info();
    show_sink_recovery();
    show_shutdown();

    println!("\nAll runtime operations API examples completed.");
    Ok(())
}

/// 演示 get_health_status() 获取健康状态
fn show_health_status() {
    print_section("Example 1: get_health_status() health state");

    println!("API signature:");
    println!("  pub fn get_health_status(&self) -> HealthStatus\n");

    println!("HealthStatus fields:");
    println!("  overall_status:      SinkStatus (Healthy/Degraded/Unhealthy/NotStarted)");
    println!("  sinks:               HashMap<String, SinkHealth>");
    println!("  channel_usage:       f64 (0.0 ~ 1.0)");
    println!("  uptime_seconds:      u64");
    println!("  metrics:             MetricsSnapshot");
    println!("  pool_stats:          Option<PoolStats>");
    println!("  encryption_key_valid: bool");

    println!("\nSinkStatus enum values:");
    println!("  Healthy             - sink running normally");
    println!("  Degraded {{ reason }}  - sink degraded but still usable");
    println!("  Unhealthy {{ error }}  - sink failed and unusable");
    println!("  NotStarted          - sink not started yet");

    println!("\nUse cases:");
    println!("  - Poll health status periodically (e.g. every 30 seconds)");
    println!("  - Integrate into the /health HTTP endpoint");
    println!("  - Trigger alert notifications");

    println!("\nCode example:");
    println!(
        r#"  let logger = LoggerManager::new().await?;
  let health = logger.get_health_status();
  println!("Overall status: {{:?}}", health.overall_status);
  println!("Channel usage: {{:.1}}%", health.channel_usage * 100.0);
  println!("Uptime: {{}}s", health.uptime_seconds);
  for (name, sink_health) in &health.sinks {{
      println!("  Sink '{{}}': {{:?}}", name, sink_health.status);
  }}"#
    );
}

/// 演示 Channel 容量和队列长度查询
fn show_channel_info() {
    print_section("Example 2: channel capacity and queue length");

    println!("API signature:");
    println!("  pub fn effective_channel_capacity(&self) -> usize");
    println!("  pub fn channel_len(&self) -> usize\n");

    println!("Config defaults:");
    let config = InklogConfig::default();
    println!(
        "  performance.channel_capacity = {} (default)",
        config.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads   = {} (default)",
        config.performance.worker_threads
    );

    println!("\nUse cases:");
    println!("  - Monitor channel usage to warn about backpressure");
    println!("  - channel_len / effective_channel_capacity > 0.8 → alert");
    println!("  - Adjust channel_capacity dynamically (requires rebuilding LoggerManager)");

    println!("\nCode example:");
    println!(
        r#"  let logger = LoggerManager::new().await?;
  let capacity = logger.effective_channel_capacity();
  let queued = logger.channel_len();
  let usage = queued as f64 / capacity as f64;
  println!("Channel: {{}}/{{}} ({{:.1}}%)", queued, capacity, usage * 100.0);
  if usage > 0.8 {{
      eprintln!("Warning: channel usage exceeds 80%!");
  }}"#
    );
}

/// 演示 Sink 恢复操作
fn show_sink_recovery() {
    print_section("Example 3: sink recovery operations");

    println!("API signature:");
    println!("  pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError>");
    println!(
        "  pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError>\n"
    );

    println!("recover_sink() usage:");
    println!("  - Manually recover a sink by name");
    println!("  - Sink names come from the config (e.g. \"console\", \"file\", \"database\")");
    println!("  - Returns Err(InklogError) when recovery fails");

    println!("\ntrigger_recovery_for_unhealthy_sinks() usage:");
    println!("  - Automatically scan all unhealthy sinks");
    println!("  - Try to recover every unhealthy sink");
    println!("  - Returns the list of successfully recovered sink names");

    println!("\nCode example:");
    println!(
        r#"  let logger = LoggerManager::new().await?;

  // Option 1: manually recover a named sink
  if let Err(e) = logger.recover_sink("database") {{
      eprintln!("failed to recover the database sink: {{}}", e);
  }}

  // Option 2: batch-recover all unhealthy sinks
  match logger.trigger_recovery_for_unhealthy_sinks() {{
      Ok(recovered) => {{
          if recovered.is_empty() {{
              println!("All sinks healthy; no recovery needed");
          }} else {{
              println!("Recovered sinks: {{:?}}", recovered);
          }}
      }}
      Err(e) => eprintln!("recovery failed: {{}}", e),
  }}"#
    );

    println!("\nRecovery strategies:");
    println!("  - Console sink: usually needs no recovery (writes straight to stdout)");
    println!("  - File sink: reopen the file handle");
    println!("  - Database sink: rebuild the connection pool");
    println!(
        "  - Fallback: if auto_fallback=true, automatically degrades to the next-priority sink"
    );
}

/// 演示优雅关闭
fn show_shutdown() {
    print_section("Example 4: graceful shutdown with shutdown()");

    println!("API signature:");
    println!("  pub fn shutdown(&self) -> Result<(), InklogError>\n");

    println!("shutdown() behavior:");
    println!("  1. Stop accepting new log records");
    println!("  2. Wait for all logs in the channel to be written to sinks");
    println!("  3. Close all sinks (file handles, database connections, etc.)");
    println!("  4. Stop the HTTP server (if enabled)");
    println!("  5. Stop all worker threads");

    println!("\nCode example:");
    println!(
        r#"  let logger = LoggerManager::new().await?;

  // Emit some logs
  tracing::info!("application started");
  tracing::info!("processing request...");

  // Graceful shutdown before the app exits
  logger.shutdown()?;
  println!("Logger shut down gracefully");"#
    );

    println!("\nCaveats:");
    println!("  - shutdown() is synchronous; it blocks until all logs are written");
    println!("  - shutdown() can take a while if the channel holds many pending logs");
    println!("  - Call shutdown() from the app's exit signal handler");
    println!(
        "  - LoggerManager's Drop impl also tries to close, but without a completion guarantee"
    );

    println!("\nFull lifecycle example:");
    println!(
        r#"  // 1. Initialize
  let logger = LoggerManager::new().await?;

  // 2. Run (emit logs)
  tracing::info!("application running");

  // 3. Monitor
  let health = logger.get_health_status();
  assert!(health.overall_status.is_operational());

  // 4. Shut down
  logger.shutdown()?;"#
    );
}
