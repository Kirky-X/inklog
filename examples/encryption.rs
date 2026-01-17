// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Encrypted file logging example
//!
//! This example demonstrates how to configure encrypted file logging
//! using AES-GCM encryption with a 256-bit key.

use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a proper 256-bit (32-byte) key and encode as base64
    // In production, use a secure key management system
    let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI="; // Base64 encoded 32 bytes

    // Set the encryption key in environment variable
    std::env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);

    let log_path: PathBuf = "logs/encrypted.log.enc".into();
    std::fs::create_dir_all("logs").ok();

    // Configure file sink with encryption
    let file_config = FileSinkConfig {
        enabled: true,
        path: log_path,
        max_size: "1MB".into(),
        rotation_time: "daily".into(),
        keep_files: 5,
        compress: false, // Encryption and compression don't work well together
        encrypt: true,
        encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // Log sensitive data
    log::info!("This message will be encrypted");
    log::warn!("Sensitive data is protected at rest");
    log::error!("Error details are also encrypted");

    println!("\nEncrypted logging example completed!");
    println!("Logs are encrypted with AES-256-GCM");
    println!("Output file: logs/encrypted.log.enc");

    // Clean up environment variable
    std::env::remove_var("INKLOG_ENCRYPTION_KEY");

    Ok(())
}
