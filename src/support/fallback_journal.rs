// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 磁盘持久化 fallback 队列（deferred-capabilities C4）。
//!
//! ERROR/FATAL 兜底缓冲（内存 100 条 LRU）溢出/淘汰的记录落盘 JSONL journal，
//! 进程启动时经 [`FallbackJournal::replay`] 重放一次后清空——关键日志跨进程
//! 不丢。落盘不做 fsync：承诺"进程崩溃后可重放"（OS page cache 级），断电
//! 窗口与 file sink 一致。

use std::io::Write as _;
use std::path::PathBuf;

use crate::LogRecord;

/// 默认 journal 容量上限：10 MiB。
pub const DEFAULT_JOURNAL_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// 持久化 fallback 日志（JSONL，逐行一条 [`LogRecord`]）。
pub struct FallbackJournal {
    path: PathBuf,
    max_bytes: u64,
}

impl FallbackJournal {
    /// 以默认容量（10 MiB）打开 journal。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self::with_limit(path, DEFAULT_JOURNAL_MAX_BYTES)
    }

    /// 以指定容量打开 journal。
    pub fn with_limit(path: impl Into<PathBuf>, max_bytes: u64) -> Self {
        Self {
            path: path.into(),
            max_bytes: max_bytes.max(1),
        }
    }

    /// 追加一条记录。超过容量上限时截断头部（丢最旧）后再追加。
    ///
    /// 返回 `false` 表示 IO 失败（调用方静默计数，不得反压主链路）。
    pub fn spill(&self, record: &LogRecord) -> bool {
        let Ok(mut line) = serde_json::to_string(record) else {
            return false;
        };
        line.push('\n');
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        if self.current_size() + line.len() as u64 > self.max_bytes {
            self.truncate_head_for(line.len() as u64);
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut f| f.write_all(line.as_bytes()))
            .is_ok()
    }

    /// 重放全部记录并清空 journal（原子语义：读取后截断）。
    ///
    /// 损坏行跳过并计数，不中断重放。返回 `(记录, 跳过的损坏行数)`。
    pub fn replay(&self) -> (Vec<LogRecord>, u64) {
        let Ok(data) = std::fs::read_to_string(&self.path) else {
            return (Vec::new(), 0);
        };
        let mut records = Vec::new();
        let mut skipped = 0u64;
        for line in data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<LogRecord>(line) {
                Ok(record) => records.push(record),
                Err(_) => skipped += 1,
            }
        }
        // 清空：重放一次后 journal 归零
        let _ = std::fs::write(&self.path, b"");
        (records, skipped)
    }

    fn current_size(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// 截断头部：从最旧开始丢弃，直到腾出 `needed` 字节空间。
    fn truncate_head_for(&self, needed: u64) {
        let Ok(data) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let lines: Vec<&str> = data.lines().collect();
        let mut kept_start = lines.len();
        let mut freed = 0u64;
        for (i, line) in lines.iter().enumerate() {
            freed += line.len() as u64 + 1;
            kept_start = i + 1;
            if freed >= needed {
                break;
            }
        }
        let body = lines[kept_start..].join("\n");
        let tmp = self.path.with_extension("journal.tmp");
        if std::fs::write(&tmp, body)
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .is_err()
        {
            // 截断失败：放弃保留，直接清空（宁可丢旧也不超限增长）
            let _ = std::fs::write(&self.path, b"");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike as _;

    fn record(msg: &str) -> LogRecord {
        let mut r = LogRecord::new(
            tracing::Level::ERROR,
            "journal::test".to_string(),
            msg.to_string(),
        );
        r.fields.insert("k".to_string(), serde_json::json!("v"));
        r.trace_id = Some("0123456789abcdef0123456789abcdef".to_string());
        r
    }

    #[test]
    fn test_spill_replay_roundtrip_preserves_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = FallbackJournal::open(dir.path().join("fb.journal"));

        journal.spill(&record("first"));
        journal.spill(&record("second"));

        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].message, "first");
        assert_eq!(records[1].message, "second");
        assert_eq!(
            records[0].trace_id.as_deref(),
            Some("0123456789abcdef0123456789abcdef") // pragma: allowlist secret — 测试夹具 trace id
        );
        assert_eq!(records[0].fields.get("k"), Some(&serde_json::json!("v")));
        assert_eq!(records[0].timestamp.year(), chrono::Utc::now().year());

        // 重放后清空：二次重放为空
        let (records, _) = journal.replay();
        assert!(records.is_empty());
    }

    #[test]
    fn test_capacity_cap_drops_oldest() {
        let dir = tempfile::TempDir::new().unwrap();
        // 上限 1 KiB：每条约 300 字节，灌 8 条必然触发截断丢最旧
        let journal = FallbackJournal::with_limit(dir.path().join("fb.journal"), 1024);
        for i in 0..8 {
            journal.spill(&record(&format!("payload-{i}-{}", "x".repeat(40))));
        }
        let (records, _) = journal.replay();
        assert!(
            records.len() < 8,
            "cap must drop oldest entries: {}",
            records.len()
        );
        // 保留的是最新的（payload-7 必在）
        assert!(
            records.iter().any(|r| r.message.starts_with("payload-7")),
            "newest entry must survive truncation"
        );
        assert!(
            records.iter().all(|r| !r.message.starts_with("payload-0")),
            "oldest entry must be dropped"
        );
    }

    #[test]
    fn test_corrupt_lines_skipped_without_panic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("fb.journal");
        journal_spill(&FallbackJournal::open(&path), &record("good-1"));
        // 半行损坏 + 合法 + 非JSON
        std::fs::write(
            &path,
            "{\"half\":\"line\n{{{{not-json\n\"line without braces\n",
        )
        .unwrap();
        let journal = FallbackJournal::open(&path);
        journal.spill(&record("good-2"));
        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 3, "corrupt lines counted and skipped");
        // good-1 已被上面的 fs::write 覆盖，仅 good-2 合法
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "good-2");
    }

    fn journal_spill(journal: &FallbackJournal, record: &LogRecord) {
        assert!(journal.spill(record));
    }

    #[test]
    fn test_replay_on_missing_file_is_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = FallbackJournal::open(dir.path().join("nonexistent.journal"));
        let (records, skipped) = journal.replay();
        assert!(records.is_empty());
        assert_eq!(skipped, 0);
    }
}
