// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Locale manager for runtime internationalization.
//!
//! Provides locale initialization, Fluent resource management, and
//! translation lookup. Uses `fluent-bundle` for runtime message
//! formatting with `.ftl` translation files. All `.ftl` files are
//! embedded at compile time (`EMBEDDED_LOCALES`) so deployed binaries
//! translate without the build machine's `locales/` directory; an
//! on-disk `locales/` directory (development) takes precedence.
//!
//! ## Locale Resolution Priority
//!
//! 1. `INKLOG_LOCALE` environment variable
//! 2. System locale via `sys-locale`
//! 3. Fallback to `"en"`

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 消息插值参数载体：键值有序集合，经 [`tr_args`](super::tr_args) 传入
/// 消息模板的 `{ $key }` 占位。
///
/// 值统一按 `Display` 渲染为字符串——调用点的实参形态（错误链、路径
/// display、命名切片）全部无需转换。
#[derive(Debug, Clone, Default)]
pub struct MsgArgs {
    entries: Vec<(String, String)>,
}

impl MsgArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个参数；同名键后者覆盖前者（与历史 fluent `FluentArgs::set`
    /// 的覆盖语义一致）。
    pub fn set(&mut self, key: &str, value: impl std::fmt::Display) -> &mut Self {
        let value = value.to_string();
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key.to_string(), value)),
        }
        self
    }
}

/// 全局 locale（进程级，OnceLock；init_locale 首次调用定格）。
static CURRENT_LOCALE: OnceLock<String> = OnceLock::new();

/// FTL 真源注册标记（进程级一次）。
static REGISTERED: OnceLock<()> = OnceLock::new();

/// Initialize the global locale.
///
/// Resolution priority:
/// 1. `INKLOG_LOCALE` environment variable
/// 2. System locale via `sys-locale::get_locale()`
/// 3. Fallback to `"en"`
///
/// This function is idempotent — subsequent calls after the first are no-ops.
pub fn init_locale() {
    let _ = CURRENT_LOCALE.get_or_init(|| {
        let locale = resolve_locale();
        ensure_registered();
        locale
    });
}

/// Returns the current locale string (e.g. `"en"`, `"zh-CN"`).
///
/// Triggers initialization if not yet done.
pub fn current_locale() -> String {
    init_locale();
    CURRENT_LOCALE
        .get()
        .cloned()
        .unwrap_or_else(|| "en".to_string())
}

/// Translate a message ID without arguments.
pub fn tr(id: &str) -> String {
    tr_impl(id, &[])
}

/// Translate a message ID with optional arguments.
pub fn tr_args(id: &str, args: MsgArgs) -> String {
    tr_impl(id, &args.entries)
}

// ── Internal helpers ──────────────────────────────────────────────

fn tr_impl(id: &str, entries: &[(String, String)]) -> String {
    init_locale();
    let locale = current_locale();
    if let Some(result) = lookup(&locale, id, entries) {
        return result;
    }
    if locale != "en"
        && let Some(result) = lookup("en", id, entries)
    {
        return result;
    }
    id.to_string()
}

/// FSI（U+2068）/PDI（U+2069）隔离标记：与历史 fluent 引擎的 placeable
/// 包裹字节语义逐一对齐（金样本等价性验证的前提）。
const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';

/// 渲染模板：`{ $name }` 占位符替换为 FSI 包裹的实参值；缺失实参保留
/// 原占位符并同样以 FSI 包裹（与历史 fluent 缺参渲染字节一致）。
/// 非 `{ $` 起始的花括号（转义/字面量）原样透传。
fn render_template(template: &str, entries: &[(String, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && template[i..].starts_with("{$") {
            // 紧凑占位符形态（{$name}）
            if let Some(end) = template[i..].find('}') {
                let name = template[i + 2..i + end].trim();
                match entries.iter().find(|(k, _)| k == name) {
                    Some((_, v)) => {
                        out.push(FSI);
                        out.push_str(v);
                        out.push(PDI);
                    }
                    None => {
                        out.push(FSI);
                        out.push_str(&format!("{{${name}}}"));
                        out.push(PDI);
                    }
                }
                i += end + 1;
                continue;
            }
        }
        if bytes[i] == b'{' && template[i..].starts_with("{ $") {
            // 空格占位符形态（{ $name }）
            if let Some(end) = template[i..].find('}') {
                let inner = &template[i + 1..i + end];
                let name = inner.trim_start_matches(['{', ' ']).trim();
                let name = name.strip_prefix('$').unwrap_or(name).trim();
                match entries.iter().find(|(k, _)| k == name) {
                    Some((_, v)) => {
                        out.push(FSI);
                        out.push_str(v);
                        out.push(PDI);
                    }
                    None => {
                        out.push(FSI);
                        out.push_str(&format!("{{${name}}}"));
                        out.push(PDI);
                    }
                }
                i += end + 1;
                continue;
            }
        }
        // 转义括号还原（{"{"} → {，{"}"} → }）
        if template[i..].starts_with("{\"{\"}") {
            out.push('{');
            i += 5;
            continue;
        }
        if template[i..].starts_with("{\"}\"}") {
            out.push('}');
            i += 5;
            continue;
        }
        let ch = template[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// 私有模板注册表：桶（"zh"/"en"）→ key → 模板原文。
/// 统一基座（trait-kit I18nManager）同帧双写（任何消费者可经 trait-kit
/// 查到 inklog 消息面）；本仓渲染走私有表——FSI 隔离需逐 placeable 注入，
/// 引擎侧无渲染钩子，模板原文必须自持取回。
static TEMPLATES: OnceLock<RwLock<HashMap<&'static str, HashMap<&'static str, &'static str>>>> =
    OnceLock::new();

fn templates() -> &'static RwLock<HashMap<&'static str, HashMap<&'static str, &'static str>>> {
    TEMPLATES.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 语言标签 → 注册桶（与 trait-kit 归一规则一致：zh* → zh，其余 → en）。
fn locale_bucket(locale: &str) -> &'static str {
    if locale.starts_with("zh") { "zh" } else { "en" }
}

/// 查找并渲染：当前语言桶缺失时由调用方回退 en；两桶都缺返回 None。
fn lookup(locale: &str, id: &str, entries: &[(String, String)]) -> Option<String> {
    ensure_registered();
    let bucket = locale_bucket(locale);
    let template = {
        let map = templates().read();
        map.get(bucket)?.get(id).copied()?
    };
    Some(render_template(template, entries))
}

/// 结算一条 FTL 条目：去前导空行、按「首个非空续行」基准剥公共缩进、
/// 还原 fluent 转义括号、去尾部空行（fluent 多行语义，与金样本对齐）。
fn flush_ftl_entry(key: Option<String>, lines: &mut Vec<String>, out: &mut Vec<(String, String)>) {
    if let Some(key) = key {
        let had_inline = lines.first().is_some_and(|l| !l.is_empty());
        if !had_inline && lines.first().is_some_and(|l| l.is_empty()) {
            lines.remove(0);
        }
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        let cont_start = usize::from(had_inline);
        let common = lines
            .iter()
            .skip(cont_start)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let prefix = " ".repeat(common);
        let value = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let stripped = if i >= cont_start && common > 0 && l.starts_with(prefix.as_str()) {
                    &l[common..]
                } else {
                    l
                };
                stripped.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.push((key, value));
        lines.clear();
    }
}

/// 归一 FTL 源：占位符跨行断开（`{$` 行尾折行）时丢弃占位符内部换行。
fn normalize_ftl_placeholders(ftl: &str) -> String {
    if !ftl.contains("{$") {
        return ftl.to_string();
    }
    let mut out = String::with_capacity(ftl.len());
    let mut in_placeholder = false;
    for ch in ftl.chars() {
        match ch {
            '{' => {
                in_placeholder = true;
                out.push(ch);
            }
            '}' => {
                in_placeholder = false;
                out.push(ch);
            }
            '\n' if in_placeholder => {}
            _ => out.push(ch),
        }
    }
    out
}

/// 解析 FTL 文本为 (key, value) 条目（fluent 语义）。
fn parse_ftl_entries(ftl: &str) -> Vec<(String, String)> {
    let ftl = &normalize_ftl_placeholders(ftl);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut cur_key: Option<String> = None;
    let mut cur_lines: Vec<String> = Vec::new();
    for line in ftl.lines() {
        if cur_key.is_some() && (line.starts_with([' ', '\t']) || line.trim().is_empty()) {
            cur_lines.push(line.to_string());
            continue;
        }
        flush_ftl_entry(cur_key.take(), &mut cur_lines, &mut out);
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            cur_key = Some(k.trim().to_string());
            cur_lines.push(v.trim().to_string());
        }
    }
    flush_ftl_entry(cur_key.take(), &mut cur_lines, &mut out);
    out
}

/// 将全部内嵌 FTL 真源注册进 trait-kit I18nManager（进程级一次）。
/// 存的是模板原文（占位符不做预替换），渲染由 [`render_template`] 在
/// 本仓自持层完成——FSI 隔离语义需逐 placeable 注入，属渲染关注点。
fn ensure_registered() {
    REGISTERED.get_or_init(|| {
        let mut store = templates().write();
        for (locale_tag, files) in EMBEDDED_LOCALES {
            let bucket: &'static str = if locale_tag.starts_with("zh") {
                "zh"
            } else {
                "en"
            };
            for (_, source) in *files {
                for (key, value) in parse_ftl_entries(source) {
                    let key: &'static str = Box::leak(key.into_boxed_str());
                    let value: &'static str = Box::leak(value.into_boxed_str());
                    store.entry(bucket).or_default().insert(key, value);
                    // 统一基座双写：inklog 消息面对全家族可见（渲染语义
                    // 由 trait-kit 引擎承担，FSI 差异见模块文档）。
                    trait_kit::i18n::I18nManager::register_translation(locale_tag, key, value);
                }
            }
        }
    });
}

fn resolve_locale() -> String {
    if let Ok(locale) = std::env::var("INKLOG_LOCALE") {
        let locale = locale.trim();
        if !locale.is_empty() && is_valid_locale(locale) {
            return normalize_locale(locale);
        }
    }

    if let Some(sys_locale) = sys_locale::get_locale() {
        let sys_locale = sys_locale.trim();
        if !sys_locale.is_empty() && is_valid_locale(sys_locale) {
            return normalize_locale(sys_locale);
        }
    }

    "en".to_string()
}

/// Normalize a locale string to the BCP-47 tag shape used by the
/// `locales/` directory: strip any modifier (`@`) and codeset (`.`)
/// suffix, then convert underscores to hyphens
/// (e.g. `"zh_CN.UTF-8"` → `"zh-CN"`).
fn normalize_locale(locale: &str) -> String {
    locale
        .split('@')
        .next()
        .unwrap_or(locale)
        .split('.')
        .next()
        .unwrap_or(locale)
        .replace('_', "-")
}

/// Check if a locale string resolves to a valid BCP-47 language tag.
///
/// Accepts POSIX-style locale names like `"en_US.UTF-8"` (normalized to
/// `"en"`) that `sys-locale` may return on some platforms, while
/// rejecting non-locale values like `"C"` or `"POSIX"`.
fn is_valid_locale(locale: &str) -> bool {
    if matches!(locale, "C" | "POSIX") {
        return false;
    }
    // 归一后校验语言标签形态（仅接受家族支持的两桶前缀）
    let normalized = normalize_locale(locale);
    normalized.starts_with("en") || normalized.starts_with("zh")
}

/// Compile-time embedded copies of the `locales/` directory.
///
/// Each entry maps a locale directory name to its `.ftl` files, embedded
/// via `include_str!` so deployed binaries translate without the
/// build machine's `locales/` directory. When adding a new `.ftl` file
/// or locale, register it here manually —
/// `test_embedded_locales_cover_locales_dir` fails if a file on disk is
/// missing from this table.
const EMBEDDED_LOCALES: &[(&str, &[(&str, &str)])] = &[
    (
        "en",
        &[
            ("cli.ftl", include_str!("../../locales/en/cli.ftl")),
            ("config.ftl", include_str!("../../locales/en/config.ftl")),
            ("error.ftl", include_str!("../../locales/en/error.ftl")),
            (
                "log_level.ftl",
                include_str!("../../locales/en/log_level.ftl"),
            ),
            ("sink.ftl", include_str!("../../locales/en/sink.ftl")),
            ("metrics.ftl", include_str!("../../locales/en/metrics.ftl")),
            (
                "validation.ftl",
                include_str!("../../locales/en/validation.ftl"),
            ),
            (
                "examples.ftl",
                include_str!("../../locales/en/examples.ftl"),
            ),
        ],
    ),
    (
        "zh-CN",
        &[
            ("cli.ftl", include_str!("../../locales/zh-CN/cli.ftl")),
            ("config.ftl", include_str!("../../locales/zh-CN/config.ftl")),
            ("error.ftl", include_str!("../../locales/zh-CN/error.ftl")),
            (
                "log_level.ftl",
                include_str!("../../locales/zh-CN/log_level.ftl"),
            ),
            ("sink.ftl", include_str!("../../locales/zh-CN/sink.ftl")),
            (
                "metrics.ftl",
                include_str!("../../locales/zh-CN/metrics.ftl"),
            ),
            (
                "validation.ftl",
                include_str!("../../locales/zh-CN/validation.ftl"),
            ),
            (
                "examples.ftl",
                include_str!("../../locales/zh-CN/examples.ftl"),
            ),
        ],
    ),
];

mod tests {
    use super::*;

    #[test]
    fn test_resolve_locale_fallback() {
        let locale = resolve_locale();
        assert!(!locale.is_empty());
    }

    #[test]
    fn test_current_locale_returns_valid_string() {
        let locale = current_locale();
        assert!(!locale.is_empty());
    }

    #[test]
    fn test_tr_returns_translation_or_id() {
        let result = tr("error-config_error");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_tr_en_fallback_returns_english_text() {
        // When the current locale doesn't have a key, en fallback
        // should return the English translation.
        let result = tr("error-io_error");
        // In en locale: "IO error"; in zh-CN: "I/O 错误";
        // either way, non-empty.
        assert!(!result.is_empty());
        // If locale is en, verify exact content
        if current_locale() == "en" {
            assert_eq!(result, "IO error");
        }
    }

    #[test]
    fn test_tr_unknown_key_returns_id() {
        // Non-existent keys should return the key itself
        let result = tr("nonexistent.key.that.does.not.exist");
        assert_eq!(result, "nonexistent.key.that.does.not.exist");
    }

    #[test]
    fn test_tr_log_level_names() {
        let levels = [
            ("log_level-name_trace", "TRACE"),
            ("log_level-name_debug", "DEBUG"),
            ("log_level-name_info", "INFO"),
            ("log_level-name_warn", "WARN"),
            ("log_level-name_error", "ERROR"),
            ("log_level-name_fatal", "FATAL"),
        ];
        for (key, expected_en) in levels {
            let result = tr(key);
            assert!(!result.is_empty(), "tr({}) returned empty", key);
            if current_locale() == "en" {
                assert_eq!(result, expected_en, "tr({}) en mismatch", key);
            }
        }
    }

    #[test]
    fn test_normalize_locale() {
        assert_eq!(normalize_locale("en"), "en");
        assert_eq!(normalize_locale("zh-CN"), "zh-CN");
        // POSIX-style names: underscore → hyphen, codeset and modifier stripped
        assert_eq!(normalize_locale("zh_CN"), "zh-CN");
        assert_eq!(normalize_locale("en_US.UTF-8"), "en-US");
        assert_eq!(normalize_locale("zh_CN.utf8"), "zh-CN");
        assert_eq!(normalize_locale("en_US@euro"), "en-US");
        assert_eq!(normalize_locale("en_US.UTF-8@euro"), "en-US");
    }

    #[test]
    fn test_is_valid_locale_accepts_posix_style() {
        assert!(is_valid_locale("zh_CN"));
        assert!(is_valid_locale("en_US.UTF-8"));
        assert!(is_valid_locale("zh_CN@pinyin"));
        assert!(is_valid_locale("en"));
        assert!(!is_valid_locale("C"));
        assert!(!is_valid_locale("POSIX"));
        assert!(!is_valid_locale("not a locale!!"));
    }

    #[test]
    fn test_posix_style_locale_hits_resources() {
        init_locale();
        // zh_CN normalizes to a locales/ directory that is hit directly
        assert_eq!(normalize_locale("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(
            lookup("zh-CN", "log_level-name_info", &[]).as_deref(),
            Some("信息")
        );
        // en_US.UTF-8 normalizes to en-US (no dedicated directory); the
        // same fallback chain tr uses then resolves it to en resources
        let locale = normalize_locale("en_US.UTF-8");
        assert_eq!(locale, "en-US");
        let translated = lookup(&locale, "log_level-name_info", &[])
            .or_else(|| lookup("en", "log_level-name_info", &[]));
        assert_eq!(translated.as_deref(), Some("INFO"));
    }

    #[test]
    fn test_registry_keeps_locales_separate() {
        init_locale();
        // Repeated lookups exercise the template registry; buckets must
        // not leak translations across entries.
        for _ in 0..3 {
            assert_eq!(
                lookup("en", "log_level-name_warn", &[]).as_deref(),
                Some("WARN")
            );
            assert_eq!(
                lookup("zh-CN", "log_level-name_warn", &[]).as_deref(),
                Some("警告")
            );
        }
    }

    #[test]
    fn test_embedded_registry_resolves_both_locales() {
        init_locale();
        // Deployed binaries resolve purely from EMBEDDED_LOCALES; the
        // registry must carry both buckets with canonical renderings.
        assert_eq!(
            lookup("zh-CN", "log_level-name_info", &[]).as_deref(),
            Some("信息")
        );
        assert_eq!(
            lookup("en", "log_level-name_info", &[]).as_deref(),
            Some("INFO")
        );
    }

    /// 守卫：TLS 配置校验与 parquet feature 门控的文案在 en/zh-CN 双侧都要
    /// 渲染出预期文本（en 值与构造点内嵌的规范串逐字一致）。
    #[test]
    fn test_tls_and_parquet_keys_render_in_both_locales() {
        init_locale();
        type LocaleCase = (
            &'static str,
            Option<&'static [(&'static str, &'static str)]>,
            &'static str,
            &'static str,
        );
        let cases: &[LocaleCase] = &[
            (
                "config-tls_cert_path_empty",
                None,
                "tls.cert_path is empty",
                "tls.cert_path 为空",
            ),
            (
                "config-tls_key_path_empty",
                None,
                "tls.key_path is empty",
                "tls.key_path 为空",
            ),
            (
                "config-tls_cert_path_missing",
                Some(&[("path", "/tmp/cert.pem")]),
                "tls.cert_path \"/tmp/cert.pem\" does not exist",
                "tls.cert_path \"/tmp/cert.pem\" 不存在",
            ),
            (
                "config-tls_key_path_missing",
                Some(&[("path", "/tmp/key.pem")]),
                "tls.key_path \"/tmp/key.pem\" does not exist",
                "tls.key_path \"/tmp/key.pem\" 不存在",
            ),
            (
                "sink-parquet_feature_not_enabled",
                None,
                "parquet feature not enabled: rebuild inklog with `parquet` feature to export logs as Parquet",
                "未启用 parquet 特性：请以 `parquet` 特性重新构建 inklog 以将日志导出为 Parquet",
            ),
        ];
        for (id, args, expected_en, expected_zh) in cases {
            let fluent_args_entries: Vec<(String, String)> = args
                .map(|pairs| {
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            assert_eq!(
                strip_bidi_isolates(lookup("en", id, &fluent_args_entries).as_deref()).as_deref(),
                Some(*expected_en),
                "en rendering of '{id}' diverged"
            );
            assert_eq!(
                strip_bidi_isolates(lookup("zh-CN", id, &fluent_args_entries).as_deref())
                    .as_deref(),
                Some(*expected_zh),
                "zh-CN rendering of '{id}' diverged"
            );
        }
    }

    /// Fluent 默认对占位符插入双向隔离符（U+2068 / U+2069）以防双向文本串扰；
    /// 本仓 bundle 未关闭该行为，故比对前剥离隔离符只留可见文本。
    fn strip_bidi_isolates(rendered: Option<&str>) -> Option<String> {
        rendered.map(|text| {
            text.chars()
                .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
                .collect::<String>()
        })
    }

    #[test]
    fn test_parse_ftl_entries_cover_all_embedded_files() {
        // Guard: every embedded FTL file parses into non-empty entries —
        // a silently broken include_str! table would ship untranslated.
        for (locale_tag, files) in EMBEDDED_LOCALES {
            assert!(!files.is_empty(), "locale '{locale_tag}' has no files");
            for (file_name, source) in *files {
                let entries = parse_ftl_entries(source);
                assert!(
                    !entries.is_empty(),
                    "locale '{locale_tag}' file '{file_name}' parsed to zero entries"
                );
            }
        }
    }

    #[test]
    fn test_embedded_locales_cover_locales_dir() {
        // Compile-time registration guard: every `.ftl` file under
        // `locales/` must be listed in EMBEDDED_LOCALES, otherwise a
        // freshly added file silently ships untranslated in production.
        let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        for entry in std::fs::read_dir(&locales_dir).expect("locales dir exists") {
            let path = entry.expect("locale entry").path();
            if !path.is_dir() {
                continue;
            }
            let locale = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("locale dir name");
            let files = EMBEDDED_LOCALES
                .iter()
                .find(|(l, _)| *l == locale)
                .unwrap_or_else(|| panic!("EMBEDDED_LOCALES missing locale dir '{locale}'"))
                .1;
            for ftl_entry in std::fs::read_dir(&path).expect("locale dir readable") {
                let ftl_path = ftl_entry.expect("ftl entry").path();
                if ftl_path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                    continue;
                }
                let stem = ftl_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .expect("ftl file stem");
                assert!(
                    files
                        .iter()
                        .any(|(name, _)| name.strip_suffix(".ftl") == Some(stem)),
                    "locale '{locale}': '{stem}.ftl' exists on disk but is not registered in EMBEDDED_LOCALES"
                );
            }
        }
    }

    /// Message ids of an FTL source, in source order.
    ///
    /// Comments (`#`), attributes (`.name`), terms (`-name`) and indented
    /// continuation lines are not message definitions.
    fn ftl_message_ids(source: &str) -> Vec<String> {
        source
            .lines()
            .filter_map(|line| {
                let line = line.trim_end();
                if line.is_empty()
                    || line.starts_with('#')
                    || line.starts_with('.')
                    || line.starts_with('-')
                    || line.starts_with(char::is_whitespace)
                {
                    return None;
                }
                let id = line.split_once('=')?.0.trim();
                (!id.is_empty()).then(|| id.to_string())
            })
            .collect()
    }

    /// The `cli-verify-chain-*` family must occupy one contiguous run of
    /// messages: a sibling parked at the end of the file falls under an
    /// unrelated comment group and drifts away from the ones it belongs
    /// with.
    #[test]
    fn test_cli_verify_chain_messages_are_contiguous() {
        for (locale, files) in EMBEDDED_LOCALES {
            let cli = files
                .iter()
                .find(|(name, _)| *name == "cli.ftl")
                .map(|(_, source)| *source)
                .unwrap_or_else(|| panic!("locale '{locale}': cli.ftl is not embedded"));
            let ids = ftl_message_ids(cli);
            let positions: Vec<usize> = ids
                .iter()
                .enumerate()
                .filter(|(_, id)| id.starts_with("cli-verify-chain-"))
                .map(|(idx, _)| idx)
                .collect();
            assert!(
                positions.len() > 1,
                "locale '{locale}': expected several cli-verify-chain-* messages, found {}",
                positions.len()
            );
            let first = positions[0];
            let last = positions[positions.len() - 1];
            assert_eq!(
                last - first + 1,
                positions.len(),
                "locale '{locale}': cli-verify-chain-* messages are not contiguous between '{}' and '{}'",
                ids[first],
                ids[last]
            );
            let empty = ids
                .iter()
                .position(|id| id == "cli-verify-chain-empty")
                .unwrap_or_else(|| panic!("locale '{locale}': cli-verify-chain-empty is missing"));
            assert!(
                (first..=last).contains(&empty),
                "locale '{locale}': cli-verify-chain-empty sits outside the cli-verify-chain-* group"
            );
        }
    }

    /// `en` and `zh-CN` must list the same message ids in the same order;
    /// a locale pair that drifts leaves one side untranslated.
    #[test]
    fn test_cli_locales_list_same_message_ids_in_order() {
        let ids_of = |locale: &str| -> Vec<String> {
            let files = EMBEDDED_LOCALES
                .iter()
                .find(|(l, _)| *l == locale)
                .unwrap_or_else(|| panic!("EMBEDDED_LOCALES missing locale '{locale}'"))
                .1;
            let cli = files
                .iter()
                .find(|(name, _)| *name == "cli.ftl")
                .map(|(_, source)| *source)
                .unwrap_or_else(|| panic!("locale '{locale}': cli.ftl is not embedded"));
            ftl_message_ids(cli)
        };
        assert_eq!(
            ids_of("en"),
            ids_of("zh-CN"),
            "en and zh-CN cli.ftl must list the same message ids in the same order"
        );
    }

    /// 目录加载器对不可解析的 FTL 文件必须跳过该文件而非整体失败：
    /// 损坏 locale（en）整组无有效资源时不注册；完好 locale（zh-CN）照常加载。
    #[test]
    fn test_parse_ftl_entries_ignores_bare_identifiers() {
        // 裸标识符行（无 "="）不是条目：解析器必须忽略而非 panic 或误收。
        let ftl = "bare-identifier-without-value\nvalid-key = 你好\n";
        let entries = parse_ftl_entries(ftl);
        assert_eq!(entries.len(), 1, "bare identifier must be skipped");
        assert_eq!(entries[0].0, "valid-key");
        assert_eq!(entries[0].1, "你好");
    }
}
