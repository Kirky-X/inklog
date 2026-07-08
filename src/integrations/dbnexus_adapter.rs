// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
/// let adapter = DbNexusLogDbAdapter::new(pool, "logs");
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
    /// * `table_name` - The target log table name for batch INSERT statements
    pub fn new(pool: Arc<dyn ConnectionPool + Send + Sync>, table_name: &str) -> Self {
        Self {
            pool,
            table_name: table_name.to_string(),
        }
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
                .collect();

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

/// Builds a single INSERT SQL statement for the given log record.
///
/// Escapes single quotes in string values by doubling them (`'` → `''`),
/// following the SQL standard. `file` and `line` are emitted as `NULL`
/// when absent.
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
fn build_insert_sql(record: &LogRecord, table_name: &str) -> String {
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

    format!(
        "INSERT INTO {} (timestamp, level, target, message, fields, file, line, thread_id) \
         VALUES ('{}', '{}', '{}', '{}', '{}', {}, {}, '{}')",
        table_name,
        timestamp,
        level,
        target.replace('\'', "''"),
        message,
        fields_escaped,
        file,
        line,
        thread_id.replace('\'', "''")
    )
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
        let adapter = DbNexusLogDbAdapter::new(pool, "logs");
        assert_eq!(adapter.table_name(), "logs");
    }

    /// R-inklog-module-002 #2: `execute_log` runs DDL successfully.
    #[tokio::test]
    async fn adapter_execute_log_runs_ddl() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs");
        adapter
            .execute_log(CREATE_TABLE_SQL)
            .await
            .expect("execute_log should succeed for DDL");
    }

    /// R-inklog-module-002 #3: `batch_insert` inserts all records atomically.
    #[tokio::test]
    async fn adapter_batch_insert_inserts_records() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs");

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
        let adapter = DbNexusLogDbAdapter::new(pool, "logs");
        adapter
            .batch_insert(Vec::new())
            .await
            .expect("empty batch should succeed");
    }

    /// R-inklog-module-002 #5: `DbNexusLogDbAdapter` satisfies `LogDbProvider`.
    #[tokio::test]
    async fn adapter_satisfies_log_db_provider() {
        let pool = create_sqlite_pool().await;
        let adapter = DbNexusLogDbAdapter::new(pool, "logs");
        fn assert_impl<T: LogDbProvider>(_: &T) {}
        assert_impl(&adapter);
    }

    /// R-inklog-module-002 #6: `DbNexusLogDbAdapter` works through dyn dispatch.
    #[tokio::test]
    async fn adapter_dyn_dispatch_works() {
        let pool = create_sqlite_pool().await;
        let adapter: Arc<dyn LogDbProvider + Send + Sync> =
            Arc::new(DbNexusLogDbAdapter::new(pool, "logs"));

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

        let sql = build_insert_sql(&record, "logs");
        assert!(
            sql.contains("it''s a test"),
            "message single quotes should be escaped, got: {sql}"
        );
        assert!(
            sql.contains("thread''1"),
            "thread_id single quotes should be escaped, got: {sql}"
        );
    }
}
