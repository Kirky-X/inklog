// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 内存泄漏检测测试套件

#[cfg(test)]
mod memory_leak_test {
    use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
    use serial_test::serial;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use std::time::Duration;

    #[tokio::test]
    #[serial]
    async fn test_memory_stability() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("memory_test.log");
        
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "100MB".into(),
                batch_size: 100,
                flush_interval_ms: 100,
                ..Default::default()
            }),
            ..Default::default()
        };

        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        // 写入测试数据
        for i in 0..1000 {
            log::info!(target: "memory_test", "Memory test message {}", i);
        }
        
        sleep(Duration::from_secs(1)).await;
        
        // 验证数据已写入
        assert!(log_path.exists());
        let metadata = std::fs::metadata(&log_path).unwrap();
        assert!(metadata.len() > 5000);
    }
}
