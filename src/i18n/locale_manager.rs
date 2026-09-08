// Copyright (c) 2026 Kirky.X
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

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

/// Global locale manager state.
/// Parsed resources are leaked as `&'static` so each thread's bundle
/// cache can hold bundles borrowing them; `FluentBundle` itself lives in
/// the per-thread `BUNDLES` cache because it contains non-Send
/// memoizer internals.
static MANAGER: std::sync::OnceLock<RwLock<ManagerState>> = std::sync::OnceLock::new();

struct ManagerState {
    locale: String,
    /// locale_tag → (resource_name → FluentResource)
    resources: &'static HashMap<String, HashMap<String, FluentResource>>,
    /// Cached language identifiers per locale tag
    lang_ids: &'static HashMap<String, LanguageIdentifier>,
}

// Per-thread cache of locale tag → prebuilt bundle. Entries are keyed
// by locale tag, so a locale switch simply misses and rebuilds.
thread_local! {
    static BUNDLES: RefCell<HashMap<String, FluentBundle<&'static FluentResource>>> =
        RefCell::new(HashMap::new());
}

/// Initialize the global locale.
///
/// Resolution priority:
/// 1. `INKLOG_LOCALE` environment variable
/// 2. System locale via `sys-locale::get_locale()`
/// 3. Fallback to `"en"`
///
/// This function is idempotent — subsequent calls after the first are no-ops.
pub fn init_locale() {
    MANAGER.get_or_init(|| {
        let locale = resolve_locale();
        let (resources, lang_ids) = load_resources();
        RwLock::new(ManagerState {
            locale,
            // Leak so per-thread bundles can borrow the resources for 'static
            resources: Box::leak(Box::new(resources)),
            lang_ids: Box::leak(Box::new(lang_ids)),
        })
    });
}

/// Returns the current locale string (e.g. `"en"`, `"zh-CN"`).
///
/// Triggers initialization if not yet done.
pub fn current_locale() -> String {
    init_locale();
    MANAGER.get().unwrap().read().locale.clone()
}

/// Translate a message ID with optional arguments.
pub fn tr_args<'a>(id: &str, args: impl Into<FluentArgs<'a>>) -> String {
    tr_impl(id, Some(&args.into()))
}

/// Translate a message ID without arguments.
pub fn tr(id: &str) -> String {
    tr_impl(id, None)
}

// ── Internal helpers ──────────────────────────────────────────────

fn tr_impl(id: &str, args: Option<&FluentArgs<'_>>) -> String {
    init_locale();
    let locale = current_locale();

    // Try current locale, then fallback to en
    if let Some(result) = format_message(&locale, id, args) {
        return result;
    }
    if locale != "en"
        && let Some(result) = format_message("en", id, args)
    {
        return result;
    }
    // Return message ID as last resort
    id.to_string()
}

/// Format a message using the per-thread cached bundle for `locale`.
/// Bundles are built once per (thread, locale); the heavy work
/// (file I/O / FTL parsing) was done at init time.
fn format_message(locale: &str, id: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
    BUNDLES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache.contains_key(locale) {
            let manager = MANAGER.get().unwrap().read();
            let res_map = manager.resources.get(locale)?;
            let lang_id = manager.lang_ids.get(locale)?;
            let mut bundle: FluentBundle<&'static FluentResource> =
                FluentBundle::new(vec![lang_id.clone()]);
            for resource in res_map.values() {
                let _ = bundle.add_resource(resource);
            }
            cache.insert(locale.to_string(), bundle);
        }
        let bundle = cache.get(locale)?;
        let message = bundle.get_message(id)?;
        let pattern = message.value()?;
        let mut errors = vec![];
        let result = bundle
            .format_pattern(pattern, args, &mut errors)
            .to_string();
        Some(result)
    })
}

fn resolve_locale() -> String {
    // 0. 单元测试进程固定 en：与 CI（Linux，LANG 未设）行为一致。
    // Windows 中文系统的 sys-locale 检测出 zh，会使断言英文错误消息
    // 的测试失败；测试进程内固定 locale 是对环境差异的隔离。
    #[cfg(test)]
    if std::env::var("INKLOG_LOCALE").as_deref() != Ok("zh") {
        return "en".to_string();
    }

    // 1. Environment variable (highest priority)
    if let Ok(locale) = std::env::var("INKLOG_LOCALE") {
        let locale = locale.trim();
        if !locale.is_empty() && is_valid_locale(locale) {
            return normalize_locale(locale);
        }
    }

    // 2. System locale detection
    if let Some(sys_locale) = sys_locale::get_locale() {
        let sys_locale = sys_locale.trim();
        if !sys_locale.is_empty() && is_valid_locale(sys_locale) {
            return normalize_locale(sys_locale);
        }
    }

    // 3. Fallback
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
    // Reject known non-BCP-47 values
    if matches!(locale, "C" | "POSIX") {
        return false;
    }
    // Normalize, then validate as a BCP-47 language identifier
    normalize_locale(locale).parse::<LanguageIdentifier>().is_ok()
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
            (
                "validation.ftl",
                include_str!("../../locales/en/validation.ftl"),
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
                "validation.ftl",
                include_str!("../../locales/zh-CN/validation.ftl"),
            ),
        ],
    ),
];

fn load_resources() -> (
    HashMap<String, HashMap<String, FluentResource>>,
    HashMap<String, LanguageIdentifier>,
) {
    let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    load_resources_with_fallback(&locales_dir)
}

/// Prefer an on-disk locales directory (development hot-editing); the
/// build-machine path only exists during development, so deployed
/// binaries fall back to the embedded copies.
fn load_resources_with_fallback(
    locales_dir: &std::path::Path,
) -> (
    HashMap<String, HashMap<String, FluentResource>>,
    HashMap<String, LanguageIdentifier>,
) {
    let (resources, lang_ids) = load_resources_from_dir(locales_dir);
    if !resources.is_empty() {
        return (resources, lang_ids);
    }
    load_embedded_resources()
}

/// Load Fluent resources from an on-disk locale directory laid out as
/// `<dir>/<locale>/<name>.ftl`. Returns empty maps when the directory
/// does not exist or cannot be read.
fn load_resources_from_dir(
    dir: &std::path::Path,
) -> (
    HashMap<String, HashMap<String, FluentResource>>,
    HashMap<String, LanguageIdentifier>,
) {
    let mut resources: HashMap<String, HashMap<String, FluentResource>> = HashMap::new();
    let mut lang_ids: HashMap<String, LanguageIdentifier> = HashMap::new();

    if !dir.exists() {
        return (resources, lang_ids);
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[inklog] WARNING: failed to read locales dir: {e}");
            return (resources, lang_ids);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let locale_str = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let lang_id: LanguageIdentifier = match locale_str.parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("[inklog] WARNING: invalid locale identifier: {locale_str}");
                continue;
            }
        };

        let mut locale_resources = HashMap::new();

        if let Ok(ftl_entries) = std::fs::read_dir(&path) {
            for ftl_entry in ftl_entries.flatten() {
                let ftl_path = ftl_entry.path();
                if ftl_path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                    continue;
                }
                let source = match std::fs::read_to_string(&ftl_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!(
                            "[inklog] WARNING: failed to read {}: {e}",
                            ftl_path.display()
                        );
                        continue;
                    }
                };
                let resource_name = ftl_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                match FluentResource::try_new(source) {
                    Ok(r) => {
                        locale_resources.insert(resource_name, r);
                    }
                    Err((_res, errs)) => {
                        for err in errs {
                            eprintln!(
                                "[inklog] WARNING: FTL parse error in {}: {err:?}",
                                ftl_path.display()
                            );
                        }
                    }
                }
            }
        }

        if !locale_resources.is_empty() {
            resources.insert(locale_str.clone(), locale_resources);
            lang_ids.insert(locale_str, lang_id);
        }
    }

    (resources, lang_ids)
}

/// Build resources from [`EMBEDDED_LOCALES`] through the same
/// `FluentResource` parse path as [`load_resources_from_dir`].
fn load_embedded_resources() -> (
    HashMap<String, HashMap<String, FluentResource>>,
    HashMap<String, LanguageIdentifier>,
) {
    let mut resources: HashMap<String, HashMap<String, FluentResource>> = HashMap::new();
    let mut lang_ids: HashMap<String, LanguageIdentifier> = HashMap::new();

    for &(locale_str, files) in EMBEDDED_LOCALES {
        let Ok(lang_id) = locale_str.parse::<LanguageIdentifier>() else {
            eprintln!("[inklog] WARNING: invalid embedded locale identifier: {locale_str}");
            continue;
        };

        let mut locale_resources = HashMap::new();
        for &(file_name, source) in files {
            // Key by file stem so embedded resources are indistinguishable
            // from filesystem-loaded ones.
            let resource_name = file_name.strip_suffix(".ftl").unwrap_or(file_name);
            match FluentResource::try_new(source.to_string()) {
                Ok(r) => {
                    locale_resources.insert(resource_name.to_string(), r);
                }
                Err((_res, errs)) => {
                    for err in errs {
                        eprintln!(
                            "[inklog] WARNING: FTL parse error in embedded {locale_str}/{file_name}: {err:?}"
                        );
                    }
                }
            }
        }

        if !locale_resources.is_empty() {
            resources.insert(locale_str.to_string(), locale_resources);
            lang_ids.insert(locale_str.to_string(), lang_id);
        }
    }

    (resources, lang_ids)
}

#[cfg(test)]
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
            format_message("zh-CN", "log_level-name_info", None).as_deref(),
            Some("信息")
        );
        // en_US.UTF-8 normalizes to en-US (no dedicated directory); the
        // same fallback chain tr() uses then resolves it to en resources
        let locale = normalize_locale("en_US.UTF-8");
        assert_eq!(locale, "en-US");
        let translated = format_message(&locale, "log_level-name_info", None)
            .or_else(|| format_message("en", "log_level-name_info", None));
        assert_eq!(translated.as_deref(), Some("INFO"));
    }

    #[test]
    fn test_bundle_cache_keeps_locales_separate() {
        init_locale();
        // Repeated lookups exercise cached bundles; a locale switch must
        // not leak translations across cache entries.
        for _ in 0..3 {
            assert_eq!(
                format_message("en", "log_level-name_warn", None).as_deref(),
                Some("WARN")
            );
            assert_eq!(
                format_message("zh-CN", "log_level-name_warn", None).as_deref(),
                Some("警告")
            );
        }
    }

    #[test]
    fn test_load_resources_missing_dir_falls_back_to_embedded() {
        // Simulates a deployed binary: no on-disk locales directory at
        // all. Translations must still work via the embedded copies,
        // resolved through the same Fluent path as the filesystem loader.
        let (resources, lang_ids) =
            load_resources_with_fallback(std::path::Path::new("/nonexistent/inklog/locales"));
        assert!(lang_ids.contains_key("en"), "en must load from embedded");
        assert!(
            lang_ids.contains_key("zh-CN"),
            "zh-CN must load from embedded"
        );

        let mut bundle = FluentBundle::new(vec![lang_ids["zh-CN"].clone()]);
        for resource in resources["zh-CN"].values() {
            let _ = bundle.add_resource(resource);
        }
        let message = bundle
            .get_message("log_level-name_info")
            .expect("embedded zh-CN message");
        let pattern = message.value().expect("pattern");
        let mut errors = vec![];
        assert_eq!(
            bundle.format_pattern(pattern, None, &mut errors).to_string(),
            "信息"
        );
    }

    #[test]
    fn test_embedded_resources_match_filesystem_entries() {
        let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let (fs_resources, _) = load_resources_from_dir(&locales_dir);
        let (embedded, _) = load_embedded_resources();
        assert!(!embedded.is_empty(), "embedded table must not be empty");
        for (locale, fs_map) in &fs_resources {
            let emb_map = embedded.get(locale).unwrap_or_else(|| {
                panic!("EMBEDDED_LOCALES missing locale dir '{locale}'")
            });
            assert_eq!(
                emb_map.len(),
                fs_map.len(),
                "locale '{locale}': embedded resource count differs from filesystem"
            );
            for name in fs_map.keys() {
                assert!(
                    emb_map.contains_key(name),
                    "locale '{locale}': embedded resources missing '{name}'"
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
}
