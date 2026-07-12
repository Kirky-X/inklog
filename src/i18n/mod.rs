// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! ICU4X-backed internationalization formatting for log operations.
//!
//! Provides locale-aware number formatting, date formatting, plural rules,
//! and string collation via the `icu` crate (ICU4X 2.x). Useful for
//! generating locale-sensitive log messages (e.g. "1 event" vs "2 events"),
//! formatting log counters, displaying log timestamps, normalizing log
//! levels, and sorting log fields by locale-specific collation rules.
//!
//! Enable with the `i18n` cargo feature:
//! ```toml
//! [dependencies]
//! inklog = { version = "...", features = ["i18n"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use inklog::i18n::LogI18nFormatter;
//!
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

/// Errors returned by [`LogI18nFormatter`] operations.
#[derive(Debug, Error)]
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
}
