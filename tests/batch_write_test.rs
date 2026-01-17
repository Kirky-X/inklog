// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use inklog::config::DatabaseDriver;
use inklog::sink::database::DatabaseSink;
use inklog::sink::LogSink;
use inklog::{log_record::LogRecord, DatabaseSinkConfig};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tracing::Level;

// ============ Test Helper Functions ============

/// Creates a DatabaseSink for testing with SQLite
fn create_test_database_sink(
    batch_size: usize,
    flush_interval_ms: u64,
) -> (TempDir, DatabaseSink, String) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let config = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: url.clone(),
        batch_size,
        flush_interval_ms,
        ..Default::default()
    };

    let sink = DatabaseSink::new(config).expect("Failed to create DatabaseSink");
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
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for flush interval to pass
    std::thread::sleep(Duration::from_millis(1100));

    // Write 4th record - this triggers time-based flush (3 records flushed)
    let record = LogRecord::new(Level::INFO, "batch_test".into(), "Trigger flush".into());
    sink.write(&record).expect("Failed to write log record");

    // Wait for flush to complete
    std::thread::sleep(Duration::from_millis(200));

    let count_before = count_database_logs(&url);
    // After time-based flush, 4 records should be in DB
    assert_eq!(count_before, 4, "时间刷新应该写入4条记录");

    // Write 5 more records to trigger batch-based flush (batch_size=5)
    for i in 4..9 {
        let record = LogRecord::new(Level::INFO, "batch_test".into(), format!("Message {}", i));
        sink.write(&record).expect("Failed to write log record");
    }

    // Wait for batch flush to complete
    std::thread::sleep(Duration::from_millis(500));

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

    let record1 = LogRecord::new(Level::INFO, "timeout_test".into(), "First message".into());
    sink.write(&record1)
        .expect("Failed to write first log record");

    std::thread::sleep(Duration::from_millis(500));

    let record2 = LogRecord::new(Level::INFO, "timeout_test".into(), "Second message".into());
    sink.write(&record2)
        .expect("Failed to write second log record");

    std::thread::sleep(Duration::from_millis(500));

    let count = count_database_logs(&url);

    assert!(count >= 1, "超时刷新应该触发写入，当前记录数: {}", count);

    println!("超时刷新测试通过！刷新间隔: 300ms, 实际写入: {}", count);
}
