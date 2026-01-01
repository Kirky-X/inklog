//! S3归档与数据库集成示例
//!
//! 这个示例展示了如何将S3归档服务与数据库日志集成，包括：
//! - 配置数据库存储日志
//! - 设置S3归档服务
//! - 演示归档流程
//! - 验证归档结果

use inklog::config::DatabaseDriver;
use inklog::{archive::ArchiveServiceBuilder, InklogConfig, LoggerManager};
use sea_orm::Database;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("=== S3 Archive with Database Integration Example ===\n");

    // 创建内存数据库用于演示（实际应用中应使用真实数据库）
    let database_url = "sqlite::memory:";
    let db_connection = Database::connect(database_url).await?;

    println!("✓ Database connection established");

    // 配置日志系统
    let config = InklogConfig {
        global: inklog::config::GlobalConfig {
            level: "info".to_string(),
            ..Default::default()
        },
        console_sink: Some(inklog::config::ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        }),
        database_sink: Some(inklog::config::DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            url: database_url.to_string(),
            flush_interval_ms: 5000,
            archive_to_s3: true,
            archive_after_days: 7,
            ..Default::default()
        }),
        s3_archive: Some(inklog::S3ArchiveConfig {
            enabled: true,
            bucket: "my-log-archive-bucket".to_string(),
            region: "us-west-2".to_string(),
            archive_interval_days: 1, // 每天归档一次（用于演示）
            local_retention_days: 7,  // 本地保留7天
            compression: inklog::archive::CompressionType::Zstd,
            storage_class: inklog::archive::StorageClass::StandardIa, // 低频访问存储
            prefix: "database_logs/".to_string(),
            max_file_size_mb: 50,
            ..Default::default()
        }),
        ..Default::default()
    };

    // 创建日志管理器
    let manager = LoggerManager::with_config(config).await?;
    println!("✓ LoggerManager created with database and S3 archive support");

    // 启动归档服务
    manager.start_archive_service().await?;
    println!("✓ S3 archive service started");

    // 记录一些示例日志到数据库
    println!("\n--- Recording Sample Logs ---");
    for i in 0..10 {
        let level = match i % 3 {
            0 => "INFO",
            1 => "WARN",
            _ => "ERROR",
        };

        log::log!(
            log::Level::Info,
            "Sample log message #{} with level {}",
            i,
            level
        );
    }

    println!("✓ Sample logs recorded to database");

    // 等待日志刷新到数据库
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    // 演示直接使用ArchiveServiceBuilder（带数据库连接）
    println!("\n--- Direct ArchiveService with Database ---");

    let archive_config = inklog::S3ArchiveConfig {
        enabled: true,
        bucket: "demo-archive-bucket".to_string(),
        region: "us-east-1".to_string(),
        archive_interval_days: 1,
        local_retention_days: 3,
        compression: inklog::archive::CompressionType::Gzip,
        storage_class: inklog::archive::StorageClass::Glacier,
        prefix: "db_archives/".to_string(),
        max_file_size_mb: 25,
        ..Default::default()
    };

    // 构建带数据库连接的归档服务
    let archive_service = ArchiveServiceBuilder::new()
        .config(archive_config)
        .database_connection(db_connection)
        .build()
        .await?;

    println!("✓ Archive service with database connection built");
    println!("  Configuration:");
    println!("    - Bucket: {}", archive_service.bucket());
    println!("    - Region: {}", archive_service.region());
    println!(
        "    - Archive Interval: {} days",
        archive_service.archive_interval_days()
    );
    println!(
        "    - Local Retention: {} days",
        archive_service.local_retention_days()
    );
    println!("    - Compression: {:?}", archive_service.compression());
    println!("    - Storage Class: {:?}", archive_service.storage_class());

    // 演示手动归档（注意：由于我们没有真实的日志数据，这会失败）
    println!("\n--- Manual Archive Demonstration ---");
    match manager.trigger_archive().await {
        Ok(archive_key) => {
            println!("✓ Manual archive completed successfully");
            println!("  Archive key: {}", archive_key);
        }
        Err(e) => {
            println!("✗ Manual archive failed (expected in demo): {}", e);
            println!("  This is expected since we don't have real log data to archive");
        }
    }

    // 演示归档服务的高级功能
    println!("\n--- Advanced Archive Features ---");

    // 列出归档文件（需要实际的S3连接）
    match archive_service.list_archives(None, None).await {
        Ok(archives) => {
            println!("✓ Found {} archived files", archives.len());
            for (i, archive) in archives.iter().take(5).enumerate() {
                println!("  {}. {} ({} bytes)", i + 1, archive.key, archive.size);
            }
        }
        Err(e) => {
            println!("✗ Could not list archives (expected without S3): {}", e);
        }
    }

    // 停止归档服务
    manager.stop_archive_service().await?;
    println!("✓ S3 archive service stopped");

    println!("\n=== Database Integration Example Completed ===");
    println!("\nKey takeaways:");
    println!("  - Database logs can be automatically archived to S3");
    println!("  - Archive service supports multiple storage classes");
    println!("  - Compression reduces storage costs");
    println!("  - Local retention policies manage disk space");
    println!("  - Manual and automatic archiving are supported");
    println!("\nProduction considerations:");
    println!("  - Use appropriate S3 storage classes for cost optimization");
    println!("  - Set up proper IAM roles and bucket policies");
    println!("  - Monitor archive operations and costs");
    println!("  - Test restore procedures regularly");
    println!("  - Implement proper error handling and alerting");

    Ok(())
}
