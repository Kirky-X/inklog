// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Integrations module - external service integrations.

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub mod dbnexus_adapter;
pub mod infra;
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub mod kit;

// Re-export infra types at module level for two-level import paths
pub use infra::{
    Cache, Config, Database, InklogConfigAdapter, MockCache, MockConfig, MockDatabaseAdapter,
    OxCacheAdapter,
};

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub use dbnexus_adapter::DbNexusLogDbAdapter;

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub use kit::InklogModule;
