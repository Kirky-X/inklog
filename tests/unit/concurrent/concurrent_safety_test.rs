// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 并发安全测试套件
// 测试多线程同时访问共享资源的安全性

#[cfg(test)]
mod concurrent_safety_test {
    use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
    use serial_test::serial;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    #[serial]
    async fn test_concurrent_file_writes() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("concurrent_test.log");
        
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "100MB".into(),
                batch_size: 100,
                flush_interval_ms: 100,
                ..Default::default()
            }),
            performance: inklog::config::PerformanceConfig {
                worker_threads: 4,
                channel_capacity: 10000,
                ..Default::default()
            },
            ..Default::default()
        };

        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        let num_threads = 4;
        let messages_per_thread = 100;
        let barrier = Arc::new(Barrier::new(num_threads));
        
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    for i in 0..messages_per_thread {
                        log::info!(target: "concurrent_test", "Thread {} - Message {}", thread_id, i);
                    }
                })
            })
            .collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert!(log_path.exists());
        let metadata = std::fs::metadata(&log_path).unwrap();
        assert!(metadata.len() > 1000);
    }
}
