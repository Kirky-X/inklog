//! # 日志模板模块
//!
//! 提供自定义日志格式化和字段提取功能，支持灵活的日志消息模板配置。
//!
//! ## 概述
//!
//! `LogTemplate` 结构体实现日志消息的模板化渲染，支持自定义占位符和格式。
//! 通过模板系统，可以灵活控制日志输出的格式和内容。

use crate::log_record::LogRecord;
use serde_json::Value;

fn format_field(key: &str, value: &Value) -> String {
    match value {
        Value::String(s) => format!("{}={}", key, s),
        Value::Number(n) => format!("{}={}", key, n),
        Value::Bool(b) => format!("{}={}", key, b),
        Value::Null => format!("{}={}", key, "null"),
        _ => format!("{}={}", key, value),
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LogTemplate {
    template: String,
    placeholders: Vec<Placeholder>,
}

#[derive(Debug, Clone)]
enum Placeholder {
    Timestamp,
    Level,
    Target,
    Message,
    File,
    Line,
    ThreadId,
    Fields,
    Literal(String),
}

impl LogTemplate {
    pub fn new(template: &str) -> Self {
        let mut placeholders = Vec::new();
        let mut current = String::new();
        let mut in_placeholder = false;

        for (idx, ch) in template.chars().enumerate() {
            if ch == '{' {
                if idx > 0 && template.chars().nth(idx - 1) == Some('\\') {
                    current.push(ch);
                } else {
                    if !current.is_empty() {
                        placeholders.push(Placeholder::Literal(current.clone()));
                        current.clear();
                    }
                    in_placeholder = true;
                }
            } else if ch == '}' && in_placeholder {
                let placeholder_name = current.trim().to_lowercase();
                match placeholder_name.as_str() {
                    "timestamp" => placeholders.push(Placeholder::Timestamp),
                    "level" => placeholders.push(Placeholder::Level),
                    "target" => placeholders.push(Placeholder::Target),
                    "message" => placeholders.push(Placeholder::Message),
                    "file" => placeholders.push(Placeholder::File),
                    "line" => placeholders.push(Placeholder::Line),
                    "thread_id" => placeholders.push(Placeholder::ThreadId),
                    "fields" => placeholders.push(Placeholder::Fields),
                    _ => {
                        placeholders.push(Placeholder::Literal(format!("{{{}}}", current)));
                    }
                }
                current.clear();
                in_placeholder = false;
            } else {
                // Either in placeholder or not, push the character
                current.push(ch);
            }
        }

        if !current.is_empty() {
            placeholders.push(Placeholder::Literal(current));
        }

        Self {
            template: template.to_string(),
            placeholders,
        }
    }

    pub fn render(&self, record: &LogRecord) -> String {
        let mut result = String::new();

        for placeholder in &self.placeholders {
            match placeholder {
                Placeholder::Timestamp => {
                    result.push_str(
                        &record
                            .timestamp
                            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                            .to_string(),
                    );
                }
                Placeholder::Level => {
                    result.push_str(&record.level);
                }
                Placeholder::Target => {
                    result.push_str(&record.target);
                }
                Placeholder::Message => {
                    result.push_str(&record.message);
                }
                Placeholder::File => {
                    if let Some(ref file) = record.file {
                        result.push_str(file);
                    }
                }
                Placeholder::Line => {
                    if let Some(line) = record.line {
                        result.push_str(&line.to_string());
                    }
                }
                Placeholder::ThreadId => {
                    result.push_str(&record.thread_id);
                }
                Placeholder::Fields => {
                    if !record.fields.is_empty() {
                        result.push(' ');
                        let fields_str = record
                            .fields
                            .iter()
                            .map(|(k, v)| format_field(k, v))
                            .collect::<Vec<_>>()
                            .join(" ");
                        result.push_str(&fields_str);
                    }
                }
                Placeholder::Literal(lit) => {
                    result.push_str(lit);
                }
            }
        }

        result
    }
}

impl Default for LogTemplate {
    fn default() -> Self {
        Self::new("{timestamp} [{level}] {target} - {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::log_record::LogRecord;
    use chrono::Utc;
    use serde_json::Value;
    use std::collections::HashMap;

    fn create_test_record() -> LogRecord {
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "test_module".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::from([
                ("user".to_string(), Value::String("123".to_string())),
                ("action".to_string(), Value::String("login".to_string())),
            ]),
            file: Some("/path/to/test.rs".to_string()),
            line: Some(42),
            thread_id: "abc123".to_string(),
        }
    }

    #[test]
    fn test_default_format() {
        let template = LogTemplate::default();
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("INFO"));
        assert!(output.contains("test_module"));
        assert!(output.contains("Test message"));
    }

    #[test]
    fn test_custom_format() {
        let template = LogTemplate::new("[{timestamp}] [{level}] {message}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.starts_with("["));
        assert!(output.contains("] [INFO] Test message"));
    }

    #[test]
    fn test_all_placeholders() {
        let template =
            LogTemplate::new("{timestamp} [{level}] {target} - {message} ({file}:{line})");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("/path/to/test.rs:42"));
    }

    #[test]
    fn test_fields_placeholder() {
        let template = LogTemplate::new("{message} {fields}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("user=123"), "Output: {}", output);
        assert!(output.contains("action=login"), "Output: {}", output);
    }

    #[test]
    fn test_literal_braces() {
        let template = LogTemplate::new("{{literal}} {message}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.starts_with("{literal}"));
    }

    #[test]
    fn test_empty_fields() {
        let template = LogTemplate::new("{message}");
        let mut record = create_test_record();
        record.fields.clear();
        let output = template.render(&record);
        assert_eq!(output, "Test message");
    }

    #[test]
    fn test_thread_id_placeholder() {
        let template = LogTemplate::new("{message} [thread:{thread_id}]");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("[thread:abc123]"));
    }

    #[test]
    fn test_unknown_placeholder() {
        let template = LogTemplate::new("{message} {unknown}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("{unknown}"));
    }

    #[test]
    fn test_multiple_timestamps() {
        let template = LogTemplate::new("{timestamp} - {timestamp}");
        let record = create_test_record();
        let output = template.render(&record);
        let parts: Vec<&str> = output.split(" - ").collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], parts[1]);
    }

    #[test]
    fn test_special_characters_in_message() {
        let template = LogTemplate::new("{message}");
        let mut record = create_test_record();
        record.message = "Special chars: \"quotes\" & <brackets>".to_string();
        let output = template.render(&record);
        assert!(output.contains("quotes"));
        assert!(output.contains("&"));
        assert!(output.contains("brackets"));
    }

    #[test]
    fn test_numeric_fields() {
        let template = LogTemplate::new("{message} {fields}");
        let mut record = create_test_record();
        record.fields = HashMap::from([
            (
                "count".to_string(),
                Value::Number(serde_json::Number::from(42)),
            ),
            (
                "price".to_string(),
                Value::Number(serde_json::Number::from_f64(19.99).unwrap()),
            ),
        ]);
        let output = template.render(&record);
        assert!(output.contains("count=42"));
        assert!(output.contains("price=19.99"));
    }

    #[test]
    fn test_boolean_fields() {
        let template = LogTemplate::new("{message} {fields}");
        let mut record = create_test_record();
        record.fields = HashMap::from([
            ("active".to_string(), Value::Bool(true)),
            ("deleted".to_string(), Value::Bool(false)),
        ]);
        let output = template.render(&record);
        assert!(output.contains("active=true"));
        assert!(output.contains("deleted=false"));
    }

    #[test]
    fn test_null_fields() {
        let template = LogTemplate::new("{message} {fields}");
        let mut record = create_test_record();
        record.fields = HashMap::from([("optional".to_string(), Value::Null)]);
        let output = template.render(&record);
        assert!(output.contains("optional=null"));
    }

    #[test]
    fn test_empty_line_and_file() {
        let template = LogTemplate::new("{message} ({file}:{line})");
        let mut record = create_test_record();
        record.file = None;
        record.line = None;
        let output = template.render(&record);
        // When both file and line are None, it renders as "(:)"
        assert!(output.contains("Test message"));
    }

    #[test]
    fn test_template_clone() {
        let template1 = LogTemplate::new("{timestamp} [{level}] {message}");
        let template2 = template1.clone();
        let record = create_test_record();
        let output1 = template1.render(&record);
        let output2 = template2.render(&record);
        assert_eq!(output1, output2);
    }

    #[test]
    fn test_escaped_brace() {
        let template = LogTemplate::new(r"{{escaped}} {message}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.starts_with("{escaped}"));
    }

    #[test]
    fn test_complex_format() {
        let template =
            LogTemplate::new("[{timestamp}] [{level}] [{thread_id}] {target} - {message} {fields}");
        let record = create_test_record();
        let output = template.render(&record);
        assert!(output.contains("[INFO]"));
        assert!(output.contains("[abc123]"));
        assert!(output.contains("test_module"));
        assert!(output.contains("user=123"));
        assert!(output.contains("action=login"));
    }
}
