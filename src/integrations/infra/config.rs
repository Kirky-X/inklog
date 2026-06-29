// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Config trait - 抽象配置访问
//!
//! 提供配置值的统一访问接口，支持多种基本类型。

/// Config trait - 抽象配置访问
///
/// 提供配置值的类型化访问接口，支持字符串、整数、布尔值和浮点数。
/// 实现必须保证线程安全（`Send + Sync`）。
///
/// # 实现要求
///
/// - 所有方法使用 `&self`（不可变引用），支持并发访问
/// - 返回类型使用 `Option`，不存在的键返回 `None`
/// - 类型转换失败时返回 `None`（静默失败）
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::Config;
///
/// fn example(config: &dyn Config) {
///     let level = config.get_string("log.level").unwrap_or("info".to_string());
///     let port = config.get_int("server.port").unwrap_or(8080) as u16;
///     let debug = config.get_bool("debug").unwrap_or(false);
/// }
/// ```
pub trait Config: Send + Sync {
    /// 获取字符串配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键（支持点分隔的层级路径，如 `"database.host"`）
    ///
    /// # 返回
    ///
    /// 键存在返回 `Some(value)`，否则返回 `None`
    fn get_string(&self, key: &str) -> Option<String>;

    /// 获取整数配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键
    ///
    /// # 返回
    ///
    /// 键存在且可转换为整数返回 `Some(value)`，否则返回 `None`
    fn get_int(&self, key: &str) -> Option<i64>;

    /// 获取布尔配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键
    ///
    /// # 返回
    ///
    /// 键存在且可转换为布尔值返回 `Some(value)`，否则返回 `None`
    fn get_bool(&self, key: &str) -> Option<bool>;

    /// 获取浮点数配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键
    ///
    /// # 返回
    ///
    /// 键存在且可转换为浮点数返回 `Some(value)`，否则返回 `None`
    fn get_float(&self, key: &str) -> Option<f64>;
}

// ============================================================================
// InklogConfigAdapter - InklogConfig 配置适配器实现
// ============================================================================

use crate::InklogConfig;
use crate::InklogError;

/// InklogConfig 配置适配器
///
/// 将 `InklogConfig` 适配为 `Config` trait。
/// 使用点分隔的键路径访问嵌套配置字段。
///
/// # 支持的键路径
///
/// | 键路径 | 映射字段 |
/// |--------|----------|
/// | `global.level` | `config.global.level` |
/// | `global.format` | `config.global.format` |
/// | `global.masking_enabled` | `config.global.masking_enabled` |
/// | `global.auto_fallback` | `config.global.auto_fallback` |
/// | `file_sink.enabled` | `config.file_sink.enabled` |
/// | `file_sink.path` | `config.file_sink.path` |
/// | `file_sink.max_size` | `config.file_sink.max_size` |
/// | `file_sink.compress` | `config.file_sink.compress` |
/// | `console_sink.enabled` | `config.console_sink.enabled` |
/// | `console_sink.colored` | `config.console_sink.colored` |
/// | `database_sink.enabled` | `config.database_sink.enabled` |
/// | `database_sink.url` | `config.database_sink.url` |
/// | `database_sink.pool_size` | `config.database_sink.pool_size` |
/// | `performance.worker_threads` | `config.performance.worker_threads` |
/// | `performance.channel_capacity` | `config.performance.channel_capacity` |
/// | `http_server.enabled` | `config.http_server.enabled` |
/// | `http_server.host` | `config.http_server.host` |
/// | `http_server.port` | `config.http_server.port` |
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::config::{Config, InklogConfigAdapter};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = InklogConfigAdapter::new()?;
///     
///     let level = config.get_string("global.level");
///     let port = config.get_int("http_server.port");
///     let enabled = config.get_bool("file_sink.enabled");
///     
///     Ok(())
/// }
/// ```
pub struct InklogConfigAdapter {
    config: InklogConfig,
}

impl InklogConfigAdapter {
    /// 创建新的配置适配器
    ///
    /// 从搜索路径加载配置文件，并应用环境变量覆盖。
    ///
    /// # 搜索路径（优先级从高到低）
    ///
    /// 1. `$INKLOG_CONFIG_PATH` 环境变量指定的路径
    /// 2. `inklog_config.toml`（当前目录）
    /// 3. `~/.config/inklog/config.toml`
    /// 4. `/etc/inklog/config.toml`
    /// 5. 默认配置
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 错误
    ///
    /// - `InklogError::ConfigError` - 配置加载或验证失败
    pub fn new() -> Result<Self, InklogError> {
        let config = InklogConfig::load_sync()
            .map_err(|e| InklogError::ConfigError(format!("Failed to load config: {}", e)))?;
        Ok(Self { config })
    }

    /// 从现有配置创建适配器
    ///
    /// 用于测试或需要自定义配置加载逻辑的场景。
    ///
    /// # 参数
    ///
    /// * `config` - 已加载的配置实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use inklog::config::InklogConfig;
    /// use inklog::infrastructure::config::InklogConfigAdapter;
    ///
    /// let inklog_config = InklogConfig::default();
    /// let adapter = InklogConfigAdapter::from_config(inklog_config);
    /// ```
    pub fn from_config(config: InklogConfig) -> Self {
        Self { config }
    }

    /// 获取底层配置引用
    ///
    /// 用于需要访问完整配置结构的场景。
    pub fn inner(&self) -> &InklogConfig {
        &self.config
    }

    /// 获取底层配置的可变引用
    ///
    /// 用于需要修改配置的场景（如热更新）。
    pub fn inner_mut(&mut self) -> &mut InklogConfig {
        &mut self.config
    }
}

impl std::convert::AsRef<InklogConfig> for InklogConfigAdapter {
    fn as_ref(&self) -> &InklogConfig {
        &self.config
    }
}

impl std::convert::AsMut<InklogConfig> for InklogConfigAdapter {
    fn as_mut(&mut self) -> &mut InklogConfig {
        &mut self.config
    }
}

impl Default for InklogConfigAdapter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::from_config(InklogConfig::default()))
    }
}

impl Config for InklogConfigAdapter {
    fn get_string(&self, key: &str) -> Option<String> {
        match key {
            // Global config
            "global.level" => Some(self.config.global.level.clone()),
            "global.format" => Some(self.config.global.format.clone()),
            "global.masking_enabled" => Some(self.config.global.masking_enabled.to_string()),
            "global.auto_fallback" => Some(self.config.global.auto_fallback.to_string()),
            "global.fallback_initial_delay_ms" => {
                Some(self.config.global.fallback_initial_delay_ms.to_string())
            }
            "global.fallback_max_delay_ms" => {
                Some(self.config.global.fallback_max_delay_ms.to_string())
            }
            "global.fallback_max_retries" => {
                Some(self.config.global.fallback_max_retries.to_string())
            }

            // Console sink
            "console_sink.enabled" => self
                .config
                .console_sink
                .as_ref()
                .map(|c| c.enabled.to_string()),
            "console_sink.colored" => self
                .config
                .console_sink
                .as_ref()
                .map(|c| c.colored.to_string()),
            "console_sink.stderr_levels" => self.config.console_sink.as_ref().map(|c| {
                c.stderr_levels
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            }),
            "console_sink.masking_enabled" => self
                .config
                .console_sink
                .as_ref()
                .map(|c| c.masking_enabled.to_string()),

            // File sink
            "file_sink.enabled" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.enabled.to_string()),
            "file_sink.path" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.path.to_string_lossy().to_string()),
            "file_sink.max_size" => self.config.file_sink.as_ref().map(|f| f.max_size.clone()),
            "file_sink.rotation_time" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.rotation_time.clone()),
            "file_sink.keep_files" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.keep_files.to_string()),
            "file_sink.compress" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.compress.to_string()),
            "file_sink.compression_level" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.compression_level.to_string()),
            "file_sink.encrypt" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.encrypt.to_string()),
            "file_sink.retention_days" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.retention_days.to_string()),
            "file_sink.max_total_size" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.max_total_size.clone()),
            "file_sink.cleanup_interval_minutes" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.cleanup_interval_minutes.to_string()),
            "file_sink.batch_size" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.batch_size.to_string()),
            "file_sink.flush_interval_ms" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.flush_interval_ms.to_string()),
            "file_sink.masking_enabled" => self
                .config
                .file_sink
                .as_ref()
                .map(|f| f.masking_enabled.to_string()),

            // Database sink
            "database_sink.enabled" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.enabled.to_string()),
            "database_sink.url" => self.config.database_sink.as_ref().map(|d| d.url.clone()),
            "database_sink.pool_size" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.pool_size.to_string()),
            "database_sink.batch_size" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.batch_size.to_string()),
            "database_sink.flush_interval_ms" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.flush_interval_ms.to_string()),
            "database_sink.table_name" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.table_name.clone()),
            "database_sink.archive_format" => self
                .config
                .database_sink
                .as_ref()
                .map(|d| d.archive_format.clone()),

            // Performance
            "performance.worker_threads" => {
                Some(self.config.performance.worker_threads.to_string())
            }
            "performance.channel_capacity" => {
                Some(self.config.performance.channel_capacity.to_string())
            }

            // HTTP server
            "http_server.enabled" => self
                .config
                .http_server
                .as_ref()
                .map(|h| h.enabled.to_string()),
            "http_server.host" => self.config.http_server.as_ref().map(|h| h.host.clone()),
            "http_server.port" => self.config.http_server.as_ref().map(|h| h.port.to_string()),
            "http_server.metrics_path" => self
                .config
                .http_server
                .as_ref()
                .map(|h| h.metrics_path.clone()),
            "http_server.health_path" => self
                .config
                .http_server
                .as_ref()
                .map(|h| h.health_path.clone()),

            // Encryption key env
            "file_sink.encryption_key_env" => self
                .config
                .file_sink
                .as_ref()
                .and_then(|f| f.encryption_key_env.clone()),

            _ => None,
        }
    }

    fn get_int(&self, key: &str) -> Option<i64> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }

    fn get_float(&self, key: &str) -> Option<f64> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }
}

// ============================================================================
// MockConfig - 单元测试用的 Mock 实现
// ============================================================================

use std::collections::HashMap;
use std::sync::RwLock;

/// Mock 配置实现（用于单元测试）
///
/// 提供可编程的配置存储，支持运行时修改配置值。
/// 使用 `RwLock<HashMap>` 保证线程安全。
///
/// # 示例
///
/// ```ignore
/// use inklog::infrastructure::config::{Config, MockConfig};
///
/// // 创建带初始值的配置
/// let config = MockConfig::new()
///     .with_value("level", "debug")
///     .with_value("port", "8080")
///     .with_value("enabled", "true");
///
/// assert_eq!(config.get_string("level"), Some("debug".to_string()));
/// assert_eq!(config.get_int("port"), Some(8080));
/// assert_eq!(config.get_bool("enabled"), Some(true));
///
/// // 运行时修改
/// config.set("level", "info");
/// assert_eq!(config.get_string("level"), Some("info".to_string()));
/// ```
pub struct MockConfig {
    values: RwLock<HashMap<String, String>>,
}

impl MockConfig {
    /// 创建空的 Mock 配置
    pub fn new() -> Self {
        Self {
            values: RwLock::new(HashMap::new()),
        }
    }

    /// 链式方法：添加初始配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键
    /// * `value` - 配置值（字符串形式）
    ///
    /// # 返回
    ///
    /// 返回 `Self`，支持链式调用
    pub fn with_value(self, key: &str, value: &str) -> Self {
        {
            let mut values = self.values.write().unwrap();
            values.insert(key.to_string(), value.to_string());
        }
        self
    }

    /// 运行时修改配置值
    ///
    /// # 参数
    ///
    /// * `key` - 配置键
    /// * `value` - 配置值（字符串形式）
    pub fn set(&self, key: &str, value: &str) {
        let mut values = self.values.write().unwrap();
        values.insert(key.to_string(), value.to_string());
    }
}

impl Default for MockConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Config for MockConfig {
    fn get_string(&self, key: &str) -> Option<String> {
        let values = self.values.read().unwrap();
        values.get(key).cloned()
    }

    fn get_int(&self, key: &str) -> Option<i64> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }

    fn get_float(&self, key: &str) -> Option<f64> {
        self.get_string(key).and_then(|s| s.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InklogConfig;

    // ============================================================================
    // InklogConfigAdapter 测试
    // ============================================================================

    #[test]
    fn test_inklog_config_adapter_from_default_config() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        // 测试默认值
        assert_eq!(adapter.get_string("global.level"), Some("info".to_string()));
        // worker_threads 默认是 3（参见 PerformanceConfig 的 #[config(default = 3usize)]）
        assert_eq!(adapter.get_int("performance.worker_threads"), Some(3));
    }

    #[test]
    fn test_inklog_config_adapter_get_string() {
        let mut config = InklogConfig::default();
        config.global.level = "debug".to_string();

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(
            adapter.get_string("global.level"),
            Some("debug".to_string())
        );
        assert_eq!(adapter.get_string("nonexistent.key"), None);
    }

    #[test]
    fn test_inklog_config_adapter_get_int() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        // worker_threads 默认是 3
        assert_eq!(adapter.get_int("performance.worker_threads"), Some(3));

        // 不存在的键
        assert_eq!(adapter.get_int("nonexistent.key"), None);
    }

    #[test]
    fn test_inklog_config_adapter_get_bool() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        // global.auto_fallback 默认是 true
        assert_eq!(adapter.get_bool("global.auto_fallback"), Some(true));

        // global.masking_enabled 默认是 true
        assert_eq!(adapter.get_bool("global.masking_enabled"), Some(true));
    }

    #[test]
    fn test_inklog_config_adapter_file_sink() {
        let config = InklogConfig {
            file_sink: Some(crate::FileSinkConfig {
                enabled: true,
                path: std::path::PathBuf::from("/var/log/app.log"),
                max_size: "200MB".to_string(),
                compress: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_bool("file_sink.enabled"), Some(true));
        assert_eq!(
            adapter.get_string("file_sink.path"),
            Some("/var/log/app.log".to_string())
        );
        assert_eq!(
            adapter.get_string("file_sink.max_size"),
            Some("200MB".to_string())
        );
        assert_eq!(adapter.get_bool("file_sink.compress"), Some(true));
    }

    #[test]
    fn test_inklog_config_adapter_http_server() {
        let config = InklogConfig {
            http_server: Some(crate::HttpServerConfig {
                enabled: true,
                host: "0.0.0.0".to_string(),
                port: 9090,
                ..Default::default()
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_bool("http_server.enabled"), Some(true));
        assert_eq!(
            adapter.get_string("http_server.host"),
            Some("0.0.0.0".to_string())
        );
        assert_eq!(adapter.get_int("http_server.port"), Some(9090));
    }

    // ============================================================================
    // MockConfig 测试
    // ============================================================================

    #[test]
    fn test_mock_config_new() {
        let config = MockConfig::new();
        assert_eq!(config.get_string("any_key"), None);
    }

    #[test]
    fn test_mock_config_default() {
        let config = MockConfig::default();
        assert_eq!(config.get_string("any_key"), None);
    }

    #[test]
    fn test_mock_config_with_value() {
        let config = MockConfig::new()
            .with_value("level", "debug")
            .with_value("port", "8080")
            .with_value("enabled", "true");

        assert_eq!(config.get_string("level"), Some("debug".to_string()));
        assert_eq!(config.get_string("port"), Some("8080".to_string()));
        assert_eq!(config.get_string("enabled"), Some("true".to_string()));
    }

    #[test]
    fn test_mock_config_get_string() {
        let config = MockConfig::new().with_value("name", "test_value");

        assert_eq!(config.get_string("name"), Some("test_value".to_string()));
        assert_eq!(config.get_string("nonexistent"), None);
    }

    #[test]
    fn test_mock_config_get_int() {
        let config = MockConfig::new()
            .with_value("port", "8080")
            .with_value("invalid_int", "not_a_number");

        assert_eq!(config.get_int("port"), Some(8080));
        assert_eq!(config.get_int("invalid_int"), None);
        assert_eq!(config.get_int("nonexistent"), None);
    }

    #[test]
    fn test_mock_config_get_bool() {
        let config = MockConfig::new()
            .with_value("enabled", "true")
            .with_value("disabled", "false")
            .with_value("invalid_bool", "yes");

        assert_eq!(config.get_bool("enabled"), Some(true));
        assert_eq!(config.get_bool("disabled"), Some(false));
        assert_eq!(config.get_bool("invalid_bool"), None);
        assert_eq!(config.get_bool("nonexistent"), None);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_mock_config_get_float() {
        let config = MockConfig::new()
            .with_value("ratio", "3.14159")
            .with_value("invalid_float", "not_a_float");

        assert_eq!(config.get_float("ratio"), Some(3.14159));
        assert_eq!(config.get_float("invalid_float"), None);
        assert_eq!(config.get_float("nonexistent"), None);
    }

    #[test]
    fn test_mock_config_set_runtime() {
        let config = MockConfig::new().with_value("level", "debug");

        assert_eq!(config.get_string("level"), Some("debug".to_string()));

        // 运行时修改
        config.set("level", "info");
        assert_eq!(config.get_string("level"), Some("info".to_string()));

        // 添加新值
        config.set("new_key", "new_value");
        assert_eq!(config.get_string("new_key"), Some("new_value".to_string()));
    }

    #[test]
    fn test_mock_config_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let config = Arc::new(MockConfig::new().with_value("counter", "0"));

        let mut handles = vec![];

        for i in 0..10 {
            let cfg = Arc::clone(&config);
            handles.push(thread::spawn(move || {
                cfg.set("counter", &i.to_string());
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 最终值应该是最后一次写入的值
        let final_value = config.get_int("counter");
        assert!(final_value.is_some());
        assert!((0..10).contains(&final_value.unwrap()));
    }

    // ============================================================================
    // InklogConfigAdapter 补充测试 - 覆盖剩余 get_string 分支
    // ============================================================================

    #[test]
    fn test_adapter_inner_and_inner_mut() {
        let config = InklogConfig::default();
        let mut adapter = InklogConfigAdapter::from_config(config);

        // inner() 返回底层配置引用
        assert_eq!(adapter.inner().global.level, "info");

        // inner_mut() 返回可变引用
        adapter.inner_mut().global.level = "debug".to_string();
        assert_eq!(adapter.inner().global.level, "debug");
    }

    #[test]
    fn test_adapter_as_ref_as_mut() {
        let config = InklogConfig::default();
        let mut adapter = InklogConfigAdapter::from_config(config);

        // AsRef<InklogConfig>
        let ref_config: &InklogConfig = AsRef::as_ref(&adapter);
        assert_eq!(ref_config.global.level, "info");

        // AsMut<InklogConfig>
        let mut_config: &mut InklogConfig = AsMut::as_mut(&mut adapter);
        mut_config.global.level = "warn".to_string();
        assert_eq!(AsRef::<InklogConfig>::as_ref(&adapter).global.level, "warn");
    }

    #[test]
    fn test_adapter_default() {
        // Default impl 调用 new()，失败时回退到 from_config(default)
        let adapter = InklogConfigAdapter::default();
        // 应该至少能获取到默认配置值
        assert_eq!(adapter.get_string("global.level"), Some("info".to_string()));
    }

    #[test]
    fn test_adapter_global_fallback_fields() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(
            adapter.get_string("global.fallback_initial_delay_ms"),
            Some("1000".to_string())
        );
        assert_eq!(
            adapter.get_string("global.fallback_max_delay_ms"),
            Some("60000".to_string())
        );
        assert_eq!(
            adapter.get_string("global.fallback_max_retries"),
            Some("10".to_string())
        );

        // get_int 应该能解析这些数值
        assert_eq!(
            adapter.get_int("global.fallback_initial_delay_ms"),
            Some(1000)
        );
        assert_eq!(adapter.get_int("global.fallback_max_retries"), Some(10));
    }

    #[test]
    fn test_adapter_console_sink_all_fields() {
        let config = InklogConfig {
            console_sink: Some(crate::ConsoleSinkConfig {
                enabled: true,
                colored: false,
                stderr_levels: vec!["error".to_string(), "fatal".to_string()],
                masking_enabled: true,
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_bool("console_sink.enabled"), Some(true));
        assert_eq!(adapter.get_bool("console_sink.colored"), Some(false));
        assert_eq!(
            adapter.get_string("console_sink.stderr_levels"),
            Some("error,fatal".to_string())
        );
        assert_eq!(adapter.get_bool("console_sink.masking_enabled"), Some(true));
    }

    #[test]
    fn test_adapter_console_sink_none_returns_none() {
        let config = InklogConfig {
            console_sink: None,
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_string("console_sink.enabled"), None);
        assert_eq!(adapter.get_string("console_sink.colored"), None);
        assert_eq!(adapter.get_string("console_sink.stderr_levels"), None);
        assert_eq!(adapter.get_string("console_sink.masking_enabled"), None);
    }

    #[test]
    fn test_adapter_file_sink_all_fields() {
        let config = InklogConfig {
            file_sink: Some(crate::FileSinkConfig {
                enabled: true,
                path: std::path::PathBuf::from("/var/log/app.log"),
                max_size: "200MB".to_string(),
                rotation_time: "hourly".to_string(),
                keep_files: 7,
                compress: false,
                compression_level: 5,
                encrypt: true,
                encryption_key_env: Some("MY_KEY".to_string()),
                retention_days: 14,
                max_total_size: "2GB".to_string(),
                cleanup_interval_minutes: 30,
                batch_size: 200,
                flush_interval_ms: 50,
                masking_enabled: false,
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(
            adapter.get_string("file_sink.rotation_time"),
            Some("hourly".to_string())
        );
        assert_eq!(adapter.get_int("file_sink.keep_files"), Some(7));
        assert_eq!(adapter.get_int("file_sink.compression_level"), Some(5));
        assert_eq!(adapter.get_bool("file_sink.encrypt"), Some(true));
        assert_eq!(adapter.get_int("file_sink.retention_days"), Some(14));
        assert_eq!(
            adapter.get_string("file_sink.max_total_size"),
            Some("2GB".to_string())
        );
        assert_eq!(
            adapter.get_int("file_sink.cleanup_interval_minutes"),
            Some(30)
        );
        assert_eq!(adapter.get_int("file_sink.batch_size"), Some(200));
        assert_eq!(adapter.get_int("file_sink.flush_interval_ms"), Some(50));
        assert_eq!(adapter.get_bool("file_sink.masking_enabled"), Some(false));
        assert_eq!(
            adapter.get_string("file_sink.encryption_key_env"),
            Some("MY_KEY".to_string())
        );
    }

    #[test]
    fn test_adapter_file_sink_none_returns_none() {
        let config = InklogConfig {
            file_sink: None,
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_string("file_sink.enabled"), None);
        assert_eq!(adapter.get_string("file_sink.encryption_key_env"), None);
        assert_eq!(adapter.get_int("file_sink.batch_size"), None);
    }

    #[test]
    fn test_adapter_database_sink_all_fields() {
        let config = InklogConfig {
            database_sink: Some(crate::DatabaseSinkConfig {
                enabled: true,
                url: "postgres://localhost/logs".to_string(),
                pool_size: 20,
                batch_size: 500,
                flush_interval_ms: 250,
                table_name: "app_logs".to_string(),
                archive_format: "parquet".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_bool("database_sink.enabled"), Some(true));
        assert_eq!(adapter.get_int("database_sink.pool_size"), Some(20));
        assert_eq!(adapter.get_int("database_sink.batch_size"), Some(500));
        assert_eq!(
            adapter.get_int("database_sink.flush_interval_ms"),
            Some(250)
        );
        assert_eq!(
            adapter.get_string("database_sink.table_name"),
            Some("app_logs".to_string())
        );
        assert_eq!(
            adapter.get_string("database_sink.archive_format"),
            Some("parquet".to_string())
        );
    }

    #[test]
    fn test_adapter_database_sink_none_returns_none() {
        let config = InklogConfig {
            database_sink: None,
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_string("database_sink.enabled"), None);
        assert_eq!(adapter.get_string("database_sink.url"), None);
    }

    #[test]
    fn test_adapter_http_server_all_fields() {
        let config = InklogConfig {
            http_server: Some(crate::HttpServerConfig {
                enabled: true,
                host: "0.0.0.0".to_string(),
                port: 8080,
                metrics_path: "/custom_metrics".to_string(),
                health_path: "/custom_health".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(
            adapter.get_string("http_server.metrics_path"),
            Some("/custom_metrics".to_string())
        );
        assert_eq!(
            adapter.get_string("http_server.health_path"),
            Some("/custom_health".to_string())
        );
        assert_eq!(adapter.get_int("http_server.port"), Some(8080));
    }

    #[test]
    fn test_adapter_http_server_none_returns_none() {
        let config = InklogConfig {
            http_server: None,
            ..Default::default()
        };

        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_string("http_server.enabled"), None);
        assert_eq!(adapter.get_string("http_server.metrics_path"), None);
        assert_eq!(adapter.get_string("http_server.health_path"), None);
    }

    #[test]
    fn test_adapter_get_float() {
        let config = InklogConfig::default();
        let adapter = InklogConfigAdapter::from_config(config);

        // fallback_initial_delay_ms = 1000, 可解析为 f64
        assert_eq!(
            adapter.get_float("global.fallback_initial_delay_ms"),
            Some(1000.0)
        );
        // 不存在的键 → None
        assert_eq!(adapter.get_float("nonexistent.key"), None);
    }

    #[test]
    fn test_adapter_get_int_invalid_returns_none() {
        let config = InklogConfig::default();
        // global.level = "info"，无法解析为 i64
        let adapter = InklogConfigAdapter::from_config(config);

        assert_eq!(adapter.get_int("global.level"), None);
    }
}
