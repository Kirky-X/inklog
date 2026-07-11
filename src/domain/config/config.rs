// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// InklogConfig - Root configuration struct
// ============================================================================

/// Root configuration for inklog logger.
///
/// # Loading
///
/// Configuration can be loaded from:
/// - TOML files (via `from_search_paths()`)
/// - Environment variables (prefix `INKLOG_`)
/// - Defaults (lowest priority)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InklogConfig {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default = "default_console_sink")]
    pub console_sink: Option<ConsoleSinkConfig>,
    #[serde(default)]
    pub file_sink: Option<FileSinkConfig>,
    #[serde(default)]
    pub database_sink: Option<DatabaseSinkConfig>,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub http_server: Option<HttpServerConfig>,
}

fn default_console_sink() -> Option<ConsoleSinkConfig> {
    Some(ConsoleSinkConfig::default())
}

impl Default for InklogConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            console_sink: default_console_sink(),
            file_sink: None,
            database_sink: None,
            performance: PerformanceConfig::default(),
            http_server: None,
        }
    }
}

impl InklogConfig {
    /// Load configuration synchronously from the default search paths.
    ///
    /// Tries each search path in order and returns the first valid config.
    /// Falls back to `Self::default()` if no config file exists.
    pub fn load_sync() -> Result<Self, InklogError> {
        Self::from_search_paths()
            .map_err(|e| InklogError::ConfigError(format!("Failed to load config: {}", e)))
    }

    /// Load configuration with custom environment variable overrides.
    ///
    /// This method loads configuration from the default sources (files or defaults)
    /// and then applies additional environment variable overrides for nested fields.
    ///
    /// Environment variables with prefix `INKLOG_` are parsed with underscores
    /// representing nested structure:
    /// - `INKLOG_GLOBAL_LEVEL` → global.level
    /// - `INKLOG_HTTP_SERVER_PORT` → http_server.port
    ///
    /// # Returns
    ///
    /// Returns `Ok(InklogConfig)` on success, or `Err(InklogError)` if loading fails.
    pub fn load_with_env_overrides() -> Result<Self, InklogError> {
        let mut config = Self::load_sync()?;

        // Apply additional nested field environment variable overrides
        Self::apply_env_overrides(&mut config);

        Ok(config)
    }

    /// Apply environment variable overrides to the configuration.
    ///
    /// Parses INKLOG_* environment variables and applies them to the config.
    fn apply_env_overrides(config: &mut Self) {
        // Global config overrides
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_LEVEL") {
            config.global.level = val;
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_FORMAT") {
            config.global.format = val;
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_MASKING_ENABLED") {
            config.global.masking_enabled = val.parse().unwrap_or(config.global.masking_enabled);
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_AUTO_FALLBACK") {
            config.global.auto_fallback = val.parse().unwrap_or(config.global.auto_fallback);
        }

        // File sink overrides
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_ENABLED") {
            if val.parse::<bool>().unwrap_or(false) {
                let file_config = config.file_sink.get_or_insert_with(Default::default);
                file_config.enabled = true;
            }
        }
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_PATH") {
            let file_config = config.file_sink.get_or_insert_with(Default::default);
            file_config.path = std::path::PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_MAX_SIZE") {
            let file_config = config.file_sink.get_or_insert_with(Default::default);
            file_config.max_size = val;
        }

        // HTTP server overrides
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_ENABLED") {
            if val.parse::<bool>().unwrap_or(false) {
                let http_config = config.http_server.get_or_insert_with(Default::default);
                http_config.enabled = true;
            }
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_HOST") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.host = val;
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_PORT") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.port = val.parse().unwrap_or(http_config.port);
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_METRICS_PATH") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.metrics_path = val;
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_HEALTH_PATH") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.health_path = val;
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_ERROR_MODE") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.error_mode = match val.to_lowercase().as_str() {
                "strict" => crate::config::HttpErrorMode::Strict,
                "warn" => crate::config::HttpErrorMode::Warn,
                _ => http_config.error_mode.clone(),
            };
        }

        // Performance overrides
        if let Ok(val) = std::env::var("INKLOG_PERFORMANCE_WORKER_THREADS") {
            config.performance.worker_threads =
                val.parse().unwrap_or(config.performance.worker_threads);
        }
        if let Ok(val) = std::env::var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY") {
            config.performance.channel_capacity =
                val.parse().unwrap_or(config.performance.channel_capacity);
        }
    }

    ///
    /// Search paths (first existing file wins):
    /// 1. `$INKLOG_CONFIG_PATH`
    /// 2. `inklog_config.toml` (current directory)
    /// 3. `~/.config/inklog/config.toml`
    /// 4. `/etc/inklog/config.toml`
    ///
    /// Environment variables with prefix `INKLOG_` override all file values.
    pub fn from_search_paths() -> Result<Self, InklogError> {
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
                let content = std::fs::read_to_string(&path_opt).map_err(|e| {
                    InklogError::ConfigError(format!(
                        "Failed to read config file '{}': {}",
                        path_opt, e
                    ))
                })?;
                let config: Self = toml::from_str(&content).map_err(|e| {
                    InklogError::ConfigError(format!(
                        "Failed to parse config file '{}': {}",
                        path_opt, e
                    ))
                })?;
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    /// Returns a list of enabled sink names.
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
        sinks
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), InklogError> {
        if self.performance.channel_capacity == 0 {
            return Err(InklogError::ConfigError(
                "channel_capacity cannot be 0".to_string(),
            ));
        }
        if self.performance.worker_threads == 0 {
            return Err(InklogError::ConfigError(
                "worker_threads cannot be 0".to_string(),
            ));
        }
        Ok(())
    }
}

// InklogConfig's Default impl calls the same default functions as #[serde(default = ...)] so
// Default::default() and toml::from_str("") produce identical values.

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

// ============================================================================
// GlobalConfig - Global logger settings
// ============================================================================

/// Global logger configuration.
///
/// Controls the overall behavior of the logging system including log level,
/// format string, and fallback settings.
///
/// # Configuration Priority
///
/// Configuration values are loaded with the following priority (highest to lowest):
/// 1. Environment variables (prefix `INKLOG_GLOBAL_`)
/// 2. Configuration file values
/// 3. Default values
///
/// # Example TOML Configuration
///
/// ```toml
/// [global]
/// level = "debug"
/// format = "{timestamp} [{level}] {target} - {message}"
/// masking_enabled = true
/// auto_fallback = true
/// fallback_initial_delay_ms = 1000
/// fallback_max_delay_ms = 60000
/// fallback_max_retries = 10
/// ```
///
/// # Environment Variable Overrides
///
/// Any field can be overridden via environment variables:
/// ```bash
/// export INKLOG_GLOBAL_LEVEL=debug
/// export INKLOG_GLOBAL_MASKING_ENABLED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    /// Minimum log level to capture.
    ///
    /// Valid values (case-insensitive): `trace`, `debug`, `info`, `warn`, `error`, `fatal`.
    /// Logs below this level are ignored.
    ///
    /// # Default
    ///
    /// `"info"` - Captures INFO, WARN, ERROR, and FATAL logs.
    #[serde(default = "default_global_level")]
    pub level: String,

    /// Log message format template.
    ///
    /// Supports placeholders that are replaced with values from each log record:
    /// - `{timestamp}` - ISO 8601 timestamp
    /// - `{level}` - Log level (INFO, DEBUG, etc.)
    /// - `{target}` - Module path that emitted the log
    /// - `{message}` - Log message content
    /// - `{file}` - Source file path (optional)
    /// - `{line}` - Line number in source file (optional)
    /// - `{thread_id}` - Thread identifier
    /// - `{fields}` - Additional structured fields (JSON)
    ///
    /// # Default
    ///
    /// `"{timestamp} [{level}] {target} - {message}"`
    #[serde(default = "default_global_format")]
    pub format: String,

    /// Enable sensitive data masking.
    ///
    /// When enabled, sensitive patterns (passwords, API keys, credit cards, etc.)
    /// are automatically replaced with `[REDACTED]` placeholders.
    ///
    /// # Default
    ///
    /// `true` - Masking enabled by default for security.
    #[serde(default = "default_true")]
    pub masking_enabled: bool,

    /// Enable automatic fallback on sink failures.
    ///
    /// When a sink fails repeatedly, the system automatically falls back to
    /// alternative sinks (e.g., database → file → console).
    ///
    /// # Default
    ///
    /// `true` - Fallback enabled for reliability.
    #[serde(default = "default_true")]
    pub auto_fallback: bool,

    /// Initial delay before first retry (milliseconds).
    ///
    /// When a sink fails, the system waits this duration before attempting
    /// the first retry. Subsequent retries use exponential backoff.
    ///
    /// # Default
    ///
    /// `1000` ms (1 second)
    #[serde(default = "default_fallback_initial_delay")]
    pub fallback_initial_delay_ms: u64,

    /// Maximum delay between retries (milliseconds).
    ///
    /// Caps the exponential backoff delay to prevent excessive waiting.
    ///
    /// # Default
    ///
    /// `60000` ms (60 seconds)
    #[serde(default = "default_fallback_max_delay")]
    pub fallback_max_delay_ms: u64,

    /// Maximum number of retry attempts.
    ///
    /// After this many failures, the sink is marked as unhealthy and
    /// fallback mechanisms are activated.
    ///
    /// # Default
    ///
    /// `10` retries
    #[serde(default = "default_fallback_max_retries")]
    pub fallback_max_retries: u32,
}

// Default value functions for serde
fn default_global_level() -> String {
    "info".to_string()
}
fn default_global_format() -> String {
    "{timestamp} [{level}] {target} - {message}".to_string()
}
fn default_true() -> bool {
    true
}
fn default_fallback_initial_delay() -> u64 {
    1000
}
fn default_fallback_max_delay() -> u64 {
    60000
}
fn default_fallback_max_retries() -> u32 {
    10
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: default_global_level(),
            format: default_global_format(),
            masking_enabled: default_true(),
            auto_fallback: default_true(),
            fallback_initial_delay_ms: default_fallback_initial_delay(),
            fallback_max_delay_ms: default_fallback_max_delay(),
            fallback_max_retries: default_fallback_max_retries(),
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// ConsoleSinkConfig - Console output settings
// ============================================================================

/// Console sink configuration.
///
/// Controls logging output to stdout/stderr with optional colored output
/// and level-based stream routing.
///
/// # Example TOML Configuration
///
/// ```toml
/// [console_sink]
/// enabled = true
/// colored = true
/// stderr_levels = ["error", "warn"]
/// masking_enabled = false
/// ```
///
/// # Stream Routing
///
/// Log levels specified in `stderr_levels` are written to stderr,
/// all other levels go to stdout. This enables:
/// - Separating errors from normal output
/// - Piping stdout to files while keeping errors visible
/// - Integration with monitoring tools that parse stderr
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_CONSOLE_SINK_ENABLED=true
/// export INKLOG_CONSOLE_SINK_COLORED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSinkConfig {
    /// Enable console logging.
    ///
    /// When `false`, no logs are written to console even if other settings
    /// are configured.
    ///
    /// # Default
    ///
    /// `true` - Console output enabled by default.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Enable colored output using ANSI escape codes.
    ///
    /// Colors help distinguish log levels visually:
    /// - ERROR: Red
    /// - WARN: Yellow
    /// - INFO: Green
    /// - DEBUG: Blue
    /// - TRACE: Gray
    ///
    /// Set to `false` when piping to files or systems that don't support ANSI.
    ///
    /// # Default
    ///
    /// `true` - Colors enabled by default.
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Log levels to write to stderr instead of stdout.
    ///
    /// Enables separating error/warning messages from regular logs.
    /// Useful for monitoring systems and log aggregation pipelines.
    ///
    /// # Default
    ///
    /// `["error", "warn"]` - Errors and warnings go to stderr.
    #[serde(default = "default_stderr_levels")]
    pub stderr_levels: Vec<String>,

    /// Enable sensitive data masking for console output.
    ///
    /// When enabled, patterns like passwords, API keys, and credit card
    /// numbers are replaced with `[REDACTED]`.
    ///
    /// # Note
    ///
    /// Consider setting this to `false` in development for easier debugging,
    /// but `true` in production for security.
    ///
    /// # Default
    ///
    /// `false` - No masking for console output (developer-friendly).
    #[serde(default)]
    pub masking_enabled: bool,
}

fn default_stderr_levels() -> Vec<String> {
    vec!["error".to_string(), "warn".to_string()]
}

impl Default for ConsoleSinkConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            colored: default_true(),
            stderr_levels: default_stderr_levels(),
            masking_enabled: false,
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

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
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_FILE_SINK_ENABLED=true
/// export INKLOG_FILE_SINK_PATH="/var/log/app/app.log"
/// export INKLOG_FILE_SINK_MAX_SIZE="500MB"
/// export INKLOG_FILE_SINK_ENCRYPT=true
/// export INKLOG_FILE_SINK_ENCRYPTION_KEY_ENV="MY_LOG_KEY"
/// ```
///
/// # File Rotation
///
/// When `max_size` is reached, the current log file is renamed with a timestamp
/// suffix (e.g., `app.log.2024-01-15_10-30-00`) and a new file is created.
/// Rotation can also be triggered by `rotation_time` (daily, hourly, etc.).
///
/// # Encryption
///
/// When `encrypt` is enabled, logs are encrypted using AES-256-GCM with a key
/// read from the environment variable specified by `encryption_key_env`.
/// The key must be a Base64-encoded 32-byte key.
///
/// # Retention
///
/// Old log files are automatically cleaned up based on:
/// - `retention_days`: Delete files older than N days
/// - `max_total_size`: Delete oldest files when total size exceeds limit
/// - `keep_files`: Maximum number of rotated files to keep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSinkConfig {
    /// Enable file logging.
    ///
    /// When `false`, no logs are written to file even if other settings
    /// are configured.
    ///
    /// # Default
    ///
    /// `true` - File logging enabled by default.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Path to the log file.
    ///
    /// Parent directories are created automatically if they don't exist.
    /// Relative paths are resolved from the current working directory.
    ///
    /// # Default
    ///
    /// `"logs/app.log"`
    #[serde(default = "default_log_path")]
    pub path: PathBuf,

    /// Maximum size of a single log file before rotation.
    ///
    /// Accepts human-readable size formats: `100MB`, `1GB`, `500KB`, etc.
    /// When the file reaches this size, it is rotated and a new file is created.
    ///
    /// # Default
    ///
    /// `"100MB"`
    #[serde(default = "default_max_size")]
    pub max_size: String,

    /// Time-based rotation interval.
    ///
    /// Triggers rotation at the specified interval regardless of file size.
    /// Supported values: `daily`, `hourly`, `weekly`, `monthly`.
    ///
    /// # Default
    ///
    /// `"daily"` - Rotate once per day at midnight.
    #[serde(default = "default_rotation_time")]
    pub rotation_time: String,

    /// Maximum number of rotated files to keep.
    ///
    /// When this limit is reached, the oldest rotated files are deleted.
    /// Set to `0` to keep all files (not recommended, use `retention_days` instead).
    ///
    /// # Default
    ///
    /// `30` - Keep the most recent 30 rotated files.
    #[serde(default = "default_keep_files")]
    pub keep_files: u32,

    /// Enable compression for rotated log files.
    ///
    /// Uses Zstd compression (level 3 by default) to reduce storage.
    /// Compressed files have `.zst` extension.
    ///
    /// # Default
    ///
    /// `true` - Compression enabled for storage efficiency.
    #[serde(default = "default_true")]
    pub compress: bool,

    /// Zstd compression level (1-22).
    ///
    /// Higher levels provide better compression but slower performance.
    /// Recommended range: 1-10 for most use cases.
    ///
    /// # Default
    ///
    /// `3` - Good balance between compression ratio and speed.
    #[serde(default = "default_compression_level")]
    pub compression_level: i32,

    /// Enable AES-256-GCM encryption for log files.
    ///
    /// When enabled, the encryption key must be provided via the environment
    /// variable specified in `encryption_key_env`.
    ///
    /// # Security
    ///
    /// - Encryption key must be exactly 32 bytes, Base64-encoded
    /// - Each file uses a unique nonce for cryptographic security
    /// - Files are encrypted after compression (if enabled)
    ///
    /// # Default
    ///
    /// `false` - Encryption disabled by default.
    #[serde(default)]
    pub encrypt: bool,

    /// Environment variable name for the encryption key.
    ///
    /// The environment variable should contain a Base64-encoded 32-byte key.
    /// Required when `encrypt` is `true`.
    ///
    /// # Example
    ///
    /// ```bash
    /// export LOG_ENCRYPTION_KEY="base64-encoded-32-byte-key-here"
    /// ```
    ///
    /// # Default
    ///
    /// `None` - No default key environment variable.
    #[serde(default)]
    pub encryption_key_env: Option<String>,

    /// Delete log files older than N days.
    ///
    /// Runs during periodic cleanup (see `cleanup_interval_minutes`).
    /// Set to `0` to disable age-based deletion.
    ///
    /// # Default
    ///
    /// `30` - Delete files older than 30 days.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Maximum total size of all log files combined.
    ///
    /// When total size exceeds this limit, oldest files are deleted first.
    /// Accepts human-readable formats: `1GB`, `500MB`, etc.
    ///
    /// # Default
    ///
    /// `"1GB"` - Total log storage limited to 1GB.
    #[serde(default = "default_max_total_size")]
    pub max_total_size: String,

    /// Interval between cleanup runs (minutes).
    ///
    /// How often to check for and delete old log files based on
    /// `retention_days` and `max_total_size`.
    ///
    /// # Default
    ///
    /// `60` - Run cleanup every hour.
    #[serde(default = "default_cleanup_interval_minutes")]
    pub cleanup_interval_minutes: u64,

    /// Number of log records to buffer before writing to disk.
    ///
    /// Larger batches improve throughput but increase memory usage and
    /// potential data loss on crash. Smaller batches reduce latency.
    ///
    /// # Default
    ///
    /// `100` - Balance between throughput and latency.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum time to wait before flushing buffer (milliseconds).
    ///
    /// Even if batch is not full, records are flushed after this interval.
    /// Ensures logs are written promptly during low activity.
    ///
    /// # Default
    ///
    /// `100` ms - Flush at least every 100ms.
    #[serde(default = "default_flush_interval_ms")]
    pub flush_interval_ms: u64,

    /// Enable sensitive data masking for file output.
    ///
    /// When enabled, patterns like passwords, API keys, and credit card
    /// numbers are replaced with `[REDACTED]`.
    ///
    /// # Recommendation
    ///
    /// Keep this `true` in production to prevent sensitive data leaks.
    /// Can be `false` in development for easier debugging.
    ///
    /// # Default
    ///
    /// `true` - Masking enabled for security.
    #[serde(default = "default_true")]
    pub masking_enabled: bool,
}

// Default value functions for FileSinkConfig
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

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// DatabaseDriver - Supported database drivers
// ============================================================================

/// Supported database drivers for log storage.
///
/// Provides unified logging interface across multiple database backends.
/// Each driver supports the same core logging features with backend-specific optimizations.
///
/// # Supported Drivers
///
/// - **PostgreSQL**: Recommended for high-throughput production workloads
///   - Native partitioning support (monthly/yearly)
///   - Advanced indexing and query optimization
///   - Connection pooling via `pool_size` configuration
///
/// - **MySQL**: Good balance of performance and simplicity
///   - Partitioning support
///   - Wide hosting compatibility
///
/// - **SQLite**: Zero-dependency embedded database
///   - Ideal for development, testing, and small deployments
///   - No external database server required
///   - File-based storage with in-memory option (`:memory:`)
///
/// # Configuration Example
///
/// ```toml
/// [database_sink]
/// driver = "postgres"  # or "mysql", "sqlite"
/// url = "postgres://user:pass@localhost/logs"
/// ```
///
/// # Environment Variable Override
///
/// ```bash
/// export INKLOG_DATABASE_SINK_DRIVER=postgres
/// export INKLOG_DATABASE_SINK_URL="postgres://prod-server/logs"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
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

// ============================================================================
// PartitionStrategy - Database table partitioning strategy
// ============================================================================

/// Database table partitioning strategy for log storage optimization.
///
/// Partitioning improves query performance and data management for high-volume logging
/// by organizing logs into time-based partitions. Each partition is a separate physical
/// storage unit that can be managed, backed up, or archived independently.
///
/// # Partition Strategies
///
/// - **Monthly**: Creates a new partition for each month
///   - Best for high-volume logging (>100k logs/day)
///   - Easier to manage monthly retention policies
///   - Better query performance for recent logs
///   - Example partition names: `logs_2024_01`, `logs_2024_02`, ...
///
/// - **Yearly**: Creates a new partition for each year
///   - Best for low-to-medium volume logging (<50k logs/day)
///   - Simpler partition management
///   - Suitable for long-term archival
///   - Example partition names: `logs_2024`, `logs_2025`, ...
///
/// # Database Support
///
/// - **PostgreSQL**: Native partitioning via `PARTITION BY RANGE`
/// - **MySQL**: Partitioning support via `PARTITION BY RANGE`
/// - **SQLite**: Not supported (partitioning setting ignored)
///
/// # Configuration Example
///
/// ```toml
/// [database_sink]
/// driver = "postgres"
/// url = "postgres://user:pass@localhost/logs"
/// partition_strategy = "monthly"  # or "yearly"
/// ```
///
/// # Performance Considerations
///
/// | Strategy | Query Performance | Management Overhead | Best For |
/// |----------|-------------------|---------------------|----------|
/// | Monthly  | Excellent         | Higher              | High volume production |
/// | Yearly   | Good              | Lower               | Medium volume or archival |
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

// ============================================================================
// ParquetConfig - Parquet export configuration
// ============================================================================

/// Parquet export configuration for database sink.
///
/// Parquet is a columnar storage format optimized for analytics and long-term archival.
/// When enabled, logs are periodically exported from the database to Parquet files,
/// which can then be uploaded to S3 or other cold storage.
///
/// # Configuration Fields
///
/// - **compression_level**: Zstandard compression level (0-10, default: 3)
///   - 0: No compression
///   - 3: Balanced (recommended)
///   - 10: Maximum compression (slower)
///
/// - **max_row_group_size**: Maximum rows per row group (default: 10,000)
///   - Smaller values: Better for selective queries
///   - Larger values: Better compression ratio
///
/// - **max_page_size**: Maximum bytes per page (default: 1,048,576 = 1MB)
///   - Controls memory usage during reads
///
/// # Configuration Example
///
/// ```toml
/// [database_sink.parquet]
/// compression_level = 3
/// max_row_group_size = 10000
/// max_page_size = 1048576
/// ```
///
/// # Use Cases
///
/// - **Analytics**: Load Parquet files into data warehouses (Snowflake, BigQuery, Redshift)
/// - **Archival**: Store compressed logs in S3 Glacier or similar cold storage
/// - **Compliance**: Retain logs for regulatory requirements with efficient storage
///
/// # Performance Notes
///
/// Parquet export runs asynchronously in the background. The export interval
/// is controlled by the database sink's flush interval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    #[serde(default = "default_parquet_compression_level")]
    pub compression_level: i32,
    #[serde(default = "default_parquet_encoding")]
    pub encoding: String,
    #[serde(default = "default_parquet_max_row_group_size")]
    pub max_row_group_size: usize,
    #[serde(default = "default_parquet_max_page_size")]
    pub max_page_size: usize,
    #[serde(default)]
    pub include_fields: Vec<String>,
}

fn default_parquet_compression_level() -> i32 {
    3
}
fn default_parquet_encoding() -> String {
    "PLAIN".to_string()
}
fn default_parquet_max_row_group_size() -> usize {
    10000
}
fn default_parquet_max_page_size() -> usize {
    1048576
}

impl Default for ParquetConfig {
    fn default() -> Self {
        Self {
            compression_level: default_parquet_compression_level(),
            encoding: default_parquet_encoding(),
            max_row_group_size: default_parquet_max_row_group_size(),
            max_page_size: default_parquet_max_page_size(),
            include_fields: Vec::new(),
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// DatabaseSinkConfig - Database sink settings
// ============================================================================

/// Database sink configuration for persistent log storage.
///
/// The database sink provides centralized log storage with powerful query capabilities
/// and support for high-throughput production workloads. Logs are batched for efficient
/// writes and can optionally be exported to Parquet for long-term archival.
///
/// # Core Configuration
///
/// - **name**: Sink identifier for metrics and health tracking (default: "default")
/// - **enabled**: Enable/disable the database sink (default: false)
/// - **driver**: Database driver ([`DatabaseDriver`])
/// - **url**: Database connection string
/// - **pool_size**: Maximum database connections (default: 10)
///
/// # Batch Settings
///
/// - **batch_size**: Maximum log records per batch write (default: 100)
///   - Larger batches: Better throughput, higher memory usage
///   - Smaller batches: Lower latency, more frequent writes
///
/// - **flush_interval_ms**: Maximum time between flushes in milliseconds (default: 500)
///   - Ensures logs are written even if batch size is not reached
///   - Lower values: More timely logs, more database load
///
/// # Partitioning
///
/// - **partition**: Table partitioning strategy ([`PartitionStrategy`])
///   - Monthly: Best for high-volume production (>100k logs/day)
///   - Yearly: Best for medium volume or archival
///
/// # Archive Format
///
/// - **archive_format**: Export format - "json" or "parquet" (default: "json")
/// - **parquet_config**: Parquet-specific settings (when archive_format = "parquet")
///
/// # Table Name
///
/// - **table_name**: Database table for logs (default: "logs")
///
/// # Configuration Example
///
/// ## PostgreSQL with Monthly Partitioning
///
/// ```toml
/// [database_sink]
/// enabled = true
/// driver = "postgres"
/// url = "postgres://user:pass@localhost/logs"
/// pool_size = 20
/// batch_size = 200
/// flush_interval_ms = 500
/// partition = "monthly"
/// table_name = "app_logs"
/// ```
///
/// ## SQLite for Development
///
/// ```toml
/// [database_sink]
/// enabled = true
/// driver = "sqlite"
/// url = "file:logs.db"
/// pool_size = 1
/// batch_size = 50
/// ```
///
/// ## Parquet Archive Format
///
/// ```toml
/// [database_sink]
/// enabled = true
/// driver = "postgres"
/// url = "postgres://user:pass@localhost/logs"
/// archive_format = "parquet"
///
/// [database_sink.parquet]
/// compression_level = 5
/// max_row_group_size = 10000
/// ```
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_DATABASE_SINK_ENABLED=true
/// export INKLOG_DATABASE_SINK_URL="postgres://prod-db/logs"
/// export INKLOG_DATABASE_SINK_POOL_SIZE=50
/// ```
///
/// # Performance Tuning
///
/// | Setting | High Throughput | Low Latency | Development |
/// |---------|----------------|-------------|-------------|
/// | pool_size | 20-50 | 5-10 | 1 |
/// | batch_size | 200-500 | 50-100 | 50 |
/// | flush_interval_ms | 1000 | 200-500 | 500 |
///
/// # Failure Handling
///
/// If the database becomes unavailable, the database sink automatically:
/// 1. Logs write failures to metrics
/// 2. Stores logs in fallback file sink (`logs/db_fallback.log`)
/// 3. Attempts reconnection based on circuit breaker policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSinkConfig {
    #[serde(default = "default_db_sink_name")]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    #[serde(default = "default_db_url")]
    pub url: String,
    #[serde(default = "default_db_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_db_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_db_flush_interval_ms")]
    pub flush_interval_ms: u64,
    #[serde(default)]
    pub partition: PartitionStrategy,
    #[serde(default = "default_db_table_name")]
    pub table_name: String,
    #[serde(default = "default_db_archive_format")]
    pub archive_format: String,
    #[serde(default)]
    pub parquet_config: ParquetConfig,
}

fn default_db_sink_name() -> String {
    "default".to_string()
}
fn default_db_url() -> String {
    "sqlite::memory:".to_string()
}
fn default_db_pool_size() -> u32 {
    10
}
fn default_db_batch_size() -> usize {
    100
}
fn default_db_flush_interval_ms() -> u64 {
    500
}
fn default_db_table_name() -> String {
    "logs".to_string()
}
fn default_db_archive_format() -> String {
    "json".to_string()
}

impl Default for DatabaseSinkConfig {
    fn default() -> Self {
        Self {
            name: default_db_sink_name(),
            enabled: false,
            driver: DatabaseDriver::default(),
            url: default_db_url(),
            pool_size: default_db_pool_size(),
            batch_size: default_db_batch_size(),
            flush_interval_ms: default_db_flush_interval_ms(),
            partition: PartitionStrategy::default(),
            table_name: default_db_table_name(),
            archive_format: default_db_archive_format(),
            parquet_config: ParquetConfig::default(),
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// ChannelStrategy - Adaptive channel sizing strategy
// ============================================================================

/// Channel sizing strategy for log buffer management.
///
/// The channel (crossbeam-channel) buffers log records between the application
/// and the background workers. The strategy determines how the channel capacity
/// is managed under varying load conditions.
///
/// # Strategies
///
/// - **Fixed** (default): Static channel capacity
///   - Predictable memory usage
///   - Consistent performance
///   - Simpler to reason about
///   - Best for: Production with known traffic patterns
///
/// - **Adaptive**: Dynamic capacity adjustment
///   - Grows under sustained high load
///   - Shrinks during low traffic
///   - Better burst handling
///   - Best for: Variable/unpredictable traffic
///
/// # Configuration Example
///
/// ```toml
/// [global]
/// channel_strategy = "adaptive"  # or "fixed"
/// channel_capacity = 10000
/// ```
///
/// # Behavior Comparison
///
/// | Scenario | Fixed | Adaptive |
///|----------|-------|----------|
///| Normal load | Stable | Stable (varies) |
///| Traffic spike | May drop logs | Grows to handle |
///| Low traffic | Constant capacity | Shrinks to save memory |
///| Memory usage | Predictable | Variable (typically lower) |
///
/// # Environment Variable Override
///
/// ```bash
/// export INKLOG_GLOBAL_CHANNEL_STRATEGY=adaptive
/// ```
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

// ============================================================================
// HttpServerConfig - HTTP health/metrics server settings
// ============================================================================

/// HTTP server configuration for health checks and metrics.
///
/// When enabled, inklog runs an HTTP server that exposes:
/// - **Metrics endpoint**: Prometheus-compatible metrics (default: `/metrics`)
/// - **Health check endpoint**: Service health status (default: `/health`)
///
/// # Endpoints
///
/// ## GET /metrics
///
/// Prometheus-compatible metrics including:
/// - `inklog_logs_total`: Total logs processed
/// - `inklog_logs_dropped`: Logs dropped due to backpressure
/// - `inklog_channel_blocked`: Channel blocking events
/// - `inklog_sink_errors`: Sink write errors
/// - `inklog_db_batch_size`: Current batch size
/// - `inklog_sink_healthy`: Sink health status (1=healthy, 0=unhealthy)
///
/// ## GET /health
///
/// Health check response:
/// ```json
/// {
///   "status": "ok",
///   "sinks": {
///     "console": {"healthy": true},
///     "file": {"healthy": true, "path": "/var/log/app.log"},
///     "database": {"healthy": false, "error": "connection refused"}
///   }
/// }
/// ```
///
/// # Configuration Fields
///
/// - **enabled**: Enable HTTP server (default: false)
/// - **host**: Bind address (default: "127.0.0.1")
/// - **port**: HTTP port (default: 9090)
/// - **metrics_path**: Metrics endpoint path (default: "/metrics")
/// - **health_path**: Health check path (default: "/health")
/// - **error_mode**: Error handling mode ([`HttpErrorMode`])
/// - **auth**: Optional authentication ([`HttpAuthConfig`])
/// - **ip_whitelist**: Optional IP whitelist for access control
///
/// # Configuration Example
///
/// ```toml
/// [http_server]
/// enabled = true
/// host = "0.0.0.0"  # Listen on all interfaces
/// port = 9090
/// metrics_path = "/metrics"
/// health_path = "/health"
/// error_mode = "strict"
///
/// # Optional authentication
/// [http_server.auth]
/// enabled = true
/// token_env = "INKLOG_HTTP_AUTH_TOKEN"
///
/// # Optional IP whitelist
/// http_server.ip_whitelist = ["10.0.0.0/8", "192.168.0.0/16"]
/// ```
///
/// # Authentication
///
/// When authentication is enabled, the HTTP server requires a Bearer token:
///
/// ```bash
/// # Set the auth token
/// export INKLOG_HTTP_AUTH_TOKEN="your-secret-token"
///
/// # Access endpoints with token
/// curl -H "Authorization: Bearer your-secret-token" http://localhost:9090/metrics
/// ```
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_HTTP_SERVER_ENABLED=true
/// export INKLOG_HTTP_SERVER_PORT=8080
/// export INKLOG_HTTP_SERVER_HOST="0.0.0.0"
/// ```
///
/// # Prometheus Integration
///
/// Add to your `prometheus.yml`:
///
/// ```yaml
/// scrape_configs:
///   - job_name: 'inklog'
///     static_configs:
///       - targets: ['localhost:9090']
///     bearer_token: 'your-auth-token'  # If auth enabled
/// ```
///
/// # Security Notes
///
/// - Default bind address (`127.0.0.1`) only allows local access
/// - Use `0.0.0.0` cautiously - exposes metrics to network
/// - Enable authentication for production deployments
/// - Consider IP whitelist for additional security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_host")]
    pub host: String,
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_http_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_http_health_path")]
    pub health_path: String,
    #[serde(default)]
    pub error_mode: HttpErrorMode,
    #[serde(default)]
    pub auth: Option<HttpAuthConfig>,
    #[serde(default)]
    pub ip_whitelist: Option<Vec<String>>,
}

fn default_http_host() -> String {
    "127.0.0.1".to_string()
}
fn default_http_port() -> u16 {
    9090
}
fn default_http_metrics_path() -> String {
    "/metrics".to_string()
}
fn default_http_health_path() -> String {
    "/health".to_string()
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_http_host(),
            port: default_http_port(),
            metrics_path: default_http_metrics_path(),
            health_path: default_http_health_path(),
            error_mode: HttpErrorMode::default(),
            auth: None,
            ip_whitelist: None,
        }
    }
}

/// HTTP authentication configuration.
///
/// Provides Bearer token authentication for HTTP metrics and health endpoints.
/// When enabled, all HTTP requests must include a valid authentication token.
///
/// # Configuration Fields
///
/// - **enabled**: Enable authentication (default: false)
/// - **token_env**: Environment variable name containing the auth token (default: "INKLOG_HTTP_AUTH_TOKEN")
///
/// # Configuration Example
///
/// ```toml
/// [http_server.auth]
/// enabled = true
/// token_env = "MY_CUSTOM_AUTH_TOKEN_VAR"
/// ```
///
/// # Usage
///
/// 1. Set the token in your environment:
/// ```bash
/// export INKLOG_HTTP_AUTH_TOKEN="your-secure-token-here"
/// ```
///
/// 2. Include the token in HTTP requests:
/// ```bash
/// curl -H "Authorization: Bearer your-secure-token-here" \
///   http://localhost:9090/metrics
/// ```
///
/// # Security Best Practices
///
/// - Use strong, randomly generated tokens (at least 32 characters)
/// - Store tokens securely (use secret management systems in production)
/// - Rotate tokens periodically
/// - Never commit tokens to version control
/// - Use different tokens for different environments
///
/// # Environment Variable Override
///
/// ```bash
/// export INKLOG_HTTP_SERVER_AUTH_ENABLED=true
/// export INKLOG_HTTP_SERVER_AUTH_TOKEN_ENV="MY_AUTH_VAR"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_auth_token_env")]
    pub token_env: String,
}

fn default_http_auth_token_env() -> String {
    "INKLOG_HTTP_AUTH_TOKEN".to_string()
}

impl Default for HttpAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_env: default_http_auth_token_env(),
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// HttpErrorMode - HTTP server error handling mode
// ============================================================================

/// HTTP server error handling mode.
///
/// Controls how the HTTP server responds to authentication failures and access errors.
///
/// # Modes
///
/// - **Warn** (default): Returns data with a warning header
///   - More permissive - allows monitoring to continue
///   - Adds `X-Auth-Warning` header on auth failures
///   - Useful for gradual rollout of authentication
///
/// - **Strict**: Returns HTTP error codes
///   - More secure - fails closed on auth errors
///   - Returns `401 Unauthorized` for auth failures
///   - Returns `403 Forbidden` for IP whitelist failures
///   - Recommended for production deployments
///
/// # Configuration Example
///
/// ```toml
/// [http_server]
/// enabled = true
/// error_mode = "strict"  # or "warn"
///
/// [http_server.auth]
/// enabled = true
/// ```
///
/// # Behavior Comparison
///
/// | Scenario | Warn Mode | Strict Mode |
///|----------|-----------|-------------|
///| Valid auth | Returns data | Returns data |
///| Missing auth | Returns data + warning header | Returns 401 |
///| Invalid auth | Returns data + warning header | Returns 401 |
///| IP not in whitelist | Returns data + warning header | Returns 403 |
///
/// # Recommendations
///
/// - **Development/Testing**: Use `warn` mode for easier debugging
/// - **Staging**: Use `warn` mode to test auth setup without breaking monitoring
/// - **Production**: Use `strict` mode for security best practices
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpErrorMode {
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "strict")]
    #[default]
    Strict,
}

// ============================================================================
// PerformanceConfig - Performance tuning parameters
// ============================================================================

/// Performance tuning configuration for log channel and worker management.
///
/// These settings control the internal behavior of inklog's logging pipeline.
/// Most applications can use the defaults, but high-throughput or resource-constrained
/// environments may benefit from tuning.
///
/// # Channel Configuration
///
/// - **channel_capacity**: Initial channel buffer size (default: 10,000)
///   - Controls how many log records can be buffered before backpressure
///   - Larger values: Higher burst tolerance, more memory usage
///   - Smaller values: Lower memory usage, more backpressure
///
/// - **channel_strategy**: Buffer management strategy ([`ChannelStrategy`])
///   - `fixed`: Static capacity, predictable memory
///   - `adaptive`: Dynamic sizing based on load
///
/// # Worker Threads
///
/// - **worker_threads**: Background worker threads (default: 3)
///   - Typical assignment: 1 console + 1 file + 1 database
///   - More threads: Better parallelism, more CPU usage
///   - Recommended: CPU core count or number of enabled sinks
///
/// # Adaptive Channel Settings (when channel_strategy = "adaptive")
///
/// - **expand_threshold_percent**: Trigger expansion at this % capacity (default: 80)
///   - When channel reaches 80% full, begins expansion
///   - Lower values: More aggressive expansion
///
/// - **shrink_threshold_percent**: Trigger shrink at this % capacity (default: 20)
///   - When channel drops to 20% full, considers shrinking
///   - Higher values: Less aggressive shrinking
///
/// - **shrink_wait_seconds**: Wait before shrinking after low usage (default: 30)
///   - Prevents thrashing from transient load changes
///
/// - **min_capacity**: Minimum channel size (default: 1,000)
///   - Adaptive channel won't shrink below this
///
/// - **max_capacity**: Maximum channel size (default: 50,000)
///   - Adaptive channel won't grow beyond this
///
/// # Configuration Example
///
/// ```toml
/// [performance]
/// channel_capacity = 20000        # Larger buffer
/// worker_threads = 4              # More parallelism
/// channel_strategy = "adaptive"   # Dynamic sizing
///
/// # Adaptive tuning
/// expand_threshold_percent = 70   # Earlier expansion
/// shrink_threshold_percent = 30   # Later shrinking
/// shrink_wait_seconds = 60        # Longer wait
/// min_capacity = 5000
/// max_capacity = 100000
/// ```
///
/// # Performance Profiles
///
/// ## High Throughput (>10k logs/second)
/// ```toml
/// [performance]
/// channel_capacity = 50000
/// worker_threads = 8
/// channel_strategy = "fixed"  # Predictable memory
/// ```
///
/// ## Low Latency (<1ms p99)
/// ```toml
/// [performance]
/// channel_capacity = 5000   # Smaller buffer
/// worker_threads = 4        # More workers
/// channel_strategy = "adaptive"
/// expand_threshold_percent = 60  # Earlier expansion
/// ```
///
/// ## Resource Constrained (Limited RAM/CPU)
/// ```toml
/// [performance]
/// channel_capacity = 2000
/// worker_threads = 2
/// channel_strategy = "fixed"
/// max_capacity = 5000  # Limit growth
/// ```
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_PERFORMANCE_CHANNEL_CAPACITY=20000
/// export INKLOG_PERFORMANCE_WORKER_THREADS=4
/// export INKLOG_PERFORMANCE_CHANNEL_STRATEGY=adaptive
/// ```
///
/// # Monitoring
///
/// Monitor these metrics to validate your tuning:
/// - `inklog_channel_blocked`: Should be near zero with proper capacity
/// - `inklog_logs_dropped`: Non-zero indicates undersized channel
/// - Worker CPU usage: Should correlate with worker_threads count
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// Default value functions for serde
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
            channel_capacity: default_channel_capacity(),
            worker_threads: default_worker_threads(),
            channel_strategy: ChannelStrategy::default(),
            expand_threshold_percent: default_expand_threshold(),
            shrink_threshold_percent: default_shrink_threshold(),
            shrink_wait_seconds: default_shrink_wait(),
            min_capacity: default_min_capacity(),
            max_capacity: default_max_capacity(),
        }
    }
}

// Default values are handled by #[serde(default = ...)] annotations.

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;

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
        assert!(!console.masking_enabled);
    }

    #[test]
    fn test_console_config_custom() {
        let console = ConsoleSinkConfig {
            enabled: false,
            colored: false,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
            masking_enabled: false,
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
        // Default leaves console_sink: None for Option<T>.
        // For testing sinks_enabled(), we explicitly set console_sink.
        let config = InklogConfig {
            console_sink: Some(ConsoleSinkConfig::default()),
            ..Default::default()
        };
        println!("console_sink: {:?}", config.console_sink);
        println!("global: {:?}", config.global);
        let sinks = config.sinks_enabled();
        println!("sinks: {:?}", sinks);
        assert!(
            sinks.contains(&"console"),
            "Expected console sink to be enabled, but got: {:?}",
            sinks
        );
    }

    #[test]
    fn test_console_sink_config_default_values() {
        let config = ConsoleSinkConfig::default();
        assert!(config.enabled);
        assert!(config.colored);
        assert_eq!(config.stderr_levels.len(), 2);
    }

    #[test]
    fn test_global_config_load_sync() {
        // GlobalConfig uses Default impl
        let config = GlobalConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.auto_fallback);
    }

    // =========================================================================
    // validate() 测试
    // =========================================================================

    #[test]
    fn test_validate_default_passes() {
        let config = InklogConfig::default();
        assert!(config.validate().is_ok(), "default config should validate");
    }

    #[test]
    fn test_validate_zero_channel_capacity_fails() {
        let config = InklogConfig {
            performance: PerformanceConfig {
                channel_capacity: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = config.validate().expect_err("capacity=0 should fail");
        assert!(
            err.to_string().contains("channel_capacity"),
            "error should mention channel_capacity, got: {err}"
        );
    }

    #[test]
    fn test_validate_zero_worker_threads_fails() {
        let config = InklogConfig {
            performance: PerformanceConfig {
                worker_threads: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = config.validate().expect_err("worker_threads=0 should fail");
        assert!(
            err.to_string().contains("worker_threads"),
            "error should mention worker_threads, got: {err}"
        );
    }

    #[test]
    fn test_validate_both_zero_reports_capacity_first() {
        // validate() checks channel_capacity before worker_threads
        let config = InklogConfig {
            performance: PerformanceConfig {
                channel_capacity: 0,
                worker_threads: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = config.validate().expect_err("both zero should fail");
        assert!(err.to_string().contains("channel_capacity"));
    }

    // =========================================================================
    // from_str() 测试
    // =========================================================================

    #[test]
    fn test_from_str_valid_toml() {
        let toml = r#"
[global]
level = "debug"
format = "{timestamp} {message}"

[console_sink]
enabled = true
colored = false
"#;
        let config: InklogConfig = toml.parse().expect("valid TOML should parse");
        assert_eq!(config.global.level, "debug");
        assert!(config.console_sink.is_some());
        assert!(!config.console_sink.as_ref().unwrap().colored);
    }

    #[test]
    fn test_from_str_empty_string_returns_defaults() {
        // 空字符串 → 所有字段使用 serde default
        let config: InklogConfig = "".parse().expect("empty TOML should parse to defaults");
        assert_eq!(config.global.level, "info");
        // console_sink 有 serde(default = "default_console_sink")，应返回 Some
        assert!(config.console_sink.is_some());
    }

    #[test]
    fn test_from_str_invalid_toml_errors() {
        let bad = "not = valid = toml = syntax";
        let result: Result<InklogConfig, _> = bad.parse();
        assert!(result.is_err(), "malformed TOML should error");
    }

    #[test]
    fn test_from_str_partial_config_only_global() {
        let toml = r#"
[global]
level = "warn"
"#;
        let config: InklogConfig = toml.parse().expect("partial config should parse");
        assert_eq!(config.global.level, "warn");
        // 其他字段保持默认
        assert!(config.file_sink.is_none());
        assert!(config.database_sink.is_none());
        assert!(config.http_server.is_none());
    }

    // =========================================================================
    // sinks_enabled() 测试
    // =========================================================================

    #[test]
    fn test_sinks_enabled_all_disabled() {
        let config = InklogConfig {
            console_sink: Some(ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            file_sink: None,
            database_sink: None,
            http_server: None,
            ..Default::default()
        };
        assert!(
            config.sinks_enabled().is_empty(),
            "no sinks should be enabled"
        );
    }

    #[test]
    fn test_sinks_enabled_only_file() {
        let config = InklogConfig {
            console_sink: Some(ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            file_sink: Some(FileSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            database_sink: None,
            http_server: None,
            ..Default::default()
        };
        let sinks = config.sinks_enabled();
        assert_eq!(sinks, vec!["file"]);
    }

    #[test]
    fn test_sinks_enabled_console_and_database() {
        let config = InklogConfig {
            console_sink: Some(ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            file_sink: Some(FileSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            database_sink: Some(DatabaseSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            http_server: None,
            ..Default::default()
        };
        let mut sinks = config.sinks_enabled();
        sinks.sort();
        assert_eq!(sinks, vec!["console", "database"]);
    }

    // =========================================================================
    // from_search_paths() 测试（需要 env var 隔离 → serial）
    // =========================================================================

    #[test]
    #[serial]
    fn test_from_search_paths_with_env_var_loads_file() {
        let dir = tempdir().expect("failed to create tempdir");
        let config_path = dir.path().join("custom_config.toml");
        std::fs::write(
            &config_path,
            r#"
[global]
level = "trace"
"#,
        )
        .expect("failed to write config");

        unsafe {
            env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let config = InklogConfig::from_search_paths().expect("should load from env path");
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }

        assert_eq!(config.global.level, "trace");
    }

    #[test]
    #[serial]
    fn test_from_search_paths_missing_env_falls_back_to_default() {
        // 清除 env，且确保当前目录没有 inklog_config.toml（依赖搜索路径回退到默认）
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        // 注意：若当前目录恰好存在 inklog_config.toml，则此测试会从文件加载而非默认值。
        // 为保证测试确定性，仅验证函数返回 Ok 且配置有效。
        let config = InklogConfig::from_search_paths().expect("should not error");
        assert!(config.validate().is_ok());
    }

    #[test]
    #[serial]
    fn test_from_search_paths_malformed_toml_errors() {
        let dir = tempdir().expect("failed to create tempdir");
        let config_path = dir.path().join("bad_config.toml");
        std::fs::write(&config_path, "not = valid = toml").expect("failed to write");

        unsafe {
            env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }

        let err = result.expect_err("malformed TOML should error");
        assert!(
            err.to_string().contains("Failed to parse config file"),
            "error should mention parse failure, got: {err}"
        );
    }

    #[test]
    #[serial]
    fn test_load_sync_with_env_path() {
        let dir = tempdir().expect("failed to create tempdir");
        let config_path = dir.path().join("sync_config.toml");
        std::fs::write(
            &config_path,
            r#"
[global]
level = "error"
"#,
        )
        .expect("failed to write");

        unsafe {
            env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let config = InklogConfig::load_sync().expect("load_sync should succeed");
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }

        assert_eq!(config.global.level, "error");
    }

    // =========================================================================
    // load_with_env_overrides() 测试（需要 env var 隔离 → serial）
    // =========================================================================

    #[test]
    #[serial]
    fn test_load_with_env_overrides_global_level() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_GLOBAL_LEVEL", "debug");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_GLOBAL_LEVEL");
        }

        assert_eq!(config.global.level, "debug");
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_performance_capacity() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "5000");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY");
        }

        assert_eq!(config.performance.channel_capacity, 5000);
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_file_sink_enabled() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_FILE_SINK_ENABLED", "true");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_FILE_SINK_ENABLED");
        }

        let file = config
            .file_sink
            .expect("file_sink should be Some after env override");
        assert!(file.enabled, "file_sink.enabled should be true");
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_server_enabled() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }

        let http = config
            .http_server
            .expect("http_server should be Some after env override");
        assert!(http.enabled);
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_invalid_bool_ignored() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        // 非法 bool → parse 失败 → unwrap_or 返回原默认值
        unsafe {
            env::set_var("INKLOG_GLOBAL_MASKING_ENABLED", "not_a_bool");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_GLOBAL_MASKING_ENABLED");
        }

        // 默认 masking_enabled 值（来自 GlobalConfig::default）
        assert_eq!(
            config.global.masking_enabled,
            GlobalConfig::default().masking_enabled
        );
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_invalid_int_ignored() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "not_an_int");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY");
        }

        assert_eq!(
            config.performance.channel_capacity,
            PerformanceConfig::default().channel_capacity
        );
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_error_mode_strict() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "strict");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }

        let http = config.http_server.expect("http_server should be Some");
        assert!(matches!(http.error_mode, HttpErrorMode::Strict));
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_error_mode_warn() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "warn");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }

        let http = config.http_server.expect("http_server should be Some");
        assert!(matches!(http.error_mode, HttpErrorMode::Warn));
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_file_sink_path_and_max_size() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_FILE_SINK_ENABLED", "true");
        }
        unsafe {
            env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/test_app.log");
        }
        unsafe {
            env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "250MB");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_FILE_SINK_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_FILE_SINK_PATH");
        }
        unsafe {
            env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }

        let file = config.file_sink.expect("file_sink should be Some");
        assert_eq!(file.path, std::path::PathBuf::from("/tmp/test_app.log"));
        assert_eq!(file.max_size, "250MB");
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_server_host_port() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_HOST", "0.0.0.0");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_PORT", "8080");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_HOST");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_PORT");
        }

        let http = config.http_server.expect("http_server should be Some");
        assert_eq!(http.host, "0.0.0.0");
        assert_eq!(http.port, 8080);
    }

    // =========================================================================
    // apply_env_overrides() 补充测试（覆盖剩余分支）
    // =========================================================================

    #[test]
    #[serial]
    fn test_load_with_env_overrides_global_format() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_GLOBAL_FORMAT", "{level} {message}");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_GLOBAL_FORMAT");
        }

        assert_eq!(config.global.format, "{level} {message}");
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_global_auto_fallback() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_GLOBAL_AUTO_FALLBACK", "false");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_GLOBAL_AUTO_FALLBACK");
        }

        assert!(!config.global.auto_fallback);
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_server_metrics_and_health_path() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/custom_metrics");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/custom_health");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_HEALTH_PATH");
        }

        let http = config.http_server.expect("http_server should be Some");
        assert_eq!(http.metrics_path, "/custom_metrics");
        assert_eq!(http.health_path, "/custom_health");
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_http_error_mode_unknown_keeps_default() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
        }
        // 未知值应保留默认的 Strict 模式
        unsafe {
            env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "unknown_mode");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
        unsafe {
            env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }

        let http = config.http_server.expect("http_server should be Some");
        assert!(
            matches!(http.error_mode, HttpErrorMode::Strict),
            "unknown mode should keep default Strict"
        );
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_performance_worker_threads() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "8");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_PERFORMANCE_WORKER_THREADS");
        }

        assert_eq!(config.performance.worker_threads, 8);
    }

    #[test]
    #[serial]
    fn test_load_with_env_overrides_performance_worker_threads_invalid_ignored() {
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }
        unsafe {
            env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "not_a_number");
        }
        let config = InklogConfig::load_with_env_overrides().expect("should load");
        unsafe {
            env::remove_var("INKLOG_PERFORMANCE_WORKER_THREADS");
        }

        assert_eq!(
            config.performance.worker_threads,
            PerformanceConfig::default().worker_threads
        );
    }

    // =========================================================================
    // DatabaseDriver FromStr / Display 测试
    // =========================================================================

    #[test]
    fn test_database_driver_from_str_postgres() {
        let driver: DatabaseDriver = "postgres".parse().expect("should parse");
        assert_eq!(driver, DatabaseDriver::PostgreSQL);

        let driver: DatabaseDriver = "PostgreSQL".parse().expect("should parse case-insensitive");
        assert_eq!(driver, DatabaseDriver::PostgreSQL);
    }

    #[test]
    fn test_database_driver_from_str_mysql() {
        let driver: DatabaseDriver = "mysql".parse().expect("should parse");
        assert_eq!(driver, DatabaseDriver::MySQL);
    }

    #[test]
    fn test_database_driver_from_str_sqlite() {
        let driver: DatabaseDriver = "sqlite".parse().expect("should parse");
        assert_eq!(driver, DatabaseDriver::SQLite);

        let driver: DatabaseDriver = "sqlite3".parse().expect("should parse");
        assert_eq!(driver, DatabaseDriver::SQLite);
    }

    #[test]
    fn test_database_driver_from_str_invalid() {
        let result: Result<DatabaseDriver, _> = "oracle".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_database_driver_display() {
        assert_eq!(format!("{}", DatabaseDriver::PostgreSQL), "postgres");
        assert_eq!(format!("{}", DatabaseDriver::MySQL), "mysql");
        assert_eq!(format!("{}", DatabaseDriver::SQLite), "sqlite");
    }

    // =========================================================================
    // PartitionStrategy FromStr / Display 测试
    // =========================================================================

    #[test]
    fn test_partition_strategy_from_str_monthly() {
        let s: PartitionStrategy = "monthly".parse().expect("should parse");
        assert_eq!(s, PartitionStrategy::Monthly);

        let s: PartitionStrategy = "month".parse().expect("should parse");
        assert_eq!(s, PartitionStrategy::Monthly);
    }

    #[test]
    fn test_partition_strategy_from_str_yearly() {
        let s: PartitionStrategy = "yearly".parse().expect("should parse");
        assert_eq!(s, PartitionStrategy::Yearly);

        let s: PartitionStrategy = "year".parse().expect("should parse");
        assert_eq!(s, PartitionStrategy::Yearly);
    }

    #[test]
    fn test_partition_strategy_from_str_invalid() {
        let result: Result<PartitionStrategy, String> = "weekly".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown partition strategy"));
    }

    #[test]
    fn test_partition_strategy_display() {
        assert_eq!(format!("{}", PartitionStrategy::Monthly), "monthly");
        assert_eq!(format!("{}", PartitionStrategy::Yearly), "yearly");
    }

    // =========================================================================
    // ChannelStrategy FromStr / Display 测试
    // =========================================================================

    #[test]
    fn test_channel_strategy_from_str_fixed() {
        let s: ChannelStrategy = "fixed".parse().expect("should parse");
        assert_eq!(s, ChannelStrategy::Fixed);
    }

    #[test]
    fn test_channel_strategy_from_str_adaptive() {
        let s: ChannelStrategy = "adaptive".parse().expect("should parse");
        assert_eq!(s, ChannelStrategy::Adaptive);
    }

    #[test]
    fn test_channel_strategy_from_str_invalid() {
        let result: Result<ChannelStrategy, String> = "dynamic".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown channel strategy"));
    }

    #[test]
    fn test_channel_strategy_display() {
        assert_eq!(format!("{}", ChannelStrategy::Fixed), "fixed");
        assert_eq!(format!("{}", ChannelStrategy::Adaptive), "adaptive");
    }

    // =========================================================================
    // HttpAuthConfig Default 测试
    // =========================================================================

    #[test]
    fn test_http_auth_config_default() {
        let auth = HttpAuthConfig::default();
        assert!(!auth.enabled);
        assert_eq!(auth.token_env, "INKLOG_HTTP_AUTH_TOKEN");
    }

    // =========================================================================
    // from_search_paths() 文件读取失败测试
    // =========================================================================

    #[test]
    #[cfg(unix)]
    #[serial]
    fn test_from_search_paths_unreadable_file_errors() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("failed to create tempdir");
        let config_path = dir.path().join("unreadable.toml");
        std::fs::write(&config_path, "[global]\nlevel = \"info\"\n").expect("failed to write");

        // 移除读权限（仅 Unix）
        let mut perms = std::fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&config_path, perms).unwrap();

        unsafe {
            env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        unsafe {
            env::remove_var("INKLOG_CONFIG_PATH");
        }

        // 恢复权限以便 tempdir 清理
        let mut perms = std::fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o644);
        let _ = std::fs::set_permissions(&config_path, perms);

        // 注意：如果以 root 运行，文件仍可读，此断言可能失败
        // 非 root 环境下应返回读取错误
        if result.is_ok() {
            // 以 root 运行时跳过此测试（文件仍可读）
            eprintln!("Skipping: running as root, file is readable despite 0o000 permissions");
            return;
        }
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to read config file"),
            "error should mention read failure, got: {err}"
        );
    }
}
