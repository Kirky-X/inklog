// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Database logging example
//!
//! This example demonstrates how to configure database logging
//! with SQLite and automatic batch writes.
//!
//! # Two Configuration Options
//!
//! ## Option 1: DatabaseConfig (recommended)
//! This is the current recommended approach.

//! ## Option 2: Legacy DatabaseSinkConfig (removed)
//! ```rust,ignore
//! use inklog::config::DatabaseSinkConfig;  // Old API, now removed
//! let config = DatabaseSinkConfig::with_url("sqlite://logs.db")?;
//! ```

use inklog::{DatabaseConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;
use std::time::Duration;

const DB_PATH: &str = "logs/example.db";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use SQLite for simplicity (can be PostgreSQL, MySQL)
    let db_path: PathBuf = DB_PATH.into();
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    // Configure database sink using new DatabaseConfig (recommended)
    #[cfg(feature = "dbnexus")]
    {
        // Using new dbnexus-based configuration
        println!("Using new DatabaseConfig with dbnexus integration");
    }

    // Configure database sink using new DatabaseConfig (recommended)
    let db_config = DatabaseConfig {
        enabled: true,
        url: db_url.clone(),
        pool_size: 5,
        batch_size: 100,
        flush_interval_ms: 1000,
    };

    let config = InklogConfig {
        db_config: Some(db_config),
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
