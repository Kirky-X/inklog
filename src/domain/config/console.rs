// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Console sink configuration.

use super::global::default_true;
use crate::support::processing::template::OutputFormat;
use serde::{Deserialize, Serialize};

// ============================================================================
// ConsoleSinkConfig - Console output settings
// ============================================================================

/// Console sink configuration.
///
/// Controls logging output to stdout/stderr with optional colored output
/// and level-based stream routing.
///
/// # Example TOML Configuration
///
/// ```toml
/// [console_sink]
/// enabled = true
/// colored = true
/// stderr_levels = ["error", "warn"]
/// masking_enabled = true
/// ```
///
/// # Stream Routing
///
/// Log levels specified in `stderr_levels` are written to stderr,
/// all other levels go to stdout. This enables:
/// - Separating errors from normal output
/// - Piping stdout to files while keeping errors visible
/// - Integration with monitoring tools that parse stderr
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_CONSOLE_SINK_ENABLED=true
/// export INKLOG_CONSOLE_SINK_COLORED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSinkConfig {
    /// Enable console logging.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Enable colored output using ANSI escape codes.
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Log levels to write to stderr instead of stdout.
    #[serde(default = "default_stderr_levels")]
    pub stderr_levels: Vec<String>,

    /// Enable PII masking for console output: structured PII (emails, phone
    /// numbers, ID/bank card numbers) and values under sensitive field names
    /// are masked before the record is written to stdout/stderr.
    ///
    /// 推荐新名 `pii_masking_enabled`；旧名 `masking_enabled` 为兼容别名，
    /// 继续接受（serde alias），序列化时始终输出旧名。
    ///
    /// # 三个易混淆开关的分工
    ///
    /// - **sanitizer**（`global.masking_enabled`，推荐名 `sanitizer_enabled`）：
    ///   注入转义与消息脱敏，作用于 subscriber 入口（全局）。
    /// - **pii_masking**（本字段，推荐名 `pii_masking_enabled`）：结构化 PII
    ///   掩码，作用于本 sink 输出前。
    /// - **safe_message**（`InklogError::safe_message`，error.rs 的
    ///   SENSITIVE_PATTERNS）：错误消息出口脱敏，独立于以上两个开关。
    ///
    /// # Default
    ///
    /// `true` - Masking enabled by default for security consistency with
    /// [`GlobalConfig`](super::GlobalConfig).
    #[serde(default = "default_true", alias = "pii_masking_enabled")]
    pub masking_enabled: bool,

    /// Output format: text (template-based) or JSON (NDJSON).
    ///
    /// When `Json`, colored output is automatically disabled.
    #[serde(default)]
    pub output_format: OutputFormat,
}

fn default_stderr_levels() -> Vec<String> {
    vec!["error".to_string(), "warn".to_string()]
}

impl Default for ConsoleSinkConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            colored: default_true(),
            stderr_levels: default_stderr_levels(),
            masking_enabled: default_true(),
            output_format: OutputFormat::default(),
        }
    }
}

impl ConsoleSinkConfig {
    /// Validate console sink configuration.
    ///
    /// Ensures `stderr_levels` contains only valid log level names.
    /// Invalid entries are removed with a warning.
    pub fn validate(&mut self) {
        let original_len = self.stderr_levels.len();
        self.stderr_levels.retain(|level| {
            if !crate::LogLevel::is_valid_level(level) {
                tracing::warn!(level = %level, "Invalid stderr_levels entry, removing");
                false
            } else {
                true
            }
        });
        if self.stderr_levels.len() != original_len {
            tracing::info!(
                remaining = self.stderr_levels.len(),
                "Removed invalid stderr_levels entries"
            );
        }
    }

    /// Return invalid `stderr_levels` entries without mutating.
    ///
    /// Useful for strict validation before normalization auto-corrects them.
    pub fn invalid_stderr_levels(&self) -> Vec<&str> {
        self.stderr_levels
            .iter()
            .filter(|level| !crate::LogLevel::is_valid_level(level))
            .map(|s| s.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_console_sink_config() {
        let cfg = ConsoleSinkConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.colored);
        assert_eq!(
            cfg.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
        assert!(cfg.masking_enabled);
    }

    #[test]
    fn test_validate_removes_invalid_levels() {
        let mut cfg = ConsoleSinkConfig::default();
        cfg.stderr_levels = vec![
            "error".into(),
            "invalid_level".into(),
            "warn".into(),
            "bogus".into(),
        ];
        cfg.validate();
        assert_eq!(
            cfg.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
    }

    #[test]
    fn test_validate_keeps_valid_levels() {
        let mut cfg = ConsoleSinkConfig::default();
        cfg.stderr_levels = vec![
            "trace".into(),
            "debug".into(),
            "info".into(),
            "warn".into(),
            "warning".into(),
            "error".into(),
            "fatal".into(),
            "critical".into(),
        ];
        let original_len = cfg.stderr_levels.len();
        cfg.validate();
        assert_eq!(cfg.stderr_levels.len(), original_len);
    }

    #[test]
    fn test_invalid_stderr_levels_returns_invalid() {
        let cfg = ConsoleSinkConfig {
            stderr_levels: vec![
                "error".into(),
                "bogus".into(),
                "warn".into(),
                "typo_err".into(),
            ],
            ..Default::default()
        };
        let invalid = cfg.invalid_stderr_levels();
        assert_eq!(invalid, vec!["bogus", "typo_err"]);
    }

    #[test]
    fn test_invalid_stderr_levels_empty_when_all_valid() {
        let cfg = ConsoleSinkConfig {
            stderr_levels: vec!["error".into(), "warn".into()],
            ..Default::default()
        };
        assert!(cfg.invalid_stderr_levels().is_empty());
    }

    #[test]
    fn test_deserialize_masking_enabled_legacy_and_alias_keys() {
        // 旧键名 `masking_enabled`：既有配置文件必须继续解析（兼容别名）
        let legacy: ConsoleSinkConfig =
            toml::from_str("masking_enabled = false\n").unwrap();
        assert!(!legacy.masking_enabled);

        // 新键名 `pii_masking_enabled`（推荐写法，serde alias）解析到同一字段
        let renamed: ConsoleSinkConfig =
            toml::from_str("pii_masking_enabled = false\n").unwrap();
        assert!(!renamed.masking_enabled);
    }

    #[test]
    fn test_serialize_emits_legacy_masking_enabled_key() {
        // 序列化始终输出旧键名，保证既有配置消费者/生成器不受影响
        let cfg = ConsoleSinkConfig {
            masking_enabled: false,
            ..Default::default()
        };
        let rendered = toml::to_string(&cfg).unwrap();
        assert!(
            rendered.contains("masking_enabled = false"),
            "serialized TOML must keep the legacy key, got: {rendered}"
        );
        assert!(
            !rendered.contains("pii_masking_enabled"),
            "alias must not be emitted on serialization, got: {rendered}"
        );
    }
}
