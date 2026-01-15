//! S3 Archive Service Example
//!
//! Demonstrates S3 archive configuration, service lifecycle,
//! and manual archive triggering.

use inklog::{archive::ArchiveServiceBuilder, InklogConfig, LoggerManager};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
            archive_interval_days: 7,
            local_retention_days: 30,
            compression: inklog::archive::CompressionType::Zstd,
            storage_class: inklog::archive::StorageClass::Standard,
            prefix: "logs/".to_string(),
            max_file_size_mb: 100,
            ..Default::default()
        }),
        ..Default::default()
    };

    let manager = LoggerManager::with_config(config).await?;
    println!("S3 archive example started");

    // Start archive service
    manager.start_archive_service().await?;
    println!("  Archive service: started");

    // Log sample messages
    log::info!("Sample log message for archiving");
    log::warn!("Warning message");
    log::error!("Error message");

    // Simulate delay for log accumulation
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Manual archive trigger
    match manager.trigger_archive().await {
        Ok(archive_key) => println!("  Manual archive: {archive_key}"),
        Err(e) => println!("  Manual archive skipped: {e}"),
    }

    // Direct ArchiveServiceBuilder usage
    println!("\nDirect ArchiveService Usage:");
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
        force_path_style: true,
        skip_bucket_validation: true,
        ..Default::default()
    };

    let archive_service = ArchiveServiceBuilder::new()
        .config(archive_config)
        .build()
        .await?;

    println!("    Bucket: {}", archive_service.bucket());
    println!("    Region: {}", archive_service.region());
    println!(
        "    Interval: {} days",
        archive_service.archive_interval_days()
    );
    println!(
        "    Retention: {} days",
        archive_service.local_retention_days()
    );

    manager.stop_archive_service().await?;
    println!("\nS3 archive example completed");

    Ok(())
}
