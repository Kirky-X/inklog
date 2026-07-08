// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Database provider abstraction for inklog's trait-kit integration.
//!
//! Defines the [`LogDbProvider`] trait — the minimal database interface that
//! inklog consumes when wiring [`InklogModule`] through trait-kit 0.2.2
//! AsyncKit. The trait uses explicit `Pin<Box<dyn Future + Send>>` return
//! types (rather than `async fn` in trait) to remain dyn-compatible
//! (`dyn LogDbProvider` compiles) without pulling in the `async-trait`
//! crate. This mirrors the pattern used by dbnexus's `DbCacheProvider`
//! (see `dbnexus/src/domain/cache_provider.rs`).
//!
//! # Rule 7 divergence from `design.md` / `spec.md`
//!
//! `design.md` Decision 2 and `spec.md` R-inklog-module-001 specify
//! `LogEntry` and `LogError` as the parameter and error types. These types
//! do not exist in inklog — the codebase uses [`LogRecord`] and
//! [`InklogError`] respectively. We use the existing types rather than
//! creating redundant duplicates (Rule 11: 惯例优先于新颖).

use std::future::Future;
use std::pin::Pin;

use crate::{InklogError, LogRecord};

/// Database provider abstraction consumed by inklog's trait-kit integration.
///
/// Defines the minimal execute/batch-insert interface for persisting log
/// records. Implementations include [`DbNexusLogDbAdapter`](crate::integrations::dbnexus_adapter::DbNexusLogDbAdapter)
/// (feature-gated behind `kit`) and any custom adapter the user supplies.
///
/// # Object safety
///
/// The trait is dyn-compatible: `Arc<dyn LogDbProvider + Send + Sync>`
/// compiles. Methods return `Pin<Box<dyn Future + Send + 'a>>` (lifetime-tied
/// to `&self`) rather than using `async fn` in trait, because `dyn`-compatible
/// dispatch requires the explicit `Pin<Box>` indirection.
pub trait LogDbProvider: Send + Sync {
    /// Execute a raw SQL statement (e.g. DDL or single INSERT).
    ///
    /// Returns `Err(InklogError::DatabaseError(..))` on backend failure.
    #[allow(
        clippy::type_complexity,
        reason = "Pin<Box<dyn Future + Send>> is the canonical dyn-compatible async trait dispatch type"
    )]
    fn execute_log<'a>(
        &'a self,
        sql: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>>;

    /// Batch-insert log records atomically (all-or-nothing).
    ///
    /// Returns `Err(InklogError::DatabaseError(..))` on backend failure.
    #[allow(
        clippy::type_complexity,
        reason = "Pin<Box<dyn Future + Send>> is the canonical dyn-compatible async trait dispatch type"
    )]
    fn batch_insert<'a>(
        &'a self,
        entries: Vec<LogRecord>,
    ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// In-memory `LogDbProvider` mock for verifying the trait contract.
    struct MockLogDbProvider {
        executed_sqls: Mutex<Vec<String>>,
        inserted_records: Mutex<Vec<LogRecord>>,
    }

    impl MockLogDbProvider {
        fn new() -> Self {
            Self {
                executed_sqls: Mutex::new(Vec::new()),
                inserted_records: Mutex::new(Vec::new()),
            }
        }
    }

    impl LogDbProvider for MockLogDbProvider {
        fn execute_log<'a>(
            &'a self,
            sql: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>> {
            Box::pin(async move {
                self.executed_sqls
                    .lock()
                    .expect("mock lock poisoned")
                    .push(sql.to_string());
                Ok(())
            })
        }

        fn batch_insert<'a>(
            &'a self,
            entries: Vec<LogRecord>,
        ) -> Pin<Box<dyn Future<Output = Result<(), InklogError>> + Send + 'a>> {
            Box::pin(async move {
                let mut store = self.inserted_records.lock().expect("mock lock poisoned");
                store.extend(entries);
                Ok(())
            })
        }
    }

    /// R-inklog-module-001 #1: trait is object-safe —
    /// `Arc<dyn LogDbProvider + Send + Sync>` compiles and dispatches.
    #[test]
    fn log_db_provider_is_object_safe() {
        fn assert_dyn_compatible(_: Arc<dyn LogDbProvider + Send + Sync>) {}
        let mock = Arc::new(MockLogDbProvider::new());
        assert_dyn_compatible(mock);
    }

    /// R-inklog-module-001 #2: `execute_log` returns `Pin<Box<dyn Future + Send>>`
    /// — verified by calling `.await` on the returned future inside a tokio runtime.
    #[tokio::test]
    async fn execute_log_returns_pin_box_future() {
        let provider = MockLogDbProvider::new();
        provider
            .execute_log("CREATE TABLE logs (id INTEGER PRIMARY KEY)")
            .await
            .expect("execute_log should succeed");
        let sqls = provider.executed_sqls.lock().expect("lock");
        assert_eq!(sqls.len(), 1);
        assert!(sqls[0].contains("CREATE TABLE"));
    }

    /// R-inklog-module-001 #3: `batch_insert` stores all records.
    #[tokio::test]
    async fn batch_insert_stores_all_records() {
        let provider = MockLogDbProvider::new();
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
        provider
            .batch_insert(records)
            .await
            .expect("batch_insert should succeed");
        let stored = provider.inserted_records.lock().expect("lock");
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].message, "message_a");
        assert_eq!(stored[1].message, "message_b");
    }

    /// R-inklog-module-001 #4: `batch_insert` with empty vec succeeds (no-op).
    #[tokio::test]
    async fn batch_insert_empty_vec_succeeds() {
        let provider = MockLogDbProvider::new();
        provider
            .batch_insert(Vec::new())
            .await
            .expect("empty batch should succeed");
        let stored = provider.inserted_records.lock().expect("lock");
        assert!(stored.is_empty());
    }

    /// R-inklog-module-001 #5: trait can be used through a trait object
    /// (`Arc<dyn LogDbProvider + Send + Sync>`) — the dyn dispatch path
    /// that `InklogModule::build` will use.
    #[tokio::test]
    async fn log_db_provider_dyn_dispatch_works() {
        let provider: Arc<dyn LogDbProvider + Send + Sync> = Arc::new(MockLogDbProvider::new());
        provider
            .execute_log("SELECT 1")
            .await
            .expect("execute via dyn should succeed");
        provider
            .batch_insert(vec![LogRecord::new(
                tracing::Level::INFO,
                "dyn_test".to_string(),
                "via dyn".to_string(),
            )])
            .await
            .expect("batch_insert via dyn should succeed");
    }

    /// R-inklog-module-001 #6: multiple `execute_log` calls accumulate.
    #[tokio::test]
    async fn execute_log_accumulates_multiple_calls() {
        let provider = MockLogDbProvider::new();
        provider.execute_log("SQL_1").await.expect("first");
        provider.execute_log("SQL_2").await.expect("second");
        provider.execute_log("SQL_3").await.expect("third");
        let sqls = provider.executed_sqls.lock().expect("lock");
        assert_eq!(sqls.len(), 3);
        assert_eq!(sqls[0], "SQL_1");
        assert_eq!(sqls[2], "SQL_3");
    }
}
