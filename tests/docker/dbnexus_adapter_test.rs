// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! DbNexusAdapter Docker 数据库集成测试
//!
//! 覆盖 `DbNexusAdapter::new`、`with_table_name`、`insert_batch`、`is_healthy`、
//! 以及连接失败路径。仅在 `INKLOG_TEST_DB_URL` 设置时执行。

use super::cleanup_sqlite_file;
use super::err_to_string;
use super::get_test_db_url_or_skip;
use super::make_log_record;
use inklog::integrations::infra::database::{Database, DbNexusAdapter};
use serial_test::serial;

/// 辅助：创建测试用 DbNexusAdapter（带 logs 表）。
async fn setup_adapter(url: &str, table_name: &str) -> DbNexusAdapter {
    let adapter = DbNexusAdapter::with_table_name(url, 5, table_name)
        .await
        .expect("Failed to create DbNexusAdapter");
    // 确保表存在
    let session = adapter
        .pool()
        .get_session("admin")
        .await
        .expect("Failed to get session");
    session
        .execute_raw(super::CREATE_TABLE_SQL)
        .await
        .expect("Failed to create table");
    adapter
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_new_and_healthy() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return, // 跳过：未设置 INKLOG_TEST_DB_URL
    };

    let adapter = DbNexusAdapter::new(&url, 5).await.expect("new() failed");
    assert_eq!(adapter.table_name(), "logs");
    assert!(
        adapter.is_healthy().await,
        "新建的适配器应健康（数据库可达）"
    );
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_with_table_name() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let adapter = DbNexusAdapter::with_table_name(&url, 3, "custom_logs")
        .await
        .expect("with_table_name() failed");
    assert_eq!(adapter.table_name(), "custom_logs");
    assert!(adapter.is_healthy().await);
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_insert_batch_empty() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let adapter = setup_adapter(&url, "logs").await;
    // 空切片：应立即返回 Ok(0)，不触发任何 SQL
    let count = adapter
        .insert_batch(&[])
        .await
        .expect("insert_batch(empty) should succeed");
    assert_eq!(count, 0, "空批次应返回 0");
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_insert_batch_single() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let table = "logs_single";
    let adapter = setup_adapter(&url, table).await;
    let record = make_log_record("INFO", "docker_test", "single record");

    let count = adapter
        .insert_batch(&[record])
        .await
        .expect("insert_batch(single) should succeed");
    assert_eq!(count, 1, "应成功写入 1 条记录");
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_insert_batch_multiple() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let table = "logs_multi";
    let adapter = setup_adapter(&url, table).await;
    let records = vec![
        make_log_record("INFO", "docker_test", "message 1"),
        make_log_record("WARN", "docker_test", "message 2"),
        make_log_record("ERROR", "docker_test", "message 3"),
    ];

    let count = adapter
        .insert_batch(&records)
        .await
        .expect("insert_batch(multiple) should succeed");
    assert_eq!(count, 3, "应成功写入 3 条记录");
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_insert_batch_special_chars() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let table = "logs_special";
    let adapter = setup_adapter(&url, table).await;
    // 包含单引号、反斜杠、Unicode 等特殊字符
    let records = vec![
        make_log_record("INFO", "docker_test", "It's a test"),
        make_log_record("INFO", "docker_test", "back\\slash test"),
        make_log_record("INFO", "docker_test", "中文测试 日本語 русский"),
    ];

    let count = adapter
        .insert_batch(&records)
        .await
        .expect("insert_batch(special chars) should succeed");
    assert_eq!(count, 3, "特殊字符记录应全部写入");
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_insert_batch_nonexistent_table() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    // 不创建表，直接尝试插入
    let adapter = DbNexusAdapter::with_table_name(&url, 2, "nonexistent_table_xyz")
        .await
        .expect("adapter creation should succeed");
    let record = make_log_record("INFO", "docker_test", "to nonexistent table");

    let result = adapter.insert_batch(&[record]).await;
    assert!(
        result.is_err(),
        "写入不存在的表应返回错误，但结果: {:?}",
        result.as_ref().map(|c| *c)
    );
    let err_msg = err_to_string(&result.unwrap_err());
    assert!(
        err_msg.contains("Database") || err_msg.contains("Batch insert"),
        "错误信息应包含 Database 或 Batch insert，实际: {}",
        err_msg
    );
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_connection_failure_invalid_url() {
    // 无效 URL 应在 new() 阶段失败
    let result = DbNexusAdapter::new("invalid://not-a-real-db", 1).await;
    let err = match result {
        Ok(_) => panic!("无效 URL 应导致 new() 失败，但成功了"),
        Err(e) => e,
    };
    let err_str = err_to_string(&err);
    assert!(
        err_str.contains("Database") || err_str.contains("pool"),
        "错误信息应包含 Database 或 pool，实际: {}",
        err_str
    );
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_from_pool_reuses_connection() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let adapter1 = DbNexusAdapter::new(&url, 3)
        .await
        .expect("first adapter failed");
    let pool = adapter1.pool().clone();

    // 用同一个 pool 创建第二个适配器（不同表名）
    let adapter2 = DbNexusAdapter::from_pool(pool, "shared_pool_logs");
    assert_eq!(adapter2.table_name(), "shared_pool_logs");
    assert!(adapter2.is_healthy().await, "共享连接池的适配器应健康");
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_pool_accessor() {
    let url = match get_test_db_url_or_skip() {
        Some(u) => u,
        None => return,
    };

    let adapter = DbNexusAdapter::new(&url, 2).await.expect("new() failed");
    // pool() 访问器应返回非空引用（通过 is_healthy 间接验证）
    let _pool_ref = adapter.pool();
    assert!(adapter.is_healthy().await);
}

#[tokio::test]
#[serial]
async fn test_dbnexus_adapter_cleanup_sqlite() {
    // 验证 SQLite 文件清理辅助函数不会 panic
    let url = super::unique_sqlite_url("cleanup_test");
    cleanup_sqlite_file(&url); // 文件不存在，应静默通过
    assert!(url.starts_with("sqlite:///tmp/inklog_test_cleanup_test_"));
}
