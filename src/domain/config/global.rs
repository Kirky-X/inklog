// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Global logger configuration.

use serde::{Deserialize, Serialize};

use crate::support::processing::template::OutputFormat;

/// Returns `true` — shared default for several `bool` fields.
pub(crate) fn default_true() -> bool {
    true
}

// ============================================================================
// GlobalConfig - Global logger settings
// ============================================================================

/// Global logger configuration.
///
/// Controls the overall behavior of the logging system including log level,
/// format string, and fallback settings.
///
/// # Configuration Priority
///
/// Configuration values are loaded with the following priority (highest to lowest):
/// 1. Environment variables (prefix `INKLOG_GLOBAL_`)
/// 2. Configuration file values
/// 3. Default values
///
/// # Example TOML Configuration
///
/// ```toml
/// [global]
/// level = "debug"
/// format = "{timestamp} [{level}] {target} - {message}"
/// masking_enabled = true
/// auto_fallback = true
/// fallback_initial_delay_ms = 1000
/// fallback_max_delay_ms = 60000
/// fallback_max_retries = 10
/// ```
///
/// # Environment Variable Overrides
///
/// Any field can be overridden via environment variables:
/// ```bash
/// export INKLOG_GLOBAL_LEVEL=debug
/// export INKLOG_GLOBAL_MASKING_ENABLED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    /// Minimum log level to capture.
    ///
    /// Valid values (case-insensitive): `trace`, `debug`, `info`, `warn`, `error`, `fatal`.
    /// Logs below this level are ignored.
    ///
    /// # Default
    ///
    /// `"info"` - Captures INFO, WARN, ERROR, and FATAL logs.
    #[serde(default = "default_global_level")]
    pub level: String,

    /// Log message format template.
    ///
    /// Supports placeholders that are replaced with values from each log record:
    /// - `{timestamp}` - ISO 8601 timestamp
    /// - `{level}` - Log level (INFO, DEBUG, etc.)
    /// - `{target}` - Module path that emitted the log
    /// - `{message}` - Log message content
    /// - `{file}` - Source file path (optional)
    /// - `{line}` - Line number in source file (optional)
    /// - `{thread_id}` - Thread identifier
    /// - `{fields}` - Additional structured fields (JSON)
    ///
    /// # Default
    ///
    /// `"{timestamp} [{level}] {target} - {message}"`
    #[serde(default = "default_global_format")]
    pub format: String,

    /// Enable the log sanitizer: CWE-117 injection escaping (newlines/control
    /// characters) plus message-level sensitive-data redaction, applied by the
    /// tracing subscriber before records enter any sink.
    ///
    /// 推荐新名 `sanitizer_enabled`；旧名 `masking_enabled` 为兼容别名，
    /// 继续接受（serde alias），序列化时始终输出旧名。
    ///
    /// # 三个易混淆开关的分工
    ///
    /// - **sanitizer**（本字段，推荐名 `sanitizer_enabled`）：注入转义与消息
    ///   脱敏，作用于 subscriber 入口（全局）。
    /// - **pii_masking**（`console_sink` / `file_sink` 的 `masking_enabled`，
    ///   推荐名 `pii_masking_enabled`）：结构化 PII 掩码（邮箱、手机号、证件/
    ///   银行卡号、敏感字段名），作用于 sink 输出前。
    /// - **safe_message**（`InklogError::safe_message`，error.rs 的
    ///   SENSITIVE_PATTERNS）：错误消息出口脱敏，独立于以上两个开关。
    ///
    /// # Default
    ///
    /// `true` - Masking enabled by default for security.
    #[serde(default = "default_true", alias = "sanitizer_enabled")]
    pub masking_enabled: bool,

    /// Enable automatic fallback on sink failures.
    ///
    /// When a sink fails repeatedly, the system automatically falls back to
    /// alternative sinks (e.g., database → file → console).
    ///
    /// # Default
    ///
    /// `true` - Fallback enabled for reliability.
    #[serde(default = "default_true")]
    pub auto_fallback: bool,

    /// Initial delay before first retry (milliseconds).
    ///
    /// When a sink fails, the system waits this duration before attempting
    /// the first retry. Subsequent retries use exponential backoff.
    ///
    /// # Default
    ///
    /// `1000` ms (1 second)
    #[serde(default = "default_fallback_initial_delay")]
    pub fallback_initial_delay_ms: u64,

    /// Maximum delay between retries (milliseconds).
    ///
    /// Caps the exponential backoff delay to prevent excessive waiting.
    ///
    /// # Default
    ///
    /// `60000` ms (60 seconds)
    #[serde(default = "default_fallback_max_delay")]
    pub fallback_max_delay_ms: u64,

    /// Maximum number of retry attempts.
    ///
    /// After this many failures, the sink is marked as unhealthy and
    /// fallback mechanisms are activated.
    ///
    /// # Default
    ///
    /// `10` retries
    #[serde(default = "default_fallback_max_retries")]
    pub fallback_max_retries: u32,

    /// Output format for log sinks.
    ///
    /// Controls whether logs are rendered as human-readable text or JSON.
    ///
    /// # Default
    ///
    /// `OutputFormat::Text`
    #[serde(default)]
    pub output_format: OutputFormat,
}

fn default_global_level() -> String {
    "info".to_string()
}
fn default_global_format() -> String {
    "{timestamp} [{level}] {target} - {message}".to_string()
}
fn default_fallback_initial_delay() -> u64 {
    1000
}
fn default_fallback_max_delay() -> u64 {
    60000
}
fn default_fallback_max_retries() -> u32 {
    10
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: default_global_level(),
            format: default_global_format(),
            masking_enabled: default_true(),
            auto_fallback: default_true(),
            fallback_initial_delay_ms: default_fallback_initial_delay(),
            fallback_max_delay_ms: default_fallback_max_delay(),
            fallback_max_retries: default_fallback_max_retries(),
            output_format: OutputFormat::default(),
        }
    }
}

impl GlobalConfig {
    /// Validate and adjust the configuration.
    ///
    /// # Behavior
    ///
    /// Some invalid values are corrected **in place** with a `tracing::warn!`
    /// for each adjustment:
    /// - `fallback_initial_delay_ms` is clamped to `fallback_max_delay_ms`
    /// - `fallback_max_retries == 0` is reset to `1`
    ///
    /// An invalid `level` cannot be corrected safely and returns `Err`.
    pub fn validate(&mut self) -> Result<(), String> {
        if self.fallback_initial_delay_ms > self.fallback_max_delay_ms {
            tracing::warn!("{}", crate::i18n::tr("warn-delay_clamp"));
            self.fallback_initial_delay_ms = self.fallback_max_delay_ms;
        }
        if self.fallback_max_retries == 0 {
            tracing::warn!("{}", crate::i18n::tr("warn-fallback_retries_zero"));
            self.fallback_max_retries = 1;
        }
        if !self.level.is_empty() && !crate::LogLevel::is_valid_level(&self.level) {
            return Err(format!(
                "invalid log level \"{}\", expected one of: {}",
                self.level,
                crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let cfg = GlobalConfig::default();
        assert_eq!(cfg.level, "info");
        assert_eq!(cfg.format, "{timestamp} [{level}] {target} - {message}");
        assert!(cfg.masking_enabled);
        assert!(cfg.auto_fallback);
        assert_eq!(cfg.fallback_initial_delay_ms, 1000);
        assert_eq!(cfg.fallback_max_delay_ms, 60000);
        assert_eq!(cfg.fallback_max_retries, 10);
    }

    #[test]
    fn test_validate_clamps_initial_delay() {
        let mut cfg = GlobalConfig::default();
        cfg.fallback_initial_delay_ms = 99999;
        cfg.fallback_max_delay_ms = 5000;
        cfg.validate().unwrap();
        assert_eq!(cfg.fallback_initial_delay_ms, 5000);
    }

    #[test]
    fn test_validate_resets_zero_retries() {
        let mut cfg = GlobalConfig::default();
        cfg.fallback_max_retries = 0;
        cfg.validate().unwrap();
        assert_eq!(cfg.fallback_max_retries, 1);
    }

    #[test]
    fn test_validate_no_change_when_valid() {
        let mut cfg = GlobalConfig::default();
        cfg.fallback_initial_delay_ms = 1000;
        cfg.fallback_max_delay_ms = 60000;
        cfg.fallback_max_retries = 3;
        cfg.validate().unwrap();
        assert_eq!(cfg.fallback_initial_delay_ms, 1000);
        assert_eq!(cfg.fallback_max_retries, 3);
    }

    #[test]
    fn test_validate_rejects_invalid_level() {
        let mut cfg = GlobalConfig::default();
        cfg.level = "verbose".to_string();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("verbose"), "unexpected error: {err}");
    }

    #[test]
    fn test_validate_accepts_valid_levels() {
        for level in ["trace", "debug", "info", "warn", "error", "fatal", "INFO"] {
            let mut cfg = GlobalConfig::default();
            cfg.level = level.to_string();
            assert!(
                cfg.validate().is_ok(),
                "level '{level}' should be accepted"
            );
        }
    }

    #[test]
    fn test_validate_allows_empty_level() {
        // 空 level 交由上层加载逻辑决定默认值，此处不视为非法
        let mut cfg = GlobalConfig::default();
        cfg.level = String::new();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_deserialize_masking_enabled_legacy_and_alias_keys() {
        // 旧键名 `masking_enabled`：既有配置文件必须继续解析（兼容别名）
        let legacy: GlobalConfig =
            toml::from_str("level = \"info\"\nmasking_enabled = false\n").unwrap();
        assert!(!legacy.masking_enabled);

        // 新键名 `sanitizer_enabled`（推荐写法，serde alias）解析到同一字段
        let renamed: GlobalConfig =
            toml::from_str("level = \"info\"\nsanitizer_enabled = false\n").unwrap();
        assert!(!renamed.masking_enabled);
    }

    #[test]
    fn test_serialize_emits_legacy_masking_enabled_key() {
        // 序列化始终输出旧键名，保证既有配置消费者/生成器不受影响
        let cfg = GlobalConfig {
            masking_enabled: false,
            ..Default::default()
        };
        let rendered = toml::to_string(&cfg).unwrap();
        assert!(
            rendered.contains("masking_enabled = false"),
            "serialized TOML must keep the legacy key, got: {rendered}"
        );
        assert!(
            !rendered.contains("sanitizer_enabled"),
            "alias must not be emitted on serialization, got: {rendered}"
        );
    }
}
