// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Custom format example
//!
//! This example demonstrates how to create custom log formats
//! using the format configuration option.

use inklog::config::GlobalConfig;
use inklog::{InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom format string
    let format_string = "[{timestamp}] [{level:>5}] {target} - {message} | {file}:{line}";

    // Configure with custom format
    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".into(),
            format: format_string.to_string(),
            masking_enabled: true,
        },
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // Log with custom format
    log::info!("This message uses the custom format");
    log::debug!("Debug messages also use the format");
    log::warn!("Warning messages include file and line info");

    println!("\nCustom format example completed!");
    println!("Format: [timestamp] [LEVEL] target - message | file:line");

    Ok(())
}
