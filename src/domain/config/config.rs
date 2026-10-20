// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Root configuration struct and loading logic.

use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::console::ConsoleSinkConfig;
use super::database::DatabaseDriver;
use super::database::DatabaseSinkConfig;
use super::file_sink::FileSinkConfig;
use super::global::GlobalConfig;
use super::http::HttpServerConfig;
use super::performance::MAX_CHANNEL_CAPACITY;
use super::performance::PerformanceConfig;
use super::rate_limit::RateLimitConfig;
use super::sampling::SamplingConfig;

// InklogConfig - Root configuration struct

/// Root configuration for inklog logger.
///
/// # Loading
///
/// Configuration can be loaded from:
/// - TOML files (via `from_search_paths()`)
/// - Environment variables (prefix `INKLOG_`)
/// - Defaults (lowest priority)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InklogConfig {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default = "default_console_sink")]
    pub console_sink: Option<ConsoleSinkConfig>,
    #[serde(default)]
    pub file_sink: Option<FileSinkConfig>,
    #[serde(default)]
    pub database_sink: Option<DatabaseSinkConfig>,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub http_server: Option<HttpServerConfig>,
    /// Per-crate target level presets (e.g. `{"hyper" = "warn", "my_crate" = "debug"}`).
    ///
    /// These are merged into the `EnvFilter` alongside the global level and
    /// `RUST_LOG` overrides. Target-level entries have lower priority than
    /// `RUST_LOG` but higher than the global default.
    #[serde(default)]
    pub target_levels: HashMap<String, String>,
    /// 采样策略：限流压力下决定保留哪些记录（未配置 = 内置兜底采样）。
    #[serde(default)]
    pub sampling: SamplingConfig,
    /// 按 target 前缀分组的限流配额（未配置 = 订阅器不接线，全局行为不变）。
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

fn default_console_sink() -> Option<ConsoleSinkConfig> {
    Some(ConsoleSinkConfig::default())
}

/// INKLOG_* 环境变量覆盖前缀（搜索路径 `INKLOG_CONFIG_PATH` 与 STATIC_FIELDS
/// 专用钩子之外的覆盖变量全集）。
const ENV_PREFIX: &str = "INKLOG_";

/// env 覆盖值的取值规则：对应目标字段的语义类型；非法值 warn 后保留现值。
#[derive(Clone, Copy)]
enum EnvValueRule {
    /// 任意字符串（数值/布尔字面量按原文字符串落位）
    Str,
    /// 合法日志级别（`LogLevel::is_valid_level`）
    Level,
    /// 非空字符串（空值 warn 后保留现值）
    Format,
    /// 布尔（true/false；不可解析不覆盖）
    Bool,
    /// 只启用语义：仅可解析为 `true` 时生效（sink `ENABLED` 单向闸门）
    EnableOnly,
    /// u16 端口
    Port,
    /// usize
    Usize,
    /// u32
    U32,
    /// u64
    U64,
    /// 文件路径：拒绝 `ParentDir`/NUL（遍历安全）
    SafePath,
    /// HTTP 路径：解码后 `/` 开头且无遍历/反斜杠/控制字符
    SafeHttpPath,
    /// 体积字符串：去空白后以 ASCII 数字开头
    SizeStr,
    /// HTTP error_mode：`strict`/`warn`（大小写不敏感，落位规范化小写）
    ErrorMode,
    /// 数据库 URL：拒绝 `ParentDir`/NUL
    DbUrl,
    /// 服务身份字段：非空白且无控制字符
    Identity,
}

/// 一条 env 覆盖映射：环境变量名 → 点分配置路径 + 取值规则。
struct EnvOverrideSpec {
    env_name: &'static str,
    path: &'static str,
    rule: EnvValueRule,
    /// 旧实现先 `get_or_insert_with` 再赋值的路径：env 变量出现即实体化
    /// Option 父节点（值非法时也实体化，字段保持默认）——既有测试钉死的
    /// 加载语义，此处显式保留
    materialize: bool,
}

/// INKLOG_* 覆盖的单一映射表。
///
/// 取代原逐字段 `std::env::var("INKLOG_*")` 硬编码分支：读取为前缀过滤的
/// 通用遍历、映射与本表单源、定型经 confers `EnvSource::infer_config_value`
/// （确定性类型推断）。`INKLOG_GLOBAL_STATIC_FIELDS` 不列入（逗号 k=v 表由
/// 专用钩子承载）；`INKLOG_CONFIG_PATH` 是搜索路径而非覆盖项。未列入本表的
/// INKLOG_* 变量与既有行为一致——忽略。
static ENV_OVERRIDE_SPECS: &[EnvOverrideSpec] = &[
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_LEVEL",
        path: "global.level",
        rule: EnvValueRule::Level,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_FORMAT",
        path: "global.format",
        rule: EnvValueRule::Format,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_MASKING_ENABLED",
        path: "global.masking_enabled",
        rule: EnvValueRule::Bool,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_AUTO_FALLBACK",
        path: "global.auto_fallback",
        rule: EnvValueRule::Bool,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_SERVICE_NAME",
        path: "global.service_name",
        rule: EnvValueRule::Identity,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_SERVICE_INSTANCE",
        path: "global.service_instance",
        rule: EnvValueRule::Identity,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_SERVICE_ENV",
        path: "global.service_env",
        rule: EnvValueRule::Identity,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_GLOBAL_SERVICE_VERSION",
        path: "global.service_version",
        rule: EnvValueRule::Identity,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_FILE_SINK_ENABLED",
        path: "file_sink.enabled",
        rule: EnvValueRule::EnableOnly,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_FILE_SINK_PATH",
        path: "file_sink.path",
        rule: EnvValueRule::SafePath,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_FILE_SINK_MAX_SIZE",
        path: "file_sink.max_size",
        rule: EnvValueRule::SizeStr,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_ENABLED",
        path: "http_server.enabled",
        rule: EnvValueRule::EnableOnly,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_HOST",
        path: "http_server.host",
        rule: EnvValueRule::Str,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_PORT",
        path: "http_server.port",
        rule: EnvValueRule::Port,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_METRICS_PATH",
        path: "http_server.metrics_path",
        rule: EnvValueRule::SafeHttpPath,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_HEALTH_PATH",
        path: "http_server.health_path",
        rule: EnvValueRule::SafeHttpPath,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_HTTP_SERVER_ERROR_MODE",
        path: "http_server.error_mode",
        rule: EnvValueRule::ErrorMode,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_PERFORMANCE_WORKER_THREADS",
        path: "performance.worker_threads",
        rule: EnvValueRule::Usize,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_PERFORMANCE_CHANNEL_CAPACITY",
        path: "performance.channel_capacity",
        rule: EnvValueRule::Usize,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_DATABASE_SINK_URL",
        path: "database_sink.url",
        rule: EnvValueRule::DbUrl,
        materialize: false,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_DATABASE_SINK_POOL_SIZE",
        path: "database_sink.pool_size",
        rule: EnvValueRule::U32,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_DATABASE_SINK_BATCH_SIZE",
        path: "database_sink.batch_size",
        rule: EnvValueRule::Usize,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS",
        path: "database_sink.flush_interval_ms",
        rule: EnvValueRule::U64,
        materialize: true,
    },
    EnvOverrideSpec {
        env_name: "INKLOG_DATABASE_SINK_TABLE_NAME",
        path: "database_sink.table_name",
        rule: EnvValueRule::Str,
        materialize: true,
    },
];

/// 按规则解释单条 env 值：`Some(v)` = 可应用的补丁值，`None` = 已 warn、
/// 保留现值不应用。`current` 仅用于 warn 文案的现值参数。
fn interpret_env_value(
    spec: &EnvOverrideSpec,
    raw: &str,
    current: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    // 定型单源于 confers：确定性推断（bool → i64 → u64 → f64 → String）
    let inferred = confers::config::EnvSource::infer_config_value(raw);
    match spec.rule {
        EnvValueRule::Str => Some(serde_json::Value::String(raw.to_string())),
        EnvValueRule::Level => {
            if crate::LogLevel::is_valid_level(raw) {
                Some(serde_json::Value::String(raw.to_string()))
            } else {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("val", raw);
                args.set(
                    "current",
                    current.and_then(|v| v.as_str()).unwrap_or_default(),
                );
                tracing::warn!("{}", crate::i18n::tr_args("config-env_invalid_level", args));
                None
            }
        }
        EnvValueRule::Format => {
            if raw.is_empty() {
                tracing::warn!("{}", crate::i18n::tr("config-env_invalid_format"));
                None
            } else {
                Some(serde_json::Value::String(raw.to_string()))
            }
        }
        EnvValueRule::Bool => inferred.as_bool().map(serde_json::Value::Bool),
        EnvValueRule::EnableOnly => match inferred.as_bool() {
            Some(true) => Some(serde_json::Value::Bool(true)),
            _ => None,
        },
        EnvValueRule::Port => fit_uint::<u16>(&inferred).map(serde_json::Value::from),
        EnvValueRule::Usize => fit_uint::<usize>(&inferred).map(serde_json::Value::from),
        EnvValueRule::U32 => fit_uint::<u32>(&inferred).map(serde_json::Value::from),
        EnvValueRule::U64 => fit_uint::<u64>(&inferred).map(serde_json::Value::from),
        EnvValueRule::SafePath => unsafe_path_reason(raw).map_or_else(
            || Some(serde_json::Value::String(raw.to_string())),
            |()| {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("path", raw);
                tracing::warn!("{}", crate::i18n::tr_args("config-env_unsafe_path", args));
                None
            },
        ),
        EnvValueRule::SafeHttpPath => {
            if is_safe_http_path(raw) {
                Some(serde_json::Value::String(raw.to_string()))
            } else {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("path", raw);
                args.set("env_var", spec.env_name);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_unsafe_http_path", args)
                );
                None
            }
        }
        EnvValueRule::SizeStr => {
            let has_numeric = raw
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count()
                > 0;
            if has_numeric {
                Some(serde_json::Value::String(raw.to_string()))
            } else {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("val", raw);
                tracing::warn!("{}", crate::i18n::tr_args("config-env_invalid_size", args));
                None
            }
        }
        EnvValueRule::ErrorMode => {
            let lower = raw.to_ascii_lowercase();
            if lower == "strict" || lower == "warn" {
                Some(serde_json::Value::String(lower))
            } else {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("val", raw);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_unknown_error_mode", args)
                );
                None
            }
        }
        EnvValueRule::DbUrl => unsafe_path_reason(raw).map_or_else(
            || Some(serde_json::Value::String(raw.to_string())),
            |()| {
                tracing::warn!("{}", crate::i18n::tr("config-env_invalid_db_url"));
                None
            },
        ),
        EnvValueRule::Identity => {
            if raw.trim().is_empty() {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("var", spec.env_name);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_blank_value_ignored", args)
                );
                None
            } else if crate::domain::config::global::reject_control_chars(spec.env_name, raw)
                .is_err()
            {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("var", spec.env_name);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_control_char_ignored", args)
                );
                None
            } else {
                Some(serde_json::Value::String(raw.to_string()))
            }
        }
    }
}

/// 路径类 env 值的安全判定：含 `ParentDir` 组件或 NUL 字节时返回 `Some(())`。
fn unsafe_path_reason(raw: &str) -> Option<()> {
    let has_traversal = std::path::PathBuf::from(raw)
        .components()
        .any(|c| c == std::path::Component::ParentDir);
    (has_traversal || raw.contains('\0')).then_some(())
}

/// 推断值须为目标无符号整型可容纳的整数字面量（否则按「不可解析」保留现值——
/// 与旧实现 `parse().unwrap_or(current)` 的静默回退一致）。
fn fit_uint<T>(inferred: &confers::types::ConfigValue) -> Option<T>
where
    T: TryFrom<i64> + TryFrom<u64>,
{
    match inferred {
        confers::types::ConfigValue::I64(v) => T::try_from(*v).ok(),
        confers::types::ConfigValue::U64(v) => T::try_from(*v).ok(),
        _ => None,
    }
}

/// STATIC_FIELDS 专用钩子：逗号分隔 k=v 表逐段校验（空段跳过；键/值非空且
/// 无控制字符才入库，坏段 warn 后跳过该段），合法段合并进
/// `global.static_fields`（插入语义，与文件配置同键共存时 env 段追加）。
fn apply_static_fields_env(env_vars: &[(String, String)], json: &mut serde_json::Value) {
    let Some((_, raw)) = env_vars
        .iter()
        .find(|(name, _)| name == "INKLOG_GLOBAL_STATIC_FIELDS")
    else {
        return;
    };
    for segment in raw.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        match segment.split_once('=') {
            Some((key, value))
                if !key.trim().is_empty()
                    && !value.is_empty()
                    && crate::domain::config::global::reject_control_chars(
                        "static field",
                        value,
                    )
                    .is_ok()
                    && crate::domain::config::global::reject_control_chars(
                        "static field key",
                        key,
                    )
                    .is_ok() =>
            {
                json_path_set(
                    json,
                    "global.static_fields",
                    serde_json::json!({ key.trim(): value }),
                );
            }
            _ => {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("segment", segment);
                args.set("raw", raw);
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("config-env_invalid_static_field", args)
                );
            }
        }
    }
}

/// 在 JSON 树上按点分路径读取现值（用于 warn 文案参数）。
fn json_path_get<'a>(json: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = json;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// 按点分路径设置补丁值；叶值为对象且目标已存在对象时逐键合并
/// （static_fields 的插入语义，不整表替换）。
fn json_path_set(json: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let segments: Vec<&str> = path.split('.').collect();
    let (leaf, parents) = segments.split_last().unwrap_or_else(|| {
        panic!(
            "{}",
            crate::i18n::tr("config-json_path_has_segment_invariant")
        )
    });
    let mut current = json;
    for segment in parents {
        let map = current.as_object_mut().unwrap_or_else(|| {
            panic!(
                "{}",
                crate::i18n::tr("config-json_intermediate_node_object_invariant")
            )
        });
        let entry = map
            .entry(*segment)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        // Option 字段序列化为 null：补丁进入前先实体化为对象（各字段由
        // serde default 补齐，等价旧实现 get_or_insert_with(Default::default)）
        if entry.is_null() {
            *entry = serde_json::Value::Object(serde_json::Map::new());
        }
        current = entry;
    }
    let map = current.as_object_mut().unwrap_or_else(|| {
        panic!(
            "{}",
            crate::i18n::tr("config-json_leaf_parent_object_invariant")
        )
    });
    let both_objects = matches!(
        (map.get(*leaf), &value),
        (
            Some(serde_json::Value::Object(_)),
            serde_json::Value::Object(_)
        )
    );
    if both_objects {
        if let (Some(serde_json::Value::Object(existing)), serde_json::Value::Object(new)) =
            (map.get_mut(*leaf), value)
        {
            for (k, v) in new {
                existing.insert(k, v);
            }
        }
    } else {
        map.insert((*leaf).to_string(), value);
    }
}

/// Decode `%XX` percent-encoding so path validation cannot be bypassed with
/// encoded traversal, backslashes, or control characters. Invalid escape
/// sequences are kept as-is.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(high) = (bytes[i + 1] as char).to_digit(16)
            && let Some(low) = (bytes[i + 2] as char).to_digit(16)
        {
            decoded.push(((high * 16) + low) as u8);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// An HTTP path override is safe when the percent-decoded value starts with
/// `/` and contains no traversal, backslashes, or control characters.
fn is_safe_http_path(value: &str) -> bool {
    let decoded = percent_decode(value);
    decoded.starts_with('/')
        && !decoded.contains("..")
        && !decoded.contains('\\')
        && !decoded.chars().any(char::is_control)
}

/// Validate the `前缀.*` wildcard form of an `ip_whitelist` entry (e.g. `10.*`,
/// `192.168.*`): after stripping the `.*` suffix the remainder must be 1-3
/// dot-separated decimal segments (each 0-255) and must not contain `/`.
///
/// This keeps config-time validation consistent with the runtime
/// `whitelist_entry_matches` in http_server, which explicitly supports the
/// `<prefix>.*` form (with a trailing-dot guard against overreaching prefix
/// matches). Previously only `IpAddr`/`IpNet` were accepted, so `10.*` was
/// rejected at load time despite being valid at runtime.
fn is_valid_prefix_wildcard_entry(entry: &str) -> bool {
    let Some(prefix) = entry.strip_suffix(".*") else {
        return false;
    };
    if prefix.contains('/') {
        return false;
    }
    let mut segments = 0;
    for segment in prefix.split('.') {
        if segment.is_empty()
            || !segment.bytes().all(|b| b.is_ascii_digit())
            || segment.parse::<u8>().is_err()
        {
            return false;
        }
        segments += 1;
        if segments > 3 {
            return false;
        }
    }
    // split(".") 至少产出一段；空 prefix（"::.*"、".*"）已被空段检查拒绝
    true
}

impl Default for InklogConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            console_sink: default_console_sink(),
            file_sink: None,
            database_sink: None,
            performance: PerformanceConfig::default(),
            http_server: None,
            target_levels: HashMap::new(),
            sampling: SamplingConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl InklogConfig {
    /// Load configuration synchronously from the default search paths.
    pub fn load_sync() -> Result<Self, InklogError> {
        Self::from_search_paths().map_err(|e| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-load_failed", args))
        })
    }

    /// Load configuration with custom environment variable overrides.
    pub fn load_with_env_overrides() -> Result<Self, InklogError> {
        let mut config = Self::load_sync()?;
        Self::apply_env_overrides(&mut config);
        Ok(config)
    }

    /// Apply INKLOG_* environment variable overrides to the configuration.
    ///
    /// 读取→映射→定型三段均为数据驱动：前缀过滤的通用环境变量遍历、
    /// [`ENV_OVERRIDE_SPECS`] 单一映射表（env 名 → 点分配置路径 + 取值
    /// 规则）、confers `EnvSource::infer_config_value` 确定性定型。逐字段
    /// 语义（级别校验、路径安全、ENABLED 单向闸门、服务身份约束）由规则
    /// 解释器执行：非法值 warn 后保留现值，加载不失败（既有语义）。
    /// 未列入映射表的 INKLOG_* 变量与既有行为一致——忽略。
    fn apply_env_overrides(config: &mut Self) {
        let env_vars: Vec<(String, String)> = std::env::vars()
            .filter(|(name, _)| name.starts_with(ENV_PREFIX))
            .collect();

        // JSON 视图承载补丁：规则解释读取现值（warn 文案参数）与补丁落位
        // 共用同一棵树；序列化途经自有的 Serialize derive，不引入外部数据。
        let mut json = serde_json::to_value(config.clone())
            .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("config-serialize_invariant")));

        for spec in ENV_OVERRIDE_SPECS {
            let Some((_, raw)) = env_vars.iter().find(|(name, _)| *name == spec.env_name) else {
                continue;
            };
            let current = json_path_get(&json, spec.path);
            match interpret_env_value(spec, raw, current) {
                Some(value) => json_path_set(&mut json, spec.path, value),
                // 值非法且该路径旧语义为「变量出现即实体化」：补一个空对象把
                // Option 父节点落地，字段保持默认（如 ERROR_MODE 未知值场景）
                None if spec.materialize => {
                    if let Some((parent, _)) = spec.path.rsplit_once('.') {
                        json_path_set(
                            &mut json,
                            parent,
                            serde_json::Value::Object(serde_json::Map::new()),
                        );
                    }
                }
                None => {}
            }
        }

        // STATIC_FIELDS 逗号分隔 k=v 表：通用映射无法表达，保留专用 env
        // 钩子（分段校验复用 reject_control_chars，坏段 warn 后跳过）
        apply_static_fields_env(&env_vars, &mut json);

        *config = serde_json::from_value(json)
            .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("config-env_patch_types_invariant")));
    }

    /// Search paths (first existing file wins):
    /// 1. `$INKLOG_CONFIG_PATH`
    /// 2. `inklog_config.toml` (current directory)
    /// 3. `~/.config/inklog/config.toml` (user config dir)
    /// 4. Platform-specific system config:
    ///    - Unix/Linux: `/etc/inklog/config.toml`
    ///    - Windows: `%ProgramData%\inklog\config.toml`
    pub fn from_search_paths() -> Result<Self, InklogError> {
        let mut search_paths: Vec<Option<String>> = vec![
            std::env::var("INKLOG_CONFIG_PATH").ok(),
            Some("inklog_config.toml".to_string()),
            dirs::config_dir().map(|p| {
                p.join("inklog")
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string()
            }),
        ];
        search_paths.push(Self::system_config_path());

        // Maximum config file size (1 MB) to prevent memory exhaustion
        const MAX_CONFIG_SIZE: u64 = 1024 * 1024;

        for path_opt in search_paths.into_iter().flatten() {
            if std::path::Path::new(&path_opt).exists() {
                // Guard against oversized config files (potential DoS)
                if let Ok(meta) = std::fs::metadata(&path_opt)
                    && meta.len() > MAX_CONFIG_SIZE
                {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("size", meta.len().to_string());
                    args.set("max", MAX_CONFIG_SIZE.to_string());
                    return Err(InklogError::ConfigError(crate::i18n::tr_args(
                        "config-exceeds_max_size",
                        args,
                    )));
                }
                let content = std::fs::read_to_string(&path_opt).map_err(|e| {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("err", e.to_string());
                    InklogError::ConfigError(crate::i18n::tr_args("config-read_failed", args))
                })?;
                let mut config: Self = toml::from_str(&content).map_err(|e| {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("path", path_opt.clone());
                    args.set("err", e.to_string());
                    InklogError::ConfigError(crate::i18n::tr_args("config-parse_failed", args))
                })?;
                // 配置加载完成的构造出口：先规范化可修正项，再执行硬校验
                config.normalize();
                config.validate()?;
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    /// Returns the platform-specific system configuration path.
    ///
    /// - Unix/Linux: `/etc/inklog/config.toml`
    /// - Windows: `%ProgramData%\inklog\config.toml`
    fn system_config_path() -> Option<String> {
        #[cfg(unix)]
        {
            Some("/etc/inklog/config.toml".to_string())
        }
        #[cfg(windows)]
        {
            std::env::var("ProgramData")
                .ok()
                .map(|base| format!("{}\\inklog\\config.toml", base))
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    /// Returns a list of enabled sink names.
    pub fn sinks_enabled(&self) -> Vec<&'static str> {
        let mut sinks = Vec::new();
        if self.console_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("console");
        }
        if self.file_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("file");
        }
        if self.database_sink.as_ref().is_some_and(|c| c.enabled) {
            sinks.push("database");
        }
        sinks
    }

    /// Normalize configuration values by auto-correcting invalid settings.
    ///
    /// Unlike `validate()` which returns errors, this method adjusts values
    /// in-place with warnings. Call before `validate()` to auto-fix common
    /// misconfigurations. Problems that cannot be auto-corrected are logged
    /// here and surface as errors from `validate()`.
    pub fn normalize(&mut self) {
        fn warn_validation(err: String) {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", err);
            tracing::warn!(
                "{}",
                crate::i18n::tr_args("log-config_validation_failed", args)
            );
        }
        if let Err(e) = self.global.validate() {
            warn_validation(e);
        }
        if let Err(e) = self.performance.validate() {
            warn_validation(e);
        }
        if let Some(ref mut db) = self.database_sink
            && let Err(e) = db.validate()
        {
            warn_validation(e);
        }
        if let Some(ref mut console) = self.console_sink {
            console.validate();
        }
    }

    /// Validate the configuration.
    ///
    /// Checks for common misconfigurations and returns a descriptive error
    /// for the first problem found.
    pub fn validate(&self) -> Result<(), InklogError> {
        // Performance
        if self.performance.channel_capacity == 0 {
            return Err(InklogError::ConfigError(crate::i18n::tr(
                "config-channel_capacity_zero",
            )));
        }
        if self.performance.channel_capacity > MAX_CHANNEL_CAPACITY {
            return Err(InklogError::ConfigError(format!(
                "channel_capacity {} exceeds maximum {MAX_CHANNEL_CAPACITY}",
                self.performance.channel_capacity
            )));
        }
        if self.performance.worker_threads == 0 {
            return Err(InklogError::ConfigError(crate::i18n::tr(
                "config-worker_threads_zero",
            )));
        }
        if self.performance.max_capacity < self.performance.min_capacity {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("max", self.performance.max_capacity.to_string());
            args.set("min", self.performance.min_capacity.to_string());
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-max_capacity_lt_min",
                args,
            )));
        }

        // Global identity fields
        // 身份字段是本加载链的硬校验出口（normalize 只 warn）：空白值/控制
        // 字符在此以 Err 拒绝加载
        self.global
            .validate_identity()
            .map_err(InklogError::ConfigError)?;

        // Global log level
        if !crate::LogLevel::is_valid_level(&self.global.level) {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("level", self.global.level.clone());
            args.set("valid", crate::LogLevel::VALID_LEVEL_STRINGS.join(", "));
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-invalid_log_level",
                args,
            )));
        }

        // File sink
        if let Some(ref file) = self.file_sink
            && file.enabled
        {
            if file.path.as_os_str().is_empty() {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-file_path_empty",
                )));
            }
            if file.batch_size == 0 {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-file_batch_size_zero",
                )));
            }
            file.validate().map_err(InklogError::ConfigError)?;
        }

        // Database sink
        if let Some(ref db) = self.database_sink
            && db.enabled
            && db.driver != DatabaseDriver::SQLite
            && db.pool_size == 0
        {
            return Err(InklogError::ConfigError(format!(
                "database_sink.pool_size must be at least 1 for the {} driver, got 0",
                db.driver
            )));
        }

        // HTTP server
        if let Some(ref http) = self.http_server
            && http.enabled
        {
            if http.port == 0 {
                return Err(InklogError::ConfigError(crate::i18n::tr(
                    "config-http_port_zero",
                )));
            }
            // ip_whitelist 条目格式校验：解析失败的条目在运行期会被静默跳过
            // （fail-closed），导致预期放行的 IP 被拒且无任何提示，故在配置期
            // 直接拒绝坏条目。合法形式三选一：精确 IP、CIDR、`前缀.*` 通配
            // （运行期 whitelist_entry_matches 支持的同款语法）。
            if let Some(ref whitelist) = http.ip_whitelist {
                for (idx, entry) in whitelist.iter().enumerate() {
                    let parseable = entry.parse::<std::net::IpAddr>().is_ok()
                        || entry.parse::<ipnet::IpNet>().is_ok()
                        || is_valid_prefix_wildcard_entry(entry);
                    if !parseable {
                        return Err(InklogError::ConfigError(format!(
                            "http_server.ip_whitelist[{idx}] is not a valid IP, CIDR, or prefix wildcard (e.g. \"10.*\"): {entry:?}"
                        )));
                    }
                }
            }
            if let Some(ref tls) = http.tls {
                tls.validate().map_err(InklogError::ConfigError)?;
            }
        }

        // Console sink stderr_levels
        if let Some(ref console) = self.console_sink
            && let Some(invalid) = console.invalid_stderr_levels().first()
        {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("level", *invalid);
            args.set("valid", crate::LogLevel::VALID_LEVEL_STRINGS.join(", "));
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-invalid_stderr_level",
                args,
            )));
        }

        // Sampling policy
        self.sampling.validate()?;

        // Per-target rate limit quota groups
        self.rate_limit.validate()?;

        Ok(())
    }
}

#[cfg(feature = "schema")]
impl InklogConfig {
    /// 导出配置的 JSON Schema（`schema` feature；与 `inklog-cli generate
    /// --schema` 及入库产物 `config_schema.json` 同源）。
    ///
    /// 字段序为 serde_json 的确定性排序，输出适合入库做漂移校验。
    ///
    /// # Panics
    ///
    /// 仅当 `serde_json` 序列化 `schemars::Schema` 失败时 panic。载荷是
    /// 纯数据（无非法 map 键、无非字符串数值边界），该路径不可达——
    /// expect 表达的是序列化不变量，不是可恢复的错误分支。
    #[must_use]
    pub fn json_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(InklogConfig)).unwrap_or_else(|_| {
            panic!(
                "{}",
                crate::i18n::tr("config-json_schema_serialize_invariant")
            )
        })
    }
}

impl std::str::FromStr for InklogConfig {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut config: Self = toml::from_str(s)?;
        config.normalize();
        config.validate().map_err(|e| {
            use serde::de::Error as _;
            toml::de::Error::custom(e.to_string())
        })?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::http::HttpErrorMode;

    // Mutex to serialize env var tests (env vars are process-global state)
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_validate_default_config() {
        let config = InklogConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let mut config = InklogConfig::default();
        config.global.level = "verbose".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("Invalid log level"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_validate_valid_log_levels() {
        for level in &[
            "trace", "debug", "info", "warn", "error", "fatal", "INFO", "Debug",
        ] {
            let mut config = InklogConfig::default();
            config.global.level = level.to_string();
            assert!(
                config.validate().is_ok(),
                "level '{}' should be valid",
                level
            );
        }
    }

    #[test]
    fn test_validate_zero_channel_capacity() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = 0;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("channel_capacity"));
    }

    #[test]
    fn test_validate_zero_worker_threads() {
        let mut config = InklogConfig::default();
        config.performance.worker_threads = 0;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("worker_threads"));
    }

    #[test]
    fn test_validate_max_capacity_less_than_min() {
        let mut config = InklogConfig::default();
        config.performance.max_capacity = 100;
        config.performance.min_capacity = 500;
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("max_capacity"));
    }

    #[test]
    fn test_validate_file_sink_empty_path() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::new(),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("file_sink.path"));
    }

    #[test]
    fn test_validate_file_sink_zero_batch_size() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            batch_size: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn test_validate_http_port_zero() {
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            port: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn test_validate_http_disabled_port_zero_ok() {
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: false,
            port: 0,
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_system_config_path_returns_platform_path() {
        let path = InklogConfig::system_config_path();
        assert!(
            path.is_some(),
            "system_config_path should return Some on unix"
        );
        let path = path.unwrap();
        #[cfg(unix)]
        assert_eq!(path, "/etc/inklog/config.toml");
        #[cfg(windows)]
        assert!(path.contains("inklog") && path.contains("config.toml"));
    }

    #[test]
    fn test_from_search_paths_falls_back_to_default() {
        // 与会临时设置 INKLOG_CONFIG_PATH 的测试互斥，避免并发读全局环境变量竞态
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
        // When no config files exist, should return default config
        let config = InklogConfig::from_search_paths().unwrap();
        assert_eq!(config.global.level, "info");
    }

    #[test]
    fn test_env_override_invalid_level_falls_back() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // invalid log level should not be applied
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "invalid_level");
        }
        let mut config = InklogConfig::default();
        let original_level = config.global.level.clone();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(
            config.global.level, original_level,
            "invalid level should not override current value"
        );
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_env_override_identity_fields() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_SERVICE_NAME", "orders");
            std::env::set_var("INKLOG_GLOBAL_SERVICE_INSTANCE", "orders-7f3a");
            std::env::set_var("INKLOG_GLOBAL_SERVICE_ENV", "prod");
            std::env::set_var("INKLOG_GLOBAL_SERVICE_VERSION", "1.2.3");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.service_name.as_deref(), Some("orders"));
        assert_eq!(
            config.global.service_instance.as_deref(),
            Some("orders-7f3a")
        );
        assert_eq!(config.global.service_env.as_deref(), Some("prod"));
        assert_eq!(config.global.service_version.as_deref(), Some("1.2.3"));
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_NAME");
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_INSTANCE");
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_ENV");
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_VERSION");
        }
    }

    #[test]
    fn test_env_override_blank_identity_value_ignored() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // 空值不得覆盖为 Some("")：应保持 None 并告警忽略
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_SERVICE_NAME", "   ");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(
            config.global.service_name.is_none(),
            "blank identity env value must not become Some(...)"
        );
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_NAME");
        }
    }

    #[test]
    fn test_env_override_static_fields_parses_key_value_pairs() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_STATIC_FIELDS", "region=cn-north-1,az=a");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(
            config
                .global
                .static_fields
                .get("region")
                .map(String::as_str),
            Some("cn-north-1")
        );
        assert_eq!(
            config.global.static_fields.get("az").map(String::as_str),
            Some("a")
        );
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_STATIC_FIELDS");
        }
    }

    #[test]
    fn test_env_override_static_fields_skips_invalid_segments() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // 无 '=' 的段与空段跳过；合法段仍生效
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_STATIC_FIELDS", "good=1,,bad-segment");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.static_fields.len(), 1);
        assert!(config.global.static_fields.contains_key("good"));
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_STATIC_FIELDS");
        }
    }

    #[test]
    fn test_env_override_control_char_identity_ignored() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // env 值含控制字符：在覆盖点告警忽略（保持 None），不进入配置
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_SERVICE_NAME", "orders\nfake");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(
            config.global.service_name.is_none(),
            "control-character env value must be ignored"
        );
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_SERVICE_NAME");
        }
    }

    #[test]
    fn test_env_override_static_fields_control_char_segment_skipped() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_STATIC_FIELDS", "good=1,bad=line1\nline2");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(
            !config.global.static_fields.contains_key("bad"),
            "control-character static field must be skipped"
        );
        assert!(config.global.static_fields.contains_key("good"));
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_STATIC_FIELDS");
        }
    }

    #[test]
    fn test_validate_hard_rejects_invalid_identity_fields() {
        // InklogConfig::validate 是加载链的硬校验出口：身份字段非法必须 Err
        let mut config = InklogConfig::default();
        config.global.service_name = Some("bad\nvalue".to_string());
        assert!(config.validate().is_err());

        let mut config = InklogConfig::default();
        config.global.service_env = Some("   ".to_string());
        assert!(config.validate().is_err());

        // 默认配置（无身份）仍通过
        assert!(InklogConfig::default().validate().is_ok());
    }

    #[test]
    fn test_sinks_enabled_default() {
        let config = InklogConfig::default();
        let sinks = config.sinks_enabled();
        // default has console enabled
        assert!(sinks.contains(&"console"));
    }

    #[test]
    fn test_sinks_enabled_with_file() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("test.log"),
            ..Default::default()
        });
        let sinks = config.sinks_enabled();
        assert!(sinks.contains(&"console"));
        assert!(sinks.contains(&"file"));
    }

    #[test]
    fn test_sinks_enabled_none() {
        let mut config = InklogConfig::default();
        config.console_sink = None;
        config.file_sink = None;
        config.database_sink = None;
        let sinks = config.sinks_enabled();
        assert!(sinks.is_empty());
    }

    #[test]
    fn test_normalize_fixes_invalid_settings() {
        let mut config = InklogConfig::default();
        config.global.fallback_max_retries = 0;
        config.performance.expand_threshold_percent = 200;
        config.normalize();
        // global.validate should reset retries
        assert_eq!(config.global.fallback_max_retries, 1);
        // performance.validate should clamp
        assert_eq!(config.performance.expand_threshold_percent, 100);
    }

    #[test]
    fn test_from_str_parses_toml() {
        let toml_str = r#"
[global]
level = "debug"
"#;
        let config: InklogConfig = toml_str.parse().expect("should parse TOML");
        assert_eq!(config.global.level, "debug");
    }

    #[test]
    fn test_apply_env_overrides_valid_level() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "warn");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.level, "warn");
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_apply_env_overrides_format() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_FORMAT", "{message}");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.format, "{message}");
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_FORMAT");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/test.log");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("/tmp/test.log")
        );
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_path_traversal_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "../etc/passwd");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // path with parent dir traversal should be ignored, file_sink should remain None
        assert!(config.file_sink.is_none());
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_server() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "true");
            std::env::set_var("INKLOG_HTTP_SERVER_HOST", "0.0.0.0");
            std::env::set_var("INKLOG_HTTP_SERVER_PORT", "8080");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        let http = config.http_server.as_ref().unwrap();
        assert!(http.enabled);
        assert_eq!(http.host, "0.0.0.0");
        assert_eq!(http.port, 8080);
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
            std::env::remove_var("INKLOG_HTTP_SERVER_HOST");
            std::env::remove_var("INKLOG_HTTP_SERVER_PORT");
        }
    }

    #[test]
    fn test_apply_env_overrides_performance() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "8");
            std::env::set_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "2048");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.performance.worker_threads, 8);
        assert_eq!(config.performance.channel_capacity, 2048);
        unsafe {
            std::env::remove_var("INKLOG_PERFORMANCE_WORKER_THREADS");
            std::env::remove_var("INKLOG_PERFORMANCE_CHANNEL_CAPACITY");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_strict() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "strict");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Strict
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_warn() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "warn");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Warn
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_masking_and_fallback() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_MASKING_ENABLED", "false");
            std::env::set_var("INKLOG_GLOBAL_AUTO_FALLBACK", "false");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(!config.global.masking_enabled);
        assert!(!config.global.auto_fallback);
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_MASKING_ENABLED");
            std::env::remove_var("INKLOG_GLOBAL_AUTO_FALLBACK");
        }
    }

    #[test]
    fn test_env_override_mapping_table_single_source() {
        // 映射表是 env→路径→规则的唯一来源：每条目带前缀、点分路径、非空规则
        assert_eq!(ENV_OVERRIDE_SPECS.len(), 24);
        for spec in ENV_OVERRIDE_SPECS {
            assert!(
                spec.env_name.starts_with(ENV_PREFIX),
                "env name must carry the INKLOG_ prefix: {}",
                spec.env_name
            );
            assert!(
                spec.path.contains('.') && !spec.path.starts_with('.'),
                "path must be a dotted config path: {}",
                spec.path
            );
            if spec.materialize {
                assert!(
                    spec.path
                        .rsplit_once('.')
                        .is_some_and(|(parent, _)| !parent.contains('.')),
                    "materialize specs patch a depth-2 parent: {}",
                    spec.path
                );
            }
        }
    }

    #[test]
    fn test_env_override_mapping_table_nested_paths() {
        // 规格映射表：每字段一条 INKLOG_* → 值断言（嵌套路径经点分路径落位）
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "debug");
            std::env::set_var("INKLOG_GLOBAL_FORMAT", "json");
            std::env::set_var("INKLOG_FILE_SINK_PATH", "/var/log/x.log");
            std::env::set_var("INKLOG_HTTP_SERVER_PORT", "9999");
            std::env::set_var("INKLOG_DATABASE_SINK_URL", "sqlite:///tmp/audit.db");
            std::env::set_var("INKLOG_PERFORMANCE_WORKER_THREADS", "4");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert_eq!(config.global.level, "debug");
        assert_eq!(config.global.format, "json");
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("/var/log/x.log")
        );
        assert_eq!(config.http_server.as_ref().unwrap().port, 9999);
        assert_eq!(
            config.database_sink.as_ref().unwrap().url,
            "sqlite:///tmp/audit.db"
        );
        assert_eq!(config.performance.worker_threads, 4);
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
            std::env::remove_var("INKLOG_GLOBAL_FORMAT");
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
            std::env::remove_var("INKLOG_HTTP_SERVER_PORT");
            std::env::remove_var("INKLOG_DATABASE_SINK_URL");
            std::env::remove_var("INKLOG_PERFORMANCE_WORKER_THREADS");
        }
    }

    #[test]
    fn test_env_override_enabled_invalid_does_not_enable() {
        // ENABLED 单向语义：不可解析为 true 的值（如 "banana"）不启用组件
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_ENABLED", "banana");
            std::env::set_var("INKLOG_HTTP_SERVER_ENABLED", "banana");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(
            config.file_sink.is_none(),
            "invalid ENABLED value must not enable the file sink"
        );
        assert!(
            config.http_server.is_none(),
            "invalid ENABLED value must not enable the http server"
        );
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_ENABLED");
            std::env::remove_var("INKLOG_HTTP_SERVER_ENABLED");
        }
    }

    #[test]
    fn test_load_sync_returns_config() {
        // Must hold ENV_MUTEX to ensure INKLOG_CONFIG_PATH is not set by another test
        let _lock = ENV_MUTEX.lock().unwrap();
        // load_sync calls from_search_paths which falls back to default
        let config = InklogConfig::load_sync().unwrap();
        assert_eq!(config.global.level, "info");
    }

    #[test]
    fn test_load_with_env_overrides_applies() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_LEVEL", "debug");
        }
        let config = InklogConfig::load_with_env_overrides().unwrap();
        assert_eq!(config.global.level, "debug");
        // Cleanup is guaranteed by _lock Drop (mutex release)
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_LEVEL");
        }
    }

    #[test]
    fn test_from_search_paths_rejects_oversized_config() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("oversized.toml");
        // Create a file > 1MB
        let oversized = vec![b'#'; 1024 * 1024 + 1];
        std::fs::write(&config_path, oversized).unwrap();

        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds maximum size")
        );
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
    }

    #[test]
    fn test_sinks_enabled_with_database() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            ..Default::default()
        });
        let sinks = config.sinks_enabled();
        assert!(sinks.contains(&"console"));
        assert!(sinks.contains(&"database"));
    }

    #[test]
    fn test_normalize_with_database_sink() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            batch_size: 0,
            ..Default::default()
        });
        // normalize should call db.validate without panic
        config.normalize();
    }

    #[test]
    fn test_apply_env_overrides_file_sink_enabled() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_ENABLED", "true");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert!(config.file_sink.as_ref().unwrap().enabled);
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_ENABLED");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_max_size_valid() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "100MB");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.file_sink.is_some());
        assert_eq!(config.file_sink.as_ref().unwrap().max_size, "100MB");
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_file_sink_max_size_invalid() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "INVALID");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // Invalid size should be ignored, file_sink should remain None
        assert!(config.file_sink.is_none());
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_metrics_and_health_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/custom/metrics");
            std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/custom/health");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.http_server.is_some());
        let http = config.http_server.as_ref().unwrap();
        assert_eq!(http.metrics_path, "/custom/metrics");
        assert_eq!(http.health_path, "/custom/health");
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
            std::env::remove_var("INKLOG_HTTP_SERVER_HEALTH_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_error_mode_unknown_keeps_current() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_ERROR_MODE", "unknown_mode");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // Unknown mode should keep current value (default is Strict)
        assert!(matches!(
            config.http_server.as_ref().unwrap().error_mode,
            HttpErrorMode::Strict
        ));
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_ERROR_MODE");
        }
    }

    #[test]
    fn test_apply_env_overrides_database_sink_url() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_DATABASE_SINK_URL", "postgres://localhost/testdb");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.database_sink.is_some());
        assert_eq!(
            config.database_sink.as_ref().unwrap().url,
            "postgres://localhost/testdb"
        );
        unsafe {
            std::env::remove_var("INKLOG_DATABASE_SINK_URL");
        }
    }

    #[test]
    fn test_apply_env_overrides_database_sink_pool_size() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_DATABASE_SINK_POOL_SIZE", "20");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        assert!(config.database_sink.is_some());
        assert_eq!(config.database_sink.as_ref().unwrap().pool_size, 20);
        unsafe {
            std::env::remove_var("INKLOG_DATABASE_SINK_POOL_SIZE");
        }
    }

    #[test]
    fn test_apply_env_overrides_global_format_empty() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_GLOBAL_FORMAT", "");
        }
        let mut config = InklogConfig::default();
        let original_format = config.global.format.clone();
        InklogConfig::apply_env_overrides(&mut config);
        // Empty format should be ignored, keeping original value
        assert_eq!(config.global.format, original_format);
        unsafe {
            std::env::remove_var("INKLOG_GLOBAL_FORMAT");
        }
    }

    #[test]
    fn test_validate_invalid_stderr_levels() {
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            stderr_levels: vec!["error".into(), "bogus_level".into()],
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("bogus_level"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_file_sink_retained_with_invalid_max_size() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_FILE_SINK_PATH", "/tmp/test.log");
            std::env::set_var("INKLOG_FILE_SINK_MAX_SIZE", "INVALID");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // file_sink should be created with the path, even though max_size is invalid
        assert!(config.file_sink.is_some());
        assert_eq!(
            config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("/tmp/test.log")
        );
        // max_size should remain default ("100MB") since INVALID was rejected
        assert_eq!(config.file_sink.as_ref().unwrap().max_size, "100MB");
        unsafe {
            std::env::remove_var("INKLOG_FILE_SINK_PATH");
            std::env::remove_var("INKLOG_FILE_SINK_MAX_SIZE");
        }
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("/%2E%2E/etc"), "/../etc");
        assert_eq!(percent_decode("/a%5Cb"), "/a\\b");
        assert_eq!(percent_decode("/a%0Ab"), "/a\nb");
        // 非法转义序列按原样保留
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%2"), "%2");
        assert_eq!(percent_decode("%2z"), "%2z");
        assert_eq!(percent_decode("/plain/path"), "/plain/path");
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_traversal_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/%2E%2E/etc/passwd");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后为 "/../etc/passwd"，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_backslash_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_HEALTH_PATH", "/a%5Cb");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后含 '\'，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_HEALTH_PATH");
        }
    }

    #[test]
    fn test_apply_env_overrides_http_path_encoded_control_char_rejected() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_HTTP_SERVER_METRICS_PATH", "/metrics%0A");
        }
        let mut config = InklogConfig::default();
        InklogConfig::apply_env_overrides(&mut config);
        // 解码后含控制字符，必须被拒绝
        assert!(config.http_server.is_none());
        unsafe {
            std::env::remove_var("INKLOG_HTTP_SERVER_METRICS_PATH");
        }
    }

    #[test]
    fn test_validate_rejects_oversized_channel_capacity() {
        let mut config = InklogConfig::default();
        config.performance.channel_capacity = MAX_CHANNEL_CAPACITY + 1;
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("channel_capacity"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_zero_pool_size_non_sqlite_db() {
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::PostgreSQL,
            pool_size: 0,
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("pool_size"),
            "unexpected error: {err}"
        );

        // SQLite 不受此守卫约束（validate 会强制为 1）
        let mut config = InklogConfig::default();
        config.database_sink = Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            pool_size: 0,
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_file_sink_invalid_max_size_rejected() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            max_size: "INVALID".to_string(),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("max_size"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_file_sink_encrypt_requires_existing_env() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: std::path::PathBuf::from("logs/test.log"),
            encrypt: true,
            encryption_key_env: Some("INKLOG_TEST_CONFIG_ENCRYPT_KEY".to_string()),
            ..Default::default()
        });

        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY") };
        assert!(config.validate().is_err());

        // SAFETY: test-only env var mutation
        unsafe { std::env::set_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY", "test-key") };
        assert!(config.validate().is_ok());

        // SAFETY: test-only env var mutation
        unsafe { std::env::remove_var("INKLOG_TEST_CONFIG_ENCRYPT_KEY") };
    }

    #[test]
    fn test_validate_http_tls_paths() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "-----BEGIN CERTIFICATE-----").unwrap();
        std::fs::write(&key, "-----BEGIN PRIVATE KEY-----").unwrap();

        // 证书/密钥文件缺失时拒绝
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            tls: Some(crate::config::http::TlsConfig {
                cert_path: "/nonexistent/cert.pem".to_string(),
                key_path: key.to_string_lossy().to_string(),
            }),
            ..Default::default()
        });
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("cert_path"),
            "unexpected error: {err}"
        );

        // 文件齐备时通过
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            tls: Some(crate::config::http::TlsConfig {
                cert_path: cert.to_string_lossy().to_string(),
                key_path: key.to_string_lossy().to_string(),
            }),
            ..Default::default()
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_http_ip_whitelist_entries() {
        // 合法 IP / CIDR / `前缀.*` 通配条目通过
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            ip_whitelist: Some(vec![
                "10.0.0.1".to_string(),
                "192.168.1.0/24".to_string(),
                "2001:db8::1".to_string(),
                "10.*".to_string(),
                "192.168.*".to_string(),
            ]),
            ..Default::default()
        });
        assert!(config.validate().is_ok());

        // 非法条目拒绝，错误消息含条目原文与序号
        for bad in ["10.0.0.0/33", "not-an-ip"] {
            let mut config = InklogConfig::default();
            config.http_server = Some(HttpServerConfig {
                enabled: true,
                ip_whitelist: Some(vec!["10.0.0.1".to_string(), bad.to_string()]),
                ..Default::default()
            });
            let err = config.validate().unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("ip_whitelist[1]") && msg.contains(bad),
                "unexpected error for {bad}: {msg}"
            );
        }
    }

    #[test]
    fn test_is_valid_prefix_wildcard_entry() {
        // 1-3 段点分十进制前缀均合法
        assert!(is_valid_prefix_wildcard_entry("10.*"));
        assert!(is_valid_prefix_wildcard_entry("192.168.*"));
        assert!(is_valid_prefix_wildcard_entry("192.168.1.*"));

        // 超过 3 段（完整 IP 加 ".*"）不合法
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.1.*"));
        // 非数字段
        assert!(!is_valid_prefix_wildcard_entry("abc.*"));
        assert!(!is_valid_prefix_wildcard_entry("10.x.*"));
        // 段越界（>255）
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.300.*"));
        assert!(!is_valid_prefix_wildcard_entry("256.*"));
        // 含 CIDR 分隔符
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.0/24.*"));
        assert!(!is_valid_prefix_wildcard_entry("10.0/24.*"));
        // 空前缀 / 空段
        assert!(!is_valid_prefix_wildcard_entry(".*"));
        assert!(!is_valid_prefix_wildcard_entry("10..*"));
        // 非通配形式一律不由此函数判定
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.1"));
        assert!(!is_valid_prefix_wildcard_entry("10.0.0.0/24"));
    }

    #[test]
    fn test_validate_http_ip_whitelist_prefix_wildcard_entries() {
        // `10.*` / `192.168.*` 与运行期 whitelist_entry_matches 语法一致，须通过校验
        let mut config = InklogConfig::default();
        config.http_server = Some(HttpServerConfig {
            enabled: true,
            ip_whitelist: Some(vec!["10.*".to_string(), "192.168.*".to_string()]),
            ..Default::default()
        });
        assert!(config.validate().is_ok());

        // 非法通配形式拒绝，错误消息含条目原文与序号
        for bad in [
            "abc.*",
            "10.0.0.300.*",
            "10.0.0.0/24.*",
            ".*",
            "10.0.0.1.*",
            "-1.*",
        ] {
            let mut config = InklogConfig::default();
            config.http_server = Some(HttpServerConfig {
                enabled: true,
                ip_whitelist: Some(vec![bad.to_string()]),
                ..Default::default()
            });
            let err = config.validate().unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("ip_whitelist[0]") && msg.contains(bad),
                "unexpected error for {bad}: {msg}"
            );
        }
    }

    #[test]
    fn test_from_search_paths_rejects_invalid_config_values() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("invalid.toml");
        std::fs::write(
            &config_path,
            r#"
[global]
level = "verbose"
"#,
        )
        .unwrap();

        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", config_path.to_str().unwrap());
        }
        let result = InklogConfig::from_search_paths();
        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
        assert!(
            result.is_err(),
            "config with invalid level must be rejected"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid log level"),
            "expected invalid log level error"
        );
    }

    #[test]
    fn test_from_str_rejects_invalid_values() {
        // channel_capacity = 0 无法自动修正，必须报错
        let result: Result<InklogConfig, _> = "[performance]\nchannel_capacity = 0\n".parse();
        assert!(result.is_err());

        // 非法日志级别必须报错
        let result: Result<InklogConfig, _> = "[global]\nlevel = \"verbose\"\n".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_sampling_toml_section_parses() {
        let toml_str = r#"
[sampling.per_level]
info = 10

[sampling.per_target_prefix."app::audit"]
keep_level = "debug"

[sampling.per_target_prefix."app::noise"]
sample_every_n = 2
"#;
        let config: InklogConfig = toml_str.parse().expect("should parse sampling TOML");
        assert_eq!(config.sampling.per_level.get("info"), Some(&10));
        let audit = config
            .sampling
            .per_target_prefix
            .get("app::audit")
            .expect("audit rule should exist");
        assert_eq!(audit.keep_level.as_deref(), Some("debug"));
        assert_eq!(audit.sample_every_n, 1, "rule rate must default to 1");
        let noise = config
            .sampling
            .per_target_prefix
            .get("app::noise")
            .expect("noise rule should exist");
        assert_eq!(noise.keep_level, None);
        assert_eq!(noise.sample_every_n, 2);
    }

    #[test]
    fn test_sampling_section_absent_defaults_to_empty() {
        let config: InklogConfig = "".parse().expect("empty TOML should parse");
        assert!(
            config.sampling.is_empty(),
            "absent [sampling] section must yield an empty policy"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_sampling() {
        // 零采样率拒绝
        let result: Result<InklogConfig, _> = "[sampling.per_level]\ninfo = 0\n".parse();
        assert!(result.is_err(), "zero per-level rate must be rejected");

        // 非法级别键拒绝
        let result: Result<InklogConfig, _> = "[sampling.per_level]\nverbose = 5\n".parse();
        assert!(result.is_err(), "invalid level key must be rejected");

        // 非法 keep_level 拒绝
        let result: Result<InklogConfig, _> = r#"
[sampling.per_target_prefix."app"]
keep_level = "verbose"
"#
        .parse();
        assert!(result.is_err(), "invalid keep_level must be rejected");
    }

    #[test]
    fn test_rate_limit_rules_toml_section_parses() {
        let toml_str = r#"
[rate_limit.rules]
"app::audit" = 500
"app::noise" = 10
"#;
        let config: InklogConfig = toml_str.parse().expect("should parse rate_limit TOML");
        assert_eq!(config.rate_limit.rules.get("app::audit"), Some(&500));
        assert_eq!(config.rate_limit.rules.get("app::noise"), Some(&10));
        assert!(!config.rate_limit.is_empty());
    }

    #[test]
    fn test_rate_limit_section_absent_defaults_to_empty() {
        let config: InklogConfig = "".parse().expect("empty TOML should parse");
        assert!(
            config.rate_limit.is_empty(),
            "absent [rate_limit] section must yield an empty quota config"
        );
    }

    #[cfg(feature = "schema")]
    #[test]
    fn test_config_schema_artifact_matches_committed_file() {
        let generated = serde_json::to_string_pretty(&InklogConfig::json_schema()).unwrap();
        let committed = include_str!("../../../config_schema.json");
        assert_eq!(
            generated.trim_end(),
            committed.trim_end(),
            "config_schema.json has drifted from the current config types — re-run: \
             cargo run --features cli --bin inklog-cli -- generate --schema --output config_schema.json"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_rate_limit_rules() {
        // 零速率拒绝
        let result: Result<InklogConfig, _> = "[rate_limit.rules]\n\"app::noise\" = 0\n".parse();
        assert!(result.is_err(), "zero per-group rate must be rejected");

        // 空前缀拒绝（TOML 裸键可表空串）
        let result: Result<InklogConfig, _> = "[rate_limit.rules]\n\"\" = 10\n".parse();
        assert!(result.is_err(), "empty prefix must be rejected");
    }
}
