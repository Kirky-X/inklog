// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 国际化 (i18n) 格式化示例
//!
//! 演示 `inklog::i18n::LogI18nFormatter` 的使用：
//!
//! 1. 创建多语言格式化器（en-US / zh-CN）
//! 2. 数字格式化（千分位、小数点）
//! 3. 事件计数复数形式（"One" / "Other"）
//! 4. 日期时间戳格式化
//! 5. 日志级别规范化
//! 6. 字段名区域排序比较
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin i18n
//! ```

use inklog::i18n::LogI18nFormatter;
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog internationalization (i18n) formatting examples");

    show_locale_creation();
    show_number_formatting();
    show_event_count_plural();
    show_timestamp_formatting();
    show_log_level_normalization();
    show_field_comparison();
    show_error_handling();

    println!("\nAll i18n formatting examples shown.");
}

/// 演示多语言格式化器创建
fn show_locale_creation() {
    print_section("Example 1: create multi-locale formatters");

    println!("Create the en-US formatter:");
    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US locale");
    println!("  ✓ LogI18nFormatter::new(\"en-US\") created");

    println!("\nCreate the zh-CN formatter:");
    let _fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN locale");
    println!("  ✓ LogI18nFormatter::new(\"zh-CN\") created");

    println!("\nSupported BCP-47 language tags:");
    println!("  en-US: English (United States)");
    println!("  zh-CN: Chinese (Simplified)");
    println!("  ja-JP: Japanese (Japan)");
    println!("  de-DE: German (Germany)");

    // 使用格式化器演示字段比较
    match fmt_en.compare_fields("a", "b") {
        Ok(std::cmp::Ordering::Less) => println!("\nField comparison example: \"a\" < \"b\""),
        Ok(other) => println!("\nField comparison example: {:?}", other),
        Err(e) => println!("\nField comparison failed: {}", e),
    }
}

/// 演示数字格式化
fn show_number_formatting() {
    print_section("Example 2: number formatting");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let test_values = [1234.56, 1_234_567.89, 0.001, 999_999.0];

    println!("{:<15} {:<25} {:<25}", "value", "en-US", "zh-CN");
    println!("{}", "-".repeat(65));

    for &val in &test_values {
        let en_str = fmt_en.format_number(val).unwrap_or_else(|e| e.to_string());
        let zh_str = fmt_zh.format_number(val).unwrap_or_else(|e| e.to_string());
        println!("{:<15} {:<25} {:<25}", val, en_str, zh_str);
    }

    println!("\nNotes:");
    println!("  - en-US: comma thousands separator, dot decimal point");
    println!("  - zh-CN: the separator may differ (depends on ICU data)");
    println!("  - Use case: format performance metrics and counters in logs");
}

/// 演示事件计数复数形式
fn show_event_count_plural() {
    print_section("Example 3: event count plurals");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let counts = [0, 1, 2, 5, 100, 1000];

    println!("{:<10} {:<15} {:<15}", "Count", "en-US", "zh-CN");
    println!("{}", "-".repeat(40));

    for &count in &counts {
        let en_plural = fmt_en
            .format_event_count(count)
            .unwrap_or_else(|e| format!("error: {e}"));
        let zh_plural = fmt_zh
            .format_event_count(count)
            .unwrap_or_else(|e| format!("error: {e}"));
        println!("{:<10} {:<15} {:<15}", count, en_plural, zh_plural);
    }

    println!("\nUse case: generate locale-aware log messages");
    println!("  en: \"One event processed\" / \"Other events processed\"");
    println!("  zh: \"Other 事件已处理\" (Chinese has no plural inflection)");
}

/// 演示时间戳格式化
fn show_timestamp_formatting() {
    print_section("Example 4: timestamp formatting");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let dates = [(2026, 1, 15), (2026, 7, 4), (2026, 12, 25)];

    println!("{:<15} {:<25} {:<25}", "date", "en-US", "zh-CN");
    println!("{}", "-".repeat(65));

    for (y, m, d) in &dates {
        let en_ts = fmt_en
            .format_timestamp(*y, *m, *d)
            .unwrap_or_else(|e| e.to_string());
        let zh_ts = fmt_zh
            .format_timestamp(*y, *m, *d)
            .unwrap_or_else(|e| e.to_string());
        println!("{:<4}-{:02}-{:02}  {:<25} {:<25}", y, m, d, en_ts, zh_ts);
    }

    println!("\nUse case: localized display of log timestamps");
    println!("  - Use the ICU4X DateTimeFormatter medium format");
    println!("  - Adapts automatically to the date conventions of the target language");
}

/// 演示日志级别规范化
fn show_log_level_normalization() {
    print_section("Example 5: log level normalization");

    let fmt = LogI18nFormatter::new("en-US").expect("en-US");

    let levels = [
        "trace", "debug", "info", "warn", "error", "fatal", "INFO", "Debug",
    ];

    println!("Log level normalization (unified upper case):");
    println!("{}", "-".repeat(40));
    for level in &levels {
        let normalized = fmt
            .format_log_level(level)
            .unwrap_or_else(|e| format!("error: {e}"));
        println!("  {:<10} → {}", level, normalized);
    }

    println!("\nUse case: unify the displayed log level format");
}

/// 演示字段名区域排序比较
fn show_field_comparison() {
    use std::cmp::Ordering;

    print_section("Example 6: locale-sensitive field name comparison");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");

    let pairs = [
        ("apple", "banana"),
        ("banana", "apple"),
        ("apple", "apple"),
        ("error", "warning"),
        ("debug", "info"),
    ];

    println!("{:<20} {:<20} {:<15}", "field A", "field B", "order");
    println!("{}", "-".repeat(55));

    for (a, b) in &pairs {
        let result = fmt_en.compare_fields(a, b).unwrap_or(Ordering::Equal);
        let result_str = match result {
            Ordering::Less => "Less (<)",
            Ordering::Greater => "Greater (>)",
            Ordering::Equal => "Equal (==)",
        };
        println!("{:<20} {:<20} {:<15}", a, b, result_str);
    }

    println!("\nUse case: order log fields alphabetically");
    println!("  - Use the ICU4X Collator for locale-sensitive ordering");
    println!("  - Supports the ordering rules of different languages");
}

/// 演示错误处理
fn show_error_handling() {
    print_section("Example 7: error handling");

    println!("Invalid locale tag:");
    match LogI18nFormatter::new("not-valid!!!") {
        Ok(_) => println!("  unexpected success"),
        Err(e) => println!("  ✓ expected error: {}", e),
    }

    println!("\nNon-finite number formatting:");
    let fmt = LogI18nFormatter::new("en-US").expect("en-US");
    match fmt.format_number(f64::NAN) {
        Ok(_) => println!("  unexpected success"),
        Err(e) => println!("  ✓ NaN error: {}", e),
    }
    match fmt.format_number(f64::INFINITY) {
        Ok(_) => println!("  unexpected success"),
        Err(e) => println!("  ✓ Infinity error: {}", e),
    }

    println!("\nError types:");
    println!("  I18nError::InvalidLocale  - the locale tag failed to parse");
    println!("  I18nError::InvalidNumber  - a non-finite number");
    println!("  I18nError::DateError      - an invalid date component");
    println!("  I18nError::FormatError    - ICU4X formatting failed");
}
