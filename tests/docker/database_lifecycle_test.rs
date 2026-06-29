// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! DatabaseSink 完整生命周期 Docker 集成测试
//!
//! 覆盖 init → write → flush → shutdown 完整流程，以及并发写入场景。

use super::get_test_db_url_or_skip;
use super::make_log_record;
use inklog::config::DatabaseSinkConfig;
use inklog::integrations::infra::database::DbNexusAdapter;
use inklog::integrations::infra::Database as DatabaseTrait;
use inklog::sink::database::DatabaseSink;
use inklog::sink::LogSink;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

async fn setup_sink(url: &str, batch_size: usize) -> DatabaseSink {
    let adapter = DbNexusAdapter::with_table_name(url, 5, "logs")
        .await
        .expect("Failed to create DbNexusAdapter");
    let session = adapter
        .pool()
        .get_session("admin")
        .await
        .expect("Failed to get session");
    session
        .execute_raw(super::CREATE_TABLE_SQL)
        .await
        .expect("Failed to create table");

    let db: Arc<dyn DatabaseTrait> = Arc::new(adapter);
    let config = DatabaseSinkConfig {
        name: "docker_lifecycle".to_string(),
        enabled: true,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: url.to_string(),
        batch_size,
        flush_interval_ms: 500,
        pool_size: 5,
        partition: inklog::config::PartitionStrategy::default(),
        table_name: "logs".to_string(),
        archive_format: "json".to_string(),
        parquet_config: inklog::config::ParquetConfig::default(),
    };
    DatabaseSink::new_with_config(db, Some(config)).expect("Failed to create DatabaseSink")
}

#[tokio::test]
#[serial]
async fn test_lifecycle_init_write_flush_shutdown() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 5).await;

    for i in 0..5 {
        let record = make_log_record("INFO", "lifecycle", &format!("step {}", i));
        sink.write(&record).expect("write: should succeed");
    }

    sink.flush().expect("flush: should succeed");
    sink.shutdown().expect("shutdown: should succeed");
}

#[tokio::test]
#[serial]
async fn test_lifecycle_write_after_flush() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 10).await;

    for i in 0..3 {
        sink.write(&make_log_record(
            "INFO",
            "lifecycle",
            &format!("first {}", i),
        ))
        .expect("write first batch");
    }
    sink.flush().expect("first flush");

    for i in 0..3 {
        sink.write(&make_log_record(
            "WARN",
            "lifecycle",
            &format!("second {}", i),
        ))
        .expect("write second batch");
    }
    sink.flush().expect("second flush");

    sink.shutdown().expect("shutdown");
}

#[tokio::test]
#[serial]
async fn test_lifecycle_concurrent_writes() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 10).await;
    let sink = Arc::new(sink);
    let sink_clone = sink.clone();

    let handle = std::thread::spawn(move || {
        for i in 0..10 {
            let record = make_log_record("INFO", "concurrent", &format!("thread1-{}", i));
            let _ = sink_clone.write(&record);
        }
    });

    for i in 0..10 {
        let record = make_log_record("ERROR", "concurrent", &format!("main-{}", i));
        let _ = sink.write(&record);
    }

    handle.join().expect("thread should complete");
    sink.flush().expect("flush after concurrent");
    sink.shutdown().expect("shutdown");
}

#[tokio::test]
#[serial]
async fn test_lifecycle_flush_repeatedly() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 100).await;

    for round in 0..3 {
        for i in 0..5 {
            sink.write(&make_log_record(
                "INFO",
                "lifecycle",
                &format!("round{}-{}", round, i),
            ))
            .expect("write");
        }
        sink.flush().expect("flush");
        std::thread::sleep(Duration::from_millis(50));
    }

    sink.shutdown().expect("shutdown");
}

#[tokio::test]
#[serial]
async fn test_lifecycle_shutdown_drains_buffer() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 100).await;

    for i in 0..5 {
        sink.write(&make_log_record(
            "INFO",
            "drain",
            &format!("unflushed {}", i),
        ))
        .expect("write");
    }

    sink.shutdown().expect("shutdown should drain buffer");
}
