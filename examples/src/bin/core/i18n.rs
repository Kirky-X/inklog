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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr("example-i18n-inklog-internationalization"));

    show_locale_creation();
    show_number_formatting();
    show_event_count_plural();
    show_timestamp_formatting();
    show_log_level_normalization();
    show_field_comparison();
    show_error_handling();

    println!("\n{}", tr("example-i18n-all-i18n-formatting-examples"));
}

/// 演示多语言格式化器创建
fn show_locale_creation() {
    print_section(&tr("example-i18n-example-1-create-multi-locale"));

    println!("{}", tr("example-i18n-create-the-en-us-formatter"));
    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US locale");
    println!("  {}", tr("example-i18n-logi18nformatter-new-en-us"));

    println!("\n{}", tr("example-i18n-create-the-zh-cn-formatter"));
    let _fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN locale");
    println!("  {}", tr("example-i18n-logi18nformatter-new-zh-cn"));

    println!("\n{}", tr("example-i18n-supported-bcp-47-language-tags"));
    println!("  {}", tr("example-i18n-en-us-english-united-states"));
    println!("  {}", tr("example-i18n-zh-cn-chinese-simplified"));
    println!("  {}", tr("example-i18n-ja-jp-japanese-japan"));
    println!("  {}", tr("example-i18n-de-de-german-germany"));

    // 使用格式化器演示字段比较
    match fmt_en.compare_fields("a", "b") {
        Ok(std::cmp::Ordering::Less) => {
            println!("\n{}", tr("example-i18n-field-comparison-example-a-b"))
        }
        Ok(other) => println!(
            "\n{}",
            tr_args(
                "example-i18n-field-comparison-example",
                &[("other", &format!("{:?}", other))]
            )
        ),
        Err(e) => println!(
            "\n{}",
            tr_args("example-i18n-field-comparison-failed", &[("e", &e)])
        ),
    }
}

/// 演示数字格式化
fn show_number_formatting() {
    print_section(&tr("example-i18n-example-2-number-formatting"));

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

    println!("\n{}", tr("example-i18n-notes"));
    println!("  {}", tr("example-i18n-en-us-comma-thousands-separator"));
    println!("  {}", tr("example-i18n-zh-cn-the-separator-may-differ"));
    println!("  {}", tr("example-i18n-use-case-format-performance"));
}

/// 演示事件计数复数形式
fn show_event_count_plural() {
    print_section(&tr("example-i18n-example-3-event-count-plurals"));

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

    println!("\n{}", tr("example-i18n-use-case-generate-locale-aware"));
    println!("  {}", tr("example-i18n-en-one-event-processed-other"));
    println!("  {}", tr("example-i18n-zh-other-chinese-has-no-plural"));
}

/// 演示时间戳格式化
fn show_timestamp_formatting() {
    print_section(&tr("example-i18n-example-4-timestamp-formatting"));

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

    println!("\n{}", tr("example-i18n-use-case-localized-display-of"));
    println!("  {}", tr("example-i18n-use-the-icu4x-datetimeformatter"));
    println!("  {}", tr("example-i18n-adapts-automatically-to-the"));
}

/// 演示日志级别规范化
fn show_log_level_normalization() {
    print_section(&tr("example-i18n-example-5-log-level"));

    let fmt = LogI18nFormatter::new("en-US").expect("en-US");

    let levels = [
        "trace", "debug", "info", "warn", "error", "fatal", "INFO", "Debug",
    ];

    println!("{}", tr("example-i18n-log-level-normalization-unified"));
    println!("{}", "-".repeat(40));
    for level in &levels {
        let normalized = fmt
            .format_log_level(level)
            .unwrap_or_else(|e| format!("error: {e}"));
        println!("  {:<10} → {}", level, normalized);
    }

    println!("\n{}", tr("example-i18n-use-case-unify-the-displayed"));
}

/// 演示字段名区域排序比较
fn show_field_comparison() {
    use std::cmp::Ordering;

    print_section(&tr("example-i18n-example-6-locale-sensitive"));

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

    println!("\n{}", tr("example-i18n-use-case-order-log-fields"));
    println!("  {}", tr("example-i18n-use-the-icu4x-collator-for"));
    println!("  {}", tr("example-i18n-supports-the-ordering-rules-of"));
}

/// 演示错误处理
fn show_error_handling() {
    print_section(&tr("example-i18n-example-7-error-handling"));

    println!("{}", tr("example-i18n-invalid-locale-tag"));
    match LogI18nFormatter::new("not-valid!!!") {
        Ok(_) => println!("  {}", tr("example-i18n-unexpected-success")),
        Err(e) => println!("  {}", tr_args("example-i18n-expected-error", &[("e", &e)])),
    }

    println!("\n{}", tr("example-i18n-non-finite-number-formatting"));
    let fmt = LogI18nFormatter::new("en-US").expect("en-US");
    match fmt.format_number(f64::NAN) {
        Ok(_) => println!("  {}", tr("example-i18n-unexpected-success")),
        Err(e) => println!("  {}", tr_args("example-i18n-nan-error", &[("e", &e)])),
    }
    match fmt.format_number(f64::INFINITY) {
        Ok(_) => println!("  {}", tr("example-i18n-unexpected-success")),
        Err(e) => println!("  {}", tr_args("example-i18n-infinity-error", &[("e", &e)])),
    }

    println!("\n{}", tr("example-i18n-error-types"));
    println!("  {}", tr("example-i18n-i18nerror-invalidlocale-the"));
    println!("  {}", tr("example-i18n-i18nerror-invalidnumber-a-non"));
    println!("  {}", tr("example-i18n-i18nerror-dateerror-an-invalid"));
    println!("  {}", tr("example-i18n-i18nerror-formaterror-icu4x"));
}
