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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-runtime-ops-inklog-loggermanager-runtime"));

    show_health_status();
    show_channel_info();
    show_sink_recovery();
    show_shutdown();

    println!("\n{}", tr("example-runtime-ops-all-runtime-operations-api"));
    Ok(())
}

/// 演示 get_health_status() 获取健康状态
fn show_health_status() {
    print_section(&tr("example-runtime-ops-example-1-get-health-status"));

    println!("{}", tr("example-runtime-ops-api-signature"));
    println!("  pub fn get_health_status(&self) -> HealthStatus\n");

    println!("{}", tr("example-runtime-ops-healthstatus-fields"));
    println!("  {}", tr("example-runtime-ops-overall-status-sinkstatus"));
    println!(
        "  {}",
        tr("example-runtime-ops-sinks-hashmap-string-sinkhealth")
    );
    println!("  {}", tr("example-runtime-ops-channel-usage-f64-0-0-1-0"));
    println!("  {}", tr("example-runtime-ops-uptime-seconds-u64"));
    println!("  {}", tr("example-runtime-ops-metrics-metricssnapshot"));
    println!(
        "  {}",
        tr("example-runtime-ops-pool-stats-option-poolstats")
    );
    println!("  {}", tr("example-runtime-ops-encryption-key-valid-bool"));

    println!("\n{}", tr("example-runtime-ops-sinkstatus-enum-values"));
    println!(
        "  {}",
        tr("example-runtime-ops-healthy-sink-running-normally")
    );
    println!("  Degraded {{ reason }}  - sink degraded but still usable");
    println!("  Unhealthy {{ error }}  - sink failed and unusable");
    println!(
        "  {}",
        tr("example-runtime-ops-notstarted-sink-not-started-yet")
    );

    println!("\n{}", tr("example-runtime-ops-use-cases"));
    println!(
        "  {}",
        tr("example-runtime-ops-poll-health-status-periodically")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-integrate-into-the-health-http")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-trigger-alert-notifications")
    );

    println!("\n{}", tr("example-runtime-ops-code-example"));
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
    print_section(&tr("example-runtime-ops-example-2-channel-capacity-and"));

    println!("{}", tr("example-runtime-ops-api-signature"));
    println!("  pub fn effective_channel_capacity(&self) -> usize");
    println!("  pub fn channel_len(&self) -> usize\n");

    println!("{}", tr("example-runtime-ops-config-defaults"));
    let config = InklogConfig::default();
    println!(
        "  {}",
        tr_args(
            "example-runtime-ops-performance-channel-capacity",
            &[("channel_capacity", &config.performance.channel_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-runtime-ops-performance-worker-threads",
            &[("worker_threads", &config.performance.worker_threads)]
        )
    );

    println!("\n{}", tr("example-runtime-ops-use-cases"));
    println!(
        "  {}",
        tr("example-runtime-ops-monitor-channel-usage-to-warn")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-channel-len-effective-channel")
    );
    println!("  {}", tr("example-runtime-ops-adjust-channel-capacity"));

    println!("\n{}", tr("example-runtime-ops-code-example"));
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
    print_section(&tr("example-runtime-ops-example-3-sink-recovery"));

    println!("{}", tr("example-runtime-ops-api-signature"));
    println!("  pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError>");
    println!(
        "  pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError>\n"
    );

    println!("{}", tr("example-runtime-ops-recover-sink-usage"));
    println!(
        "  {}",
        tr("example-runtime-ops-manually-recover-a-sink-by-name")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-sink-names-come-from-the-config")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-returns-err-inklogerror-when")
    );

    println!(
        "\n{}",
        tr("example-runtime-ops-trigger-recovery-for-unhealthy")
    );
    println!("  {}", tr("example-runtime-ops-automatically-scan-all"));
    println!(
        "  {}",
        tr("example-runtime-ops-try-to-recover-every-unhealthy")
    );
    println!("  {}", tr("example-runtime-ops-returns-the-list-of"));

    println!("\n{}", tr("example-runtime-ops-code-example"));
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

    println!("\n{}", tr("example-runtime-ops-recovery-strategies"));
    println!(
        "  {}",
        tr("example-runtime-ops-console-sink-usually-needs-no")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-file-sink-reopen-the-file-handle")
    );
    println!("  {}", tr("example-runtime-ops-database-sink-rebuild-the"));
    println!(
        "  - Fallback: if auto_fallback=true, automatically degrades to the next-priority sink"
    );
}

/// 演示优雅关闭
fn show_shutdown() {
    print_section(&tr("example-runtime-ops-example-4-graceful-shutdown"));

    println!("{}", tr("example-runtime-ops-api-signature"));
    println!("  pub fn shutdown(&self) -> Result<(), InklogError>\n");

    println!("{}", tr("example-runtime-ops-shutdown-behavior"));
    println!(
        "  {}",
        tr("example-runtime-ops-1-stop-accepting-new-log-records")
    );
    println!("  {}", tr("example-runtime-ops-2-wait-for-all-logs-in-the"));
    println!(
        "  {}",
        tr("example-runtime-ops-3-close-all-sinks-file-handles")
    );
    println!("  {}", tr("example-runtime-ops-4-stop-the-http-server-if"));
    println!("  {}", tr("example-runtime-ops-5-stop-all-worker-threads"));

    println!("\n{}", tr("example-runtime-ops-code-example"));
    println!(
        r#"  let logger = LoggerManager::new().await?;

  // Emit some logs
  tracing::info!("application started");
  tracing::info!("processing request...");

  // Graceful shutdown before the app exits
  logger.shutdown()?;
  println!("Logger shut down gracefully");"#
    );

    println!("\n{}", tr("example-runtime-ops-caveats"));
    println!("  {}", tr("example-runtime-ops-shutdown-is-synchronous-it"));
    println!(
        "  {}",
        tr("example-runtime-ops-shutdown-can-take-a-while-if")
    );
    println!(
        "  {}",
        tr("example-runtime-ops-call-shutdown-from-the-app-s")
    );
    println!(
        "  - LoggerManager's Drop impl also tries to close, but without a completion guarantee"
    );

    println!("\n{}", tr("example-runtime-ops-full-lifecycle-example"));
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
