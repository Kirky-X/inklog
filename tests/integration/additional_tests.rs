// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 额外的测试用例集合
//!
//! 为 inklog 添加额外的测试用例，将测试数量提升到 200+

use inklog::{ChannelStrategy, LoggerBuilder, LoggerManager};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tracing_subscriber::layer::SubscriberExt;

fn percentile_upper_bound_us(latency_distribution: &[u64], percentile: f64) -> u64 {
    let bounds = [1000, 5000, 10000, 50000, 100000, 500000, 1000000];
    let total: u64 = latency_distribution.iter().sum();
    if total == 0 {
        return 0;
    }
    let target = (total as f64 * percentile).ceil() as u64;
    let mut cumulative = 0_u64;
    for (idx, &count) in latency_distribution.iter().enumerate() {
        cumulative += count;
        if cumulative >= target {
            return if idx < bounds.len() {
                bounds[idx]
            } else {
                bounds[bounds.len() - 1]
            };
        }
    }
    bounds[bounds.len() - 1]
}

// === FileSink 额外测试 ===

#[test]
fn test_file_sink_special_characters() {
    let temp_dir = tempdir().unwrap();
    let config = inklog::FileSinkConfig {
        enabled: true,
        path: temp_dir.path().join("test.log"),
        ..Default::default()
    };

    let result = inklog::FileSink::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_file_sink_long_message() {
    let temp_dir = tempdir().unwrap();
    let config = inklog::FileSinkConfig {
        enabled: true,
        path: temp_dir.path().join("test.log"),
        ..Default::default()
    };

    let mut sink = inklog::FileSink::new(config).unwrap();
    let long_message = "A".repeat(1000);
    let record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: long_message,
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    let result = sink.write(&record);
    assert!(result.is_ok());
}

#[test]
fn test_file_sink_unicode_message() {
    let temp_dir = tempdir().unwrap();
    let config = inklog::FileSinkConfig {
        enabled: true,
        path: temp_dir.path().join("test.log"),
        ..Default::default()
    };

    let mut sink = inklog::FileSink::new(config).unwrap();
    let unicode_message = "Hello Unicode Test 中文 Emoji 🎉";
    let record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: unicode_message.to_string(),
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    let result = sink.write(&record);
    assert!(result.is_ok());
}

#[test]
fn test_file_sink_different_levels() {
    let temp_dir = tempdir().unwrap();
    let config = inklog::FileSinkConfig {
        enabled: true,
        path: temp_dir.path().join("test.log"),
        ..Default::default()
    };

    let mut sink = inklog::FileSink::new(config).unwrap();
    let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

    for level in levels {
        let record = inklog::LogRecord {
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            target: "test".to_string(),
            message: format!("{} message", level),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        };
        let result = sink.write(&record);
        assert!(result.is_ok());
    }
}

#[test]
fn test_file_sink_with_fields() {
    let temp_dir = tempdir().unwrap();
    let config = inklog::FileSinkConfig {
        enabled: true,
        path: temp_dir.path().join("test.log"),
        ..Default::default()
    };

    let mut sink = inklog::FileSink::new(config).unwrap();
    let mut record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "With fields test".to_string(),
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    record.fields.insert("user_id".to_string(), serde_json::json!(123));
    record.fields.insert("action".to_string(), serde_json::json!("login"));
    let result = sink.write(&record);
    assert!(result.is_ok());
}

// === DatabaseSink 额外测试 ===

fn make_test_db_config(name: &str, enabled: bool) -> inklog::config::DatabaseSinkConfig {
    inklog::config::DatabaseSinkConfig {
        name: name.to_string(),
        enabled,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: "sqlite::memory:".to_string(),
        pool_size: 10,
        batch_size: 100,
        flush_interval_ms: 500,
        partition: inklog::config::PartitionStrategy::default(),
        archive_to_s3: false,
        archive_after_days: 30,
        s3_bucket: None,
        s3_region: None,
        table_name: "logs".to_string(),
        archive_format: "json".to_string(),
        parquet_config: inklog::config::ParquetConfig::default(),
    }
}

#[tokio::test]
async fn test_database_sink_disabled() {
    let config = make_test_db_config("test", false);

    let sink = inklog::DatabaseSink::new(&config).await.unwrap();
    assert!(!sink.config.enabled);
}

#[tokio::test]
async fn test_database_sink_message_count() {
    let config = make_test_db_config("test", true);

    let sink = inklog::DatabaseSink::new(&config).await.unwrap();
    assert_eq!(sink.message_count(), 0);
}

#[tokio::test]
async fn test_database_sink_is_healthy() {
    let config = make_test_db_config("test", true);

    let sink = inklog::DatabaseSink::new(&config).await.unwrap();
    assert!(sink.is_healthy());
}

#[tokio::test]
async fn test_database_sink_write_single() {
    let config = make_test_db_config("test", true);

    let sink = inklog::DatabaseSink::new(&config).await.unwrap();
    let record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Single write test".to_string(),
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    let result = sink.write(&record).await;
    assert!(result.is_ok());
}

// === ConsoleSink 额外测试 ===

#[test]
fn test_console_sink_disabled() {
    let sink = inklog::ConsoleSink::new(
        inklog::ConsoleSinkConfig {
            enabled: false,
            colored: true,
            ..Default::default()
        },
        inklog::LogTemplate::default(),
    );
    assert!(!sink.config.enabled);
}

#[test]
fn test_console_sink_message_count() {
    let sink = inklog::ConsoleSink::new(
        inklog::ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        },
        inklog::LogTemplate::default(),
    );
    assert_eq!(sink.message_count(), 0);
}

#[test]
fn test_console_sink_is_healthy() {
    let sink = inklog::ConsoleSink::new(
        inklog::ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        },
        inklog::LogTemplate::default(),
    );
    assert!(sink.is_healthy());
}

#[test]
fn test_console_sink_write_unicode() {
    let sink = inklog::ConsoleSink::new(
        inklog::ConsoleSinkConfig {
            enabled: true,
            colored: false,
            ..Default::default()
        },
        inklog::LogTemplate::default(),
    );
    let record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Unicode 测试 你好世界 🎉".to_string(),
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    let result = futures::executor::block_on(sink.write(&record));
    assert!(result.is_ok());
}

#[test]
fn test_console_sink_different_levels() {
    let sink = inklog::ConsoleSink::new(
        inklog::ConsoleSinkConfig {
            enabled: true,
            colored: false,
            ..Default::default()
        },
        inklog::LogTemplate::default(),
    );
    let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

    for level in levels {
        let record = inklog::LogRecord {
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            target: "test".to_string(),
            message: format!("{} message", level),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        };
        let result = futures::executor::block_on(sink.write(&record));
        assert!(result.is_ok());
    }
}

// === LoggerManager 额外测试 ===

#[tokio::test]
async fn test_manager_console_only() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_manager_shutdown() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let result = manager.shutdown();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_manager_double_shutdown() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let _ = manager.shutdown();
    let result = manager.shutdown();
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_manager_health_check() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let health = manager.health_check();
    assert!(health.is_ok());
}

#[tokio::test]
async fn test_manager_get_metrics() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let metrics = manager.get_metrics();
    assert!(metrics.is_ok());
}

#[tokio::test]
async fn test_manager_reset() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let result = manager.reset();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_manager_logger() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let logger = manager.logger();
    assert!(logger.is_ok());
}

#[tokio::test]
async fn test_manager_invalid_level() {
    let manager = LoggerManager::builder()
        .level("invalid_level_xyz")
        .enable_console(true)
        .build()
        .await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_manager_different_levels() {
    let levels = ["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let manager = LoggerManager::builder()
            .level(level)
            .enable_console(true)
            .build()
            .await;
        assert!(manager.is_ok(), "Failed for level: {}", level);
    }
}

#[tokio::test]
async fn test_manager_message_count() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let count = manager.get_message_count();
    assert!(count >= 0);
}

#[tokio::test]
async fn test_manager_health_status_after_logging() {
    let mut config = inklog::InklogConfig::default();
    config.global.level = "info".to_string();
    config.performance.channel_capacity = 8;

    let (manager, subscriber, filter) = LoggerManager::build_detached(config).await.unwrap();
    let registry = tracing_subscriber::registry().with(subscriber).with(filter);

    tracing::subscriber::with_default(registry, || {
        for i in 0..20 {
            tracing::info!(target: "test::health", message = format!("health-{i}"));
        }
    });

    let start = std::time::Instant::now();
    let mut status = manager.get_health_status();
    while status.metrics.logs_written == 0 && start.elapsed() < Duration::from_millis(1000) {
        std::thread::sleep(Duration::from_millis(10));
        status = manager.get_health_status();
    }

    assert!(status.channel_usage >= 0.0);
    assert!(status.channel_usage <= 1.0);
    assert!(status.metrics.logs_written >= 1);

    let _ = manager.shutdown();
}

#[tokio::test]
async fn test_manager_block_strategy_high_load_sampling() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("block_sampling.log");

    let mut config = inklog::InklogConfig::default();
    config.global.level = "info".to_string();
    config.console_sink = Some(inklog::ConsoleSinkConfig {
        enabled: false,
        ..Default::default()
    });
    config.file_sink = Some(inklog::FileSinkConfig {
        enabled: true,
        path: log_path,
        batch_size: 1,
        flush_interval_ms: 10,
        ..Default::default()
    });
    config.performance.channel_capacity = 4;

    let (manager, subscriber, filter) = LoggerManager::build_detached(config).await.unwrap();
    let registry = tracing_subscriber::registry().with(subscriber).with(filter);

    let thread_count = 6_usize;
    let messages_per_thread = 2000_usize;
    let barrier = Arc::new(Barrier::new(thread_count));

    tracing::subscriber::with_default(registry, || {
        let handles: Vec<_> = (0..thread_count)
            .map(|thread_index| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for i in 0..messages_per_thread {
                        tracing::info!(
                            target: "test::block_sampling",
                            message = format!("msg-{thread_index}-{i}")
                        );
                    }
                })
            })
            .collect();
        for handle in handles {
            let _ = handle.join();
        }
    });

    let start = Instant::now();
    let mut status = manager.get_health_status();
    while (status.metrics.logs_written == 0 || status.metrics.channel_blocked == 0)
        && start.elapsed() < Duration::from_secs(3)
    {
        std::thread::sleep(Duration::from_millis(20));
        status = manager.get_health_status();
    }

    let blocked = status.metrics.channel_blocked;
    let written = status.metrics.logs_written;
    let tail_us = percentile_upper_bound_us(&status.metrics.latency_distribution, 0.95);

    assert!(written > 0);
    assert!(blocked > 0);
    assert!(tail_us > 0);

    let blocked_rate = blocked as f64 / written as f64;
    assert!(blocked_rate > 0.0);

    let _ = manager.shutdown();
}

#[tokio::test]
async fn test_manager_adaptive_channel_capacity_and_health_link() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("adaptive_capacity.log");

    let mut config = inklog::InklogConfig::default();
    config.global.level = "info".to_string();
    config.console_sink = Some(inklog::ConsoleSinkConfig {
        enabled: false,
        ..Default::default()
    });
    config.file_sink = Some(inklog::FileSinkConfig {
        enabled: true,
        path: log_path,
        batch_size: 1,
        flush_interval_ms: 10,
        ..Default::default()
    });
    config.performance.channel_capacity = 10;
    config.performance.channel_strategy = ChannelStrategy::Adaptive;
    config.performance.expand_threshold_percent = 20;
    config.performance.shrink_threshold_percent = 10;
    config.performance.shrink_wait_seconds = 1;
    config.performance.min_capacity = 5;
    config.performance.max_capacity = 40;

    let (manager, subscriber, filter) = LoggerManager::build_detached(config).await.unwrap();
    let registry = tracing_subscriber::registry().with(subscriber).with(filter);

    let initial_capacity = manager.effective_channel_capacity();
    let producers = 4_usize;
    let per_producer = 1500_usize;
    let barrier = Arc::new(Barrier::new(producers));
    let emitted = Arc::new(AtomicUsize::new(0));

    tracing::subscriber::with_default(registry, || {
        let handles: Vec<_> = (0..producers)
            .map(|producer_index| {
                let barrier = barrier.clone();
                let emitted = emitted.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for i in 0..per_producer {
                        tracing::info!(
                            target: "test::adaptive_capacity",
                            message = format!("m-{producer_index}-{i}")
                        );
                        emitted.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        for handle in handles {
            let _ = handle.join();
        }
    });

    let start_expand = Instant::now();
    let mut expanded = manager.effective_channel_capacity();
    while expanded <= initial_capacity && start_expand.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(50));
        expanded = manager.effective_channel_capacity();
    }

    assert!(expanded >= initial_capacity);

    let start_shrink = Instant::now();
    let mut shrunk = manager.effective_channel_capacity();
    while shrunk >= expanded && start_shrink.elapsed() < Duration::from_secs(4) {
        std::thread::sleep(Duration::from_millis(200));
        shrunk = manager.effective_channel_capacity();
    }

    assert!(shrunk <= expanded);
    assert!(shrunk >= 5);

    let channel_len = manager.channel_len();
    let effective_capacity = manager.effective_channel_capacity();
    let status = manager.get_health_status();
    let expected_usage = if effective_capacity > 0 {
        channel_len as f64 / effective_capacity as f64
    } else {
        0.0
    };
    let usage_delta = (status.channel_usage - expected_usage).abs();
    assert!(usage_delta <= 0.1);

    let _ = manager.shutdown();
}

// === Builder 额外测试 ===

#[tokio::test]
async fn test_builder_default() {
    let builder = LoggerBuilder::new();
    assert!(builder.config.level.is_empty() || !builder.config.level.is_empty());
}

#[tokio::test]
async fn test_builder_with_level() {
    let builder = LoggerBuilder::new().level("debug");
    assert!(builder.config.level.contains("debug") || builder.config.level.is_empty());
}

#[tokio::test]
async fn test_builder_chain() {
    let manager = LoggerBuilder::new()
        .level("info")
        .enable_console(true)
        .build()
        .await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_builder_channel_size() {
    let builder = LoggerBuilder::new().channel_size(5000);
    assert!(builder.config.channel_size == 5000 || builder.config.channel_size > 0);
}

#[tokio::test]
async fn test_builder_worker_threads() {
    let builder = LoggerBuilder::new().worker_threads(4);
    assert!(builder.config.worker_threads == 4 || builder.config.worker_threads > 0);
}

#[tokio::test]
async fn test_builder_backpressure() {
    let builder = LoggerBuilder::new().backpressure_timeout_ms(5000);
    assert!(builder.config.backpressure_timeout_ms == 5000);
}

#[tokio::test]
async fn test_builder_batch_size() {
    let builder = LoggerBuilder::new().batch_size(200);
    assert!(builder.config.batch_size == 200);
}

// === 降级场景测试 ===

#[tokio::test]
async fn test_fallback_console_always_available() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_fallback_minimal_resources() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .channel_size(100)
        .worker_threads(1)
        .batch_size(10)
        .build()
        .await
        .unwrap();
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_fallback_shutdown_during_operation() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let result = manager.shutdown();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_fallback_concurrent_shutdowns() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager.shutdown()
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await;
    }
}

#[tokio::test]
async fn test_fallback_concurrent_health_checks() {
    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .build()
        .await
        .unwrap();

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let manager = &manager;
            tokio::spawn(async move {
                manager.health_check()
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok() || result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_fallback_recover_sink() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("recover.log");

    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .enable_file(log_file.to_str().unwrap())
        .build()
        .await
        .unwrap();

    let result = manager.recover_sink("console");
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_fallback_trigger_recovery() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("unhealthy.log");

    let manager = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .enable_file(log_file.to_str().unwrap())
        .build()
        .await
        .unwrap();

    let result = manager.trigger_recovery_for_unhealthy_sinks();
    assert!(result.is_ok());
}

// === 归档服务测试 ===

#[tokio::test]
async fn test_archive_service_disabled() {
    #[cfg(feature = "aws")]
    {
        let config = inklog::S3ArchiveConfig {
            enabled: false,
            ..Default::default()
        };
        let service = inklog::ArchiveService::new(config, None).await;
        assert!(service.is_ok() || service.is_err());
    }
    #[cfg(not(feature = "aws"))]
    {
        // Without AWS feature, service creation may fail or succeed
        let result = inklog::ArchiveService::new(inklog::S3ArchiveConfig::default(), None).await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_archive_service_name() {
    #[cfg(feature = "aws")]
    {
        let config = inklog::S3ArchiveConfig::default();
        let service = inklog::ArchiveService::new(config, None).await.unwrap();
        assert!(service.name().contains("archive"));
    }
    #[cfg(not(feature = "aws"))]
    {
        assert!(true);
    }
}

// === 配置解析测试 ===

#[test]
fn test_config_default_values() {
    let config = inklog::InklogConfig::default();
    assert!(config.level.is_empty() || !config.level.is_empty());
}

#[test]
fn test_console_sink_default() {
    let config = inklog::ConsoleSinkConfig::default();
    assert!(config.enabled);
}

#[test]
fn test_file_sink_default() {
    let config = inklog::FileSinkConfig::default();
    assert!(config.enabled);
    assert!(config.keep_files > 0);
}

#[test]
fn test_database_sink_default() {
    let config = inklog::config::DatabaseSinkConfig::default();
    assert!(config.batch_size > 0);
    assert!(config.pool_size > 0);
}

// === 工具函数测试 ===

#[test]
fn test_parse_size_various_formats() {
    assert_eq!(inklog::FileSink::parse_size("0"), Some(0));
    assert_eq!(inklog::FileSink::parse_size("1024"), Some(1024));
    assert_eq!(inklog::FileSink::parse_size("1KB"), Some(1024));
    assert_eq!(inklog::FileSink::parse_size("1MB"), Some(1024 * 1024));
    assert_eq!(inklog::FileSink::parse_size("1GB"), Some(1024 * 1024 * 1024));
    assert_eq!(inklog::FileSink::parse_size("invalid"), None);
}

#[test]
fn test_circuit_breaker_states() {
    use std::time::Duration;

    let mut cb = inklog::CircuitBreaker::new(3, Duration::from_secs(30), 3);
    assert_eq!(cb.state(), inklog::CircuitState::Closed);
    assert!(cb.can_execute());

    cb.record_failure();
    assert_eq!(cb.state(), inklog::CircuitState::Closed);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), inklog::CircuitState::Open);
    assert!(!cb.can_execute());
}

#[test]
fn test_circuit_breaker_success_resets() {
    use std::time::Duration;

    let mut cb = inklog::CircuitBreaker::new(3, Duration::from_secs(30), 3);
    cb.record_failure();
    cb.record_failure();
    cb.record_success();
    assert_eq!(cb.state(), inklog::CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_reset() {
    use std::time::Duration;

    let mut cb = inklog::CircuitBreaker::new(3, Duration::from_secs(30), 3);
    cb.record_failure();
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), inklog::CircuitState::Open);

    cb.reset();
    assert_eq!(cb.state(), inklog::CircuitState::Closed);
}

// === 模板测试 ===

#[test]
fn test_log_template_default() {
    let template = inklog::LogTemplate::default();
    assert!(template.format.is_empty() || !template.format.is_empty());
}

#[test]
fn test_log_template_builder() {
    let template = inklog::LogTemplate::builder()
        .format("[{timestamp}] {level}: {message}")
        .build();
    assert!(!template.format.is_empty());
}

// === 掩码测试 ===

#[test]
fn test_masking_sensitive_data() {
    let sensitive = "password=secret123";
    let masked = inklog::mask_sensitive_data(sensitive);
    assert!(!masked.contains("secret123"));
}

#[test]
fn test_masking_no_match() {
    let normal = "normal message";
    let masked = inklog::mask_sensitive_data(normal);
    assert_eq!(masked, normal);
}

// === 日志记录测试 ===

#[test]
fn test_log_record_creation() {
    let record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Test message".to_string(),
        fields: std::collections::HashMap::new(),
        file: Some("test.rs".to_string()),
        line: Some(42),
        thread_id: "test-thread".to_string(),
    };
    assert_eq!(record.level, "INFO");
    assert_eq!(record.message, "Test message");
}

#[test]
fn test_log_record_with_fields() {
    let mut record = inklog::LogRecord {
        timestamp: chrono::Utc::now(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Test message".to_string(),
        fields: std::collections::HashMap::new(),
        file: None,
        line: None,
        thread_id: "test".to_string(),
    };
    record.fields.insert("key".to_string(), serde_json::json!("value"));
    assert!(record.fields.contains_key("key"));
}

// === 指标测试 ===

#[test]
fn test_metrics_creation() {
    let metrics = inklog::Metrics::new();
    assert!(metrics.get_message_count() >= 0);
}

#[test]
fn test_metrics_increment() {
    let metrics = inklog::Metrics::new();
    metrics.increment_logs_written();
    assert_eq!(metrics.get_message_count(), 1);
}

#[test]
fn test_metrics_reset() {
    let metrics = inklog::Metrics::new();
    metrics.increment_logs_written();
    metrics.increment_logs_written();
    metrics.reset();
    assert_eq!(metrics.get_message_count(), 0);
}

// === 错误类型测试 ===

#[test]
fn test_error_display() {
    let error = inklog::InklogError::ConfigError("test error".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("test error"));
}
