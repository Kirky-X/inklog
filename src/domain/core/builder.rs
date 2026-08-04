// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Logger builder and dependency injection types.

use super::LoggerManager;
use crate::InklogError;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::integrations::Database;
use crate::integrations::{Cache, Config};
use crate::{ConsoleSinkConfig, FileSinkConfig, InklogConfig};
use std::sync::Arc;

// ============================================================================
// LoggerDependencies - Dependency injection container
// ============================================================================

/// LoggerManager 的依赖集合
///
/// 用于依赖注入模式，允许外部提供缓存、配置和数据库实现。
/// 所有字段都是可选的，未提供的依赖将使用默认实现。
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use inklog::{LoggerManager, LoggerDependencies};
/// use inklog::infrastructure::{MockCache, MockConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let deps = LoggerDependencies {
///         cache: Some(Arc::new(MockCache::new())),
///         config: Some(Arc::new(MockConfig::new())),
///         #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
///         database: None,
///     };
///     let logger = LoggerManager::with_dependencies(deps).await?;
///     Ok(())
/// }
/// ```
#[derive(Default)]
pub struct LoggerDependencies {
    /// 缓存依赖（可选）
    ///
    /// 用于缓存日志元数据、配置值等。
    /// 如果未提供，LoggerManager 将创建默认的内存缓存。
    pub cache: Option<Arc<dyn Cache>>,

    /// 配置依赖（可选）
    ///
    /// 用于动态获取配置值，支持运行时配置更新。
    /// 如果未提供，LoggerManager 将从文件系统加载配置。
    pub config: Option<Arc<dyn Config>>,

    /// 数据库依赖（可选，仅当启用 dbnexus feature 时）
    ///
    /// 用于日志记录的持久化存储。
    /// 如果未提供但配置了数据库 sink，LoggerManager 将创建默认连接池。
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub database: Option<Arc<dyn Database>>,
}

impl std::fmt::Debug for LoggerDependencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("LoggerDependencies");
        builder
            .field("cache", &self.cache.as_ref().map(|_| "Arc<dyn Cache>"))
            .field("config", &self.config.as_ref().map(|_| "Arc<dyn Config>"));
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        builder.field(
            "database",
            &self.database.as_ref().map(|_| "Arc<dyn Database>"),
        );
        builder.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_valid_level() {
        let builder = LoggerBuilder::new().level("debug");
        assert!(builder.validation_errors.is_empty());
        assert_eq!(builder.config.global.level, "debug");
    }

    #[test]
    fn test_builder_invalid_level() {
        let builder = LoggerBuilder::new().level("invalid_level");
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.validation_errors[0].contains("Invalid log level"));
    }

    #[test]
    fn test_builder_case_insensitive_level() {
        let builder = LoggerBuilder::new().level("DEBUG");
        assert!(builder.validation_errors.is_empty());
    }

    #[test]
    fn test_builder_multiple_errors() {
        let builder = LoggerBuilder::new().level("bad1").level("bad2");
        assert_eq!(builder.validation_errors.len(), 2);
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_port_zero() {
        let builder = LoggerBuilder::new().http_port(0);
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.validation_errors[0].contains("HTTP port"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_port_valid() {
        let builder = LoggerBuilder::new().http_port(8080);
        assert!(builder.validation_errors.is_empty());
    }

    #[tokio::test]
    async fn test_builder_build_fails_with_validation_errors() {
        let result = LoggerBuilder::new().level("invalid").build().await;
        assert!(result.is_err());
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("Builder validation failed"));
                assert!(msg.contains("Invalid log level"));
            }
            Ok(_) => panic!("Expected build to fail with validation errors"),
        }
    }

    #[test]
    fn test_builder_file_when_no_file_sink() {
        let mut builder = LoggerBuilder::new();
        builder.config.file_sink = None;
        builder = builder.file("/tmp/test.log");
        assert!(builder.config.file_sink.is_some());
        let file_cfg = builder.config.file_sink.as_ref().unwrap();
        assert!(file_cfg.enabled);
        assert_eq!(file_cfg.path, std::path::PathBuf::from("/tmp/test.log"));
    }

    #[test]
    fn test_builder_file_when_file_sink_exists() {
        let mut builder = LoggerBuilder::new();
        builder.config.file_sink = Some(FileSinkConfig::default());
        builder = builder.file("/tmp/updated.log");
        let file_cfg = builder.config.file_sink.as_ref().unwrap();
        assert!(file_cfg.enabled);
        assert_eq!(file_cfg.path, std::path::PathBuf::from("/tmp/updated.log"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_unknown() {
        let builder = LoggerBuilder::new().http_error_mode("invalid_mode");
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.validation_errors[0].contains("Unknown HTTP error mode"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_warn() {
        let builder = LoggerBuilder::new().http_error_mode("warn");
        assert!(builder.validation_errors.is_empty());
        assert!(matches!(
            builder.config.http_server.as_ref().unwrap().error_mode,
            crate::HttpErrorMode::Warn
        ));
    }
}

// ============================================================================
// LoggerBuilder - Fluent builder API
// ============================================================================

/// Logger 构建器，支持链式配置和依赖注入
///
/// 支持两种配置模式：
/// 1. **纯配置模式**：通过 `.level()`, `.file()` 等方法配置
/// 2. **依赖注入模式**：通过 `.cache()`, `.config()`, `.database()` 注入实现
/// 3. **混合模式**：同时使用配置和依赖注入
///
/// # 示例
///
/// ## 纯配置模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .level("debug")
///     .file("logs/app.log")
///     .build().await?;
/// ```
///
/// ## 依赖注入模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .cache(Arc::new(MockCache::new()))
///     .config(Arc::new(MockConfig::new()))
///     .build().await?;
/// ```
///
/// ## 混合模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .level("debug")
///     .cache(Arc::new(MockCache::new()))  // 使用自定义缓存，其他用配置
///     .build().await?;
/// ```
#[derive(Default)]
pub struct LoggerBuilder {
    pub(crate) config: InklogConfig,
    pub(crate) deps: LoggerDependencies,
    /// Accumulated validation errors for deferred reporting at `build()` time.
    pub(crate) validation_errors: Vec<String>,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        let level_str = level.into();
        if !crate::LogLevel::is_valid_level(&level_str) {
            self.validation_errors.push(format!(
                "Invalid log level '{}'. Valid levels: {}",
                level_str,
                crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
            ));
        }
        self.config.global.level = level_str;
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.config.global.format = format.into();
        self
    }

    pub fn console(mut self, enabled: bool) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.enabled = enabled;
        } else if enabled {
            self.config.console_sink = Some(ConsoleSinkConfig::default());
        }
        self
    }

    pub fn file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.enabled = true;
            file.path = path.into();
        } else {
            let path_buf = path.into();
            self.config.file_sink = Some(FileSinkConfig {
                enabled: true,
                path: path_buf,
                ..Default::default()
            });
        }
        self
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub fn database(mut self, url: impl Into<String>) -> Self {
        let url_str = url.into();
        let config = crate::DatabaseSinkConfig {
            name: "default".to_string(),
            enabled: true,
            driver: crate::DatabaseDriver::default(),
            url: url_str,
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 500,
            partition: crate::PartitionStrategy::default(),
            table_name: "logs".to_string(),
            archive_format: crate::ArchiveFormat::default(),
            parquet_config: crate::ParquetConfig::default(),
        };
        self.config.database_sink = Some(config);
        self
    }

    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.config.performance.channel_capacity = capacity;
        self
    }

    pub fn worker_threads(mut self, threads: usize) -> Self {
        self.config.performance.worker_threads = threads;
        self
    }

    // === Console 配置快捷方法 ===

    pub fn console_colored(mut self, colored: bool) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.colored = colored;
        } else if colored {
            self.config.console_sink = Some(ConsoleSinkConfig {
                colored,
                ..Default::default()
            });
        }
        self
    }

    pub fn console_stderr_levels(mut self, levels: &[&str]) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.stderr_levels = levels.iter().map(|s| (*s).to_string()).collect();
        } else {
            self.config.console_sink = Some(ConsoleSinkConfig {
                stderr_levels: levels.iter().map(|s| (*s).to_string()).collect(),
                ..Default::default()
            });
        }
        self
    }

    // === File 配置快捷方法 ===

    pub fn file_max_size(mut self, max_size: impl Into<String>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.max_size = max_size.into();
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                max_size: max_size.into(),
                ..Default::default()
            });
        }
        self
    }

    pub fn file_compress(mut self, compress: bool) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.compress = compress;
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                compress,
                ..Default::default()
            });
        }
        self
    }

    pub fn file_rotation_time(mut self, rotation: impl Into<String>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.rotation_time = rotation.into();
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                rotation_time: rotation.into(),
                ..Default::default()
            });
        }
        self
    }

    pub fn file_keep_files(mut self, keep: u32) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.keep_files = keep;
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                keep_files: keep,
                ..Default::default()
            });
        }
        self
    }

    // === HTTP Server 配置快捷方法 ===

    /// 启用或禁用HTTP监控服务器
    ///
    /// # Arguments
    /// * `enabled` - 是否启用HTTP服务器
    ///
    /// # Example
    /// ```ignore
    /// let _logger = LoggerManager::builder()
    ///     .enable_http_server(true)
    ///     .build()
    ///     .await?;
    /// ```
    #[cfg(feature = "http")]
    pub fn enable_http_server(mut self, enabled: bool) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.enabled = enabled;
        } else if enabled {
            self.config.http_server = Some(crate::HttpServerConfig {
                enabled: true,
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器监听主机
    ///
    /// # Arguments
    /// * `host` - 监听主机地址（如 "127.0.0.1" 或 "0.0.0.0"）
    #[cfg(feature = "http")]
    pub fn http_host(mut self, host: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.host = host.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                host: host.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器监听端口
    ///
    /// # Arguments
    /// * `port` - 监听端口号 (1-65535)
    #[cfg(feature = "http")]
    pub fn http_port(mut self, port: u16) -> Self {
        if port == 0 {
            self.validation_errors
                .push("HTTP port must be between 1 and 65535".to_string());
        }
        if let Some(ref mut http) = self.config.http_server {
            http.port = port;
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                port,
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器指标路径
    ///
    /// # Arguments
    /// * `path` - Prometheus指标端点路径（默认 "/metrics"）
    #[cfg(feature = "http")]
    pub fn http_metrics_path(mut self, path: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.metrics_path = path.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                metrics_path: path.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器健康检查路径
    ///
    /// # Arguments
    /// * `path` - 健康检查端点路径（默认 "/health"）
    #[cfg(feature = "http")]
    pub fn http_health_path(mut self, path: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.health_path = path.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                health_path: path.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器错误处理模式
    ///
    /// # Arguments
    /// * `mode` - 错误处理模式（"warn" 或 "strict"）。未知模式会记录验证错误并回退到默认值 "strict"。
    #[cfg(feature = "http")]
    pub fn http_error_mode(mut self, mode: impl Into<String>) -> Self {
        let mode_str = mode.into();
        let error_mode = match mode_str.to_lowercase().as_str() {
            "warn" => crate::HttpErrorMode::Warn,
            "strict" => crate::HttpErrorMode::Strict,
            _ => {
                self.validation_errors.push(format!(
                    "Unknown HTTP error mode '{}'. Valid modes: warn, strict. Using default 'strict'.",
                    mode_str
                ));
                crate::HttpErrorMode::default()
            }
        };
        if let Some(ref mut http) = self.config.http_server {
            http.error_mode = error_mode;
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                error_mode,
                ..Default::default()
            });
        }
        self
    }

    // === 依赖注入方法 ===

    /// 注入自定义 Cache 实现
    ///
    /// 用于测试场景或需要自定义缓存行为的场景。
    /// 如果未调用此方法，LoggerManager 将创建默认的内存缓存。
    ///
    /// # Arguments
    /// * `cache` - 实现 `Cache` trait 的缓存实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockCache;
    ///
    /// let logger = LoggerManager::builder()
    ///     .cache(Arc::new(MockCache::new()))
    ///     .build().await?;
    /// ```
    pub fn cache(mut self, cache: Arc<dyn Cache>) -> Self {
        self.deps.cache = Some(cache);
        self
    }

    /// 注入自定义 Config 实现
    ///
    /// 用于动态配置场景，允许运行时更新配置值。
    /// 如果未调用此方法，LoggerManager 将从文件系统加载配置。
    ///
    /// # Arguments
    /// * `config` - 实现 `Config` trait 的配置实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockConfig;
    ///
    /// let logger = LoggerManager::builder()
    ///     .config(Arc::new(MockConfig::new()))
    ///     .build().await?;
    /// ```
    pub fn config(mut self, config: Arc<dyn Config>) -> Self {
        self.deps.config = Some(config);
        self
    }

    /// 注入自定义 Database 实现
    ///
    /// 用于数据库 sink 的自定义连接管理。
    /// 如果未调用此方法但配置了数据库 sink，LoggerManager 将创建默认连接池。
    ///
    /// # Arguments
    /// * `database` - 实现 `Database` trait 的数据库实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockDatabaseAdapter;
    ///
    /// let logger = LoggerManager::builder()
    ///     .with_database(Arc::new(MockDatabaseAdapter::new()))
    ///     .build().await?;
    /// ```
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub fn with_database(mut self, database: Arc<dyn Database>) -> Self {
        self.deps.database = Some(database);
        self
    }

    /// 构建 LoggerManager 实例
    ///
    /// 根据配置和注入的依赖创建 LoggerManager。
    /// 优先使用注入的依赖，未注入的依赖将使用配置创建默认实现。
    ///
    /// # Returns
    /// 成功返回 `Ok(LoggerManager)`，失败返回 `Err(InklogError)`
    pub async fn build(self) -> Result<LoggerManager, InklogError> {
        // Report all accumulated validation errors at once
        if !self.validation_errors.is_empty() {
            return Err(InklogError::ConfigError(format!(
                "Builder validation failed with {} error(s):\n  - {}",
                self.validation_errors.len(),
                self.validation_errors.join("\n  - ")
            )));
        }

        // 如果有任何注入的依赖，使用 with_dependencies
        let has_deps = self.deps.cache.is_some() || self.deps.config.is_some() || {
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            {
                self.deps.database.is_some()
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            {
                false
            }
        };

        if has_deps {
            // 有依赖注入，使用 with_dependencies
            // 但需要先把 config 中的配置应用到 deps.config
            let mut deps = self.deps;

            // 如果注入了 Config trait，将 InklogConfig 的值应用到它
            // 注意：这里我们不覆盖已注入的 config，因为用户明确注入了
            // 但我们可以保留 self.config 用于其他配置项

            // 如果没有注入 config，但有其他注入，我们需要创建一个包含 self.config 的 deps
            if deps.config.is_none() {
                // 将 self.config 通过 InklogConfigAdapter 注入
                // 这允许 mixed mode 正常工作
                deps.config = Some(Arc::new(
                    crate::integrations::infra::InklogConfigAdapter::from_config(
                        self.config.clone(),
                    ),
                ));
            }

            LoggerManager::with_dependencies(deps).await
        } else {
            // 纯配置模式
            LoggerManager::with_config(self.config).await
        }
    }
}
