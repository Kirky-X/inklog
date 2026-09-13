// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Integrations module - external service integrations.

#[cfg(feature = "dbnexus-audit")]
pub mod audit_bridge;
#[cfg(feature = "config-confers")]
pub mod confers_config;
pub mod infra;
#[cfg(all(
    feature = "kit",
    any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    )
))]
pub mod kit;

// dbnexus AuditStorage 端口的 inklog 写入桥（审计事件 → 结构化日志 → 落库）
#[cfg(feature = "dbnexus-audit")]
pub use audit_bridge::InklogAuditStorage;
// confers 配置加载 + watch 热更新
#[cfg(feature = "config-confers")]
pub use confers_config::{ConfersConfigWatcher, HotReloadValues, load_config_via_confers};

// Re-export infra types at module level for two-level import paths
pub use infra::{
    Cache, Config, Database, InklogConfigAdapter, OxCacheAdapter, OxCacheAdapterBuilder,
};
// mock 仅测试面可见（src 内联测试经 cfg(test)；外部消费者显式 test-utils）
#[cfg(any(test, feature = "test-utils"))]
pub use infra::{MockCache, MockConfig, MockDatabaseAdapter};

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
pub use infra::DbNexusAdapter;

#[cfg(all(
    feature = "kit",
    any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    )
))]
pub use kit::{InklogBuildObserver, InklogModule, create_inklog_scope, populate_inklog_scope};
