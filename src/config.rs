// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::archive::SecretString;
use crate::config_validator::{validate_log_level, validate_non_empty, validate_path, validate_positive, validate_url};
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
            s3_archive: None,
            performance: PerformanceConfig::default(),
            http_server: None,
        }
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
        if self.s3_archive.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("s3_archive");
        }
        sinks
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_masking_enabled")]
    pub masking_enabled: bool,
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

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            format: default_format(),
            masking_enabled: default_masking_enabled(),
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
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    pub url: String,
    pub pool_size: u32,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
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
            enabled: false,
            driver: DatabaseDriver::PostgreSQL,
            url: "postgres://localhost/logs".to_string(),
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 500,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceConfig {
    pub channel_capacity: usize,
    pub worker_threads: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 10000,
            worker_threads: 3,
        }
    }
}

impl InklogConfig {
    #[cfg(feature = "confers")]
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, InklogError> {
        let content = fs::read_to_string(path)?;
        let mut config: InklogConfig = toml::from_str(&content)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    #[cfg(feature = "confers")]
    pub fn load() -> Result<Self, InklogError> {
        // Try common locations
        let locations = [
            "/etc/inklog/config.toml",
            "~/.config/inklog/config.toml",
            "./inklog_config.toml",
            "./config.toml",
        ];

        for loc in locations.iter() {
            // Basic path expansion (simplistic for now)
            let path = if loc.starts_with("~") {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(loc.replace("~", &home))
                } else {
                    PathBuf::from(loc)
                }
            } else {
                PathBuf::from(loc)
            };

            if path.exists() {
                return Self::from_file(path);
            }
        }

        // If no file found, load from default
        let mut config = Self::default();
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    #[cfg(feature = "confers")]
    pub fn load_with_watch(
    ) -> Result<(Self, PathBuf, tokio::sync::mpsc::Receiver<PathBuf>), InklogError> {
        use tokio::sync::mpsc;

        let locations = [
            "/etc/inklog/config.toml",
            "~/.config/inklog/config.toml",
            "./inklog_config.toml",
            "./config.toml",
        ];

        let mut config_path: Option<PathBuf> = None;

        for loc in locations.iter() {
            let path = if loc.starts_with("~") {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(loc.replace("~", &home))
                } else {
                    PathBuf::from(loc)
                }
            } else {
                PathBuf::from(loc)
            };

            if path.exists() {
                config_path = Some(path);
                break;
            }
        }

        let config_path = match config_path {
            Some(path) => path,
            None => {
                return Err(InklogError::ConfigError(
                    "No config file found for watching".to_string(),
                ));
            }
        };

        let config = Self::from_file(&config_path)?;

        let (tx, rx) = mpsc::channel(1);
        let watch_path = config_path.clone();

        tokio::spawn(async move {
            let mut last_modified = std::fs::metadata(&watch_path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                if let Ok(metadata) = std::fs::metadata(&watch_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified {
                            last_modified = modified;
                            let _ = tx.send(watch_path.clone()).await;
                        }
                    }
                }
            }
        });

        Ok((config, config_path, rx))
    }

    pub fn validate(&self) -> Result<(), InklogError> {
        use crate::config_validator::{validate_log_level, validate_non_empty, validate_path, validate_positive};

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

    #[test]
    fn test_apply_env_overrides_level() {
        std::env::set_var("INKLOG_LEVEL", "debug");

        let mut config = InklogConfig::default();
        assert_eq!(config.global.level, "info");
        config.apply_env_overrides();
        assert_eq!(config.global.level, "debug");

        std::env::remove_var("INKLOG_LEVEL");
    }

    #[test]
    fn test_apply_env_overrides_file_path() {
        std::env::set_var("INKLOG_FILE_PATH", "/custom/path/app.log");

        let mut config = InklogConfig {
            file_sink: Some(FileSinkConfig::default()),
            ..Default::default()
        };
        config.apply_env_overrides();
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            PathBuf::from("/custom/path/app.log")
        );

        std::env::remove_var("INKLOG_FILE_PATH");
    }

    #[test]
    fn test_apply_env_overrides_file_enabled() {
        std::env::set_var("INKLOG_FILE_ENABLED", "false");

        let mut config = InklogConfig {
            file_sink: Some(FileSinkConfig::default()),
            ..Default::default()
        };
        assert!(config.file_sink.as_ref().unwrap().enabled);
        config.apply_env_overrides();
        assert!(!config.file_sink.as_ref().unwrap().enabled);

        std::env::remove_var("INKLOG_FILE_ENABLED");
    }

    #[test]
    fn test_apply_env_overrides_performance() {
        std::env::set_var("INKLOG_CHANNEL_CAPACITY", "5000");
        std::env::set_var("INKLOG_WORKER_THREADS", "4");

        let mut config = InklogConfig::default();
        assert_eq!(config.performance.channel_capacity, 10000);
        assert_eq!(config.performance.worker_threads, 3);
        config.apply_env_overrides();
        assert_eq!(config.performance.channel_capacity, 5000);
        assert_eq!(config.performance.worker_threads, 4);

        std::env::remove_var("INKLOG_CHANNEL_CAPACITY");
        std::env::remove_var("INKLOG_WORKER_THREADS");
    }

    #[test]
    fn test_apply_env_overrides_db_url() {
        std::env::set_var("INKLOG_DB_URL", "postgres://user:pass@localhost/logs");

        let mut config = InklogConfig {
            database_sink: Some(DatabaseSinkConfig::default()),
            ..Default::default()
        };
        config.apply_env_overrides();
        assert_eq!(
            config.database_sink.as_ref().unwrap().url,
            "postgres://user:pass@localhost/logs"
        );

        std::env::remove_var("INKLOG_DB_URL");
    }
}
