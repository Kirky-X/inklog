// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 日志模板示例
//!
//! 演示 LogTemplate 的用法、自定义占位符和不同格式效果。
//!
//! # 内容
//!
//! 1. 默认模板渲染
//! 2. 自定义模板（简洁/详细/JSON 风格）
//! 3. 完整占位符列表
//! 4. 结构化字段（{fields}）渲染
//! 5. 多格式对比
//! 6. 边界场景（空消息、特殊字符、Unicode）
//! 7. 渲染输出验证（assert_eq!）
//! 8. 最佳实践建议
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin template
//! ```

use inklog::tracing::Level;
use inklog::{LogRecord, LogTemplate};
use inklog_examples::common::{tr, tr_args};
use serde_json::Value;
use std::collections::HashMap;

fn main() {
    println!("{}\n", tr("example-template-inklog-log-template-examples"));

    default_template();
    custom_templates();
    full_placeholders();
    fields_placeholder();
    multiple_formats_comparison();
    edge_cases();
    render_verification();
    best_practices();

    println!(
        "\n{}",
        tr("example-template-all-template-examples-completed")
    );
}

/// 示例 1：默认模板
///
/// 使用 `LogTemplate::default()` 渲染标准格式。
fn default_template() {
    println!("{}\n", tr("example-template-example-1-default-template"));

    let template = LogTemplate::default();
    let record = create_sample_record();

    let rendered = template.render(&record);
    println!("Template: {{timestamp}} [{{level}}] {{target}} - {{message}}");
    println!(
        "{}",
        tr_args("example-template-output", &[("rendered", &rendered)])
    );

    assert!(rendered.contains("[INFO]"), "should contain the log level");
    assert!(
        rendered.contains("my_app::module"),
        "should contain the target"
    );
    assert!(
        rendered.contains("user logged in successfully"),
        "should contain the message"
    );
    println!("{}\n", tr("example-template-render-check-passed"));
}

/// 示例 2：自定义模板
///
/// 演示多种自定义模板格式。
fn custom_templates() {
    println!("{}\n", tr("example-template-example-2-custom-templates"));

    let record = create_sample_record();

    // 简洁格式
    let simple = LogTemplate::new("[{level}] {message}");
    let simple_out = simple.render(&record);
    println!("Simple format: [{{level}}] {{message}}");
    println!(
        "  {}",
        tr_args("example-template-output-2", &[("simple_out", &simple_out)])
    );
    assert_eq!(simple_out, "[INFO] user logged in successfully");
    println!("  {}", tr("example-template-exact-match"));

    // 标准格式
    let standard = LogTemplate::new("{timestamp} | {level} | {target} | {message}");
    let standard_out = standard.render(&record);
    println!("\nStandard format: {{timestamp}} | {{level}} | {{target}} | {{message}}");
    println!(
        "  {}",
        tr_args(
            "example-template-output-3",
            &[("standard_out", &standard_out)]
        )
    );
    assert!(standard_out.contains("INFO"));
    assert!(standard_out.contains("my_app::module"));
    println!("  {}", tr("example-template-contains-key-fields"));

    // 详细格式（含文件和行号）
    let detailed = LogTemplate::new(
        "[{timestamp}] [{level}] [{thread_id}] {target} - {message} ({file}:{line})",
    );
    let detailed_out = detailed.render(&record);
    println!(
        "\nDetailed format: [{{timestamp}}] [{{level}}] [{{thread_id}}] {{target}} - {{message}} ({{file}}:{{line}})"
    );
    println!(
        "  {}",
        tr_args(
            "example-template-output-4",
            &[("detailed_out", &detailed_out)]
        )
    );
    assert!(
        detailed_out.contains("src/main.rs"),
        "should contain the file name"
    );
    assert!(
        detailed_out.contains("42"),
        "should contain the line number"
    );
    println!("  {}\n", tr("example-template-file-and-line-are-correct"));
}

/// 示例 3：完整占位符列表
///
/// 展示所有可用占位符及其渲染效果。
fn full_placeholders() {
    println!(
        "{}\n",
        tr("example-template-example-3-full-placeholder-list")
    );

    let record = create_sample_record();

    let placeholders = vec![
        ("{timestamp}", "log timestamp"),
        ("{level}", "log level (TRACE/DEBUG/INFO/WARN/ERROR)"),
        ("{target}", "log target (module path)"),
        ("{message}", "log message content"),
        ("{thread_id}", "thread ID"),
        ("{file}", "source file name (if available)"),
        ("{line}", "source file line (if available)"),
        ("{fields}", "structured fields (JSON format)"),
    ];

    println!(
        "{}",
        tr_args(
            "example-template-rendered-output",
            &[
                ("value", &format!("{:<15}", "placeholder")),
                ("value2", &format!("{:<30}", "description"))
            ]
        )
    );
    println!("{}", "-".repeat(75));

    for (placeholder, desc) in placeholders {
        let tpl = LogTemplate::new(placeholder);
        let rendered = tpl.render(&record);
        println!("{:<15} {:<30} {}", placeholder, desc, rendered);
    }
    println!();
}

/// 示例 4：结构化字段渲染
///
/// 演示 {fields} 占位符输出结构化字段。
fn fields_placeholder() {
    println!("{}\n", tr("example-template-example-4-structured-field"));

    let mut record = create_sample_record();
    record.fields = HashMap::from([
        ("user_id".to_string(), Value::Number(12345.into())),
        (
            "action".to_string(),
            Value::String("user_login".to_string()),
        ),
        ("success".to_string(), Value::Bool(true)),
    ]);

    // 消息 + 字段
    let tpl = LogTemplate::new("{message} | {fields}");
    let rendered = tpl.render(&record);
    println!("Template: {{message}} | {{fields}}");
    println!(
        "{}",
        tr_args("example-template-output", &[("rendered", &rendered)])
    );
    assert!(
        rendered.contains("user logged in successfully"),
        "should contain the message"
    );
    assert!(
        rendered.contains("user_id"),
        "should contain the user_id field"
    );
    assert!(
        rendered.contains("user_login"),
        "should contain the action value"
    );
    println!("{}", tr("example-template-field-rendering-check-passed"));

    // 仅字段
    let tpl2 = LogTemplate::new("{fields}");
    let fields_only = tpl2.render(&record);
    println!(
        "\n{}",
        tr_args(
            "example-template-fields-only",
            &[("fields_only", &fields_only)]
        )
    );
    assert!(
        fields_only.contains("success"),
        "should contain the success field"
    );
    println!("{}\n", tr("example-template-fields-only-rendering-check"));
}

/// 示例 5：多格式对比
///
/// 同一日志记录用不同模板渲染的效果对比。
fn multiple_formats_comparison() {
    println!("{}\n", tr("example-template-example-5-multi-format"));

    let record = create_sample_record();

    let formats = vec![
        ("simple", "[{level}] {message}"),
        ("standard", "{timestamp} [{level}] {target} - {message}"),
        (
            "detailed",
            "[{timestamp}] [{level}] [{thread_id}] {file}:{line} - {message}",
        ),
        (
            "JSON style",
            r#"{{"time":"{timestamp}","level":"{level}","msg":"{message}"}}"#,
        ),
        (
            "pipe separated",
            "{timestamp} | {level} | {target} | {message}",
        ),
        ("bordered", "│ {level:<5} │ {target} │ {message}"),
    ];

    println!(
        "{}",
        tr_args(
            "example-template-rendered-output-2",
            &[("value", &format!("{:<12}", "format name"))]
        )
    );
    println!("{}", "-".repeat(80));
    for (name, format) in formats {
        let tpl = LogTemplate::new(format);
        println!("{:<12} {}", name, tpl.render(&record));
    }
    println!();
}

/// 示例 6：边界场景
///
/// 测试空消息、特殊字符、Unicode 等边界情况。
fn edge_cases() {
    println!("{}\n", tr("example-template-example-6-edge-cases"));

    let tpl = LogTemplate::new("[{level}] {message}");

    // 空消息
    let empty = LogRecord::new(Level::INFO, "test".to_string(), String::new());
    let empty_out = tpl.render(&empty);
    println!(
        "{}",
        tr_args(
            "example-template-empty-message",
            &[("empty_out", &empty_out)]
        )
    );
    assert_eq!(empty_out, "[INFO] ");
    println!(
        "  {}",
        tr("example-template-empty-message-handled-correctly")
    );

    // 特殊字符
    let special = LogRecord::new(
        Level::WARN,
        "test".to_string(),
        "path: C:\\Users\\admin\nnewline\ttab".to_string(),
    );
    let special_out = tpl.render(&special);
    println!(
        "\n{}",
        tr_args(
            "example-template-special-characters",
            &[("special_out", &special_out)]
        )
    );
    assert!(
        special_out.contains("C:\\Users\\admin"),
        "should keep backslashes"
    );
    println!("  {}", tr("example-template-special-characters-preserved"));

    // Unicode
    let unicode = LogRecord::new(
        Level::INFO,
        "test".to_string(),
        "日本語テスト 🎉 Привет мир".to_string(),
    );
    let unicode_out = tpl.render(&unicode);
    println!(
        "\n{}",
        tr_args("example-template-unicode", &[("unicode_out", &unicode_out)])
    );
    assert!(unicode_out.contains("日本語"), "should keep Japanese text");
    assert!(unicode_out.contains("🎉"), "should keep the emoji");
    println!("  {}", tr("example-template-unicode-handled-correctly"));

    // 超长消息
    let long_msg = "x".repeat(1000);
    let long_record = LogRecord::new(Level::INFO, "test".to_string(), long_msg.clone());
    let long_out = tpl.render(&long_record);
    assert_eq!(long_out, format!("[INFO] {}", long_msg));
    println!(
        "\n{}",
        tr_args(
            "example-template-long-message-chars-chars",
            &[("value", &(long_msg.len())), ("value2", &(long_out.len()))]
        )
    );
    println!(
        "  {}\n",
        tr("example-template-long-message-handled-correctly")
    );
}

/// 示例 7：渲染输出验证
///
/// 使用 assert_eq! 精确验证渲染结果。
fn render_verification() {
    println!("{}\n", tr("example-template-example-7-render-output"));

    let record = LogRecord::new(
        Level::ERROR,
        "payment".to_string(),
        "payment failed".to_string(),
    );

    // 精确匹配验证
    let tpl1 = LogTemplate::new("[{level}] {message}");
    assert_eq!(tpl1.render(&record), "[ERROR] payment failed");
    println!(
        "{}",
        tr("example-template-error-payment-failed-exact-match")
    );

    // 包含验证
    let tpl2 = LogTemplate::new("{timestamp} [{level}] {target}: {message}");
    let out = tpl2.render(&record);
    assert!(out.contains("ERROR"), "should contain the level");
    assert!(out.contains("payment"), "should contain the target");
    assert!(out.contains("payment failed"), "should contain the message");
    println!(
        "{}",
        tr_args("example-template-containment-verified", &[("out", &out)])
    );

    // 不同级别渲染一致性
    let levels = [
        Level::TRACE,
        Level::DEBUG,
        Level::INFO,
        Level::WARN,
        Level::ERROR,
    ];
    let tpl = LogTemplate::new("[{level}]");
    for level in &levels {
        let r = LogRecord::new(*level, "test".to_string(), "msg".to_string());
        let out = tpl.render(&r);
        let expected = format!("[{}]", level.to_string().to_uppercase());
        assert_eq!(out, expected, "level {} renders inconsistently", level);
    }
    println!(
        "{}\n",
        tr("example-template-all-5-levels-render-consistently")
    );
}

/// 最佳实践建议
fn best_practices() {
    println!("{}\n", tr("example-template-best-practices"));

    println!("{}", tr("example-template-1-template-selection-guide"));
    println!("   Development: [{{level}}] {{message}} — short and fast");
    println!("   Production: {{timestamp}} [{{level}}] {{target}} - {{message}} — complete");
    println!(
        "   Debugging: [{{timestamp}}] [{{level}}] {{file}}:{{line}} - {{message}} — points at source"
    );
    println!(
        "   JSON collection: {{\"ts\":\"{{timestamp}}\",\"lvl\":\"{{level}}\",\"msg\":\"{{message}}\"}} — structured"
    );

    println!("\n{}", tr("example-template-2-performance-notes"));
    println!(
        "   - LogTemplate::new() parses placeholders at construction; render() only concatenates strings"
    );
    println!("   {}", tr("example-template-create-one-template-instance"));
    println!("   - {{fields}} rendering involves JSON serialization; use with care at high rates");

    println!("\n{}", tr("example-template-3-placeholder-notes"));
    println!("   - Unknown placeholders (such as {{unknown}}) render as an empty string");
    println!("   - {{file}} and {{line}} render as empty when source information is unavailable");
    println!("   - Double braces {{}} escape a literal brace (common in JSON formats)");

    println!("\n{}", tr("example-template-4-integration-with-tracing"));
    println!(
        "   {}",
        tr("example-template-logtemplate-is-used-mainly-to")
    );
    println!("   {}", tr("example-template-the-tracing-output-format-is"));
    println!("   {}", tr("example-template-they-work-together-tracing"));
}

/// 创建示例日志记录
fn create_sample_record() -> LogRecord {
    let mut record = LogRecord::new(
        Level::INFO,
        "my_app::module".to_string(),
        "user logged in successfully".to_string(),
    );
    record.file = Some("src/main.rs".to_string());
    record.line = Some(42);
    record
}
