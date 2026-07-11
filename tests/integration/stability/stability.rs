// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use inklog::LoggerManager;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info};

#[tokio::test]
#[ignore = "Requires Docker database environment"]
async fn test_long_running_stability() {
    // Set up database environment variables for Docker
    std::env::set_var("INKLOG_DATABASE_SINK_ENABLED", "false");

    let logger = LoggerManager::new()
        .await
        .expect("Failed to create LoggerManager");
    let duration = Duration::from_secs(5); // Default 5s, increase for real stability test
    let start = Instant::now();

    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let mut count = 0;
                while start.elapsed() < duration {
                    info!(target: "stability", "Thread {} log {}", i, count);
                    if count % 100 == 0 {
                        error!(target: "stability", "Thread {} error {}", i, count);
                    }
                    count += 1;
                    thread::sleep(Duration::from_millis(1));
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Thread join failed");
    }

    let status = logger.get_health_status();
    assert!(status.overall_status.is_operational());
    println!("Stability test passed. Metrics: {:?}", status.metrics);
}
