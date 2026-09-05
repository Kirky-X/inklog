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
use serde_json::Value;
use std::collections::HashMap;

fn main() {
    println!("=== inklog 日志模板示例 ===\n");

    default_template();
    custom_templates();
    full_placeholders();
    fields_placeholder();
    multiple_formats_comparison();
    edge_cases();
    render_verification();
    best_practices();

    println!("\n所有模板示例演示完毕。");
}

/// 示例 1：默认模板
///
/// 使用 `LogTemplate::default()` 渲染标准格式。
fn default_template() {
    println!("--- 示例 1：默认模板 ---\n");

    let template = LogTemplate::default();
    let record = create_sample_record();

    let rendered = template.render(&record);
    println!("模板: {{timestamp}} [{{level}}] {{target}} - {{message}}");
    println!("输出: {}", rendered);

    // 验证渲染结果
    assert!(rendered.contains("[INFO]"), "应包含日志级别");
    assert!(rendered.contains("my_app::module"), "应包含 target");
    assert!(rendered.contains("用户登录成功"), "应包含消息");
    println!("✓ 渲染验证通过\n");
}

/// 示例 2：自定义模板
///
/// 演示多种自定义模板格式。
fn custom_templates() {
    println!("--- 示例 2：自定义模板 ---\n");

    let record = create_sample_record();

    // 简洁格式
    let simple = LogTemplate::new("[{level}] {message}");
    let simple_out = simple.render(&record);
    println!("简洁格式: [{{level}}] {{message}}");
    println!("  输出: {}", simple_out);
    assert_eq!(simple_out, "[INFO] 用户登录成功");
    println!("  ✓ 精确匹配");

    // 标准格式
    let standard = LogTemplate::new("{timestamp} | {level} | {target} | {message}");
    let standard_out = standard.render(&record);
    println!("\n标准格式: {{timestamp}} | {{level}} | {{target}} | {{message}}");
    println!("  输出: {}", standard_out);
    assert!(standard_out.contains("INFO"));
    assert!(standard_out.contains("my_app::module"));
    println!("  ✓ 包含关键字段");

    // 详细格式（含文件和行号）
    let detailed = LogTemplate::new(
        "[{timestamp}] [{level}] [{thread_id}] {target} - {message} ({file}:{line})",
    );
    let detailed_out = detailed.render(&record);
    println!(
        "\n详细格式: [{{timestamp}}] [{{level}}] [{{thread_id}}] {{target}} - {{message}} ({{file}}:{{line}})"
    );
    println!("  输出: {}", detailed_out);
    assert!(detailed_out.contains("src/main.rs"), "应包含文件名");
    assert!(detailed_out.contains("42"), "应包含行号");
    println!("  ✓ 文件和行号正确\n");
}

/// 示例 3：完整占位符列表
///
/// 展示所有可用占位符及其渲染效果。
fn full_placeholders() {
    println!("--- 示例 3：完整占位符列表 ---\n");

    let record = create_sample_record();

    let placeholders = vec![
        ("{timestamp}", "日志时间戳"),
        ("{level}", "日志级别（TRACE/DEBUG/INFO/WARN/ERROR）"),
        ("{target}", "日志目标（模块路径）"),
        ("{message}", "日志消息内容"),
        ("{thread_id}", "线程 ID"),
        ("{file}", "源文件名（如有）"),
        ("{line}", "源文件行号（如有）"),
        ("{fields}", "结构化字段（JSON 格式）"),
    ];

    println!("{:<15} {:<30} 渲染结果", "占位符", "说明");
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
    println!("--- 示例 4：结构化字段渲染 ---\n");

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
    println!("模板: {{message}} | {{fields}}");
    println!("输出: {}", rendered);
    assert!(rendered.contains("用户登录成功"), "应包含消息");
    assert!(rendered.contains("user_id"), "应包含 user_id 字段");
    assert!(rendered.contains("user_login"), "应包含 action 值");
    println!("✓ 字段渲染验证通过");

    // 仅字段
    let tpl2 = LogTemplate::new("{fields}");
    let fields_only = tpl2.render(&record);
    println!("\n仅字段: {}", fields_only);
    assert!(fields_only.contains("success"), "应包含 success 字段");
    println!("✓ 纯字段渲染验证通过\n");
}

/// 示例 5：多格式对比
///
/// 同一日志记录用不同模板渲染的效果对比。
fn multiple_formats_comparison() {
    println!("--- 示例 5：多格式对比 ---\n");

    let record = create_sample_record();

    let formats = vec![
        ("简洁格式", "[{level}] {message}"),
        ("标准格式", "{timestamp} [{level}] {target} - {message}"),
        (
            "详细格式",
            "[{timestamp}] [{level}] [{thread_id}] {file}:{line} - {message}",
        ),
        (
            "JSON 风格",
            r#"{{"time":"{timestamp}","level":"{level}","msg":"{message}"}}"#,
        ),
        ("管道分隔", "{timestamp} | {level} | {target} | {message}"),
        ("带边框", "│ {level:<5} │ {target} │ {message}"),
    ];

    println!("{:<12} 渲染结果", "格式名称");
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
    println!("--- 示例 6：边界场景 ---\n");

    let tpl = LogTemplate::new("[{level}] {message}");

    // 空消息
    let empty = LogRecord::new(Level::INFO, "test".to_string(), String::new());
    let empty_out = tpl.render(&empty);
    println!("空消息:   '{}'", empty_out);
    assert_eq!(empty_out, "[INFO] ");
    println!("  ✓ 空消息处理正确");

    // 特殊字符
    let special = LogRecord::new(
        Level::WARN,
        "test".to_string(),
        "路径: C:\\Users\\admin\n换行\t制表符".to_string(),
    );
    let special_out = tpl.render(&special);
    println!("\n特殊字符: '{}'", special_out);
    assert!(special_out.contains("C:\\Users\\admin"), "应保留反斜杠");
    println!("  ✓ 特殊字符保留完整");

    // Unicode
    let unicode = LogRecord::new(
        Level::INFO,
        "test".to_string(),
        "日本語テスト 🎉 Привет мир".to_string(),
    );
    let unicode_out = tpl.render(&unicode);
    println!("\nUnicode:  '{}'", unicode_out);
    assert!(unicode_out.contains("日本語"), "应保留日文");
    assert!(unicode_out.contains("🎉"), "应保留 emoji");
    println!("  ✓ Unicode 处理正确");

    // 超长消息
    let long_msg = "x".repeat(1000);
    let long_record = LogRecord::new(Level::INFO, "test".to_string(), long_msg.clone());
    let long_out = tpl.render(&long_record);
    assert_eq!(long_out, format!("[INFO] {}", long_msg));
    println!(
        "\n超长消息: {} 字符 → {} 字符",
        long_msg.len(),
        long_out.len()
    );
    println!("  ✓ 超长消息处理正确\n");
}

/// 示例 7：渲染输出验证
///
/// 使用 assert_eq! 精确验证渲染结果。
fn render_verification() {
    println!("--- 示例 7：渲染输出验证 ---\n");

    let record = LogRecord::new(Level::ERROR, "payment".to_string(), "支付失败".to_string());

    // 精确匹配验证
    let tpl1 = LogTemplate::new("[{level}] {message}");
    assert_eq!(tpl1.render(&record), "[ERROR] 支付失败");
    println!("✓ [ERROR] 支付失败 — 精确匹配");

    // 包含验证
    let tpl2 = LogTemplate::new("{timestamp} [{level}] {target}: {message}");
    let out = tpl2.render(&record);
    assert!(out.contains("ERROR"), "应包含级别");
    assert!(out.contains("payment"), "应包含 target");
    assert!(out.contains("支付失败"), "应包含消息");
    println!("✓ {} — 包含验证", out);

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
        assert_eq!(out, expected, "级别 {} 渲染不一致", level);
    }
    println!("✓ 5 个级别渲染一致性验证通过\n");
}

/// 最佳实践建议
fn best_practices() {
    println!("--- 最佳实践 ---\n");

    println!("1. 模板选择指南：");
    println!("   开发环境: [{{level}}] {{message}} — 简洁快速");
    println!("   生产环境: {{timestamp}} [{{level}}] {{target}} - {{message}} — 信息完整");
    println!("   调试场景: [{{timestamp}}] [{{level}}] {{file}}:{{line}} - {{message}} — 定位源码");
    println!(
        "   JSON 采集: {{\"ts\":\"{{timestamp}}\",\"lvl\":\"{{level}}\",\"msg\":\"{{message}}\"}} — 结构化采集"
    );

    println!("\n2. 性能考虑：");
    println!("   - LogTemplate::new() 在构造时解析占位符，render() 仅做字符串拼接");
    println!("   - 建议全局创建一个模板实例复用，避免重复构造");
    println!("   - {{fields}} 渲染涉及 JSON 序列化，高频场景慎用");

    println!("\n3. 占位符注意事项：");
    println!("   - 未知占位符（如 {{unknown}}）渲染为空字符串");
    println!("   - {{file}} 和 {{line}} 在无源码信息时渲染为空");
    println!("   - 双花括号 {{}} 用于转义字面花括号（JSON 格式常用）");

    println!("\n4. 与 tracing 集成：");
    println!("   - LogTemplate 主要用于 LogRecord 渲染");
    println!("   - tracing 的输出格式由 tracing-subscriber 控制");
    println!("   - 两者可配合：tracing 收集日志 → LogTemplate 格式化输出");
}

/// 创建示例日志记录
fn create_sample_record() -> LogRecord {
    let mut record = LogRecord::new(
        Level::INFO,
        "my_app::module".to_string(),
        "用户登录成功".to_string(),
    );
    record.file = Some("src/main.rs".to_string());
    record.line = Some(42);
    record
}
