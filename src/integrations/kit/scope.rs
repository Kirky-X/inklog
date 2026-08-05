// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Per-request scoped dependency isolation for inklog.
//!
//! Provides `create_inklog_scope()` — a helper that creates an `AsyncScope`
//! for per-request capability isolation. After building modules with the
//! main `AsyncKit`, insert capabilities into the scope with
//! `scope.insert::<InklogModule>(db_cap)`. Each request handler retrieves
//! the capability via `scope.require::<InklogModule>()`.

use std::sync::Arc;

use trait_kit::prelude::AsyncScope;

use crate::integrations::infra::Database;

/// Create a new empty `AsyncScope` for inklog per-request isolation.
///
/// After building with the main `AsyncKit`, insert the database capability:
///
/// ```rust,ignore
/// let built = kit.build().await?;
/// let db = built.require::<InklogModule>()?;
///
/// // Per request:
/// let scope = create_inklog_scope();
/// scope.insert::<InklogModule>(db);
/// let request_db = scope.require::<InklogModule>()?;
/// ```
#[must_use]
pub fn create_inklog_scope() -> AsyncScope {
    AsyncScope::new()
}

/// Populate an `AsyncScope` with the inklog database capability.
///
/// Convenience function that inserts the `Arc<dyn Database + Send + Sync>`
/// into the scope so it can be retrieved with `scope.require::<InklogModule>()`.
pub fn populate_inklog_scope(scope: &AsyncScope, db: Arc<dyn Database + Send + Sync>) {
    scope.insert::<super::InklogModule>(db);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// create_inklog_scope returns a valid empty scope.
    #[test]
    fn create_scope_succeeds() {
        let scope = create_inklog_scope();
        // Empty scope should not contain InklogModule yet
        assert!(!scope.contains::<super::super::InklogModule>());
    }

    /// Each call creates an independent scope.
    #[test]
    fn scopes_are_independent() {
        let _scope1 = create_inklog_scope();
        let _scope2 = create_inklog_scope();
        // Both scopes exist independently
    }

    /// Insert + require round-trip works for InklogModule.
    #[tokio::test]
    async fn scope_insert_require_roundtrip() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};
        use trait_kit::prelude::*;

        use super::super::InklogModule;

        // Build with main kit first
        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 2,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<dbnexus::DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        let built = kit.build().await.expect("build");
        let db = built.require::<InklogModule>().expect("require from kit");

        // Insert into scope and retrieve
        let scope = create_inklog_scope();
        populate_inklog_scope(&scope, db);
        assert!(scope.contains::<InklogModule>());

        let retrieved = scope.require::<InklogModule>().expect("require from scope");
        assert!(retrieved.is_healthy().await);
    }
}
