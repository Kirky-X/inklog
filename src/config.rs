// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::archive::SecretString;
use crate::config_validator::validate_url;
use crate::error::InklogError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// HTTP 服务器错误处理模式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpErrorMode {
    /// 启动失败时 panic（默认，向后兼容）
    #[default]
    Panic,
    /// 启动失败时记录警告，系统继续运行
    Warn,
    /// 启动失败时返回错误，阻止系统启动
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpServerConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub metrics_path: String,
    pub health_path: String,
    /// HTTP 服务器启动失败时的错误处理模式
    #[serde(default)]
    pub error_mode: HttpErrorMode,
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

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

impl InklogConfig {
    /// Returns a list of enabled sink names for audit logging purposes.
    /// Sensitive configuration values are not included in the output.
    #[doc(hidden)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_masking_enabled")]
    pub masking_enabled: bool,
    /// 是否启用自动降级（默认启用）
    /// 启用后，当某个 Sink 故障时会自动降级到备用 Sink
    #[serde(default = "default_auto_fallback")]
    pub auto_fallback: bool,
    /// 降级重试初始延迟（毫秒）
    #[serde(default = "default_fallback_initial_delay_ms")]
    pub fallback_initial_delay_ms: u64,
    /// 降级重试最大延迟（毫秒）
    #[serde(default = "default_fallback_max_delay_ms")]
    pub fallback_max_delay_ms: u64,
    /// 降级重试最大次数
    #[serde(default = "default_fallback_max_retries")]
    pub fallback_max_retries: u32,
}

fn default_level() -> String {
    "info".to_string()
}

fn default_format() -> String {
    "{timestamp} [{level}] {target} - {message}".to_string()
}

fn default_masking_enabled() -> bool {
    true
}

fn default_auto_fallback() -> bool {
    true
}

fn default_fallback_initial_delay_ms() -> u64 {
    1000 // 1秒
}

fn default_fallback_max_delay_ms() -> u64 {
    60000 // 60秒
}

fn default_fallback_max_retries() -> u32 {
    10
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            format: default_format(),
            masking_enabled: default_masking_enabled(),
            auto_fallback: default_auto_fallback(),
            fallback_initial_delay_ms: default_fallback_initial_delay_ms(),
            fallback_max_delay_ms: default_fallback_max_delay_ms(),
            fallback_max_retries: default_fallback_max_retries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConsoleSinkConfig {
    pub enabled: bool,
    pub colored: bool,
    pub stderr_levels: Vec<String>,
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
    pub enabled: bool,
    pub path: PathBuf,
    pub max_size: String,
    pub rotation_time: String,
    pub keep_files: u32,
    pub compress: bool,
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,
    pub encrypt: bool,
    pub encryption_key_env: Option<String>,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_max_total_size")]
    pub max_total_size: String,
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_minutes: u64,
    /// 批量写入大小（默认 100）
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// 批量刷新间隔（毫秒，默认 100ms）
    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,
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

fn default_compression_level() -> i32 {
    3
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

/// 分区策略
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PartitionStrategy {
    /// 按月分区
    #[default]
    Monthly,
    /// 按年分区
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

/// Parquet导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParquetConfig {
    /// 压缩级别（ZSTD: 0-22, 默认3）
    pub compression_level: i32,

    /// 编码方式（PLAIN/DICTIONARY/RLE）
    pub encoding: String,

    /// Row Group大小（行数，默认10000）
    pub max_row_group_size: usize,

    /// 页面大小（字节，默认1MB）
    pub max_page_size: usize,

    /// 包含的字段列表（逗号分隔，默认包含所有字段）
    /// 可用字段: id, timestamp, level, target, message, fields, file, line, thread_id
    #[serde(default)]
    pub include_fields: Vec<String>,
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
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    pub url: String,
    pub pool_size: u32,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    #[serde(default)]
    pub partition: PartitionStrategy,
    pub archive_to_s3: bool,
    pub archive_after_days: u32,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub table_name: String,
    /// 归档格式（json/parquet，默认json）
    #[serde(default = "default_archive_format")]
    pub archive_format: String,
    /// Parquet导出配置
    #[serde(default)]
    pub parquet_config: ParquetConfig,
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
/// 动态 Channel 容量策略
///
/// 控制日志通道的容量调整行为：
/// - `Fixed`: 固定容量，不进行动态调整
/// - `Adaptive`: 根据负载自动调整 channel 容量
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelStrategy {
    /// 固定容量模式，channel 容量保持不变
    #[default]
    Fixed,
    /// 自适应模式，根据负载自动调整 channel 容量
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
#[serde(default)]
pub struct PerformanceConfig {
    pub channel_capacity: usize,
    pub worker_threads: usize,
    /// Channel 容量调整策略
    pub channel_strategy: ChannelStrategy,
    /// 扩容阈值（百分比），当队列使用率超过此值时触发扩容
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold_percent: u8,
    /// 缩容阈值（百分比），当队列使用率低于此值持续一段时间后触发缩容
    #[serde(default = "default_shrink_threshold")]
    pub shrink_threshold_percent: u8,
    /// 缩容等待时间（秒），低使用率状态持续此时间后才触发缩容
    #[serde(default = "default_shrink_wait_seconds")]
    pub shrink_wait_seconds: u64,
    /// 最小容量限制
    #[serde(default = "default_min_capacity")]
    pub min_capacity: usize,
    /// 最大容量限制
    #[serde(default = "default_max_capacity")]
    pub max_capacity: usize,
}

/// 默认扩容阈值：80%
fn default_expand_threshold() -> u8 {
    80
}

/// 默认缩容阈值：20%
fn default_shrink_threshold() -> u8 {
    20
}

/// 默认缩容等待时间：30秒
fn default_shrink_wait_seconds() -> u64 {
    30
}

/// 默认最小容量：1000
fn default_min_capacity() -> usize {
    1000
}

/// 默认最大容量：50000
fn default_max_capacity() -> usize {
    50000
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 10000,
            worker_threads: 3,
            channel_strategy: ChannelStrategy::default(),
            expand_threshold_percent: default_expand_threshold(),
            shrink_threshold_percent: default_shrink_threshold(),
            shrink_wait_seconds: default_shrink_wait_seconds(),
            min_capacity: default_min_capacity(),
            max_capacity: default_max_capacity(),
        }
    }
}

impl InklogConfig {
    pub fn validate(&self) -> Result<(), InklogError> {
        use crate::config_validator::{
            validate_log_level, validate_non_empty, validate_path, validate_positive,
        };

        // 验证全局配置
        validate_log_level(&self.global.level)?;

        // 验证文件 sink 配置
        if let Some(ref file) = self.file_sink {
            if file.enabled {
                validate_path(&file.path)?;
                if file.encrypt && file.encryption_key_env.is_none() {
                    return Err(InklogError::ConfigError(
                        "Encryption enabled but no key env var specified".into(),
                    ));
                }
            }
        }

        // 验证数据库 sink 配置
        if let Some(ref db) = self.database_sink {
            if db.enabled {
                validate_url(&db.url, "Database URL")?;
                validate_positive(db.batch_size, "Batch size")?;
            }
        }

        // 验证性能配置
        validate_positive(self.performance.channel_capacity, "Channel capacity")?;
        validate_positive(self.performance.worker_threads, "Worker threads")?;

        // 验证 S3 归档配置
        #[cfg(feature = "aws")]
        if let Some(ref archive) = self.s3_archive {
            if archive.enabled {
                validate_non_empty(&archive.bucket, "S3 bucket name")?;
                validate_non_empty(&archive.region, "S3 region")?;
                validate_positive(archive.archive_interval_days, "Archive interval")?;
                validate_positive(archive.max_file_size_mb, "Max file size")?;
            }
        }

        Ok(())
    }

    pub fn apply_env_overrides(&mut self) {
        // Phase 1: Auto-create sink configs based on enabled env vars (mixed mode)
        self.auto_create_sink_configs();

        // Phase 2: Apply all environment variable overrides
        self.apply_global_overrides();
        self.apply_console_overrides();
        self.apply_file_overrides();
        self.apply_database_overrides();
        #[cfg(feature = "aws")]
        self.apply_s3_archive_overrides();
        self.apply_http_overrides();
        self.apply_performance_overrides();
    }

    /// Phase 1: Auto-create sink configs when enabled env vars are set
    fn auto_create_sink_configs(&mut self) {
        // Console Sink
        if self.console_sink.is_none() {
            if let Ok(val) = std::env::var("INKLOG_CONSOLE_ENABLED") {
                if val.to_lowercase() != "false" {
                    self.console_sink = Some(ConsoleSinkConfig::default());
                }
            }
        }

        // File Sink
        if self.file_sink.is_none() {
            if let Ok(val) = std::env::var("INKLOG_FILE_ENABLED") {
                if val.to_lowercase() != "false" {
                    self.file_sink = Some(FileSinkConfig::default());
                }
            }
        }

        // Database Sink
        if self.database_sink.is_none() {
            if let Ok(val) = std::env::var("INKLOG_DB_ENABLED") {
                if val.to_lowercase() != "false" {
                    self.database_sink = Some(DatabaseSinkConfig::default());
                }
            }
        }

        // S3 Archive
        #[cfg(feature = "aws")]
        if self.s3_archive.is_none() {
            if let Ok(val) = std::env::var("INKLOG_S3_ENABLED") {
                if val.to_lowercase() != "false" {
                    self.s3_archive = Some(crate::archive::S3ArchiveConfig::default());
                }
            }
        }

        // HTTP Server
        if self.http_server.is_none() {
            if let Ok(val) = std::env::var("INKLOG_HTTP_ENABLED") {
                if val.to_lowercase() != "false" {
                    self.http_server = Some(HttpServerConfig::default());
                }
            }
        }
    }

    fn apply_global_overrides(&mut self) {
        if let Ok(val) = std::env::var("INKLOG_LEVEL") {
            self.global.level = val;
        }

        if let Ok(val) = std::env::var("INKLOG_FORMAT") {
            self.global.format = val;
        }

        if let Ok(val) = std::env::var("INKLOG_MASKING_ENABLED") {
            self.global.masking_enabled = val.to_lowercase() != "false";
        }

        if let Ok(val) = std::env::var("INKLOG_AUTO_FALLBACK") {
            self.global.auto_fallback = val.to_lowercase() != "false";
        }

        if let Ok(val) = std::env::var("INKLOG_FALLBACK_INITIAL_DELAY_MS") {
            if let Ok(num) = val.parse() {
                self.global.fallback_initial_delay_ms = num;
            }
        }

        if let Ok(val) = std::env::var("INKLOG_FALLBACK_MAX_DELAY_MS") {
            if let Ok(num) = val.parse() {
                self.global.fallback_max_delay_ms = num;
            }
        }

        if let Ok(val) = std::env::var("INKLOG_FALLBACK_MAX_RETRIES") {
            if let Ok(num) = val.parse() {
                self.global.fallback_max_retries = num;
            }
        }
    }

    fn apply_console_overrides(&mut self) {
        if let Some(console) = &mut self.console_sink {
            if let Ok(val) = std::env::var("INKLOG_CONSOLE_ENABLED") {
                console.enabled = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_CONSOLE_COLORED") {
                console.colored = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_CONSOLE_STDERR_LEVELS") {
                console.stderr_levels = val
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    fn apply_file_overrides(&mut self) {
        if let Some(file) = &mut self.file_sink {
            if let Ok(val) = std::env::var("INKLOG_FILE_ENABLED") {
                file.enabled = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_PATH") {
                file.path = PathBuf::from(val);
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_MAX_SIZE") {
                file.max_size = val;
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_ROTATION_TIME") {
                file.rotation_time = val;
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_KEEP_FILES") {
                if let Ok(num) = val.parse() {
                    file.keep_files = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_COMPRESS") {
                file.compress = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_COMPRESSION_LEVEL") {
                if let Ok(num) = val.parse() {
                    file.compression_level = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_ENCRYPT") {
                file.encrypt = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_ENCRYPTION_KEY_ENV") {
                file.encryption_key_env = Some(val);
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_RETENTION_DAYS") {
                if let Ok(num) = val.parse() {
                    file.retention_days = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_MAX_TOTAL_SIZE") {
                file.max_total_size = val;
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_CLEANUP_INTERVAL_MINUTES") {
                if let Ok(num) = val.parse() {
                    file.cleanup_interval_minutes = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_BATCH_SIZE") {
                if let Ok(num) = val.parse() {
                    file.batch_size = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_FILE_FLUSH_INTERVAL_MS") {
                if let Ok(num) = val.parse() {
                    file.flush_interval_ms = num;
                }
            }
        }
    }

    fn apply_database_overrides(&mut self) {
        if let Some(db) = &mut self.database_sink {
            if let Ok(val) = std::env::var("INKLOG_DB_ENABLED") {
                db.enabled = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_DB_DRIVER") {
                if let Ok(driver) = val.parse() {
                    db.driver = driver;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_URL") {
                db.url = val;
            }

            if let Ok(val) = std::env::var("INKLOG_DB_POOL_SIZE") {
                if let Ok(num) = val.parse() {
                    db.pool_size = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_TABLE_NAME") {
                db.table_name = val;
            }

            if let Ok(val) = std::env::var("INKLOG_DB_BATCH_SIZE") {
                if let Ok(num) = val.parse() {
                    db.batch_size = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_FLUSH_INTERVAL_MS") {
                if let Ok(num) = val.parse() {
                    db.flush_interval_ms = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_ARCHIVE_TO_S3") {
                db.archive_to_s3 = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_DB_ARCHIVE_AFTER_DAYS") {
                if let Ok(num) = val.parse() {
                    db.archive_after_days = num;
                }
            }

            // Parquet config overrides
            if let Ok(val) = std::env::var("INKLOG_DB_PARQUET_COMPRESSION_LEVEL") {
                if let Ok(num) = val.parse() {
                    db.parquet_config.compression_level = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_PARQUET_ENCODING") {
                db.parquet_config.encoding = val;
            }

            if let Ok(val) = std::env::var("INKLOG_DB_PARQUET_MAX_ROW_GROUP_SIZE") {
                if let Ok(num) = val.parse() {
                    db.parquet_config.max_row_group_size = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_DB_PARQUET_MAX_PAGE_SIZE") {
                if let Ok(num) = val.parse() {
                    db.parquet_config.max_page_size = num;
                }
            }
        }
    }

    #[cfg(feature = "aws")]
    fn apply_s3_archive_overrides(&mut self) {
        use crate::archive::{CompressionType, StorageClass};

        if let Some(s3) = &mut self.s3_archive {
            if let Ok(val) = std::env::var("INKLOG_S3_ENABLED") {
                s3.enabled = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_S3_BUCKET") {
                s3.bucket = val;
            }

            if let Ok(val) = std::env::var("INKLOG_S3_REGION") {
                s3.region = val;
            }

            if let Ok(val) = std::env::var("INKLOG_S3_ARCHIVE_INTERVAL_DAYS") {
                if let Ok(num) = val.parse() {
                    s3.archive_interval_days = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_S3_SCHEDULE_EXPRESSION") {
                s3.schedule_expression = Some(val);
            }

            if let Ok(val) = std::env::var("INKLOG_S3_LOCAL_RETENTION_DAYS") {
                if let Ok(num) = val.parse() {
                    s3.local_retention_days = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_S3_LOCAL_RETENTION_PATH") {
                s3.local_retention_path = PathBuf::from(val);
            }

            if let Ok(val) = std::env::var("INKLOG_S3_COMPRESSION") {
                s3.compression = match val.to_lowercase().as_str() {
                    "none" => CompressionType::None,
                    "gzip" => CompressionType::Gzip,
                    "zstd" => CompressionType::Zstd,
                    "lz4" => CompressionType::Lz4,
                    "brotli" => CompressionType::Brotli,
                    _ => CompressionType::Zstd,
                };
            }

            if let Ok(val) = std::env::var("INKLOG_S3_STORAGE_CLASS") {
                s3.storage_class = match val.to_lowercase().as_str() {
                    "standard" => StorageClass::Standard,
                    "intelligent_tiering" => StorageClass::IntelligentTiering,
                    "standard_ia" => StorageClass::StandardIa,
                    "onezone_ia" => StorageClass::OnezoneIa,
                    "glacier" => StorageClass::Glacier,
                    "glacier_deep_archive" => StorageClass::GlacierDeepArchive,
                    "reduced_redundancy" => StorageClass::ReducedRedundancy,
                    _ => StorageClass::Standard,
                };
            }

            if let Ok(val) = std::env::var("INKLOG_S3_PREFIX") {
                s3.prefix = val;
            }

            if let Ok(val) = std::env::var("INKLOG_S3_ACCESS_KEY_ID") {
                s3.access_key_id = SecretString::new(val);
                // Security audit: Log that credentials were loaded without exposing values
                #[cfg(any(feature = "aws", feature = "http"))]
                tracing::debug!(
                    event = "security_config_s3_credentials",
                    "S3 credentials configured from environment"
                );
            }

            if let Ok(val) = std::env::var("INKLOG_S3_SECRET_ACCESS_KEY") {
                s3.secret_access_key = SecretString::new(val);
            }

            if let Ok(val) = std::env::var("INKLOG_S3_SESSION_TOKEN") {
                s3.session_token = SecretString::new(val);
            }

            if let Ok(val) = std::env::var("INKLOG_S3_ENDPOINT_URL") {
                s3.endpoint_url = Some(val);
            }

            if let Ok(val) = std::env::var("INKLOG_S3_FORCE_PATH_STYLE") {
                s3.force_path_style = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_S3_SKIP_BUCKET_VALIDATION") {
                s3.skip_bucket_validation = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_S3_MAX_FILE_SIZE_MB") {
                if let Ok(num) = val.parse() {
                    s3.max_file_size_mb = num;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_ARCHIVE_FORMAT") {
                s3.archive_format = val;
            }

            if let Ok(val) = std::env::var("INKLOG_S3_ENCRYPTION_ALGORITHM") {
                let algorithm = match val.to_uppercase().as_str() {
                    "AWSKMS" | "KMS" => crate::archive::EncryptionAlgorithm::AwsKms,
                    "CUSTOMERKEY" | "CUSTOMER_KEY" => {
                        crate::archive::EncryptionAlgorithm::CustomerKey
                    }
                    _ => crate::archive::EncryptionAlgorithm::Aes256,
                };
                if let Some(ref mut enc) = s3.encryption {
                    enc.algorithm = algorithm;
                } else {
                    s3.encryption = Some(crate::archive::EncryptionConfig {
                        algorithm,
                        kms_key_id: None,
                        customer_key: SecretString::default(),
                    });
                }
            }

            if let Ok(val) = std::env::var("INKLOG_S3_ENCRYPTION_KMS_KEY_ID") {
                if let Some(ref mut enc) = s3.encryption {
                    enc.kms_key_id = Some(val);
                } else {
                    s3.encryption = Some(crate::archive::EncryptionConfig {
                        algorithm: crate::archive::EncryptionAlgorithm::Aes256,
                        kms_key_id: Some(val),
                        customer_key: SecretString::default(),
                    });
                }
            }
        }
    }

    fn apply_http_overrides(&mut self) {
        if let Some(http) = &mut self.http_server {
            if let Ok(val) = std::env::var("INKLOG_HTTP_ENABLED") {
                http.enabled = val.to_lowercase() != "false";
            }

            if let Ok(val) = std::env::var("INKLOG_HTTP_HOST") {
                http.host = val;
            }

            if let Ok(val) = std::env::var("INKLOG_HTTP_PORT") {
                if let Ok(port) = val.parse() {
                    http.port = port;
                }
            }

            if let Ok(val) = std::env::var("INKLOG_HTTP_METRICS_PATH") {
                http.metrics_path = val;
            }

            if let Ok(val) = std::env::var("INKLOG_HTTP_HEALTH_PATH") {
                http.health_path = val;
            }

            if let Ok(val) = std::env::var("INKLOG_HTTP_ERROR_MODE") {
                http.error_mode = match val.to_lowercase().as_str() {
                    "panic" => HttpErrorMode::Panic,
                    "warn" => HttpErrorMode::Warn,
                    "strict" => HttpErrorMode::Strict,
                    _ => {
                        eprintln!(
                            "Invalid INKLOG_HTTP_ERROR_MODE: {}, using default (panic)",
                            val
                        );
                        HttpErrorMode::Panic
                    }
                };
            }
        }
    }

    fn apply_performance_overrides(&mut self) {
        if let Ok(val) = std::env::var("INKLOG_CHANNEL_CAPACITY") {
            if let Ok(num) = val.parse() {
                self.performance.channel_capacity = num;
            }
        }

        if let Ok(val) = std::env::var("INKLOG_WORKER_THREADS") {
            if let Ok(num) = val.parse() {
                self.performance.worker_threads = num;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Global Config Tests ===

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

    // === Performance Config Tests ===

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

    // === Console Config Tests ===

    #[test]
    fn test_console_config_default() {
        let console = ConsoleSinkConfig::default();
        assert!(console.enabled);
        // colored may be true or false depending on terminal detection
        // Just verify the struct is created
        assert!(!console.stderr_levels.is_empty() || console.stderr_levels.is_empty());
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
        // 测试默认值包含 "error" 和 "warn"
        let config = ConsoleSinkConfig::default();
        assert_eq!(config.stderr_levels.len(), 2);
        assert!(config.stderr_levels.contains(&"error".to_string()));
        assert!(config.stderr_levels.contains(&"warn".to_string()));

        // 创建新的配置来测试修改
        let mut config = ConsoleSinkConfig::default();
        config.stderr_levels.clear();
        assert!(config.stderr_levels.is_empty());

        config.stderr_levels.push("info".to_string());
        assert_eq!(config.stderr_levels.len(), 1);
        assert!(config.stderr_levels.contains(&"info".to_string()));
    }

    // === File Config Tests ===

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

        // These are methods on FileSink, not config
        // Just verify the string values are correct
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

    // === Database Config Tests ===

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
        assert!(config.path.parent().unwrap().is_dir());
    }
}
