// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! S3 Archive with Database Integration Example
//!
//! Demonstrates integrating S3 archive with database logging.
//! Note: dbnexus is built on top of Sea-ORM, so both APIs are available.
//! This example uses dbnexus API (recommended), but Sea-ORM can also be used
//! when dbnexus feature is disabled.

use std::error::Error;

use inklog::{archive::ArchiveServiceBuilder, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("S3 Archive with Database Integration Example");
    println!("(Using dbnexus - which wraps Sea-ORM internally)\n");

    // Create in-memory database for demo
    let database_url = "sqlite::memory:";

    // 使用 dbnexus 连接池（底层基于 Sea-ORM）
    // dbnexus is built on Sea-ORM, so sea-orm types are also available
    let db_pool = dbnexus::pool::DbPool::new(database_url).await?;
    let session = db_pool.get_session("admin").await?;
    println!("Database: connected via dbnexus (Sea-ORM powered)");

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
            name: "default".to_string(),
            enabled: true,
            driver: inklog::config::DatabaseDriver::SQLite,
            url: database_url.to_string(),
            pool_size: 5,
            batch_size: 100,
            flush_interval_ms: 5000,
            partition: inklog::config::PartitionStrategy::default(),
            archive_to_s3: false,
            archive_after_days: 30,
            s3_bucket: None,
            s3_region: None,
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            parquet_config: inklog::config::ParquetConfig::default(),
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
    println!("LoggerManager: initialized");

    // Start archive service
    manager.start_archive_service().await?;
    println!("Archive service: started\n");

    // Record sample logs to database
    println!("Recording sample logs:");
    for i in 0..10 {
        log::info!("Sample log message #{i}");
    }
    println!("Sample logs recorded\n");

    // Wait for logs to flush to database
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    // Direct ArchiveService with dbnexus Session
    println!("Direct ArchiveService with Database:");
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
        .database_session(session)
        .build()
        .await?;

    println!("  Bucket: {}", archive_service.bucket());
    println!("  Region: {}\n", archive_service.region());

    // Manual archive demonstration
    println!("Manual archive demonstration:");
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
    println!("\nS3 archive with database example completed!");
    println!("Note: S3 operations require valid AWS credentials and bucket configuration");

    Ok(())
}
