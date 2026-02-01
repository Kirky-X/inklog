// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Database logging example
//!
//! This example demonstrates how to configure database logging
//! with SQLite and automatic batch writes.
//!
//! # Database Configuration
//!
//! Use DatabaseSinkConfig to configure the database sink.

use inklog::{DatabaseSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;
use std::time::Duration;

const DB_PATH: &str = "logs/example.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use SQLite for simplicity (can be PostgreSQL, MySQL)
    let db_path: PathBuf = DB_PATH.into();
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    println!("Using DatabaseSinkConfig with dbnexus integration");

    // Configure database sink using DatabaseSinkConfig
    let db_config = DatabaseSinkConfig {
        name: "default".to_string(),
        enabled: true,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: db_url.clone(),
        pool_size: 5,
        batch_size: 100,
        flush_interval_ms: 1000,
        partition: inklog::config::PartitionStrategy::default(),
        archive_to_s3: false,
        archive_after_days: 30,
        s3_bucket: None,
        s3_region: None,
        table_name: "logs".to_string(),
        archive_format: "json".to_string(),
        parquet_config: inklog::config::ParquetConfig::default(),
    };

    let config = InklogConfig {
        database_sink: Some(db_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // Log some messages
    for i in 0..50 {
        log::info!(target: "database_example", "Database log message #{}", i);
    }

    // Wait for batch to be written
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Verify logs were written
    let count = query_log_count(&db_url).await;
    println!("Database logging example completed!");
    println!("Total logs written to database: {count}");
    println!("Database location: {db_url}");

    Ok(())
}

/// Query the total number of logs stored in the database
#[cfg(not(feature = "dbnexus"))]
async fn query_log_count(url: &str) -> i64 {
    use inklog::sink::database::Entity;
    use sea_orm::{Database, EntityTrait};

    let db = match Database::connect(url).await {
        Ok(db) => db,
        Err(_) => return 0,
    };
    match Entity::find().all(&db).await {
        Ok(logs) => logs.len() as i64,
        Err(_) => 0,
    }
}

#[cfg(feature = "dbnexus")]
async fn query_log_count(_url: &str) -> i64 {
    // With dbnexus, query results require additional API usage
    // See dbnexus documentation for query patterns
    0
}
