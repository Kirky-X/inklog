// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Sink registry for dynamic sink creation.
//!
//! This module provides a registry pattern for creating sinks dynamically,
//! enabling third-party sink implementations and runtime configuration.

use crate::config::FileSinkConfig;
use crate::error::InklogError;
use crate::sink::file::FileSink;
use crate::sink::LogSink;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Factory trait for creating sinks.
///
/// Implement this trait to create custom sink factories that can be
/// registered with the `SinkRegistry`.
pub trait SinkFactory: Send + Sync {
    /// Create a new sink instance.
    fn create(&self) -> Result<Arc<dyn LogSink>, InklogError>;

    /// Get the sink type name.
    fn sink_type(&self) -> &'static str;

    /// Get sink metadata for discovery.
    fn metadata(&self) -> SinkMetadata;
}

/// Metadata for a sink type.
#[derive(Debug, Clone)]
pub struct SinkMetadata {
    /// Human-readable name
    pub name: String,
    /// Description of the sink
    pub description: String,
    /// Supported features
    pub features: Vec<String>,
    /// Configuration schema (JSON Schema format)
    pub config_schema: Option<serde_json::Value>,
}

/// Registry for managing sink factories.
pub struct SinkRegistry {
    factories: HashMap<String, Box<dyn SinkFactory>>,
}

impl Default for SinkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SinkRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a sink factory.
    pub fn register<F: SinkFactory + 'static>(&mut self, factory: F) {
        let sink_type = factory.sink_type().to_string();
        info!("Registering sink factory: {}", sink_type);
        self.factories.insert(sink_type, Box::new(factory));
    }

    /// Create a sink by type name.
    pub fn create(&self, sink_type: &str) -> Result<Arc<dyn LogSink>, InklogError> {
        let factory = self
            .factories
            .get(sink_type)
            .ok_or_else(|| InklogError::ConfigError(format!("Unknown sink type: {}", sink_type)))?;
        factory.create()
    }

    /// List all registered sink types.
    pub fn list_sinks(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }

    /// Get metadata for a sink type.
    pub fn get_metadata(&self, sink_type: &str) -> Option<SinkMetadata> {
        self.factories.get(sink_type).map(|f| f.metadata())
    }

    /// Check if a sink type is registered.
    pub fn has_sink(&self, sink_type: &str) -> bool {
        self.factories.contains_key(sink_type)
    }

    /// Unregister a sink type.
    pub fn unregister(&mut self, sink_type: &str) -> Option<Box<dyn SinkFactory>> {
        self.factories.remove(sink_type)
    }

    /// Clear all registered factories.
    pub fn clear(&mut self) {
        self.factories.clear();
    }
}

/// Factory for creating FileSink instances.
pub struct FileSinkFactory {
    config: FileSinkConfig,
}

impl FileSinkFactory {
    /// Create a new factory with the given configuration.
    pub fn new(config: FileSinkConfig) -> Self {
        Self { config }
    }
}

impl SinkFactory for FileSinkFactory {
    fn create(&self) -> Result<Arc<dyn LogSink>, InklogError> {
        let sink = FileSink::new(self.config.clone())?;
        Ok(Arc::new(sink))
    }

    fn sink_type(&self) -> &'static str {
        "file"
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata {
            name: "File Sink".to_string(),
            description: "Writes logs to files with rotation, compression, and encryption support."
                .to_string(),
            features: vec![
                "rotation".to_string(),
                "compression".to_string(),
                "encryption".to_string(),
                "batching".to_string(),
            ],
            config_schema: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_registry_registration() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        assert!(registry.has_sink("file"));
        assert!(!registry.has_sink("nonexistent"));
    }

    #[test]
    fn test_registry_create() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        let sink = registry.create("file");
        assert!(sink.is_ok());

        let nonexistent = registry.create("nonexistent");
        assert!(nonexistent.is_err());
    }

    #[test]
    fn test_registry_list_sinks() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        let sinks = registry.list_sinks();
        assert_eq!(sinks.len(), 1);
        assert!(sinks.contains(&"file"));
    }

    #[test]
    fn test_registry_metadata() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        let metadata = registry.get_metadata("file");
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.name, "File Sink");
        assert!(metadata.features.contains(&"rotation".to_string()));
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        assert!(registry.has_sink("file"));

        let removed = registry.unregister("file");
        assert!(removed.is_some());
        assert!(!registry.has_sink("file"));
    }
}
