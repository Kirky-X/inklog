// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Internationalization and locale-aware formatting for log operations.
//!
//! This is a **core feature** — always compiled, no cargo feature gate required.
//!
//! Provides locale-aware number formatting, date formatting, plural rules,
//! and string collation via the `icu` crate (ICU4X 2.x). Useful for
//! generating locale-sensitive log messages (e.g. "1 event" vs "2 events"),
//! formatting log counters, displaying log timestamps, normalizing log
//! levels, and sorting log fields by locale-specific collation rules.
//!
//! Runtime message translation is provided by `fluent-bundle` with `.ftl`
//! translation files. Locale is automatically detected via `sys-locale`
//! and can be overridden with the `INKLOG_LOCALE` environment variable.
//!
//! # Example
//!
//! ```rust,ignore
//! use inklog::i18n::LogI18nFormatter;
//!
//! // Use the current system locale
//! let fmt = LogI18nFormatter::new_default()?;
//!
//! // Or specify a locale explicitly
//! let fmt = LogI18nFormatter::new("en-US")?;
//! let plural = fmt.format_event_count(1)?; // "One"
//! let ts = fmt.format_timestamp(2026, 7, 11)?;
//! let level = fmt.format_log_level("info")?; // "INFO"
//! ```

use icu::collator::CollatorBorrowed;
use icu::decimal::DecimalFormatter;
use icu::locale::Locale;
use icu::plurals::PluralRules;
use thiserror::Error;

mod i18n_impl;
mod locale_manager;

pub use locale_manager::{current_locale, init_locale, tr, tr_args};

/// Errors returned by [`LogI18nFormatter`] operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum I18nError {
    /// BCP-47 locale string could not be parsed.
    #[error("invalid locale '{input}': {reason}")]
    InvalidLocale { input: String, reason: String },
    /// Number value could not be formatted (e.g. NaN, Infinity, or parse failure).
    #[error("invalid number '{input}': {reason}")]
    InvalidNumber { input: String, reason: String },
    /// Date component out of range or otherwise invalid.
    #[error("date error: {0}")]
    DateError(String),
    /// Underlying ICU4X data or formatting failure.
    #[error("formatting error: {0}")]
    FormatError(String),
}

/// Locale-aware formatter backed by ICU4X compiled data.
///
/// Construct with [`LogI18nFormatter::new`] using a BCP-47 locale tag
/// (e.g. `"en-US"`, `"zh-CN"`). All formatters are created eagerly so
/// that repeated formatting calls are allocation-light.
pub struct LogI18nFormatter {
    locale: Locale,
    decimal_formatter: DecimalFormatter,
    plural_rules: PluralRules,
    collator: CollatorBorrowed<'static>,
}

impl std::fmt::Debug for LogI18nFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogI18nFormatter")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_parsing_en() {
        let fmt = LogI18nFormatter::new("en-US");
        assert!(fmt.is_ok(), "en-US should parse successfully");
    }

    #[test]
    fn test_locale_parsing_zh() {
        let fmt = LogI18nFormatter::new("zh-CN");
        assert!(fmt.is_ok(), "zh-CN should parse successfully");
    }

    #[test]
    fn test_invalid_locale() {
        let result = LogI18nFormatter::new("not-a-valid-locale!!!");
        assert!(result.is_err(), "invalid locale should return error");
        match result.err().unwrap() {
            I18nError::InvalidLocale { input, .. } => assert_eq!(input, "not-a-valid-locale!!!"),
            other => panic!("expected InvalidLocale, got {other:?}"),
        }
    }

    #[test]
    fn test_format_event_count() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        assert_eq!(
            fmt.format_event_count(1).expect("plural 1"),
            "One",
            "en: count=1 should be One"
        );
        assert_eq!(
            fmt.format_event_count(2).expect("plural 2"),
            "Other",
            "en: count=2 should be Other"
        );
    }

    #[test]
    fn test_format_number_en() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(1_234_567.89_f64).expect("format number");
        // en-US: thousands separator is comma, decimal separator is period
        assert!(
            result.contains(','),
            "en-US number should contain thousands separator: got '{result}'"
        );
        assert!(
            result.contains('.'),
            "en-US number should contain decimal point: got '{result}'"
        );
    }

    #[test]
    fn test_format_number_not_finite() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        assert!(fmt.format_number(f64::NAN).is_err());
        assert!(fmt.format_number(f64::INFINITY).is_err());
    }

    #[test]
    fn test_format_timestamp() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_timestamp(2026, 7, 11).expect("format timestamp");
        assert!(
            result.contains("2026"),
            "timestamp should contain year: got '{result}'"
        );
        assert!(
            !result.is_empty(),
            "timestamp should be non-empty: got '{result}'"
        );
    }

    #[test]
    fn test_format_log_level() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        assert_eq!(
            fmt.format_log_level("info").expect("log level"),
            "INFO",
            "info should be normalized to INFO"
        );
        assert_eq!(
            fmt.format_log_level("debug").expect("log level"),
            "DEBUG",
            "debug should be normalized to DEBUG"
        );
        assert_eq!(
            fmt.format_log_level("ERROR").expect("log level"),
            "ERROR",
            "ERROR should stay ERROR"
        );
    }

    #[test]
    fn test_compare_fields() {
        let fmt = LogI18nFormatter::new("en").expect("en locale");
        assert_eq!(
            fmt.compare_fields("apple", "banana").expect("compare"),
            std::cmp::Ordering::Less,
            "apple < banana"
        );
        assert_eq!(
            fmt.compare_fields("banana", "apple").expect("compare"),
            std::cmp::Ordering::Greater,
            "banana > apple"
        );
        assert_eq!(
            fmt.compare_fields("apple", "apple").expect("compare"),
            std::cmp::Ordering::Equal,
            "apple == apple"
        );
    }

    #[test]
    fn test_debug_impl() {
        let fmt = LogI18nFormatter::new("en-US").expect("en-US locale");
        let debug_str = format!("{:?}", fmt);
        assert!(debug_str.contains("LogI18nFormatter"));
        assert!(debug_str.contains("locale"));
    }

    #[test]
    fn test_format_event_count_arabic_zero() {
        // Arabic has a "Zero" plural category for count=0
        let fmt = LogI18nFormatter::new("ar").expect("ar locale");
        let result = fmt.format_event_count(0).expect("plural 0");
        assert_eq!(result, "Zero");
    }

    #[test]
    fn test_format_event_count_polish_few() {
        // Polish has a "Few" plural category for count=2,3,4
        let fmt = LogI18nFormatter::new("pl").expect("pl locale");
        let result = fmt.format_event_count(3).expect("plural 3");
        assert_eq!(result, "Few");
    }

    #[test]
    fn test_format_event_count_arabic_two() {
        // Arabic has a "Two" plural category for count=2
        let fmt = LogI18nFormatter::new("ar").expect("ar locale");
        let result = fmt.format_event_count(2).expect("plural 2");
        assert_eq!(result, "Two");
    }

    #[test]
    fn test_format_event_count_arabic_many() {
        // Arabic has a "Many" plural category for count=11..99
        let fmt = LogI18nFormatter::new("ar").expect("ar locale");
        let result = fmt.format_event_count(11).expect("plural 11");
        assert_eq!(result, "Many");
    }

    #[test]
    fn test_new_default() {
        // new_default() should create a formatter using the current system locale
        let current = current_locale();
        let fmt = LogI18nFormatter::new_default();
        assert!(
            fmt.is_ok(),
            "new_default() should succeed for locale '{}', got error: {:?}",
            current,
            fmt.err()
        );
        let fmt = fmt.unwrap();
        let debug_str = format!("{:?}", fmt);
        assert!(
            debug_str.contains(&current),
            "formatter locale should match current_locale '{}', got: {}",
            current,
            debug_str
        );
    }
}
