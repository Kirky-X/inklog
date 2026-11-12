// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Locale-aware formatting delegating to the trait-kit `I18nFormatter`
//! (ICU4X single source through the family base; `PluralCategory` via
//! the rc.14 re-export — no self-held `icu` dependency).

use std::cmp::Ordering;

use trait_kit::i18n::PluralCategory;

use super::{I18nError, LogI18nFormatter};

/// Map a [`PluralCategory`] to its capitalized CLDR name (e.g. `"One"`, `"Other"`).
fn plural_category_name(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "Zero",
        PluralCategory::One => "One",
        PluralCategory::Two => "Two",
        PluralCategory::Few => "Few",
        PluralCategory::Many => "Many",
        PluralCategory::Other => "Other",
    }
}

impl LogI18nFormatter {
    /// Create a new formatter using the current system locale.
    ///
    /// Delegates to [`LogI18nFormatter::new`] with the locale
    /// detected by [`current_locale`](super::current_locale).
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidLocale`] if the detected locale
    /// cannot be parsed, or [`I18nError::FormatError`] if ICU4X
    /// lacks compiled data for it.
    pub fn new_default() -> Result<Self, I18nError> {
        let locale = super::current_locale();
        Self::new(&locale)
    }

    /// Create a new formatter for the given BCP-47 locale tag.
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidLocale`] if the tag cannot be parsed,
    /// or [`I18nError::FormatError`] if ICU4X lacks compiled data for it.
    pub fn new(locale: &str) -> Result<Self, I18nError> {
        let inner = trait_kit::i18n::I18nFormatter::new(locale).map_err(|e| {
            // 构造失败 = locale 无效或无编译数据：与历史 ICU 直连语义对齐，
            // 标签解析类失败归类 InvalidLocale，其余保留 FormatError。
            let text = e.to_string();
            if text.contains("invalid locale") || text.contains("Invalid extension") {
                I18nError::InvalidLocale {
                    input: locale.to_string(),
                    reason: text,
                }
            } else {
                I18nError::FormatError(format!(
                    "trait-kit I18nFormatter construction failed: {text}"
                ))
            }
        })?;
        Ok(Self {
            inner,
            locale: locale.to_string(),
        })
    }

    /// Format a floating-point number with locale-sensitive grouping
    /// and decimal separators.
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidNumber`] for non-finite values or
    /// if the value cannot be parsed into a fixed decimal.
    pub fn format_number(&self, value: f64) -> Result<String, I18nError> {
        if !value.is_finite() {
            return Err(I18nError::InvalidNumber {
                input: value.to_string(),
                reason: "value is not finite (NaN or Infinity)".into(),
            });
        }
        self.inner
            .format_number(value)
            .map_err(|e| I18nError::FormatError(e.to_string()))
    }

    /// Return the plural category name for `count` in the formatter's locale
    /// (e.g. `"One"` for English count=1, `"Other"` for count=2).
    ///
    /// Use this to build locale-aware log messages such as
    /// "1 event" vs "2 events".
    ///
    /// # Errors
    /// This method does not currently fail, but returns `Result` for API
    /// consistency with the other formatting methods.
    pub fn format_event_count(&self, count: u64) -> Result<String, I18nError> {
        let category = self
            .inner
            .plural_category(count)
            .map_err(|e| I18nError::FormatError(e.to_string()))?;
        Ok(plural_category_name(category).to_string())
    }

    /// Format an ISO calendar date (year / month / day) as a log
    /// timestamp using a medium-length locale-specific pattern.
    ///
    /// # Errors
    /// Returns [`I18nError::DateError`] if any component is out of range,
    /// or [`I18nError::FormatError`] if the formatter cannot be constructed.
    pub fn format_timestamp(&self, year: i32, month: u8, day: u8) -> Result<String, I18nError> {
        self.inner
            .format_date(year, month, day)
            .map_err(|e| I18nError::DateError(e.to_string()))
    }

    /// Normalize a log level string to uppercase canonical form
    /// (e.g. `"info"` → `"INFO"`, `"debug"` → `"DEBUG"`).
    ///
    /// # Errors
    /// This method does not currently fail, but returns `Result` for API
    /// consistency with the other formatting methods.
    pub fn format_log_level(&self, level: &str) -> Result<String, I18nError> {
        Ok(level.to_uppercase())
    }

    /// Compare two log field names using locale-sensitive collation rules.
    ///
    /// # Errors
    /// This method does not currently fail, but returns `Result` for API
    /// consistency with the other formatting methods.
    pub fn compare_fields(&self, a: &str, b: &str) -> Result<Ordering, I18nError> {
        self.inner
            .compare(a, b)
            .map_err(|e| I18nError::FormatError(e.to_string()))
    }
}
