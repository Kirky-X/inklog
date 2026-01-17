// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Basic Inklog usage example
//!
//! This example demonstrates the simplest way to use Inklog for logging.

use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger with default configuration
    let _logger = LoggerManager::new().await?;

    // Log messages at different levels
    log::trace!("This is a trace message");
    log::debug!("This is a debug message");
    log::info!("This is an info message");
    log::warn!("This is a warning message");
    log::error!("This is an error message");

    println!("Basic logging example completed!");

    // Logger is automatically shut down when dropped
    Ok(())
}
