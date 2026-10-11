// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 日志内容净化示例（Layer 0 零依赖）
//!
//! 演示 LogSanitizer、SanitizerConfig、EscapeMode 的使用，覆盖敏感数据脱敏、
//! 严格转义、JSON 安全转义、长度限制与换行转义。
//!
//! 注意：EscapeMode::Strict 实际行为是将控制字符、反斜杠、双引号转义为
//! `\uXXXX` 形式（unicode 转义），并非 HTML 实体转义。本示例按真实行为演示。
//!
//! # 运行
//! ```bash
//! cargo run --bin log_sanitizer
//! ```

use inklog::{EscapeMode, LogSanitizer, SanitizerConfig};
use inklog_examples::common::{print_section, print_separator};
use regex::Regex;

fn main() {
    println!("=== inklog log sanitizer examples ===\n");

    show_default_sensitive_redaction();
    show_custom_sensitive_patterns();
    show_strict_escape_mode();
    show_json_safe_escape_mode();
    show_minimal_escape_mode();
    show_length_limit();
    show_newline_escaping();
    show_custom_replacement();

    println!("\n✓ all log sanitizer examples completed");
}

/// 展示默认敏感数据脱敏（邮箱、密码、Token、API Key、卡号等）
fn show_default_sensitive_redaction() {
    print_separator("1. Default sensitive data masking");
    let sanitizer = LogSanitizer::new();

    print_section("1.1 Email masking");
    let input = "contact email: user@example.com";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[EMAIL]"));
    assert!(!output.contains("user@example.com"));

    print_section("1.2 Password masking");
    let input = "User password=secret123";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("secret123"));

    print_section("1.3 API key masking");
    let input = "api_key=sk-DEMO-1234567890";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("sk-DEMO-"));

    print_section("1.4 Bank card number masking");
    let input = "card number: 6222021234567890";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[CARD_NUM]"));

    print_section("1.5 Bearer token masking");
    let input = "Authorization: Bearer abc.def.ghi";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[TOKEN]"));
}

/// 展示自定义敏感模式（手机号、身份证号）
fn show_custom_sensitive_patterns() {
    print_separator("2. Custom sensitive patterns (phone, national ID)");
    let mut sanitizer = LogSanitizer::new();

    // 添加手机号脱敏模式（11 位中国大陆手机号）
    // 注意：必须使用 \b 词边界，避免匹配身份证号内部的 11 位子串
    let phone_re = Regex::new(r"\b1[3-9]\d{9}\b").expect("the phone regex is valid");
    sanitizer.add_pattern(phone_re, "[PHONE]".to_string());

    // 添加 18 位身份证号脱敏模式
    let id_re = Regex::new(
        r"\b[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b",
    )
    .expect("the ID regex is valid");
    sanitizer.add_pattern(id_re, "[ID_CARD]".to_string());

    print_section("2.1 Phone number masking");
    let input = "contact phone: 13812345678";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[PHONE]"));
    assert!(!output.contains("13812345678"));

    print_section("2.2 National ID masking");
    let input = "national ID: 110101199001011234";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("[ID_CARD]"));
    assert!(!output.contains("110101199001011234"));
}

/// 展示 EscapeMode::Strict 严格转义（控制字符与特殊字符转 \uXXXX）
fn show_strict_escape_mode() {
    print_separator("3. EscapeMode::Strict escaping");
    let config = SanitizerConfig {
        mode: EscapeMode::Strict,
        custom_replacements: vec![],
        ..Default::default()
    };
    let sanitizer = LogSanitizer::with_config(config);

    print_section("3.1 Control and special character escaping");
    let input = "Hello\nWorld\\path\"quote";
    let output = sanitizer.sanitize(input);
    println!("Raw: {:?}", input);
    println!("Sanitized: {}", output);
    // Strict 模式：\n → \u000a，\\ → \u005c，\" → \u0022
    assert!(output.contains("\\u000a"));
    assert!(output.contains("\\u005c"));
    assert!(output.contains("\\u0022"));
    assert!(!output.contains('\n'));

    print_section("3.2 Printable characters are unchanged");
    let input = "Hello World 123";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert_eq!(output, "Hello World 123");
}

/// 展示 EscapeMode::JsonSafe JSON 安全转义
fn show_json_safe_escape_mode() {
    print_separator("4. EscapeMode::JsonSafe JSON escaping");
    let config = SanitizerConfig {
        mode: EscapeMode::JsonSafe,
        custom_replacements: vec![],
        ..Default::default()
    };
    let sanitizer = LogSanitizer::with_config(config);

    let input = "quote\"backslash\\newline\ntab\t";
    let output = sanitizer.sanitize(input);
    println!("Raw: {:?}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("\\\""));
    assert!(output.contains("\\\\"));
    assert!(output.contains("\\n"));
    assert!(output.contains("\\t"));
    assert!(!output.contains('\n'));
    assert!(!output.contains('\t'));
}

/// 展示 EscapeMode::Minimal（默认）最小转义
fn show_minimal_escape_mode() {
    print_separator("5. EscapeMode::Minimal escaping (default)");
    let config = SanitizerConfig {
        mode: EscapeMode::Minimal,
        custom_replacements: vec![],
        ..Default::default()
    };
    let sanitizer = LogSanitizer::with_config(config);

    let input = "line1\nline2\r\ttabbed\x00null";
    let output = sanitizer.sanitize(input);
    println!("Raw: {:?}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("\\n"));
    assert!(output.contains("\\r"));
    assert!(output.contains("\\t"));
    assert!(output.contains("\\x00"));
    assert!(!output.contains('\n'));
    assert!(!output.contains('\r'));
    assert!(!output.contains('\t'));
}

/// 展示长度限制
fn show_length_limit() {
    print_separator("6. Length limiting (max_length)");
    let config = SanitizerConfig {
        max_length: 10,
        ..Default::default()
    };
    let sanitizer = LogSanitizer::with_config(config);

    let input = "This is a very long message that should be truncated";
    let output = sanitizer.sanitize(input);
    println!("Raw ({} bytes): {}", input.len(), input);
    println!("Sanitized ({} bytes): {}", output.len(), output);
    assert!(output.contains("...[truncated]"));
    assert!(output.len() <= 10 + "...[truncated]".len());
}

/// 展示换行转义（默认配置的 custom_replacements）
fn show_newline_escaping() {
    print_separator("7. Newline escaping (default custom_replacements)");
    let sanitizer = LogSanitizer::new();

    let input = "line1\nline2\rline3\r\nline4";
    let output = sanitizer.sanitize(input);
    println!("Raw: {:?}", input);
    println!("Sanitized: {}", output);
    assert!(!output.contains('\n'));
    assert!(!output.contains('\r'));
}

/// 展示自定义字符串替换
fn show_custom_replacement() {
    print_separator("8. Custom string replacement (add_replacement)");
    let mut sanitizer = LogSanitizer::new();
    sanitizer.add_replacement("internal".to_string(), "external".to_string());

    let input = "call internal line";
    let output = sanitizer.sanitize(input);
    println!("Raw: {}", input);
    println!("Sanitized: {}", output);
    assert!(output.contains("external"));
    assert!(!output.contains("internal"));
}
