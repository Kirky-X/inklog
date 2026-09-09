// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! File sink configuration.

use super::global::default_true;
use crate::support::processing::template::OutputFormat;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// FileSinkConfig - File output settings
// ============================================================================

/// File sink configuration.
///
/// Controls logging output to files with support for rotation, compression,
/// encryption, and retention policies.
///
/// # Features
///
/// - **Log Rotation**: Automatic file rotation by size or time interval
/// - **Compression**: Zstd compression for rotated log files
/// - **Encryption**: AES-256-GCM encryption for sensitive logs
/// - **Retention**: Automatic cleanup of old log files based on age or total size
/// - **Batching**: Configurable batch size for improved throughput
///
/// # Example TOML Configuration
///
/// ```toml
/// [file_sink]
/// enabled = true
/// path = "logs/app.log"
/// max_size = "100MB"
/// rotation_time = "daily"
/// keep_files = 30
/// compress = true
/// compression_level = 3
/// encrypt = false
/// encryption_key_env = "LOG_ENCRYPTION_KEY"
/// retention_days = 30
/// max_total_size = "1GB"
/// cleanup_interval_minutes = 60
/// batch_size = 100
/// flush_interval_ms = 100
/// masking_enabled = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSinkConfig {
    /// Enable file logging.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Path to the log file.
    #[serde(default = "default_log_path")]
    pub path: PathBuf,

    /// Maximum size of a single log file before rotation.
    #[serde(default = "default_max_size")]
    pub max_size: String,

    /// Time-based rotation interval.
    #[serde(default = "default_rotation_time")]
    pub rotation_time: String,

    /// Maximum number of rotated files to keep.
    #[serde(default = "default_keep_files")]
    pub keep_files: u32,

    /// Enable compression for rotated log files.
    #[serde(default = "default_true")]
    pub compress: bool,

    /// Zstd compression level (1-22).
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,

    /// Enable AES-256-GCM encryption for log files.
    #[serde(default)]
    pub encrypt: bool,

    /// Environment variable name for the encryption key.
    #[serde(default)]
    pub encryption_key_env: Option<String>,

    /// Delete log files older than N days.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Maximum total size of all log files combined.
    #[serde(default = "default_max_total_size")]
    pub max_total_size: String,

    /// Interval between cleanup runs (minutes).
    #[serde(default = "default_cleanup_interval_minutes")]
    pub cleanup_interval_minutes: u64,

    /// Number of log records to buffer before writing to disk.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum time to wait before flushing buffer (milliseconds).
    #[serde(default = "default_flush_interval_ms")]
    pub flush_interval_ms: u64,

    /// Enable PII masking for file output: structured PII (emails, phone
    /// numbers, ID/bank card numbers) and values under sensitive field names
    /// are masked before the record is persisted.
    ///
    /// 推荐新名 `pii_masking_enabled`；旧名 `masking_enabled` 为兼容别名，
    /// 继续接受（serde alias），序列化时始终输出旧名。
    ///
    /// # 三个易混淆开关的分工
    ///
    /// - **sanitizer**（`global.masking_enabled`，推荐名 `sanitizer_enabled`）：
    ///   注入转义与消息脱敏，作用于 subscriber 入口（全局）。
    /// - **pii_masking**（本字段，推荐名 `pii_masking_enabled`）：结构化 PII
    ///   掩码，作用于本 sink 持久化前。
    /// - **safe_message**（`InklogError::safe_message`，error.rs 的
    ///   SENSITIVE_PATTERNS）：错误消息出口脱敏，独立于以上两个开关。
    #[serde(default = "default_true", alias = "pii_masking_enabled")]
    pub masking_enabled: bool,

    /// Output format: text or JSON (NDJSON).
    #[serde(default)]
    pub output_format: OutputFormat,
}

fn default_log_path() -> PathBuf {
    PathBuf::from("logs/app.log")
}
fn default_max_size() -> String {
    "100MB".to_string()
}
fn default_rotation_time() -> String {
    "daily".to_string()
}
fn default_keep_files() -> u32 {
    30
}
fn default_compression_level() -> i32 {
    3
}
fn default_retention_days() -> u32 {
    30
}
fn default_max_total_size() -> String {
    "1GB".to_string()
}
fn default_cleanup_interval_minutes() -> u64 {
    60
}
fn default_batch_size() -> usize {
    100
}
fn default_flush_interval_ms() -> u64 {
    100
}

impl Default for FileSinkConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            path: default_log_path(),
            max_size: default_max_size(),
            rotation_time: default_rotation_time(),
            keep_files: default_keep_files(),
            compress: default_true(),
            compression_level: default_compression_level(),
            encrypt: false,
            encryption_key_env: None,
            retention_days: default_retention_days(),
            max_total_size: default_max_total_size(),
            cleanup_interval_minutes: default_cleanup_interval_minutes(),
            batch_size: default_batch_size(),
            flush_interval_ms: default_flush_interval_ms(),
            masking_enabled: default_true(),
            output_format: OutputFormat::default(),
        }
    }
}

impl FileSinkConfig {
    /// Validate the configuration.
    ///
    /// Checks:
    /// - `compression_level` is in valid range (1..=22 for zstd, 1..=9 for gzip)
    /// - `max_size` and `max_total_size` are parseable size strings
    /// - If `encrypt` is true, `encryption_key_env` must be `Some` and the
    ///   referenced environment variable must exist
    pub fn validate(&self) -> Result<(), String> {
        if self.compression_level < 1 || self.compression_level > 22 {
            return Err(format!(
                "compression_level must be between 1 and 22, got {}",
                self.compression_level
            ));
        }
        parse_config_size(&self.max_size)
            .map_err(|e| format!("max_size \"{}\": {e}", self.max_size))?;
        parse_config_size(&self.max_total_size)
            .map_err(|e| format!("max_total_size \"{}\": {e}", self.max_total_size))?;
        if self.encrypt {
            match self.encryption_key_env.as_ref() {
                None => {
                    return Err("encrypt is enabled but encryption_key_env is not set".to_string());
                }
                Some(env_name) if std::env::var(env_name).is_err() => {
                    return Err(format!(
                        "encrypt is enabled but environment variable \"{env_name}\" is not set"
                    ));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }
}

/// Parse a size string (e.g. `"100MB"`, `"1GB"`, `"512KB"`, bare bytes) with
/// the same rules as the runtime size parser, so invalid values can be
/// rejected at configuration time instead of silently falling back.
fn parse_config_size(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim().to_uppercase();

    let (multiplier, suffix_len): (u64, usize) = if size_str.ends_with("TB") {
        (1024 * 1024 * 1024 * 1024, 2)
    } else if size_str.ends_with("GB") {
        (1024 * 1024 * 1024, 2)
    } else if size_str.ends_with("MB") {
        (1024 * 1024, 2)
    } else if size_str.ends_with("KB") {
        (1024, 2)
    } else if size_str.ends_with("B") {
        (1, 1)
    } else {
        (1, 0)
    };

    let num_str = &size_str[..size_str.len() - suffix_len];
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("\"{num_str}\" is not a valid size number"))?;

    num.checked_mul(multiplier)
        .ok_or_else(|| format!("\"{size_str}\" overflows u64"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rejects_invalid_compression_level() {
        let mut config = FileSinkConfig::default();
        config.compression_level = 0;
        assert!(config.validate().is_err());

        config.compression_level = 23;
        assert!(config.validate().is_err());

        config.compression_level = 3;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_encrypt_without_key_env() {
        let mut config = FileSinkConfig::default();
        config.encrypt = true;
        config.encryption_key_env = None;
        assert!(config.validate().is_err());

        // 引用的环境变量真实存在才允许 encrypt = true
        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_FILE_SINK_KEY_ENV") };
        config.encryption_key_env = Some("INKLOG_TEST_FILE_SINK_KEY_ENV".to_string());
        assert!(config.validate().is_err());

        // SAFETY: test-only env var mutation
        unsafe { std::env::set_var("INKLOG_TEST_FILE_SINK_KEY_ENV", "test-key") };
        assert!(config.validate().is_ok());

        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_FILE_SINK_KEY_ENV") };
    }

    #[test]
    fn test_validate_rejects_invalid_size_strings() {
        let mut config = FileSinkConfig::default();

        config.max_size = "INVALID".to_string();
        assert!(config.validate().is_err());

        config.max_size = String::new();
        assert!(config.validate().is_err());

        config.max_size = "10XB".to_string();
        assert!(config.validate().is_err());

        config.max_size = "100MB".to_string();
        config.max_total_size = "no-size".to_string();
        assert!(config.validate().is_err());

        config.max_total_size = "1GB".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_accepts_valid_size_strings() {
        let mut config = FileSinkConfig::default();
        for size in ["100", "512KB", "100MB", "1GB", "2TB", " 5MB "] {
            config.max_size = size.to_string();
            config.max_total_size = size.to_string();
            assert!(
                config.validate().is_ok(),
                "size \"{size}\" should be accepted"
            );
        }
    }

    #[test]
    fn test_parse_config_size() {
        assert_eq!(parse_config_size("100").unwrap(), 100);
        assert_eq!(parse_config_size("1KB").unwrap(), 1024);
        assert_eq!(parse_config_size("100MB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_config_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_config_size("  5MB  ").unwrap(), 5 * 1024 * 1024);
        assert!(parse_config_size("invalid").is_err());
        assert!(parse_config_size("").is_err());
        assert!(parse_config_size("99999999999999999999TB").is_err());
    }

    #[test]
    fn test_deserialize_masking_enabled_legacy_and_alias_keys() {
        // 旧键名 `masking_enabled`：既有配置文件必须继续解析（兼容别名）
        let legacy: FileSinkConfig = toml::from_str("masking_enabled = false\n").unwrap();
        assert!(!legacy.masking_enabled);

        // 新键名 `pii_masking_enabled`（推荐写法，serde alias）解析到同一字段
        let renamed: FileSinkConfig = toml::from_str("pii_masking_enabled = false\n").unwrap();
        assert!(!renamed.masking_enabled);
    }

    #[test]
    fn test_serialize_emits_legacy_masking_enabled_key() {
        // 序列化始终输出旧键名，保证既有配置消费者/生成器不受影响
        let cfg = FileSinkConfig {
            masking_enabled: false,
            ..Default::default()
        };
        let rendered = toml::to_string(&cfg).unwrap();
        assert!(
            rendered.contains("masking_enabled = false"),
            "serialized TOML must keep the legacy key, got: {rendered}"
        );
        assert!(
            !rendered.contains("pii_masking_enabled"),
            "alias must not be emitted on serialization, got: {rendered}"
        );
    }
}
