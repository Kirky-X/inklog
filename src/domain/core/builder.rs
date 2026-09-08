// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Logger builder and dependency injection types.

use super::LoggerManager;
use crate::InklogError;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
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
///         #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
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
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub database: Option<Arc<dyn Database>>,
}

impl std::fmt::Debug for LoggerDependencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("LoggerDependencies");
        builder
            .field("cache", &self.cache.as_ref().map(|_| "Arc<dyn Cache>"))
            .field("config", &self.config.as_ref().map(|_| "Arc<dyn Config>"));
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
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

    #[test]
    fn test_builder_invalid_level_keeps_default_value() {
        // 校验失败时不把非法值写入配置，保持默认 "info"
        let builder = LoggerBuilder::new().level("not-a-level");
        assert_eq!(builder.validation_errors.len(), 1);
        assert_eq!(builder.config.global.level, "info");
    }

    #[test]
    fn test_builder_invalid_level_keeps_previous_value() {
        // 合法值先设置，随后非法值不应覆盖
        let builder = LoggerBuilder::new().level("debug").level("not-a-level");
        assert_eq!(builder.validation_errors.len(), 1);
        assert_eq!(builder.config.global.level, "debug");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_port_zero_does_not_write_config() {
        // 非法端口不写入配置：http_server 保持 None（未被创建）
        let builder = LoggerBuilder::new().http_port(0);
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.config.http_server.is_none());
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_port_zero_does_not_overwrite_existing() {
        // 非法端口不应覆盖之前设置的合法端口
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_port(8080)
            .http_port(0);
        assert_eq!(builder.validation_errors.len(), 1);
        assert_eq!(builder.config.http_server.as_ref().unwrap().port, 8080);
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
                assert!(
                    msg.contains("Builder validation failed") || msg.contains("构建器验证失败")
                );
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

    // =========================================================================
    // database() 配置 setter 与 driver 推断测试（需要 db 后端 feature）
    // =========================================================================

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_pool_size_sets_config() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_pool_size(5);
        let db = builder.config.database_sink.as_ref().unwrap();
        assert_eq!(db.pool_size, 5);
        assert!(db.enabled);
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_pool_size_creates_config_when_absent() {
        let builder = LoggerBuilder::new().with_pool_size(3);
        let db = builder.config.database_sink.as_ref().unwrap();
        assert_eq!(db.pool_size, 3);
        // 仅 setter 不启用 db sink
        assert!(!db.enabled);
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_batch_size_sets_config() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_batch_size(50);
        assert_eq!(builder.config.database_sink.as_ref().unwrap().batch_size, 50);
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_flush_interval_ms_sets_config() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_flush_interval_ms(250);
        assert_eq!(
            builder.config.database_sink.as_ref().unwrap().flush_interval_ms,
            250
        );
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_table_name_sets_config() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_table_name("app_logs");
        assert_eq!(
            builder.config.database_sink.as_ref().unwrap().table_name,
            "app_logs"
        );
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_admin_role_sets_config() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_admin_role("log_admin");
        assert_eq!(
            builder.config.database_sink.as_ref().unwrap().admin_role,
            "log_admin"
        );
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_driver_sets_config() {
        let builder = LoggerBuilder::new()
            .database("sqlite::memory:")
            .with_driver(crate::DatabaseDriver::MySQL);
        let db = builder.config.database_sink.as_ref().unwrap();
        assert!(matches!(db.driver, crate::DatabaseDriver::MySQL));
        // driver 已显式设置
        assert!(builder.db_driver_explicit);
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_database_infers_postgres_driver() {
        // URL 可推断时使用推断值（旧实现恒为默认 SQLite）
        let builder = LoggerBuilder::new().database("postgres://localhost/logs");
        let db = builder.config.database_sink.as_ref().unwrap();
        assert!(matches!(db.driver, crate::DatabaseDriver::PostgreSQL));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_database_infers_mysql_driver() {
        let builder = LoggerBuilder::new().database("mysql://localhost/logs");
        assert!(matches!(
            builder.config.database_sink.as_ref().unwrap().driver,
            crate::DatabaseDriver::MySQL
        ));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_database_infers_sqlite_memory_driver() {
        // "sqlite::memory:" 无 "//"，scheme 解析为 "sqlite"
        let builder = LoggerBuilder::new().database("sqlite::memory:");
        assert!(matches!(
            builder.config.database_sink.as_ref().unwrap().driver,
            crate::DatabaseDriver::SQLite
        ));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_database_unknown_scheme_keeps_default_driver() {
        // 无法推断时保持默认值（SQLite）
        let builder = LoggerBuilder::new().database("custom-backend://localhost");
        let db = builder.config.database_sink.as_ref().unwrap();
        assert!(matches!(db.driver, crate::DatabaseDriver::SQLite));
        assert!(builder.validation_errors.is_empty());
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_infer_driver_common_schemes() {
        use crate::DatabaseDriver;
        assert_eq!(LoggerBuilder::infer_driver("postgres://h/db"), Some(DatabaseDriver::PostgreSQL));
        assert_eq!(
            LoggerBuilder::infer_driver("postgresql://h/db"),
            Some(DatabaseDriver::PostgreSQL)
        );
        assert_eq!(LoggerBuilder::infer_driver("mysql://h/db"), Some(DatabaseDriver::MySQL));
        assert_eq!(LoggerBuilder::infer_driver("sqlite://f.db"), Some(DatabaseDriver::SQLite));
        assert_eq!(LoggerBuilder::infer_driver("sqlite3://f.db"), Some(DatabaseDriver::SQLite));
        assert_eq!(LoggerBuilder::infer_driver("sqlite::memory:"), Some(DatabaseDriver::SQLite));
        assert_eq!(LoggerBuilder::infer_driver("duckdb://f.db"), Some(DatabaseDriver::DuckDB));
        // 大小写不敏感
        assert_eq!(LoggerBuilder::infer_driver("POSTGRES://h/db"), Some(DatabaseDriver::PostgreSQL));
        // 无法识别
        assert_eq!(LoggerBuilder::infer_driver("weird://x"), None);
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_with_driver_conflicting_url_records_error() {
        // 显式 driver 与 URL 推断冲突 → validation error
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .with_driver(crate::DatabaseDriver::SQLite);
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.validation_errors[0].contains("conflicts with"));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_builder_database_conflicting_explicit_driver_records_error() {
        // 先显式设置 driver，再提供冲突 scheme 的 URL → validation error
        let builder = LoggerBuilder::new()
            .with_driver(crate::DatabaseDriver::MySQL)
            .database("postgres://localhost/logs");
        assert_eq!(builder.validation_errors.len(), 1);
        assert!(builder.validation_errors[0].contains("conflicts with"));
        // URL 可推断时推断值生效
        assert!(matches!(
            builder.config.database_sink.as_ref().unwrap().driver,
            crate::DatabaseDriver::PostgreSQL
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
    /// 是否通过 `with_driver` 显式设置过数据库 driver
    /// （用于与 URL scheme 推断结果做冲突校验）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub(crate) db_driver_explicit: bool,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置全局日志级别。
    ///
    /// 非法级别会记录 validation error（`build()` 时统一返回 `Err`），
    /// 且不会写入配置——配置字段保持原值/默认值。
    pub fn level(mut self, level: impl Into<String>) -> Self {
        let level_str = level.into();
        if !crate::LogLevel::is_valid_level(&level_str) {
            self.validation_errors.push(format!(
                "Invalid log level '{}'. Valid levels: {}",
                level_str,
                crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
            ));
            return self;
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

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn database(mut self, url: impl Into<String>) -> Self {
        let url_str = url.into();
        let inferred = Self::infer_driver(&url_str);
        // 显式设置过 driver 且与 URL scheme 推断结果冲突时记录 validation error
        if self.db_driver_explicit
            && let Some(ref db) = self.config.database_sink
            && let Some(ref target) = inferred
            && db.driver != *target
        {
            self.validation_errors.push(format!(
                "Database driver '{}' (explicitly set) conflicts with driver '{}' inferred from URL '{}'. The inferred driver takes precedence.",
                db.driver, target, url_str
            ));
        }
        if let Some(ref mut db) = self.config.database_sink {
            db.enabled = true;
            db.url = url_str;
            if let Some(target) = inferred {
                db.driver = target;
            }
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                enabled: true,
                url: url_str,
                driver: inferred.unwrap_or_default(),
                ..Default::default()
            });
        }
        self
    }

    /// 根据 URL scheme 推断数据库 driver。
    ///
    /// 与 `DatabaseDriver` 的 `FromStr` 接受的名字保持一致：
    /// postgres/postgresql、mysql、sqlite/sqlite3、duckdb。
    /// 无法识别的 scheme 返回 `None`，调用方保持默认值。
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    fn infer_driver(url: &str) -> Option<crate::DatabaseDriver> {
        let scheme = url.split(':').next()?;
        match scheme.to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Some(crate::DatabaseDriver::PostgreSQL),
            "mysql" => Some(crate::DatabaseDriver::MySQL),
            "sqlite" | "sqlite3" => Some(crate::DatabaseDriver::SQLite),
            "duckdb" => Some(crate::DatabaseDriver::DuckDB),
            _ => None,
        }
    }

    // === Database 配置快捷方法 ===

    /// 显式设置数据库 driver。
    ///
    /// 若与已配置 URL 的 scheme 推断结果冲突，会记录 validation error
    /// （`build()` 时统一返回 `Err`）。
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_driver(mut self, driver: crate::DatabaseDriver) -> Self {
        if let Some(ref db) = self.config.database_sink
            && let Some(target) = Self::infer_driver(&db.url)
            && target != driver
        {
            self.validation_errors.push(format!(
                "Database driver '{}' conflicts with driver '{}' inferred from URL '{}'.",
                driver, target, db.url
            ));
        }
        self.db_driver_explicit = true;
        if let Some(ref mut db) = self.config.database_sink {
            db.driver = driver;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                driver,
                ..Default::default()
            });
        }
        self
    }

    /// 设置数据库连接池大小（默认 10）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_pool_size(mut self, pool_size: u32) -> Self {
        if let Some(ref mut db) = self.config.database_sink {
            db.pool_size = pool_size;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                pool_size,
                ..Default::default()
            });
        }
        self
    }

    /// 设置数据库批量写入大小（默认 100）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        if let Some(ref mut db) = self.config.database_sink {
            db.batch_size = batch_size;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                batch_size,
                ..Default::default()
            });
        }
        self
    }

    /// 设置数据库 flush 间隔（毫秒，默认 500）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_flush_interval_ms(mut self, flush_interval_ms: u64) -> Self {
        if let Some(ref mut db) = self.config.database_sink {
            db.flush_interval_ms = flush_interval_ms;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                flush_interval_ms,
                ..Default::default()
            });
        }
        self
    }

    /// 设置数据库日志表名（默认 "logs"）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        let table_name = table_name.into();
        if let Some(ref mut db) = self.config.database_sink {
            db.table_name = table_name;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                table_name,
                ..Default::default()
            });
        }
        self
    }

    /// 设置数据库管理员角色名（默认 "admin"）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub fn with_admin_role(mut self, admin_role: impl Into<String>) -> Self {
        let admin_role = admin_role.into();
        if let Some(ref mut db) = self.config.database_sink {
            db.admin_role = admin_role;
        } else {
            self.config.database_sink = Some(crate::DatabaseSinkConfig {
                admin_role,
                ..Default::default()
            });
        }
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
    ///
    /// 非法端口（0）会记录 validation error（`build()` 时统一返回 `Err`），
    /// 且不会写入配置——配置字段保持原值/默认值。
    #[cfg(feature = "http")]
    pub fn http_port(mut self, port: u16) -> Self {
        if port == 0 {
            self.validation_errors
                .push("HTTP port must be between 1 and 65535".to_string());
            return self;
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
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
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
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("count", self.validation_errors.len());
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-builder_validation_failed",
                args,
            )));
        }

        // 如果有任何注入的依赖，使用 with_dependencies
        let has_deps = self.deps.cache.is_some() || self.deps.config.is_some() || {
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            {
                self.deps.database.is_some()
            }
            #[cfg(not(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            )))]
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
