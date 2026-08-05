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
    print_separator("inklog 输出格式示例");

    show_output_format_variants();
    show_from_str_parsing();
    show_display_trait();
    show_text_format_rendering();
    show_json_format_rendering();
    show_sink_config_usage();
    show_best_practices();

    println!("\n✓ 所有输出格式示例演示完成");
}

/// 展示 OutputFormat 枚举变体
fn show_output_format_variants() {
    print_section("1. OutputFormat 枚举变体");

    let text = OutputFormat::Text;
    let json = OutputFormat::Json;
    let default = OutputFormat::default();

    println!("OutputFormat::Text    = {:?}", text);
    println!("OutputFormat::Json    = {:?}", json);
    println!("OutputFormat::default() = {:?}", default);
    println!("\n默认格式: {:?}（人类可读文本）", default);
    assert_eq!(default, OutputFormat::Text);

    println!("\n等价比较：");
    println!(
        "  Text == Text → {}",
        OutputFormat::Text == OutputFormat::Text
    );
    println!(
        "  Text == Json → {}",
        OutputFormat::Text == OutputFormat::Json
    );
}

/// 展示 FromStr 解析
fn show_from_str_parsing() {
    print_section("2. FromStr 字符串解析");

    println!("字符串 → OutputFormat 解析：\n");

    let cases = ["text", "Text", "TEXT", "json", "Json", "JSON"];
    for s in &cases {
        match OutputFormat::from_str(s) {
            Ok(fmt) => println!("  {:?} → {:?} ✓", s, fmt),
            Err(e) => println!("  {:?} → 错误: {}", s, e),
        }
    }

    // 验证解析结果
    assert_eq!(OutputFormat::from_str("text").unwrap(), OutputFormat::Text);
    assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);

    // 无效值
    let invalid = OutputFormat::from_str("xml");
    assert!(invalid.is_err());
    println!("\n  {:?} → 错误（无效值）: {}", "xml", invalid.unwrap_err());
}

/// 展示 Display trait
fn show_display_trait() {
    print_section("3. Display trait 输出");

    println!("OutputFormat → 字符串：\n");
    println!("  OutputFormat::Text → \"{}\"", OutputFormat::Text);
    println!("  OutputFormat::Json → \"{}\"", OutputFormat::Json);

    assert_eq!(format!("{}", OutputFormat::Text), "text");
    assert_eq!(format!("{}", OutputFormat::Json), "json");
}

/// 展示 Text 格式的日志渲染效果
fn show_text_format_rendering() {
    print_section("4. Text 格式渲染（默认）");

    let template = LogTemplate::default();
    let record = create_sample_record();

    let rendered = template.render(&record);
    println!("LogTemplate::default() 渲染：");
    println!("  {}", rendered);
    println!("\n特点：");
    println!("  - 人类可读，适合控制台和日志文件");
    println!("  - 包含时间戳、级别、目标、消息");
    println!("  - 结构化字段以 JSON 附加在末尾");
}

/// 展示 Json 格式的日志渲染效果
fn show_json_format_rendering() {
    print_section("5. Json 格式渲染（NDJSON）");

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

    println!("NDJSON 格式输出：");
    println!("  {}", json_line);
    println!("\n特点：");
    println!("  - 机器可解析，适合日志采集系统");
    println!("  - 每行一个完整 JSON 对象");
    println!("  - 可直接被 Fluentd/Logstash/Promtail 等采集");
    println!("  - 支持嵌套结构化字段");

    // 验证 JSON 合法性
    let parsed: Value = serde_json::from_str(&json_line).unwrap();
    assert_eq!(parsed["level"], "INFO");
    assert_eq!(parsed["message"], "用户登录成功");
    println!("\n  ✓ JSON 输出可被标准解析器正确解析");
}

/// 展示在 Sink 配置中使用 OutputFormat
fn show_sink_config_usage() {
    print_section("6. 在 Sink 配置中使用 OutputFormat");

    use inklog::config::{ConsoleSinkConfig, FileSinkConfig};

    // Console Sink 使用 JSON 格式
    let console_cfg = ConsoleSinkConfig {
        enabled: true,
        colored: false,
        stderr_levels: vec![],
        masking_enabled: false,
        output_format: OutputFormat::Json,
    };
    println!("Console Sink 配置（JSON 格式）：");
    println!("  output_format = {:?}", console_cfg.output_format);

    // File Sink 使用 Text 格式
    let file_cfg = FileSinkConfig {
        enabled: true,
        output_format: OutputFormat::Text,
        ..Default::default()
    };
    println!("\nFile Sink 配置（Text 格式）：");
    println!("  output_format = {:?}", file_cfg.output_format);

    println!("\nTOML 配置示例：");
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
    print_section("7. 最佳实践");

    println!("OutputFormat 选择指南：\n");

    println!("1. Text 格式适用场景：");
    println!("   - 开发环境控制台输出");
    println!("   - 人类直接阅读的日志文件");
    println!("   - 调试和故障排查");

    println!("\n2. Json 格式适用场景：");
    println!("   - 生产环境结构化日志采集");
    println!("   - 日志分析平台（ELK/Loki/Datadog）");
    println!("   - 需要字段级查询和过滤的场景");

    println!("\n3. 混合使用：");
    println!("   - Console → Text（便于开发查看）");
    println!("   - File → Json（便于采集分析）");
    println!("   - Database → 结构化字段直接入库");

    println!("\n4. 性能考虑：");
    println!("   - Text 格式渲染更快（简单字符串拼接）");
    println!("   - Json 格式需序列化（约 2-3x 开销）");
    println!("   - 高吞吐场景建议 File 用 Text，采集层转 Json");
}

/// 创建示例日志记录
fn create_sample_record() -> LogRecord {
    let mut record = LogRecord::new(
        Level::INFO,
        "my_app::auth".to_string(),
        "用户登录成功".to_string(),
    );
    record.fields = HashMap::from([
        ("user_id".to_string(), Value::Number(12345.into())),
        ("action".to_string(), Value::String("login".to_string())),
    ]);
    record
}
