// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! FileSink 配置构造与文件操作工具。
//!
//! 把 `examples/src/bin/file.rs` 中可复用的逻辑提取出来，便于单元测试覆盖：
//!
//! - [`create_file_config`]：按参数构造 [`FileSinkConfig`]，提供合理的默认值。
//! - [`create_log_record`]：构造带级别/消息/目标的 [`LogRecord`]。
//! - [`write_level_records`]：把多个级别的日志写入任意 sink。
//! - [`cleanup_files`]：按前缀删除指定目录下的临时文件，返回删除数量。

use anyhow::Result;
use chrono::Utc;
use inklog::domain::config::FileSinkConfig;
use inklog::support::io::sink::LogSink;
use inklog::LogRecord;
use std::fs;
use std::path::{Path, PathBuf};

/// 构造 FileSink 配置。
///
/// `max_size` 接受字符串是为了与 [`FileSinkConfig::max_size`] 的字符串格式保持一致
/// （例如 `"10MB"`、`"100"`）。其余字段使用示例中常用的默认值，避免调用方重复填写。
pub fn create_file_config(path: &str, max_size: &str, compress: bool) -> FileSinkConfig {
    FileSinkConfig {
        enabled: true,
        path: PathBuf::from(path),
        max_size: max_size.to_string(),
        rotation_time: "daily".to_string(),
        keep_files: 10,
        compress,
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        retention_days: 7,
        max_total_size: "1GB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 100,
        flush_interval_ms: 1000,
        masking_enabled: false,
    }
}

/// 创建带指定级别、消息和目标的日志记录。
///
/// 时间戳使用当前 UTC 时间，`thread_id` 固定为 `"main"`，便于测试断言。
pub fn create_log_record(level: &str, message: &str, target: &str) -> LogRecord {
    LogRecord {
        timestamp: Utc::now(),
        level: level.to_string(),
        message: message.to_string(),
        target: target.to_string(),
        fields: Default::default(),
        file: None,
        line: None,
        thread_id: "main".to_string(),
    }
}

/// 把多个级别的日志按顺序写入 sink，并 flush。
///
/// 每个级别生成一条记录，消息使用 `format!("这是一条 {} 级别日志", level)`。
/// 返回写入的记录数（等于 `levels.len()`），便于调用方做完整性检查。
pub async fn write_level_records(sink: &dyn LogSink, levels: &[&str]) -> Result<usize> {
    let mut written = 0;
    for level in levels {
        let record = create_log_record(level, &format!("这是一条 {} 级别日志", level), "file_ops");
        sink.write(&record).await?;
        written += 1;
    }
    sink.flush().await?;
    Ok(written)
}

/// 删除指定目录下文件名包含 `prefix` 的所有文件，返回删除数量。
///
/// `log_path` 用于定位日志所在目录（取其 parent）；如果目录不存在则返回 0。
/// 单个文件删除失败不会中断整体清理，但会汇总到返回的 `Result` 中。
pub fn cleanup_files(log_path: &str, prefix: &str) -> Result<usize> {
    let log_dir = match Path::new(log_path).parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };

    if !log_dir.exists() {
        return Ok(0);
    }

    let mut deleted = 0usize;
    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.contains(prefix) && fs::remove_file(entry.path()).is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inklog::support::io::sink::file::FileSink;
    use tempfile::tempdir;

    #[test]
    fn test_create_file_config_defaults() {
        // 验证：构造的配置字段与传入参数一致，且未启用压缩/加密。
        let config = create_file_config("/tmp/test.log", "10MB", false);
        assert!(config.enabled);
        assert_eq!(config.path, PathBuf::from("/tmp/test.log"));
        assert_eq!(config.max_size, "10MB");
        assert_eq!(config.rotation_time, "daily");
        assert_eq!(config.keep_files, 10);
        assert!(!config.compress);
        assert_eq!(config.compression_level, 3);
        assert!(!config.encrypt);
        assert!(config.encryption_key_env.is_none());
        assert_eq!(config.retention_days, 7);
        assert_eq!(config.max_total_size, "1GB");
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_ms, 1000);
        assert!(!config.masking_enabled);
    }

    #[test]
    fn test_create_file_config_with_compression() {
        // 验证：启用压缩时 compress=true，其余字段保持默认。
        let config = create_file_config("/tmp/compressed.log", "100", true);
        assert!(config.compress);
        assert_eq!(config.max_size, "100");
        assert_eq!(config.compression_level, 3);
    }

    #[test]
    fn test_create_log_record_fields() {
        // 验证：所有字段按入参设置；thread_id 固定 main；fields 为空。
        let record = create_log_record("WARN", "disk almost full", "disk_monitor");
        assert_eq!(record.level, "WARN");
        assert_eq!(record.message, "disk almost full");
        assert_eq!(record.target, "disk_monitor");
        assert_eq!(record.thread_id, "main");
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
        // timestamp 必须是"刚刚"的时间，防止误用零值
        let now = Utc::now();
        let diff = now.signed_duration_since(record.timestamp);
        assert!(diff.num_seconds().abs() < 5, "timestamp 应为当前时间");
    }

    #[tokio::test]
    async fn test_write_level_records() {
        // 验证：写入 5 个级别记录后返回 5，且文件非空。
        let dir = tempdir().expect("创建临时目录失败");
        let log_path = dir.path().join("write_levels.log");
        let path_str = log_path.to_str().unwrap();

        let config = create_file_config(path_str, "10MB", false);
        let sink = FileSink::new(config).expect("创建 FileSink 失败");

        let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        let written = write_level_records(&sink, &levels).await.expect("写入失败");
        assert_eq!(written, 5);

        // 文件应已被创建且包含内容
        assert!(log_path.exists(), "日志文件应存在");
        let content = fs::read_to_string(&log_path).expect("读取文件失败");
        assert!(!content.is_empty(), "日志文件不应为空");
        for level in &levels {
            assert!(content.contains(level), "日志内容应包含级别 {}", level);
        }
    }

    #[tokio::test]
    async fn test_write_level_records_empty() {
        // 验证：空级别列表返回 0 且不报错。
        let dir = tempdir().expect("创建临时目录失败");
        let log_path = dir.path().join("empty.log");
        let config = create_file_config(log_path.to_str().unwrap(), "10MB", false);
        let sink = FileSink::new(config).expect("创建 FileSink 失败");
        let written = write_level_records(&sink, &[])
            .await
            .expect("写入空列表应成功");
        assert_eq!(written, 0);
    }

    #[test]
    fn test_cleanup_files_removes_matching() {
        // 验证：cleanup_files 只删除文件名包含 prefix 的文件，并返回正确数量。
        let dir = tempdir().expect("创建临时目录失败");
        let prefix = "inklog_test_cleanup_match";
        let match_a = dir.path().join(format!("{}_a.log", prefix));
        let match_b = dir.path().join(format!("{}_b.log", prefix));
        let nomatch = dir.path().join("other_file.log");
        fs::write(&match_a, b"a").unwrap();
        fs::write(&match_b, b"b").unwrap();
        fs::write(&nomatch, b"c").unwrap();

        // log_path 指向目录内任一文件即可（cleanup 取 parent）；anchor 自身不存在，故不计数。
        let anchor = dir.path().join(format!("{}.log", prefix));
        let deleted = cleanup_files(anchor.to_str().unwrap(), prefix).expect("cleanup 失败");
        assert_eq!(deleted, 2, "应删除 2 个匹配文件（match_a、match_b）");
        assert!(!match_a.exists());
        assert!(!match_b.exists());
        assert!(nomatch.exists(), "未匹配的文件不应被删除");
    }

    #[test]
    fn test_cleanup_files_no_match() {
        // 验证：无匹配文件时返回 0，不报错。
        let dir = tempdir().expect("创建临时目录失败");
        let anchor = dir.path().join("anchor.log");
        let deleted = cleanup_files(anchor.to_str().unwrap(), "nonexistent_prefix_xyz")
            .expect("cleanup 失败");
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_cleanup_files_missing_dir() {
        // 验证：目录不存在时返回 0，不报错（fail-loud 的反面：缺目录视为无文件可删）。
        let deleted = cleanup_files("/nonexistent/path/file.log", "any").expect("cleanup 失败");
        assert_eq!(deleted, 0);
    }
}
