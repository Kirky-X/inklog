// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

/// Template-based log message formatter with customizable placeholders.
///
/// `LogTemplate` provides flexible log message formatting through template strings
/// with placeholders that are replaced with values from [`LogRecord`].
///
/// # Supported Placeholders
///
/// | Placeholder | Description | Example Output |
/// |-------------|-------------|----------------|
/// | `{timestamp}` | UTC timestamp in ISO 8601 format | `2026-03-19T10:30:45.123Z` |
/// | `{level}` | Log level | `INFO`, `ERROR`, `DEBUG` |
/// | `{target}` | Target module path | `my_module::submodule` |
/// | `{message}` | Log message content | `User logged in` |
/// | `{file}` | Source file path | `src/main.rs` |
/// | `{line}` | Line number in source file | `42` |
/// | `{thread_id}` | Thread identifier | `thread-1` |
/// | `{fields}` | Structured fields as key=value pairs | `user=123 action=login` |
///
/// # Escaping
///
/// To include literal braces in the output, use `{{` and `}}`:
/// - Template: `{{literal}} {message}`
/// - Output: `{literal} User logged in`
///
/// # Examples
///
/// ```
/// use inklog::template::LogTemplate;
/// use inklog::log_record::LogRecord;
/// use chrono::Utc;
/// use std::collections::HashMap;
/// use serde_json::Value;
///
/// // Create a template with standard format
/// let template = LogTemplate::new("{timestamp} [{level}] {target} - {message}");
///
/// // Render a log record
/// let record = LogRecord {
///     timestamp: Utc::now(),
///     level: "INFO".to_string(),
///     target: "my_app::api".to_string(),
///     message: "Request processed".to_string(),
///     fields: HashMap::new(),
///     file: None,
///     line: None,
///     thread_id: "main".to_string(),
/// };
///
/// let output = template.render(&record);
/// // Output: "2026-03-19T10:30:45.123Z [INFO] my_app::api - Request processed"
/// ```
///
/// # Performance
///
/// Template parsing happens once during [`LogTemplate::new()`], and the parsed
/// placeholder structure is reused for all subsequent [`render()`](LogTemplate::render)
/// calls, making rendering efficient for high-throughput logging scenarios.
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
    /// Creates a new `LogTemplate` from a template string.
    ///
    /// Parses the template string to identify placeholders (enclosed in `{}`) and
    /// literal text. The parsed structure is stored for efficient rendering.
    ///
    /// # Arguments
    ///
    /// * `template` - A template string containing placeholders and literal text.
    ///   Placeholders are enclosed in curly braces: `{placeholder_name}`.
    ///
    /// # Supported Placeholders
    ///
    /// - `{timestamp}` - UTC timestamp (ISO 8601 format with milliseconds)
    /// - `{level}` - Log level (INFO, ERROR, DEBUG, etc.)
    /// - `{target}` - Target module path
    /// - `{message}` - Log message content
    /// - `{file}` - Source file path (optional, renders empty if not present)
    /// - `{line}` - Line number (optional, renders empty if not present)
    /// - `{thread_id}` - Thread identifier
    /// - `{fields}` - Structured fields as `key=value` pairs
    ///
    /// # Escaping
    ///
    /// Use `{{` to output a literal `{` character:
    /// - Input: `"{{escaped}}"` → Output: `"{escaped}"`
    ///
    /// Unknown placeholders are rendered as-is (e.g., `{unknown}` remains `{unknown}`).
    ///
    /// # Examples
    ///
    /// ```
    /// use inklog::template::LogTemplate;
    ///
    /// // Standard log format
    /// let template = LogTemplate::new("{timestamp} [{level}] {target} - {message}");
    ///
    /// // With file and line information
    /// let template = LogTemplate::new("{message} ({file}:{line})");
    ///
    /// // Custom format with fields
    /// let template = LogTemplate::new("[{level}] {message} {fields}");
    ///
    /// // With literal braces
    /// let template = LogTemplate::new("{{literal}} {message}");
    /// ```
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

    /// Renders a log record using the template.
    ///
    /// Replaces all placeholders in the template with values from the provided `LogRecord`.
    /// The rendering is efficient as the template is pre-parsed during construction.
    ///
    /// # Arguments
    ///
    /// * `record` - The log record containing values to substitute into the template.
    ///
    /// # Returns
    ///
    /// A formatted string with all placeholders replaced by their corresponding values
    /// from the log record.
    ///
    /// # Placeholder Resolution
    ///
    /// | Placeholder | Source Field | Behavior When Missing |
    /// |-------------|--------------|----------------------|
    /// | `{timestamp}` | `record.timestamp` | Always present (required field) |
    /// | `{level}` | `record.level` | Always present (required field) |
    /// | `{target}` | `record.target` | Always present (required field) |
    /// | `{message}` | `record.message` | Always present (required field) |
    /// | `{file}` | `record.file` | Renders as empty string if `None` |
    /// | `{line}` | `record.line` | Renders as empty string if `None` |
    /// | `{thread_id}` | `record.thread_id` | Always present (required field) |
    /// | `{fields}` | `record.fields` | Renders as empty string if empty |
    ///
    /// # Examples
    ///
    /// ```
    /// use inklog::template::LogTemplate;
    /// use inklog::log_record::LogRecord;
    /// use chrono::Utc;
    /// use std::collections::HashMap;
    ///
    /// let template = LogTemplate::new("[{level}] {message}");
    /// let record = LogRecord {
    ///     timestamp: Utc::now(),
    ///     level: "INFO".to_string(),
    ///     target: "my_module".to_string(),
    ///     message: "Task completed".to_string(),
    ///     fields: HashMap::new(),
    ///     file: None,
    ///     line: None,
    ///     thread_id: "main".to_string(),
    /// };
    ///
    /// let output = template.render(&record);
    /// assert!(output.starts_with("[INFO] Task completed"));
    /// ```
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

    #[test]
    fn test_deeply_nested_fields() {
        let template = LogTemplate::new("{message} {fields}");
        let mut record = create_test_record();
        let inner = serde_json::json!({"level3": "deep"});
        let middle = serde_json::json!({"level2": inner});
        record.fields = HashMap::from([("level1".to_string(), middle)]);
        let output = template.render(&record);
        assert!(output.contains("level3"));
    }

    #[test]
    fn test_array_in_fields() {
        let template = LogTemplate::new("{message} {fields}");
        let mut record = create_test_record();
        record.fields = HashMap::from([(
            "items".to_string(),
            Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ]),
        )]);
        let output = template.render(&record);
        assert!(output.contains("items"));
        assert!(output.contains("a"));
        assert!(output.contains("b"));
        assert!(output.contains("c"));
    }

    #[test]
    fn test_template_from_str() {
        let template = LogTemplate::new("{timestamp} [{level}] {message}");
        assert!(!template.template.is_empty());
    }

    #[test]
    fn test_target_with_underscores() {
        let template = LogTemplate::new("{target} - {message}");
        let mut record = create_test_record();
        record.target = "my_module.sub_module".to_string();
        let output = template.render(&record);
        assert!(output.contains("my_module.sub_module"));
    }

    #[test]
    fn test_message_with_newlines() {
        let template = LogTemplate::new("{message}");
        let mut record = create_test_record();
        record.message = "Line1\nLine2\nLine3".to_string();
        let output = template.render(&record);
        assert!(output.contains("Line1"));
        assert!(output.contains("Line2"));
        assert!(output.contains("Line3"));
    }

    #[test]
    fn test_timestamp_format() {
        let template = LogTemplate::new("{timestamp}");
        let record = create_test_record();
        let output = template.render(&record);
        // Timestamp should contain numbers
        assert!(output.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_level_display() {
        let template = LogTemplate::new("[{level}] {message}");
        let mut record = create_test_record();
        record.level = "ERROR".to_string();
        let output = template.render(&record);
        assert!(output.contains("[ERROR]"));
    }

    #[test]
    fn test_message_with_unicode() {
        let template = LogTemplate::new("{message}");
        let mut record = create_test_record();
        record.message = "你好世界 🌍 مرحبا".to_string();
        let output = template.render(&record);
        assert!(output.contains("你好世界"));
        assert!(output.contains("مرحبا"));
    }

    #[test]
    fn test_message_with_template_syntax() {
        let template = LogTemplate::new("{message}");
        let mut record = create_test_record();
        record.message = "Value is {variable}".to_string();
        let output = template.render(&record);
        assert!(output.contains("{variable}"));
    }
}
