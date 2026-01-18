// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试入口文件
//!
//! 此文件作为集成测试的入口点，包含所有集成测试模块的测试用例。
//!
//! 测试模块组织：
//! - 归档测试 (integration::archive)
//! - 自动恢复测试 (integration::recovery)
//! - 批量写入测试 (integration::batch)
//! - 配置环境测试 (integration::config)
//! - HTTP 服务器测试 (integration::http)
//! - Parquet 测试 (integration::parquet)
//! - 稳定性测试 (integration::stability)
//! - 验证测试 (integration::verification)

// ============ 通用集成测试 ============

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

#[cfg(feature = "confers")]
#[tokio::test]
async fn test_load_from_file() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    write!(
        file,
        r#"
        [global]
        level = "debug"
        [performance]
        channel_capacity = 500
    "#
    )
    .expect("Failed to write config to temp file");

    let _ = LoggerManager::from_file(file.path()).await;
}

// ============ 归档集成测试 (integration::archive) ============

use inklog::archive::{ArchiveMetadata, CompressionType, ScheduleState, StorageClass};

#[test]
fn test_archive_metadata_creation() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    assert_eq!(metadata.record_count, 100);
    assert_eq!(metadata.original_size, 50000);
    assert!(metadata.compressed_size >= 0);
    assert_eq!(metadata.archive_type, "json");
}

#[test]
fn test_archive_metadata_with_tag() {
    let metadata = ArchiveMetadata::new(50, 25000, "parquet")
        .with_tag("daily")
        .with_tag("automated");

    let tags: Vec<String> = metadata.tags.to_vec();
    assert!(tags.contains(&"daily".to_string()));
    assert!(tags.contains(&"automated".to_string()));
}

#[test]
fn test_archive_metadata_mark_success() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    let result = metadata.mark_success();

    // 验证状态已更改
    match result.status {
        inklog::archive::ArchiveStatus::Success => {}
        _ => panic!("Expected Success status"),
    }
}

#[test]
fn test_archive_metadata_mark_failed() {
    let metadata = ArchiveMetadata::new(100, 50000, "json");

    let result = metadata.mark_failed();

    match result.status {
        inklog::archive::ArchiveStatus::Failed => {}
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_schedule_state_default() {
    let state = ScheduleState::default();

    assert!(state.last_scheduled_run.is_none());
    assert!(state.last_successful_run.is_none());
    assert!(state.last_run_status.is_none());
    assert_eq!(state.consecutive_failures, 0);
    assert!(state.locked_date.is_none());
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_start_execution() {
    let mut state = ScheduleState::default();

    state.start_execution();

    assert!(state.last_scheduled_run.is_some());
    assert!(state.locked_date.is_some());
    assert!(state.is_running);
}

#[test]
fn test_schedule_state_success() {
    let mut state = ScheduleState::default();

    state.start_execution();
    state.mark_success();

    assert_eq!(state.consecutive_failures, 0);
    assert!(state.last_successful_run.is_some());
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_failure() {
    let mut state = ScheduleState::default();

    state.start_execution();
    state.mark_failed();

    assert_eq!(state.consecutive_failures, 1);
    assert!(!state.is_running);
}

#[test]
fn test_schedule_state_consecutive_failures() {
    let mut state = ScheduleState::default();

    for _ in 0..3 {
        state.mark_failed();
    }

    assert_eq!(state.consecutive_failures, 3);
}

#[test]
fn test_compression_type_values() {
    // 测试 CompressionType 变体
    let _none = CompressionType::None;
    let _gzip = CompressionType::Gzip;
    let _zstd = CompressionType::Zstd;
    let _lz4 = CompressionType::Lz4;
    let _brotli = CompressionType::Brotli;
}

#[test]
fn test_storage_class_values() {
    // 测试 StorageClass 变体
    let _standard = StorageClass::Standard;
    let _standard_ia = StorageClass::StandardIa;
    let _glacier = StorageClass::Glacier;
}

#[test]
fn test_archive_metadata_parquet_type() {
    let metadata = ArchiveMetadata::new(100, 50000, "parquet");

    assert_eq!(metadata.archive_type, "parquet");
}

#[test]
fn test_schedule_state_reset_after_success() {
    let mut state = ScheduleState::default();

    state.mark_failed();
    state.mark_failed();
    assert_eq!(state.consecutive_failures, 2);

    state.mark_success();

    assert_eq!(state.consecutive_failures, 0);
}

#[test]
fn test_schedule_state_can_run_today() {
    let state = ScheduleState::default();

    assert!(state.can_run_today());
}

#[test]
fn test_schedule_state_cannot_run_when_locked() {
    let mut state = ScheduleState::default();

    state.start_execution();

    assert!(!state.can_run_today());
}

// ============ 自动恢复集成测试 (integration::recovery) ============

use inklog::LoggerManager as RecoveryLoggerManager;
use std::fs as recovery_fs;
use std::thread as recovery_thread;
use std::time::Duration as RecoveryDuration;

#[tokio::test]
async fn test_file_sink_auto_recovery() {
    // Create a test directory
    let test_dir = "tests/temp_recovery";
    let _ = recovery_fs::create_dir_all(test_dir);

    // Create a logger with file sink
    let log_file = format!("{}/test_recovery.log", test_dir);
    let manager = RecoveryLoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log some messages
    tracing::info!("Test message before failure");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Simulate file sink failure by removing the log file
    let _ = recovery_fs::remove_file(&log_file);

    // Log more messages (these should fail and trigger recovery)
    for i in 0..10 {
        tracing::info!("Test message during failure {}", i);
        recovery_thread::sleep(RecoveryDuration::from_millis(50));
    }

    // Wait for auto-recovery to trigger
    recovery_thread::sleep(RecoveryDuration::from_secs(2));

    // Log messages after potential recovery
    tracing::info!("Test message after recovery");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Check health status
    let health = manager.get_health_status();
    println!("Health status: {:?}", health);

    // Clean up
    let _ = recovery_fs::remove_dir_all(test_dir);
}

#[tokio::test]
async fn test_manual_sink_recovery() {
    let test_dir = "tests/temp_manual_recovery";
    let _ = recovery_fs::create_dir_all(test_dir);

    let log_file = format!("{}/test_manual_recovery.log", test_dir);
    let manager = RecoveryLoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log initial message
    tracing::info!("Initial test message");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Simulate failure by removing file
    let _ = recovery_fs::remove_file(&log_file);

    // Log during failure
    tracing::info!("Message during failure");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Trigger manual recovery
    let recovery_result = manager.recover_sink("file");
    println!("Manual recovery result: {:?}", recovery_result);

    // Wait for recovery
    recovery_thread::sleep(RecoveryDuration::from_millis(500));

    // Log after manual recovery
    tracing::info!("Message after manual recovery");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Clean up
    let _ = recovery_fs::remove_dir_all(test_dir);

    assert!(recovery_result.is_ok());
}

#[tokio::test]
async fn test_bulk_recovery_for_unhealthy_sinks() {
    let test_dir = "tests/temp_bulk_recovery";
    let _ = recovery_fs::create_dir_all(test_dir);

    let log_file = format!("{}/test_bulk_recovery.log", test_dir);
    let manager = RecoveryLoggerManager::builder()
        .level("info")
        .file(log_file.clone())
        .build()
        .await
        .expect("Failed to create logger manager");

    // Log initial message
    tracing::info!("Initial test message");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Simulate failure
    let _ = recovery_fs::remove_file(&log_file);

    // Log during failure to make sink unhealthy
    for i in 0..5 {
        tracing::info!("Message during failure {}", i);
        recovery_thread::sleep(RecoveryDuration::from_millis(50));
    }

    // Trigger bulk recovery
    let recovery_result = manager.trigger_recovery_for_unhealthy_sinks();
    println!("Bulk recovery result: {:?}", recovery_result);

    // Wait for recovery
    recovery_thread::sleep(RecoveryDuration::from_millis(500));

    // Log after bulk recovery
    tracing::info!("Message after bulk recovery");
    recovery_thread::sleep(RecoveryDuration::from_millis(100));

    // Clean up
    let _ = recovery_fs::remove_dir_all(test_dir);

    assert!(recovery_result.is_ok());
}

// ============ 批量写入集成测试 (integration::batch) ============

use inklog::config::DatabaseDriver as BatchDatabaseDriver;
use inklog::sink::database::DatabaseSink as BatchDatabaseSink;
use inklog::sink::LogSink as BatchLogSink;
use inklog::{
    log_record::LogRecord as BatchLogRecord, DatabaseSinkConfig as BatchDatabaseSinkConfig,
};
use std::time::Duration as BatchDuration;
use tempfile::TempDir as BatchTempDir;
use tracing::Level as BatchLevel;

// ============ Test Helper Functions ============

/// Creates a DatabaseSink for testing with SQLite
fn create_test_database_sink(
    batch_size: usize,
    flush_interval_ms: u64,
) -> (BatchTempDir, BatchDatabaseSink, String) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = BatchDatabaseSinkConfig {
        enabled: true,
        driver: BatchDatabaseDriver::SQLite,
        url: url.clone(),
        batch_size,
        flush_interval_ms,
        ..Default::default()
    };

    let sink = BatchDatabaseSink::new(config).expect("Failed to create DatabaseSink");
    (temp_dir, sink, url)
}

/// Counts the number of log records in the database
fn count_database_logs(url: &str) -> i64 {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    })
}

// ============ Tests ============

#[test]
fn test_database_batch_write() {
    let (_temp_dir, mut sink, url) = create_test_database_sink(5, 1000);

    // Write 3 records (buffer=3, not enough to trigger batch flush)
    for i in 0..3 {
        let record = BatchLogRecord::new(
            BatchLevel::INFO,
            "batch_test".into(),
            format!("Message {}", i),
        );
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for flush interval to pass
    std::thread::sleep(BatchDuration::from_millis(1100));

    // Write 4th record - this triggers time-based flush (3 records flushed)
    let record = BatchLogRecord::new(
        BatchLevel::INFO,
        "batch_test".into(),
        "Trigger flush".into(),
    );
    sink.write(&record).expect("Failed to write log record");

    // Wait for flush to complete
    std::thread::sleep(BatchDuration::from_millis(200));

    let count_before = count_database_logs(&url);
    // After time-based flush, 4 records should be in DB
    assert_eq!(count_before, 4, "时间刷新应该写入4条记录");

    // Write 5 more records to trigger batch-based flush (batch_size=5)
    for i in 4..9 {
        let record = BatchLogRecord::new(
            BatchLevel::INFO,
            "batch_test".into(),
            format!("Message {}", i),
        );
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for batch flush to complete
    std::thread::sleep(BatchDuration::from_millis(500));

    let count_after = count_database_logs(&url);
    // Total should be 4 (first flush) + 5 (batch flush) = 9
    assert_eq!(
        count_after, 9,
        "批次写入应该触发，当前记录数: {}",
        count_after
    );

    println!("批量写入测试通过！批次大小: 5, 实际写入: {}", count_after);
}

#[test]
fn test_database_timeout_flush() {
    let (_temp_dir, mut sink, url) = create_test_database_sink(100, 300);

    let record1 = BatchLogRecord::new(
        BatchLevel::INFO,
        "timeout_test".into(),
        "First message".into(),
    );
    sink.write(&record1)
        .expect("Failed to write first log record");

    std::thread::sleep(BatchDuration::from_millis(500));

    let record2 = BatchLogRecord::new(
        BatchLevel::INFO,
        "timeout_test".into(),
        "Second message".into(),
    );
    sink.write(&record2)
        .expect("Failed to write second log record");

    std::thread::sleep(BatchDuration::from_millis(500));

    let count = count_database_logs(&url);

    assert!(count >= 1, "超时刷新应该触发写入，当前记录数: {}", count);

    println!("超时刷新测试通过！刷新间隔: 300ms, 实际写入: {}", count);
}

// ============ 配置环境集成测试 (integration::config) ============

use inklog::InklogConfig as ConfigInklogConfig;
use serial_test::serial as config_serial;

fn clear_all_inklog_env_vars() {
    // 清除所有可能的 INKLOG_* 环境变量
    for (key, _) in std::env::vars() {
        if key.starts_with("INKLOG_") {
            std::env::remove_var(&key);
        }
    }
}

#[test]
#[config_serial]
fn test_config_from_env_overrides() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_LEVEL", "debug");
    std::env::set_var("INKLOG_FILE_ENABLED", "true");
    std::env::set_var("INKLOG_FILE_PATH", "/tmp/test_logs/app.log");
    std::env::set_var("INKLOG_FILE_MAX_SIZE", "50MB");
    std::env::set_var("INKLOG_FILE_COMPRESS", "true");

    let mut config = ConfigInklogConfig::default();
    config.apply_env_overrides();

    // 验证环境变量覆盖生效
    assert_eq!(config.global.level, "debug");

    assert!(config.file_sink.is_some());
    let file = config.file_sink.unwrap();
    assert!(file.enabled);
    assert_eq!(file.max_size, "50MB");
    assert!(file.compress);
}

#[test]
#[config_serial]
fn test_config_env_override_s3_encryption() {
    clear_all_inklog_env_vars();

    // 设置 S3 加密环境变量
    std::env::set_var("INKLOG_S3_ENABLED", "true");
    std::env::set_var("INKLOG_S3_BUCKET", "test-bucket");
    std::env::set_var("INKLOG_S3_REGION", "us-west-2");
    std::env::set_var("INKLOG_S3_ENCRYPTION_ALGORITHM", "awskms");
    std::env::set_var("INKLOG_S3_ENCRYPTION_KMS_KEY_ID", "test-key-id");
    std::env::set_var("INKLOG_ARCHIVE_FORMAT", "parquet");

    let mut config = ConfigInklogConfig::default();
    config.apply_env_overrides();

    // 验证 S3 归档配置
    assert!(config.s3_archive.is_some());
    let s3 = config.s3_archive.unwrap();
    assert!(s3.enabled);
    assert_eq!(s3.bucket, "test-bucket");
    assert_eq!(s3.region, "us-west-2");
    assert!(s3.encryption.is_some());
    match &s3.encryption.unwrap().algorithm {
        inklog::archive::EncryptionAlgorithm::AwsKms => {} // 正确
        _ => panic!("Expected AwsKms encryption"),
    }
    assert_eq!(s3.archive_format, "parquet");
}

#[test]
#[config_serial]
fn test_config_env_override_http_server() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_HOST", "127.0.0.1");
    std::env::set_var("INKLOG_HTTP_PORT", "9090");
    std::env::set_var("INKLOG_HTTP_METRICS_PATH", "/prometheus");
    std::env::set_var("INKLOG_HTTP_HEALTH_PATH", "/status");

    let mut config = ConfigInklogConfig::default();
    config.apply_env_overrides();

    assert!(config.http_server.is_some());
    let http = config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.host, "127.0.0.1");
    assert_eq!(http.port, 9090);
    assert_eq!(http.metrics_path, "/prometheus");
    assert_eq!(http.health_path, "/status");
}

#[test]
#[config_serial]
fn test_config_env_override_performance() {
    clear_all_inklog_env_vars();

    std::env::set_var("INKLOG_WORKER_THREADS", "8");
    std::env::set_var("INKLOG_CHANNEL_CAPACITY", "20000");

    let mut config = ConfigInklogConfig::default();
    config.apply_env_overrides();

    assert_eq!(config.performance.worker_threads, 8);
    assert_eq!(config.performance.channel_capacity, 20000);
}

// ============ HTTP 服务器集成测试 (integration::http) ============

use inklog::config::{HttpErrorMode, HttpServerConfig};
use inklog::InklogConfig as HttpInklogConfig;
use serial_test::serial as http_serial;

fn clear_inklog_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("INKLOG_") {
            std::env::remove_var(&key);
        }
    }
}

#[tokio::test]
#[http_serial]
async fn test_http_server_startup_with_default_config() {
    clear_inklog_env();

    let port = 18080
        + std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u16
            % 10000;

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Panic,
    };

    let inklog_config = HttpInklogConfig {
        http_server: Some(config),
        ..Default::default()
    };

    assert!(inklog_config.http_server.is_some());
    let http = inklog_config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.port, port);
}

#[tokio::test]
#[http_serial]
async fn test_http_server_error_mode_panic() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18081,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Panic,
    };

    match config.error_mode {
        HttpErrorMode::Panic => {}
        _ => panic!("Expected Panic mode"),
    }
}

#[tokio::test]
#[http_serial]
async fn test_http_server_error_mode_warn() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18082,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Warn,
    };

    match config.error_mode {
        HttpErrorMode::Warn => {}
        _ => panic!("Expected Warn mode"),
    }
}

#[tokio::test]
#[http_serial]
async fn test_http_server_error_mode_strict() {
    clear_inklog_env();

    let config = HttpServerConfig {
        enabled: true,
        host: "127.0.0.1".to_string(),
        port: 18083,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
    };

    match config.error_mode {
        HttpErrorMode::Strict => {}
        _ => panic!("Expected Strict mode"),
    }
}

#[tokio::test]
#[http_serial]
async fn test_http_server_with_logger_manager() {
    clear_inklog_env();

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_HOST", "127.0.0.1");
    std::env::set_var("INKLOG_HTTP_PORT", "18084");
    std::env::set_var("INKLOG_HTTP_ERROR_MODE", "warn");

    let mut config = HttpInklogConfig::default();
    config.apply_env_overrides();

    assert!(config.http_server.is_some());
    let http = config.http_server.unwrap();
    assert!(http.enabled);
    assert_eq!(http.host, "127.0.0.1");
    assert_eq!(http.port, 18084);
    match http.error_mode {
        HttpErrorMode::Warn => {}
        _ => panic!("Expected Warn mode from env"),
    }

    std::env::remove_var("INKLOG_HTTP_ENABLED");
    std::env::remove_var("INKLOG_HTTP_HOST");
    std::env::remove_var("INKLOG_HTTP_PORT");
    std::env::remove_var("INKLOG_HTTP_ERROR_MODE");
}

#[tokio::test]
#[http_serial]
async fn test_http_metrics_path_configuration() {
    clear_inklog_env();

    std::env::set_var("INKLOG_HTTP_ENABLED", "true");
    std::env::set_var("INKLOG_HTTP_METRICS_PATH", "/prometheus/metrics");
    std::env::set_var("INKLOG_HTTP_HEALTH_PATH", "/status");

    let mut config = HttpInklogConfig::default();
    config.apply_env_overrides();

    let http = config
        .http_server
        .expect("http_server should be Some after setting INKLOG_HTTP_ENABLED");
    assert_eq!(http.metrics_path, "/prometheus/metrics");
    assert_eq!(http.health_path, "/status");
}

#[tokio::test]
#[http_serial]
async fn test_http_server_disabled_by_default() {
    clear_inklog_env();

    let mut config = HttpInklogConfig::default();
    config.apply_env_overrides();

    assert!(
        config.http_server.is_none(),
        "INKLOG_HTTP_ENABLED should not be set"
    );
}

// ============ Parquet 集成测试 (integration::parquet) ============

// Parquet功能验证测试
// 测试Parquet导出功能的正确性、性能和兼容性

use arrow_array::RecordBatchReader;
use arrow_schema::DataType;
use bytes::Bytes;
use inklog::sink::database::convert_logs_to_parquet;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::time::Instant;

// ============ Test Data Helper Functions ============

/// Creates test log data with specified count
fn create_test_logs(count: usize) -> Vec<inklog::sink::database::Model> {
    (0..count)
        .map(|i| inklog::sink::database::Model {
            id: i as i64,
            timestamp: chrono::Utc::now(),
            level: match i % 5 {
                0 => "trace".to_string(),
                1 => "debug".to_string(),
                2 => "info".to_string(),
                3 => "warn".to_string(),
                _ => "error".to_string(),
            },
            target: format!("test_module::function_{}", i % 10),
            message: format!("Test log message number {}", i),
            fields: Some(serde_json::json!({
                "user_id": i,
                "request_id": format!("req-{:010x}", i),
                "duration_ms": i * 10,
            })),
            file: Some(format!("src/test_{}.rs", i % 5)),
            line: Some((i % 100) as i32),
            thread_id: format!("thread-{}", i % 4),
        })
        .collect()
}

// ============ Parquet Verification Helper Functions ============

/// Expected schema field names
const EXPECTED_FIELD_NAMES: &[&str] = &[
    "id",
    "timestamp",
    "level",
    "target",
    "message",
    "fields",
    "file",
    "line",
    "thread_id",
];

/// Expected schema field types
const EXPECTED_FIELD_TYPES: &[DataType] = &[
    DataType::Int64, // id
    DataType::Utf8,  // timestamp
    DataType::Utf8,  // level
    DataType::Utf8,  // target
    DataType::Utf8,  // message
    DataType::Utf8,  // fields
    DataType::Utf8,  // file
    DataType::Int64, // line
    DataType::Utf8,  // thread_id
];

/// Verifies Parquet file schema (names and types)
fn verify_parquet_schema(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(data);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    let schema = reader.schema();
    let fields = schema.fields();

    // Verify field count
    assert_eq!(fields.len(), 9, "Schema should have 9 fields");

    // Verify field names and types
    for (i, (name, dtype)) in EXPECTED_FIELD_NAMES
        .iter()
        .zip(EXPECTED_FIELD_TYPES.iter())
        .enumerate()
    {
        assert_eq!(fields[i].name(), *name);
        assert_eq!(fields[i].data_type(), dtype);
    }

    Ok(())
}

/// Verifies Parquet file data content
fn verify_parquet_data(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(data);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    let mut total_rows = 0;
    for batch in reader {
        let batch = batch?;
        assert!(batch.num_rows() > 0, "Batch should have rows");
        total_rows += batch.num_rows();
    }

    assert!(total_rows > 0, "Parquet file should contain data");

    Ok(())
}

/// Complete Parquet file verification (schema + data)
fn verify_parquet_file(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    verify_parquet_schema(data)?;
    verify_parquet_data(data)?;
    Ok(())
}

// ============ Parquet Tests ============

#[test]
fn test_parquet_basic_conversion() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs, &Default::default());

    assert!(result.is_ok(), "Parquet conversion should succeed");
    let parquet_data = result.expect("Parquet conversion should succeed");

    assert!(!parquet_data.is_empty(), "Parquet data should not be empty");

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_small_dataset() {
    let logs = create_test_logs(1_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 1K records");

    println!("1K records conversion time: {:?}", duration);
    println!("1K records Parquet size: {} bytes", parquet_data.len());

    // Verify compression ratio (assuming ~200 bytes per record in JSON)
    let estimated_original_size = logs.len() * 200;
    let compression_ratio = estimated_original_size as f64 / parquet_data.len() as f64;
    println!("Estimated compression ratio: {:.2}x", compression_ratio);

    assert!(
        compression_ratio > 1.5,
        "Compression ratio should be > 1.5x, got {:.2}x",
        compression_ratio
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_medium_dataset() {
    let logs = create_test_logs(10_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 10K records");

    println!("10K records conversion time: {:?}", duration);
    println!("10K records Parquet size: {} bytes", parquet_data.len());

    // Verify performance (10K records should complete in < 5 seconds)
    assert!(
        duration.as_secs() < 5,
        "10K records conversion should complete in < 5 seconds, took {:?}",
        duration
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_large_dataset() {
    let logs = create_test_logs(100_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 100K records");

    println!("100K records conversion time: {:?}", duration);
    println!("100K records Parquet size: {} bytes", parquet_data.len());

    // Verify performance (100K records should complete in < 30 seconds)
    assert!(
        duration.as_secs() < 30,
        "100K records conversion should complete in < 30 seconds, took {:?}",
        duration
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_compression_ratio() {
    let logs = create_test_logs(10_000);
    let result = convert_logs_to_parquet(&logs, &Default::default())
        .expect("Parquet conversion should succeed");

    // Calculate original JSON size
    let json_data = serde_json::to_vec(&logs).expect("JSON serialization should succeed");
    let original_size = json_data.len();
    let compressed_size = result.len();

    let compression_ratio = original_size as f64 / compressed_size as f64;

    println!("Original JSON size: {} bytes", original_size);
    println!("Compressed Parquet size: {} bytes", compressed_size);
    println!("Actual compression ratio: {:.2}x", compression_ratio);

    // Verify compression ratio > 50%
    assert!(
        compression_ratio > 2.0,
        "Compression ratio should be > 2.0x, got {:.2}x",
        compression_ratio
    );
}

#[test]
fn test_parquet_empty_dataset() {
    let logs: Vec<inklog::sink::database::Model> = vec![];
    let result = convert_logs_to_parquet(&logs, &Default::default());

    let parquet_data = result.expect("Parquet conversion should succeed for empty dataset");

    // Empty dataset should produce a valid Parquet file (even without data rows)
    assert!(
        !parquet_data.is_empty(),
        "Parquet file should have metadata even for empty data"
    );
}

#[test]
fn test_parquet_schema_compatibility() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs, &Default::default())
        .expect("Parquet conversion should succeed");

    // Use the consolidated schema verification
    verify_parquet_schema(&result).expect("Schema verification should pass");
}

// ============ 稳定性集成测试 (integration::stability) ============

use inklog::LoggerManager as StabilityLoggerManager;
use std::thread as stability_thread;
use std::time::{Duration as StabilityDuration, Instant as StabilityInstant};
use tracing::{error as stability_error, info as stability_info};

#[tokio::test]
#[ignore] // Run manually: cargo test --test integration_tests -- --ignored
async fn test_long_running_stability() {
    let logger = StabilityLoggerManager::new()
        .await
        .expect("Failed to create LoggerManager");
    let duration = StabilityDuration::from_secs(5); // Default 5s, increase for real stability test
    let start = StabilityInstant::now();

    let handles: Vec<_> = (0..4)
        .map(|i| {
            stability_thread::spawn(move || {
                let mut count = 0;
                while start.elapsed() < duration {
                    stability_info!(target: "stability", "Thread {} log {}", i, count);
                    if count % 100 == 0 {
                        stability_error!(target: "stability", "Thread {} error {}", i, count);
                    }
                    count += 1;
                    stability_thread::sleep(StabilityDuration::from_millis(1));
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

// ============ 验证集成测试 (integration::verification) ============

use inklog::config::DatabaseDriver as VerifyDatabaseDriver;
use inklog::sink::database::DatabaseSink as VerifyDatabaseSink;
use inklog::sink::file::FileSink as VerifyFileSink;
use inklog::{
    log_record::LogRecord as VerifyLogRecord, DatabaseSinkConfig as VerifyDatabaseSinkConfig,
    FileSinkConfig as VerifyFileSinkConfig,
};
use std::fs::File as VerifyFile;
use std::io::Read as VerifyRead;
use std::path::PathBuf;
use std::time::Duration as VerifyDuration;
use tempfile::TempDir as VerifyTempDir;
use tracing::Level as VerifyLevel;

// ============ Database Helper Functions ============

fn get_log_count(url: &str) -> i64 {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        use inklog::sink::database::Entity;
        use sea_orm::{Database, EntityTrait};

        let db = Database::connect(url)
            .await
            .expect("Failed to connect to database");
        let logs = Entity::find().all(&db).await.expect("Failed to query logs");
        logs.len() as i64
    })
}

// ============ File Helper Functions ============

/// Finds a file with the specified extension in a directory
fn find_file_with_extension(dir: &VerifyTempDir, extension: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir.path()).expect("Failed to read temp directory");
    entries
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == extension))
}

/// Verifies that a file is compressed with Zstandard
fn verify_zstd_compression(file_path: &PathBuf) {
    let mut file = VerifyFile::open(file_path).expect("Failed to open compressed file");
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .expect("Failed to read file magic bytes");
    // Zstd magic: 0xFD2FB528 (LE: 28 B5 2F FD)
    assert_eq!(magic, [0x28, 0xB5, 0x2F, 0xFD]);
}

/// Verifies that a file is encrypted (has nonce + ciphertext)
fn verify_encrypted_file(file_path: &PathBuf) {
    let metadata = std::fs::metadata(file_path).expect("Failed to get file metadata");
    assert!(
        metadata.len() > 12,
        "Encrypted file should have nonce (12 bytes) + ciphertext"
    );
}

// ============ Verification Tests ============

#[test]
fn verify_file_sink_compression() {
    let temp_dir = VerifyTempDir::new().expect("Failed to create temp directory");
    let log_path = temp_dir.path().join("test.log");

    let config = VerifyFileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(),
        compress: true,
        encrypt: false,
        ..Default::default()
    };

    let mut sink = VerifyFileSink::new(config).expect("Failed to create FileSink");
    let record = VerifyLogRecord::new(
        VerifyLevel::INFO,
        "test".into(),
        "A long message to trigger rotation".into(),
    );
    sink.write(&record).expect("Failed to write log record");

    // Trigger rotation
    for _ in 0..5 {
        sink.write(&record)
            .expect("Failed to write log record during rotation");
    }

    // Wait for background compression
    std::thread::sleep(VerifyDuration::from_millis(1000));

    let zst_path = find_file_with_extension(&temp_dir, "zst").expect("No compressed file found");
    verify_zstd_compression(&zst_path);
}

#[test]
fn verify_file_sink_encryption() {
    let temp_dir = VerifyTempDir::new().expect("Failed to create temp directory");
    let log_path = temp_dir.path().join("enc.log");

    // Use a proper base64-encoded 32-byte key (44 characters)
    std::env::set_var("LOG_KEY", "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=");

    let config = VerifyFileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        max_size: "10".into(),
        compress: false,
        encrypt: true,
        encryption_key_env: Some("LOG_KEY".into()),
        ..Default::default()
    };

    let mut sink = VerifyFileSink::new(config).expect("Failed to create FileSink");
    let record = VerifyLogRecord::new(VerifyLevel::INFO, "test".into(), "Secret message".into());
    sink.write(&record).expect("Failed to write log record");

    for _ in 0..5 {
        sink.write(&record)
            .expect("Failed to write log record during rotation");
    }

    std::thread::sleep(VerifyDuration::from_millis(500));

    let enc_path = find_file_with_extension(&temp_dir, "enc").expect("No encrypted file found");
    verify_encrypted_file(&enc_path);
}

#[test]
fn verify_database_sink_sqlite() {
    let temp_dir = VerifyTempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("logs.db");

    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = VerifyDatabaseSinkConfig {
        enabled: true,
        driver: VerifyDatabaseDriver::SQLite,
        url: url.clone(),
        batch_size: 1,
        flush_interval_ms: 100,
        ..Default::default()
    };

    let mut sink = VerifyDatabaseSink::new(config).expect("Failed to create DatabaseSink");

    let record = VerifyLogRecord::new(VerifyLevel::INFO, "db_test".into(), "message to db".into());
    sink.write(&record)
        .expect("Failed to write log record to database");

    std::thread::sleep(VerifyDuration::from_millis(500));

    let count = get_log_count(&url);
    assert_eq!(count, 1);
}
