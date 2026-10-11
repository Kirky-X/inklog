// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 生产环境配置示例
//!
//! 演示如何在不同环境（开发/预发布/生产）中配置 inklog。
//!
//! # 内容
//!
//! 1. 环境差异化配置对比（dev / staging / prod）
//! 2. Builder 模式完整配置
//! 3. 配置文件（TOML）格式
//! 4. 初始化后健康状态验证
//! 5. 优雅关闭流程
//! 6. 生产环境最佳实践
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin production
//! ```

use inklog::LoggerManager;
use inklog_examples::common::{tr, tr_args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-production-inklog-production-configuration")
    );

    show_env_comparison();
    show_builder_production_config().await?;
    show_toml_config();
    show_health_check_after_init().await?;
    show_graceful_shutdown().await?;
    show_error_handling().await;
    show_best_practices();

    println!("\n{}", tr("example-production-all-production-examples"));
    Ok(())
}

/// 示例 1：环境差异化配置对比
///
/// 展示开发、预发布、生产三种环境的典型配置差异。
fn show_env_comparison() {
    println!("{}\n", tr("example-production-example-1-per-environment"));

    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Setting", "dev", "staging", "prod"
    );
    println!("{}", "-".repeat(80));
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Log level", "debug", "info", "info"
    );
    println!("{:<20} {:<20} {:<20} {:<20}", "Console", "on", "on", "on");
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "File sink", "off", "on", "on"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Compression", "off", "Zstd", "Zstd"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Rotation", "off", "daily", "daily"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Keep files", "N/A", "7", "30"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Channel capacity", "1000", "5000", "10000"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Worker threads", "1", "2", "4"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "HTTP server", "off", "on", "on+TLS"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Database sink", "off", "off", "on"
    );
    println!();
}

/// 示例 2：Builder 模式完整生产配置
///
/// 使用 Builder 模式构建生产级配置，实际初始化并验证。
async fn show_builder_production_config() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-production-example-2-builder-pattern-for")
    );

    let logger = LoggerManager::builder()
        .level("info")
        .format("{timestamp} [{level}] {target} - {message}")
        .console(true)
        .console_colored(true)
        .file("logs/production.log")
        .file_max_size("100MB")
        .file_compress(true)
        .file_rotation_time("daily")
        .file_keep_files(30)
        .channel_capacity(10000)
        .worker_threads(4)
        .build()
        .await?;

    println!("{}", tr("example-production-production-configuration"));
    println!("  {}", tr("example-production-log-level-info"));
    println!("  {}", tr("example-production-console-on-colored"));
    println!(
        "  {}",
        tr("example-production-file-logs-production-log-100mb")
    );
    println!("  {}", tr("example-production-compression-zstd"));
    println!("  {}", tr("example-production-channel-10000"));
    println!("  {}\n", tr("example-production-workers-4"));

    // 验证配置生效
    tracing::info!("{}", tr("example-production-production-log-system-started"));
    tracing::debug!("{}", tr("example-production-this-debug-log-is-not-emitted"));
    println!(
        "{}",
        tr("example-production-level-filter-verified-debug-is")
    );

    // 模拟业务日志
    for i in 1..=3 {
        tracing::info!(
            request_id = %uuid::Uuid::new_v4(),
            iteration = i,
            "handling request"
        );
    }
    println!("{}", tr("example-production-3-business-logs-recorded"));

    tracing::warn!(
        component = "database",
        latency_ms = 150,
        threshold_ms = 100,
        "database query latency exceeded the threshold"
    );
    println!("{}", tr("example-production-alert-log-recorded"));

    tracing::error!(
        error_code = "E001",
        component = "cache",
        retry_count = 3,
        "cache connection failed"
    );
    println!("{}\n", tr("example-production-error-log-recorded"));

    logger.shutdown()?;
    println!("{}\n", tr("example-production-shut-down-gracefully"));

    Ok(())
}

/// 示例 3：TOML 配置文件格式
///
/// 展示生产环境推荐的 TOML 配置文件内容。
fn show_toml_config() {
    println!(
        "{}\n",
        tr("example-production-example-3-toml-configuration")
    );

    println!("{}", tr("example-production-inklog-toml-recommended"));
    println!(
        r#"[global]
level = "info"
format = "{{timestamp}} [{{level}}] {{target}} - {{message}}"

[console_sink]
enabled = true

[file_sink]
enabled = true
path = "logs/production.log"
max_size = "100MB"
rotation = "daily"
keep_files = 30
compress = true

[performance]
channel_capacity = 10000
worker_threads = 4

[http_server]
enabled = true
host = "127.0.0.1"
port = 9090

[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/inklog"
batch_size = 100
flush_interval_ms = 500"#
    );

    println!("\n{}", tr("example-production-how-to-load"));
    println!("  // Search for inklog.toml or INKLOG_CONFIG_PATH automatically");
    println!("  let _logger = LoggerManager::load().await?;");
    println!();
    println!("  // Specify the configuration file path");
    println!("  let _logger = LoggerManager::from_file(\"/etc/inklog/prod.toml\").await?;");
    println!();
}

/// 示例 4：初始化后健康状态验证
///
/// 演示初始化后如何验证系统健康状态。
async fn show_health_check_after_init() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-production-example-4-health-check-after")
    );

    let logger = LoggerManager::builder()
        .level("info")
        .console(true)
        .build()
        .await?;

    let health = logger.get_health_status();
    println!("{}", tr("example-production-health-check"));
    println!(
        "  {}",
        tr_args(
            "example-production-overall-status",
            &[("overall_status", &format!("{:?}", health.overall_status))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-production-uptime-seconds-s",
            &[("uptime_seconds", &health.uptime_seconds)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-production-channel-usage",
            &[("value", &format!("{:.1}", health.channel_usage * 100.0))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-production-encryption-valid",
            &[("encryption_key_valid", &health.encryption_key_valid)]
        )
    );

    // 验证健康状态可获取（状态取决于 Sink 初始化时机）
    println!("  {}", tr("example-production-health-status-retrieved"));

    // 写入日志后再检查状态
    tracing::info!("{}", tr("example-production-health-check-log"));
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let health2 = logger.get_health_status();
    println!(
        "  {}",
        tr_args(
            "example-production-overall-status-after-writing-a",
            &[("overall_status", &format!("{:?}", health2.overall_status))]
        )
    );

    let capacity = logger.effective_channel_capacity();
    let queued = logger.channel_len();
    println!("\n{}", tr("example-production-channel-info"));
    println!(
        "  {}",
        tr_args("example-production-capacity", &[("capacity", &capacity)])
    );
    println!(
        "  {}",
        tr_args("example-production-queued", &[("queued", &queued)])
    );
    assert!(capacity > 0, "channel capacity must be greater than 0");
    println!("  {}\n", tr("example-production-channel-verified"));

    logger.shutdown()?;
    Ok(())
}

/// 示例 5：优雅关闭流程
///
/// 演示应用退出前的完整关闭流程。
async fn show_graceful_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-production-example-5-graceful-shutdown"));

    let logger = LoggerManager::builder()
        .level("info")
        .console(true)
        .channel_capacity(1000)
        .build()
        .await?;

    // 模拟业务运行
    for i in 1..=5 {
        tracing::info!(batch = i, "processing batch");
    }
    println!(
        "{}",
        tr("example-production-5-logs-written-into-the-channel")
    );

    // 关闭前检查
    let queued_before = logger.channel_len();
    println!(
        "  {}",
        tr_args(
            "example-production-channel-queue-before-shutdown",
            &[("queued_before", &queued_before)]
        )
    );

    logger.shutdown()?;
    println!("{}", tr("example-production-shutdown-completed"));
    println!("  {}", tr("example-production-all-logs-persisted-to-sinks"));
    println!("  {}", tr("example-production-worker-threads-stopped"));
    println!("  {}\n", tr("example-production-resources-released"));

    Ok(())
}

/// 示例 6：错误处理
///
/// 演示配置初始化可能遇到的错误及处理方式。
async fn show_error_handling() {
    println!("{}\n", tr("example-production-example-6-error-handling"));

    println!("{}", tr("example-production-scenario-1-load-a-nonexistent"));
    match LoggerManager::from_file("/nonexistent/config.toml").await {
        Ok(_) => println!("  {}", tr("example-production-unexpected-success")),
        Err(e) => {
            println!(
                "  {}",
                tr_args("example-production-expected-error", &[("e", &e)])
            );
            println!(
                "  Handling: fall back to the default config or ask the user to check the path"
            );
        }
    }

    // 场景 2：无效的日志级别（Builder 验证）
    println!(
        "\n{}",
        tr("example-production-scenario-2-an-invalid-log-level")
    );
    match LoggerManager::builder()
        .level("invalid_level")
        .build()
        .await
    {
        Ok(_) => println!("  {}", tr("example-production-unexpected-success")),
        Err(e) => {
            println!(
                "  {}",
                tr_args("example-production-expected-error", &[("e", &e)])
            );
            println!("  {}", tr("example-production-handling-use-a-valid-level"));
        }
    }

    println!("\n{}", tr("example-production-error-handling-advice"));
    println!("  {}", tr("example-production-always-handle-loggermanager"));
    println!(
        "  {}",
        tr("example-production-give-a-meaningful-error-message")
    );
    println!(
        "  {}\n",
        tr("example-production-consider-a-config-fallback")
    );
}

/// 最佳实践建议
fn show_best_practices() {
    println!("{}\n", tr("example-production-production-best-practices"));

    println!("{}", tr("example-production-1-configuration-management"));
    println!(
        "   {}",
        tr("example-production-keep-configuration-in-a-toml")
    );
    println!("   {}", tr("example-production-inject-secrets-such-as-the"));
    println!(
        "   {}",
        tr("example-production-use-a-different-configuration")
    );

    println!("\n{}", tr("example-production-2-file-rotation"));
    println!("   {}", tr("example-production-production-must-enable-log"));
    println!(
        "   {}",
        tr("example-production-recommended-daily-rotation-30")
    );
    println!(
        "   {}",
        tr("example-production-enable-zstd-compression-to-save")
    );

    println!("\n{}", tr("example-production-3-performance-tuning"));
    println!(
        "   {}",
        tr("example-production-channel-capacity-size-it-for")
    );
    println!(
        "   {}",
        tr("example-production-worker-threads-match-the-cpu")
    );
    println!(
        "   {}",
        tr("example-production-watch-channel-len-capacity-and")
    );

    println!("\n{}", tr("example-production-4-monitoring-integration"));
    println!("   {}", tr("example-production-enable-the-http-server-to"));
    println!("   {}", tr("example-production-have-prometheus-scrape-the"));
    println!("   {}", tr("example-production-configure-alert-rules-on"));

    println!("\n{}", tr("example-production-5-security-hardening"));
    println!(
        "   {}",
        tr("example-production-enable-tls-on-the-production")
    );
    println!(
        "   {}",
        tr("example-production-configure-an-ip-whitelist-to")
    );
    println!("   {}", tr("example-production-enable-bearer-token"));
    println!("   {}", tr("example-production-use-logsanitizer-to-redact"));
}
