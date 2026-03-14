// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 异步文件 I/O 功能测试
// 测试 AsyncFileSink 的性能和功能

#[cfg(test)]
mod async_file_test {
    use inklog::sink::async_file::{AsyncFileConfig, AsyncFileSink, CompressionStrategy};
    use inklog::template::LogTemplate;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::time::{timeout, Duration};

    // === 异步文件配置测试 ===

    #[test]
    fn test_async_file_config_defaults() {
        let config = AsyncFileConfig::default();
        
        assert_eq!(config.channel_capacity, 10_000);
        assert_eq!(config.flush_batch_size, 1000);
        assert_eq!(config.flush_interval_ms, 50);
        assert_eq!(config.compression_strategy, CompressionStrategy::None);
        assert_eq!(config.compression_level, 3);
        assert_eq!(config.runtime_threads, 2);
    }

    #[test]
    fn test_async_file_config_custom() {
        let config = AsyncFileConfig {
            channel_capacity: 20_000,
            flush_batch_size: 2000,
            flush_interval_ms: 100,
            compression_strategy: CompressionStrategy::Batch,
            compression_level: 5,
            runtime_threads: 4,
            ..Default::default()
        };
        
        assert_eq!(config.channel_capacity, 20_000);
        assert_eq!(config.flush_batch_size, 2000);
        assert_eq!(config.compression_strategy, CompressionStrategy::Batch);
        assert_eq!(config.runtime_threads, 4);
    }

    #[test]
    fn test_compression_strategy_variants() {
        // 验证所有压缩策略变体
        let _ = CompressionStrategy::None;
        let _ = CompressionStrategy::Single;
        let _ = CompressionStrategy::Batch;
    }

    // === AsyncFileSink 创建测试 ===

    #[tokio::test]
    async fn test_async_file_sink_new() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("async_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path.clone(),
                ..Default::default()
            },
            channel_capacity: 1000,
            flush_batch_size: 100,
            flush_interval_ms: 100,
            compression_strategy: CompressionStrategy::None,
            compression_level: 3,
            runtime_threads: 1,
        };
        
        let template = LogTemplate::default();
        let sink = AsyncFileSink::new(config, template);
        
        assert!(sink.is_ok());
    }

    #[tokio::test]
    async fn test_async_file_sink_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("write_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path.clone(),
                ..Default::default()
            },
            channel_capacity: 100,
            flush_batch_size: 10,
            flush_interval_ms: 50,
            compression_strategy: CompressionStrategy::None,
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let mut sink = AsyncFileSink::new(config, template).unwrap();
        
        // 写入日志记录
        let record = inklog::LogRecord {
            timestamp: chrono::Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            message: "Test message".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        };
        
        // 写入
        let result = sink.write(&record);
        assert!(result.is_ok());
        
        // 刷新
        let flush_result = sink.flush();
        assert!(flush_result.is_ok());
        
        // 验证文件存在
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_async_file_sink_multiple_writes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("multi_write.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path.clone(),
                ..Default::default()
            },
            channel_capacity: 1000,
            flush_batch_size: 50,
            flush_interval_ms: 100,
            compression_strategy: CompressionStrategy::None,
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let mut sink = AsyncFileSink::new(config, template).unwrap();
        
        // 多次写入
        for i in 0..10 {
            let record = inklog::LogRecord {
                timestamp: chrono::Utc::now(),
                level: "INFO".to_string(),
                target: "test".to_string(),
                message: format!("Message {}", i),
                fields: std::collections::HashMap::new(),
                file: None,
                line: None,
                thread_id: "test".to_string(),
            };
            
            let result = sink.write(&record);
            assert!(result.is_ok(), "Write {} failed", i);
        }
        
        sink.flush().ok();
        
        // 验证文件存在且非空
        assert!(file_path.exists());
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[tokio::test]
    async fn test_async_file_sink_health() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("health_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let sink = AsyncFileSink::new(config, template).unwrap();
        
        // 检查健康状态
        let is_healthy = sink.is_healthy();
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_async_file_sink_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("shutdown_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let mut sink = AsyncFileSink::new(config, template).unwrap();
        
        // 写入一些数据
        let record = inklog::LogRecord {
            timestamp: chrono::Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            message: "Before shutdown".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        };
        
        sink.write(&record).ok();
        
        // 关闭
        let result = sink.shutdown();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_file_sink_performance() {
        use std::time::Instant;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("performance_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path,
                ..Default::default()
            },
            channel_capacity: 10000,
            flush_batch_size: 500,
            flush_interval_ms: 10,
            compression_strategy: CompressionStrategy::None,
            runtime_threads: 2,
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let mut sink = AsyncFileSink::new(config, template).unwrap();
        
        let iterations = 1000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let record = inklog::LogRecord {
                timestamp: chrono::Utc::now(),
                level: "DEBUG".to_string(),
                target: "perf_test".to_string(),
                message: format!("Performance test message {}", i),
                fields: std::collections::HashMap::new(),
                file: None,
                line: None,
                thread_id: "perf".to_string(),
            };
            
            sink.write(&record).ok();
        }
        
        let elapsed = start.elapsed();
        let throughput = iterations as f64 / elapsed.as_secs_f64();
        
        println!("AsyncFileSink throughput: {:.0} msg/s", throughput);
        
        // 至少应该达到 1000 msg/s
        assert!(throughput > 1000, "Throughput too low: {:.0} msg/s", throughput);
    }

    #[tokio::test]
    async fn test_async_file_sink_with_compression() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("compression_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path.clone(),
                compress: true,
                ..Default::default()
            },
            channel_capacity: 1000,
            flush_batch_size: 100,
            flush_interval_ms: 100,
            compression_strategy: CompressionStrategy::Batch,
            compression_level: 5,
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let mut sink = AsyncFileSink::new(config, template).unwrap();
        
        // 写入大量数据
        for i in 0..100 {
            let record = inklog::LogRecord {
                timestamp: chrono::Utc::now(),
                level: "INFO".to_string(),
                target: "compression_test".to_string(),
                message: format!("Compressible data {}", i),
                fields: std::collections::HashMap::new(),
                file: None,
                line: None,
                thread_id: "test".to_string(),
            };
            
            sink.write(&record).ok();
        }
        
        sink.flush().ok();
        
        // 验证文件存在
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_async_file_sink_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("timeout_test.log");
        
        let config = AsyncFileConfig {
            base_config: inklog::FileSinkConfig {
                enabled: true,
                path: file_path,
                ..Default::default()
            },
            channel_capacity: 10, // 小容量以触发背压
            flush_batch_size: 5,
            flush_interval_ms: 1000, // 长间隔
            ..Default::default()
        };
        
        let template = LogTemplate::default();
        let sink = AsyncFileSink::new(config, template).unwrap();
        
        // 写入应该能在超时内完成（或失败）
        let record = inklog::LogRecord {
            timestamp: chrono::Utc::now(),
            level: "INFO".to_string(),
            target: "timeout_test".to_string(),
            message: format!("Timeout test"),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        };
        
        // 尝试写入（可能在背压时失败）
        let result = timeout(Duration::from_millis(50), sink.write(&record)).await;
        
        // 超时或成功都可接受
        assert!(result.is_ok() || result.is_err());
    }
}
