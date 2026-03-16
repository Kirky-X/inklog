// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InklogConfig {
    pub global: GlobalConfig,
    pub console_sink: Option<ConsoleSinkConfig>,
    pub file_sink: Option<FileSinkConfig>,
    pub database_sink: Option<DatabaseSinkConfig>,
    #[cfg(feature = "aws")]
    pub s3_archive: Option<crate::archive::S3ArchiveConfig>,
    pub performance: PerformanceConfig,
    pub http_server: Option<HttpServerConfig>,
}

impl InklogConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.performance.channel_capacity == 0 {
            return Err("channel_capacity cannot be 0".to_string());
        }
        if self.performance.worker_threads == 0 {
            return Err("worker_threads cannot be 0".to_string());
        }
        Ok(())
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(level) = std::env::var("INKLOG_LEVEL") {
            self.global.level = level;
        }
        if let Ok(format) = std::env::var("INKLOG_FORMAT") {
            self.global.format = format;
        }
        if let Ok(enabled) = std::env::var("INKLOG_CONSOLE_ENABLED") {
            if enabled.to_lowercase() != "false" {
                self.console_sink = Some(crate::config::ConsoleSinkConfig::default());
            }
        }
        if let Ok(enabled) = std::env::var("INKLOG_FILE_ENABLED") {
            if enabled.to_lowercase() != "false" {
                self.file_sink = Some(crate::config::FileSinkConfig::default());
            }
        }
        if let Ok(path) = std::env::var("INKLOG_FILE_PATH") {
            if let Some(ref mut file) = self.file_sink {
                file.path = std::path::PathBuf::from(path);
            }
        }
        if let Ok(max_size) = std::env::var("INKLOG_FILE_MAX_SIZE") {
            if let Some(ref mut file) = self.file_sink {
                file.max_size = max_size;
            }
        }
        if let Ok(compress) = std::env::var("INKLOG_FILE_COMPRESS") {
            if let Some(ref mut file) = self.file_sink {
                file.compress = compress.to_lowercase() != "false";
            }
        }
        #[cfg(feature = "aws")]
        {
            if let Ok(enabled) = std::env::var("INKLOG_S3_ENABLED") {
                let is_enabled = enabled.to_lowercase() != "false";
                if is_enabled {
                    let mut s3 = crate::archive::S3ArchiveConfig::default();
                    s3.enabled = true;
                    self.s3_archive = Some(s3);
                }
            }
            if let Ok(bucket) = std::env::var("INKLOG_S3_BUCKET") {
                if let Some(ref mut s3) = self.s3_archive {
                    s3.bucket = bucket;
                }
            }
            if let Ok(region) = std::env::var("INKLOG_S3_REGION") {
                if let Some(ref mut s3) = self.s3_archive {
                    s3.region = region;
                }
            }
            if let Ok(format) = std::env::var("INKLOG_ARCHIVE_FORMAT") {
                if let Some(ref mut s3) = self.s3_archive {
                    s3.archive_format = format;
                }
            }
            if let Ok(algorithm) = std::env::var("INKLOG_S3_ENCRYPTION_ALGORITHM") {
                if let Some(ref mut s3) = self.s3_archive {
                    let algo = match algorithm.to_lowercase().as_str() {
                        "awskms" => crate::archive::EncryptionAlgorithm::AwsKms,
                        "aes256" => crate::archive::EncryptionAlgorithm::Aes256,
                        _ => crate::archive::EncryptionAlgorithm::Aes256,
                    };
                    let key_id = std::env::var("INKLOG_S3_ENCRYPTION_KMS_KEY_ID").ok();
                    s3.encryption = Some(crate::archive::EncryptionConfig {
                        algorithm: algo,
                        kms_key_id: key_id,
                        customer_key: crate::archive::SecretString::default(),
                    });
                }
            }
        }
        if let Ok(enabled) = std::env::var("INKLOG_HTTP_ENABLED") {
            let is_enabled = enabled.to_lowercase() != "false";
            if is_enabled {
                let mut http = crate::config::HttpServerConfig::default();
                http.enabled = true;
                self.http_server = Some(http);
            }
        }
        if let Ok(host) = std::env::var("INKLOG_HTTP_HOST") {
            if let Some(ref mut http) = self.http_server {
                http.host = host;
            }
        }
        if let Ok(port) = std::env::var("INKLOG_HTTP_PORT") {
            if let Ok(port_num) = port.parse() {
                if let Some(ref mut http) = self.http_server {
                    http.port = port_num;
                }
            }
        }
        if let Ok(metrics_path) = std::env::var("INKLOG_HTTP_METRICS_PATH") {
            if let Some(ref mut http) = self.http_server {
                http.metrics_path = metrics_path;
            }
        }
        if let Ok(health_path) = std::env::var("INKLOG_HTTP_HEALTH_PATH") {
            if let Some(ref mut http) = self.http_server {
                http.health_path = health_path;
            }
        }
        if let Ok(error_mode) = std::env::var("INKLOG_HTTP_ERROR_MODE") {
            if let Some(ref mut http) = self.http_server {
                http.error_mode = match error_mode.to_lowercase().as_str() {
                    "warn" => crate::config::HttpErrorMode::Warn,
                    "panic" => crate::config::HttpErrorMode::Panic,
                    "strict" => crate::config::HttpErrorMode::Strict,
                    _ => crate::config::HttpErrorMode::Warn,
                };
            }
        }
        if let Ok(threads) = std::env::var("INKLOG_WORKER_THREADS") {
            if let Ok(num) = threads.parse() {
                self.performance.worker_threads = num;
            }
        }
        if let Ok(capacity) = std::env::var("INKLOG_CHANNEL_CAPACITY") {
            if let Ok(num) = capacity.parse() {
                self.performance.channel_capacity = num;
            }
        }
    }

    pub fn sinks_enabled(&self) -> Vec<&'static str> {
        let mut sinks = Vec::new();
        if self.console_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("console");
        }
        if self.file_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("file");
        }
        if self.database_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("database");
        }
        #[cfg(feature = "aws")]
        if self.s3_archive.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("s3_archive");
        }
        sinks
    }

    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: InklogConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let search_paths = vec![
            std::env::var("INKLOG_CONFIG_PATH").ok(),
            Some("inklog_config.toml".to_string()),
            dirs::config_dir().map(|p| {
                p.join("inklog")
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string()
            }),
            Some("/etc/inklog/config.toml".to_string()),
        ];

        for path_opt in search_paths.into_iter().flatten() {
            if std::path::Path::new(&path_opt).exists() {
                return Self::from_file(&path_opt);
            }
        }

        Ok(InklogConfig::default())
    }
}

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

impl Default for InklogConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            console_sink: Some(ConsoleSinkConfig::default()),
            file_sink: None,
            database_sink: None,
            #[cfg(feature = "aws")]
            s3_archive: None,
            performance: PerformanceConfig::default(),
            http_server: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub masking_enabled: bool,
    #[serde(default = "default_true")]
    pub auto_fallback: bool,
    #[serde(default = "default_initial_delay")]
    pub fallback_initial_delay_ms: u64,
    #[serde(default = "default_max_delay")]
    pub fallback_max_delay_ms: u64,
    #[serde(default = "default_retries")]
    pub fallback_max_retries: u32,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "{timestamp} [{level}] {target} - {message}".to_string()
}

fn default_true() -> bool {
    true
}

fn default_initial_delay() -> u64 {
    1000
}

fn default_max_delay() -> u64 {
    60000
}

fn default_retries() -> u32 {
    10
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "{timestamp} [{level}] {target} - {message}".to_string(),
            masking_enabled: true,
            auto_fallback: true,
            fallback_initial_delay_ms: 1000,
            fallback_max_delay_ms: 60000,
            fallback_max_retries: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSinkConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub colored: bool,
    #[serde(default = "default_stderr_levels")]
    pub stderr_levels: Vec<String>,
}

fn default_stderr_levels() -> Vec<String> {
    vec!["error".to_string(), "warn".to_string()]
}

impl Default for ConsoleSinkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            colored: true,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSinkConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub path: PathBuf,
    #[serde(default = "default_max_size")]
    pub max_size: String,
    #[serde(default = "default_rotation_time")]
    pub rotation_time: String,
    #[serde(default = "default_keep_files")]
    pub keep_files: u32,
    #[serde(default = "default_true")]
    pub compress: bool,
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,
    #[serde(default = "default_false")]
    pub encrypt: bool,
    pub encryption_key_env: Option<String>,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_max_total_size")]
    pub max_total_size: String,
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_minutes: u64,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,
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

fn default_false() -> bool {
    false
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

fn default_cleanup_interval() -> u64 {
    60
}

fn default_batch_size() -> usize {
    100
}

fn default_flush_interval() -> u64 {
    100
}

impl Default for FileSinkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("logs/app.log"),
            max_size: "100MB".to_string(),
            rotation_time: "daily".to_string(),
            keep_files: 30,
            compress: true,
            compression_level: 3,
            encrypt: false,
            encryption_key_env: None,
            retention_days: 30,
            max_total_size: "1GB".to_string(),
            cleanup_interval_minutes: 60,
            batch_size: 100,
            flush_interval_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DatabaseDriver {
    #[serde(rename = "postgres")]
    #[default]
    PostgreSQL,
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "sqlite")]
    SQLite,
}

impl std::str::FromStr for DatabaseDriver {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(DatabaseDriver::PostgreSQL),
            "mysql" => Ok(DatabaseDriver::MySQL),
            "sqlite" | "sqlite3" => Ok(DatabaseDriver::SQLite),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for DatabaseDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDriver::PostgreSQL => write!(f, "postgres"),
            DatabaseDriver::MySQL => write!(f, "mysql"),
            DatabaseDriver::SQLite => write!(f, "sqlite"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PartitionStrategy {
    #[serde(rename = "monthly")]
    #[default]
    Monthly,
    #[serde(rename = "yearly")]
    Yearly,
}

impl std::str::FromStr for PartitionStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monthly" | "month" => Ok(PartitionStrategy::Monthly),
            "yearly" | "year" => Ok(PartitionStrategy::Yearly),
            _ => Err(format!("Unknown partition strategy: {}", s)),
        }
    }
}

impl std::fmt::Display for PartitionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionStrategy::Monthly => write!(f, "monthly"),
            PartitionStrategy::Yearly => write!(f, "yearly"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default = "default_max_row_group_size")]
    pub max_row_group_size: usize,
    #[serde(default = "default_max_page_size")]
    pub max_page_size: usize,
    #[serde(default)]
    pub include_fields: Vec<String>,
}

fn default_max_row_group_size() -> usize {
    10000
}

fn default_encoding() -> String {
    "PLAIN".to_string()
}

fn default_max_page_size() -> usize {
    1024 * 1024
}

impl Default for ParquetConfig {
    fn default() -> Self {
        Self {
            compression_level: 3,
            encoding: "PLAIN".to_string(),
            max_row_group_size: 10000,
            max_page_size: 1024 * 1024,
            include_fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSinkConfig {
    #[serde(default = "default_db_name")]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    pub url: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,
    #[serde(default)]
    pub partition: PartitionStrategy,
    #[serde(default)]
    pub archive_to_s3: bool,
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: u32,
    #[serde(default)]
    pub s3_bucket: Option<String>,
    #[serde(default)]
    pub s3_region: Option<String>,
    #[serde(default = "default_table_name")]
    pub table_name: String,
    #[serde(default = "default_archive_format")]
    pub archive_format: String,
    #[serde(default)]
    pub parquet_config: ParquetConfig,
}

fn default_db_name() -> String {
    "default".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_archive_after_days() -> u32 {
    30
}

fn default_table_name() -> String {
    "logs".to_string()
}

fn default_archive_format() -> String {
    "json".to_string()
}

impl Default for DatabaseSinkConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            enabled: false,
            driver: DatabaseDriver::PostgreSQL,
            url: "postgres://localhost/logs".to_string(),
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 500,
            partition: PartitionStrategy::Monthly,
            archive_to_s3: false,
            archive_after_days: 30,
            s3_bucket: None,
            s3_region: Some("us-east-1".to_string()),
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            parquet_config: ParquetConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelStrategy {
    #[serde(rename = "fixed")]
    #[default]
    Fixed,
    #[serde(rename = "adaptive")]
    Adaptive,
}

impl std::str::FromStr for ChannelStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fixed" => Ok(ChannelStrategy::Fixed),
            "adaptive" => Ok(ChannelStrategy::Adaptive),
            _ => Err(format!("Unknown channel strategy: {}", s)),
        }
    }
}

impl std::fmt::Display for ChannelStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelStrategy::Fixed => write!(f, "fixed"),
            ChannelStrategy::Adaptive => write!(f, "adaptive"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_health_path")]
    pub health_path: String,
    #[serde(default)]
    pub error_mode: HttpErrorMode,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    9090
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_health_path() -> String {
    "/health".to_string()
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 9090,
            metrics_path: "/metrics".to_string(),
            health_path: "/health".to_string(),
            error_mode: HttpErrorMode::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpErrorMode {
    #[serde(rename = "panic")]
    #[default]
    Panic,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "strict")]
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    #[serde(default)]
    pub channel_strategy: ChannelStrategy,
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold_percent: u8,
    #[serde(default = "default_shrink_threshold")]
    pub shrink_threshold_percent: u8,
    #[serde(default = "default_shrink_wait")]
    pub shrink_wait_seconds: u64,
    #[serde(default = "default_min_capacity")]
    pub min_capacity: usize,
    #[serde(default = "default_max_capacity")]
    pub max_capacity: usize,
}

fn default_channel_capacity() -> usize {
    10000
}

fn default_worker_threads() -> usize {
    3
}

fn default_expand_threshold() -> u8 {
    80
}

fn default_shrink_threshold() -> u8 {
    20
}

fn default_shrink_wait() -> u64 {
    30
}

fn default_min_capacity() -> usize {
    1000
}

fn default_max_capacity() -> usize {
    50000
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 10000,
            worker_threads: 3,
            channel_strategy: ChannelStrategy::default(),
            expand_threshold_percent: 80,
            shrink_threshold_percent: 20,
            shrink_wait_seconds: 30,
            min_capacity: 1000,
            max_capacity: 50000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_default() {
        let global = GlobalConfig::default();
        assert_eq!(global.level, "info");
        assert!(global.auto_fallback);
        assert!(global.fallback_initial_delay_ms > 0);
        assert!(global.fallback_max_delay_ms > 0);
    }

    #[test]
    fn test_global_config_setters() {
        let global = GlobalConfig {
            level: "debug".to_string(),
            auto_fallback: false,
            ..Default::default()
        };

        assert_eq!(global.level, "debug");
        assert!(!global.auto_fallback);
    }

    #[test]
    fn test_performance_config_default() {
        let perf = PerformanceConfig::default();
        assert_eq!(perf.channel_capacity, 10000);
        assert_eq!(perf.worker_threads, 3);
        assert_eq!(perf.channel_strategy, ChannelStrategy::Fixed);
    }

    #[test]
    fn test_performance_config_channel_strategy_fixed() {
        let perf = PerformanceConfig {
            channel_strategy: ChannelStrategy::Fixed,
            channel_capacity: 5000,
            ..Default::default()
        };
        match perf.channel_strategy {
            ChannelStrategy::Fixed => {}
            ChannelStrategy::Adaptive => panic!("Expected Fixed strategy"),
        }
        assert_eq!(perf.channel_capacity, 5000);
    }

    #[test]
    fn test_performance_config_channel_strategy_adaptive() {
        let perf = PerformanceConfig {
            channel_strategy: ChannelStrategy::Adaptive,
            channel_capacity: 20000,
            expand_threshold_percent: 70,
            shrink_threshold_percent: 30,
            shrink_wait_seconds: 60,
            min_capacity: 2000,
            max_capacity: 50000,
            ..Default::default()
        };
        match perf.channel_strategy {
            ChannelStrategy::Adaptive => {}
            ChannelStrategy::Fixed => panic!("Expected Adaptive strategy"),
        }
        assert_eq!(perf.expand_threshold_percent, 70);
        assert_eq!(perf.shrink_threshold_percent, 30);
        assert_eq!(perf.shrink_wait_seconds, 60);
        assert_eq!(perf.min_capacity, 2000);
        assert_eq!(perf.max_capacity, 50000);
    }

    #[test]
    fn test_console_config_default() {
        let console = ConsoleSinkConfig::default();
        assert!(console.enabled);
    }

    #[test]
    fn test_console_config_custom() {
        let console = ConsoleSinkConfig {
            enabled: false,
            colored: false,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
        };
        assert!(!console.enabled);
        assert!(!console.colored);
        assert_eq!(console.stderr_levels.len(), 2);
    }

    #[test]
    fn test_console_config_stderr_levels() {
        let config = ConsoleSinkConfig::default();
        assert_eq!(config.stderr_levels.len(), 2);
        assert!(config.stderr_levels.contains(&"error".to_string()));
        assert!(config.stderr_levels.contains(&"warn".to_string()));
    }

    #[test]
    fn test_file_config_default() {
        let file = FileSinkConfig::default();
        assert!(file.enabled);
        assert!(!file.max_size.is_empty());
        assert!(!file.rotation_time.is_empty());
        assert!(file.keep_files > 0);
        assert!(file.retention_days > 0);
    }

    #[test]
    fn test_file_config_rotation_times() {
        let mut config = FileSinkConfig::default();

        for time in ["hourly", "daily", "weekly", "monthly"] {
            config.rotation_time = time.to_string();
            assert_eq!(config.rotation_time, time);
        }
    }

    #[test]
    fn test_file_config_parse_size() {
        let config = FileSinkConfig::default();
        assert_eq!(config.max_size, "100MB");
        assert_eq!(config.max_total_size, "1GB");
    }

    #[test]
    fn test_file_config_batch_settings() {
        let config = FileSinkConfig {
            batch_size: 500,
            flush_interval_ms: 50,
            ..Default::default()
        };
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.flush_interval_ms, 50);
    }

    #[test]
    fn test_file_config_encryption_settings() {
        let config = FileSinkConfig {
            encrypt: true,
            encryption_key_env: Some("CUSTOM_KEY_VAR".to_string()),
            ..Default::default()
        };
        assert!(config.encrypt);
        assert_eq!(
            config.encryption_key_env,
            Some("CUSTOM_KEY_VAR".to_string())
        );
    }

    #[test]
    fn test_database_config_default() {
        let db = DatabaseSinkConfig::default();
        assert!(!db.enabled);
        assert!(!db.url.is_empty());
        assert_eq!(db.table_name, "logs");
        assert!(db.batch_size > 0);
        assert!(db.flush_interval_ms > 0);
    }

    #[test]
    fn test_database_config_url_parsing() {
        let config = DatabaseSinkConfig {
            url: "postgres://user:pass@localhost:5432/logs".to_string(),
            ..Default::default()
        };
        assert!(config.url.starts_with("postgres://"));
        assert!(config.url.contains("localhost"));
    }

    #[test]
    fn test_database_config_batch_settings() {
        let config = DatabaseSinkConfig {
            batch_size: 1000,
            flush_interval_ms: 500,
            ..Default::default()
        };
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.flush_interval_ms, 500);
    }

    #[test]
    fn test_file_config_path_operations() {
        let config = FileSinkConfig {
            path: PathBuf::from("/var/log/app.log"),
            ..Default::default()
        };

        assert!(config.path.is_absolute());
        assert_eq!(
            config.path.file_name().unwrap().to_string_lossy(),
            "app.log"
        );
    }

    #[test]
    fn test_inklog_config_sinks_enabled() {
        let config = InklogConfig::default();
        let sinks = config.sinks_enabled();
        assert!(sinks.contains(&"console"));
    }
}
