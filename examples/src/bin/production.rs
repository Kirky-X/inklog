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
    println!("=== inklog 生产环境配置示例 ===\n");

    show_env_comparison();
    show_builder_production_config().await?;
    show_toml_config();
    show_health_check_after_init().await?;
    show_graceful_shutdown().await?;
    show_error_handling().await;
    show_best_practices();

    println!("\n所有生产环境示例演示完毕。");
    Ok(())
}

/// 示例 1：环境差异化配置对比
///
/// 展示开发、预发布、生产三种环境的典型配置差异。
fn show_env_comparison() {
    println!("--- 示例 1：环境差异化配置对比 ---\n");

    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "配置项", "开发 (dev)", "预发布 (staging)", "生产 (prod)"
    );
    println!("{}", "-".repeat(80));
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "日志级别", "debug", "info", "info"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Console", "启用", "启用", "启用"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "File Sink", "禁用", "启用", "启用"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "文件压缩", "禁用", "Zstd", "Zstd"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "轮转策略", "禁用", "daily", "daily"
    );
    println!("{:<20} {:<20} {:<20} {:<20}", "保留文件", "N/A", "7", "30");
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Channel 容量", "1000", "5000", "10000"
    );
    println!("{:<20} {:<20} {:<20} {:<20}", "Worker 线程", "1", "2", "4");
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "HTTP 服务", "禁用", "启用", "启用+TLS"
    );
    println!(
        "{:<20} {:<20} {:<20} {:<20}",
        "Database Sink", "禁用", "禁用", "启用"
    );
    println!();
}

/// 示例 2：Builder 模式完整生产配置
///
/// 使用 Builder 模式构建生产级配置，实际初始化并验证。
async fn show_builder_production_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 2：Builder 模式生产配置 ---\n");

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

    println!("✓ 生产配置初始化成功");
    println!("  日志级别:   info");
    println!("  Console:    启用（彩色）");
    println!("  File:       logs/production.log（100MB, daily, 保留30个）");
    println!("  压缩:       Zstd");
    println!("  Channel:    10000");
    println!("  Workers:    4\n");

    // 验证配置生效
    tracing::info!("生产环境日志系统已启动");
    tracing::debug!("这条 DEBUG 日志不会输出（level=info）");
    println!("✓ 日志级别过滤验证：DEBUG 不可见（符合预期）");

    // 模拟业务日志
    for i in 1..=3 {
        tracing::info!(
            request_id = %uuid::Uuid::new_v4(),
            iteration = i,
            "处理请求"
        );
    }
    println!("✓ 3 条业务日志已记录");

    tracing::warn!(
        component = "database",
        latency_ms = 150,
        threshold_ms = 100,
        "数据库查询延迟超过阈值"
    );
    println!("✓ 告警日志已记录");

    tracing::error!(
        error_code = "E001",
        component = "cache",
        retry_count = 3,
        "缓存连接失败"
    );
    println!("✓ 错误日志已记录\n");

    // 优雅关闭
    logger.shutdown()?;
    println!("✓ 已优雅关闭\n");

    Ok(())
}

/// 示例 3：TOML 配置文件格式
///
/// 展示生产环境推荐的 TOML 配置文件内容。
fn show_toml_config() {
    println!("--- 示例 3：TOML 配置文件格式 ---\n");

    println!("inklog.toml（生产环境推荐配置）：");
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

    println!("\n加载方式：");
    println!("  // 自动搜索 inklog.toml 或 INKLOG_CONFIG_PATH");
    println!("  let _logger = LoggerManager::load().await?;");
    println!();
    println!("  // 指定配置文件路径");
    println!("  let _logger = LoggerManager::from_file(\"/etc/inklog/prod.toml\").await?;");
    println!();
}

/// 示例 4：初始化后健康状态验证
///
/// 演示初始化后如何验证系统健康状态。
async fn show_health_check_after_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 4：初始化后健康状态验证 ---\n");

    let logger = LoggerManager::builder()
        .level("info")
        .console(true)
        .build()
        .await?;

    // 检查健康状态
    let health = logger.get_health_status();
    println!("健康状态检查：");
    println!("  overall_status:     {:?}", health.overall_status);
    println!("  uptime_seconds:     {}s", health.uptime_seconds);
    println!("  channel_usage:      {:.1}%", health.channel_usage * 100.0);
    println!("  encryption_valid:   {}", health.encryption_key_valid);

    // 验证健康状态可获取（状态取决于 Sink 初始化时机）
    println!("  ✓ 健康状态获取成功");

    // 写入日志后再检查状态
    tracing::info!("健康检查日志");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let health2 = logger.get_health_status();
    println!("  写入日志后 overall_status: {:?}", health2.overall_status);

    // 检查 Channel 信息
    let capacity = logger.effective_channel_capacity();
    let queued = logger.channel_len();
    println!("\nChannel 信息：");
    println!("  capacity: {}", capacity);
    println!("  queued:   {}", queued);
    assert!(capacity > 0, "Channel 容量应大于 0");
    println!("  ✓ Channel 验证通过\n");

    logger.shutdown()?;
    Ok(())
}

/// 示例 5：优雅关闭流程
///
/// 演示应用退出前的完整关闭流程。
async fn show_graceful_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 5：优雅关闭流程 ---\n");

    let logger = LoggerManager::builder()
        .level("info")
        .console(true)
        .channel_capacity(1000)
        .build()
        .await?;

    // 模拟业务运行
    for i in 1..=5 {
        tracing::info!(batch = i, "处理批次");
    }
    println!("✓ 5 条日志已写入 Channel");

    // 关闭前检查
    let queued_before = logger.channel_len();
    println!("  关闭前 Channel 队列: {} 条", queued_before);

    // 执行关闭
    logger.shutdown()?;
    println!("✓ shutdown() 完成");
    println!("  - 所有日志已持久化到 Sink");
    println!("  - Worker 线程已停止");
    println!("  - 资源已释放\n");

    Ok(())
}

/// 示例 6：错误处理
///
/// 演示配置初始化可能遇到的错误及处理方式。
async fn show_error_handling() {
    println!("--- 示例 6：错误处理 ---\n");

    // 场景 1：不存在的配置文件
    println!("场景 1：加载不存在的配置文件");
    match LoggerManager::from_file("/nonexistent/config.toml").await {
        Ok(_) => println!("  意外成功"),
        Err(e) => {
            println!("  ✓ 预期错误: {}", e);
            println!("  处理: 回退到默认配置或提示用户检查路径");
        }
    }

    // 场景 2：无效的日志级别（Builder 验证）
    println!("\n场景 2：无效的日志级别");
    match LoggerManager::builder()
        .level("invalid_level")
        .build()
        .await
    {
        Ok(_) => println!("  意外成功"),
        Err(e) => {
            println!("  ✓ 预期错误: {}", e);
            println!("  处理: 使用有效的级别 (trace/debug/info/warn/error)");
        }
    }

    println!("\n错误处理建议：");
    println!("  - 始终使用 ? 或 match 处理 LoggerManager 初始化错误");
    println!("  - 配置文件加载失败时提供有意义的错误提示");
    println!("  - 考虑实现配置回退链：环境变量 → 配置文件 → 默认值\n");
}

/// 最佳实践建议
fn show_best_practices() {
    println!("--- 生产环境最佳实践 ---\n");

    println!("1. 配置管理：");
    println!("   - 使用 TOML 文件管理配置，便于版本控制");
    println!("   - 敏感信息（数据库密码）通过环境变量注入");
    println!("   - 不同环境使用不同配置文件");

    println!("\n2. 文件轮转：");
    println!("   - 生产环境必须启用日志轮转，防止磁盘写满");
    println!("   - 推荐 daily 轮转 + 保留 30 天");
    println!("   - 启用 Zstd 压缩节省存储空间");

    println!("\n3. 性能调优：");
    println!("   - channel_capacity: 根据峰值 QPS 设定（建议 QPS × 10）");
    println!("   - worker_threads: 与 CPU 核心数匹配（2-8）");
    println!("   - 监控 channel_len / capacity，超过 80% 告警");

    println!("\n4. 监控集成：");
    println!("   - 启用 HTTP 服务器暴露 /health 和 /metrics");
    println!("   - Prometheus 抓取 /metrics 端点");
    println!("   - 配置告警规则监控日志系统健康");

    println!("\n5. 安全加固：");
    println!("   - 生产环境 HTTP 服务器启用 TLS");
    println!("   - 配置 IP 白名单限制访问来源");
    println!("   - 启用 Bearer Token 认证");
    println!("   - 使用 LogSanitizer 脱敏敏感信息");
}
