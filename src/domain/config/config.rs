// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Root configuration struct and loading logic.

use crate::InklogError;
use serde::{Deserialize, Serialize};

use super::console::ConsoleSinkConfig;
use super::database::DatabaseSinkConfig;
use super::file_sink::FileSinkConfig;
use super::global::GlobalConfig;
use super::http::HttpServerConfig;
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
    pub fn load_sync() -> Result<Self, InklogError> {
        Self::from_search_paths()
            .map_err(|e| InklogError::ConfigError(format!("Failed to load config: {}", e)))
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
        if let Ok(val) = std::env::var("INKLOG_FILE_SINK_ENABLED")
            && val.parse::<bool>().unwrap_or(false)
        {
            let file_config = config.file_sink.get_or_insert_with(Default::default);
            file_config.enabled = true;
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
                "strict" => HttpErrorMode::Strict,
                "warn" => HttpErrorMode::Warn,
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

    /// Search paths (first existing file wins):
    /// 1. `$INKLOG_CONFIG_PATH`
    /// 2. `inklog_config.toml` (current directory)
    /// 3. `~/.config/inklog/config.toml`
    /// 4. `/etc/inklog/config.toml`
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
    ///
    /// Checks for common misconfigurations and returns a descriptive error
    /// for the first problem found.
    pub fn validate(&self) -> Result<(), InklogError> {
        // --- Performance ---
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
        if self.performance.max_capacity < self.performance.min_capacity {
            return Err(InklogError::ConfigError(format!(
                "performance.max_capacity ({}) must be >= min_capacity ({})",
                self.performance.max_capacity, self.performance.min_capacity
            )));
        }

        // --- Global log level ---
        let valid_levels = ["trace", "debug", "info", "warn", "error", "fatal"];
        if !valid_levels.contains(&self.global.level.to_lowercase().as_str()) {
            return Err(InklogError::ConfigError(format!(
                "Invalid log level '{}'. Valid levels: {}",
                self.global.level,
                valid_levels.join(", ")
            )));
        }

        // --- File sink ---
        if let Some(ref file) = self.file_sink
            && file.enabled
        {
            if file.path.as_os_str().is_empty() {
                return Err(InklogError::ConfigError(
                    "file_sink.path must not be empty when file sink is enabled".to_string(),
                ));
            }
            if file.batch_size == 0 {
                return Err(InklogError::ConfigError(
                    "file_sink.batch_size must be > 0".to_string(),
                ));
            }
        }

        // --- HTTP server ---
        if let Some(ref http) = self.http_server
            && http.enabled
            && http.port == 0
        {
            return Err(InklogError::ConfigError(
                "http_server.port must be in range 1-65535".to_string(),
            ));
        }

        Ok(())
    }
}

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
