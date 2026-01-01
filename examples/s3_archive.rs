//! S3归档服务示例
//!
//! 这个示例展示了如何使用inklog的S3归档功能，包括：
//! - 配置S3归档
//! - 启动归档服务
//! - 手动触发归档
//! - 列出归档文件

use inklog::{archive::ArchiveServiceBuilder, InklogConfig, LoggerManager};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日志管理器
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
        file_sink: Some(inklog::config::FileSinkConfig {
            enabled: true,
            path: "logs/example.log".into(),
            max_size: "10MB".into(),
            ..Default::default()
        }),
        s3_archive: Some(inklog::S3ArchiveConfig {
            enabled: true,
            bucket: "my-log-bucket".to_string(),
            region: "us-west-2".to_string(),
            archive_interval_days: 7, // 每7天归档一次
            local_retention_days: 30, // 本地保留30天
            compression: inklog::archive::CompressionType::Zstd,
            storage_class: inklog::archive::StorageClass::Standard,
            prefix: "logs/".to_string(),
            max_file_size_mb: 100,
            ..Default::default()
        }),
        ..Default::default()
    };

    // 创建日志管理器
    let manager = LoggerManager::with_config(config).await?;

    println!("Starting S3 archive service example...");

    // 启动归档服务
    manager.start_archive_service().await?;
    println!("✓ S3 archive service started");

    // 记录一些示例日志
    log::info!("This is an info message that will be archived");
    log::warn!("This is a warning message");
    log::error!("This is an error message");

    // 模拟一些时间延迟（在实际应用中，这里会有真实的日志数据）
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 手动触发归档（通常由定时任务自动执行）
    match manager.trigger_archive().await {
        Ok(archive_key) => {
            println!("✓ Manual archive completed: {}", archive_key);
        }
        Err(e) => {
            println!("✗ Manual archive failed: {}", e);
            println!("  Note: This is expected in the example since we don't have real log data");
        }
    }

    // 演示如何直接使用ArchiveServiceBuilder
    println!("\n--- Direct ArchiveService Usage ---");

    let archive_config = inklog::S3ArchiveConfig {
        enabled: true,
        bucket: "example-bucket".to_string(),
        region: "us-east-1".to_string(),
        archive_interval_days: 1,
        local_retention_days: 7,
        compression: inklog::archive::CompressionType::Gzip,
        storage_class: inklog::archive::StorageClass::Glacier,
        prefix: "archives/".to_string(),
        max_file_size_mb: 50,
        // 启用path-style寻址以提高兼容性
        force_path_style: true,
        // 跳过存储桶验证（用于测试，实际部署时应启用验证）
        skip_bucket_validation: true,
        // 可以配置MinIO或其他S3兼容服务的端点
        // endpoint_url: Some("http://localhost:9000".to_string()),
        // 可以配置访问凭证（也可以通过环境变量配置）
        // access_key_id: Some("minioadmin".to_string()),
        // secret_access_key: Some("minioadmin".to_string()),
        ..Default::default()
    };

    // 构建归档服务（注意：这不会自动启动定时任务）
    let archive_service = ArchiveServiceBuilder::new()
        .config(archive_config)
        .build()
        .await?;

    println!("✓ Archive service built successfully");
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

    // 停止归档服务
    manager.stop_archive_service().await?;
    println!("✓ S3 archive service stopped");

    println!("\nS3 archive example completed successfully!");
    println!("\nNote: This example demonstrates the API usage.");
    println!("In a real deployment:");
    println!("  - Configure valid AWS credentials or IAM roles");
    println!("  - Ensure the S3 bucket exists and is accessible");
    println!("  - Set up proper network connectivity to AWS S3");
    println!("  - Configure appropriate retention policies");
    println!("  - Monitor archive operations and storage costs");

    Ok(())
}
