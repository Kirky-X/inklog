// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! File sink configuration.

use super::global::default_true;
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

    /// Enable sensitive data masking for file output.
    #[serde(default = "default_true")]
    pub masking_enabled: bool,
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
        }
    }
}
