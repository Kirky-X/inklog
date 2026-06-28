// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Typed keys for Kit capability and config registration.
//!
//! Provides [`CapabilityKey`] implementations for inklog's core infrastructure
//! traits ([`Config`], [`Cache`], [`Database`]) and a [`ConfigKey`] implementation
//! for typed [`InklogConfig`] storage.
//!
//! # Naming
//!
//! Capability keys use the `*CapabilityKey` suffix to avoid a naming collision
//! with [`trait_kit::core::config::ConfigKey`] (the trait), which would clash
//! with a bare `ConfigKey` struct under glob imports of both
//! `trait_kit::prelude::*` and `inklog::integrations::kit::*`.

#[cfg(feature = "dbnexus")]
use crate::integrations::infra::Database;
use crate::integrations::infra::{Cache, Config};
use crate::InklogConfig;
use trait_kit::core::capability::CapabilityKey;
use trait_kit::core::config::ConfigKey as ConfigKeyTrait;

/// Capability key for the [`Config`] trait — dynamic key-value config access.
///
/// Register an `Arc<dyn Config>` via `kit.provide::<ConfigCapabilityKey>(value)`
/// and retrieve it via `kit.require::<ConfigCapabilityKey>()`.
pub struct ConfigCapabilityKey;

impl CapabilityKey for ConfigCapabilityKey {
    type Capability = dyn Config;
    const NAME: &'static str = "config";
}

/// Capability key for the [`Cache`] trait.
///
/// Register an `Arc<dyn Cache>` via `kit.provide::<CacheCapabilityKey>(value)`
/// and retrieve it via `kit.require::<CacheCapabilityKey>()`.
pub struct CacheCapabilityKey;

impl CapabilityKey for CacheCapabilityKey {
    type Capability = dyn Cache;
    const NAME: &'static str = "cache";
}

/// Capability key for the [`Database`] trait (requires `dbnexus` feature).
///
/// Register an `Arc<dyn Database>` via
/// `kit.provide::<DatabaseCapabilityKey>(value)` and retrieve it via
/// `kit.require::<DatabaseCapabilityKey>()`.
#[cfg(feature = "dbnexus")]
pub struct DatabaseCapabilityKey;

#[cfg(feature = "dbnexus")]
impl CapabilityKey for DatabaseCapabilityKey {
    type Capability = dyn Database;
    const NAME: &'static str = "database";
}

/// Config key for typed [`InklogConfig`] storage in Kit.
///
/// Store the config via `kit.set_config::<InklogConfigKey>(config)` and read it
/// via `kit.config::<InklogConfigKey>()?` (returns a `ConfigHandle<InklogConfig>`).
///
/// Unlike [`ConfigCapabilityKey`] (which exposes dynamic key-value access via
/// the `Config` trait), `InklogConfigKey` stores the full typed `InklogConfig`
/// struct and supports hot-reload through [`ConfigHandle::set`](trait_kit::core::config::ConfigHandle::set).
pub struct InklogConfigKey;

impl ConfigKeyTrait for InklogConfigKey {
    type Config = InklogConfig;
    const NAME: &'static str = "inklog_config";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_capability_key_name() {
        assert_eq!(ConfigCapabilityKey::NAME, "config");
    }

    #[test]
    fn cache_capability_key_name() {
        assert_eq!(CacheCapabilityKey::NAME, "cache");
    }

    #[cfg(feature = "dbnexus")]
    #[test]
    fn database_capability_key_name() {
        assert_eq!(DatabaseCapabilityKey::NAME, "database");
    }

    #[test]
    fn inklog_config_key_name() {
        assert_eq!(InklogConfigKey::NAME, "inklog_config");
    }

    #[test]
    fn kit_registers_config_capability() {
        use crate::integrations::infra::MockConfig;
        use std::sync::Arc;
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        let config: Arc<dyn Config> = Arc::new(MockConfig::new());
        kit.provide::<ConfigCapabilityKey>(config).unwrap();

        assert!(kit.contains::<ConfigCapabilityKey>());
        let retrieved = kit.require::<ConfigCapabilityKey>().unwrap();
        assert!(retrieved.get_string("any").is_none() || retrieved.get_string("any").is_some());
    }

    #[test]
    fn kit_registers_cache_capability() {
        use crate::integrations::infra::MockCache;
        use std::sync::Arc;
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        let cache: Arc<dyn Cache> = Arc::new(MockCache::new());
        kit.provide::<CacheCapabilityKey>(cache).unwrap();

        assert!(kit.contains::<CacheCapabilityKey>());
    }

    #[test]
    fn kit_stores_inklog_config() {
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        let config = InklogConfig::default();
        kit.set_config::<InklogConfigKey>(config);

        assert!(kit.contains_config::<InklogConfigKey>());
        let handle = kit.config::<InklogConfigKey>().unwrap();
        assert_eq!(handle.load().global.level, "info");
    }

    #[test]
    fn kit_config_handle_supports_hot_reload() {
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        kit.set_config::<InklogConfigKey>(InklogConfig::default());
        let handle = kit.config::<InklogConfigKey>().unwrap();

        let mut new_config = InklogConfig::default();
        new_config.global.level = "debug".to_string();
        handle.set(new_config);

        assert_eq!(handle.load().global.level, "debug");
    }

    #[test]
    fn duplicate_capability_registration_fails() {
        use crate::integrations::infra::MockCache;
        use std::sync::Arc;
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        let cache: Arc<dyn Cache> = Arc::new(MockCache::new());
        kit.provide::<CacheCapabilityKey>(cache).unwrap();

        let second: Arc<dyn Cache> = Arc::new(MockCache::new());
        assert!(kit.provide::<CacheCapabilityKey>(second).is_err());
    }

    #[test]
    fn missing_capability_returns_error() {
        use trait_kit::kit::Kit;

        let kit = Kit::new();
        assert!(kit.require::<CacheCapabilityKey>().is_err());
    }
}
