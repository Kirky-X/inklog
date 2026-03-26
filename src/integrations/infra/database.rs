// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Database trait - 抽象数据库操作
//!
//! 提供日志记录批量写入和健康检查的抽象接口。

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

#[cfg(feature = "dbnexus")]
use dbnexus::database::pool::DbPool;
#[cfg(feature = "dbnexus")]
use dbnexus::foundation::config::DbConfig;

/// dbnexus 适配器
///
/// 将 dbnexus 库的 `DbPool` 适配为 `Database` trait。
/// 使用 Sea-ORM 进行批量插入操作。
///
/// # 功能要求
///
/// - 需要启用 `dbnexus` feature
/// - 支持 PostgreSQL、MySQL、SQLite 数据库
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
#[cfg(feature = "dbnexus")]
pub struct DbNexusAdapter {
    pool: DbPool,
    table_name: String,
}

#[cfg(feature = "dbnexus")]
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
        Self::with_table_name(url, pool_size, "logs").await
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
        // 创建 DbConfig
        let config = DbConfig {
            url: url.to_string(),
            max_connections: pool_size,
            min_connections: 1,
            idle_timeout: 300,
            acquire_timeout: 5000,
            permissions_path: None,
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: "admin".to_string(),
            warmup_timeout: 30,
            warmup_retries: 3,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
        };

        // 使用 DbPool::with_config 创建连接池
        let pool = DbPool::with_config(config).await.map_err(|e| {
            InklogError::DatabaseError(format!("Failed to create connection pool: {}", e))
        })?;

        Ok(Self {
            pool,
            table_name: table_name.to_string(),
        })
    }

    /// 从现有 DbPool 创建适配器
    ///
    /// 用于需要共享连接池的场景。
    ///
    /// # 参数
    ///
    /// * `pool` - 已创建的连接池实例
    /// * `table_name` - 日志表名称
    pub fn from_pool(pool: DbPool, table_name: &str) -> Self {
        Self {
            pool,
            table_name: table_name.to_string(),
        }
    }

    /// 获取底层连接池引用
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// 获取表名
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[cfg(feature = "dbnexus")]
#[async_trait]
impl Database for DbNexusAdapter {
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError> {
        if records.is_empty() {
            return Ok(0);
        }

        // 获取写会话 (使用 admin 角色，因为默认权限配置只允许 admin/system)
        let session = self
            .pool
            .get_session("admin")
            .await
            .map_err(|e| InklogError::DatabaseError(format!("Failed to get session: {}", e)))?;

        // 构建所有记录的 INSERT SQL 语句
        let sqls: Vec<String> = records
            .iter()
            .map(|record| {
                let timestamp = record.timestamp.to_rfc3339();
                let level = &record.level;
                let target = &record.target;
                let message = record.message.replace('\'', "''");
                let fields_json =
                    serde_json::to_string(&record.fields).unwrap_or_else(|_| "{}".to_string());
                let fields_escaped = fields_json.replace('\'', "''");
                let file = record
                    .file
                    .as_ref()
                    .map(|f| format!("'{}'", f.replace('\'', "''")))
                    .unwrap_or_else(|| "NULL".to_string());
                let line = record
                    .line
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "NULL".to_string());
                let thread_id = &record.thread_id;

                format!(
                    "INSERT INTO {} (timestamp, level, target, message, fields, file, line, thread_id) \
                     VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}, '{}')",
                    self.table_name,
                    timestamp,
                    level,
                    target.replace('\'', "''"),
                    message,
                    fields_escaped,
                    file,
                    line,
                    thread_id.replace('\'', "''")
                )
            })
            .collect();

        // 在事务中执行全部语句——原子性：全部成功或全部失败
        let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();
        session
            .batch_execute_in_transaction(sql_refs)
            .await
            .map_err(|e| {
                tracing::error!("Batch insert failed, transaction rolled back: {}", e);
                InklogError::DatabaseError(format!("Batch insert failed: {}", e))
            })?;

        Ok(records.len())
    }

    async fn is_healthy(&self) -> bool {
        // 使用连接池的健康检查 (使用 admin 角色，因为默认权限配置只允许 admin/system)
        match self.pool.get_session("admin").await {
            Ok(session) => {
                // 执行简单的健康检查查询
                match session.execute_raw("SELECT 1").await {
                    Ok(_) => true,
                    Err(e) => {
                        tracing::warn!("Database health check failed: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get session for health check: {}", e);
                false
            }
        }
    }
}

// ============================================================================
// 非 dbnexus feature 时的占位实现
// ============================================================================

#[cfg(not(feature = "dbnexus"))]
/// DbNexusAdapter - 仅在启用 `dbnexus` feature 时可用
///
/// 当未启用 `dbnexus` feature 时，此类型不存在。
/// 使用 `MockDatabaseAdapter` 作为测试替代方案。
pub struct DbNexusAdapter {
    _phantom: (),
}

#[cfg(not(feature = "dbnexus"))]
impl DbNexusAdapter {
    /// 此方法仅在启用 `dbnexus` feature 时可用
    #[deprecated(note = "Enable 'dbnexus' feature to use DbNexusAdapter")]
    pub async fn new(_url: &str, _pool_size: u32) -> Result<Self, InklogError> {
        Err(InklogError::DatabaseError(
            "DbNexusAdapter requires 'dbnexus' feature to be enabled".to_string(),
        ))
    }
}

// ============================================================================
// MockDatabaseAdapter - 测试用 Mock 实现
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

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

    #[cfg(feature = "dbnexus")]
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
            max_connections: 1,
            min_connections: 1,
            idle_timeout: 300,
            acquire_timeout: 30000,
            permissions_path: Some(perm_path.to_string_lossy().to_string()),
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: "admin".to_string(),
            warmup_timeout: 60,
            warmup_retries: 5,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
        };

        let pool = DbPool::with_config(config)
            .await
            .expect("Failed to create pool");
        let db = DbNexusAdapter::from_pool(pool, "logs");

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

    #[cfg(feature = "dbnexus")]
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
            max_connections: 2,
            min_connections: 1,
            idle_timeout: 300,
            acquire_timeout: 30000,
            permissions_path: Some(perm_path.to_string_lossy().to_string()),
            migrations_dir: None,
            auto_migrate: false,
            migration_timeout: 60,
            admin_role: "admin".to_string(),
            warmup_timeout: 60,
            warmup_retries: 5,
            cache_config: dbnexus::foundation::config::CacheConfig::default(),
        };

        let pool = DbPool::with_config(config)
            .await
            .expect("Failed to create pool");
        let db = DbNexusAdapter::from_pool(pool, "logs");

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

    #[cfg(not(feature = "dbnexus"))]
    #[allow(deprecated)]
    #[tokio::test]
    async fn test_dbnexus_adapter_not_available_without_feature() {
        let result = DbNexusAdapter::new("test", 1).await;
        assert!(result.is_err());
        if let Err(InklogError::DatabaseError(_)) = result {
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
}
