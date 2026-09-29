// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Sink registry for dynamic sink creation.
//!
//! This module provides a registry pattern for creating sinks dynamically,
//! enabling third-party sink implementations and runtime configuration.

use super::FileSink;
use super::LogSink;
use crate::FileSinkConfig;
use crate::InklogError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Factory trait for creating sinks.
///
/// Implement this trait to create custom sink factories that can be
/// registered with the `SinkRegistry`.
#[async_trait]
pub trait SinkFactory: Send + Sync {
    /// Create a new sink instance.
    async fn create(&self) -> Result<Arc<dyn LogSink>, InklogError>;

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
    pub async fn create(&self, sink_type: &str) -> Result<Arc<dyn LogSink>, InklogError> {
        let factory = self.factories.get(sink_type).ok_or_else(|| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("type", sink_type);
            InklogError::ConfigError(crate::i18n::tr_args("config-unknown_sink_type", args))
        })?;
        factory.create().await
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

#[async_trait]
impl SinkFactory for FileSinkFactory {
    async fn create(&self) -> Result<Arc<dyn LogSink>, InklogError> {
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
            config_schema: file_config_schema(),
        }
    }
}

/// FileSinkConfig 的 JSON Schema：`schema` feature 下填充真实导出，
/// 未启用时维持 None（与 SinkMetadata 的 Option 语义一致）。
///
/// 进程级缓存一次生成：输出是类型的纯函数，而 metadata() 是公开
/// discovery 端口，可能被管理/健康端点轮询，缓存把每次调用从
/// 「新建 SchemaGenerator 走全类型树」降为一次 Value 克隆。
#[cfg(feature = "schema")]
static FILE_SINK_CONFIG_SCHEMA: std::sync::LazyLock<Option<serde_json::Value>> =
    std::sync::LazyLock::new(|| {
        let schema = serde_json::to_value(schemars::schema_for!(crate::FileSinkConfig));
        // Schema→Value 为纯数据序列化，不可失败；debug 构建下显性化，
        // release 的 .ok() 仅为适配 metadata() 的无错误通道签名
        debug_assert!(schema.is_ok(), "FileSinkConfig schema serialization failed");
        schema.ok()
    });

#[cfg(feature = "schema")]
fn file_config_schema() -> Option<serde_json::Value> {
    std::sync::LazyLock::force(&FILE_SINK_CONFIG_SCHEMA).clone()
}

#[cfg(not(feature = "schema"))]
fn file_config_schema() -> Option<serde_json::Value> {
    None
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

    #[tokio::test]
    async fn test_registry_create() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let factory = FileSinkFactory::new(config);
        registry.register(factory);

        let sink = registry.create("file").await;
        assert!(sink.is_ok());

        let nonexistent = registry.create("nonexistent").await;
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

    #[cfg(feature = "schema")]
    #[test]
    fn test_registry_metadata_config_schema_filled_with_feature() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("schema_probe.log"),
            ..Default::default()
        };
        let factory = FileSinkFactory::new(config);

        let metadata = factory.metadata();
        let schema = metadata
            .config_schema
            .as_ref()
            .expect("schema feature must fill FileSinkFactory config_schema");

        // FileSinkConfig 顶层属性可发现（JSON Schema properties 暴露）
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("schema must be an object with properties");
        for field in ["enabled", "path", "max_size"] {
            assert!(props.contains_key(field), "schema must expose `{field}`");
        }
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

    #[test]
    fn test_registry_default() {
        let registry = SinkRegistry::default();
        assert_eq!(registry.list_sinks().len(), 0);
        assert!(!registry.has_sink("file"));
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test1.log"),
            ..Default::default()
        };
        registry.register(FileSinkFactory::new(config));

        assert_eq!(registry.list_sinks().len(), 1);

        registry.clear();
        assert_eq!(registry.list_sinks().len(), 0);
        assert!(!registry.has_sink("file"));
    }

    #[test]
    fn test_registry_unregister_nonexistent() {
        let mut registry = SinkRegistry::new();
        let removed = registry.unregister("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_registry_get_metadata_nonexistent() {
        let registry = SinkRegistry::new();
        assert!(registry.get_metadata("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_registry_create_after_unregister() {
        let mut registry = SinkRegistry::new();

        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        registry.register(FileSinkFactory::new(config));
        let _ = registry.unregister("file");

        // Creating after unregister should fail
        let result = registry.create("file").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_file_sink_factory_metadata() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let factory = FileSinkFactory::new(config);
        let metadata = factory.metadata();
        assert_eq!(metadata.name, "File Sink");
        assert!(metadata.description.contains("rotation"));
        assert!(metadata.features.contains(&"rotation".to_string()));
        assert!(metadata.features.contains(&"compression".to_string()));
        assert!(metadata.features.contains(&"encryption".to_string()));
        assert!(metadata.features.contains(&"batching".to_string()));
        // schema feature 下 factory 填充真实 FileSinkConfig schema，否则维持 None
        #[cfg(feature = "schema")]
        assert!(metadata.config_schema.is_some());
        #[cfg(not(feature = "schema"))]
        assert!(metadata.config_schema.is_none());
    }
}
