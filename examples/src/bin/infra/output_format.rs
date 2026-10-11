// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 输出格式示例（Layer 0 零依赖）
//!
//! 演示 `inklog::OutputFormat` 枚举的使用：
//!
//! 1. `OutputFormat::Text`（默认）人类可读文本格式
//! 2. `OutputFormat::Json` NDJSON 机器可解析格式
//! 3. FromStr / Display trait 实现
//! 4. 在 Sink 配置中使用 OutputFormat
//! 5. 不同格式的输出对比
//!
//! # 运行
//! ```bash
//! cargo run --bin output_format
//! ```

use inklog::tracing::Level;
use inklog::{LogRecord, LogTemplate, OutputFormat};
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    print_separator(&tr("example-output-format-inklog-output-format-example"));

    show_output_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_text_format_rendering();
    show_json_format_rendering();
    show_sink_config_usage();
    show_best_practices();

    println!(
        "\n{}",
        tr("example-output-format-all-output-format-examples")
    );
}

/// 展示 OutputFormat 枚举变体
#[allow(clippy::eq_op)] // 示例代码故意展示相同变体比较
fn show_output_format_variants() {
    print_section(&tr("example-output-format-example-1-outputformat-enum"));

    let text = OutputFormat::Text;
    let json = OutputFormat::Json;
    let default = OutputFormat::default();

    println!(
        "{}",
        tr_args(
            "example-output-format-outputformat-text",
            &[("text", &format!("{:?}", text))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-output-format-outputformat-json",
            &[("json", &format!("{:?}", json))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-output-format-outputformat-default",
            &[("default", &format!("{:?}", default))]
        )
    );
    println!(
        "\n{}",
        tr_args(
            "example-output-format-default-format-human-readable",
            &[("default", &format!("{:?}", default))]
        )
    );
    assert_eq!(default, OutputFormat::Text);

    println!("\n{}", tr("example-output-format-equivalence-check"));
    let is_text_equal = OutputFormat::Text == OutputFormat::Text;
    println!(
        "  {}",
        tr_args(
            "example-output-format-text-text",
            &[("is_text_equal", &is_text_equal)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-output-format-text-json",
            &[("value", &(OutputFormat::Text == OutputFormat::Json))]
        )
    );
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section(&tr(
        "example-output-format-example-2-fromstr-string-parsing",
    ));

    println!(
        "{}\n",
        tr("example-output-format-string-outputformat-parsing")
    );

    let cases = ["text", "Text", "TEXT", "json", "Json", "JSON"];
    for s in &cases {
        match OutputFormat::from_str(s) {
            Ok(fmt) => println!(
                "  {}",
                tr_args(
                    "example-output-format-from-str-parsed-check",
                    &[("s", &format!("{:?}", s)), ("fmt", &format!("{:?}", fmt))]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-output-format-error",
                    &[("s", &format!("{:?}", s)), ("e", &e)]
                )
            ),
        }
    }

    // 验证解析结果
    assert_eq!(OutputFormat::from_str("text").unwrap(), OutputFormat::Text);
    assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);

    // 无效值
    let invalid = OutputFormat::from_str("xml");
    assert!(invalid.is_err());
    println!(
        "\n  {}",
        tr_args(
            "example-output-format-error-invalid-value",
            &[
                ("value", &format!("{:?}", "xml")),
                ("value2", &(invalid.unwrap_err()))
            ]
        )
    );
}

/// 展示 Display trait
fn show_display_trait() {
    print_section(&tr("example-output-format-example-3-display-trait-output"));

    println!("{}\n", tr("example-output-format-outputformat-string"));
    println!(
        "  {}",
        tr_args(
            "example-output-format-outputformat-text-2",
            &[("value", &(OutputFormat::Text))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-output-format-outputformat-json-2",
            &[("value", &(OutputFormat::Json))]
        )
    );

    assert_eq!(format!("{}", OutputFormat::Text), "text");
    assert_eq!(format!("{}", OutputFormat::Json), "json");
}

/// 展示 Text 格式的日志渲染效果
fn show_text_format_rendering() {
    print_section(&tr(
        "example-output-format-example-4-text-rendering-default",
    ));

    let template = LogTemplate::default();
    let record = create_sample_record();

    let rendered = template.render(&record);
    println!(
        "{}",
        tr("example-output-format-logtemplate-default-rendering")
    );
    println!("  {}", rendered);
    println!("\n{}", tr("example-output-format-characteristics"));
    println!(
        "  {}",
        tr("example-output-format-human-readable-suits-consoles")
    );
    println!(
        "  {}",
        tr("example-output-format-contains-timestamp-level-target")
    );
    println!(
        "  {}",
        tr("example-output-format-structured-fields-appended-as")
    );
}

/// 展示 Json 格式的日志渲染效果
fn show_json_format_rendering() {
    print_section(&tr("example-output-format-example-5-json-rendering-ndjson"));

    let record = create_sample_record();

    // Json 格式输出为 NDJSON（每行一个 JSON 对象）
    let json_line = serde_json::to_string(&serde_json::json!({
        "timestamp": record.timestamp.to_rfc3339(),
        "level": record.level,
        "target": record.target,
        "message": record.message,
        "fields": record.fields,
    }))
    .unwrap();

    println!("{}", tr("example-output-format-ndjson-output"));
    println!("  {}", json_line);
    println!("\n{}", tr("example-output-format-characteristics"));
    println!(
        "  {}",
        tr("example-output-format-machine-parseable-suits-log")
    );
    println!(
        "  {}",
        tr("example-output-format-one-complete-json-object-per")
    );
    println!(
        "  {}",
        tr("example-output-format-directly-collectable-by-fluentd")
    );
    println!(
        "  {}",
        tr("example-output-format-supports-nested-structured")
    );

    // 验证 JSON 合法性
    let parsed: Value = serde_json::from_str(&json_line).unwrap();
    assert_eq!(parsed["level"], "INFO");
    assert_eq!(parsed["message"], "user logged in");
    println!(
        "\n  {}",
        tr("example-output-format-json-output-parses-correctly")
    );
}

/// 展示在 Sink 配置中使用 OutputFormat
fn show_sink_config_usage() {
    print_section(&tr("example-output-format-example-6-outputformat-in-sink"));

    use inklog::config::{ConsoleSinkConfig, FileSinkConfig};

    // Console Sink 使用 JSON 格式
    let console_cfg = ConsoleSinkConfig {
        enabled: true,
        colored: false,
        stderr_levels: vec![],
        masking_enabled: false,
        output_format: OutputFormat::Json,
        secret_scan_enabled: false,
    };
    println!(
        "{}",
        tr("example-output-format-console-sink-config-json-format")
    );
    println!(
        "  {}",
        tr_args(
            "example-output-format-output-format",
            &[("output_format", &format!("{:?}", console_cfg.output_format))]
        )
    );

    // File Sink 使用 Text 格式
    let file_cfg = FileSinkConfig {
        enabled: true,
        output_format: OutputFormat::Text,
        ..Default::default()
    };
    println!(
        "\n{}",
        tr("example-output-format-file-sink-config-text-format")
    );
    println!(
        "  {}",
        tr_args(
            "example-output-format-output-format",
            &[("output_format", &format!("{:?}", file_cfg.output_format))]
        )
    );

    println!("\n{}", tr("example-output-format-toml-config-example"));
    println!(
        r#"
[console_sink]
enabled = true
output_format = "json"

[file_sink]
enabled = true
path = "logs/app.log"
output_format = "text""#
    );
}

/// 最佳实践建议
fn show_best_practices() {
    print_section(&tr("example-output-format-7-best-practices"));

    println!(
        "{}\n",
        tr("example-output-format-outputformat-selection-guide")
    );

    println!("{}", tr("example-output-format-1-text-format-use-cases"));
    println!(
        "   {}",
        tr("example-output-format-development-console-output")
    );
    println!(
        "   {}",
        tr("example-output-format-log-files-read-directly-by")
    );
    println!(
        "   {}",
        tr("example-output-format-debugging-and-troubleshooting")
    );

    println!("\n{}", tr("example-output-format-2-json-format-use-cases"));
    println!(
        "   {}",
        tr("example-output-format-production-structured-log")
    );
    println!(
        "   {}",
        tr("example-output-format-log-analytics-platforms-elk")
    );
    println!(
        "   {}",
        tr("example-output-format-suits-field-level-querying-and")
    );

    println!("\n{}", tr("example-output-format-3-mixing-formats"));
    println!("   {}", tr("example-output-format-console-text-easy-for"));
    println!(
        "   {}",
        tr("example-output-format-file-json-easy-to-collect-and")
    );
    println!(
        "   {}",
        tr("example-output-format-database-structured-fields")
    );

    println!(
        "\n{}",
        tr("example-output-format-4-performance-considerations")
    );
    println!(
        "   {}",
        tr("example-output-format-text-format-renders-faster")
    );
    println!(
        "   {}",
        tr("example-output-format-json-format-needs-serialization")
    );
    println!(
        "   {}",
        tr("example-output-format-for-high-throughput-use-text")
    );
}

/// 创建示例日志记录
fn create_sample_record() -> LogRecord {
    let mut record = LogRecord::new(
        Level::INFO,
        "my_app::auth".to_string(),
        "user logged in".to_string(),
    );
    record.fields = HashMap::from([
        ("user_id".to_string(), Value::Number(12345.into())),
        ("action".to_string(), Value::String("login".to_string())),
    ]);
    record
}
