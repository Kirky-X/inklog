// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Docker 数据库集成测试入口
//!
//! 通过 `INKLOG_TEST_DB_URL` 环境变量连接真实数据库，覆盖需要真实 DB 连接的代码路径。
//! 未设置环境变量时，所有测试被跳过（不导致 `cargo test` 失败）。
//!
//! # 启动 Docker 数据库
//!
//! ```bash
//! docker compose -f docker/docker-compose.test.yml up -d
//! export INKLOG_TEST_DB_URL="sqlite:///tmp/inklog_test.db"
//! cargo test --test docker --features sqlite
//! ```
//!
//! # 测试边界
//!
//! 仅测试 inklog 自身的 `DbNexusAdapter` / `DatabaseSink` 适配器逻辑，
//! 不测试 `dbnexus` / `sea-orm` 库内部实现。

#![cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]

use inklog::InklogError;
use std::env;

mod database_lifecycle_test;
mod database_sink_test;
mod dbnexus_adapter_test;

/// 读取 `INKLOG_TEST_DB_URL` 环境变量。
pub fn get_test_db_url() -> Option<String> {
    env::var("INKLOG_TEST_DB_URL").ok()
}

/// 检测 Docker 数据库是否可用，不可用则跳过测试。
pub fn get_test_db_url_or_skip() -> Option<String> {
    let url = env::var("INKLOG_TEST_DB_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    Some(url)
}

/// 构造一个唯一的 SQLite 文件 URL，用于隔离测试。
///
/// `name` 中的字符仅保留字母/数字/下划线/连字符，其余一律替换为 `_`，
/// 防止外部输入破坏 `/tmp` 下的文件路径拼接。
pub fn unique_sqlite_url(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let pid = std::process::id();
    format!(
        "sqlite:///tmp/inklog_test_{}_{}.db?mode=rwc",
        sanitized, pid
    )
}

/// 辅助函数：构造测试用 LogRecord。
pub fn make_log_record(level: &str, target: &str, message: &str) -> inklog::LogRecord {
    use inklog::tracing::Level;
    let lvl = match level.to_uppercase().as_str() {
        "ERROR" => Level::ERROR,
        "WARN" => Level::WARN,
        "INFO" => Level::INFO,
        "DEBUG" => Level::DEBUG,
        _ => Level::TRACE,
    };
    inklog::LogRecord::new(lvl, target.to_string(), message.to_string())
}

/// 清理：删除指定 URL 对应的 SQLite 文件（仅对 sqlite:// URL 生效）。
pub fn cleanup_sqlite_file(url: &str) {
    if let Some(path) = url
        .strip_prefix("sqlite://")
        .and_then(|s| s.split('?').next())
    {
        let _ = std::fs::remove_file(path);
    }
}

/// 将 `InklogError` 转为可读字符串，用于断言失败信息。
pub fn err_to_string(e: &InklogError) -> String {
    e.to_string()
}

#[test]
fn unique_sqlite_url_keeps_safe_characters() {
    let pid = std::process::id();
    assert_eq!(
        unique_sqlite_url("cleanup_test-1"),
        format!("sqlite:///tmp/inklog_test_cleanup_test-1_{pid}.db?mode=rwc")
    );
}

#[test]
fn unique_sqlite_url_sanitizes_unsafe_characters() {
    let pid = std::process::id();
    assert_eq!(
        unique_sqlite_url("bad name/../x"),
        format!("sqlite:///tmp/inklog_test_bad_name____x_{pid}.db?mode=rwc")
    );
}
