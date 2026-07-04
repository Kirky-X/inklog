// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! trait-kit integration — capability registry and module definitions.
//!
//! Provides typed keys for registering inklog's core infrastructure capabilities
//! ([`Config`](crate::integrations::infra::Config), [`Cache`](crate::integrations::infra::Cache),
//! [`Database`](crate::integrations::infra::Database)) and the [`crate::InklogConfig`] value
//! in a [`trait_kit::kit::Kit`].
//!
//! # Overview
//!
//! | Key | Kind | Stored Type |
//! |-----|------|-------------|
//! | [`ConfigCapabilityKey`] | Capability | `Arc<dyn Config>` |
//! | [`CacheCapabilityKey`] | Capability | `Arc<dyn Cache>` |
//! | [`DatabaseCapabilityKey`] | Capability | `Arc<dyn Database>` (dbnexus feature) |
//! | [`InklogConfigKey`] | Config | `InklogConfig` |
//!
//! [`InklogModule`] ties these together as a [`trait_kit::core::module::Module`]
//! whose [`InklogModuleBuilder`] produces a `Kit` pre-populated with the config.

pub mod keys;
pub mod module;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
pub use keys::DatabaseCapabilityKey;
pub use keys::{CacheCapabilityKey, ConfigCapabilityKey, InklogConfigKey};
pub use module::{InklogModule, InklogModuleBuilder};
