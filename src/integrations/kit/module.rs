// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! `InklogModule` — trait-kit module definition for the inklog logging system.
//!
//! [`InklogModule`] implements [`trait_kit::core::module::Module`] and exposes
//! a [`trait_kit::kit::Kit`] as its capability. The [`InklogModuleBuilder`]
//! constructs a `Kit` pre-populated with the [`InklogConfig`] typed config,
//! enabling downstream trait-kit modules to require the config and register
//! additional capabilities (cache, database) on the same shared `Kit`.
//!
//! # Example
//!
//! ```ignore
//! use inklog::integrations::kit::{InklogModule, InklogModuleBuilder};
//! use trait_kit::prelude::*;
//!
//! let kit = InklogModuleBuilder::new()
//!     .config(inklog::InklogConfig::default())
//!     .build()?;
//! ```

use trait_kit::core::builder::{ModuleBuilder, WithConfig};
use trait_kit::core::marker::NoRequirements;
use trait_kit::core::module::Module;
use trait_kit::kit::Kit;

use crate::integrations::kit::keys::InklogConfigKey;
use crate::{InklogConfig, InklogError};

/// trait-kit module definition for the inklog logging system.
///
/// The module's capability is a [`Kit`] pre-populated with [`InklogConfig`].
/// Use [`InklogModuleBuilder`] to construct it.
pub struct InklogModule;

impl Module for InklogModule {
    const NAME: &'static str = "inklog";

    type Config = InklogConfig;
    type Requirements = NoRequirements;
    type Capability = Kit;
    type Error = InklogError;
    type Builder = InklogModuleBuilder;
}

/// Builder for [`InklogModule`].
///
/// Produces a [`Kit`] with the provided [`InklogConfig`] registered under
/// [`InklogConfigKey`]. Additional capabilities (cache, database) can be
/// registered on the returned `Kit` after build.
pub struct InklogModuleBuilder {
    config: Option<InklogConfig>,
}

impl InklogModuleBuilder {
    /// Create a new builder with no config (use `WithConfig::config()` to set it).
    pub fn new() -> Self {
        Self { config: None }
    }
}

impl Default for InklogModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WithConfig<InklogModule> for InklogModuleBuilder {
    fn config(mut self, config: InklogConfig) -> Self {
        self.config = Some(config);
        self
    }
}

impl ModuleBuilder<InklogModule> for InklogModuleBuilder {
    fn build(self) -> Result<Kit, InklogError> {
        let kit = Kit::new();
        let config = self.config.unwrap_or_default();
        kit.set_config::<InklogConfigKey>(config);
        Ok(kit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use trait_kit::core::marker::NoConfig;

    #[test]
    fn module_name() {
        assert_eq!(InklogModule::NAME, "inklog");
    }

    #[test]
    fn builder_default_uses_default_config() {
        let kit = InklogModuleBuilder::default().build().unwrap();
        let handle = kit.config::<InklogConfigKey>().unwrap();
        assert_eq!(handle.load().global.level, "info");
    }

    #[test]
    fn builder_with_config_stores_provided_config() {
        let mut config = InklogConfig::default();
        config.global.level = "debug".to_string();

        let kit = InklogModuleBuilder::new().config(config).build().unwrap();

        let handle = kit.config::<InklogConfigKey>().unwrap();
        assert_eq!(handle.load().global.level, "debug");
    }

    #[test]
    fn builder_new_starts_empty() {
        let builder = InklogModuleBuilder::new();
        assert!(builder.config.is_none());
    }

    #[test]
    fn built_kit_is_empty_of_capabilities() {
        use crate::integrations::kit::keys::CacheCapabilityKey;

        let kit = InklogModuleBuilder::default().build().unwrap();
        assert!(!kit.contains::<CacheCapabilityKey>());
    }

    #[test]
    fn built_kit_can_register_additional_capabilities() {
        use crate::integrations::infra::{Cache, MockCache};
        use crate::integrations::kit::keys::CacheCapabilityKey;

        let kit = InklogModuleBuilder::default().build().unwrap();
        let cache: Arc<dyn Cache> = Arc::new(MockCache::new());
        kit.provide::<CacheCapabilityKey>(cache).unwrap();

        assert!(kit.contains::<CacheCapabilityKey>());
    }

    #[test]
    fn no_config_marker_is_default() {
        let _ = NoConfig;
    }

    #[test]
    fn no_requirements_marker_is_default() {
        let _ = NoRequirements;
    }
}
