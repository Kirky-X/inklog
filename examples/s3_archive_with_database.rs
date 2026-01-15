//! S3 Archive with Database Integration Example
//!
//! Demonstrates integrating S3 archive with database logging.

use inklog::config::DatabaseDriver;
use inklog::{archive::ArchiveServiceBuilder, InklogConfig, LoggerManager};
use sea_orm::Database;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("S3 Archive with Database Integration Example");

    // Create in-memory database for demo
    let database_url = "sqlite::memory:";
    let db_connection = Database::connect(database_url).await?;
    println!("  Database: connected");

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
            archive_interval_days: 1,
            local_retention_days: 7,
            compression: inklog::archive::CompressionType::Zstd,
            storage_class: inklog::archive::StorageClass::StandardIa,
            prefix: "database_logs/".to_string(),
            max_file_size_mb: 50,
            ..Default::default()
        }),
        ..Default::default()
    };

    let manager = LoggerManager::with_config(config).await?;
    println!("  LoggerManager: initialized");

    // Start archive service
    manager.start_archive_service().await?;
    println!("  Archive service: started");

    // Record sample logs to database
    println!("\nRecording sample logs:");
    for i in 0..10 {
        log::info!("Sample log message #{i}");
    }
    println!("  Sample logs recorded");

    // Wait for logs to flush to database
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    // Direct ArchiveService with Database
    println!("\nDirect ArchiveService with Database:");
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

    let archive_service = ArchiveServiceBuilder::new()
        .config(archive_config)
        .database_connection(db_connection)
        .build()
        .await?;

    println!("    Bucket: {}", archive_service.bucket());
    println!("    Region: {}", archive_service.region());

    // Manual archive demonstration
    println!("\nManual archive demonstration:");
    match manager.trigger_archive().await {
        Ok(archive_key) => println!("  Archive: {archive_key}"),
        Err(e) => println!("  Archive skipped (expected): {e}"),
    }

    // List archives
    println!("\nListing archives:");
    match archive_service.list_archives(None, None).await {
        Ok(archives) => println!("  Found {} archived files", archives.len()),
        Err(e) => println!("  Could not list archives (expected without S3): {e}"),
    }

    // Stop archive service
    manager.stop_archive_service().await?;
    println!("\nS3 archive with database example completed");

    Ok(())
}
