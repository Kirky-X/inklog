// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 多 Sink 降级示例
// 展示 File Sink 故障时自动降级到 Database，Database 故障时降级到 Console

use inklog::{DatabaseSinkConfig, FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Inklog 多 Sink 降级示例 ===\n");

    println!("降级策略：");
    println!("  File Sink 故障 → 降级到 Database");
    println!("  Database Sink 故障 → 降级到 Console");
    println!("  所有 Sink 故障 → 降级到 Console（最终保证）\n");

    // 场景1：File + Database + Console 组合（带自动降级）
    println!("场景1：多 Sink 组合（File → Database → Console）");
    println!("-------------------------------------------");

    let config = InklogConfig {
        global: inklog::config::GlobalConfig {
            level: "info".to_string(),
            auto_fallback: true, // 启用自动降级
            fallback_initial_delay_ms: 100,
            fallback_max_retries: 3,
            ..Default::default()
        },
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: "logs/primary.log".into(),
            max_size: "10MB".into(),
            ..Default::default()
        }),
        database_sink: Some(DatabaseSinkConfig {
            enabled: true,
            driver: inklog::config::DatabaseDriver::SQLite,
            url: "sqlite://logs/fallback_test.db".to_string(),
            pool_size: 3,
            batch_size: 50,
            flush_interval_ms: 500,
            table_name: "logs".to_string(),
            ..Default::default()
        }),
        console_sink: Some(inklog::config::ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    // 创建日志目录
    std::fs::create_dir_all("logs").ok();

    let manager = LoggerManager::with_config(config).await?;

    // 写入日志
    log::info!("Primary log message - should go to file");
    log::warn!("Warning message - will be logged via primary sink");
    log::error!("Error message - demonstrates multi-sink logging");

    println!("\n日志已写入，检查以下目标：");
    println!("  1. logs/primary.log - 主文件");
    println!("  2. logs/fallback_test.db - 数据库（降级目标）");
    println!("  3. 控制台输出\n");

    // 场景2：模拟故障转移
    println!("场景2：手动触发故障转移");
    println!("-------------------------------------------");

    // 获取健康状态
    let health = manager.get_health_status();
    println!("当前健康状态：");
    println!(
        "  File Sink: {:?}",
        health
            .sinks
            .get("file")
            .map(|s| &s.status)
            .unwrap_or(&inklog::SinkStatus::NotStarted)
    );
    println!(
        "  Database Sink: {:?}",
        health
            .sinks
            .get("database")
            .map(|s| &s.status)
            .unwrap_or(&inklog::SinkStatus::NotStarted)
    );
    println!(
        "  Console Sink: {:?}",
        health
            .sinks
            .get("console")
            .map(|s| &s.status)
            .unwrap_or(&inklog::SinkStatus::NotStarted)
    );

    // 手动触发恢复（如果需要）
    // manager.recover_sink("file").await?;

    println!("\n降级配置说明：");
    println!("  - auto_fallback: true - 启用自动降级");
    println!("  - fallback_initial_delay_ms: 100 - 降级初始延迟");
    println!("  - fallback_max_retries: 3 - 最大重试次数\n");

    println!("实际故障场景：");
    println!("  1. 当文件磁盘满时，File Sink 故障，自动降级到 Database");
    println!("  2. 当数据库连接断开时，Database Sink 故障，自动降级到 Console");
    println!("  3. 当所有 Sink 都不可用时，确保至少记录到 Console\n");

    manager.shutdown().ok();

    println!("=== 示例完成 ===");
    Ok(())
}
