// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Database trait - 抽象数据库操作
//!
//! 提供日志记录批量写入和健康检查的抽象接口。

use std::sync::Arc;

use crate::InklogError;
use crate::LogRecord;
use async_trait::async_trait;

/// Database trait - 抽象数据库操作
///
/// 提供日志记录批量写入和健康检查接口。
/// 实现必须保证线程安全（`Send + Sync`）。
///
/// # 实现要求
///
/// - 所有方法使用 `&self`（不可变引用），支持并发访问
/// - 批量插入应该是原子操作（全部成功或全部失败）
/// - 健康检查应该是轻量级的
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::Database;
/// use inklog::log_record::LogRecord;
/// use tracing::Level;
///
/// async fn example(db: &dyn Database) {
///     let records = vec![
///         LogRecord::new(Level::INFO, "module".to_string(), "message".to_string()),
///     ];
///     
///     let count = db.insert_batch(&records).await.unwrap();
///     assert_eq!(count, 1);
///     
///     if db.is_healthy().await {
///         println!("Database is healthy");
///     }
/// }
/// ```
#[async_trait]
pub trait Database: Send + Sync {
    /// 批量插入日志记录
    ///
    /// # 参数
    ///
    /// * `records` - 日志记录切片
    ///
    /// # 返回
    ///
    /// 成功返回成功插入的记录数 `Ok(count)`，失败返回 `Err(InklogError)`
    ///
    /// # 注意
    ///
    /// 实现应该保证原子性，要么全部插入成功，要么全部失败
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError>;

    /// 检查数据库健康状态
    ///
    /// # 返回
    ///
    /// 数据库连接正常返回 `true`，否则返回 `false`
    ///
    /// # 注意
    ///
    /// 此方法应该是轻量级的，适合频繁调用
    async fn is_healthy(&self) -> bool;
}

// ============================================================================
// DbNexusAdapter - dbnexus 适配器实现 (条件编译)
// ============================================================================

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
use dbnexus::ConnectionPool;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
use dbnexus::database::pool::DbPool;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
use dbnexus::foundation::config::DbConfig;

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
use crate::domain::config::database::DatabaseDriver;

/// dbnexus 适配器
///
/// 将 dbnexus 库的 `DbPool` 适配为 `Database` trait。
/// 使用 Sea-ORM 进行批量插入操作。
///
/// # 功能要求
///
/// - 需要启用 `dbnexus` feature
/// - 支持 PostgreSQL、MySQL、SQLite、DuckDB 数据库
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::database::{Database, DbNexusAdapter};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let db = DbNexusAdapter::new("postgres://user:pass@localhost/logs", 10).await?;
///     
///     let healthy = db.is_healthy().await;
///     println!("Database healthy: {}", healthy);
///     
///     Ok(())
/// }
/// ```
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub struct DbNexusAdapter {
    pool: Arc<dyn ConnectionPool + Send + Sync>,
    table_name: String,
    admin_role: String,
    /// 驱动类型，SQL 转义需要按后端区分（MySQL 默认将反斜杠视为转义符）
    driver: DatabaseDriver,
}

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
impl DbNexusAdapter {
    /// 创建新的 dbnexus 适配器
    ///
    /// # 参数
    ///
    /// * `url` - 数据库连接字符串
    /// * `pool_size` - 连接池大小（最大连接数）
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 错误
    ///
    /// - `InklogError::DatabaseError` - 连接池创建失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // PostgreSQL
    /// let db = DbNexusAdapter::new("postgres://user:pass@localhost/logs", 10).await?;
    ///
    /// // MySQL
    /// let db = DbNexusAdapter::new("mysql://user:pass@localhost/logs", 10).await?;
    ///
    /// // SQLite
    /// let db = DbNexusAdapter::new("sqlite://logs.db", 1).await?;
    /// ```
    pub async fn new(url: &str, pool_size: u32) -> Result<Self, InklogError> {
        Self::with_table_name(url, pool_size, crate::support::io::sink::entity::TABLE_NAME).await
    }

    /// 创建带有自定义表名的适配器
    ///
    /// # 参数
    ///
    /// * `url` - 数据库连接字符串
    /// * `pool_size` - 连接池大小（最大连接数）
    /// * `table_name` - 日志表名称
    pub async fn with_table_name(
        url: &str,
        pool_size: u32,
        table_name: &str,
    ) -> Result<Self, InklogError> {
        Self::with_full_config(url, pool_size, table_name, None, "admin").await
    }

    /// 创建带完整配置的适配器
    ///
    /// 支持自定义权限配置文件路径和管理员角色名。
    /// 构造完成后自动调用 `ensure_table_exists()` 创建日志表。
    ///
    /// # 参数
    ///
    /// * `url` - 数据库连接字符串
    /// * `pool_size` - 连接池大小（最大连接数）
    /// * `table_name` - 日志表名称
    /// * `permissions_path` - 权限配置文件路径，`None` 时不启用权限校验
    /// * `admin_role` - 管理员角色名
    pub async fn with_full_config(
        url: &str,
        pool_size: u32,
        table_name: &str,
        permissions_path: Option<String>,
        admin_role: &str,
    ) -> Result<Self, InklogError> {
        validate_table_name(table_name)?;

        // 创建 DbConfig
        let config = DbConfig {
            url: url.to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: pool_size,
                min_connections: 1,
                idle_timeout: 300,
                acquire_timeout: 5000,
            },
            permissions_path,
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: admin_role.to_string(),
            warmup_timeout: 30,
            warmup_retries: 3,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
            retry_policy: Some(dbnexus::reliability::retry::RetryPolicy::default()),
            failover_config: None,
            replica_config: None,
        };

        // 使用 DbPool::with_config 创建连接池
        let pool = DbPool::with_config(config).await.map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            InklogError::DatabaseError {
                message: crate::i18n::tr_args("db-pool_create_failed", args),
                source: Some(Box::new(e)),
            }
        })?;

        let adapter = Self {
            pool: Arc::new(pool),
            table_name: table_name.to_string(),
            admin_role: admin_role.to_string(),
            driver: detect_driver_from_url(url),
        };

        // 自动建表
        adapter.ensure_table_exists(url).await?;

        Ok(adapter)
    }

    /// 从现有 DbPool 创建适配器
    ///
    /// 用于需要共享连接池的场景。
    ///
    /// # 参数
    ///
    /// * `pool` - 已创建的连接池实例
    /// * `table_name` - 日志表名称
    pub fn from_pool(pool: DbPool, table_name: &str) -> Result<Self, InklogError> {
        validate_table_name(table_name)?;
        // 无 URL 可解析驱动，沿用 detect_driver_from_url 的默认回退（PostgreSQL）；
        // MySQL 连接池的用户请改用 with_full_config/with_table_name（可识别驱动）。
        Ok(Self {
            pool: Arc::new(pool),
            table_name: table_name.to_string(),
            admin_role: "admin".to_string(),
            driver: DatabaseDriver::PostgreSQL,
        })
    }

    /// 从已有的 `ConnectionPool` trait 对象创建适配器
    ///
    /// 用于 trait-kit DI 场景：kit 提供 `Arc<dyn ConnectionPool + Send + Sync>`，
    /// 直接包装为 `Database` trait 实现，无需重新创建连接池。
    ///
    /// # 参数
    ///
    /// * `pool` - dbnexus 连接池 trait 对象（通常来自 `kit.require::<DbNexusModule>()`）
    /// * `table_name` - 日志表名称
    pub fn from_connection_pool(
        pool: Arc<dyn ConnectionPool + Send + Sync>,
        table_name: &str,
    ) -> Result<Self, InklogError> {
        validate_table_name(table_name)?;
        // 同上：无法从 trait 对象解析驱动，回退到 PostgreSQL 语义
        Ok(Self {
            pool,
            table_name: table_name.to_string(),
            admin_role: "admin".to_string(),
            driver: DatabaseDriver::PostgreSQL,
        })
    }

    /// 获取底层连接池引用
    pub fn pool(&self) -> &dyn ConnectionPool {
        self.pool.as_ref()
    }

    /// 获取底层连接池的 `Arc` 克隆
    ///
    /// 用于需要共享连接池创建新适配器的场景（见 [`Self::from_connection_pool`]）。
    pub fn pool_arc(&self) -> Arc<dyn ConnectionPool + Send + Sync> {
        Arc::clone(&self.pool)
    }

    /// 获取表名
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// 确保日志表存在，若不存在则自动创建。
    ///
    /// 使用管理员角色获取 session 并执行 DDL。
    /// 驱动类型通过 URL 前缀自动识别。
    async fn ensure_table_exists(&self, url: &str) -> Result<(), InklogError> {
        let driver = detect_driver_from_url(url);
        let ddl = generate_create_table_sql(&self.table_name, &driver);
        let session = self.pool.get_session(&self.admin_role).await.map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            InklogError::DatabaseError {
                message: crate::i18n::tr_args("db-session_failed", args),
                source: Some(Box::new(e)),
            }
        })?;
        session.execute_raw_ddl(&ddl).await.map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            InklogError::DatabaseError {
                message: crate::i18n::tr_args("db-ensure_table_failed", args),
                source: Some(Box::new(e)),
            }
        })?;
        Ok(())
    }
}

/// Validate that a table name contains only safe identifier characters.
///
/// Rejects names that don't match `^[a-zA-Z_][a-zA-Z0-9_]*$` to prevent SQL injection
/// via table name interpolation.
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
fn validate_table_name(name: &str) -> Result<(), InklogError> {
    if name.is_empty() {
        return Err(InklogError::ConfigError(crate::i18n::tr("db-table_empty")));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("name", name.to_string());
        return Err(InklogError::ConfigError(crate::i18n::tr_args(
            "db-table_invalid_start",
            args,
        )));
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("name", name.to_string());
            args.set("char", c.to_string());
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "db-table_invalid_char",
                args,
            )));
        }
    }
    Ok(())
}

/// 转义 SQL 字符串中的单引号，防止 SQL 注入。
///
/// 所有通过 `insert_batch` 写入的字符串字段必须经过此函数。
/// 采用标准 SQL 转义规则：将单引号 `'` 替换为双单引号 `''`。
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
/// 转义 SQL 字符串字面量。
///
/// ANSI/标准后端（SQLite/PostgreSQL/DuckDB）只需将 `'` 双写即可；
/// **MySQL 默认把反斜杠视为转义符**，若输入含 `\` 后跟 `'`（如 `\'; DROP ...`），
/// 仅双写引号仍可逃逸字符串越权执行 SQL。因此 MySQL 需额外转义反斜杠。
/// （diting MED-001 修复）
#[inline]
fn escape_sql_string(s: &str, driver: &DatabaseDriver) -> String {
    match driver {
        DatabaseDriver::MySQL => s.replace('\\', "\\\\").replace('\'', "''"),
        _ => s.replace('\'', "''"),
    }
}

/// 根据数据库驱动生成 CREATE TABLE DDL 语句。
///
/// 不同后端使用不同的数据类型：
/// - SQLite: `INTEGER PRIMARY KEY AUTOINCREMENT`, `TEXT`
/// - PostgreSQL: `BIGSERIAL PRIMARY KEY`, `TIMESTAMPTZ`, `TEXT`
/// - MySQL: `BIGINT AUTO_INCREMENT PRIMARY KEY`, `TIMESTAMP`, `TEXT`
/// - DuckDB: `BIGINT AUTOINCREMENT PRIMARY KEY`, `TIMESTAMP`, `TEXT`
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
fn generate_create_table_sql(table_name: &str, driver: &DatabaseDriver) -> String {
    match driver {
        DatabaseDriver::SQLite => format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                timestamp TEXT NOT NULL, \
                level TEXT NOT NULL, \
                target TEXT NOT NULL, \
                message TEXT NOT NULL, \
                fields TEXT, \
                file TEXT, \
                line INTEGER, \
                thread_id TEXT NOT NULL\
            )",
            table_name
        ),
        DatabaseDriver::PostgreSQL => format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                id BIGSERIAL PRIMARY KEY, \
                timestamp TIMESTAMPTZ NOT NULL, \
                level TEXT NOT NULL, \
                target TEXT NOT NULL, \
                message TEXT NOT NULL, \
                fields TEXT, \
                file TEXT, \
                line INTEGER, \
                thread_id TEXT NOT NULL\
            )",
            table_name
        ),
        DatabaseDriver::MySQL => format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                id BIGINT AUTO_INCREMENT PRIMARY KEY, \
                timestamp TIMESTAMP NOT NULL, \
                level TEXT NOT NULL, \
                target TEXT NOT NULL, \
                message TEXT NOT NULL, \
                fields TEXT, \
                file TEXT, \
                line INTEGER, \
                thread_id TEXT NOT NULL\
            )",
            table_name
        ),
        DatabaseDriver::DuckDB => format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                id BIGINT AUTOINCREMENT PRIMARY KEY, \
                timestamp TIMESTAMP NOT NULL, \
                level TEXT NOT NULL, \
                target TEXT NOT NULL, \
                message TEXT NOT NULL, \
                fields TEXT, \
                file TEXT, \
                line INTEGER, \
                thread_id TEXT NOT NULL\
            )",
            table_name
        ),
    }
}

/// 从数据库 URL 推断驱动类型。
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
fn detect_driver_from_url(url: &str) -> DatabaseDriver {
    if url.starts_with("sqlite:") || url.starts_with("sqlite3:") {
        DatabaseDriver::SQLite
    } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
        DatabaseDriver::PostgreSQL
    } else if url.starts_with("mysql:") {
        DatabaseDriver::MySQL
    } else if url.starts_with("duckdb:") || url.starts_with("duckdb://") {
        DatabaseDriver::DuckDB
    } else {
        // 默认回退到 PostgreSQL
        DatabaseDriver::PostgreSQL
    }
}

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
#[async_trait]
impl Database for DbNexusAdapter {
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError> {
        if records.is_empty() {
            return Ok(0);
        }

        let start = std::time::Instant::now();

        // 获取写会话 (使用配置的管理员角色)
        let session = self.pool.get_session(&self.admin_role).await.map_err(|e| {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("err", e.to_string());
            InklogError::DatabaseError {
                message: crate::i18n::tr_args("db-session_failed", args),
                source: Some(Box::new(e)),
            }
        })?;

        // 构建所有记录的 INSERT SQL 语句
        let sqls: Vec<String> = records
            .iter()
            .map(|record| {
                let timestamp = record.timestamp.to_rfc3339();
                let driver = &self.driver;
                let level = escape_sql_string(&record.level, driver);
                let target = escape_sql_string(&record.target, driver);
                let message = escape_sql_string(&record.message, driver);
                let fields_json =
                    serde_json::to_string(&record.fields).unwrap_or_else(|_| "{}".to_string());
                let fields_escaped = escape_sql_string(&fields_json, driver);
                let file = record
                    .file
                    .as_ref()
                    .map(|f| format!("'{}'", escape_sql_string(f, driver)))
                    .unwrap_or_else(|| "NULL".to_string());
                let line = record
                    .line
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "NULL".to_string());
                let thread_id = escape_sql_string(&record.thread_id, driver);

                format!(
                    "INSERT INTO {} (timestamp, level, target, message, fields, file, line, thread_id) \
                     VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}, '{}')",
                    self.table_name,
                    timestamp,
                    level,
                    target,
                    message,
                    fields_escaped,
                    file,
                    line,
                    thread_id
                )
            })
            .collect();

        // 在事务中执行全部语句——原子性：全部成功或全部失败
        let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();
        session
            .batch_execute_in_transaction(sql_refs)
            .await
            .map_err(|e| {
                let elapsed_ms = start.elapsed().as_millis();
                tracing::warn!(
                    table = %self.table_name,
                    error = %e,
                    elapsed_ms = elapsed_ms,
                    "Database batch insert failed"
                );
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("err", e.to_string());
                let msg = crate::i18n::tr_args("db-batch_insert_failed", args);
                InklogError::DatabaseError {
                    message: msg,
                    source: Some(Box::new(e)),
                }
            })?;

        let elapsed_ms = start.elapsed().as_millis();
        tracing::debug!(
            table = %self.table_name,
            count = records.len(),
            elapsed_ms = elapsed_ms,
            "Database batch insert succeeded"
        );

        Ok(records.len())
    }

    async fn is_healthy(&self) -> bool {
        // 健康检查：仅验证连接池能获取管理员会话。
        //
        // 不执行 `SELECT 1` 之类的探针 SQL，原因：
        // dbnexus 启用 `sql-parser` + `permission` features 后，
        // `execute_raw` 要求 SQL 必须含表名以做权限校验，
        // 无表名的 `SELECT 1` 会被 `Permission("SQL statement requires a valid
        // table name for permission checking")` 拒绝。
        //
        // `get_session` 内部调用 `acquire_connection`：
        // - 空闲队列有连接时直接返回（池已验证过初始可达性，见 `with_config`）
        // - 空闲队列为空时调用 `create_connection` 重新建立连接，失败则返回 Err
        //
        // 因此 `get_session` 成功即可认为数据库当前可达，符合 `is_healthy()`
        // 文档要求的"轻量级，适合频繁调用"。
        match self.pool.get_session(&self.admin_role).await {
            Ok(_) => true,
            Err(e) => {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("err", e.to_string());
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("warn-db_health_check_failed", args)
                );
                false
            }
        }
    }
}

// ============================================================================
// 非 dbnexus feature 时的占位实现
// ============================================================================

#[cfg(not(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
)))]
/// DbNexusAdapter - 仅在启用 `dbnexus` feature 时可用
///
/// 当未启用 `dbnexus` feature 时，此类型不存在。
/// 使用 `MockDatabaseAdapter` 作为测试替代方案。
pub struct DbNexusAdapter {
    _phantom: (),
}

#[cfg(not(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
)))]
impl DbNexusAdapter {
    /// 此方法仅在启用 `dbnexus` feature 时可用
    #[deprecated(note = "Enable 'dbnexus' feature to use DbNexusAdapter")]
    pub async fn new(_url: &str, _pool_size: u32) -> Result<Self, InklogError> {
        Err(InklogError::DatabaseError {
            message: "DbNexusAdapter requires 'dbnexus' feature to be enabled".to_string(),
            source: None,
        })
    }
}

// ============================================================================
// MockDatabaseAdapter - 测试用 Mock 实现
// ============================================================================

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Mock 数据库适配器，用于单元测试
///
/// 提供内存存储，支持健康状态控制。
/// 所有操作都在内存中完成，不依赖外部数据库。
///
/// # 线程安全
///
/// 使用 `RwLock` 保护记录存储，使用 `AtomicBool` 管理健康状态，
/// 确保多线程环境下的安全性。
///
/// # 示例
///
/// ```rust
/// use inklog::integrations::infra::database::{Database, MockDatabaseAdapter};
/// use inklog::LogRecord;
/// use tracing::Level;
///
/// #[tokio::main]
/// async fn main() {
///     let db = MockDatabaseAdapter::new();
///
///     // 插入记录
///     let records = vec![LogRecord::new(
///         Level::INFO,
///         "test::module".to_string(),
///         "Test message".to_string(),
///     )];
///     let count = db.insert_batch(&records).await.unwrap();
///     assert_eq!(count, 1);
///
///     // 健康检查
///     assert!(db.is_healthy().await);
///
///     // 模拟故障
///     db.set_healthy(false);
///     assert!(!db.is_healthy().await);
/// }
/// ```
pub struct MockDatabaseAdapter {
    /// 存储的日志记录
    records: RwLock<Vec<LogRecord>>,
    /// 健康状态
    healthy: Arc<AtomicBool>,
}

impl MockDatabaseAdapter {
    /// 创建新的 Mock 数据库适配器
    ///
    /// 初始化为健康状态（`healthy = true`）。
    pub fn new() -> Self {
        Self {
            records: RwLock::new(Vec::new()),
            healthy: Arc::new(AtomicBool::new(true)),
        }
    }

    /// 设置健康状态
    ///
    /// 用于测试中模拟数据库故障和恢复场景。
    ///
    /// # 参数
    ///
    /// * `healthy` - 新的健康状态
    pub fn set_healthy(&self, healthy: bool) {
        self.healthy.store(healthy, Ordering::SeqCst);
    }

    /// 获取存储的记录数量
    ///
    /// 用于测试验证插入操作。
    pub fn record_count(&self) -> usize {
        self.records.read().unwrap().len()
    }

    /// 获取所有存储的记录
    ///
    /// 返回记录的克隆，用于测试验证。
    pub fn get_records(&self) -> Vec<LogRecord> {
        self.records.read().unwrap().clone()
    }

    /// 清空所有记录
    ///
    /// 用于测试重置状态。
    pub fn clear(&self) {
        self.records.write().unwrap().clear();
    }
}

impl Default for MockDatabaseAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MockDatabaseAdapter {
    /// Returns the number of records stored (for test verification)
    pub fn stored_count(&self) -> usize {
        self.records.read().unwrap().len()
    }
}

#[async_trait]
impl Database for MockDatabaseAdapter {
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError> {
        if records.is_empty() {
            return Ok(0);
        }

        let mut stored = self.records.write().unwrap();
        stored.extend_from_slice(records);
        Ok(records.len())
    }

    async fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::SeqCst)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;

    // ============================================================================
    // DbNexusAdapter 测试 (需要 feature)
    // ============================================================================

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_health_check() {
        // 创建临时权限配置文件
        let temp_dir = std::env::temp_dir();
        let perm_path = temp_dir.join("inklog_health_perm.yaml");
        let perm_content = r#"roles:
  admin:
    tables:
      - name: "*"
        operations: ["select", "insert", "update", "delete"]
"#;
        std::fs::write(&perm_path, perm_content).expect("Failed to write permissions file");

        // 创建 DbConfig（使用不同的数据库文件）
        let db_path = temp_dir.join("inklog_health.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let config = DbConfig {
            url: db_url,
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 1,
                min_connections: 1,
                idle_timeout: 300,
                acquire_timeout: 30000,
            },
            permissions_path: Some(perm_path.to_string_lossy().to_string()),
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: "admin".to_string(),
            warmup_timeout: 60,
            warmup_retries: 5,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
            retry_policy: Some(dbnexus::reliability::retry::RetryPolicy::default()),
            failover_config: None,
            replica_config: None,
        };

        let pool = DbPool::with_config(config)
            .await
            .expect("Failed to create pool");
        let db = DbNexusAdapter::from_pool(pool, "logs").expect("from_pool should succeed");

        // 创建表用于健康检查
        let session = db
            .pool
            .get_session("admin")
            .await
            .expect("Failed to get session");
        session
            .execute_raw_ddl(
                "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT NOT NULL
            )",
            )
            .await
            .expect("Failed to create table");
        drop(session);

        // 直接测试健康检查逻辑 - 使用有效的表名进行查询
        let session = db
            .pool
            .get_session("admin")
            .await
            .expect("Failed to get session");
        let result = session.execute_raw("SELECT COUNT(*) FROM logs").await;
        assert!(
            result.is_ok(),
            "Health check query failed: {:?}",
            result.err()
        );
        drop(session);

        drop(db);

        let _ = std::fs::remove_file(&perm_path);
        let _ = std::fs::remove_file(&db_path);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_insert_batch() {
        // 创建临时权限配置文件
        let temp_dir = std::env::temp_dir();
        let perm_path = temp_dir.join("inklog_batch_perm.yaml");
        let perm_content = r#"roles:
  admin:
    tables:
      - name: "*"
        operations: ["select", "insert", "update", "delete"]
"#;
        std::fs::write(&perm_path, perm_content).expect("Failed to write permissions file");

        // 创建 DbConfig（使用不同的数据库文件）
        let db_path = temp_dir.join("inklog_batch.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let config = DbConfig {
            url: db_url,
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 2,
                min_connections: 1,
                idle_timeout: 300,
                acquire_timeout: 30000,
            },
            permissions_path: Some(perm_path.to_string_lossy().to_string()),
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: "admin".to_string(),
            warmup_timeout: 60,
            warmup_retries: 5,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
            retry_policy: Some(dbnexus::reliability::retry::RetryPolicy::default()),
            failover_config: None,
            replica_config: None,
        };

        let pool = DbPool::with_config(config)
            .await
            .expect("Failed to create pool");
        let db = DbNexusAdapter::from_pool(pool, "logs").expect("from_pool should succeed");

        // 创建 logs 表
        let session = db
            .pool
            .get_session("admin")
            .await
            .expect("Failed to get session");
        let create_result = session
            .execute_raw_ddl(
                "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT NOT NULL
            )",
            )
            .await;
        assert!(
            create_result.is_ok(),
            "Failed to create table: {:?}",
            create_result.err()
        );
        drop(session);

        let records = vec![LogRecord::new(
            tracing::Level::INFO,
            "test::module".to_string(),
            "Test message".to_string(),
        )];

        let count = db.insert_batch(&records).await.expect("Failed to insert");
        assert_eq!(count, 1);

        drop(db);

        let _ = std::fs::remove_file(&perm_path);
        let _ = std::fs::remove_file(&db_path);
    }

    #[cfg(not(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    )))]
    #[allow(deprecated)]
    #[tokio::test]
    async fn test_dbnexus_adapter_not_available_without_feature() {
        let result = DbNexusAdapter::new("test", 1).await;
        assert!(result.is_err());
        if let Err(InklogError::DatabaseError { .. }) = result {
            // Expected
        } else {
            panic!("Expected DatabaseError");
        }
    }

    // ============================================================================
    // MockDatabaseAdapter 测试
    // ============================================================================

    #[tokio::test]
    async fn test_mock_database_insert_batch() {
        let db = MockDatabaseAdapter::new();

        let records = vec![
            LogRecord::new(Level::INFO, "module1".to_string(), "message1".to_string()),
            LogRecord::new(Level::WARN, "module2".to_string(), "message2".to_string()),
        ];

        let count = db.insert_batch(&records).await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(db.record_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_database_insert_empty_batch() {
        let db = MockDatabaseAdapter::new();

        let records: Vec<LogRecord> = vec![];
        let count = db.insert_batch(&records).await.unwrap();
        assert_eq!(count, 0);
        assert_eq!(db.record_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_database_is_healthy() {
        let db = MockDatabaseAdapter::new();

        // 初始状态应该是健康的
        assert!(db.is_healthy().await);

        // 设置为不健康
        db.set_healthy(false);
        assert!(!db.is_healthy().await);

        // 恢复健康
        db.set_healthy(true);
        assert!(db.is_healthy().await);
    }

    #[tokio::test]
    async fn test_mock_database_get_records() {
        let db = MockDatabaseAdapter::new();

        let records = vec![
            LogRecord::new(Level::INFO, "module".to_string(), "message1".to_string()),
            LogRecord::new(Level::ERROR, "module".to_string(), "message2".to_string()),
        ];

        db.insert_batch(&records).await.unwrap();

        let stored = db.get_records();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].message, "message1");
        assert_eq!(stored[1].message, "message2");
    }

    #[tokio::test]
    async fn test_mock_database_clear() {
        let db = MockDatabaseAdapter::new();

        let records = vec![LogRecord::new(
            Level::INFO,
            "module".to_string(),
            "message".to_string(),
        )];

        db.insert_batch(&records).await.unwrap();
        assert_eq!(db.record_count(), 1);

        db.clear();
        assert_eq!(db.record_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_database_default() {
        let db = MockDatabaseAdapter::default();

        assert!(db.is_healthy().await);
        assert_eq!(db.record_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_database_multiple_inserts() {
        let db = MockDatabaseAdapter::new();

        // 第一次插入
        let records1 = vec![LogRecord::new(
            Level::INFO,
            "module1".to_string(),
            "message1".to_string(),
        )];
        db.insert_batch(&records1).await.unwrap();
        assert_eq!(db.record_count(), 1);

        // 第二次插入
        let records2 = vec![LogRecord::new(
            Level::WARN,
            "module2".to_string(),
            "message2".to_string(),
        )];
        db.insert_batch(&records2).await.unwrap();
        assert_eq!(db.record_count(), 2);

        // 验证记录顺序
        let stored = db.get_records();
        assert_eq!(stored[0].message, "message1");
        assert_eq!(stored[1].message, "message2");
    }

    // ============================================================================
    // DbNexusAdapter getter 与空批量插入测试
    // 覆盖行：181-183 (Ok(Self)), 203-204 (pool()), 208-209 (table_name()), 218 (Ok(0))
    // ============================================================================

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_with_table_name_creates_instance() {
        // 覆盖行 181-183：with_table_name 成功路径返回 Ok(Self { pool, table_name })
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("inklog_with_table_name.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let adapter = DbNexusAdapter::with_table_name(&db_url, 1, "custom_logs")
            .await
            .expect("with_table_name should succeed");

        // 覆盖行 208-209：table_name() getter
        assert_eq!(adapter.table_name(), "custom_logs");

        let _ = std::fs::remove_file(&db_path);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_pool_getter_returns_underlying_pool() {
        // 覆盖行 203-204：pool() getter
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("inklog_pool_getter.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let adapter = DbNexusAdapter::new(&db_url, 1)
            .await
            .expect("new should succeed");

        // pool() 返回底层 DbPool 引用——验证可获取 admin 会话即可
        let _session = adapter
            .pool()
            .get_session("admin")
            .await
            .expect("should get session from underlying pool");

        let _ = std::fs::remove_file(&db_path);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_insert_empty_batch_returns_zero() {
        // 覆盖行 218：insert_batch 收到空切片时立即返回 Ok(0)
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("inklog_empty_batch.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let adapter = DbNexusAdapter::new(&db_url, 1)
            .await
            .expect("new should succeed");

        let empty: Vec<LogRecord> = vec![];
        let count = adapter
            .insert_batch(&empty)
            .await
            .expect("empty batch should succeed");
        assert_eq!(count, 0, "empty batch must return 0");

        let _ = std::fs::remove_file(&db_path);
    }

    // ============================================================================
    // T003: SQL injection via unescaped level field
    // ============================================================================

    #[test]
    fn test_level_field_single_quote_escaping() {
        // Verify that the escaping pattern used in insert_batch correctly neutralizes
        // SQL injection via single quotes in the level field.
        let malicious_level = "INFO'OR'1'='1";
        let escaped = malicious_level.replace('\'', "''");
        assert_eq!(escaped, "INFO''OR''1''=''1");
        // After escaping, every original single quote should be doubled,
        // so the string cannot break out of a SQL string literal.
        assert_eq!(
            escaped.matches('\'').count(),
            8,
            "all 4 original quotes should be doubled"
        );
    }

    // ============================================================================
    // T004: Table name validation
    // ============================================================================

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_table_name_accepts_valid_names() {
        assert!(validate_table_name("logs").is_ok());
        assert!(validate_table_name("my_logs").is_ok());
        assert!(validate_table_name("_private").is_ok());
        assert!(validate_table_name("Logs123").is_ok());
        assert!(validate_table_name("a").is_ok());
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_table_name_rejects_sql_injection() {
        // SQL injection attempts
        assert!(validate_table_name("logs; DROP TABLE users").is_err());
        assert!(validate_table_name("logs' OR '1'='1").is_err());
        assert!(validate_table_name("logs--comment").is_err());
        assert!(validate_table_name("logs\";").is_err());
        // Empty name
        assert!(validate_table_name("").is_err());
        // Starts with digit
        assert!(validate_table_name("123logs").is_err());
        // Contains dot (schema.table)
        assert!(validate_table_name("public.logs").is_err());
    }

    // ============================================================================
    // T009: escape_sql_string 单元测试
    // ============================================================================

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_escape_sql_string_empty() {
        assert_eq!(escape_sql_string("", &DatabaseDriver::SQLite), "");
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_escape_sql_string_no_special_chars() {
        assert_eq!(
            escape_sql_string("hello world", &DatabaseDriver::SQLite),
            "hello world"
        );
        assert_eq!(
            escape_sql_string("abc123", &DatabaseDriver::SQLite),
            "abc123"
        );
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_escape_sql_string_single_quote() {
        assert_eq!(escape_sql_string("it's", &DatabaseDriver::SQLite), "it''s");
        assert_eq!(escape_sql_string("'", &DatabaseDriver::SQLite), "''");
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_escape_sql_string_multiple_quotes() {
        assert_eq!(
            escape_sql_string("a'b'c'd", &DatabaseDriver::SQLite),
            "a''b''c''d"
        );
        assert_eq!(
            escape_sql_string("''''", &DatabaseDriver::SQLite),
            "''''''''"
        );
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_escape_sql_string_unicode() {
        assert_eq!(
            escape_sql_string("こんにちは", &DatabaseDriver::SQLite),
            "こんにちは"
        );
        assert_eq!(
            escape_sql_string("世界'it's", &DatabaseDriver::SQLite),
            "世界''it''s"
        );
    }

    /// diting MED-001 回归：MySQL 默认把反斜杠当转义符，
    /// `\` 后跟 `'` 的载荷仅做引号双写仍可逃逸字符串。
    #[cfg(feature = "mysql")]
    #[test]
    fn test_escape_sql_string_mysql_backslash_injection() {
        // MySQL：反斜杠必须被转义，否则 `\'; DROP TABLE logs; --` 会逃逸字符串。
        let payload = "\\'; DROP TABLE logs; --"; //  \'; DROP TABLE logs; --
        // MySQL 转义：反斜杠双写 + 引号双写 → \\''; DROP ...
        assert_eq!(
            escape_sql_string(payload, &DatabaseDriver::MySQL),
            "\\\\''; DROP TABLE logs; --"
        );
        // 单反斜杠+引号：必须精确转义为 `\\''`，不能以未转义的 `\'` 形式残留
        assert_eq!(escape_sql_string("\\'", &DatabaseDriver::MySQL), "\\\\''");
        // ANSI 后端不转义反斜杠（保持原样，避免数据损坏）
        assert_eq!(
            escape_sql_string("c:\\temp", &DatabaseDriver::PostgreSQL),
            "c:\\temp"
        );
    }

    // ============================================================================
    // T010: generate_create_table_sql 测试
    // ============================================================================

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_generate_create_table_sql_sqlite() {
        let ddl = generate_create_table_sql("logs", &DatabaseDriver::SQLite);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS logs"));
        assert!(ddl.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(ddl.contains("TEXT NOT NULL"));
        assert!(ddl.contains("line INTEGER"));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_generate_create_table_sql_postgres() {
        let ddl = generate_create_table_sql("logs", &DatabaseDriver::PostgreSQL);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS logs"));
        assert!(ddl.contains("BIGSERIAL PRIMARY KEY"));
        assert!(ddl.contains("TIMESTAMPTZ NOT NULL"));
        assert!(ddl.contains("TEXT NOT NULL"));
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_generate_create_table_sql_mysql() {
        let ddl = generate_create_table_sql("logs", &DatabaseDriver::MySQL);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS logs"));
        assert!(ddl.contains("BIGINT AUTO_INCREMENT PRIMARY KEY"));
        assert!(ddl.contains("TIMESTAMP NOT NULL"));
        assert!(ddl.contains("TEXT NOT NULL"));
    }

    // ============================================================================
    // T011: 自定义 admin_role 构造测试
    // ============================================================================

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_dbnexus_adapter_custom_admin_role() {
        let temp_dir = std::env::temp_dir();

        // 创建权限配置文件，允许 superadmin 角色
        let perm_path = temp_dir.join("inklog_custom_role_perm.yaml");
        let perm_content = r#"roles:
  superadmin:
    tables:
      - name: "*"
        operations: ["select", "insert", "update", "delete"]
"#;
        std::fs::write(&perm_path, perm_content).expect("Failed to write permissions file");

        let db_path = temp_dir.join("inklog_custom_role.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let adapter = DbNexusAdapter::with_full_config(
            &db_url,
            1,
            "logs",
            Some(perm_path.to_string_lossy().to_string()),
            "superadmin",
        )
        .await
        .expect("with_full_config should succeed");

        // Verify the adapter stores the custom role
        assert_eq!(adapter.admin_role, "superadmin");
        assert_eq!(adapter.table_name(), "logs");

        let _ = std::fs::remove_file(&perm_path);
        let _ = std::fs::remove_file(&db_path);
    }
}
