// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 本地日志文件检索（含加密/压缩格式解包）。
//!
//! 供 `inklog-cli query` 与嵌入式消费方使用：对日志文件（默认模板格式
//! `{timestamp} [{level}] {target} - {message}`，兼容 JSON lines）按时间范围/
//! 级别/关键词检索。加密归档（`ENCLOG1` v1/v2 头）经 AES-256-GCM 解包后
//! 解析，密钥来自环境变量（与 `inklog-cli decrypt` 同一约定）；
//! `.zst`/`.gz` 压缩归档按对应 feature 解包。
//!
//! # Example
//!
//! ```no_run
//! use inklog::support::query::{query_paths, QueryOptions};
//! use std::path::PathBuf;
//!
//! let opts = QueryOptions {
//!     level: Some("warn".into()),
//!     keyword: Some("timeout".into()),
//!     ..Default::default()
//! };
//! let entries = query_paths(&[PathBuf::from("logs/app.log")], &opts, None).unwrap();
//! for e in entries {
//!     println!("{} {} {}", e.timestamp.map(|t| t.to_rfc3339()).unwrap_or_default(), e.level, e.message);
//! }
//! ```

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::{open_validated_file, InklogError};

/// 查询过滤条件。
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// 仅保留该时刻（含）之后的记录（RFC3339 语义）
    pub since: Option<DateTime<Utc>>,
    /// 仅保留该时刻（含）之前的记录
    pub until: Option<DateTime<Utc>>,
    /// 最低级别过滤（如 "warn" → 保留 WARN/ERROR/FATAL）
    pub level: Option<String>,
    /// 消息/target 子串过滤（大小写敏感）
    pub keyword: Option<String>,
    /// 最多返回的记录数（0 = 不限制）
    pub limit: usize,
}

/// 检索结果条目。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    /// 记录时间戳（无法解析行的行格式缺省为 None）
    pub timestamp: Option<DateTime<Utc>>,
    /// 日志级别
    pub level: String,
    /// 目标模块
    pub target: String,
    /// 消息内容
    pub message: String,
    /// 来源文件路径
    pub source: PathBuf,
}

/// 级别 → 序数（用于最低级别过滤）。
pub(crate) fn level_rank(level: &str) -> u8 {
    match level.to_ascii_uppercase().as_str() {
        "TRACE" => 0,
        "DEBUG" => 1,
        "INFO" => 2,
        "WARN" | "WARNING" => 3,
        "ERROR" => 4,
        "FATAL" | "CRITICAL" => 5,
        _ => u8::MAX,
    }
}

/// 解包单个日志文件为 UTF-8 文本。
///
/// - `ENCLOG1` 魔数（`inklog-cli decrypt` 同款 v1/v2 头）→ AES-256-GCM 解密；
///   需要 `key_env` 指定持有密钥的环境变量名；
/// - `.zst` → zstd 解压（`compression` feature）；
///   `.gz` → gzip 解压（`gzip` feature）；feature 未启用时返回配置错误；
/// - 其他 → 原样读取。
pub fn read_log_file(path: &Path, key_env: Option<&str>) -> Result<String, InklogError> {
    // 先读原始字节：魔数判断需要窥视文件头
    let mut file = open_validated_file(path)?;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)?;

    if raw.starts_with(ENCRYPTED_MAGIC) {
        let env = key_env.ok_or_else(|| {
            InklogError::ConfigError(format!(
                "encrypted log file '{}' requires a key env var (use --key-env, \
                 conventionally INKLOG_ENCRYPTION_KEY)",
                path.display()
            ))
        })?;
        let plaintext = decrypt_bytes(&raw, path, env)?;
        return String::from_utf8(plaintext).map_err(|e| {
            InklogError::ConfigError(format!(
                "decrypted log file '{}' is not valid UTF-8: {e}",
                path.display()
            ))
        });
    }

    match path.extension().and_then(|e| e.to_str()) {
        Some("zst") => {
            #[cfg(feature = "compression")]
            {
                let mut decoder = zstd::stream::Decoder::new(std::io::Cursor::new(&raw))
                    .map_err(|e| {
                        InklogError::ConfigError(format!("zstd decode failed for '{}': {e}", path.display()))
                    })?;
                let mut out = String::new();
                decoder.read_to_string(&mut out).map_err(|e| {
                    InklogError::ConfigError(format!("zstd decode failed for '{}': {e}", path.display()))
                })?;
                Ok(out)
            }
            #[cfg(not(feature = "compression"))]
            {
                let _ = path;
                Err(InklogError::ConfigError(
                    "'.zst' log files require the 'compression' feature".to_string(),
                ))
            }
        }
        Some("gz") => {
            #[cfg(feature = "gzip")]
            {
                use std::io::Read as _;
                let mut decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(&raw));
                let mut out = String::new();
                decoder.read_to_string(&mut out).map_err(|e| {
                    InklogError::ConfigError(format!(
                        "gzip decode failed for '{}': {e}",
                        path.display()
                    ))
                })?;
                Ok(out)
            }
            #[cfg(not(feature = "gzip"))]
            {
                let _ = path;
                Err(InklogError::ConfigError(
                    "'.gz' log files require the 'gzip' feature".to_string(),
                ))
            }
        }
        _ => String::from_utf8(raw).map_err(|e| {
            InklogError::ConfigError(format!(
                "log file '{}' is not valid UTF-8: {e}",
                path.display()
            ))
        }),
    }
}

/// `ENCLOG1\0`
const ENCRYPTED_MAGIC: &[u8] = b"ENCLOG1\0";

/// AES-256-GCM 解密（与 `inklog-cli decrypt` 的 v1(algo=1)/v2/legacy 格式对齐）。
fn decrypt_bytes(raw: &[u8], path: &Path, key_env: &str) -> Result<Vec<u8>, InklogError> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};

    if raw.len() < 10 {
        return Err(InklogError::ConfigError(format!(
            "encrypted file '{}' too small for a header",
            path.display()
        )));
    }
    let version = u16::from_le_bytes([raw[8], raw[9]]);
    let key_from_env =
        |salt: Option<&[u8]>| -> Result<zeroize::Zeroizing<[u8; 32]>, InklogError> {
        match salt {
            Some(s) => crate::support::io::sink::encryption::get_encryption_key_with_salt(key_env, s),
            None => crate::support::io::sink::encryption::get_encryption_key(key_env),
        }
    };

    match version {
        1 => {
            let algo = u16::from_le_bytes([raw[10], raw[11]]);
            let (nonce_bytes, ciphertext): ([u8; 12], &[u8]) = if algo == 1 {
                if raw.len() < 24 {
                    return Err(InklogError::ConfigError("truncated v1 header".into()));
                }
                (raw[12..24].try_into().expect("12 bytes"), &raw[24..])
            } else {
                // Legacy：MAGIC(8) + VER(2) + NONCE(12)
                if raw.len() < 22 {
                    return Err(InklogError::ConfigError("truncated legacy header".into()));
                }
                (raw[10..22].try_into().expect("12 bytes"), &raw[22..])
            };
            // v1 头不含盐：密码模式不可恢复，提前给出与 CLI 一致的诊断
            if crate::support::io::sink::encryption::env_key_is_password(key_env) {
                return Err(InklogError::ConfigError(
                    "v1 encrypted files written with a password-derived key are \
                     unrecoverable: the v1 header does not store the PBKDF2 salt"
                        .to_string(),
                ));
            }
            let key = key_from_env(None)?;
            let cipher = Aes256Gcm::new((&*key).into());
            let nonce = aes_gcm::Nonce::from(nonce_bytes);
            cipher
                .decrypt(&nonce, ciphertext)
                .map_err(|_| InklogError::ConfigError("decryption failed (wrong key?)".into()))
        }
        2 => {
            if raw.len() < 40 {
                return Err(InklogError::ConfigError(format!(
                    "truncated v2 header in '{}': expected 40 bytes, got {}",
                    path.display(),
                    raw.len()
                )));
            }
            let salt: [u8; 16] = raw[12..28].try_into().expect("16 bytes");
            let key = key_from_env(Some(&salt))?;
            let nonce: [u8; 12] = raw[28..40].try_into().expect("12 bytes");
            let cipher = Aes256Gcm::new((&*key).into());
            let nonce = aes_gcm::Nonce::from(nonce);
            cipher
                .decrypt(&nonce, &raw[40..])
                .map_err(|_| InklogError::ConfigError("decryption failed (wrong key?)".into()))
        }
        other => Err(InklogError::ConfigError(format!(
            "unsupported encryption version {other} in '{}'",
            path.display()
        ))),
    }
}

/// 解析一行日志（默认模板格式；容错 JSON lines；其余行返回 None）。
pub(crate) fn parse_line(line: &str, source: &Path) -> Option<LogEntry> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    // JSON lines 容错
    if trimmed.starts_with('{') {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let timestamp = json
                .get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(parse_timestamp);
            return Some(LogEntry {
                timestamp,
                level: json
                    .get("level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("INFO")
                    .to_string(),
                target: json
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                message: json
                    .get("message")
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                source: source.to_path_buf(),
            });
        }
    }
    // 默认模板：{timestamp} [{level}] {target} - {message}
    //   2026-09-11T01:02:03.456Z [INFO] my::target - hello world
    let ts_end = trimmed.find(" [")?;
    let timestamp = parse_timestamp(&trimmed[..ts_end]);
    let rest = &trimmed[ts_end + 2..];
    let level_end = rest.find(']')?;
    let level = rest[..level_end].trim().to_string();
    if level.is_empty() || !level.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let after_level = &rest[level_end + 1..];
    let after_level = after_level.strip_prefix(' ').unwrap_or(after_level);
    let (target, message) = match after_level.split_once(" - ") {
        Some((t, m)) => (t.to_string(), m.to_string()),
        None => (String::new(), after_level.to_string()),
    };
    Some(LogEntry {
        timestamp,
        level,
        target,
        message,
        source: source.to_path_buf(),
    })
}

/// 宽容的时间戳解析：RFC3339 → `%Y-%m-%dT%H:%M:%S%.3fZ`（FileSink 默认渲染）。
fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(t) = DateTime::parse_from_rfc3339(raw) {
        return Some(t.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.3fZ")
        .ok()
        .map(|naive| naive.and_utc())
}

/// 展开输入路径：文件原样收集，目录递归收集全部文件（排序保证确定性）。
fn expand_paths(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = inputs.to_vec();
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&p) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
        } else if p.is_file() {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// 对文件/目录集合执行检索（核心入口）。
///
/// 单个不可读/不可解包的文件不中断整体查询（跳过并继续）；全部失败时
/// 返回最后一个错误。
pub fn query_paths(
    inputs: &[PathBuf],
    opts: &QueryOptions,
    key_env: Option<&str>,
) -> Result<Vec<LogEntry>, InklogError> {
    let min_rank = opts.level.as_deref().map(level_rank);
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut last_error: Option<InklogError> = None;

    for path in expand_paths(inputs) {
        let content = match read_log_file(&path, key_env) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(target: "inklog::query", error = %e, path = %path.display(), "skip unreadable log file");
                last_error = Some(e);
                continue;
            }
        };
        for line in content.lines() {
            let Some(entry) = parse_line(line, &path) else {
                continue;
            };
            if let Some(min) = min_rank {
                let rank = level_rank(&entry.level);
                if rank == u8::MAX || rank < min {
                    continue;
                }
            }
            if let Some(since) = opts.since
                && entry.timestamp.is_none_or(|t| t < since)
            {
                continue;
            }
            if let Some(until) = opts.until
                && entry.timestamp.is_none_or(|t| t > until)
            {
                continue;
            }
            if let Some(keyword) = &opts.keyword
                && !entry.message.contains(keyword.as_str())
                && !entry.target.contains(keyword.as_str())
            {
                continue;
            }
            entries.push(entry);
            if opts.limit > 0 && entries.len() >= opts.limit {
                return Ok(entries);
            }
        }
    }

    if entries.is_empty()
        && let Some(e) = last_error
        && expand_paths(inputs).is_empty()
    {
        return Err(e);
    }
    Ok(entries)
}

/// 由 `inklog-cli` 使用的便捷封装：执行检索并返回 (结果, 退出码)。
///
/// 退出码契约：`0` = 有匹配/成功，`2` = 无匹配，`1` = 错误。
pub fn query_exit_code(entries: &[LogEntry]) -> i32 {
    if entries.is_empty() {
        2
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    const SAMPLE: &str = "\
2026-09-11T01:00:00.123Z [INFO] app::boot - service started
2026-09-11T02:00:00.000Z [WARN] app::net - slow upstream timeout
2026-09-11T03:30:00.000Z [ERROR] app::db - connection refused
not a log line
2026-09-11T04:00:00.000Z [FATAL] app::db - giving up
";

    #[test]
    fn test_parse_line_default_template() {
        let entry = parse_line(
            "2026-09-11T01:00:00.123Z [INFO] app::boot - service started",
            Path::new("x.log"),
        )
        .expect("must parse");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.target, "app::boot");
        assert_eq!(entry.message, "service started");
        assert_eq!(entry.timestamp.unwrap().to_rfc3339(), "2026-09-11T01:00:00.123+00:00");
    }

    #[test]
    fn test_parse_line_json_and_garbage() {
        let entry = parse_line(
            r#"{"timestamp":"2026-09-11T01:00:00Z","level":"WARN","target":"j","message":"{\"k\":1}"}"#,
            Path::new("x.log"),
        )
        .expect("JSON line must parse");
        assert_eq!(entry.level, "WARN");
        assert!(parse_line("not a log line", Path::new("x")).is_none());
        assert!(parse_line("", Path::new("x")).is_none());
    }

    #[test]
    fn test_query_filters_level_keyword_and_limit() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_file(dir.path(), "app.log", SAMPLE);

        // 级别过滤：warn 以上
        let opts = QueryOptions { level: Some("warn".into()), ..Default::default() };
        let got = query_paths(&[f.clone()], &opts, None).unwrap();
        assert_eq!(got.len(), 3);
        assert!(got.iter().all(|e| level_rank(&e.level) >= 3));

        // 关键词
        let opts = QueryOptions { keyword: Some("connection".into()), ..Default::default() };
        let got = query_paths(&[f.clone()], &opts, None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].message, "connection refused");

        // limit
        let opts = QueryOptions { limit: 2, ..Default::default() };
        let got = query_paths(&[f.clone()], &opts, None).unwrap();
        assert_eq!(got.len(), 2);

        // 时间范围
        let opts = QueryOptions {
            since: Some(DateTime::parse_from_rfc3339("2026-09-11T03:00:00Z").unwrap().with_timezone(&Utc)),
            until: Some(DateTime::parse_from_rfc3339("2026-09-11T03:45:00Z").unwrap().with_timezone(&Utc)),
            ..Default::default()
        };
        let got = query_paths(&[f], &opts, None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].level, "ERROR");
    }

    #[test]
    fn test_query_directory_recursive_and_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.log", SAMPLE);
        let sub = dir.path().join("rotated");
        std::fs::create_dir_all(&sub).unwrap();
        write_file(&sub, "b.log", "2026-09-11T05:00:00.000Z [ERROR] app::x - boom\n");

        let opts = QueryOptions { level: Some("error".into()), ..Default::default() };
        let got = query_paths(&[dir.path().to_path_buf()], &opts, None).unwrap();
        // a.log: ERROR + FATAL；b.log（递归）：ERROR
        assert_eq!(got.len(), 3);
        assert_eq!(query_exit_code(&got), 0);

        let empty = query_paths(
            &[dir.path().to_path_buf()],
            &QueryOptions { level: Some("trace".into()), keyword: Some("nope".into()), ..Default::default() },
            None,
        )
        .unwrap();
        assert!(empty.is_empty());
        assert_eq!(query_exit_code(&empty), 2, "no matches → exit code 2");
    }

    #[test]
    fn test_unreadable_file_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        // 目录内的 .zst 在未启用 compression 时被跳过，其余文件正常返回
        write_file(dir.path(), "bad.zst", "not really zstd");
        write_file(dir.path(), "ok.log", "2026-09-11T01:00:00.000Z [INFO] a - ok\n");
        let got = query_paths(&[dir.path().to_path_buf()], &QueryOptions::default(), None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].message, "ok");
    }

    #[test]
    #[serial_test::serial]
    fn test_encrypted_log_roundtrip_query() {
        let dir = tempfile::tempdir().unwrap();
        let plaintext = "2026-09-11T01:00:00.000Z [ERROR] app::secret - leaked\n";
        unsafe { std::env::set_var("INKLOG_QUERY_TEST_KEY", "0123456789abcdef0123456789abcdef") };
        let key = crate::support::io::sink::encryption::get_encryption_key("INKLOG_QUERY_TEST_KEY")
            .expect("key must be set");
        // 构造 v2 加密文件（与 FileSink::encrypt_file 的 v2 头格式一致）
        use aes_gcm::aead::Aead;
        use aes_gcm::{Aes256Gcm, KeyInit};
        let salt: [u8; 16] = rand::random();
        let nonce: [u8; 12] = rand::random();
        let cipher = Aes256Gcm::new((&*key).into());
        let nonce = aes_gcm::Nonce::from(nonce);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .expect("encrypt");
        let mut blob = Vec::new();
        blob.extend_from_slice(ENCRYPTED_MAGIC);
        blob.extend_from_slice(&2u16.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);
        let enc_path = dir.path().join("app.log.enc");
        std::fs::write(&enc_path, &blob).unwrap();
        let opts = QueryOptions { level: Some("error".into()), ..Default::default() };
        let got = query_paths(&[enc_path], &opts, Some("INKLOG_QUERY_TEST_KEY")).unwrap();
        assert_eq!(got.len(), 1, "encrypted log must be unpacked and searched");
        assert_eq!(got[0].message, "leaked");
    }
}
