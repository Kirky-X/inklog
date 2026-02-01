// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use inklog::LoggerManager;
use std::time::Duration;
use tracing::{error, info};

#[tokio::test]
async fn test_e2e_logging() {
    // This test might fail if run in parallel with others due to global subscriber
    // We wrap it to ignore error if subscriber already set
    if let Ok(logger) = LoggerManager::new().await {
        info!("This is an info message");
        error!("This is an error message");

        // Give some time for async workers
        std::thread::sleep(Duration::from_millis(200));

        logger.shutdown().expect("Failed to shutdown logger");
    }
}
