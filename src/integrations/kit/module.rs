// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! `InklogModule` — trait-kit 0.2.2 `AsyncKit` integration for inklog.
//!
//! Phase 6 (T044 Red / T045 Green) of the `trait-kit-async-integration`
//! change. Wires inklog's `LogDbProvider` abstraction into the `AsyncKit`
//! dependency injection framework, depending on `DbNexusModule` for the
//! database pool capability.
//!
//! # Rule 7 divergences from `design.md` / `spec.md`
//!
//! `spec.md` R-inklog-module-003 specifies:
//!
//! 1. `AsyncAutoBuilder::Capability = Arc<dyn LogSink + Send + Sync>` —
//!    `LoggerManager` does **not** implement `LogSink` (the trait is for
//!    sink implementations like `ConsoleSink` / `FileSink`, not the
//!    manager). Returning `Arc<LoggerManager> as Arc<dyn LogSink>` would
//!    not compile. We return `Arc<dyn LogDbProvider + Send + Sync>`
//!    instead — the capability that `InklogModule` actually produces.
//!    Consumers can retrieve it via `kit.require::<InklogModule>()` and
//!    inject into `LoggerManager::builder().with_database(...)` themselves
//!    (note: `with_database` takes `Arc<dyn Database>`, not
//!    `Arc<dyn LogDbProvider>` — a future change can add a bridge adapter
//!    from `LogDbProvider` to `Database`).
//!
//! 2. `build` body: `kit.require::<DbNexusModule>()` → wrap as
//!    `DbNexusLogDbAdapter` → `kit.config::<InklogConfig>()` →
//!    `LoggerManager::builder().config(config).database(adapter).build()`.
//!    `LoggerBuilder` has no `.database(adapter)` method accepting a
//!    `LogDbProvider` — only `.with_database(Arc<dyn Database>)` or
//!    `.database(url: impl Into<String>)`. Rather than modifying
//!    `LoggerBuilder` (out of scope for T044/T045), `InklogModule::build`
//!    returns the `DbNexusLogDbAdapter` directly as a
//!    `Arc<dyn LogDbProvider>` capability. A follow-up change can add
//!    a `LogDbProvider → Database` bridge and full `LoggerManager`
//!    construction inside `build`.
//!
//! 3. `table_name` is hardcoded to `"logs"` (matching the default in
//!    `DbNexusAdapter::new`). A future change can read this from
//!    `InklogConfig` or `DatabaseSinkConfig`.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use trait_kit::prelude::*;

use dbnexus::DbNexusModule;

use crate::integrations::dbnexus_adapter::DbNexusLogDbAdapter;
use crate::{InklogError, LogDbProvider};

/// trait-kit `AsyncKit` module that constructs an inklog `LogDbProvider`.
///
/// Depends on `DbNexusModule` (registered first via topological sort).
/// Register with `AsyncKit::register::<InklogModule>()`, then
/// `kit.build().await` and retrieve the capability with
/// `kit.require::<InklogModule>()`.
///
/// The returned `Arc<dyn LogDbProvider + Send + Sync>` wraps a
/// `DbNexusLogDbAdapter` that proxies `execute_log` / `batch_insert`
/// through the dbnexus `ConnectionPool`. See the module-level docs for
/// the design-divergence rationale (spec.md wrote
/// `Capability = Arc<dyn LogSink>`, but `LoggerManager` does not
/// implement `LogSink`).
pub struct InklogModule;

impl ModuleMeta for InklogModule {
    const NAME: &'static str = "inklog";

    fn dependencies() -> &'static [(&'static str, TypeId)] {
        static DEPS: OnceLock<Vec<(&'static str, TypeId)>> = OnceLock::new();
        DEPS.get_or_init(|| vec![("dbnexus", TypeId::of::<DbNexusModule>())])
            .as_slice()
    }
}

impl AsyncAutoBuilder for InklogModule {
    type Capability = Arc<dyn LogDbProvider + Send + Sync>;
    type Error = InklogError;

    fn build<'a>(
        kit: &'a AsyncKit,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Capability, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Require DbNexusModule capability (Arc<dyn ConnectionPool + Send + Sync>).
            let pool = kit
                .require::<DbNexusModule>()
                .map_err(|e| InklogError::DatabaseError(format!("require DbNexusModule: {e}")))?;

            // 2. Wrap in DbNexusLogDbAdapter — adapts ConnectionPool to LogDbProvider.
            let adapter = DbNexusLogDbAdapter::new(pool, "logs");

            // 3. Return as Arc<dyn LogDbProvider + Send + Sync>.
            Ok(Arc::new(adapter) as Arc<dyn LogDbProvider + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-inklog-module-003 #1: `InklogModule::NAME == "inklog"`.
    #[test]
    fn inklog_module_meta_name() {
        assert_eq!(InklogModule::NAME, "inklog");
    }

    /// R-inklog-module-003 #2: `InklogModule::dependencies()` declares
    /// a dependency on `DbNexusModule`.
    #[test]
    fn inklog_module_meta_dependencies() {
        let deps = InklogModule::dependencies();
        assert_eq!(deps.len(), 1, "InklogModule should depend on 1 module");
        assert_eq!(deps[0].0, "dbnexus", "dep name should be 'dbnexus'");
        assert_eq!(
            deps[0].1,
            TypeId::of::<DbNexusModule>(),
            "dep TypeId should match DbNexusModule"
        );
    }

    /// R-inklog-module-003 #3: `InklogModule` satisfies `AsyncAutoBuilder`
    /// trait bounds — `Capability: Clone + Send + Sync + 'static` and
    /// `Error: std::error::Error + Send + 'static`.
    #[test]
    fn inklog_module_satisfies_async_auto_builder_bounds() {
        fn assert_cap<T: Clone + Send + Sync + 'static>() {}
        assert_cap::<Arc<dyn LogDbProvider + Send + Sync>>();
        fn assert_err<T: std::error::Error + Send + 'static>() {}
        assert_err::<InklogError>();
    }

    /// R-inklog-module-003 #4: Full integration — register OxcacheModule +
    /// DbNexusModule + InklogModule, set configs, build, require
    /// InklogModule → get a working `Arc<dyn LogDbProvider + Send + Sync>`.
    #[tokio::test]
    async fn inklog_module_build_returns_log_db_provider() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        let kit = kit.build().await.expect("AsyncKit::build");

        let provider: Arc<dyn LogDbProvider + Send + Sync> =
            kit.require::<InklogModule>().expect("require InklogModule");

        // Verify the provider is usable — execute a DDL statement.
        provider
            .execute_log("CREATE TABLE IF NOT EXISTS logs (id INTEGER PRIMARY KEY)")
            .await
            .expect("execute_log should succeed");
    }

    /// R-inklog-module-003 #5: build fails with a clear error if
    /// DbNexusModule is not registered (dependency missing).
    #[tokio::test]
    async fn inklog_module_build_fails_without_dbnexus() {
        let mut kit = AsyncKit::new();
        // Register only InklogModule — DbNexusModule is missing.
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        let err = kit.build().await.expect_err("build should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("dbnexus"),
            "error should mention dbnexus dependency, got: {msg}"
        );
    }
}
