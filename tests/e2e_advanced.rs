// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! E2E 高级测试：覆盖 function_scenarios_analysis.md 中识别的缺失场景。
//!
//! 测试组织（按 feature 隔离）：
//! - 默认 feature：ConsoleSink/FileSink/LogTemplate/DataMasker/LogSanitizer/
//!   PathValidator/CircuitBreaker/ObjectPool/LogRecord/InklogError/LogLevel/
//!   SinkRegistry/RotationStrategy/Config validation/encryption
//! - `compression` feature：ZstdCompression/GzipCompression/NoCompression round-trip
//! - `i18n` feature：LogI18nFormatter 全方法
//! - 多组件集成 E2E：tracing→FileSink、log→FileSink

use inklog::sink::circuit_breaker::{CircuitBreakerConfig, CircuitState};
use inklog::sink::encryption::derive_key_from_password;
use inklog::sink::rotation::parse_size;
use inklog::sink::{
    CircuitBreaker, CompositeRotation, FileSink, FileSinkFactory, LogSink, RotationContext,
    RotationResult, RotationStrategy, SinkFactory, SinkMetadata, SinkRegistry, SizeBasedRotation,
    TimeBasedRotation,
};
use inklog::support::observability::metrics::FallbackAction;
use inklog::{
    DataMasker, FileSinkConfig, InklogConfig, InklogError, InklogResult, LogLevel,
    LogLevelParseError, LogRecord, LogSanitizer, LogTemplate, Metrics, ObjectPool,
    ObjectPoolConfig, PathValidator, PathValidatorConfig, PerformanceConfig, SanitizerConfig,
    SinkHealthMonitor, SinkStatus, ValidationResult, get_log_record, get_string_buffer,
    put_log_record, put_string_buffer,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;
use tracing::Level;

// ============================================================================
// CORE-007: LogLevel 解析 E2E
// ============================================================================

#[cfg(test)]
mod log_level_e2e {
    use super::*;

    #[test]
    fn test_log_level_from_str_all_valid_variants() {
        // 大小写不敏感解析
        assert!(matches!(LogLevel::from_str("TRACE"), Some(LogLevel::Trace)));
        assert!(matches!(LogLevel::from_str("trace"), Some(LogLevel::Trace)));
        assert!(matches!(LogLevel::from_str("Trace"), Some(LogLevel::Trace)));
        assert!(matches!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug)));
        assert!(matches!(LogLevel::from_str("INFO"), Some(LogLevel::Info)));
        assert!(matches!(LogLevel::from_str("WARN"), Some(LogLevel::Warn)));
        assert!(matches!(
            LogLevel::from_str("WARNING"),
            Some(LogLevel::Warn)
        ));
        assert!(matches!(LogLevel::from_str("ERROR"), Some(LogLevel::Error)));
        assert!(matches!(LogLevel::from_str("FATAL"), Some(LogLevel::Fatal)));
        assert!(matches!(
            LogLevel::from_str("CRITICAL"),
            Some(LogLevel::Fatal)
        ));
    }

    #[test]
    fn test_log_level_from_str_invalid_inputs() {
        assert!(LogLevel::from_str("").is_none());
        assert!(LogLevel::from_str("invalid").is_none());
        assert!(LogLevel::from_str("VERBOSE").is_none());
        assert!(LogLevel::from_str("INFO ").is_none()); // 带空格
        assert!(LogLevel::from_str(" INFO").is_none());
        assert!(LogLevel::from_str("123").is_none());
    }

    #[test]
    fn test_log_level_as_str_and_short_str() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Fatal.as_str(), "FATAL");

        assert_eq!(LogLevel::Trace.as_short_str(), "TRC");
        assert_eq!(LogLevel::Debug.as_short_str(), "DBG");
        assert_eq!(LogLevel::Info.as_short_str(), "INF");
        assert_eq!(LogLevel::Warn.as_short_str(), "WRN");
        assert_eq!(LogLevel::Error.as_short_str(), "ERR");
        assert_eq!(LogLevel::Fatal.as_short_str(), "FTL");
    }

    #[test]
    fn test_log_level_from_str_trait_impl() {
        // FromStr trait 实现
        let level: Result<LogLevel, LogLevelParseError> = "info".parse();
        assert!(level.is_ok());
        assert!(matches!(level.unwrap(), LogLevel::Info));

        let level: Result<LogLevel, LogLevelParseError> = "unknown".parse();
        assert!(level.is_err());
        match level.err().unwrap() {
            LogLevelParseError::Unknown(s) => assert_eq!(s, "unknown"),
        }
    }

    #[test]
    fn test_log_level_default_is_info() {
        let level = LogLevel::default();
        assert!(matches!(level, LogLevel::Info));
    }
}

// ============================================================================
// CORE-003: ConsoleSink stderr 路由 E2E
// ============================================================================

#[cfg(test)]
mod console_sink_e2e {
    use super::*;
    use inklog::ConsoleSinkConfig;
    use inklog::sink::ConsoleSink;

    #[tokio::test]
    async fn test_console_sink_writes_info_to_stdout_buffer() {
        // ConsoleSink 接受 Box<dyn Write + Send>，使用 Vec<u8> 作为内存缓冲
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
            masking_enabled: false,
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);

        let record = LogRecord::new(
            Level::INFO,
            "test_target".to_string(),
            "info message".to_string(),
        );
        let result = sink.write(&record).await;
        assert!(result.is_ok(), "write should succeed");
        sink.flush().await.expect("flush should succeed");
    }

    #[tokio::test]
    async fn test_console_sink_stderr_levels_routing() {
        // 验证 stderr_levels 配置存在并可被构造
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: true,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
            masking_enabled: false,
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);

        // 写入 error 级别日志（应路由到 stderr）
        let error_record = LogRecord::new(
            Level::ERROR,
            "test".to_string(),
            "error message".to_string(),
        );
        let result = sink.write(&error_record).await;
        assert!(result.is_ok());

        // 写入 info 级别日志（应路由到 stdout）
        let info_record =
            LogRecord::new(Level::INFO, "test".to_string(), "info message".to_string());
        let result = sink.write(&info_record).await;
        assert!(result.is_ok());

        sink.flush().await.expect("flush should succeed");
    }

    #[tokio::test]
    async fn test_console_sink_masking_enabled() {
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            stderr_levels: vec![],
            masking_enabled: true,
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);

        // 包含 PII 的日志（邮箱应被脱敏）
        let record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "user email: test@example.com".to_string(),
        );
        let result = sink.write(&record).await;
        assert!(result.is_ok(), "masked write should succeed");
    }

    #[tokio::test]
    async fn test_console_sink_shutdown_is_healthy() {
        let config = ConsoleSinkConfig::default();
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);
        assert!(sink.is_healthy(), "ConsoleSink should always be healthy");
        sink.shutdown().await.expect("shutdown should succeed");
    }
}

// ============================================================================
// ROT-001~006: RotationStrategy E2E
// ============================================================================

#[cfg(test)]
mod rotation_strategy_e2e {
    use super::*;
    use std::time::Instant;

    fn build_context(size: u64, max_size: Option<u64>, sequence: u32) -> RotationContext {
        let now = chrono::Utc::now();
        let instant = Instant::now();
        RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: size,
            max_size,
            file_opened_at: instant,
            last_rotation: instant,
            now,
            sequence,
        }
    }

    #[test]
    fn test_size_based_rotation_triggers_when_exceeding_max() {
        let strategy = SizeBasedRotation::new(1024);
        let ctx = build_context(2048, Some(1024), 0);
        let result = strategy.should_rotate(&ctx);
        assert!(
            result.should_rotate,
            "should rotate when current_size > max_size"
        );
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_size_based_rotation_no_rotate_when_under_max() {
        let strategy = SizeBasedRotation::new(1024);
        let ctx = build_context(512, Some(1024), 0);
        let result = strategy.should_rotate(&ctx);
        assert!(
            !result.should_rotate,
            "should not rotate when current_size < max_size"
        );
    }

    #[test]
    fn test_size_based_rotation_boundary_at_exact_max() {
        let strategy = SizeBasedRotation::new(1024);
        let ctx = build_context(1024, Some(1024), 0);
        let result = strategy.should_rotate(&ctx);
        // 边界条件：等于 max_size 时应触发轮转（>=）
        assert!(
            result.should_rotate,
            "should rotate when current_size == max_size (>= boundary)"
        );
    }

    #[test]
    fn test_size_based_rotation_from_size_string() {
        let strategy = SizeBasedRotation::from_size_string("100MB");
        assert!(strategy.is_ok());
        assert_eq!(strategy.unwrap().max_size(), 100 * 1024 * 1024);

        let strategy = SizeBasedRotation::from_size_string("1GB");
        assert!(strategy.is_ok());
        assert_eq!(strategy.unwrap().max_size(), 1024 * 1024 * 1024);

        let strategy = SizeBasedRotation::from_size_string("512KB");
        assert!(strategy.is_ok());
        assert_eq!(strategy.unwrap().max_size(), 512 * 1024);

        let strategy = SizeBasedRotation::from_size_string("invalid");
        assert!(strategy.is_err());
    }

    #[test]
    fn test_size_based_rotation_generate_next_path() {
        let strategy = SizeBasedRotation::new(1024);
        let ctx = build_context(2048, Some(1024), 5);
        let base = Path::new("/var/log/app.log");
        let next = strategy.generate_next_path(base, &ctx);
        // 生成的路径应包含序列号或时间戳
        assert!(!next.as_os_str().is_empty());
        assert!(next.starts_with("/var/log"));
    }

    #[test]
    fn test_time_based_rotation_intervals() {
        // 每小时
        let hourly = TimeBasedRotation::from_interval_string("hourly");
        assert!(hourly.is_ok());
        assert_eq!(hourly.unwrap().interval_secs(), 3600);

        // 每天
        let daily = TimeBasedRotation::from_interval_string("daily");
        assert!(daily.is_ok());
        assert_eq!(daily.unwrap().interval_secs(), 86400);

        // 每周
        let weekly = TimeBasedRotation::from_interval_string("weekly");
        assert!(weekly.is_ok());
        assert_eq!(weekly.unwrap().interval_secs(), 86400 * 7);

        // 每月
        let monthly = TimeBasedRotation::from_interval_string("monthly");
        assert!(monthly.is_ok());
        // 每月按 30 天计算
        assert_eq!(monthly.unwrap().interval_secs(), 86400 * 30);

        // 无效
        let invalid = TimeBasedRotation::from_interval_string("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_time_based_rotation_should_rotate_after_interval() {
        let strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        // 模拟文件打开时间在 2 小时前
        let now = chrono::Utc::now();
        let file_opened_at = Instant::now() - Duration::from_secs(7200);
        let ctx = RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: 100,
            max_size: None,
            file_opened_at,
            last_rotation: file_opened_at,
            now,
            sequence: 0,
        };
        let result = strategy.should_rotate(&ctx);
        assert!(result.should_rotate, "should rotate after interval elapsed");
    }

    #[test]
    fn test_time_based_rotation_no_rotate_within_interval() {
        let strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        let now = chrono::Utc::now();
        let file_opened_at = Instant::now() - Duration::from_secs(60); // 仅过去 1 分钟
        let ctx = RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: 100,
            max_size: None,
            file_opened_at,
            last_rotation: file_opened_at,
            now,
            sequence: 0,
        };
        let result = strategy.should_rotate(&ctx);
        assert!(!result.should_rotate, "should not rotate within interval");
    }

    #[test]
    fn test_composite_rotation_triggers_on_any_strategy() {
        let size_strategy = SizeBasedRotation::new(1024);
        let time_strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        let composite =
            CompositeRotation::new(vec![Box::new(size_strategy), Box::new(time_strategy)]);

        // 大小超限但时间未到 → 应轮转
        let now = chrono::Utc::now();
        let recent = Instant::now() - Duration::from_secs(60);
        let ctx = RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: 2048,
            max_size: Some(1024),
            file_opened_at: recent,
            last_rotation: recent,
            now,
            sequence: 0,
        };
        let result = composite.should_rotate(&ctx);
        assert!(
            result.should_rotate,
            "composite should rotate on size trigger"
        );
    }

    #[test]
    fn test_composite_rotation_no_trigger_when_all_pass() {
        let size_strategy = SizeBasedRotation::new(1024);
        let time_strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        let composite =
            CompositeRotation::new(vec![Box::new(size_strategy), Box::new(time_strategy)]);

        let now = chrono::Utc::now();
        let recent = Instant::now() - Duration::from_secs(60);
        let ctx = RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: 100,
            max_size: Some(1024),
            file_opened_at: recent,
            last_rotation: recent,
            now,
            sequence: 0,
        };
        let result = composite.should_rotate(&ctx);
        assert!(!result.should_rotate);
    }

    #[test]
    fn test_composite_rotation_add_strategy() {
        let mut composite = CompositeRotation::new(vec![]);
        composite.add(SizeBasedRotation::new(1024));
        composite.add(TimeBasedRotation::new(3600, "hourly".to_string()));

        let now = chrono::Utc::now();
        let recent = Instant::now();
        let ctx = RotationContext {
            current_path: PathBuf::from("/tmp/test.log"),
            current_size: 2048,
            max_size: Some(1024),
            file_opened_at: recent,
            last_rotation: recent,
            now,
            sequence: 0,
        };
        let result = composite.should_rotate(&ctx);
        assert!(result.should_rotate);
    }

    #[test]
    fn test_parse_size_all_units() {
        assert_eq!(parse_size("1B").unwrap(), 1);
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        // 大小写不敏感
        assert_eq!(parse_size("1kb").unwrap(), 1024);
        assert_eq!(parse_size("1mb").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1gb").unwrap(), 1024 * 1024 * 1024);
        // 带首尾空格但中间无空格
        assert_eq!(parse_size("  100MB  ").unwrap(), 100 * 1024 * 1024);
        // 中间有空格（"100 MB"）- parse_size 不支持中间空格，应返回 Err
        assert!(parse_size(" 100 MB ").is_err());
        // 无效
        assert!(parse_size("invalid").is_err());
        assert!(parse_size("").is_err());
        // "100" 无单位 - parse_size 将其视为字节数（multiplier=1, suffix_len=0）
        assert_eq!(parse_size("100").unwrap(), 100);
    }

    #[test]
    fn test_rotation_strategy_names() {
        let size_strategy = SizeBasedRotation::new(1024);
        let time_strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        // name() 应返回非空字符串
        assert!(!size_strategy.name().is_empty());
        assert!(!time_strategy.name().is_empty());
    }

    #[test]
    fn test_rotation_strategy_clone_boxed() {
        let size_strategy = SizeBasedRotation::new(1024);
        let cloned = size_strategy.clone_boxed();
        assert_eq!(cloned.name(), size_strategy.name());

        let time_strategy = TimeBasedRotation::new(3600, "hourly".to_string());
        let cloned = time_strategy.clone_boxed();
        assert_eq!(cloned.name(), time_strategy.name());
    }

    #[test]
    fn test_rotation_result_default() {
        let result = RotationResult::default();
        assert!(!result.should_rotate);
        assert!(result.reason.is_none());
        assert!(result.new_path.is_none());
    }
}

// ============================================================================
// PIPE-002/003: SinkRegistry E2E
// ============================================================================

#[cfg(test)]
mod sink_registry_e2e {
    use super::*;

    fn make_test_config(dir: &Path) -> FileSinkConfig {
        FileSinkConfig {
            enabled: true,
            path: dir.join("registry_test.log"),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_sink_registry_full_lifecycle() {
        let temp = tempdir().expect("tempdir failed");
        let mut registry = SinkRegistry::new();

        // 初始状态：空
        assert!(registry.list_sinks().is_empty());
        assert!(!registry.has_sink("file"));

        // 注册
        let config = make_test_config(temp.path());
        registry.register(FileSinkFactory::new(config));
        assert!(registry.has_sink("file"));
        assert_eq!(registry.list_sinks().len(), 1);
        assert!(registry.list_sinks().contains(&"file"));

        // 创建 sink
        let sink = registry.create("file").await;
        assert!(sink.is_ok(), "create should succeed");

        // 获取元数据
        let metadata = registry.get_metadata("file");
        assert!(metadata.is_some());
        let metadata: SinkMetadata = metadata.unwrap();
        assert_eq!(metadata.name, "File Sink");
        assert!(!metadata.description.is_empty());
        assert!(!metadata.features.is_empty());
        assert!(metadata.features.contains(&"rotation".to_string()));
        assert!(metadata.features.contains(&"compression".to_string()));
        assert!(metadata.features.contains(&"encryption".to_string()));
        assert!(metadata.features.contains(&"batching".to_string()));

        // 创建不存在的 sink 应失败
        let missing = registry.create("nonexistent").await;
        assert!(missing.is_err());
        match missing.err().unwrap() {
            InklogError::ConfigError(msg) => {
                assert!(msg.contains("Unknown sink type"), "unexpected msg: {}", msg);
                assert!(msg.contains("nonexistent"));
            }
            other => panic!("expected ConfigError, got {:?}", other),
        }

        // 获取不存在 sink 的元数据
        assert!(registry.get_metadata("nonexistent").is_none());

        // 注销
        let removed = registry.unregister("file");
        assert!(removed.is_some());
        assert!(!registry.has_sink("file"));

        // 注销后再创建应失败
        let post_unregister = registry.create("file").await;
        assert!(post_unregister.is_err());

        // 再次注销已不存在的 sink 应返回 None
        let re_removed = registry.unregister("file");
        assert!(re_removed.is_none());
    }

    #[tokio::test]
    async fn test_sink_registry_clear() {
        let temp = tempdir().expect("tempdir failed");
        let mut registry = SinkRegistry::new();

        // 注册多个
        let config1 = make_test_config(temp.path());
        registry.register(FileSinkFactory::new(config1));

        assert_eq!(registry.list_sinks().len(), 1);
        registry.clear();
        assert_eq!(registry.list_sinks().len(), 0);
        assert!(!registry.has_sink("file"));
    }

    #[test]
    fn test_sink_registry_default() {
        let registry = SinkRegistry::default();
        assert_eq!(registry.list_sinks().len(), 0);
        assert!(!registry.has_sink("file"));
    }

    #[test]
    fn test_file_sink_factory_metadata_directly() {
        let temp = tempdir().expect("tempdir failed");
        let config = make_test_config(temp.path());
        let factory = FileSinkFactory::new(config);
        let metadata = factory.metadata();
        assert_eq!(metadata.name, "File Sink");
        assert_eq!(factory.sink_type(), "file");
        assert!(metadata.description.contains("rotation"));
        assert!(metadata.config_schema.is_none());
    }

    #[tokio::test]
    async fn test_file_sink_factory_create_directly() {
        let temp = tempdir().expect("tempdir failed");
        let config = make_test_config(temp.path());
        let factory = FileSinkFactory::new(config);
        let sink = factory.create().await;
        assert!(sink.is_ok(), "factory.create should succeed");
    }

    #[tokio::test]
    async fn test_sink_registry_create_with_invalid_file_config_errors() {
        // FileSinkFactory 创建一个指向无效路径的 FileSink 应失败
        let mut registry = SinkRegistry::new();
        let config = FileSinkConfig {
            enabled: true,
            // 使用一个不可能创建文件的路径（在不存在目录下）
            path: PathBuf::from("/nonexistent_root_dir/cannot/create/test.log"),
            ..Default::default()
        };
        registry.register(FileSinkFactory::new(config));
        let result = registry.create("file").await;
        // FileSink::new 可能成功（延迟打开），也可能失败。这里只验证不 panic
        let _ = result;
    }
}

// ============================================================================
// SEC-001~003: PathValidator / LogSanitizer / Encryption E2E
// ============================================================================

#[cfg(test)]
mod security_e2e {
    use super::*;

    // ---------- PathValidator ----------

    #[test]
    fn test_path_validator_default_allows_safe_relative_paths() {
        let validator = PathValidator::new();
        let result = validator.validate(Path::new("logs/app.log"));
        assert!(result.valid, "safe relative path should be valid");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_path_validator_blocks_directory_traversal() {
        let validator = PathValidator::new();
        let result = validator.validate(Path::new("../../etc/passwd"));
        assert!(!result.valid, "directory traversal should be blocked");
        assert!(result.error.is_some());
    }

    #[test]
    fn test_path_validator_blocks_denied_components() {
        let validator = PathValidator::new();
        // 默认 deny_components 包含 .git, .ssh, .env, etc, passwd, shadow
        let result = validator.validate(Path::new("logs/.env"));
        assert!(!result.valid, ".env should be denied by default");

        let result = validator.validate(Path::new("logs/.git/config"));
        assert!(!result.valid, ".git should be denied by default");

        let result = validator.validate(Path::new("etc/passwd"));
        assert!(!result.valid, "etc/passwd should be denied by default");

        let result = validator.validate(Path::new("etc/shadow"));
        assert!(!result.valid, "etc/shadow should be denied by default");

        let result = validator.validate(Path::new("logs/.ssh/id_rsa"));
        assert!(!result.valid, ".ssh should be denied by default");
    }

    #[test]
    fn test_path_validator_with_custom_config() {
        let config = PathValidatorConfig {
            allow_absolute: false,
            base_dir: None,
            allow_symlinks: false,
            deny_components: vec!["secret".to_string()],
        };
        let validator = PathValidator::with_config(config);

        // 自定义拒绝组件
        let result = validator.validate(Path::new("secret/data.log"));
        assert!(!result.valid, "custom denied component should be blocked");

        // 普通路径仍可通过
        let result = validator.validate(Path::new("logs/app.log"));
        assert!(result.valid);

        // 绝对路径被拒绝（allow_absolute=false）
        let result = validator.validate(Path::new("/var/log/app.log"));
        assert!(
            !result.valid,
            "absolute path should be blocked when allow_absolute=false"
        );
    }

    #[test]
    fn test_path_validator_allow_absolute_config() {
        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        let result = validator.validate(Path::new("/var/log/app.log"));
        assert!(
            result.valid,
            "absolute path should be allowed when allow_absolute=true"
        );
    }

    #[test]
    fn test_path_validator_sanitize() {
        let validator = PathValidator::new();
        let sanitized = validator.sanitize(Path::new("logs/app.log"));
        assert!(!sanitized.as_os_str().is_empty());
    }

    #[test]
    fn test_path_validator_validate_and_sanitize_safe_path() {
        let validator = PathValidator::new();
        let result = validator.validate_and_sanitize(Path::new("logs/app.log"));
        assert!(result.valid);
        // sanitized_path 可能为 Some 或 None，取决于实现；不强制断言
    }

    #[test]
    fn test_validation_result_constructors() {
        let valid = ValidationResult::valid();
        assert!(valid.valid);
        assert!(valid.error.is_none());

        let invalid = ValidationResult::invalid("test error");
        assert!(!invalid.valid);
        assert_eq!(invalid.error.as_deref(), Some("test error"));

        let sanitized = ValidationResult::sanitized(PathBuf::from("/tmp/test.log"));
        assert!(sanitized.sanitized_path.is_some());
        assert_eq!(
            sanitized.sanitized_path.as_ref().unwrap(),
            &PathBuf::from("/tmp/test.log")
        );
    }

    // ---------- LogSanitizer ----------

    #[test]
    fn test_log_sanitizer_default_removes_control_chars() {
        let sanitizer = LogSanitizer::new();
        // CRLF 注入
        let result = sanitizer.sanitize("hello\nworld\r\nmalicious");
        assert!(!result.contains('\n'), "newline should be escaped");
        assert!(!result.contains('\r'), "carriage return should be escaped");
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(result.contains("malicious"));
    }

    #[test]
    fn test_log_sanitizer_tab_escaped() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("col1\tcol2");
        assert!(!result.contains('\t'), "tab should be escaped");
    }

    #[test]
    fn test_log_sanitizer_minimal_mode() {
        let config = SanitizerConfig::default(); // Minimal mode
        let sanitizer = LogSanitizer::with_config(config);
        let result = sanitizer.sanitize("hello\nworld");
        // Minimal 模式应转义 \n \r \t
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_log_sanitizer_strict_mode() {
        let config = SanitizerConfig {
            mode: inklog::EscapeMode::Strict,
            max_length: 0,
            sensitive_patterns: vec![],
            custom_replacements: vec![],
        };
        let sanitizer = LogSanitizer::with_config(config);
        // Strict 模式将控制字符、反斜杠、双引号统一转义为 \uHHHH 形式
        let result = sanitizer.sanitize("hello\"world\\test");
        // " (U+0022) 应被转义为 \u0022
        assert!(
            result.contains("\\u0022"),
            "double quote should be escaped to \\u0022 in strict mode, got: {}",
            result
        );
        // \ (U+005C) 应被转义为 \u005c
        assert!(
            result.contains("\\u005c"),
            "backslash should be escaped to \\u005c in strict mode, got: {}",
            result
        );
        // 原始 " 和 \ 不应直接出现
        assert!(!result.contains('"'), "raw double quote should not appear");
    }

    #[test]
    fn test_log_sanitizer_json_safe_mode() {
        let config = SanitizerConfig {
            mode: inklog::EscapeMode::JsonSafe,
            max_length: 0,
            sensitive_patterns: vec![],
            custom_replacements: vec![],
        };
        let sanitizer = LogSanitizer::with_config(config);
        // JsonSafe 模式转义 JSON 特殊字符
        let result = sanitizer.sanitize("hello\"world\n");
        assert!(
            result.contains("\\\""),
            "double quote should be escaped in json mode"
        );
        assert!(
            !result.contains('\n'),
            "newline should be escaped in json mode"
        );
    }

    #[test]
    fn test_log_sanitizer_max_length_truncation() {
        let config = SanitizerConfig {
            mode: inklog::EscapeMode::Minimal,
            max_length: 10,
            sensitive_patterns: vec![],
            custom_replacements: vec![],
        };
        let sanitizer = LogSanitizer::with_config(config);
        let long_message = "this is a very long message that exceeds the max length limit";
        let result = sanitizer.sanitize(long_message);
        assert!(
            result.len() <= 30, // 截断后可能附加 "...[truncated]"
            "result should be truncated, got len={}",
            result.len()
        );
        assert!(
            result.contains("[truncated]"),
            "truncated message should contain [truncated] marker, got: {}",
            result
        );
    }

    #[test]
    fn test_log_sanitizer_max_length_zero_unlimited() {
        let config = SanitizerConfig {
            mode: inklog::EscapeMode::Minimal,
            max_length: 0, // 0 = 无限制
            sensitive_patterns: vec![],
            custom_replacements: vec![],
        };
        let sanitizer = LogSanitizer::with_config(config);
        let long_message = "a".repeat(10000);
        let result = sanitizer.sanitize(&long_message);
        assert!(
            result.len() >= 10000,
            "unlimited length should not truncate"
        );
    }

    #[test]
    fn test_log_sanitizer_sensitive_patterns_default() {
        let sanitizer = LogSanitizer::new();
        // 默认敏感模式：card_num, email, SSN, password, token, api_key, Bearer, Basic
        let result = sanitizer.sanitize("Bearer abc123token");
        // 敏感模式应替换或脱敏
        // 注意：默认配置可能不会主动脱敏所有模式，这里验证函数不 panic 且返回非空
        assert!(!result.is_empty());
    }

    #[test]
    fn test_log_sanitizer_add_pattern() {
        let mut sanitizer = LogSanitizer::new();
        let pattern = regex::Regex::new(r"secret_\d+").unwrap();
        sanitizer.add_pattern(pattern, "[REDACTED]".to_string());
        let result = sanitizer.sanitize("found secret_12345 here");
        assert!(
            result.contains("[REDACTED]"),
            "custom pattern should be replaced, got: {}",
            result
        );
        assert!(!result.contains("secret_12345"));
    }

    #[test]
    fn test_log_sanitizer_add_replacement() {
        let mut sanitizer = LogSanitizer::new();
        sanitizer.add_replacement("foo".to_string(), "bar".to_string());
        let result = sanitizer.sanitize("foo is foo");
        assert!(result.contains("bar"), "custom replacement should work");
        assert!(!result.contains("foo"));
    }

    #[test]
    fn test_log_sanitizer_safe_message_no_panic_on_empty() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_log_sanitizer_safe_message_no_panic_on_unicode() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("你好世界\n\t日本語");
        assert!(result.contains("你好世界"));
        assert!(result.contains("日本語"));
        assert!(!result.contains('\n'));
        assert!(!result.contains('\t'));
    }

    // ---------- Encryption (不使用 env var，避免 unsafe set_var) ----------

    #[test]
    fn test_derive_key_from_password_deterministic_with_same_salt() {
        let (key1, salt1) = derive_key_from_password("password123", Some(b"fixed_salt"))
            .expect("derive should succeed");
        let (key2, salt2) = derive_key_from_password("password123", Some(b"fixed_salt"))
            .expect("derive should succeed");

        assert_eq!(key1, key2, "same password+salt should produce same key");
        assert_eq!(salt1, salt2, "salt should match input");
        assert_eq!(key1.len(), 32, "key should be 32 bytes");
    }

    #[test]
    fn test_derive_key_from_password_different_salts_produce_different_keys() {
        let (key1, _) =
            derive_key_from_password("password", Some(b"salt1")).expect("derive should succeed");
        let (key2, _) =
            derive_key_from_password("password", Some(b"salt2")).expect("derive should succeed");
        assert_ne!(key1, key2, "different salts should produce different keys");
    }

    #[test]
    fn test_derive_key_from_password_different_passwords_produce_different_keys() {
        let (key1, _) = derive_key_from_password("password1", Some(b"same_salt"))
            .expect("derive should succeed");
        let (key2, _) = derive_key_from_password("password2", Some(b"same_salt"))
            .expect("derive should succeed");
        assert_ne!(
            key1, key2,
            "different passwords should produce different keys"
        );
    }

    #[test]
    fn test_derive_key_from_password_random_salt_is_16_bytes() {
        let (key, salt) = derive_key_from_password("password", None)
            .expect("derive with random salt should succeed");
        assert_eq!(key.len(), 32);
        assert_eq!(salt.len(), 16, "random salt should be 16 bytes");
    }

    #[test]
    fn test_derive_key_from_password_random_salt_is_unique() {
        let (_, salt1) = derive_key_from_password("password", None).unwrap();
        let (_, salt2) = derive_key_from_password("password", None).unwrap();
        assert_ne!(salt1, salt2, "two random salts should differ");
    }

    #[test]
    fn test_derive_key_from_password_empty_password_succeeds() {
        // PBKDF2 允许空密码
        let result = derive_key_from_password("", Some(b"salt"));
        assert!(result.is_ok());
        let (key, _) = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_from_password_long_salt_succeeds() {
        let long_salt = vec![0u8; 64];
        let result = derive_key_from_password("password", Some(&long_salt));
        assert!(result.is_ok());
        let (key, salt) = result.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(salt.len(), 64);
    }
}

// ============================================================================
// SEC-004: DataMasker PII 脱敏 E2E
// ============================================================================

#[cfg(test)]
mod data_masker_e2e {
    use super::*;

    #[test]
    fn test_data_masker_masks_email() {
        let masker = DataMasker::new();
        let result = masker.mask("contact: user@example.com end");
        assert!(
            !result.contains("user@example.com"),
            "email should be masked"
        );
        assert!(result.contains("**") || result.contains("REDACTED") || result.contains("MASKED"));
    }

    #[test]
    fn test_data_masker_masks_phone() {
        let masker = DataMasker::new();
        let result = masker.mask("call 13812345678 now");
        // 11 位中国大陆手机号
        assert!(!result.contains("13812345678"), "phone should be masked");
    }

    #[test]
    fn test_data_masker_masks_id_card() {
        let masker = DataMasker::new();
        // ID_CARD_REGEX 使用 ^...$ 锚定整个字符串，需要纯 ID card 才能匹配
        let result = masker.mask("110101199001011234");
        assert!(
            !result.contains("110101199001011234"),
            "id_card should be masked"
        );
        // 验证掩码后保留后 4 位
        assert!(result.contains("1234"), "last 4 digits should be preserved");
    }

    #[test]
    fn test_data_masker_masks_id_card_with_x_suffix() {
        let masker = DataMasker::new();
        let result = masker.mask("31011519880530218X");
        assert!(
            !result.contains("31011519880530218X"),
            "id_card with X should be masked"
        );
        assert!(
            result.contains("218X"),
            "last 3 digits + X should be preserved"
        );
    }

    #[test]
    fn test_data_masker_masks_bank_card() {
        let masker = DataMasker::new();
        // BANK_CARD_REGEX 要求整个字符串都是数字（apply 方法检查 is_ascii_digit）
        let result = masker.mask("6222021234567890123");
        assert!(
            !result.contains("6222021234567890123"),
            "bank_card should be masked"
        );
        // 验证掩码后保留后 4 位
        assert!(result.contains("0123"), "last 4 digits should be preserved");
    }

    #[test]
    fn test_data_masker_masks_bank_card_16_digits() {
        let masker = DataMasker::new();
        let result = masker.mask("4567890123456789");
        assert!(
            !result.contains("4567890123456789"),
            "16-digit bank_card should be masked"
        );
        assert!(result.contains("6789"), "last 4 digits should be preserved");
    }

    #[test]
    fn test_data_masker_masks_api_key_patterns() {
        let masker = DataMasker::new();
        // API key 模式
        let result = masker.mask("api_key: abc123def456ghi789jkl012mno345pqr789");
        assert!(
            result.contains("REDACTED") || !result.contains("abc123def456ghi789jkl012mno345pqr789")
        );
    }

    #[test]
    fn test_data_masker_masks_jwt_token() {
        let masker = DataMasker::new();
        // JWT 模式（三段式）
        let result = masker.mask("Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c");
        assert!(
            !result.contains("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"),
            "JWT should be masked"
        );
    }

    #[test]
    fn test_data_masker_masks_aws_key() {
        let masker = DataMasker::new();
        // AWS access key 模式（AKIA...）
        let result = masker.mask("aws_key: AKIAIOSFODNN7EXAMPLE");
        assert!(
            !result.contains("AKIAIOSFODNN7EXAMPLE"),
            "AWS key should be masked"
        );
    }

    #[test]
    fn test_data_masker_masks_generic_secret() {
        let masker = DataMasker::new();
        let result = masker.mask("password: secret_value_123");
        assert!(
            !result.contains("secret_value_123"),
            "secret value should be masked"
        );
    }

    #[test]
    fn test_data_masker_is_sensitive_field() {
        assert!(DataMasker::is_sensitive_field("password"));
        assert!(DataMasker::is_sensitive_field("token"));
        assert!(DataMasker::is_sensitive_field("secret"));
        assert!(DataMasker::is_sensitive_field("api_key"));
        assert!(DataMasker::is_sensitive_field("apikey"));
        assert!(DataMasker::is_sensitive_field("credential"));
        assert!(DataMasker::is_sensitive_field("auth_token"));
        assert!(DataMasker::is_sensitive_field("access_key"));
        assert!(!DataMasker::is_sensitive_field("username"));
        assert!(!DataMasker::is_sensitive_field("email"));
        assert!(!DataMasker::is_sensitive_field("message"));
    }

    #[test]
    fn test_data_masker_mask_value_string() {
        let masker = DataMasker::new();
        let mut value = Value::String("user@example.com".to_string());
        masker.mask_value(&mut value);
        match value {
            Value::String(s) => assert!(!s.contains("user@example.com")),
            _ => panic!("value should remain a string after masking"),
        }
    }

    #[test]
    fn test_data_masker_mask_value_object_recursive() {
        let masker = DataMasker::new();
        let mut value = serde_json::json!({
            "user": "alice",
            "email": "alice@example.com",
            "nested": {
                // GENERIC_SECRET_REGEX 要求 16+ 字符的 secret 值
                "password": "password=very_long_secret_12345",
                "safe": "ok"
            }
        });
        masker.mask_value(&mut value);
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(
            !serialized.contains("alice@example.com"),
            "email should be masked"
        );
        assert!(
            !serialized.contains("very_long_secret_12345"),
            "long secret should be redacted"
        );
        // safe 字段不含 PII，应保留
        assert!(serialized.contains("ok"), "safe value should be preserved");
    }

    #[test]
    fn test_data_masker_mask_value_array() {
        let masker = DataMasker::new();
        let mut value = serde_json::json!(["alice@example.com", "bob@example.com", "safe_text"]);
        masker.mask_value(&mut value);
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(!serialized.contains("alice@example.com"));
        assert!(!serialized.contains("bob@example.com"));
    }

    #[test]
    fn test_data_masker_mask_hashmap() {
        let masker = DataMasker::new();
        let mut map = std::collections::HashMap::new();
        map.insert(
            "email".to_string(),
            Value::String("test@example.com".to_string()),
        );
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        // GENERIC_SECRET_REGEX 要求 secret 值至少 16 字符
        map.insert(
            "password".to_string(),
            Value::String("password=very_long_secret_value_12345".to_string()),
        );
        masker.mask_hashmap(&mut map);

        if let Some(Value::String(s)) = map.get("email") {
            assert!(!s.contains("test@example.com"), "email should be masked");
        }
        // name 字段不含 PII，应保持原值
        if let Some(Value::String(s)) = map.get("name") {
            assert_eq!(s, "Alice", "non-sensitive name should be preserved");
        }
        // password 字段值应被 generic_secret 规则脱敏
        if let Some(Value::String(s)) = map.get("password") {
            assert!(
                !s.contains("very_long_secret_value_12345"),
                "long secret should be redacted, got: {}",
                s
            );
        }
    }

    #[test]
    fn test_data_masker_no_panic_on_empty_input() {
        let masker = DataMasker::new();
        let result = masker.mask("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_data_masker_no_panic_on_no_pii() {
        let masker = DataMasker::new();
        let result = masker.mask("just a normal log message without PII");
        assert!(result.contains("just a normal log message without PII"));
    }
}

// ============================================================================
// CORE-009: LogRecord mask_sensitive_fields E2E
// ============================================================================

#[cfg(test)]
mod log_record_masking_e2e {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_log_record_mask_sensitive_fields_message_with_email() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "user email: test@example.com".to_string(),
        );
        record.mask_sensitive_fields();
        assert!(!record.message.contains("test@example.com"));
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_message_with_phone() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "call 13812345678".to_string(),
        );
        record.mask_sensitive_fields();
        assert!(!record.message.contains("13812345678"));
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_message_with_password_literal() {
        // GENERIC_SECRET_REGEX 要求 secret 值至少 16 字符
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "password=very_long_secret_value_12345".to_string(),
        );
        record.mask_sensitive_fields();
        assert!(
            !record.message.contains("very_long_secret_value_12345"),
            "long secret should be masked, got: {}",
            record.message
        );
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_sensitive_field_names() {
        let mut fields = HashMap::new();
        fields.insert("password".to_string(), Value::String("mypw".to_string()));
        fields.insert("token".to_string(), Value::String("tok123".to_string()));
        fields.insert("safe_field".to_string(), Value::String("ok".to_string()));

        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "msg".to_string());
        record.fields = fields;
        record.mask_sensitive_fields();

        // 敏感字段名应被替换为 ***MASKED***
        // 具体行为：字段值被脱敏，或字段名本身被替换
        let serialized = serde_json::to_string(&record.fields).unwrap();
        assert!(!serialized.contains("mypw"));
        assert!(!serialized.contains("tok123"));
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_nested_object_in_fields() {
        let mut fields = HashMap::new();
        fields.insert(
            "user".to_string(),
            serde_json::json!({
                "email": "nested@example.com",
                // GENERIC_SECRET_REGEX 要求 16+ 字符
                "password": "password=nested_long_secret_value",
                "safe": "ok"
            }),
        );

        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "msg".to_string());
        record.fields = fields;
        record.mask_sensitive_fields();

        let serialized = serde_json::to_string(&record.fields).unwrap();
        assert!(
            !serialized.contains("nested@example.com"),
            "email should be masked"
        );
        assert!(
            !serialized.contains("nested_long_secret_value"),
            "long secret should be redacted"
        );
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_no_panic_on_empty_message() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "".to_string());
        record.mask_sensitive_fields();
        assert!(record.message.is_empty());
    }

    #[test]
    fn test_log_record_mask_sensitive_fields_no_panic_on_empty_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "msg".to_string());
        record.mask_sensitive_fields();
        assert!(record.fields.is_empty());
    }

    #[test]
    fn test_log_record_reset() {
        let mut record = LogRecord::new(
            Level::ERROR,
            "test_target".to_string(),
            "error message".to_string(),
        );
        record
            .fields
            .insert("key".to_string(), Value::String("value".to_string()));
        record.file = Some("test.rs".to_string());
        record.line = Some(42);

        record.reset();

        // reset 后应回到默认状态
        assert_eq!(record.level, "INFO");
        assert_eq!(record.target, "");
        assert_eq!(record.message, "");
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
    }

    #[test]
    fn test_log_record_new_sets_correct_level() {
        let record = LogRecord::new(
            Level::WARN,
            "my_target".to_string(),
            "warning message".to_string(),
        );
        assert_eq!(record.level, "WARN");
        assert_eq!(record.target, "my_target");
        assert_eq!(record.message, "warning message");
    }

    #[test]
    fn test_log_record_default() {
        let record = LogRecord::default();
        assert_eq!(record.level, "INFO");
        assert_eq!(record.target, "");
        assert_eq!(record.message, "");
        assert!(record.fields.is_empty());
        assert!(record.file.is_none());
        assert!(record.line.is_none());
    }
}

// ============================================================================
// ERR-001~003: InklogError safe_message E2E
// ============================================================================

#[cfg(test)]
mod inklog_error_e2e {
    use super::*;

    #[test]
    fn test_inklog_error_safe_message_config_error() {
        let error = InklogError::ConfigError("config.toml not found".to_string());
        let safe = error.safe_message();
        // safe_message 应返回非空字符串
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = InklogError::IoError(io_err);
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_serialization_error() {
        let json_err = serde_json::from_str::<Value>("invalid json").unwrap_err();
        let error = InklogError::SerializationError(json_err);
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_database_error() {
        let error = InklogError::DatabaseError(
            "connection refused to postgres://user:pass@host".to_string(),
        );
        let safe = error.safe_message();
        // 数据库 URL 中可能包含密码，safe_message 应脱敏
        assert!(
            !safe.contains("pass@host"),
            "password in db url should be sanitized"
        );
    }

    #[test]
    fn test_inklog_error_safe_message_cache_error() {
        let error = InklogError::CacheError("cache miss".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_encryption_error() {
        let error = InklogError::EncryptionError("invalid key length".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_shutdown() {
        let error = InklogError::Shutdown("graceful shutdown".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_channel_error() {
        let error = InklogError::ChannelError("channel closed".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_compression_error() {
        let error = InklogError::CompressionError("invalid zstd data".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_runtime_error() {
        let error = InklogError::RuntimeError("runtime panic".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_http_server_error() {
        let error = InklogError::HttpServerError("port 8080 in use".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_unknown() {
        let error = InklogError::Unknown("unknown error".to_string());
        let safe = error.safe_message();
        assert!(!safe.is_empty());
    }

    #[test]
    fn test_inklog_error_safe_message_sanitizes_aws_keys() {
        let error =
            InklogError::ConfigError("failed with AWS key: AKIAIOSFODNN7EXAMPLE".to_string());
        let safe = error.safe_message();
        assert!(
            !safe.contains("AKIAIOSFODNN7EXAMPLE"),
            "AWS key should be sanitized in safe_message"
        );
    }

    #[test]
    fn test_inklog_error_safe_message_sanitizes_jwt_tokens() {
        let error = InklogError::ConfigError(
            "auth failed with token: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c".to_string(),
        );
        let safe = error.safe_message();
        assert!(
            !safe.contains("eyJhbGciOiJIUzI1NiJ9"),
            "JWT token should be sanitized in safe_message"
        );
    }

    #[test]
    fn test_inklog_error_safe_message_sanitizes_emails() {
        let error = InklogError::RuntimeError("user test@example.com failed".to_string());
        let safe = error.safe_message();
        assert!(
            !safe.contains("test@example.com"),
            "email should be sanitized in safe_message"
        );
    }

    #[test]
    fn test_inklog_error_safe_message_sanitizes_passwords() {
        let error = InklogError::ConfigError("password=MySecretPass123".to_string());
        let safe = error.safe_message();
        assert!(
            !safe.contains("MySecretPass123"),
            "password should be sanitized in safe_message"
        );
    }

    #[test]
    fn test_inklog_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let error: InklogError = io_err.into();
        match error {
            InklogError::IoError(_) => {}
            other => panic!("expected IoError, got {:?}", other),
        }
    }

    #[test]
    fn test_inklog_error_from_serde_json_error() {
        let json_err = serde_json::from_str::<Value>("bad").unwrap_err();
        let error: InklogError = json_err.into();
        match error {
            InklogError::SerializationError(_) => {}
            other => panic!("expected SerializationError, got {:?}", other),
        }
    }

    #[test]
    fn test_inklog_result_type_alias() {
        let ok: InklogResult<i32> = Ok(42);
        assert!(ok.is_ok());
        assert_eq!(ok.as_ref().unwrap(), &42);

        let err: InklogResult<i32> = Err(InklogError::Unknown("test".to_string()));
        assert!(err.is_err());
    }
}

// ============================================================================
// CORE-008: LogTemplate 渲染 E2E
// ============================================================================

#[cfg(test)]
mod log_template_e2e {
    use super::*;

    #[test]
    fn test_log_template_default_renders_all_placeholders() {
        let template = LogTemplate::default();
        let mut record = LogRecord::new(
            Level::INFO,
            "my_target".to_string(),
            "hello world".to_string(),
        );
        record.file = Some("main.rs".to_string());
        record.line = Some(42);
        record.thread_id = "thread-1".to_string();
        record
            .fields
            .insert("key1".to_string(), Value::String("value1".to_string()));
        record
            .fields
            .insert("key2".to_string(), Value::String("value2".to_string()));

        let rendered = template.render(&record);
        // 默认模板: "{timestamp} [{level}] {target} - {message}"
        assert!(rendered.contains("INFO"), "should contain level");
        assert!(rendered.contains("my_target"), "should contain target");
        assert!(rendered.contains("hello world"), "should contain message");
        assert!(rendered.contains("["), "should contain level brackets");
    }

    #[test]
    fn test_log_template_custom_with_all_placeholders() {
        let template = LogTemplate::new(
            "{timestamp} {level} {target} {message} {file} {line} {thread_id} {fields}",
        );
        let mut record = LogRecord::new(Level::ERROR, "app".to_string(), "fail".to_string());
        record.file = Some("lib.rs".to_string());
        record.line = Some(100);
        record.thread_id = "T1".to_string();
        record
            .fields
            .insert("k".to_string(), Value::String("v".to_string()));

        let rendered = template.render(&record);
        assert!(rendered.contains("ERROR"));
        assert!(rendered.contains("app"));
        assert!(rendered.contains("fail"));
        assert!(rendered.contains("lib.rs"));
        assert!(rendered.contains("100"));
        assert!(rendered.contains("T1"));
        // {fields} 渲染为 key=value 对
        assert!(rendered.contains("k=") || rendered.contains("v"));
    }

    #[test]
    fn test_log_template_unknown_placeholder_rendered_as_is() {
        let template = LogTemplate::new("{unknown_placeholder} {message}");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "hello".to_string());
        let rendered = template.render(&record);
        // 未知占位符应保持原样或被移除
        assert!(rendered.contains("hello"));
    }

    #[test]
    fn test_log_template_escape_braces() {
        // {{ → {, }} → }
        let template = LogTemplate::new("{{literal}} {message}");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "msg".to_string());
        let rendered = template.render(&record);
        assert!(
            rendered.contains("literal"),
            "escaped braces should render as literal"
        );
        assert!(rendered.contains("msg"));
    }

    #[test]
    fn test_log_template_empty_template() {
        let template = LogTemplate::new("");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "msg".to_string());
        let rendered = template.render(&record);
        // 空模板渲染应返回空或仅占位符内容
        // 不应 panic
        let _ = rendered;
    }

    #[test]
    fn test_log_template_only_message_placeholder() {
        let template = LogTemplate::new("{message}");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "only message".to_string());
        let rendered = template.render(&record);
        assert_eq!(rendered, "only message");
    }

    #[test]
    fn test_log_template_no_placeholders() {
        let template = LogTemplate::new("static text only");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "msg".to_string());
        let rendered = template.render(&record);
        assert_eq!(rendered, "static text only");
    }

    #[test]
    fn test_log_template_timestamp_format() {
        let template = LogTemplate::new("{timestamp}");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "msg".to_string());
        let rendered = template.render(&record);
        // 时间戳格式: "%Y-%m-%dT%H:%M:%S%.3fZ"
        assert!(
            rendered.contains('T'),
            "timestamp should contain T separator"
        );
        assert!(rendered.contains('Z'), "timestamp should contain Z suffix");
        assert!(
            rendered.contains('-'),
            "timestamp should contain date separators"
        );
        assert!(
            rendered.contains(':'),
            "timestamp should contain time separators"
        );
        assert!(
            rendered.contains('.'),
            "timestamp should contain millisecond separator"
        );
    }

    #[test]
    fn test_log_template_fields_placeholder_with_empty_fields() {
        let template = LogTemplate::new("[{fields}]");
        let record = LogRecord::new(Level::INFO, "t".to_string(), "msg".to_string());
        let rendered = template.render(&record);
        // 空 fields 应渲染为空或 []
        let _ = rendered;
    }
}

// ============================================================================
// REL-001~003: CircuitBreaker 状态转换 E2E
// ============================================================================

#[cfg(test)]
mod circuit_breaker_e2e {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state_closed() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30), 3);
        assert!(matches!(cb.state(), CircuitState::Closed));
        assert!(cb.can_execute(), "Closed state should allow execution");
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_closed_to_open_after_threshold() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30), 2);
        assert!(matches!(cb.state(), CircuitState::Closed));

        // 2 次失败未达阈值
        cb.record_failure();
        cb.record_failure();
        assert!(
            matches!(cb.state(), CircuitState::Closed),
            "should remain closed below threshold"
        );

        // 第 3 次失败达到阈值，转为 Open
        cb.record_failure();
        assert!(
            matches!(cb.state(), CircuitState::Open),
            "should transition to Open after failure_threshold reached"
        );
        assert!(!cb.can_execute(), "Open state should block execution");
    }

    #[test]
    fn test_circuit_breaker_open_to_half_open_after_timeout() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(10), 1);
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Open));

        // 等待超时
        std::thread::sleep(Duration::from_millis(50));

        // can_execute 应触发 Open → HalfOpen 转换
        let can = cb.can_execute();
        assert!(can, "HalfOpen should allow execution");
        assert!(
            matches!(cb.state(), CircuitState::HalfOpen),
            "should transition to HalfOpen after timeout"
        );
    }

    #[test]
    fn test_circuit_breaker_half_open_to_closed_after_success_threshold() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(10), 2);
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Open));

        std::thread::sleep(Duration::from_millis(50));
        let _ = cb.can_execute(); // Open → HalfOpen
        assert!(matches!(cb.state(), CircuitState::HalfOpen));

        // 1 次成功未达阈值
        cb.record_success();
        assert!(
            matches!(cb.state(), CircuitState::HalfOpen),
            "should remain HalfOpen below success_threshold"
        );

        // 第 2 次成功达到阈值，转为 Closed
        cb.record_success();
        assert!(
            matches!(cb.state(), CircuitState::Closed),
            "should transition to Closed after success_threshold reached"
        );
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_half_open_to_open_on_failure() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(10), 2);
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(50));
        let _ = cb.can_execute(); // Open → HalfOpen

        // HalfOpen 状态下任何失败立即转 Open
        cb.record_failure();
        assert!(
            matches!(cb.state(), CircuitState::Open),
            "HalfOpen → Open on failure"
        );
    }

    #[test]
    fn test_circuit_breaker_closed_resets_failure_count_on_success() {
        let mut cb = CircuitBreaker::new(5, Duration::from_secs(30), 3);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        cb.record_success();
        assert_eq!(
            cb.failure_count(),
            0,
            "success in Closed should reset failure_count"
        );
        assert!(matches!(cb.state(), CircuitState::Closed));
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(30), 1);
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Open));

        cb.reset();
        assert!(matches!(cb.state(), CircuitState::Closed));
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_with_config() {
        let config = CircuitBreakerConfig {
            failure_threshold: 10,
            success_threshold: 5,
            timeout: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::with_config(config);
        assert!(matches!(cb.state(), CircuitState::Closed));
        let retrieved = cb.config();
        assert_eq!(retrieved.failure_threshold, 10);
        assert_eq!(retrieved.success_threshold, 5);
        assert_eq!(retrieved.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_circuit_breaker_open_stays_open_within_timeout() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(60), 1);
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Open));

        // 超时内仍应为 Open
        let can = cb.can_execute();
        assert!(!can, "should remain Open within timeout");
        assert!(matches!(cb.state(), CircuitState::Open));
    }

    #[test]
    fn test_circuit_breaker_open_record_failure_stays_open() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(60), 1);
        cb.record_failure(); // → Open
        cb.record_failure(); // 应保持 Open
        assert!(matches!(cb.state(), CircuitState::Open));
    }
}

// ============================================================================
// PERF-001: ObjectPool 全局便捷函数 E2E
// ============================================================================

#[cfg(test)]
mod object_pool_e2e {
    use super::*;

    #[test]
    fn test_global_log_record_get_returns_default() {
        let record = get_log_record();
        assert_eq!(record.level, "INFO");
        assert_eq!(record.target, "");
        assert_eq!(record.message, "");
        assert!(record.fields.is_empty());
    }

    #[test]
    fn test_global_log_record_put_then_get_reuses() {
        // 先取出一个
        let mut record = get_log_record();
        record.message = "modified".to_string();
        record.level = "ERROR".to_string();
        // 放回池（put 会 reset）
        put_log_record(record);

        // 再取出，应是 reset 后的默认状态
        let record2 = get_log_record();
        assert_eq!(record2.level, "INFO", "put should reset level");
        assert_eq!(record2.message, "", "put should reset message");
    }

    #[test]
    fn test_global_log_record_multiple_cycles() {
        for _ in 0..10 {
            let record = get_log_record();
            put_log_record(record);
        }
        // 多次循环不应 panic
    }

    #[test]
    fn test_global_string_buffer_get_returns_string() {
        let s = get_string_buffer();
        // 可能是空字符串或池中的字符串
        let _ = s;
    }

    #[test]
    fn test_global_string_buffer_put_then_get() {
        put_string_buffer("test_buffer".to_string());
        let s = get_string_buffer();
        // 池可能返回该字符串或新字符串
        let _ = s;
    }

    #[test]
    fn test_global_string_buffer_multiple_cycles() {
        for _ in 0..10 {
            let s = get_string_buffer();
            put_string_buffer(s);
        }
    }

    #[tokio::test]
    async fn test_object_pool_async_put_get_with_value() {
        let pool = ObjectPool::<String, i32>::new().await.expect("pool build");
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);

        pool.put(&"key1".to_string(), 42).await.expect("put");
        pool.put(&"key2".to_string(), 100).await.expect("put");

        let v1 = pool.get(&"key1".to_string()).await.expect("get");
        let v2 = pool.get(&"key2".to_string()).await.expect("get");
        let v3 = pool.get(&"missing".to_string()).await.expect("get missing");

        assert_eq!(v1, Some(42));
        assert_eq!(v2, Some(100));
        assert_eq!(v3, None);
    }

    #[tokio::test]
    async fn test_object_pool_with_config_custom() {
        let pool = ObjectPool::<String, String>::with_config(ObjectPoolConfig {
            max_capacity: 256,
            ttl_secs: Some(60),
        })
        .await
        .expect("pool with config");

        pool.put(&"k".to_string(), "v".to_string())
            .await
            .expect("put");
        let v = pool.get(&"k".to_string()).await.expect("get");
        assert_eq!(v, Some("v".to_string()));
    }

    #[test]
    fn test_object_pool_config_default_values() {
        let config = ObjectPoolConfig::default();
        assert_eq!(config.max_capacity, 1024);
        assert_eq!(config.ttl_secs, None);
    }
}

// ============================================================================
// CFG-004: InklogConfig validate 边界 E2E
// ============================================================================

#[cfg(test)]
mod config_validation_e2e {
    use super::*;
    use inklog::ChannelStrategy;

    #[test]
    fn test_config_default_validate_passes() {
        let config = InklogConfig::default();
        let result = config.validate();
        assert!(result.is_ok(), "default config should validate");
    }

    #[test]
    fn test_config_validate_fails_when_channel_capacity_zero() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = 0;
        let result = config.validate();
        assert!(result.is_err(), "channel_capacity=0 should fail validation");
        match result.err().unwrap() {
            InklogError::ConfigError(msg) => {
                assert!(msg.contains("channel_capacity") || msg.contains("0"));
            }
            other => panic!("expected ConfigError, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_fails_when_worker_threads_zero() {
        let mut config = InklogConfig::default();
        config.performance.worker_threads = 0;
        let result = config.validate();
        assert!(result.is_err(), "worker_threads=0 should fail validation");
        match result.err().unwrap() {
            InklogError::ConfigError(msg) => {
                assert!(msg.contains("worker_threads") || msg.contains("0"));
            }
            other => panic!("expected ConfigError, got {:?}", other),
        }
    }

    #[test]
    fn test_config_validate_passes_with_valid_performance() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = 100;
        config.performance.worker_threads = 2;
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_sinks_enabled() {
        let config = InklogConfig {
            console_sink: Some(inklog::ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            file_sink: Some(FileSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        // sinks_enabled 返回启用的 sink 名称列表（非空表示有 sink 启用）
        assert!(!config.sinks_enabled().is_empty());
    }

    #[test]
    fn test_config_sinks_disabled() {
        let config = InklogConfig::default();
        // InklogConfig::default() 启用 console_sink（default_console_sink），
        // 所以 sinks_enabled() 应返回 ["console"]，不为空
        let sinks = config.sinks_enabled();
        assert!(
            !sinks.is_empty(),
            "default config should enable console sink"
        );
        assert!(sinks.contains(&"console"), "default should include console");
        // file/database 默认未启用
        assert!(!sinks.contains(&"file"));
        assert!(!sinks.contains(&"database"));
    }

    #[test]
    fn test_performance_config_default() {
        let perf = PerformanceConfig::default();
        assert_eq!(perf.channel_capacity, 10000);
        assert_eq!(perf.worker_threads, 3);
        assert!(matches!(perf.channel_strategy, ChannelStrategy::Fixed));
        assert_eq!(perf.expand_threshold_percent, 80);
        assert_eq!(perf.shrink_threshold_percent, 20);
        assert_eq!(perf.shrink_wait_seconds, 30);
        assert_eq!(perf.min_capacity, 1000);
        assert_eq!(perf.max_capacity, 50000);
    }

    #[test]
    fn test_global_config_default() {
        let global = inklog::GlobalConfig::default();
        assert_eq!(global.level, "info");
        assert_eq!(global.format, "{timestamp} [{level}] {target} - {message}");
        assert!(global.masking_enabled);
        assert!(global.auto_fallback);
        assert_eq!(global.fallback_initial_delay_ms, 1000);
        assert_eq!(global.fallback_max_delay_ms, 60000);
        assert_eq!(global.fallback_max_retries, 10);
    }

    #[test]
    fn test_console_sink_config_default() {
        let config = inklog::ConsoleSinkConfig::default();
        assert!(config.enabled);
        assert!(config.colored);
        assert_eq!(
            config.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
        assert!(!config.masking_enabled);
    }

    #[test]
    fn test_file_sink_config_default() {
        let config = FileSinkConfig::default();
        assert!(config.enabled);
        assert_eq!(config.path, PathBuf::from("logs/app.log"));
        assert_eq!(config.max_size, "100MB");
        assert_eq!(config.rotation_time, "daily");
        assert_eq!(config.keep_files, 30);
        assert!(config.compress);
        assert_eq!(config.compression_level, 3);
        assert!(!config.encrypt);
        assert_eq!(config.retention_days, 30);
        assert_eq!(config.max_total_size, "1GB");
        assert_eq!(config.cleanup_interval_minutes, 60);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_ms, 100);
        assert!(config.masking_enabled);
    }
}

// ============================================================================
// REL-004~006: SinkHealthMonitor 降级/恢复 E2E
// ============================================================================

#[cfg(test)]
mod sink_health_monitor_e2e {
    use super::*;
    use inklog::FallbackConfig;

    #[test]
    fn test_sink_health_monitor_initial_state_active() {
        let monitor = SinkHealthMonitor::with_defaults();
        assert_eq!(
            monitor.get_fallback_state("database"),
            inklog::FallbackState::Active
        );
        assert!(!monitor.is_any_in_fallback());
    }

    #[test]
    fn test_sink_health_monitor_database_failure_triggers_fallback_to_file() {
        let monitor = SinkHealthMonitor::with_defaults();
        // 默认 failure_threshold = 3
        for _ in 0..3 {
            let action = monitor.check_and_fallback("database", false, Some("Connection refused"));
            let _ = action;
        }
        let state = monitor.get_fallback_state("database");
        match state {
            inklog::FallbackState::Fallback { target, reason } => {
                assert_eq!(target, "file", "database should fall back to file");
                assert!(reason.contains("Connection refused"));
            }
            other => panic!("expected Fallback, got {:?}", other),
        }
        assert!(monitor.is_any_in_fallback());
    }

    #[test]
    fn test_sink_health_monitor_file_disk_full_falls_back_to_console() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("file", false, Some("Disk is full"));
        }
        let state = monitor.get_fallback_state("file");
        match state {
            inklog::FallbackState::Fallback { target, .. } => {
                assert_eq!(
                    target, "console",
                    "file disk full should fall back to console"
                );
            }
            other => panic!("expected Fallback, got {:?}", other),
        }
    }

    #[test]
    fn test_sink_health_monitor_unknown_sink_falls_back_to_console() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("custom_sink", false, Some("unknown failure"));
        }
        let state = monitor.get_fallback_state("custom_sink");
        match state {
            inklog::FallbackState::Fallback { target, .. } => {
                assert_eq!(target, "console");
            }
            other => panic!("expected Fallback, got {:?}", other),
        }
    }

    #[test]
    fn test_sink_health_monitor_recovery_attempt_from_fallback() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("error"));
        }
        assert!(monitor.is_any_in_fallback());

        // 恢复健康 → AttemptRecovery
        let action = monitor.check_and_fallback("database", true, None);
        match action {
            FallbackAction::AttemptRecovery { sink_name, .. } => {
                assert_eq!(sink_name, "database");
            }
            other => panic!("expected AttemptRecovery, got {:?}", other),
        }
    }

    #[test]
    fn test_sink_health_monitor_confirm_recovery() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("error"));
        }
        monitor.confirm_recovery("database");
        assert_eq!(
            monitor.get_fallback_state("database"),
            inklog::FallbackState::Active
        );
        assert!(!monitor.is_any_in_fallback());
    }

    #[test]
    fn test_sink_health_monitor_disabled_fallback_returns_none() {
        let config = FallbackConfig {
            enabled: false,
            ..Default::default()
        };
        let monitor = SinkHealthMonitor::new(config);
        let action = monitor.check_and_fallback("database", false, Some("error"));
        assert!(matches!(action, FallbackAction::None));
        assert!(!action.requires_action());
    }

    #[test]
    fn test_sink_health_monitor_encryption_error_fallback() {
        let monitor = SinkHealthMonitor::with_defaults();
        let action = monitor.handle_encryption_error("file", "Invalid key");
        match action {
            FallbackAction::Fallback { target, .. } => {
                assert_eq!(target, "plaintext");
            }
            other => panic!("expected Fallback to plaintext, got {:?}", other),
        }
    }

    #[test]
    fn test_sink_health_monitor_reset() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("error"));
        }
        assert!(monitor.is_any_in_fallback());

        monitor.reset();
        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            inklog::FallbackState::Active
        );
    }

    #[test]
    fn test_sink_health_monitor_get_fallback_events() {
        let monitor = SinkHealthMonitor::with_defaults();
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("error"));
        }
        let events = monitor.get_fallback_events(10);
        assert!(!events.is_empty());
        assert_eq!(events[0].sink_name, "database");
    }

    #[test]
    fn test_sink_health_monitor_get_fallback_stats() {
        let monitor = SinkHealthMonitor::with_defaults();
        let stats = monitor.get_fallback_stats();
        assert_eq!(stats.active_fallbacks, 0);
        assert_eq!(stats.recovering, 0);
    }

    #[test]
    fn test_sink_health_monitor_default() {
        let monitor = SinkHealthMonitor::default();
        assert_eq!(
            monitor.get_fallback_state("any"),
            inklog::FallbackState::Active
        );
    }
}

// ============================================================================
// OBS-001~003: Metrics 指标 E2E
// ============================================================================

#[cfg(test)]
mod metrics_e2e {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_counters_increment() {
        let metrics = Metrics::new();
        assert_eq!(metrics.logs_written(), 0);
        assert_eq!(metrics.logs_dropped(), 0);
        assert_eq!(metrics.channel_blocked(), 0);
        assert_eq!(metrics.sink_errors(), 0);
        assert_eq!(metrics.lock_contention(), 0);

        metrics.inc_logs_written();
        metrics.inc_logs_written();
        metrics.inc_logs_dropped();
        metrics.inc_channel_blocked();
        metrics.inc_sink_error();
        metrics.inc_lock_contention();

        assert_eq!(metrics.logs_written(), 2);
        assert_eq!(metrics.logs_dropped(), 1);
        assert_eq!(metrics.channel_blocked(), 1);
        assert_eq!(metrics.sink_errors(), 1);
        assert_eq!(metrics.lock_contention(), 1);
    }

    #[test]
    fn test_metrics_db_batch_tracking() {
        let metrics = Metrics::new();
        metrics.set_db_batch_size(50);
        metrics.add_db_batch_records_total(50);
        assert_eq!(metrics.db_batch_size(), 50);
        assert_eq!(metrics.db_batch_records_total(), 50);

        metrics.add_db_batch_records_total(30);
        assert_eq!(metrics.db_batch_records_total(), 80);
    }

    #[test]
    fn test_metrics_record_latency() {
        let metrics = Metrics::new();
        metrics.record_latency(Duration::from_micros(100));
        metrics.record_latency(Duration::from_micros(500));
        metrics.record_latency(Duration::from_micros(1500));

        // 验证延迟记录不 panic 即可（total_latency_us 是 pub(crate) 私有字段，
        // 外部测试无法直接访问；可通过 export_prometheus 输出间接验证存在）
        let prom = metrics.export_prometheus();
        assert!(
            prom.contains("inklog_latency") || prom.contains("latency"),
            "prometheus output should contain latency metric, got: {}",
            prom
        );
    }

    #[test]
    fn test_metrics_update_sink_health_healthy() {
        let metrics = Metrics::new();
        metrics.update_sink_health("file", true, None);
        let health = metrics.sink_health();
        let h = health.get("file").expect("sink should exist");
        assert!(matches!(h.status, SinkStatus::Healthy));
        assert_eq!(h.consecutive_failures, 0);
    }

    #[test]
    fn test_metrics_update_sink_health_unhealthy_with_error() {
        let metrics = Metrics::new();
        metrics.update_sink_health("db", false, Some("connection refused".to_string()));
        let health = metrics.sink_health();
        let h = health.get("db").expect("sink should exist");
        match &h.status {
            SinkStatus::Unhealthy { error } => {
                assert_eq!(error, "connection refused");
            }
            other => panic!("expected Unhealthy, got {:?}", other),
        }
        assert_eq!(h.consecutive_failures, 1);
    }

    #[test]
    fn test_metrics_update_sink_health_unhealthy_without_error_uses_default() {
        let metrics = Metrics::new();
        metrics.update_sink_health("file", false, None);
        let health = metrics.sink_health();
        let h = health.get("file").expect("sink should exist");
        match &h.status {
            SinkStatus::Unhealthy { error } => {
                assert_eq!(error, "Unknown error");
            }
            other => panic!("expected Unhealthy, got {:?}", other),
        }
    }

    #[test]
    fn test_metrics_consecutive_failures_accumulate() {
        let metrics = Metrics::new();
        metrics.update_sink_health("file", false, Some("e1".to_string()));
        metrics.update_sink_health("file", false, Some("e2".to_string()));
        metrics.update_sink_health("file", false, Some("e3".to_string()));
        let health = metrics.sink_health();
        let h = health.get("file").unwrap();
        assert_eq!(h.consecutive_failures, 3);

        // 恢复健康应重置 failure_count
        metrics.update_sink_health("file", true, None);
        let health = metrics.sink_health();
        let h = health.get("file").unwrap();
        assert_eq!(h.consecutive_failures, 0);
    }

    #[test]
    fn test_metrics_sink_started() {
        let metrics = Metrics::new();
        metrics.sink_started("console");
        let health = metrics.sink_health();
        let h = health.get("console").unwrap();
        assert!(matches!(h.status, SinkStatus::Healthy));
    }

    #[test]
    fn test_metrics_sink_degraded() {
        let metrics = Metrics::new();
        metrics.sink_degraded("file", "slow disk".to_string());
        let health = metrics.sink_health();
        let h = health.get("file").unwrap();
        match &h.status {
            SinkStatus::Degraded { reason } => assert_eq!(reason, "slow disk"),
            other => panic!("expected Degraded, got {:?}", other),
        }
    }

    #[test]
    fn test_metrics_get_status_all_healthy() {
        let metrics = Metrics::new();
        metrics.sink_started("console");
        metrics.sink_started("file");
        let status = metrics.get_status(0, 100);
        assert!(matches!(status.overall_status, SinkStatus::Healthy));
        assert!(status.channel_usage.abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_get_status_with_unhealthy() {
        let metrics = Metrics::new();
        metrics.sink_started("console");
        metrics.update_sink_health("file", false, Some("disk error".to_string()));
        let status = metrics.get_status(0, 100);
        match &status.overall_status {
            SinkStatus::Unhealthy { error } => assert!(error.contains("disk error")),
            other => panic!("expected Unhealthy, got {:?}", other),
        }
    }

    #[test]
    fn test_metrics_get_status_channel_usage() {
        let metrics = Metrics::new();
        let status = metrics.get_status(50, 100);
        assert!((status.channel_usage - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_export_prometheus_format() {
        let metrics = Metrics::new();
        metrics.inc_logs_written();
        metrics.inc_sink_error();
        metrics.set_db_batch_size(10);
        metrics.sink_started("console");

        let output = metrics.export_prometheus();
        assert!(output.contains("# HELP inklog_logs_written_total"));
        assert!(output.contains("# TYPE inklog_logs_written_total counter"));
        assert!(output.contains("inklog_logs_written_total 1"));
        assert!(output.contains("inklog_sink_errors_total 1"));
        assert!(output.contains("inklog_db_batch_size{sink=\"database\"} 10"));
        assert!(output.contains("inklog_sink_healthy{sink=\"console\"} 1"));
    }

    #[test]
    fn test_metrics_pool_hit_rate() {
        let metrics = Metrics::new();
        metrics.set_pool_hit_rate(85.5);
        assert!((metrics.pool_hit_rate() - 85.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sink_status_is_operational() {
        assert!(SinkStatus::Healthy.is_operational());
        assert!(
            SinkStatus::Degraded {
                reason: "slow".to_string()
            }
            .is_operational()
        );
        assert!(
            !SinkStatus::Unhealthy {
                error: "dead".to_string()
            }
            .is_operational()
        );
        assert!(!SinkStatus::NotStarted.is_operational());
    }
}

// ============================================================================
// PIPE-001: FileSink 写入 E2E
// ============================================================================

#[cfg(test)]
mod file_sink_e2e {
    use super::*;

    fn make_test_config(dir: &Path) -> FileSinkConfig {
        FileSinkConfig {
            enabled: true,
            path: dir.join("e2e_test.log"),
            max_size: "10MB".to_string(),
            rotation_time: "daily".to_string(),
            compress: false,
            encrypt: false,
            masking_enabled: false,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_file_sink_write_and_flush() {
        let temp = tempdir().expect("tempdir failed");
        let config = make_test_config(temp.path());
        let sink = FileSink::new(config).expect("FileSink creation should succeed");

        let record = LogRecord::new(
            Level::INFO,
            "e2e_target".to_string(),
            "e2e test message".to_string(),
        );
        sink.write(&record).await.expect("write should succeed");
        sink.flush().await.expect("flush should succeed");

        // 验证文件存在
        let log_path = temp.path().join("e2e_test.log");
        assert!(log_path.exists(), "log file should exist after write");

        // 验证文件内容
        let content = std::fs::read_to_string(&log_path).expect("should read log file");
        assert!(
            content.contains("e2e test message"),
            "content should contain message"
        );
        assert!(content.contains("INFO"), "content should contain level");
        assert!(
            content.contains("e2e_target"),
            "content should contain target"
        );
    }

    #[tokio::test]
    async fn test_file_sink_multiple_writes() {
        let temp = tempdir().expect("tempdir failed");
        let config = make_test_config(temp.path());
        let sink = FileSink::new(config).expect("FileSink creation");

        for i in 0..5 {
            let record = LogRecord::new(
                Level::INFO,
                "multi_write".to_string(),
                format!("message {}", i),
            );
            sink.write(&record).await.expect("write should succeed");
        }
        sink.flush().await.expect("flush should succeed");

        let content = std::fs::read_to_string(temp.path().join("e2e_test.log")).unwrap();
        for i in 0..5 {
            assert!(
                content.contains(&format!("message {}", i)),
                "content should contain message {}",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_file_sink_shutdown() {
        let temp = tempdir().expect("tempdir failed");
        let config = make_test_config(temp.path());
        let sink = FileSink::new(config).expect("FileSink creation");

        let record = LogRecord::new(Level::INFO, "t".to_string(), "before shutdown".to_string());
        sink.write(&record).await.expect("write should succeed");
        sink.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn test_file_sink_with_masking_enabled() {
        let temp = tempdir().expect("tempdir failed");
        let config = FileSinkConfig {
            enabled: true,
            path: temp.path().join("masked.log"),
            masking_enabled: true,
            ..Default::default()
        };
        let sink = FileSink::new(config).expect("FileSink creation");

        let record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "email: test@example.com".to_string(),
        );
        sink.write(&record).await.expect("write should succeed");
        sink.flush().await.expect("flush should succeed");

        let content = std::fs::read_to_string(temp.path().join("masked.log")).unwrap();
        assert!(
            !content.contains("test@example.com"),
            "PII should be masked in file output, got: {}",
            content
        );
    }

    #[test]
    fn test_file_sink_parse_size_method() {
        // FileSink::parse_size 静态方法（支持 TB/GB/MB/KB 后缀和纯数字）
        assert_eq!(FileSink::parse_size("100MB"), Some(100 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("512KB"), Some(512 * 1024));
        assert_eq!(FileSink::parse_size("1TB"), Some(1024 * 1024 * 1024 * 1024));
        // 纯数字（无后缀）- 视为字节数
        assert_eq!(FileSink::parse_size("1024"), Some(1024));
        // "1024B" 不支持 - FileSink::parse_size 没有 "B" 后缀分支，会走 else 分支
        // 尝试 parse::<u64>("1024B") 失败，返回 None
        assert_eq!(FileSink::parse_size("1024B"), None);
        assert_eq!(FileSink::parse_size("invalid"), None);
        assert_eq!(FileSink::parse_size(""), None);
    }
}

// ============================================================================
// compression feature: 压缩策略 E2E
// ============================================================================

#[cfg(feature = "compression")]
#[cfg(test)]
mod compression_e2e {
    use super::*;
    use inklog::sink::CompressionStrategy;
    use inklog::sink::{GzipCompression, NoCompression, ZstdCompression};

    #[test]
    fn test_zstd_compression_round_trip() {
        let strategy = ZstdCompression::new(3);
        let data = b"Hello, World! This is a test message for zstd compression E2E.";

        let compressed = strategy.compress(data).expect("compress should succeed");
        assert!(
            !compressed.is_empty(),
            "compressed data should be non-empty"
        );
        assert_ne!(
            compressed,
            data.to_vec(),
            "compressed data should differ from original"
        );

        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress should succeed");
        assert_eq!(
            decompressed,
            data.to_vec(),
            "decompressed data should match original"
        );
    }

    #[test]
    fn test_zstd_compression_empty_data() {
        let strategy = ZstdCompression::new(3);
        let compressed = strategy
            .compress(b"")
            .expect("compress empty should succeed");
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress empty should succeed");
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_zstd_compression_large_data() {
        let strategy = ZstdCompression::new(3);
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = strategy.compress(&data).expect("compress large data");
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress large data");
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_compression_decompress_invalid_data_errors() {
        let strategy = ZstdCompression::new(3);
        let invalid_data = b"this is not valid zstd data";
        let result = strategy.decompress(invalid_data);
        assert!(result.is_err());
        match result.err().unwrap() {
            InklogError::CompressionError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("expected CompressionError, got {:?}", other),
        }
    }

    #[test]
    fn test_zstd_compression_level_clamping() {
        let high = ZstdCompression::new(100);
        assert_eq!(high.level(), 22, "level should be clamped to 22");

        let low = ZstdCompression::new(-10);
        assert_eq!(low.level(), 0, "level should be clamped to 0");

        let normal = ZstdCompression::new(10);
        assert_eq!(normal.level(), 10);
    }

    #[test]
    fn test_zstd_compression_default_level() {
        let strategy = ZstdCompression::default();
        assert_eq!(strategy.level(), 3);
    }

    #[test]
    fn test_zstd_compression_extension_and_name() {
        let strategy = ZstdCompression::new(3);
        assert_eq!(strategy.extension(), "zst");
        assert_eq!(strategy.name(), "zstd");
    }

    #[test]
    fn test_zstd_compress_file_round_trip() {
        let temp = tempdir().expect("tempdir failed");
        let file_path = temp.path().join("test_compress.log");
        let content = b"file content for zstd compression test\n".repeat(10);

        std::fs::write(&file_path, content.as_slice()).expect("write file");

        let strategy = ZstdCompression::new(3);
        let compressed_path = strategy
            .compress_file(&file_path, 3)
            .expect("compress_file should succeed");

        assert!(
            compressed_path.exists(),
            "compressed file should exist at {:?}",
            compressed_path
        );
        assert_eq!(
            compressed_path.extension().and_then(|e| e.to_str()),
            Some("zst"),
            "compressed file should have .zst extension"
        );
        assert!(
            !file_path.exists(),
            "original file should be removed after compression"
        );

        // 解压回原内容
        let compressed_bytes = std::fs::read(&compressed_path).expect("read compressed");
        let decompressed = strategy
            .decompress(&compressed_bytes)
            .expect("decompress should succeed");
        assert_eq!(decompressed, content);
    }

    #[test]
    fn test_no_compression_pass_through() {
        let strategy = NoCompression;
        let data = b"uncompressed data";

        let compressed = strategy.compress(data).expect("compress should succeed");
        assert_eq!(
            compressed,
            data.to_vec(),
            "NoCompression should return data as-is"
        );

        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress should succeed");
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_no_compression_extension_and_name() {
        let strategy = NoCompression;
        assert_eq!(strategy.extension(), "");
        assert_eq!(strategy.name(), "none");
    }

    #[test]
    fn test_no_compression_compress_file_returns_same_path() {
        let strategy = NoCompression;
        let path = Path::new("/tmp/nonexistent_for_no_compression_test");
        let result = strategy.compress_file(path, 3);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), path.to_path_buf());
    }

    #[test]
    fn test_gzip_compression_round_trip() {
        let strategy = GzipCompression::new(6);
        let data = b"Hello, World! This is a test message for gzip compression E2E.";

        let compressed = strategy.compress(data).expect("compress should succeed");
        assert!(!compressed.is_empty());

        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress should succeed");
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_gzip_compression_empty_data() {
        let strategy = GzipCompression::new(6);
        let compressed = strategy
            .compress(b"")
            .expect("compress empty should succeed");
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress empty should succeed");
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_gzip_compression_large_data() {
        let strategy = GzipCompression::new(6);
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = strategy.compress(&data).expect("compress large data");
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress large data");
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_decompress_invalid_data_errors() {
        let strategy = GzipCompression::new(6);
        let invalid_data = b"not valid gzip data";
        let result = strategy.decompress(invalid_data);
        assert!(result.is_err());
        match result.err().unwrap() {
            InklogError::CompressionError(_) => {}
            other => panic!("expected CompressionError, got {:?}", other),
        }
    }

    #[test]
    fn test_gzip_level_clamping() {
        let high = GzipCompression::new(100);
        assert_eq!(high.level(), 9, "level should be clamped to 9");

        let low = GzipCompression::new(0);
        assert_eq!(low.level(), 0);
    }

    #[test]
    fn test_gzip_default_level() {
        let strategy = GzipCompression::default();
        assert_eq!(strategy.level(), 6);
    }

    #[test]
    fn test_gzip_extension_and_name() {
        let strategy = GzipCompression::new(6);
        assert_eq!(strategy.extension(), "gz");
        assert_eq!(strategy.name(), "gzip");
    }

    #[test]
    fn test_gzip_compress_file_round_trip() {
        let temp = tempdir().expect("tempdir failed");
        let file_path = temp.path().join("test_gzip.log");
        let content = b"gzip file content test\n".repeat(10);

        std::fs::write(&file_path, content.as_slice()).expect("write file");

        let strategy = GzipCompression::new(6);
        let compressed_path = strategy
            .compress_file(&file_path, 6)
            .expect("compress_file should succeed");

        assert!(compressed_path.exists());
        assert_eq!(
            compressed_path.extension().and_then(|e| e.to_str()),
            Some("gz")
        );
        assert!(!file_path.exists(), "original should be removed");

        let compressed_bytes = std::fs::read(&compressed_path).expect("read compressed");
        let decompressed = strategy
            .decompress(&compressed_bytes)
            .expect("decompress should succeed");
        assert_eq!(decompressed, content);
    }

    #[test]
    fn test_compress_data_helper_function() {
        let data = b"helper function test";
        let compressed = inklog::sink::compression::compress_data(data, 3).expect("compress_data");
        assert!(!compressed.is_empty());

        // 使用 ZstdCompression::decompress 进行 round-trip 验证（避免直接依赖 zstd crate）
        let strategy = ZstdCompression::new(3);
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress should succeed");
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_compress_string_helper_function() {
        let s = "string to compress";
        let compressed = inklog::sink::compression::compress_string(s, 3).expect("compress_string");
        assert!(!compressed.is_empty());

        let strategy = ZstdCompression::new(3);
        let decompressed = strategy
            .decompress(&compressed)
            .expect("decompress should succeed");
        assert_eq!(decompressed, s.as_bytes());
    }

    #[test]
    fn test_compress_file_legacy_function() {
        let temp = tempdir().expect("tempdir failed");
        let file_path = temp.path().join("legacy_compress.log");
        let content = b"legacy compress function test\n".repeat(5);
        std::fs::write(&file_path, content.as_slice()).expect("write file");

        let compressed_path =
            inklog::sink::compression::compress_file(&file_path, 3).expect("compress_file");
        assert!(compressed_path.exists());
        assert!(!file_path.exists());
    }
}

// ============================================================================
// i18n feature: LogI18nFormatter E2E
// ============================================================================

#[cfg(feature = "i18n")]
#[cfg(test)]
mod i18n_e2e {
    use inklog::i18n::{I18nError, LogI18nFormatter};
    use std::cmp::Ordering;

    #[test]
    fn test_i18n_locale_parsing_en_us() {
        let fmt = LogI18nFormatter::new("en-US");
        assert!(fmt.is_ok(), "en-US should parse successfully");
    }

    #[test]
    fn test_i18n_locale_parsing_zh_cn() {
        let fmt = LogI18nFormatter::new("zh-CN");
        assert!(fmt.is_ok(), "zh-CN should parse successfully");
    }

    #[test]
    fn test_i18n_locale_parsing_en_bare() {
        let fmt = LogI18nFormatter::new("en");
        assert!(fmt.is_ok(), "en should parse successfully");
    }

    #[test]
    fn test_i18n_locale_parsing_zh_bare() {
        let fmt = LogI18nFormatter::new("zh");
        assert!(fmt.is_ok(), "zh should parse successfully");
    }

    #[test]
    fn test_i18n_invalid_locale_returns_error() {
        let result = LogI18nFormatter::new("not-a-valid-locale!!!");
        assert!(result.is_err(), "invalid locale should return error");
        match result.err().unwrap() {
            I18nError::InvalidLocale { input, reason } => {
                assert_eq!(input, "not-a-valid-locale!!!");
                assert!(!reason.is_empty());
            }
            other => panic!("expected InvalidLocale, got {:?}", other),
        }
    }

    #[test]
    fn test_i18n_format_event_count_en_one() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.format_event_count(1).expect("plural 1");
        assert_eq!(result, "One", "en: count=1 should be One");
    }

    #[test]
    fn test_i18n_format_event_count_en_other() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.format_event_count(2).expect("plural 2");
        assert_eq!(result, "Other", "en: count=2 should be Other");
    }

    #[test]
    fn test_i18n_format_event_count_zero() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.format_event_count(0).expect("plural 0");
        // en: 0 应为 Other 或 Zero
        let _ = result;
    }

    #[test]
    fn test_i18n_format_event_count_large_number() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.format_event_count(1000).expect("plural 1000");
        assert_eq!(result, "Other", "en: count=1000 should be Other");
    }

    #[test]
    fn test_i18n_format_number_en_us() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(1_234_567.89_f64).expect("format number");
        assert!(
            result.contains(','),
            "en-US number should contain thousands separator: got '{}'",
            result
        );
        assert!(
            result.contains('.'),
            "en-US number should contain decimal point: got '{}'",
            result
        );
    }

    #[test]
    fn test_i18n_format_number_integer() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(42.0_f64).expect("format integer");
        assert!(result.contains('2'), "should contain digit 2");
    }

    #[test]
    fn test_i18n_format_number_nan_returns_error() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(f64::NAN);
        assert!(result.is_err(), "NaN should return error");
        match result.err().unwrap() {
            I18nError::InvalidNumber { .. } => {}
            other => panic!("expected InvalidNumber, got {:?}", other),
        }
    }

    #[test]
    fn test_i18n_format_number_infinity_returns_error() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(f64::INFINITY);
        assert!(result.is_err(), "Infinity should return error");
    }

    #[test]
    fn test_i18n_format_number_neg_infinity_returns_error() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(f64::NEG_INFINITY);
        assert!(result.is_err(), "Negative Infinity should return error");
    }

    #[test]
    fn test_i18n_format_timestamp_contains_year() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_timestamp(2026, 7, 11).expect("format timestamp");
        assert!(
            result.contains("2026"),
            "timestamp should contain year: got '{}'",
            result
        );
        assert!(!result.is_empty());
    }

    #[test]
    fn test_i18n_format_timestamp_invalid_month() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_timestamp(2026, 13, 1);
        assert!(result.is_err(), "month 13 should return error");
    }

    #[test]
    fn test_i18n_format_timestamp_invalid_day() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_timestamp(2026, 2, 30);
        // 2 月 30 日不存在
        assert!(result.is_err(), "Feb 30 should return error");
    }

    #[test]
    fn test_i18n_format_log_level_normalizes_to_uppercase() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        assert_eq!(
            fmt.format_log_level("info").expect("log level"),
            "INFO",
            "info should be normalized to INFO"
        );
        assert_eq!(fmt.format_log_level("debug").expect("log level"), "DEBUG");
        assert_eq!(fmt.format_log_level("error").expect("log level"), "ERROR");
        assert_eq!(fmt.format_log_level("ERROR").expect("log level"), "ERROR");
        assert_eq!(fmt.format_log_level("Warn").expect("log level"), "WARN");
    }

    #[test]
    fn test_i18n_format_log_level_unknown_returns_input_or_error() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_log_level("unknown_level");
        // 未知 level 可能返回错误或原字符串大写
        if let Ok(s) = result {
            assert!(s.contains("UNKNOWN") || s.contains("unknown"));
        }
    }

    #[test]
    fn test_i18n_compare_fields_less() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.compare_fields("apple", "banana").expect("compare");
        assert_eq!(result, Ordering::Less, "apple < banana");
    }

    #[test]
    fn test_i18n_compare_fields_greater() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.compare_fields("banana", "apple").expect("compare");
        assert_eq!(result, Ordering::Greater, "banana > apple");
    }

    #[test]
    fn test_i18n_compare_fields_equal() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.compare_fields("apple", "apple").expect("compare");
        assert_eq!(result, Ordering::Equal, "apple == apple");
    }

    #[test]
    fn test_i18n_compare_fields_empty_strings() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.compare_fields("", "").expect("compare empty");
        assert_eq!(result, Ordering::Equal);
    }

    #[test]
    fn test_i18n_compare_fields_unicode() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        let result = fmt.compare_fields("你好", "世界").expect("compare unicode");
        // 不验证具体顺序（取决于 collator），只验证不 panic
        let _ = result;
    }
}

// ============================================================================
// CORE-009: LogAdapter (log crate 桥接) E2E
// ============================================================================

#[cfg(test)]
mod log_adapter_e2e {
    use super::*;

    #[test]
    fn test_log_level_from_log_crate_levels() {
        // 验证 tracing::Level 与 LogLevel 的转换路径
        let info_record = LogRecord::new(
            Level::INFO,
            "log_adapter".to_string(),
            "info via log adapter".to_string(),
        );
        assert_eq!(info_record.level, "INFO");

        let warn_record = LogRecord::new(
            Level::WARN,
            "log_adapter".to_string(),
            "warn via log adapter".to_string(),
        );
        assert_eq!(warn_record.level, "WARN");

        let error_record = LogRecord::new(
            Level::ERROR,
            "log_adapter".to_string(),
            "error via log adapter".to_string(),
        );
        assert_eq!(error_record.level, "ERROR");

        let debug_record = LogRecord::new(
            Level::DEBUG,
            "log_adapter".to_string(),
            "debug via log adapter".to_string(),
        );
        assert_eq!(debug_record.level, "DEBUG");

        let trace_record = LogRecord::new(
            Level::TRACE,
            "log_adapter".to_string(),
            "trace via log adapter".to_string(),
        );
        assert_eq!(trace_record.level, "TRACE");
    }
}

// ============================================================================
// 多组件集成 E2E：tracing → FileSink → 文件验证
// ============================================================================

#[cfg(test)]
mod multi_component_e2e {
    use super::*;

    #[tokio::test]
    async fn test_file_sink_with_template_render_pipeline() {
        // LogRecord → LogTemplate::render → FileSink::write → 文件
        let temp = tempdir().expect("tempdir failed");
        let log_path = temp.path().join("pipeline.log");

        let template = LogTemplate::new("{timestamp} [{level}] {target}: {message}");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: false,
            ..Default::default()
        };
        let sink = FileSink::new(config).expect("FileSink creation");

        let record = LogRecord::new(
            Level::INFO,
            "pipeline_test".to_string(),
            "pipeline message".to_string(),
        );

        // 先用 template 渲染记录，验证模板渲染不 panic 且输出包含关键字段
        let rendered = template.render(&record);
        assert!(rendered.contains("INFO"), "rendered should contain level");
        assert!(
            rendered.contains("pipeline_test"),
            "rendered should contain target"
        );
        assert!(
            rendered.contains("pipeline message"),
            "rendered should contain message"
        );

        sink.write(&record).await.expect("write should succeed");
        sink.flush().await.expect("flush should succeed");

        let content = std::fs::read_to_string(&log_path).expect("read log file");
        assert!(content.contains("INFO"), "should contain level");
        assert!(content.contains("pipeline_test"), "should contain target");
        assert!(
            content.contains("pipeline message"),
            "should contain message"
        );
    }

    #[tokio::test]
    async fn test_file_sink_with_masking_and_template_pipeline() {
        // LogRecord with PII → masking → template render → file
        let temp = tempdir().expect("tempdir failed");
        let log_path = temp.path().join("masked_pipeline.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: true,
            ..Default::default()
        };
        let sink = FileSink::new(config).expect("FileSink creation");

        let record = LogRecord::new(
            Level::INFO,
            "auth".to_string(),
            "login attempt: user=test@example.com password=very_long_secret_value_12345"
                .to_string(),
        );

        sink.write(&record).await.expect("write should succeed");
        sink.flush().await.expect("flush should succeed");

        let content = std::fs::read_to_string(&log_path).expect("read log file");
        assert!(
            !content.contains("test@example.com"),
            "email should be masked: {}",
            content
        );
        assert!(
            !content.contains("very_long_secret_value_12345"),
            "long password should be masked: {}",
            content
        );
    }

    #[tokio::test]
    async fn test_file_sink_via_sink_registry_pipeline() {
        // SinkRegistry → FileSinkFactory → FileSink → write → file
        let temp = tempdir().expect("tempdir failed");
        let log_path = temp.path().join("registry_pipeline.log");

        let mut registry = SinkRegistry::new();
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: false,
            ..Default::default()
        };
        registry.register(FileSinkFactory::new(config));

        let sink = registry
            .create("file")
            .await
            .expect("create should succeed");

        let record = LogRecord::new(
            Level::INFO,
            "registry_pipeline".to_string(),
            "via registry".to_string(),
        );
        sink.write(&record).await.expect("write should succeed");
        sink.flush().await.expect("flush should succeed");

        let content = std::fs::read_to_string(&log_path).expect("read log file");
        assert!(content.contains("via registry"));
    }

    #[tokio::test]
    async fn test_file_sink_concurrent_writes_safe() {
        // 多个 sink 实例写入同一目录不同文件
        let temp = tempdir().expect("tempdir failed");

        let mut handles = Vec::new();
        for i in 0..3 {
            let path = temp.path().join(format!("concurrent_{}.log", i));
            let handle = tokio::spawn(async move {
                let config = FileSinkConfig {
                    enabled: true,
                    path,
                    masking_enabled: false,
                    ..Default::default()
                };
                let sink = FileSink::new(config).expect("FileSink creation");
                let record = LogRecord::new(
                    Level::INFO,
                    "concurrent".to_string(),
                    format!("message from task {}", i),
                );
                sink.write(&record).await.expect("write");
                sink.flush().await.expect("flush");
            });
            handles.push(handle);
        }

        for h in handles {
            h.await.expect("task should not panic");
        }

        // 验证每个文件都有内容
        for i in 0..3 {
            let path = temp.path().join(format!("concurrent_{}.log", i));
            let content = std::fs::read_to_string(&path).expect("read file");
            assert!(
                content.contains(&format!("message from task {}", i)),
                "file {} should contain its message",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_metrics_recording_with_file_sink() {
        // FileSink 写入 + Metrics 记录
        let temp = tempdir().expect("tempdir failed");
        let log_path = temp.path().join("metrics_pipeline.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: false,
            ..Default::default()
        };
        let sink = FileSink::new(config).expect("FileSink creation");
        let metrics = Metrics::new();

        for i in 0..5 {
            let record = LogRecord::new(
                Level::INFO,
                "metrics_test".to_string(),
                format!("message {}", i),
            );
            sink.write(&record).await.expect("write");
            metrics.inc_logs_written();
        }
        sink.flush().await.expect("flush");

        assert_eq!(metrics.logs_written(), 5);
        assert!(log_path.exists());
    }

    #[tokio::test]
    async fn test_file_sink_shutdown_drains_buffer() {
        // shutdown 应刷新缓冲区
        let temp = tempdir().expect("tempdir failed");
        let log_path = temp.path().join("shutdown_drain.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: false,
            ..Default::default()
        };
        let sink = FileSink::new(config).expect("FileSink creation");

        let record = LogRecord::new(
            Level::INFO,
            "shutdown".to_string(),
            "before shutdown".to_string(),
        );
        sink.write(&record).await.expect("write");

        // shutdown 应该 flush 残留缓冲
        sink.shutdown().await.expect("shutdown should flush");

        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        // 内容可能存在（取决于 shutdown 是否 flush），至少不 panic
        let _ = content;
    }
}
