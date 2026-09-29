// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Global logger configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

    /// 磁盘持久化 fallback 队列（deferred-capabilities C4）：兜底缓冲溢出/
    /// 淘汰的 ERROR/FATAL 记录落盘 JSONL，进程启动时重放一次后清空。
    ///
    /// # Default
    ///
    /// `false`（关闭时全链路零文件 IO，行为与既往一致）
    #[serde(default)]
    pub fallback_journal: bool,

    /// fallback journal 文件路径（`fallback_journal` 开启时生效）。
    ///
    /// # Default
    ///
    /// `logs/fallback.journal`
    #[serde(default = "default_fallback_journal_path")]
    pub fallback_journal_path: String,

    /// 服务名（静态身份字段，注入每条日志记录的 `fields`，键名
    /// `service_name`）。未设置（None）时不注入。
    ///
    /// # Default
    ///
    /// `None`
    #[serde(default)]
    pub service_name: Option<String>,

    /// 服务实例标识（注入键名 `service_instance`，如 pod 名 / 进程 id 组合）。
    ///
    /// # Default
    ///
    /// `None`
    #[serde(default)]
    pub service_instance: Option<String>,

    /// 部署环境（注入键名 `service_env`，如 `prod` / `staging`）。
    ///
    /// # Default
    ///
    /// `None`
    #[serde(default)]
    pub service_env: Option<String>,

    /// 服务版本（注入键名 `service_version`）。
    ///
    /// # Default
    ///
    /// `None`
    #[serde(default)]
    pub service_version: Option<String>,

    /// 附加静态键值（与身份字段一并注入 `fields`，键名原样保留）。
    /// 用于 region/az/tenant 等部署维度标注。
    ///
    /// # Default
    ///
    /// 空 map
    #[serde(default)]
    pub static_fields: HashMap<String, String>,
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
fn default_fallback_journal_path() -> String {
    "logs/fallback.journal".to_string()
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
            fallback_journal: false,
            fallback_journal_path: default_fallback_journal_path(),
            service_name: None,
            service_instance: None,
            service_env: None,
            service_version: None,
            static_fields: HashMap::new(),
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
        self.validate_identity()?;
        Ok(())
    }

    /// 已配置身份字段的注入映射（键名 → 值；未配置任何身份时为空 map）。
    ///
    /// 固定身份键：`service_name` / `service_instance` / `service_env` /
    /// `service_version`——这四个键是**保留键**；`static_fields` 的键原样
    /// 保留并后写优先（可覆盖固定键），应用应避免以 `service_` 前缀命名
    /// 业务字段。空白值跳过（深度防御：绕过 validate 的链路不注入空白身份）。
    pub fn identity_fields(&self) -> std::collections::BTreeMap<String, serde_json::Value> {
        let mut map = std::collections::BTreeMap::new();
        let scalars = [
            ("service_name", &self.service_name),
            ("service_instance", &self.service_instance),
            ("service_env", &self.service_env),
            ("service_version", &self.service_version),
        ];
        for (key, value) in scalars {
            if let Some(v) = value {
                if v.trim().is_empty() {
                    continue;
                }
                map.insert(key.to_string(), serde_json::Value::String(v.clone()));
            }
        }
        for (key, value) in &self.static_fields {
            if value.trim().is_empty() {
                continue;
            }
            map.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        map
    }

    /// 身份字段校验（只读入口）：已设置（Some）的标量不得为空白；键/值不得
    /// 含控制字符（换行等会破坏日志行结构，TOML/env/DI 三条配置链统一拒绝）。
    ///
    /// [`InklogConfig::validate`](crate::domain::config::InklogConfig::validate)
    /// 与 manager 的 DI 装配链都经由本入口执行硬校验。
    pub fn validate_identity(&self) -> Result<(), String> {
        let scalars = [
            ("service_name", &self.service_name),
            ("service_instance", &self.service_instance),
            ("service_env", &self.service_env),
            ("service_version", &self.service_version),
        ];
        for (name, value) in scalars {
            if let Some(v) = value {
                if v.trim().is_empty() {
                    return Err(format!("global.{name} is set but blank"));
                }
                reject_control_chars(&format!("global.{name}"), v)?;
            }
        }
        for (key, value) in &self.static_fields {
            if key.is_empty() {
                return Err("global.static_fields contains an empty key".to_string());
            }
            reject_control_chars("global.static_fields key", key)?;
            reject_control_chars(&format!("global.static_fields[{key}]"), value)?;
        }
        Ok(())
    }
}

/// 拒绝含控制字符的配置值（CWE-117 邻域：换行/制表等会破坏日志行结构）。
pub(crate) fn reject_control_chars(field: &str, value: &str) -> Result<(), String> {
    if let Some(c) = value.chars().find(|&c| char::is_control(c)) {
        return Err(format!(
            "{field} contains a control character (U+{:04X})",
            c as u32
        ));
    }
    Ok(())
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
            assert!(cfg.validate().is_ok(), "level '{level}' should be accepted");
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

    #[test]
    fn test_identity_fields_default_empty() {
        let cfg = GlobalConfig::default();
        assert!(cfg.service_name.is_none());
        assert!(cfg.service_instance.is_none());
        assert!(cfg.service_env.is_none());
        assert!(cfg.service_version.is_none());
        assert!(cfg.static_fields.is_empty());
        assert!(cfg.identity_fields().is_empty());
    }

    #[test]
    fn test_identity_fields_collects_configured_values() {
        let cfg = GlobalConfig {
            service_name: Some("orders".to_string()),
            service_instance: Some("orders-7f3a".to_string()),
            service_env: Some("prod".to_string()),
            service_version: Some("1.2.3".to_string()),
            static_fields: [("region".to_string(), "cn-north-1".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let identity = cfg.identity_fields();
        assert_eq!(identity.len(), 5);
        assert_eq!(identity["service_name"], "orders");
        assert_eq!(identity["service_instance"], "orders-7f3a");
        assert_eq!(identity["service_env"], "prod");
        assert_eq!(identity["service_version"], "1.2.3");
        assert_eq!(identity["region"], "cn-north-1");
    }

    #[test]
    fn test_identity_fields_partial_configuration() {
        // 只配 service_name：其余身份键不得出现
        let cfg = GlobalConfig {
            service_name: Some("orders".to_string()),
            ..Default::default()
        };
        let identity = cfg.identity_fields();
        assert_eq!(identity.len(), 1);
        assert!(identity.contains_key("service_name"));
        assert!(!identity.contains_key("service_instance"));
    }

    #[test]
    fn test_validate_rejects_blank_identity_values() {
        for value in ["", "   "] {
            let mut cfg = GlobalConfig {
                service_name: Some(value.to_string()),
                ..Default::default()
            };
            assert!(
                cfg.validate().is_err(),
                "blank service_name '{value}' must be rejected"
            );
        }
        let mut cfg = GlobalConfig {
            service_version: Some(String::new()),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_control_characters_in_identity() {
        for value in ["orders\nprod", "orders\tpayment", "orders\u{0}x"] {
            let mut cfg = GlobalConfig {
                service_name: Some(value.to_string()),
                ..Default::default()
            };
            assert!(
                cfg.validate().is_err(),
                "control characters in service_name '{value}' must be rejected"
            );
        }
    }

    #[test]
    fn test_validate_rejects_invalid_static_fields() {
        // 空 key
        let mut cfg = GlobalConfig::default();
        cfg.static_fields.insert(String::new(), "v".to_string());
        assert!(
            cfg.validate().is_err(),
            "empty static field key must be rejected"
        );

        // key / value 含控制字符
        let mut cfg = GlobalConfig::default();
        cfg.static_fields.insert("k\n".to_string(), "v".to_string());
        assert!(cfg.validate().is_err());
        let mut cfg = GlobalConfig::default();
        cfg.static_fields.insert("k".to_string(), "v\n".to_string());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_valid_identity() {
        let mut cfg = GlobalConfig {
            service_name: Some("orders".to_string()),
            service_instance: Some("orders-7f3a".to_string()),
            service_env: Some("prod".to_string()),
            service_version: Some("1.2.3".to_string()),
            static_fields: [("region".to_string(), "cn-north-1".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_identity_fields_skip_blank_values() {
        // 深度防御：绕过 validate 的链路（如宿主手工改字段）塞入空白值时，
        // 注入映射不得包含空白身份键
        let cfg = GlobalConfig {
            service_name: Some("orders".to_string()),
            service_instance: Some("   ".to_string()),
            service_env: Some(String::new()),
            static_fields: [("blank".to_string(), " ".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let identity = cfg.identity_fields();
        assert_eq!(identity.len(), 1, "blank values must be skipped");
        assert!(identity.contains_key("service_name"));
    }

    #[test]
    fn test_validate_identity_public_entry_rejects_and_accepts() {
        // 公开校验入口（供 InklogConfig::validate 与 DI 链复用，&self 只读）
        let mut cfg = GlobalConfig {
            service_name: Some("bad\nvalue".to_string()),
            ..Default::default()
        };
        assert!(cfg.validate_identity().is_err());
        cfg.service_name = Some("good".to_string());
        assert!(cfg.validate_identity().is_ok());
    }

    #[test]
    fn test_deserialize_identity_fields_from_toml() {
        let parsed: GlobalConfig = toml::from_str(
            "service_name = \"orders\"\n\
             service_instance = \"orders-7f3a\"\n\
             service_env = \"prod\"\n\
             service_version = \"1.2.3\"\n\
             [static_fields]\nregion = \"cn-north-1\"\n",
        )
        .unwrap();
        assert_eq!(parsed.service_name.as_deref(), Some("orders"));
        assert_eq!(parsed.service_instance.as_deref(), Some("orders-7f3a"));
        assert_eq!(parsed.service_env.as_deref(), Some("prod"));
        assert_eq!(parsed.service_version.as_deref(), Some("1.2.3"));
        assert_eq!(
            parsed.static_fields.get("region").map(String::as_str),
            Some("cn-north-1")
        );
    }

    #[test]
    fn test_deserialize_identity_fields_within_root_config() {
        // 宿主视角：身份字段位于 [global] 表下（InklogConfig 全文解析）
        let parsed: crate::InklogConfig = toml::from_str(
            "[global]\nservice_name = \"orders\"\n\
             [global.static_fields]\nregion = \"cn-north-1\"\n",
        )
        .unwrap();
        assert_eq!(parsed.global.service_name.as_deref(), Some("orders"));
        assert_eq!(
            parsed
                .global
                .static_fields
                .get("region")
                .map(String::as_str),
            Some("cn-north-1")
        );
    }

    #[test]
    fn test_deserialize_without_identity_fields_still_works() {
        // 既有配置文件（无身份字段）必须继续解析
        let parsed: GlobalConfig = toml::from_str("level = \"debug\"\n").unwrap();
        assert_eq!(parsed.level, "debug");
        assert!(parsed.service_name.is_none());
        assert!(parsed.static_fields.is_empty());
    }
}
