// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `InklogModule` — trait-kit 0.4 `AsyncKit` integration for inklog.
//!
//! Wires inklog's `Database` abstraction into the `AsyncKit` dependency
//! injection framework, depending on `DbNexusModule` for the database
//! pool capability.
//!
//! `InklogModule::build` retrieves `Arc<dyn ConnectionPool + Send + Sync>`
//! from `DbNexusModule`, wraps it in `DbNexusAdapter` (which implements
//! `Database`), and returns it as `Arc<dyn Database + Send + Sync>`.
//! Consumers can inject this directly into `DatabaseSink` via
//! `LoggerBuilder::with_database(...)`.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use trait_kit::AsyncReady;
use trait_kit::prelude::*;

use dbnexus::DbNexusModule;

use crate::InklogError;
use crate::integrations::infra::Database;
use crate::integrations::infra::database::DbNexusAdapter;

/// trait-kit `AsyncKit` module that constructs an inklog `Database` impl.
///
/// Depends on `DbNexusModule` (registered first via topological sort).
/// Register with `AsyncKit::register::<InklogModule>()`, then
/// `kit.build().await` and retrieve the capability with
/// `kit.require::<InklogModule>()`.
///
/// The returned `Arc<dyn Database + Send + Sync>` wraps a `DbNexusAdapter`
/// that proxies `insert_batch` / `is_healthy` through the dbnexus
/// `ConnectionPool`. This capability can be injected directly into
/// `LoggerBuilder::with_database(...)`.
///
/// # Lifecycle
///
/// Implements `AsyncLifecycle`:
/// - `on_ready`: verifies `DbNexusModule` capability is accessible in the built kit.
/// - `on_shutdown`: logs inklog module shutdown (connection pool is owned by dbnexus).
///
/// # Health
///
/// Implements `AsyncHealthCheck`:
/// - `check`: returns `Healthy` when the database capability is present in the kit.
///   For detailed async runtime connectivity checks, use `Database::is_healthy()` directly.
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
    type Capability = Arc<dyn Database + Send + Sync>;
    type Error = InklogError;

    fn build<'a>(
        kit: &'a AsyncKit,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Capability, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Require DbNexusModule capability (Arc<dyn ConnectionPool + Send + Sync>).
            let pool = kit
                .require::<DbNexusModule>()
                .map_err(|e| InklogError::database_error(format!("require DbNexusModule: {e}")))?;

            // 2. Wrap in DbNexusAdapter — adapts ConnectionPool to Database.
            let adapter = DbNexusAdapter::from_connection_pool(
                pool,
                crate::support::io::sink::entity::TABLE_NAME,
            )?;

            // 3. Return as Arc<dyn Database + Send + Sync>.
            Ok(Arc::new(adapter) as Arc<dyn Database + Send + Sync>)
        })
    }
}

impl AsyncLifecycle for InklogModule {
    fn on_ready<'a>(
        kit: &'a AsyncKit<AsyncReady>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            // Verify DbNexusModule capability is accessible after all modules are built.
            // This catches missing or failed database dependencies early.
            kit.require::<DbNexusModule>().map_err(|e| {
                InklogError::database_error(format!(
                    "InklogModule on_ready: DbNexusModule not available: {e}"
                ))
            })?;
            tracing::debug!("InklogModule: on_ready — database dependency verified");
            Ok(())
        })
    }

    fn on_shutdown<'a>(
        _cap: &'a Self::Capability,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {
            // Connection pool lifecycle is managed by DbNexusModule.
            // InklogModule only releases its adapter wrapper here.
            tracing::debug!("InklogModule: on_shutdown — releasing database adapter");
        })
    }
}

impl AsyncHealthCheck for InklogModule {
    fn check(_cap: &Self::Capability) -> HealthStatus {
        // Capability presence confirms the database adapter was successfully built
        // and the underlying connection pool was established during build().
        //
        // Detailed async runtime connectivity checks should use
        // `Database::is_healthy()` directly (e.g. from the HTTP health endpoint).
        HealthStatus::Healthy
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
        assert_cap::<Arc<dyn Database + Send + Sync>>();
        fn assert_err<T: std::error::Error + Send + 'static>() {}
        assert_err::<InklogError>();
    }

    /// R-inklog-module-003 #4: Full integration — register OxcacheModule +
    /// DbNexusModule + InklogModule, set configs, build, require
    /// InklogModule → get a working `Arc<dyn Database + Send + Sync>`.
    #[tokio::test]
    async fn inklog_module_build_returns_database() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 5,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        let kit = kit.build().await.expect("AsyncKit::build");

        let db: Arc<dyn Database + Send + Sync> =
            kit.require::<InklogModule>().expect("require InklogModule");

        // Verify the database is usable — health check should pass.
        assert!(db.is_healthy().await);
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

    // ========================================================================
    // AsyncHealthCheck tests
    // ========================================================================

    /// AsyncHealthCheck::check returns Healthy for a valid database capability.
    #[tokio::test]
    async fn inklog_module_health_check_returns_healthy() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 5,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        let built = kit.build().await.expect("AsyncKit::build");

        let db: Arc<dyn Database + Send + Sync> = built
            .require::<InklogModule>()
            .expect("require InklogModule");
        let status = InklogModule::check(&db);
        assert_eq!(status, HealthStatus::Healthy);
    }

    // ========================================================================
    // AsyncLifecycle tests
    // ========================================================================

    /// on_ready succeeds when DbNexusModule is registered and accessible.
    #[tokio::test]
    async fn inklog_module_lifecycle_on_ready_succeeds() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 5,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        // register_lifecycle enables on_ready/on_shutdown hooks
        kit.register_lifecycle::<InklogModule>();
        // build() invokes on_ready internally — should not error
        let built = kit
            .build()
            .await
            .expect("build with lifecycle should succeed");
        drop(built);
    }

    /// on_shutdown completes without panic after a successful build.
    #[tokio::test]
    async fn inklog_module_lifecycle_shutdown_completes() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 5,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        kit.register_lifecycle::<InklogModule>();
        let built = kit.build().await.expect("build");
        // shutdown() invokes on_shutdown for each lifecycle-registered module
        built.shutdown();
    }

    /// Full integration: lifecycle + health check working together.
    #[tokio::test]
    async fn inklog_module_lifecycle_and_health_full_integration() {
        use dbnexus::foundation::config::DbConfig;
        use oxcache::integrations::kit::{OxcacheConfig, OxcacheModule};

        let mut kit = AsyncKit::new();
        kit.set_config(OxcacheConfig::default());
        kit.set_config(DbConfig {
            url: "sqlite::memory:".to_string(),
            pool_config: dbnexus::foundation::config::PoolConfig {
                max_connections: 5,
                min_connections: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        kit.register::<OxcacheModule>()
            .expect("register OxcacheModule");
        kit.register::<DbNexusModule>()
            .expect("register DbNexusModule");
        kit.register::<InklogModule>()
            .expect("register InklogModule");
        kit.register_lifecycle::<InklogModule>();
        kit.register_health_check::<InklogModule>();
        let built = kit.build().await.expect("build");

        // Health check via kit
        let status = built
            .health_check::<InklogModule>()
            .expect("health_check should succeed");
        assert_eq!(status, HealthStatus::Healthy);

        // Shutdown
        built.shutdown();
    }
}
