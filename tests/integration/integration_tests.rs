// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use inklog::LoggerManager;
use std::time::Duration;
use inklog::tracing::{error, info};

#[tokio::test]
async fn test_e2e_logging() {
    let logger = LoggerManager::new()
        .await
        .expect("logger init should succeed");
    info!("This is an info message");
    error!("This is an error message");

    // Give some time for async workers
    std::thread::sleep(Duration::from_millis(200));

    logger.shutdown().expect("Failed to shutdown logger");
}
