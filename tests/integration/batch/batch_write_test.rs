// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! DatabaseSink 批量写入与刷新语义测试。
//!
//! 核正：原实现经 sea_orm 直查 sqlite 且带双重 cfg 门控（无 db feature 时
//! `DatabaseSink` import 失败、有 db feature 时用例整体被 cfg 掉），在任何
//! feature 组合下都从未编译运行。现统一改走 DI 的 MockDatabaseAdapter
//! （test-utils feature），CI 主口径（sqlite）下真实运行。
#![cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]

use inklog::config::DatabaseSinkConfig;
use inklog::sink::LogSink;
use inklog::sink::database::DatabaseSink;
use inklog::{MockDatabaseAdapter, log_record::LogRecord};
use std::sync::Arc;
use tempfile::TempDir;

// Test Helper Functions

fn make_record(message: &str) -> LogRecord {
    LogRecord::new(
        inklog::tracing::Level::INFO,
        "batch_test".into(),
        message.into(),
    )
}

/// Creates a DatabaseSink backed by MockDatabaseAdapter with given batch settings
fn create_test_database_sink(
    batch_size: usize,
    flush_interval_ms: u64,
) -> (TempDir, DatabaseSink, Arc<MockDatabaseAdapter>) {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");

    let config = DatabaseSinkConfig {
        name: "test".to_string(),
        enabled: true,
        batch_size,
        flush_interval_ms,
        pool_size: 5,
        ..Default::default()
    };

    let mock_db = Arc::new(MockDatabaseAdapter::new());
    let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config))
        .expect("Failed to create DatabaseSink");
    (temp_dir, sink, mock_db)
}

// Tests

#[tokio::test]
async fn test_database_batch_write() {
    let (_temp_dir, sink, mock_db) = create_test_database_sink(5, 10_000);

    // 3 条（未满批次）写入：仅入缓冲，flush 后全部落库
    for i in 0..3 {
        sink.write(&make_record(&format!("Message {}", i)))
            .await
            .expect("Failed to write log record");
    }
    assert_eq!(
        mock_db.record_count(),
        0,
        "records must not persist before flush"
    );
    sink.flush().await.expect("Failed to flush");
    assert_eq!(
        mock_db.record_count(),
        3,
        "all buffered records should persist with the flush"
    );

    // 补满批次（共 5 条）再次 flush，不产生重复落库
    for i in 3..5 {
        sink.write(&make_record(&format!("Message {}", i)))
            .await
            .expect("Failed to write log record");
    }
    sink.flush().await.expect("Failed to flush");
    assert_eq!(
        mock_db.record_count(),
        5,
        "total records after flush should be 5, got: {}",
        mock_db.record_count()
    );

    println!(
        "batch write test passed! batch size: 5, actually written: {}",
        mock_db.record_count()
    );
}

#[tokio::test]
async fn test_database_manual_flush() {
    let (_temp_dir, sink, mock_db) = create_test_database_sink(100, 10_000);

    for i in 0..2 {
        sink.write(&make_record(&format!("Message {}", i)))
            .await
            .expect("Failed to write log record");
    }

    // flush_interval 未到且未满批次：仅缓冲不落库，手动 flush 触发落库
    assert_eq!(
        mock_db.record_count(),
        0,
        "records must not persist before flush"
    );
    sink.flush().await.expect("Failed to flush");
    assert_eq!(
        mock_db.record_count(),
        2,
        "manual flush should write 2 records; current count: {}",
        mock_db.record_count()
    );
}

#[tokio::test]
async fn test_database_flush_empty_buffer() {
    let (_temp_dir, sink, mock_db) = create_test_database_sink(10, 10_000);

    // 没有写入任何记录，直接 flush 应该返回 Ok 且不落库
    sink.flush()
        .await
        .expect("flushing an empty buffer should return Ok");
    assert_eq!(mock_db.record_count(), 0);
}
