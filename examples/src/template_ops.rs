// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 日志模板渲染辅助。
//!
//! 从 `examples/src/bin/template.rs` 提取共享逻辑：
//!
//! - [`create_sample_record`]：构造带固定字段的示例 LogRecord，便于跨测试复用。
//! - [`create_record_with_fields`]：构造带结构化字段的 LogRecord。
//! - [`render_formats`]：用多种模板格式渲染同一条记录，返回 `(格式名, 渲染结果)` 列表。

use inklog::tracing::Level;
use inklog::{LogRecord, LogTemplate};
use serde_json::Value;
use std::collections::HashMap;

/// 创建示例日志记录，与 `template.rs` bin 中的 `create_sample_record` 行为一致。
///
/// - level: INFO
/// - target: `my_app::module`
/// - message: `用户登录成功`
/// - file: `Some("src/main.rs")`
/// - line: `Some(42)`
pub fn create_sample_record() -> LogRecord {
    let mut record = LogRecord::new(
        Level::INFO,
        "my_app::module".to_string(),
        "用户登录成功".to_string(),
    );
    record.file = Some("src/main.rs".to_string());
    record.line = Some(42);
    record
}

/// 创建带结构化字段的日志记录。
///
/// 在 [`create_sample_record`] 基础上覆盖 `fields`。
pub fn create_record_with_fields(fields: HashMap<String, Value>) -> LogRecord {
    let mut record = create_sample_record();
    record.fields = fields;
    record
}

/// 用多种模板格式渲染同一条记录，返回 `(格式名, 渲染结果)` 列表。
///
/// 用于对比不同模板的输出。包含 5 种代表性格式：简洁、标准、详细、JSON 风格、自定义分隔符。
pub fn render_formats(record: &LogRecord) -> Vec<(String, String)> {
    let formats: Vec<(&str, &str)> = vec![
        ("简洁格式", "[{level}] {message}"),
        ("标准格式", "{timestamp} [{level}] {target} - {message}"),
        (
            "详细格式",
            "[{timestamp}] [{level}] [{thread_id}] {file}:{line} - {message}",
        ),
        (
            "JSON 风格",
            "{{\"time\":\"{timestamp}\",\"level\":\"{level}\",\"msg\":\"{message}\"}}",
        ),
        ("自定义分隔符", "{timestamp}>>>[{level}]>>> {message}"),
    ];

    formats
        .into_iter()
        .map(|(name, fmt)| {
            let template = LogTemplate::new(fmt);
            (name.to_string(), template.render(record))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sample_record() {
        // 验证：示例记录字段与文档一致。
        let record = create_sample_record();
        assert_eq!(record.level, "INFO");
        assert_eq!(record.target, "my_app::module");
        assert_eq!(record.message, "用户登录成功");
        assert_eq!(record.file.as_deref(), Some("src/main.rs"));
        assert_eq!(record.line, Some(42));
        assert!(record.fields.is_empty());
        assert!(!record.thread_id.is_empty(), "thread_id 不应为空");
    }

    #[test]
    fn test_create_record_with_fields() {
        // 验证：结构化字段被正确设置。
        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), Value::Number(12345.into()));
        fields.insert("action".to_string(), Value::String("login".to_string()));
        fields.insert("success".to_string(), Value::Bool(true));

        let record = create_record_with_fields(fields);
        assert_eq!(record.fields.len(), 3);
        assert_eq!(record.fields["user_id"], Value::Number(12345.into()));
        assert_eq!(record.fields["action"], Value::String("login".to_string()));
        assert_eq!(record.fields["success"], Value::Bool(true));
        // 基础字段仍保留
        assert_eq!(record.level, "INFO");
        assert_eq!(record.message, "用户登录成功");
    }

    #[test]
    fn test_render_formats_count() {
        // 验证：render_formats 返回 5 种格式。
        let record = create_sample_record();
        let results = render_formats(&record);
        assert_eq!(results.len(), 5, "应返回 5 种格式");
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"简洁格式"));
        assert!(names.contains(&"标准格式"));
        assert!(names.contains(&"详细格式"));
        assert!(names.contains(&"JSON 风格"));
        assert!(names.contains(&"自定义分隔符"));
    }

    #[test]
    fn test_render_all_placeholders_replaced() {
        // 验证：渲染结果不应包含未替换的占位符 `{xxx}`（{fields} 除外，本例无字段）。
        let record = create_sample_record();
        let results = render_formats(&record);
        for (name, output) in &results {
            // 排除 JSON 风格中合法的 {{ }} 转义后的 { }
            // 简单断言：渲染后不应有 {level} {timestamp} {message} {target} {file} {line} {thread_id} 字面量
            for placeholder in [
                "{level}",
                "{timestamp}",
                "{message}",
                "{target}",
                "{file}",
                "{line}",
                "{thread_id}",
            ] {
                assert!(
                    !output.contains(placeholder),
                    "格式 [{}] 输出仍包含未替换的占位符 {}: {}",
                    name,
                    placeholder,
                    output
                );
            }
        }
    }

    #[test]
    fn test_render_simple_format_content() {
        // 验证：简洁格式渲染包含级别和消息。
        let record = create_sample_record();
        let results = render_formats(&record);
        let simple = results
            .iter()
            .find(|(n, _)| n == "简洁格式")
            .map(|(_, v)| v.clone())
            .expect("应包含简洁格式");
        assert!(
            simple.contains("[INFO]"),
            "简洁格式应包含 [INFO]: {}",
            simple
        );
        assert!(
            simple.contains("用户登录成功"),
            "简洁格式应包含消息: {}",
            simple
        );
    }

    #[test]
    fn test_render_json_format_contains_keys() {
        // 验证：JSON 风格渲染包含 time/level/msg 三个键。
        let record = create_sample_record();
        let results = render_formats(&record);
        let json = results
            .iter()
            .find(|(n, _)| n == "JSON 风格")
            .map(|(_, v)| v.clone())
            .expect("应包含 JSON 风格");
        assert!(json.contains("\"time\""), "JSON 应包含 time 键: {}", json);
        assert!(json.contains("\"level\""), "JSON 应包含 level 键: {}", json);
        assert!(json.contains("\"msg\""), "JSON 应包含 msg 键: {}", json);
        assert!(json.contains("INFO"), "JSON 应包含 INFO 值: {}", json);
    }

    #[test]
    fn test_render_fields_placeholder() {
        // 验证：{fields} 占位符能渲染结构化字段为 key=value 形式。
        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), Value::Number(12345.into()));
        let record = create_record_with_fields(fields);

        let template = LogTemplate::new("{message} | {fields}");
        let output = template.render(&record);
        assert!(
            output.contains("user_id=12345"),
            "{{fields}} 应渲染为 key=value: {}",
            output
        );
    }
}
