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
use inklog_examples::common::{print_section, print_separator};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    print_separator("inklog output format example");

    show_output_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_text_format_rendering();
    show_json_format_rendering();
    show_sink_config_usage();
    show_best_practices();

    println!("\n✓ All output format examples completed");
}

/// 展示 OutputFormat 枚举变体
#[allow(clippy::eq_op)] // 示例代码故意展示相同变体比较
fn show_output_format_variants() {
    print_section("Example 1: OutputFormat enum variants");

    let text = OutputFormat::Text;
    let json = OutputFormat::Json;
    let default = OutputFormat::default();

    println!("OutputFormat::Text    = {:?}", text);
    println!("OutputFormat::Json    = {:?}", json);
    println!("OutputFormat::default() = {:?}", default);
    println!("\nDefault format: {:?} (human-readable text)", default);
    assert_eq!(default, OutputFormat::Text);

    println!("\nEquivalence check:");
    let is_text_equal = OutputFormat::Text == OutputFormat::Text;
    println!("  Text == Text → {}", is_text_equal);
    println!(
        "  Text == Json → {}",
        OutputFormat::Text == OutputFormat::Json
    );
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section("Example 2: FromStr string parsing");

    println!("String → OutputFormat parsing:\n");

    let cases = ["text", "Text", "TEXT", "json", "Json", "JSON"];
    for s in &cases {
        match OutputFormat::from_str(s) {
            Ok(fmt) => println!("  {:?} → {:?} ✓", s, fmt),
            Err(e) => println!("  {:?} → error: {}", s, e),
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
        "\n  {:?} → error (invalid value): {}",
        "xml",
        invalid.unwrap_err()
    );
}

/// 展示 Display trait
fn show_display_trait() {
    print_section("Example 3: Display trait output");

    println!("OutputFormat → string:\n");
    println!("  OutputFormat::Text → \"{}\"", OutputFormat::Text);
    println!("  OutputFormat::Json → \"{}\"", OutputFormat::Json);

    assert_eq!(format!("{}", OutputFormat::Text), "text");
    assert_eq!(format!("{}", OutputFormat::Json), "json");
}

/// 展示 Text 格式的日志渲染效果
fn show_text_format_rendering() {
    print_section("Example 4: Text rendering (default)");

    let template = LogTemplate::default();
    let record = create_sample_record();

    let rendered = template.render(&record);
    println!("LogTemplate::default() rendering:");
    println!("  {}", rendered);
    println!("\nCharacteristics:");
    println!("  - Human-readable; suits consoles and log files");
    println!("  - Contains timestamp, level, target, message");
    println!("  - Structured fields appended as JSON at the end");
}

/// 展示 Json 格式的日志渲染效果
fn show_json_format_rendering() {
    print_section("Example 5: Json rendering (NDJSON)");

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

    println!("NDJSON output:");
    println!("  {}", json_line);
    println!("\nCharacteristics:");
    println!("  - Machine-parseable; suits log collection systems");
    println!("  - One complete JSON object per line");
    println!("  - Directly collectable by Fluentd/Logstash/Promtail");
    println!("  - Supports nested structured fields");

    // 验证 JSON 合法性
    let parsed: Value = serde_json::from_str(&json_line).unwrap();
    assert_eq!(parsed["level"], "INFO");
    assert_eq!(parsed["message"], "user logged in");
    println!("\n  ✓ JSON output parses correctly with standard parsers");
}

/// 展示在 Sink 配置中使用 OutputFormat
fn show_sink_config_usage() {
    print_section("Example 6: OutputFormat in sink configs");

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
    println!("Console sink config (JSON format):");
    println!("  output_format = {:?}", console_cfg.output_format);

    // File Sink 使用 Text 格式
    let file_cfg = FileSinkConfig {
        enabled: true,
        output_format: OutputFormat::Text,
        ..Default::default()
    };
    println!("\nFile sink config (Text format):");
    println!("  output_format = {:?}", file_cfg.output_format);

    println!("\nTOML config example:");
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
    print_section("7. Best practices");

    println!("OutputFormat selection guide:\n");

    println!("1. Text format use cases:");
    println!("   - Development console output");
    println!("   - Log files read directly by humans");
    println!("   - Debugging and troubleshooting");

    println!("\n2. Json format use cases:");
    println!("   - Production structured log collection");
    println!("   - Log analytics platforms (ELK/Loki/Datadog)");
    println!("   - Suits field-level querying and filtering");

    println!("\n3. Mixing formats:");
    println!("   - Console → Text (easy for developers to read)");
    println!("   - File → Json (easy to collect and analyze)");
    println!("   - Database → structured fields stored directly");

    println!("\n4. Performance considerations:");
    println!("   - Text format renders faster (simple string concatenation)");
    println!("   - Json format needs serialization (~2-3x overhead)");
    println!("   - For high throughput, use Text for files and convert to Json at the collector");
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
