// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Root configuration struct and loading logic.

use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::console::ConsoleSinkConfig;
use super::database::DatabaseDriver;
use super::database::DatabaseSinkConfig;
use super::file_sink::FileSinkConfig;
use super::global::GlobalConfig;
use super::http::HttpServerConfig;
use super::performance::MAX_CHANNEL_CAPACITY;
use super::performance::PerformanceConfig;

// Re-export HttpErrorMode for env override match in this file
use super::http::HttpErrorMode;

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
    /// Per-crate target level presets (e.g. `{"hyper" = "warn", "my_crate" = "debug"}`).
    ///
    /// These are merged into the `EnvFilter` alongside the global level and
    /// `RUST_LOG` overrides. Target-level entries have lower priority than
    /// `RUST_LOG` but higher than the global default.
    #[serde(default)]
    pub target_levels: HashMap<String, String>,
}

fn default_console_sink() -> Option<ConsoleSinkConfig> {
    Some(ConsoleSinkConfig::default())
}

/// Decode `%XX` percent-encoding so path validation cannot be bypassed with
/// encoded traversal, backslashes, or control characters. Invalid escape
/// sequences are kept as-is.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(high) = (bytes[i + 1] as char).to_digit(16)
            && let Some(low) = (bytes[i + 2] as char).to_digit(16)
        {
            decoded.push(((high * 16) + low) as u8);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// An HTTP path override is safe when the percent-decoded value starts with
/// `/` and contains no traversal, backslashes, or control characters.
fn is_safe_http_path(value: &str) -> bool {
    let decoded = percent_decode(value);
    decoded.starts_with('/')
        && !decoded.contains("..")
        && !decoded.contains('\\')
        && !decoded.chars().any(char::is_control)
}

/// Validate the `前缀.*` wildcard form of an `ip_whitelist` entry (e.g. `10.*`,
/// `192.168.*`): after stripping the `.*` suffix the remainder must be 1-3
/// dot-separated decimal segments (each 0-255) and must not contain `/`.
///
/// This keeps config-time validation consistent with the runtime
/// `whitelist_entry_matches` in http_server, which explicitly supports the
/// `<prefix>.*` form (with a trailing-dot guard against overreaching prefix
/// matches). Previously only `IpAddr`/`IpNet` were accepted, so `10.*` was
/// rejected at load time despite being valid at runtime.
fn is_valid_prefix_wildcard_entry(entry: &str) -> bool {
    let Some(prefix) = entry.strip_suffix(".*") else {
        return false;
    };
    if prefix.contains('/') {
        return false;
    }
    let mut segments = 0;
    for segment in prefix.split('.') {
        if segment.is_empty()
            || !segment.bytes().all(|b| b.is_ascii_digit())
            || segment.parse::<u8>().is_err()
        {
            return false;
        }
        segments += 1;
        if segments > 3 {
            return false;
        }
    }
    // split(".") 至少产出一段；空 prefix（"::.*"、".*"）已被空段检查拒绝
    true
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
            target_levels: HashMap::new(),
        }
    }
}

impl InklogConfig {
    /// Load configuration synchronously from the default search paths.
    pub fn load_sync() -> Result<Self, InklogError> {
        Self::from_search_paths().map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-load_failed", args))
        })
    }

    /// Load configuration with custom environment variable overrides.
    pub fn load_with_env_overrides() -> Result<Self, InklogError> {
        let mut config = Self::load_sync()?;
        Self::apply_env_overrides(&mut config);
        Ok(config)
    }

    /// Apply environment variable overrides to the configuration.
    fn apply_env_overrides(config: &mut Self) {
        // Global config overrides
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_LEVEL") {
            // Validate against known log levels before applying (case-insensitive, includes aliases)
            if crate::LogLevel::is_valid_level(&val) {
                config.global.level = val;
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("val", &val);
                args.set("current", &config.global.level);
                tracing::warn!("{}", crate::i18n::tr_args("config-env_invalid_level", args));
            }
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_FORMAT") {
            if !val.is_empty() {
                config.global.format = val;
            } else {
                tracing::warn!("{}", crate::i18n::tr("config-env_invalid_format"));
            }
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_MASKING_ENABLED") {
            config.global.masking_enabled = val.parse().unwrap_or(config.global.masking_enabled);
        }
        if let Ok(val) = std::env::var("INKLOG_GLOBAL_AUTO_FALLBACK") {
            config.global.auto_fallback = val.parse().unwrap_or(config.global.auto_fallback);
        }

        // File sink overrides
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_ENABLED")
            && val.parse::<bool>().unwrap_or(false)
        {
            let file_config = config.file_sink.get_or_insert_with(Default::default);
            file_config.enabled = true;
        }
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_PATH") {
            // 验证路径不含遍历模式或可疑字符
            let path_buf = std::path::PathBuf::from(&val);
            let has_traversal = path_buf
                .components()
                .any(|c| c == std::path::Component::ParentDir);
            let has_null = val.contains('\0');
            if has_traversal || has_null {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", &val);
                tracing::warn!("{}", crate::i18n::tr_args("config-env_unsafe_path", args));
            } else {
                let file_config = config.file_sink.get_or_insert_with(Default::default);
                file_config.path = path_buf;
            }
        }
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_MAX_SIZE") {
            // Validate that the value looks like a parseable size string
            let trimmed = val.trim();
            let has_numeric = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
            if has_numeric == 0 {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("val", &val);
                tracing::warn!("{}", crate::i18n::tr_args("config-env_invalid_size", args));
            } else {
                let file_config = config.file_sink.get_or_insert_with(Default::default);
                file_config.max_size = val;
            }
        }

        // HTTP server overrides
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_ENABLED")
            && val.parse::<bool>().unwrap_or(false)
        {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.enabled = true;
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
            // Validate HTTP path: must start with '/' and contain no traversal
            // (checked on the percent-decoded value to catch encoded variants)
            if is_safe_http_path(&val) {
                let http_config = config.http_server.get_or_insert_with(Default::default);
                http_config.metrics_path = val;
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", &val);
                args.set("env_var", "INKLOG_HTTP_SERVER_METRICS_PATH");
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_unsafe_http_path", args)
                );
            }
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_HEALTH_PATH") {
            // Validate HTTP path: must start with '/' and contain no traversal
            // (checked on the percent-decoded value to catch encoded variants)
            if is_safe_http_path(&val) {
                let http_config = config.http_server.get_or_insert_with(Default::default);
                http_config.health_path = val;
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("path", &val);
                args.set("env_var", "INKLOG_HTTP_SERVER_HEALTH_PATH");
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_unsafe_http_path", args)
                );
            }
        }
        if let Ok(val) = std::env::var("INKLOG_HTTP_SERVER_ERROR_MODE") {
            let http_config = config.http_server.get_or_insert_with(Default::default);
            http_config.error_mode = if val.eq_ignore_ascii_case("strict") {
                HttpErrorMode::Strict
            } else if val.eq_ignore_ascii_case("warn") {
                HttpErrorMode::Warn
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("val", &val);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_unknown_error_mode", args)
                );
                http_config.error_mode.clone()
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

        // Database sink overrides
        if let Ok(val) = std::env::var("INKLOG_DATABASE_SINK_URL") {
            let path_buf = std::path::PathBuf::from(&val);
            let has_traversal = path_buf
                .components()
                .any(|c| c == std::path::Component::ParentDir);
            if has_traversal || val.contains('\0') {
                tracing::warn!("{}", crate::i18n::tr("config-env_invalid_db_url"));
            } else {
                let db_config = config.database_sink.get_or_insert_with(Default::default);
                db_config.url = val;
            }
        }
        if let Ok(val) = std::env::var("INKLOG_DATABASE_SINK_POOL_SIZE") {
            let db_config = config.database_sink.get_or_insert_with(Default::default);
            db_config.pool_size = val.parse().unwrap_or(db_config.pool_size);
        }
        if let Ok(val) = std::env::var("INKLOG_DATABASE_SINK_BATCH_SIZE") {
            let db_config = config.database_sink.get_or_insert_with(Default::default);
            db_config.batch_size = val.parse().unwrap_or(db_config.batch_size);
        }
        if let Ok(val) = std::env::var("INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS") {
            let db_config = config.database_sink.get_or_insert_with(Default::default);
            db_config.flush_interval_ms = val.parse().unwrap_or(db_config.flush_interval_ms);
        }
        if let Ok(val) = std::env::var("INKLOG_DATABASE_SINK_TABLE_NAME") {
            let db_config = config.database_sink.get_or_insert_with(Default::default);
            db_config.table_name = val;
        }
    }

    /// Search paths (first existing file wins):
    /// 1. `$INKLOG_CONFIG_PATH`
    /// 2. `inklog_config.toml` (current directory)
    /// 3. `~/.config/inklog/config.toml` (user config dir)
    /// 4. Platform-specific system config:
    ///    - Unix/Linux: `/etc/inklog/config.toml`
    ///    - Windows: `%ProgramData%\inklog\config.toml`
    pub fn from_search_paths() -> Result<Self, InklogError> {
        let mut search_paths: Vec<Option<String>> = vec![
            std::env::var("INKLOG_CONFIG_PATH").ok(),
            Some("inklog_config.toml".to_string()),
            dirs::config_dir().map(|p| {
                p.join("inklog")
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string()
            }),
        ];
        search_paths.push(Self::system_config_path());

        // Maximum config file size (1 MB) to prevent memory exhaustion
        const MAX_CONFIG_SIZE: u64 = 1024 * 1024;

        for path_opt in search_paths.into_iter().flatten() {
            if std::path::Path::new(&path_opt).exists() {
                // Guard against oversized config files (potential DoS)
                if let Ok(meta) = std::fs::metadata(&path_opt)
                    && meta.len() > MAX_CONFIG_SIZE
                {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("size", meta.len().to_string());
                    args.set("max", MAX_CONFIG_SIZE.to_string());
                    return Err(InklogError::ConfigError(crate::i18n::tr_args(
                        "config-exceeds_max_size",
                        args,
                    )));
                }
                let content = std::fs::read_to_string(&path_opt).map_err(|e| {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("err", e.to_string());
                    InklogError::ConfigError(crate::i18n::tr_args("config-read_failed", args))
                })?;
                let mut config: Self = toml::from_str(&content).map_err(|e| {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("err", e.to_string());
                    InklogError::ConfigError(crate::i18n::tr_args("config-parse_failed", args))
                })?;
                // 配置加载完成的构造出口：先规范化可修正项，再执行硬校验
                config.normalize();
                config.validate()?;
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    /// Returns the platform-specific system configuration path.
    ///
    /// - Unix/Linux: `/etc/inklog/config.toml`
    /// - Windows: `%ProgramData%\inklog\config.toml`
    fn system_config_path() -> Option<String> {
        #[cfg(unix)]
        {
            Some("/etc/inklog/config.toml".to_string())
        }
        #[cfg(windows)]
        {
            std::env::var("ProgramData")
                .ok()
                .map(|base| format!("{}\\inklog\\config.toml", base))
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
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

    /// Normalize configuration values by auto-correcting invalid settings.
    ///
    /// Unlike `validate()` which returns errors, this method adjusts values
    /// in-place with warnings. Call before `validate()` to auto-fix common
    /// misconfigurations. Problems that cannot be auto-corrected are logged
    /// here and surface as errors from `validate()`.
    pub fn normalize(&mut self) {
        if let Err(e) = self.global.validate() {
            tracing::warn!("{e}");
        }
        if let Err(e) = self.performance.validate() {
            tracing::warn!("{e}");
        }
        if let Some(ref mut db) = self.database_sink
            && let Err(e) = db.validate()
        {
            tracing::warn!("{e}");
        }
        if let Some(ref mut console) = self.console_sink {
            console.validate();
        }
    }

    /// Validate the configuration.
    ///
    /// Checks for common misconfigurations and returns a descriptive error
    /// for the first problem found.
    pub fn validate(&self) -> Result<(), InklogError> {
        // --- Performance ---
        if self.performance.channel_capacity == 0 {
            return Err(InklogError::ConfigError(crate::i18n::tr(
                "config-channel_capacity_zero",
            )));
        }
        if self.performance.channel_capacity > MAX_CHANNEL_CAPACITY {
            return Err(InklogError::ConfigError(format!(
                "channel_capacity {} exceeds maximum {MAX_CHANNEL_CAPACITY}",
                self.performance.channel_capacity
            )));
        }
        if self.performance.worker_threads == 0 {
            return Err(InklogError::ConfigError(crate::i18n::tr(
                "config-worker_threads_zero",
            )));
        }
        if self.performance.max_capacity < self.performance.min_capacity {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("max", self.performance.max_capacity.to_string());
            args.set("min", self.performance.min_capacity.to_string());
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-max_capacity_lt_min",
                args,
            )));
        }

        // --- Global log level ---
        if !crate::LogLevel::is_valid_level(&self.global.level) {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("level", self.global.level.clone());
            args.set("valid", crate::LogLevel::VALID_LEVEL_STRINGS.join(", "));
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-invalid_log_level",
                args,
            )));
        }

        // --- File sink ---
        if let Some(ref file) = self.file_sink
            && file.enabled
        {
            if file.path.as_os_str().is_empty() {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-file_path_empty",
                )));
            }
            if file.batch_size == 0 {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-file_batch_size_zero",
                )));
            }
            file.validate().map_err(InklogError::ConfigError)?;
        }

        // --- Database sink ---
        if let Some(ref db) = self.database_sink
            && db.enabled
            && db.driver != DatabaseDriver::SQLite
            && db.pool_size == 0
        {
            return Err(InklogError::ConfigError(format!(
                "database_sink.pool_size must be at least 1 for the {} driver, got 0",
                db.driver
            )));
        }

        // --- HTTP server ---
        if let Some(ref http) = self.http_server
            && http.enabled
        {
            if http.port == 0 {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-http_port_zero",
                )));
            }
            // ip_whitelist 条目格式校验：解析失败的条目在运行期会被静默跳过
            // （fail-closed），导致预期放行的 IP 被拒且无任何提示，故在配置期
            // 直接拒绝坏条目。合法形式三选一：精确 IP、CIDR、`前缀.*` 通配
            // （运行期 whitelist_entry_matches 支持的同款语法）。
            if let Some(ref whitelist) = http.ip_whitelist {
                for (idx, entry) in whitelist.iter().enumerate() {
                    let parseable = entry.parse::<std::net::IpAddr>().is_ok()
                        || entry.parse::<ipnet::IpNet>().is_ok()
                        || is_valid_prefix_wildcard_entry(entry);
                    if !parseable {
                        return Err(InklogError::ConfigError(format!(
                            "http_server.ip_whitelist[{idx}] is not a valid IP, CIDR, or prefix wildcard (e.g. \"10.*\"): {entry:?}"
                        )));
                    }
                }
            }
            if let Some(ref tls) = http.tls {
                tls.validate().map_err(InklogError::ConfigError)?;
            }
        }

        // --- Console sink stderr_levels ---
        if let Some(ref console) = self.console_sink
            && let Some(invalid) = console.invalid_stderr_levels().first()
        {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("level", *invalid);
            args.set("valid", crate::LogLevel::VALID_LEVEL_STRINGS.join(", "));
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-invalid_stderr_level",
                args,
            )));
        }

        Ok(())
    }
}

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut config: Self = toml::from_str(s)?;
        config.normalize();
        config.validate().map_err(|e| {
            use serde::de::Error as _;
            toml::de::Error::custom(e.to_string())
        })?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mutex to serialize env var tests (env vars are process-global state)
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_validate_default_config() {
        let config = InklogConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let mut config = InklogConfig::default();
        config.global.level = "verbose".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("Invalid log level"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_validate_valid_log_levels() {
        for level in &[
            "trace", "debug", "info", "warn", "error", "fatal", "INFO", "Debug",
        ] {
            let mut config = InklogConfig::default();
            config.global.level = level.to_string();
            assert!(
                config.validate().is_ok(),
                "level '{}' should be valid",
                level
            );
        }
    }

    #[test]
    fn test_validate_zero_channel_capacity() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = 0;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("channel_capacity"));
    }

    #[test]
    fn test_validate_zero_worker_threads() {
        let mut config = InklogConfig::default();
        config.performance.worker_threads = 0;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("worker_threads"));
    }

    #[test]
    fn test_validate_max_capacity_less_than_min() {
        let mut config = InklogConfig::default();
        config.performance.max_capacity = 100;
        config.performance.min_capacity = 500;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("max_capacity"));
    }

    #[test]
    fn test_validate_file_sink_empty_path() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::new(),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("file_sink.path"));
    }

    #[test]
    fn test_validate_file_sink_zero_batch_size() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            batch_size: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn test_validate_http_port_zero() {
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            port: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn test_validate_http_disabled_port_zero_ok() {
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: false,
            port: 0,
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_system_config_path_returns_platform_path() {
        let path = InklogConfig::system_config_path();
        assert!(
            path.is_some(),
            "system_config_path should return Some on unix"
        );
        let path = path.unwrap();
        #[cfg(unix)]
        assert_eq!(path, "/etc/inklog/config.toml");
        #[cfg(windows)]
        assert!(path.contains("inklog") && path.contains("config.toml"));
    }

    #[test]
    fn test_from_search_paths_falls_back_to_default() {
        // 与会临时设置 INKLOG_CONFIG_PATH 的测试互斥，避免并发读全局环境变量竞态
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
        // When no config files exist, should return default config
        let config = InklogConfig::from_search_paths().unwrap();
        assert_eq!(config.global.level, "info");
    }

    #[test]
    fn test_env_override_invalid_level_falls_back() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // invalid log level should not be applied
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "invalid_level");
        }
        let mut config = InklogConfig::default();
        let original_level = config.global.level.clone();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(
            config.global.level, original_level,
            "invalid level should not override current value"
        );
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_sinks_enabled_default() {
        let config = InklogConfig::default();
        let sinks = config.sinks_enabled();
        // default has console enabled
        assert!(sinks.contains(&"console"));
    }

    #[test]
    fn test_sinks_enabled_with_file() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("test.log"),
            ..Default::default()
        });
        let sinks = config.sinks_enabled();
        assert!(sinks.contains(&"console"));
        assert!(sinks.contains(&"file"));
    }

    #[test]
    fn test_sinks_enabled_none() {
        let mut config = InklogConfig::default();
        config.console_sink = None;
        config.file_sink = None;
        config.database_sink = None;
        let sinks = config.sinks_enabled();
        assert!(sinks.is_empty());
    }

    #[test]
    fn test_normalize_fixes_invalid_settings() {
        let mut config = InklogConfig::default();
        config.global.fallback_max_retries = 0;
        config.performance.expand_threshold_percent = 200;
        config.normalize();
        // global.validate() should reset retries
        assert_eq!(config.global.fallback_max_retries, 1);
        // performance.validate() should clamp
        assert_eq!(config.performance.expand_threshold_percent, 100);
    }

    #[test]
    fn test_from_str_parses_toml() {
        let toml_str = r#"
[global]
level = "debug"
"#;
        let config: InklogConfig = toml_str.parse().expect("should parse TOML");
        assert_eq!(config.global.level, "debug");
    }

    #[test]
    fn test_apply_env_overrides_valid_level() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "warn");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.level, "warn");
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_apply_env_overrides_format() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_FORMAT", "{message}");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.format, "{message}");
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_FORMAT");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/test.log");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("/tmp/test.log")
        );
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_path_traversal_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "../etc/passwd");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // path with parent dir traversal should be ignored, file_sink should remain None
        assert!(config.file_sink.is_none());
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_server() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
            std::env::set_var("INKLOG_HTTP_SERVER_HOST", "0.0.0.0");
            std::env::set_var("INKLOG_HTTP_SERVER_PORT", "8080");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        let http = config.http_server.as_ref().unwrap();
        assert!(http.enabled);
        assert_eq!(http.host, "0.0.0.0");
        assert_eq!(http.port, 8080);
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
            std::env::remove_var("INKLOG_HTTP_SERVER_HOST");
            std::env::remove_var("INKLOG_HTTP_SERVER_PORT");
        }
    }

    #[test]
    fn test_apply_env_overrides_performance() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "8");
            std::env::set_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "2048");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.performance.worker_threads, 8);
        assert_eq!(config.performance.channel_capacity, 2048);
        unsafe {
            std::env::remove_var("INKLOG_PERFORMANCE_WORKER_THREADS");
            std::env::remove_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_strict() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "strict");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Strict
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_warn() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "warn");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Warn
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_masking_and_fallback() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_MASKING_ENABLED", "false");
            std::env::set_var("INKLOG_GLOBAL_AUTO_FALLBACK", "false");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(!config.global.masking_enabled);
        assert!(!config.global.auto_fallback);
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_MASKING_ENABLED");
            std::env::remove_var("INKLOG_GLOBAL_AUTO_FALLBACK");
        }
    }

    #[test]
    fn test_load_sync_returns_config() {
        // Must hold ENV_MUTEX to ensure INKLOG_CONFIG_PATH is not set by another test
        let _lock = ENV_MUTEX.lock().unwrap();
        // load_sync calls from_search_paths which falls back to default
        let config = InklogConfig::load_sync().unwrap();
        assert_eq!(config.global.level, "info");
    }

    #[test]
    fn test_load_with_env_overrides_applies() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "debug");
        }
        let config = InklogConfig::load_with_env_overrides().unwrap();
        assert_eq!(config.global.level, "debug");
        // Cleanup is guaranteed by _lock Drop (mutex release)
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_from_search_paths_rejects_oversized_config() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("oversized.toml");
        // Create a file > 1MB
        let oversized = vec![b'#'; 1024 * 1024 + 1];
        std::fs::write(&config_path, oversized).unwrap();

        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds maximum size")
        );
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
    }

    #[test]
    fn test_sinks_enabled_with_database() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            ..Default::default()
        });
        let sinks = config.sinks_enabled();
        assert!(sinks.contains(&"console"));
        assert!(sinks.contains(&"database"));
    }

    #[test]
    fn test_normalize_with_database_sink() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            batch_size: 0,
            ..Default::default()
        });
        // normalize should call db.validate() without panic
        config.normalize();
    }

    #[test]
    fn test_apply_env_overrides_file_sink_enabled() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_ENABLED", "true");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert!(config.file_sink.as_ref().unwrap().enabled);
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_ENABLED");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_max_size_valid() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "100MB");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert_eq!(config.file_sink.as_ref().unwrap().max_size, "100MB");
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_max_size_invalid() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "INVALID");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // Invalid size should be ignored, file_sink should remain None
        assert!(config.file_sink.is_none());
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_metrics_and_health_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/custom/metrics");
            std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/custom/health");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        let http = config.http_server.as_ref().unwrap();
        assert_eq!(http.metrics_path, "/custom/metrics");
        assert_eq!(http.health_path, "/custom/health");
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
            std::env::remove_var("INKLOG_HTTP_SERVER_HEALTH_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_unknown_keeps_current() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "unknown_mode");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // Unknown mode should keep current value (default is Strict)
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Strict
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_database_sink_url() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_DATABASE_SINK_URL", "postgres://localhost/testdb");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.database_sink.is_some());
        assert_eq!(
            config.database_sink.as_ref().unwrap().url,
            "postgres://localhost/testdb"
        );
        unsafe {
            std::env::remove_var("INKLOG_DATABASE_SINK_URL");
        }
    }

    #[test]
    fn test_apply_env_overrides_database_sink_pool_size() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_DATABASE_SINK_POOL_SIZE", "20");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.database_sink.is_some());
        assert_eq!(config.database_sink.as_ref().unwrap().pool_size, 20);
        unsafe {
            std::env::remove_var("INKLOG_DATABASE_SINK_POOL_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_global_format_empty() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_FORMAT", "");
        }
        let mut config = InklogConfig::default();
        let original_format = config.global.format.clone();
        InklogConfig::apply_env_overrides(&mut config);
        // Empty format should be ignored, keeping original value
        assert_eq!(config.global.format, original_format);
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_FORMAT");
        }
    }

    #[test]
    fn test_validate_invalid_stderr_levels() {
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            stderr_levels: vec!["error".into(), "bogus_level".into()],
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("bogus_level"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_file_sink_retained_with_invalid_max_size() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/test.log");
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "INVALID");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // file_sink should be created with the path, even though max_size is invalid
        assert!(config.file_sink.is_some());
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("/tmp/test.log")
        );
        // max_size should remain default ("100MB") since INVALID was rejected
        assert_eq!(config.file_sink.as_ref().unwrap().max_size, "100MB");
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("/%2E%2E/etc"), "/../etc");
        assert_eq!(percent_decode("/a%5Cb"), "/a\\b");
        assert_eq!(percent_decode("/a%0Ab"), "/a\nb");
        // 非法转义序列按原样保留
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%2"), "%2");
        assert_eq!(percent_decode("%2z"), "%2z");
        assert_eq!(percent_decode("/plain/path"), "/plain/path");
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_traversal_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/%2E%2E/etc/passwd");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后为 "/../etc/passwd"，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_backslash_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/a%5Cb");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后含 '\'，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_HEALTH_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_control_char_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/metrics%0A");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后含控制字符，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
        }
    }

    #[test]
    fn test_validate_rejects_oversized_channel_capacity() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = MAX_CHANNEL_CAPACITY + 1;
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("channel_capacity"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_zero_pool_size_non_sqlite_db() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::PostgreSQL,
            pool_size: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("pool_size"),
            "unexpected error: {err}"
        );

        // SQLite 不受此守卫约束（validate 会强制为 1）
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            pool_size: 0,
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_file_sink_invalid_max_size_rejected() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            max_size: "INVALID".to_string(),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("max_size"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_file_sink_encrypt_requires_existing_env() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            encrypt: true,
            encryption_key_env: Some("INKLOG_TEST_CONFIG_ENCRYPT_KEY".to_string()),
            ..Default::default()
        });

        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY") };
        assert!(config.validate().is_err());

        // SAFETY: test-only env var mutation
        unsafe { std::env::set_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY", "test-key") };
        assert!(config.validate().is_ok());

        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY") };
    }

    #[test]
    fn test_validate_http_tls_paths() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "-----BEGIN CERTIFICATE-----").unwrap();
        std::fs::write(&key, "-----BEGIN PRIVATE KEY-----").unwrap();

        // 证书/密钥文件缺失时拒绝
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            tls: Some(crate::config::http::TlsConfig {
                cert_path: "/nonexistent/cert.pem".to_string(),
                key_path: key.to_string_lossy().to_string(),
            }),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("cert_path"),
            "unexpected error: {err}"
        );

        // 文件齐备时通过
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            tls: Some(crate::config::http::TlsConfig {
                cert_path: cert.to_string_lossy().to_string(),
                key_path: key.to_string_lossy().to_string(),
            }),
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_http_ip_whitelist_entries() {
        // 合法 IP / CIDR / `前缀.*` 通配条目通过
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            ip_whitelist: Some(vec![
                "10.0.0.1".to_string(),
                "192.168.1.0/24".to_string(),
                "2001:db8::1".to_string(),
                "10.*".to_string(),
                "192.168.*".to_string(),
            ]),
            ..Default::default()
        });
        assert!(config.validate().is_ok());

        // 非法条目拒绝，错误消息含条目原文与序号
        for bad in ["10.0.0.0/33", "not-an-ip"] {
            let mut config = InklogConfig::default();
            config.http_server = Some(HttpServerConfig {
                enabled: true,
                ip_whitelist: Some(vec!["10.0.0.1".to_string(), bad.to_string()]),
                ..Default::default()
            });
            let err = config.validate().unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("ip_whitelist[1]") && msg.contains(bad),
                "unexpected error for {bad}: {msg}"
            );
        }
    }

    #[test]
    fn test_is_valid_prefix_wildcard_entry() {
        // 1-3 段点分十进制前缀均合法
        assert!(is_valid_prefix_wildcard_entry("10.*"));
        assert!(is_valid_prefix_wildcard_entry("192.168.*"));
        assert!(is_valid_prefix_wildcard_entry("192.168.1.*"));

        // 超过 3 段（完整 IP 加 ".*"）不合法
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.1.*"));
        // 非数字段
        assert!(!is_valid_prefix_wildcard_entry("abc.*"));
        assert!(!is_valid_prefix_wildcard_entry("10.x.*"));
        // 段越界（>255）
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.300.*"));
        assert!(!is_valid_prefix_wildcard_entry("256.*"));
        // 含 CIDR 分隔符
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.0/24.*"));
        assert!(!is_valid_prefix_wildcard_entry("10.0/24.*"));
        // 空前缀 / 空段
        assert!(!is_valid_prefix_wildcard_entry(".*"));
        assert!(!is_valid_prefix_wildcard_entry("10..*"));
        // 非通配形式一律不由此函数判定
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.1"));
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.0/24"));
    }

    #[test]
    fn test_validate_http_ip_whitelist_prefix_wildcard_entries() {
        // `10.*` / `192.168.*` 与运行期 whitelist_entry_matches 语法一致，须通过校验
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            ip_whitelist: Some(vec!["10.*".to_string(), "192.168.*".to_string()]),
            ..Default::default()
        });
        assert!(config.validate().is_ok());

        // 非法通配形式拒绝，错误消息含条目原文与序号
        for bad in [
            "abc.*",
            "10.0.0.300.*",
            "10.0.0.0/24.*",
            ".*",
            "10.0.0.1.*",
            "-1.*",
        ] {
            let mut config = InklogConfig::default();
            config.http_server = Some(HttpServerConfig {
                enabled: true,
                ip_whitelist: Some(vec![bad.to_string()]),
                ..Default::default()
            });
            let err = config.validate().unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("ip_whitelist[0]") && msg.contains(bad),
                "unexpected error for {bad}: {msg}"
            );
        }
    }

    #[test]
    fn test_from_search_paths_rejects_invalid_config_values() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("invalid.toml");
        std::fs::write(
            &config_path,
            r#"
[global]
level = "verbose"
"#,
        )
        .unwrap();

        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
        assert!(
            result.is_err(),
            "config with invalid level must be rejected"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log level"),
            "expected invalid log level error"
        );
    }

    #[test]
    fn test_from_str_rejects_invalid_values() {
        // channel_capacity = 0 无法自动修正，必须报错
        let result: Result<InklogConfig, _> = "[performance]\nchannel_capacity = 0\n".parse();
        assert!(result.is_err());

        // 非法日志级别必须报错
        let result: Result<InklogConfig, _> = "[global]\nlevel = \"verbose\"\n".parse();
        assert!(result.is_err());
    }
}
