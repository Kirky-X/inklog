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
    print_separator("inklog 国际化 (i18n) 格式化示例");

    show_locale_creation();
    show_number_formatting();
    show_event_count_plural();
    show_timestamp_formatting();
    show_log_level_normalization();
    show_field_comparison();
    show_error_handling();

    println!("\n所有 i18n 格式化示例展示完毕。");
}

/// 演示多语言格式化器创建
fn show_locale_creation() {
    use std::cmp::Ordering;

    print_section("示例 1：创建多语言格式化器");

    println!("创建 en-US 格式化器：");
    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US locale");
    println!("  ✓ LogI18nFormatter::new(\"en-US\") 创建成功");

    println!("\n创建 zh-CN 格式化器：");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN locale");
    println!("  ✓ LogI18nFormatter::new(\"zh-CN\") 创建成功");

    println!("\n支持 BCP-47 语言标签：");
    println!("  en-US: 英语（美国）");
    println!("  zh-CN: 中文（简体）");
    println!("  ja-JP: 日语（日本）");
    println!("  de-DE: 德语（德国）");

    // 使用格式化器避免编译警告
    let _ = fmt_en.compare_fields("a", "b").ok();
    let _ = fmt_zh.compare_fields("a", "b").ok();
    let _ = Ordering::Equal;
}

/// 演示数字格式化
fn show_number_formatting() {
    print_section("示例 2：数字格式化");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let test_values = [1234.56, 1_234_567.89, 0.001, 999_999.0];

    println!("{:<15} {:<25} {:<25}", "数值", "en-US", "zh-CN");
    println!("{}", "-".repeat(65));

    for &val in &test_values {
        let en_str = fmt_en.format_number(val).unwrap_or_else(|e| e.to_string());
        let zh_str = fmt_zh.format_number(val).unwrap_or_else(|e| e.to_string());
        println!("{:<15} {:<25} {:<25}", val, en_str, zh_str);
    }

    println!("\n说明：");
    println!("  - en-US: 千分位逗号, 小数点");
    println!("  - zh-CN: 千分位可能不同（取决于 ICU 数据）");
    println!("  - 用途：格式化日志中的性能指标、计数器值等");
}

/// 演示事件计数复数形式
fn show_event_count_plural() {
    print_section("示例 3：事件计数复数形式");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let counts = [0, 1, 2, 5, 100, 1000];

    println!("{:<10} {:<15} {:<15}", "Count", "en-US", "zh-CN");
    println!("{}", "-".repeat(40));

    for &count in &counts {
        let en_plural = fmt_en.format_event_count(count).unwrap_or_default();
        let zh_plural = fmt_zh.format_event_count(count).unwrap_or_default();
        println!("{:<10} {:<15} {:<15}", count, en_plural, zh_plural);
    }

    println!("\n用途：生成locale-aware 的日志消息");
    println!("  en: \"One event processed\" / \"Other events processed\"");
    println!("  zh: \"Other 事件已处理\"（中文无复数变化）");
}

/// 演示时间戳格式化
fn show_timestamp_formatting() {
    print_section("示例 4：时间戳格式化");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");
    let fmt_zh = LogI18nFormatter::new("zh-CN").expect("zh-CN");

    let dates = [(2026, 1, 15), (2026, 7, 4), (2026, 12, 25)];

    println!("{:<15} {:<25} {:<25}", "日期", "en-US", "zh-CN");
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

    println!("\n用途：日志时间戳的本地化显示");
    println!("  - 使用 ICU4X DateTimeFormatter 的 medium 格式");
    println!("  - 自动适配目标语言的日期格式习惯");
}

/// 演示日志级别规范化
fn show_log_level_normalization() {
    print_section("示例 5：日志级别规范化");

    let fmt = LogI18nFormatter::new("en-US").expect("en-US");

    let levels = [
        "trace", "debug", "info", "warn", "error", "fatal", "INFO", "Debug",
    ];

    println!("日志级别规范化（统一大写）：");
    println!("{}", "-".repeat(40));
    for level in &levels {
        let normalized = fmt.format_log_level(level).unwrap_or_default();
        println!("  {:<10} → {}", level, normalized);
    }

    println!("\n用途：统一日志级别显示格式");
}

/// 演示字段名区域排序比较
fn show_field_comparison() {
    use std::cmp::Ordering;

    print_section("示例 6：字段名区域排序比较");

    let fmt_en = LogI18nFormatter::new("en-US").expect("en-US");

    let pairs = [
        ("apple", "banana"),
        ("banana", "apple"),
        ("apple", "apple"),
        ("error", "warning"),
        ("debug", "info"),
    ];

    println!("{:<20} {:<20} {:<15}", "字段 A", "字段 B", "排序结果");
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

    println!("\n用途：按字母顺序排列日志字段");
    println!("  - 使用 ICU4X Collator 进行区域敏感排序");
    println!("  - 支持不同语言的排序规则");
}

/// 演示错误处理
fn show_error_handling() {
    print_section("示例 7：错误处理");

    println!("无效 locale 标签：");
    match LogI18nFormatter::new("not-valid!!!") {
        Ok(_) => println!("  意外成功"),
        Err(e) => println!("  ✓ 预期错误: {}", e),
    }

    println!("\n非有限数字格式化：");
    let fmt = LogI18nFormatter::new("en-US").expect("en-US");
    match fmt.format_number(f64::NAN) {
        Ok(_) => println!("  意外成功"),
        Err(e) => println!("  ✓ NaN 错误: {}", e),
    }
    match fmt.format_number(f64::INFINITY) {
        Ok(_) => println!("  意外成功"),
        Err(e) => println!("  ✓ Infinity 错误: {}", e),
    }

    println!("\n错误类型：");
    println!("  I18nError::InvalidLocale  - locale 标签解析失败");
    println!("  I18nError::InvalidNumber  - 非有限数字");
    println!("  I18nError::DateError      - 日期组件无效");
    println!("  I18nError::FormatError    - ICU4X 格式化失败");
}
