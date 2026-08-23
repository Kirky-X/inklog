// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Integrations module - external service integrations.

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
