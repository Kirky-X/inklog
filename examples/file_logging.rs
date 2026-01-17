// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! File logging with rotation example
//!
//! This example demonstrates how to configure file-based logging
//! with automatic rotation and compression.

use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_path: PathBuf = "logs/example.log".into();

    // Ensure the logs directory exists
    std::fs::create_dir_all("logs").ok();

    // Configure file sink with rotation
    let file_config = FileSinkConfig {
        enabled: true,
        path: log_path,
        max_size: "10MB".into(),       // Rotate when file reaches 10MB
        rotation_time: "daily".into(), // Also rotate daily
        keep_files: 7,                 // Keep 7 rotated files
        compress: true,                // Compress rotated files with ZSTD
        encrypt: false,                // Set to true and configure key for encryption
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // Log some messages
    for i in 0..100 {
        log::info!("Log message #{}", i);
        // In a real application, logs would accumulate and trigger rotation
    }

    println!("File logging example completed!");
    println!("Logs are being written to: logs/example.log");

    Ok(())
}
