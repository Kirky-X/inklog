// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! DatabaseSink Docker 数据库集成测试
//!
//! 覆盖 `DatabaseSink` 的批量写入、flush、shutdown 逻辑，
//! 以及 `adjust_batch_size` 自适应批量调整的间接覆盖路径。

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

/// 创建测试用 DbNexusAdapter 并确保 logs 表存在。
async fn setup_adapter(url: &str) -> DbNexusAdapter {
    let adapter = DbNexusAdapter::with_table_name(url, 3, "logs")
        .await
        .expect("Failed to create DbNexusAdapter");
    // 使用 `execute_raw_ddl` 执行 DDL：dbnexus 启用 sql-parser feature 后，
    // `execute_raw` 会拦截 DDL 语句，必须用 `execute_raw_ddl` 通道（仅限 admin）。
    let session = adapter
        .pool()
        .get_session("admin")
        .await
        .expect("Failed to get session");
    session
        .execute_raw_ddl(super::CREATE_TABLE_SQL)
        .await
        .expect("Failed to create table");
    adapter
}

/// 创建测试用 DatabaseSink（带配置）。
async fn setup_sink(url: &str, batch_size: usize, flush_interval_ms: u64) -> DatabaseSink {
    let adapter = setup_adapter(url).await;
    let db: Arc<dyn DatabaseTrait> = Arc::new(adapter);
    let config = DatabaseSinkConfig {
        name: "docker_test".to_string(),
        enabled: true,
        driver: inklog::config::DatabaseDriver::SQLite,
        url: url.to_string(),
        batch_size,
        flush_interval_ms,
        pool_size: 3,
        partition: inklog::config::PartitionStrategy::default(),
        table_name: "logs".to_string(),
        archive_format: "json".to_string(),
        parquet_config: inklog::config::ParquetConfig::default(),
    };
    DatabaseSink::new_with_config(db, Some(config)).expect("Failed to create DatabaseSink")
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_create_and_shutdown() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 5, 1000).await;
    let result = sink.shutdown();
    assert!(result.is_ok(), "shutdown 应成功: {:?}", result.err());
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_write_below_batch_threshold() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 10, 60_000).await;

    for i in 0..3 {
        let record = make_log_record("INFO", "docker_test", &format!("msg {}", i));
        sink.write(&record).expect("write should succeed");
    }

    sink.flush().expect("flush should succeed");
    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_write_triggers_batch_flush() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 3, 60_000).await;

    for i in 0..3 {
        let record = make_log_record("INFO", "docker_test", &format!("batch {}", i));
        sink.write(&record).expect("write should succeed");
    }

    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_flush_interval_triggers() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 100, 200).await;

    let record = make_log_record("INFO", "docker_test", "interval flush test");
    sink.write(&record).expect("write should succeed");

    std::thread::sleep(Duration::from_millis(400));

    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_high_volume_adjust_batch_size_up() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 2, 50).await;

    // 写入大量记录，触发多次 batch flush + adjust_batch_size
    for i in 0..30 {
        let record = make_log_record("INFO", "docker_test", &format!("high vol {}", i));
        let _ = sink.write(&record);
    }

    sink.flush().expect("flush should succeed");
    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_multiple_levels() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 5, 1000).await;

    let levels = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
    for (i, lvl) in levels.iter().enumerate() {
        let record = make_log_record(lvl, "docker_test", &format!("level test {}", i));
        sink.write(&record).expect("write should succeed");
    }

    sink.flush().expect("flush should succeed");
    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_double_shutdown_safe() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 5, 1000).await;

    let first = sink.shutdown();
    assert!(first.is_ok(), "第一次 shutdown 应成功");

    let _ = sink.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_flush_empty_buffer() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let sink = setup_sink(&url, 5, 1000).await;

    sink.flush().expect("flush empty buffer should succeed");
    sink.shutdown().expect("shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_database_sink_default_config() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    // 使用 new()（默认配置，无 config 参数）
    let adapter = setup_adapter(&url).await;
    let db: Arc<dyn DatabaseTrait> = Arc::new(adapter);
    let sink = DatabaseSink::new(db).expect("new() should succeed");

    let record = make_log_record("INFO", "default_config", "test");
    sink.write(&record).expect("write should succeed");
    sink.flush().expect("flush should succeed");
    sink.shutdown().expect("shutdown should succeed");
}
