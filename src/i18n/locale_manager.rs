// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Locale manager for runtime internationalization.
//!
//! Provides locale initialization, Fluent resource management, and
//! translation lookup. Uses `fluent-bundle` for runtime message
//! formatting with `.ftl` translation files.
//!
//! ## Locale Resolution Priority
//!
//! 1. `INKLOG_LOCALE` environment variable
//! 2. System locale via `sys-locale`
//! 3. Fallback to `"en"`

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use parking_lot::RwLock;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

/// Global locale manager state.
/// Stores `FluentResource` (Send + Sync) instead of `FluentBundle`
/// because `FluentBundle` contains non-Send memoizer internals.
static MANAGER: std::sync::OnceLock<RwLock<ManagerState>> = std::sync::OnceLock::new();

struct ManagerState {
    locale: String,
    /// locale_tag → (resource_name → FluentResource)
    resources: HashMap<String, HashMap<String, FluentResource>>,
    /// Cached language identifiers per locale tag
    lang_ids: HashMap<String, LanguageIdentifier>,
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
            resources,
            lang_ids,
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
    let manager = MANAGER.get().unwrap().read();

    // Try current locale, then fallback to en
    if let Some(result) = format_message(&manager, &manager.locale, id, args) {
        return result;
    }
    if manager.locale != "en"
        && let Some(result) = format_message(&manager, "en", id, args)
    {
        return result;
    }
    // Return message ID as last resort
    id.to_string()
}

/// Create a temporary bundle and format a message.
/// Bundle creation is cheap (just memoizer init); the heavy work
/// (YAML/FTL parsing) was done at init time.
fn format_message(
    manager: &ManagerState,
    locale: &str,
    id: &str,
    args: Option<&FluentArgs<'_>>,
) -> Option<String> {
    let res_map = manager.resources.get(locale)?;
    let lang_id = manager.lang_ids.get(locale)?;
    let mut bundle = FluentBundle::new(vec![lang_id.clone()]);
    for resource in res_map.values() {
        let _ = bundle.add_resource(resource);
    }
    let message = bundle.get_message(id)?;
    let pattern = message.value()?;
    let mut errors = vec![];
    let result = bundle
        .format_pattern(pattern, args, &mut errors)
        .to_string();
    Some(result)
}

fn resolve_locale() -> String {
    // 1. Environment variable (highest priority)
    if let Ok(locale) = std::env::var("INKLOG_LOCALE") {
        let locale = locale.trim().to_string();
        if !locale.is_empty() {
            return locale;
        }
    }

    // 2. System locale detection
    if let Some(sys_locale) = sys_locale::get_locale() {
        let sys_locale = sys_locale.trim().to_string();
        if !sys_locale.is_empty() {
            return sys_locale;
        }
    }

    // 3. Fallback
    "en".to_string()
}

fn load_resources() -> (
    HashMap<String, HashMap<String, FluentResource>>,
    HashMap<String, LanguageIdentifier>,
) {
    let mut resources: HashMap<String, HashMap<String, FluentResource>> = HashMap::new();
    let mut lang_ids: HashMap<String, LanguageIdentifier> = HashMap::new();
    let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");

    if !locales_dir.exists() {
        return (resources, lang_ids);
    }

    let entries = match std::fs::read_dir(&locales_dir) {
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
        let result = tr("error.config_error");
        assert!(!result.is_empty());
    }
}
