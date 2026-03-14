// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 大数据量性能测试

#[cfg(test)]
mod large_volume_test {
    use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
    use serial_test::serial;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use std::time::Duration;

    #[tokio::test]
    #[serial]
    async fn test_large_volume_writing() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("large_volume_test.log");
        
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "1GB".into(),
                batch_size: 1000,
                flush_interval_ms: 500,
                compress: true,
                ..Default::default()
            }),
            performance: inklog::config::PerformanceConfig {
                worker_threads: 4,
                channel_capacity: 50000,
                ..Default::default()
            },
            ..Default::default()
        };

        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        // 写入大量数据
        for i in 0..5000 {
            log::info!(target: "large_volume", "Large volume test message {} - Data: {}", i, "x".repeat(100));
        }
        
        sleep(Duration::from_secs(2)).await;
        
        // 验证数据已写入
        assert!(log_path.exists());
        let metadata = std::fs::metadata(&log_path).unwrap();
        assert!(metadata.len() > 50000); // 至少50KB
    }
}
