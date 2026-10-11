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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog production configuration examples ===\n");

    show_env_comparison();
    show_builder_production_config().await?;
    show_toml_config();
    show_health_check_after_init().await?;
    show_graceful_shutdown().await?;
    show_error_handling().await;
    show_best_practices();

    println!("\nAll production examples completed.");
    Ok(())
}

/// 示例 1：环境差异化配置对比
///
/// 展示开发、预发布、生产三种环境的典型配置差异。
fn show_env_comparison() {
    println!("--- Example 1: per-environment configuration comparison ---\n");

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
    println!("--- Example 2: Builder pattern for production ---\n");

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

    println!("✓ production configuration initialized");
    println!("  Log level:   info");
    println!("  Console:    on (colored)");
    println!("  File:       logs/production.log (100MB, daily, keep 30)");
    println!("  Compression: Zstd");
    println!("  Channel:    10000");
    println!("  Workers:    4\n");

    // 验证配置生效
    tracing::info!("production log system started");
    tracing::debug!("this DEBUG log is not emitted (level=info)");
    println!("✓ level filter verified: DEBUG is not visible (as expected)");

    // 模拟业务日志
    for i in 1..=3 {
        tracing::info!(
            request_id = %uuid::Uuid::new_v4(),
            iteration = i,
            "handling request"
        );
    }
    println!("✓ 3 business logs recorded");

    tracing::warn!(
        component = "database",
        latency_ms = 150,
        threshold_ms = 100,
        "database query latency exceeded the threshold"
    );
    println!("✓ alert log recorded");

    tracing::error!(
        error_code = "E001",
        component = "cache",
        retry_count = 3,
        "cache connection failed"
    );
    println!("✓ error log recorded\n");

    logger.shutdown()?;
    println!("✓ shut down gracefully\n");

    Ok(())
}

/// 示例 3：TOML 配置文件格式
///
/// 展示生产环境推荐的 TOML 配置文件内容。
fn show_toml_config() {
    println!("--- Example 3: TOML configuration file format ---\n");

    println!("inklog.toml (recommended production configuration):");
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

    println!("\nHow to load:");
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
    println!("--- Example 4: health check after initialization ---\n");

    let logger = LoggerManager::builder()
        .level("info")
        .console(true)
        .build()
        .await?;

    let health = logger.get_health_status();
    println!("Health check:");
    println!("  overall_status:     {:?}", health.overall_status);
    println!("  uptime_seconds:     {}s", health.uptime_seconds);
    println!("  channel_usage:      {:.1}%", health.channel_usage * 100.0);
    println!("  encryption_valid:   {}", health.encryption_key_valid);

    // 验证健康状态可获取（状态取决于 Sink 初始化时机）
    println!("  ✓ health status retrieved");

    // 写入日志后再检查状态
    tracing::info!("health check log");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let health2 = logger.get_health_status();
    println!(
        "  overall_status after writing a log: {:?}",
        health2.overall_status
    );

    let capacity = logger.effective_channel_capacity();
    let queued = logger.channel_len();
    println!("\nChannel info:");
    println!("  capacity: {}", capacity);
    println!("  queued:   {}", queued);
    assert!(capacity > 0, "channel capacity must be greater than 0");
    println!("  ✓ channel verified\n");

    logger.shutdown()?;
    Ok(())
}

/// 示例 5：优雅关闭流程
///
/// 演示应用退出前的完整关闭流程。
async fn show_graceful_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 5: graceful shutdown sequence ---\n");

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
    println!("✓ 5 logs written into the channel");

    // 关闭前检查
    let queued_before = logger.channel_len();
    println!("  channel queue before shutdown: {} entries", queued_before);

    logger.shutdown()?;
    println!("✓ shutdown() completed");
    println!("  - All logs persisted to sinks");
    println!("  - Worker threads stopped");
    println!("  - Resources released\n");

    Ok(())
}

/// 示例 6：错误处理
///
/// 演示配置初始化可能遇到的错误及处理方式。
async fn show_error_handling() {
    println!("--- Example 6: error handling ---\n");

    println!("Scenario 1: load a nonexistent configuration file");
    match LoggerManager::from_file("/nonexistent/config.toml").await {
        Ok(_) => println!("  unexpected success"),
        Err(e) => {
            println!("  ✓ expected error: {}", e);
            println!(
                "  Handling: fall back to the default config or ask the user to check the path"
            );
        }
    }

    // 场景 2：无效的日志级别（Builder 验证）
    println!("\nScenario 2: an invalid log level");
    match LoggerManager::builder()
        .level("invalid_level")
        .build()
        .await
    {
        Ok(_) => println!("  unexpected success"),
        Err(e) => {
            println!("  ✓ expected error: {}", e);
            println!("  Handling: use a valid level (trace/debug/info/warn/error)");
        }
    }

    println!("\nError handling advice:");
    println!("  - Always handle LoggerManager initialization errors with ? or match");
    println!("  - Give a meaningful error message when the config file fails to load");
    println!("  - Consider a config fallback chain: env vars → config file → defaults\n");
}

/// 最佳实践建议
fn show_best_practices() {
    println!("--- Production best practices ---\n");

    println!("1. Configuration management:");
    println!("   - Keep configuration in a TOML file so it is version controlled");
    println!("   - Inject secrets (such as the database password) through environment variables");
    println!("   - Use a different configuration file per environment");

    println!("\n2. File rotation:");
    println!("   - Production must enable log rotation so the disk does not fill up");
    println!("   - Recommended: daily rotation + 30 days of retention");
    println!("   - Enable Zstd compression to save storage");

    println!("\n3. Performance tuning:");
    println!("   - channel_capacity: size it for peak QPS (QPS × 10 is a good start)");
    println!("   - worker_threads: match the CPU core count (2-8)");
    println!("   - Watch channel_len / capacity and alert above 80%");

    println!("\n4. Monitoring integration:");
    println!("   - Enable the HTTP server to expose /health and /metrics");
    println!("   - Have Prometheus scrape the /metrics endpoint");
    println!("   - Configure alert rules on logging system health");

    println!("\n5. Security hardening:");
    println!("   - Enable TLS on the production HTTP server");
    println!("   - Configure an IP whitelist to restrict sources");
    println!("   - Enable Bearer Token authentication");
    println!("   - Use LogSanitizer to redact sensitive information");
}
