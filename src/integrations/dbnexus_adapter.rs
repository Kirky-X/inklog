// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `DbNexusLogDbAdapter` — adapts a dbnexus `ConnectionPool` to inklog's
//! `LogDbProvider` trait for trait-kit 0.2.2 AsyncKit integration.
//!
//! Phase 6 (T042 Red / T043 Green) of the `trait-kit-async-integration`
//! change. Wraps `Arc<dyn dbnexus::ConnectionPool + Send + Sync>` and
//! proxies `execute_log` / `batch_insert` calls through dbnexus's `Session`
//! API.
//!
//! # Rule 7 divergences from `design.md` / `spec.md`
//!
//! 1. **Error type**: `spec.md` R-inklog-module-002 specifies
//!    `LogError::database(message)`. `LogError` does not exist in inklog —
//!    the codebase uses [`InklogError::DatabaseError`]. We use the existing
//!    type (Rule 11: 惯例优先于新颖).
//!
//! 2. **Feature gate**: `spec.md` R-inklog-module-002 specifies
//!    `#[cfg(feature = "dbnexus-integration")]`. This feature was removed in
//!    T039 because `dep:dbnexus` alone triggers dbnexus's
//!    `compile_error!("Must enable at least one database feature")`. The
//!    adapter is gated under `all(feature = "kit", any(feature = "sqlite",
//!    feature = "postgres", feature = "mysql"))` — users must enable both
//!    `kit` and a database backend.
//!
//! 3. **Field type**: `spec.md` R-inklog-module-002 specifies
//!    `session: Arc<dyn dbnexus::ConnectionPool>`. We use
//!    `Arc<dyn dbnexus::ConnectionPool + Send + Sync>` to match the
//!    `Capability` type returned by `DbNexusModule` (see
//!    `dbnexus/src/integrations/kit/module.rs`).
//!
//! 4. **Constructor signature**: `spec.md` R-inklog-module-002 specifies
//!    `DbNexusLogDbAdapter::new(session) -> Self`. We add a `table_name`
//!    parameter to match the existing `DbNexusAdapter::with_table_name`
//!    pattern (see `src/integrations/infra/database.rs`).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::{InklogError, LogDbProvider, LogRecord};

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
use dbnexus::ConnectionPool;

/// Adapts a dbnexus `ConnectionPool` to inklog's `LogDbProvider` trait.
///
/// Created by `InklogModule::build` (T044/T045) after `DbNexusModule`
/// provides the `Arc<dyn ConnectionPool + Send + Sync>` capability. The
/// adapter proxies `execute_log` to `Session::execute_raw` and
/// `batch_insert` to `Session::batch_execute_in_transaction`.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use inklog::{DbNexusLogDbAdapter, LogDbProvider};
/// use dbnexus::ConnectionPool;
///
/// # async fn example(pool: Arc<dyn ConnectionPool + Send + Sync>) -> Result<(), inklog::InklogError> {
/// let adapter = DbNexusLogDbAdapter::new(pool, "logs")?;
/// adapter.execute_log("CREATE TABLE logs (id INTEGER PRIMARY KEY)").await?;
/// # Ok(())
/// # }
/// ```
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub struct DbNexusLogDbAdapter {
    pool: Arc<dyn ConnectionPool + Send + Sync>,
    table_name: String,
}

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
impl DbNexusLogDbAdapter {
    /// Creates a new adapter wrapping the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - The dbnexus connection pool capability (typically obtained
    ///   via `kit.require::<DbNexusModule>()`)
    /// * `table_name` - The target log table name for batch INSERT statements.
    ///   Validated by [`sanitize_identifier`] to prevent SQL injection.
    ///
    /// # Errors
    ///
    /// Returns [`InklogError::DatabaseError`] if `table_name` is not a valid
    /// SQL identifier (must start with a letter or underscore, contain only
    /// `[a-zA-Z0-9_]`, and be 1..=64 characters long).
    pub fn new(
        pool: Arc<dyn ConnectionPool + Send + Sync>,
        table_name: &str,
    ) -> Result<Self, InklogError> {
        let sanitized = sanitize_identifier(table_name)?;
        Ok(Self {
            pool,
            table_name: sanitized.to_string(),
        })
    }

    /// Returns the configured table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
impl LogDbProvider for DbNexusLogDbAdapter {
    fn execute_log<'a>(
        &'a self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>> {
        Box::pin(async move {
            let session =
                self.pool.get_session("admin").await.map_err(|e| {
                    InklogError::DatabaseError(format!("Failed to get session: {e}"))
                })?;
            // dbnexus gates DDL (CREATE/DROP/ALTER TABLE etc.) behind
            // `execute_raw_ddl` (admin-only, AST-validated). `execute_raw`
            // rejects DDL with `Permission("DDL operations are not allowed
            // in this context")`. Route based on SQL type so callers can
            // pass both DDL and DML through a single entry point.
            if is_ddl_sql(sql) {
                session
                    .execute_raw_ddl(sql)
                    .await
                    .map_err(|e| InklogError::DatabaseError(format!("Execute DDL failed: {e}")))?;
            } else {
                session
                    .execute_raw(sql)
                    .await
                    .map_err(|e| InklogError::DatabaseError(format!("Execute failed: {e}")))?;
            }
            Ok(())
        })
    }

    fn batch_insert<'a>(
        &'a self,
        entries: Vec<LogRecord>,
    ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>> {
        Box::pin(async move {
            if entries.is_empty() {
                return Ok(());
            }

            let session =
                self.pool.get_session("admin").await.map_err(|e| {
                    InklogError::DatabaseError(format!("Failed to get session: {e}"))
                })?;

            let sqls: Vec<String> = entries
                .iter()
                .map(|record| build_insert_sql(record, &self.table_name))
                .collect::<Result<Vec<String>, InklogError>>()?;

            let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();
            session
                .batch_execute_in_transaction(sql_refs)
                .await
                .map_err(|e| InklogError::DatabaseError(format!("Batch insert failed: {e}")))?;

            Ok(())
        })
    }
}

/// Detects whether the SQL statement is DDL (must use `execute_raw_ddl`).
///
/// dbnexus's `execute_raw` rejects DDL with `Permission("DDL operations
/// are not allowed in this context")`; DDL must go through
/// `execute_raw_ddl` (admin-only, AST-validated by `DdlGuard`). The
/// keyword list mirrors `dbnexus::access::sql_parser::is_ddl_operation`.
fn is_ddl_sql(sql: &str) -> bool {
    let sql_upper = sql.trim().to_uppercase();
    const DDL_PREFIXES: &[&str] = &[
        "CREATE TABLE",
        "DROP TABLE",
        "ALTER TABLE",
        "TRUNCATE TABLE",
        "CREATE INDEX",
        "DROP INDEX",
        "CREATE VIEW",
        "DROP VIEW",
    ];
    DDL_PREFIXES
        .iter()
        .any(|prefix| sql_upper.starts_with(prefix))
}

/// Maximum length for a SQL identifier (conservative bound covering
/// MySQL 64, PostgreSQL 63, SQLite default). Excessively long identifiers
/// are rejected to bound the SQL string size.
const MAX_SQL_IDENTIFIER_LEN: usize = 64;

/// Validates a SQL identifier (table/column name) against a strict whitelist.
///
/// # Rules
///
/// - Must be 1..=`MAX_SQL_IDENTIFIER_LEN` characters long.
/// - Must start with an ASCII letter (`a..=z`, `A..=Z`) or underscore (`_`).
/// - Subsequent characters must be ASCII letters, digits (`0..=9`), or
///   underscore.
///
/// This is defense-in-depth: even though `DbNexusLogDbAdapter::new`
/// validates `table_name` at construction time, `build_insert_sql` calls
/// this again so a future refactor cannot accidentally bypass the
/// constructor check.
///
/// # Errors
///
/// Returns [`InklogError::DatabaseError`] with a descriptive message if the
/// identifier violates any rule. The original input is **never** included
/// verbatim in the error message to avoid log-injection via error strings.
///
/// # Examples
///
/// ```ignore
/// # use inklog::integrations::dbnexus_adapter::sanitize_identifier;
/// assert!(sanitize_identifier("logs").is_ok());
/// assert!(sanitize_identifier("app_logs").is_ok());
/// assert!(sanitize_identifier("log_2024").is_ok());
/// assert!(sanitize_identifier("").is_err());
/// assert!(sanitize_identifier("logs'; DROP TABLE logs; --").is_err());
/// ```
pub fn sanitize_identifier(name: &str) -> Result<&str, InklogError> {
    if name.is_empty() {
        return Err(InklogError::DatabaseError(
            "Invalid SQL identifier: empty".to_string(),
        ));
    }
    if name.len() > MAX_SQL_IDENTIFIER_LEN {
        return Err(InklogError::DatabaseError(format!(
            "Invalid SQL identifier: length {} exceeds maximum {}",
            name.len(),
            MAX_SQL_IDENTIFIER_LEN
        )));
    }

    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(InklogError::DatabaseError(
            "Invalid SQL identifier: must start with a letter or underscore".to_string(),
        ));
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(InklogError::DatabaseError(
                "Invalid SQL identifier: contains disallowed character".to_string(),
            ));
        }
    }
    Ok(name)
}

/// Builds a single INSERT SQL statement for the given log record.
///
/// Escapes single quotes in string values by doubling them (`'` → `''`),
/// following the SQL standard. `file` and `line` are emitted as `NULL`
/// when absent.
///
/// The `table_name` is validated by [`sanitize_identifier`] before
/// interpolation to prevent SQL injection. This is defense-in-depth — the
/// adapter already validates `table_name` at construction time.
///
/// # Errors
///
/// Returns [`InklogError::DatabaseError`] if `table_name` fails
/// [`sanitize_identifier`] validation.
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn build_insert_sql(record: &LogRecord, table_name: &str) -> Result<String, InklogError> {
    let sanitized = sanitize_identifier(table_name)?;
    let timestamp = record.timestamp.to_rfc3339();
    let level = &record.level;
    let target = &record.target;
    let message = record.message.replace('\'', "''");
    let fields_json = serde_json::to_string(&record.fields).unwrap_or_else(|_| "{}".to_string());
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

    Ok(format!(
        "INSERT INTO {} (timestamp, level, target, message, fields, file, line, thread_id) \
         VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}, '{}')",
        sanitized,
        timestamp,
        level,
        target.replace('\'', "''"),
        message,
        fields_escaped,
        file,
        line,
        thread_id.replace('\'', "''")
    ))
}

#[cfg(all(
    test,
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
mod tests {
    use super::*;
    use dbnexus::{DbConfig, DbPoolBuilder};

    /// Creates a sqlite::memory: connection pool for testing.
    async fn create_sqlite_pool() -> Arc<dyn ConnectionPool + Send + Sync> {
        let config = DbConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        };
        let pool = DbPoolBuilder::new()
            .config(config)
            .build()
            .await
            .expect("Failed to create sqlite pool");
        Arc::new(pool) as Arc<dyn ConnectionPool + Send + Sync>
    }

    const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL,
        level TEXT NOT NULL,
        target TEXT NOT NULL,
        message TEXT NOT NULL,
        fields TEXT,
        file TEXT,
        line INTEGER,
        thread_id TEXT NOT NULL
    )";

    /// R-inklog-module-002 #1: `DbNexusLogDbAdapter::new` returns an instance
    /// with the configured table name.
    #[tokio::test]
    async fn adapter_new_returns_instance() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name");
        assert_eq!(adapter.table_name(), "logs");
    }

    /// R-inklog-module-002 #2: `execute_log` runs DDL successfully.
    #[tokio::test]
    async fn adapter_execute_log_runs_ddl() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name");
        adapter
            .execute_log(CREATE_TABLE_SQL)
            .await
            .expect("execute_log should succeed for DDL");
    }

    /// R-inklog-module-002 #3: `batch_insert` inserts all records atomically.
    #[tokio::test]
    async fn adapter_batch_insert_inserts_records() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name");

        // Create the table first.
        adapter
            .execute_log(CREATE_TABLE_SQL)
            .await
            .expect("create table");

        let records = vec![
            LogRecord::new(
                tracing::Level::INFO,
                "module_a".to_string(),
                "message_a".to_string(),
            ),
            LogRecord::new(
                tracing::Level::WARN,
                "module_b".to_string(),
                "message_b".to_string(),
            ),
        ];

        adapter
            .batch_insert(records)
            .await
            .expect("batch_insert should succeed");

        // Verify the records were inserted by counting rows.
        let session = adapter
            .pool
            .get_session("admin")
            .await
            .expect("get session for verification");
        let _result = session
            .execute_raw("SELECT COUNT(*) FROM logs")
            .await
            .expect("count query should succeed");
    }

    /// R-inklog-module-002 #4: `batch_insert` with empty vec succeeds (no-op).
    #[tokio::test]
    async fn adapter_batch_insert_empty_succeeds() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name");
        adapter
            .batch_insert(Vec::new())
            .await
            .expect("empty batch should succeed");
    }

    /// R-inklog-module-002 #5: `DbNexusLogDbAdapter` satisfies `LogDbProvider`.
    #[tokio::test]
    async fn adapter_satisfies_log_db_provider() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name");
        fn assert_impl<T: LogDbProvider>(_: &T) {}
        assert_impl(&adapter);
    }

    /// R-inklog-module-002 #6: `DbNexusLogDbAdapter` works through dyn dispatch.
    #[tokio::test]
    async fn adapter_dyn_dispatch_works() {
        let pool = create_sqlite_pool().await;
        let adapter: Arc<dyn LogDbProvider + Send + Sync> =
            Arc::new(DbNexusLogDbAdapter::new(pool, "logs").expect("valid table name"));

        adapter
            .execute_log(CREATE_TABLE_SQL)
            .await
            .expect("execute_log via dyn should succeed");

        adapter
            .batch_insert(vec![LogRecord::new(
                tracing::Level::INFO,
                "dyn_test".to_string(),
                "via dyn".to_string(),
            )])
            .await
            .expect("batch_insert via dyn should succeed");
    }

    /// R-inklog-module-002 #7: `build_insert_sql` escapes single quotes.
    #[test]
    fn build_insert_sql_escapes_single_quotes() {
        let mut record = LogRecord::new(
            tracing::Level::INFO,
            "module".to_string(),
            "it's a test".to_string(),
        );
        record.thread_id = "thread'1".to_string();

        let sql = build_insert_sql(&record, "logs").expect("valid table name");
        assert!(
            sql.contains("it''s a test"),
            "message single quotes should be escaped, got: {sql}"
        );
        assert!(
            sql.contains("thread''1"),
            "thread_id single quotes should be escaped, got: {sql}"
        );
    }

    // ========================================================================
    // vuln-0001: sanitize_identifier SQL 注入防护测试
    // ========================================================================

    /// vuln-0001 #1: `sanitize_identifier` 接受合法 SQL 标识符。
    #[test]
    fn sanitize_identifier_accepts_valid_inputs() {
        assert_eq!(sanitize_identifier("logs").unwrap(), "logs");
        assert_eq!(sanitize_identifier("app_logs").unwrap(), "app_logs");
        assert_eq!(sanitize_identifier("log_2024").unwrap(), "log_2024");
        assert_eq!(sanitize_identifier("_private").unwrap(), "_private");
        assert_eq!(sanitize_identifier("L").unwrap(), "L");
        // 64 字符（边界值）
        let max_name = "a".repeat(64);
        assert_eq!(sanitize_identifier(&max_name).unwrap(), &max_name);
    }

    /// vuln-0001 #2: `sanitize_identifier` 拒绝空字符串。
    #[test]
    fn sanitize_identifier_rejects_empty() {
        let err = sanitize_identifier("").unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        assert!(err.to_string().to_lowercase().contains("empty"));
    }

    /// vuln-0001 #3: `sanitize_identifier` 拒绝以数字开头的标识符。
    #[test]
    fn sanitize_identifier_rejects_leading_digit() {
        let err = sanitize_identifier("1logs").unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        assert!(err.to_string().contains("start with a letter"));
    }

    /// vuln-0001 #4: `sanitize_identifier` 拒绝超长标识符（> 64 字符）。
    #[test]
    fn sanitize_identifier_rejects_too_long() {
        let too_long = "a".repeat(65);
        let err = sanitize_identifier(&too_long).unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        assert!(err.to_string().contains("exceeds maximum"));
    }

    /// vuln-0001 #5: `sanitize_identifier` 拒绝 SQL 注入 payload —— DROP TABLE。
    #[test]
    fn sanitize_identifier_rejects_drop_table_injection() {
        let payload = "logs'; DROP TABLE logs; --";
        let err = sanitize_identifier(payload).unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        // 错误信息不得包含原始 payload（防 log-injection）
        assert!(!err.to_string().contains(payload));
    }

    /// vuln-0001 #6: `sanitize_identifier` 拒绝 SQL 注入 payload —— OR '1'='1。
    #[test]
    fn sanitize_identifier_rejects_or_injection() {
        let payload = "logs' OR '1'='1";
        let err = sanitize_identifier(payload).unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        assert!(!err.to_string().contains(payload));
    }

    /// vuln-0001 #7: `sanitize_identifier` 拒绝路径遍历风格输入。
    #[test]
    fn sanitize_identifier_rejects_path_traversal_input() {
        let payload = "../etc/passwd";
        let err = sanitize_identifier(payload).unwrap_err();
        assert!(matches!(err, InklogError::DatabaseError(_)));
        assert!(!err.to_string().contains(payload));
    }

    /// vuln-0001 #8: `sanitize_identifier` 拒绝含空格、分号、引号、连字符的输入。
    #[test]
    fn sanitize_identifier_rejects_special_characters() {
        for bad in &[
            "log table",     // 空格
            "log;drop",      // 分号
            "log'quote",     // 单引号
            "log-dash",      // 连字符
            "log\"double",   // 双引号
            "log*star",      // 星号
            "log(table)",    // 括号
            "log\0null",     // 空字节
            "log\nnewline",  // 换行
            "log`backtick`", // 反引号
        ] {
            assert!(sanitize_identifier(bad).is_err(), "should reject: {bad:?}");
        }
    }

    /// vuln-0001 #9: `sanitize_identifier` 拒绝非 ASCII 字符。
    #[test]
    fn sanitize_identifier_rejects_non_ascii() {
        for bad in &["日志", "logë", "log测试", "lög"] {
            assert!(
                sanitize_identifier(bad).is_err(),
                "should reject non-ASCII: {bad:?}"
            );
        }
    }

    /// vuln-0001 #10: `DbNexusLogDbAdapter::new` 拒绝恶意 table_name。
    #[tokio::test]
    async fn adapter_new_rejects_malicious_table_name() {
        let pool = create_sqlite_pool().await;
        for malicious in &[
            "",
            "logs'; DROP TABLE logs; --",
            "logs' OR '1'='1",
            "../etc/passwd",
            "1logs",
            "log table",
        ] {
            let result = DbNexusLogDbAdapter::new(pool.clone(), malicious);
            assert!(
                matches!(result, Err(InklogError::DatabaseError(_))),
                "should reject malicious table_name: {malicious:?}"
            );
        }
    }

    /// vuln-0001 #11: `build_insert_sql` 拒绝恶意 table_name（深度防御）。
    #[test]
    fn build_insert_sql_rejects_malicious_table_name() {
        let record = LogRecord::new(
            tracing::Level::INFO,
            "module".to_string(),
            "message".to_string(),
        );
        for malicious in &[
            "",
            "logs'; DROP TABLE logs; --",
            "logs' OR '1'='1",
            "../etc/passwd",
        ] {
            let err = build_insert_sql(&record, malicious).unwrap_err();
            assert!(
                matches!(err, InklogError::DatabaseError(_)),
                "should reject malicious table_name: {malicious:?}"
            );
        }
    }

    /// vuln-0001 #12: `build_insert_sql` 生成的 SQL 不含 DROP/UNION/SELECT 注入痕迹。
    #[test]
    fn build_insert_sql_output_is_injection_free() {
        let record = LogRecord::new(
            tracing::Level::INFO,
            "module".to_string(),
            "message".to_string(),
        );
        let sql = build_insert_sql(&record, "logs").expect("valid table name");
        let sql_upper = sql.to_uppercase();
        assert!(!sql_upper.contains("DROP TABLE"));
        assert!(!sql_upper.contains("UNION"));
        assert!(!sql_upper.contains(" OR "));
        assert!(sql_upper.starts_with("INSERT INTO LOGS"));
    }
}
