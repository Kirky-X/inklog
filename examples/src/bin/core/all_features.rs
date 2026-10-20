// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 完整功能演示
//!
//! 综合演示 inklog 的所有主要功能模块及其组合使用。
//!
//! # 内容
//!
//! 1. 数据脱敏（DataMasker）：内置规则 + 自定义规则 + 验证
//! 2. 日志模板（LogTemplate）：多种格式渲染 + 验证
//! 3. 日志级别控制：级别过滤 + `set_level` 运行时热调
//! 4. 结构化日志：带字段的日志记录（字段经 `{fields}` 占位符落屏）
//! 5. 路径验证（PathValidator）：安全路径检查
//! 6. 对象池（ObjectPool）：高性能对象复用
//! 7. 功能组合：多模块协同工作流
//! 8. 最佳实践与使用建议
//!
//! 全局 tracing subscriber 在首个 `LoggerManager` 构建时安装，进程内只安装一次；
//! 因此本示例全程只保留一个 LoggerManager，级别切换用 `set_level` 热调，
//! 而不是重建 logger（重复构建的 logger 收不到 tracing 记录）。
//!
//! # 运行
//!
//! ```bash
//! cargo run --package inklog-examples --bin all_features
//! ```

use inklog::LogRecord;
use inklog::tracing::Level;
use inklog::{
    DataMasker, LogTemplate, LoggerManager, ObjectPool, PathValidator, PathValidatorConfig,
};
use inklog_examples::common::{tr, tr_args};
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-all-features-inklog-full-feature-demo"));

    show_data_masking();
    show_log_template();

    // 全局 tracing subscriber 只安装一次：这里构建唯一的 LoggerManager，并让它
    // 存活到功能 3、4 演示结束。重复构建第二个 logger 不会接管全局订阅者，
    // 其记录会静默丢失——切换级别请用 set_level 热调。
    let logger = LoggerManager::builder()
        .level("debug")
        .console(true)
        // {fields} 占位符把结构化字段渲染为 key=value（无字段的记录渲染为空）
        .format("{timestamp} [{level}] {target} - {message} {fields}")
        .build()
        .await?;

    show_log_levels(&logger).await?;
    show_structured_logging().await;

    // shutdown 会等待 channel 中的日志全部写出
    logger.shutdown()?;

    show_path_validation();
    show_object_pool().await?;
    show_combined_workflow();
    show_best_practices();

    println!(
        "\n{}",
        tr("example-all-features-all-feature-demos-completed")
    );
    Ok(())
}

/// 功能 1：数据脱敏
///
/// 演示 DataMasker 的内置规则和自定义规则，包含验证逻辑。
fn show_data_masking() {
    println!("{}\n", tr("example-all-features-feature-1-data-masking"));

    let masker = DataMasker::new();
    let input = "user login: email=test@example.com, phone=13812345678";
    let masked = masker.mask(input);
    println!("{}", tr("example-all-features-built-in-rule-masking"));
    println!(
        "  {}",
        tr_args("example-all-features-raw", &[("input", &input)])
    );
    println!(
        "  {}",
        tr_args("example-all-features-masked", &[("masked", &masked)])
    );
    assert_ne!(
        masked, input,
        "masked content should differ from the original"
    );
    assert!(
        !masked.contains("test@example.com"),
        "the email should be masked"
    );
    assert!(
        !masked.contains("13812345678"),
        "the phone number should be masked"
    );
    println!(
        "  {}\n",
        tr("example-all-features-email-and-phone-number-masked")
    );

    let custom_rule = inklog::MaskRule::builder("order_id")
        .pattern(r"\bORD-\d{8}\b")
        .replacement("[ORDER_ID]")
        .build()
        .expect("Invalid regex pattern");
    let custom_masker = DataMasker::builder().add_rule(custom_rule).build();
    let order_input = "order ORD-20260803 created, related order ORD-20260101";
    let order_masked = custom_masker.mask(order_input);
    println!("{}", tr("example-all-features-custom-rule-masking"));
    println!(
        "  {}",
        tr_args(
            "example-all-features-raw-2",
            &[("order_input", &order_input)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-all-features-masked-2",
            &[("order_masked", &order_masked)]
        )
    );
    assert!(
        !order_masked.contains("ORD-20260803"),
        "the first order id should be masked"
    );
    assert!(
        order_masked.contains("[ORDER_ID]"),
        "should contain the replacement marker"
    );
    // 默认 apply_fn 为 regex.replace_all：替换全部匹配项
    assert!(
        !order_masked.contains("ORD-"),
        "all order ids should be masked"
    );
    println!(
        "  {}\n",
        tr("example-all-features-custom-order-id-rule-works-all")
    );

    let clean = "this is an ordinary log line with no sensitive data";
    let clean_masked = masker.mask(clean);
    assert_eq!(
        clean, clean_masked,
        "content is unchanged when there is no sensitive data"
    );
    println!("{}", tr("example-all-features-no-sensitive-data-test"));
    println!(
        "  {}\n",
        tr("example-all-features-content-unchanged-as-expected")
    );
}

/// 功能 2：日志模板
///
/// 演示 LogTemplate 的多种格式和渲染验证。
fn show_log_template() {
    println!("{}\n", tr("example-all-features-feature-2-log-templates"));

    let record = LogRecord::new(
        Level::INFO,
        "my_app::auth".to_string(),
        "user logged in successfully".to_string(),
    );

    let default_tpl = LogTemplate::default();
    let rendered = default_tpl.render(&record);
    println!("{}", tr("example-all-features-default-template-render"));
    println!("  Template: {{timestamp}} [{{level}}] {{target}} - {{message}}");
    println!(
        "  {}",
        tr_args("example-all-features-output", &[("rendered", &rendered)])
    );
    assert!(rendered.contains("INFO"), "should contain the log level");
    assert!(
        rendered.contains("user logged in successfully"),
        "should contain the message"
    );
    println!("  {}\n", tr("example-all-features-render-check-passed"));

    let simple_tpl = LogTemplate::new("[{level}] {message}");
    let simple_rendered = simple_tpl.render(&record);
    println!("{}", tr("example-all-features-simple-template-render"));
    println!("  Template: [{{level}}] {{message}}");
    println!(
        "  {}",
        tr_args(
            "example-all-features-output-2",
            &[("simple_rendered", &simple_rendered)]
        )
    );
    assert_eq!(simple_rendered, "[INFO] user logged in successfully");
    println!("  {}\n", tr("example-all-features-exact-match-verified"));

    // 模板只支持 {name} 占位符：不支持宽度/对齐语法（{level:<5} 会原样输出），
    // 字面花括号必须转义为 {{ }}，否则会被当作占位符起始
    let formats = [
        (
            "JSON style",
            r#"{{"level":"{level}","msg":"{message}","target":"{target}"}}"#,
        ),
        (
            "pipe separated",
            "{timestamp} | {level} | {target} | {message}",
        ),
        ("bordered", "│ {level} │ {target} │ {message}"),
    ];
    println!("{}", tr("example-all-features-multi-format-comparison"));
    for (name, fmt) in formats {
        let tpl = LogTemplate::new(fmt);
        let line = tpl.render(&record);
        println!("  {}: {}", name, line);
        if name == "JSON style" {
            // 转义正确时输出为合法 JSON，可被下游日志管道直接解析
            serde_json::from_str::<serde_json::Value>(&line)
                .expect("the JSON style template render should be valid JSON");
        }
    }
    println!(
        "  {}\n",
        tr("example-all-features-json-style-output-parses-with")
    );
}

/// 功能 3：日志级别控制
///
/// 演示级别过滤与 `set_level` 运行时热调。全局 tracing subscriber 在首个
/// `LoggerManager` 构建时安装且进程内只安装一次，切换级别必须用 `set_level`
/// 热调同一实例；重建 LoggerManager 不会接管全局订阅者，其记录会静默丢失。
async fn show_log_levels(logger: &LoggerManager) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-all-features-feature-3-log-level-control")
    );

    println!("{}", tr("example-all-features-log-level-debug"));
    tracing::trace!(
        "{}",
        tr("example-all-features-trace-not-emitted-below-debug")
    );
    tracing::debug!("{}", tr("example-all-features-debug-emitted"));
    tracing::info!("{}", tr("example-all-features-info-emitted"));
    // 控制台 sink 经 worker 异步写出，短暂等待让记录落屏；
    // 生产代码无需 sleep，shutdown 会等待队列排空
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!(
        "  {}\n",
        tr("example-all-features-trace-filtered-out-debug-and")
    );

    // 运行时热调：无需重启进程或重建 logger
    logger.set_level(None, "warn")?;
    println!(
        "{}",
        tr("example-all-features-after-set-level-none-warn-hot")
    );
    tracing::info!("{}", tr("example-all-features-info-not-emitted-below-warn"));
    tracing::warn!("{}", tr("example-all-features-warn-emitted"));
    tracing::error!("{}", tr("example-all-features-error-emitted"));
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!(
        "  {}\n",
        tr("example-all-features-info-debug-trace-filtered-out")
    );

    // 恢复 info：功能 4 的 info 级结构化记录需要可见；热调本身会留下一条
    // info 级审计记录（"log level changed at runtime"）
    logger.set_level(None, "info")?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!(
        "  {}\n",
        tr("example-all-features-level-restored-to-info-set")
    );

    Ok(())
}

/// 功能 4：结构化日志
///
/// 演示带结构化字段的日志记录。字段随记录进入 sink，由 main 中配置的
/// `{fields}` 占位符渲染为 `key=value`。
async fn show_structured_logging() {
    println!(
        "{}\n",
        tr("example-all-features-feature-4-structured-logging")
    );

    // 按本文件声明的最佳实践（DataMasker 应在日志写入前调用，见功能 7 与最佳实践 1）：
    // 敏感值必须先经 DataMasker 脱敏，再写入日志
    let masker = DataMasker::new();

    // 用户操作：敏感值（邮箱/手机号）先脱敏再记录
    let email = masker.mask("alice@example.com");
    let phone = masker.mask("13812345678");
    tracing::info!(
        user_email = %email,
        phone = %phone,
        action = "purchase",
        amount = 99.99,
        currency = "USD",
        "user purchased an item"
    );

    // 请求处理：非敏感字段直接记录（数值类型直接传，无需转字符串）
    tracing::info!(
        request_id = "req-abc-123",
        method = "POST",
        path = "/api/orders",
        status = 201,
        latency_ms = 42,
        "API request completed"
    );

    // 错误处理：运行指标 + error_code
    tracing::error!(
        error_code = "PAYMENT_FAILED",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "payment processing failed"
    );

    // 等待异步 worker 把记录写到控制台（生产代码由 shutdown 排空队列）
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!(
        "✓ user operation log (user_email and phone masked by DataMasker first, action, amount, currency)"
    );
    println!(
        "{}",
        tr("example-all-features-request-log-request-id-method")
    );
    println!(
        "{}",
        tr("example-all-features-error-log-error-code-component")
    );
    println!("  fields are appended after the message as key=value (template {{fields}}); use");
    println!(
        "  {}\n",
        tr("example-all-features-loggermanager-builder-console")
    );
}

/// 功能 5：路径验证
///
/// 演示 PathValidator 的安全路径检查。
fn show_path_validation() {
    println!("{}\n", tr("example-all-features-feature-5-path-validation"));

    let validator = PathValidator::with_config(PathValidatorConfig::default());

    let test_cases: Vec<(&str, bool, &str)> = vec![
        ("logs/app.log", true, "normal relative path"),
        ("/var/log/app.log", true, "absolute path"),
        ("../etc/passwd", false, "directory traversal attack"),
        ("/etc/shadow", false, "system sensitive file"),
    ];

    println!("{:<25} {:<10} {:<10} note", "path", "expected", "actual");
    println!("{}", "-".repeat(65));

    for (path, expected_valid, desc) in test_cases {
        let result = validator.validate(Path::new(path));
        let is_valid = result.valid;
        let status = if is_valid == expected_valid {
            "✓"
        } else {
            "✗"
        };
        println!(
            "{:<25} {:<10} {:<10} {} {}",
            path,
            if expected_valid { "allow" } else { "reject" },
            if is_valid { "allow" } else { "reject" },
            status,
            desc
        );
        assert_eq!(
            is_valid, expected_valid,
            "path '{}' validation result is not as expected",
            path
        );
    }
    println!(
        "  {}\n",
        tr("example-all-features-all-path-validation-results-are")
    );
}

/// 功能 6：对象池
///
/// 演示 ObjectPool 的高性能对象复用。
async fn show_object_pool() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-all-features-feature-6-object-pool-objectpool")
    );

    let pool = ObjectPool::<String, String>::new().await?;

    // 存取操作
    pool.put(&"key1".to_string(), "value1".to_string()).await?;
    pool.put(&"key2".to_string(), "value2".to_string()).await?;

    let v1 = pool.get(&"key1".to_string()).await?;
    let v2 = pool.get(&"key2".to_string()).await?;
    let v3 = pool.get(&"missing".to_string()).await?;

    assert_eq!(v1, Some("value1".to_string()));
    assert_eq!(v2, Some("value2".to_string()));
    assert_eq!(v3, None);
    println!(
        "{}",
        tr("example-all-features-object-pool-get-put-verified")
    );
    println!(
        "  {}",
        tr_args(
            "example-all-features-key1-key2-missing",
            &[
                ("v1", &format!("{:?}", v1)),
                ("v2", &format!("{:?}", v2)),
                ("v3", &format!("{:?}", v3))
            ]
        )
    );
    println!();

    Ok(())
}

/// 功能 7：多模块组合工作流
///
/// 演示脱敏 + 模板 + 日志的组合使用。
fn show_combined_workflow() {
    println!(
        "{}\n",
        tr("example-all-features-feature-7-multi-module-combined")
    );

    let masker = DataMasker::new();
    println!("{}", tr("example-all-features-1-create-a-datamasker"));

    let template = LogTemplate::new("{timestamp} [{level}] {message}");
    println!("{}", tr("example-all-features-2-create-a-logtemplate"));

    let raw_message = "user test@example.com logged in, phone 13812345678";
    println!(
        "{}",
        tr_args(
            "example-all-features-3-raw-log",
            &[("raw_message", &raw_message)]
        )
    );

    let safe_message = masker.mask(raw_message);
    println!(
        "{}",
        tr_args(
            "example-all-features-4-masked",
            &[("safe_message", &safe_message)]
        )
    );
    assert!(!safe_message.contains("test@example.com"));

    let record = LogRecord::new(Level::INFO, "app".to_string(), safe_message);
    let rendered = template.render(&record);
    println!(
        "{}",
        tr_args(
            "example-all-features-5-template-render",
            &[("rendered", &rendered)]
        )
    );
    assert!(rendered.contains("INFO"));
    println!(
        "\n  {}\n",
        tr("example-all-features-masking-template-logging")
    );
}

/// 最佳实践建议
fn show_best_practices() {
    println!("{}\n", tr("example-all-features-best-practices"));

    println!(
        "{}",
        tr("example-all-features-1-feature-combination-principles")
    );
    println!(
        "   {}",
        tr("example-all-features-call-datamasker-before-writing")
    );
    println!(
        "   {}",
        tr("example-all-features-logtemplate-unifies-the-log")
    );
    println!(
        "   {}",
        tr("example-all-features-pathvalidator-validates-file")
    );

    println!("\n{}", tr("example-all-features-2-masking-rule-design"));
    println!(
        "   {}",
        tr("example-all-features-prefer-built-in-rules-email")
    );
    println!(
        "   {}",
        tr("example-all-features-give-custom-rules-meaningful")
    );
    println!(
        "   {}",
        tr("example-all-features-review-masking-rules-regularly")
    );

    println!("\n{}", tr("example-all-features-3-log-level-strategy"));
    println!(
        "   {}",
        tr("example-all-features-trace-development-only-disabled")
    );
    println!(
        "   {}",
        tr("example-all-features-debug-development-test-enable")
    );
    println!(
        "   {}",
        tr("example-all-features-info-production-default-records")
    );
    println!(
        "   {}",
        tr("example-all-features-warn-recoverable-exceptions")
    );
    println!(
        "   {}",
        tr("example-all-features-error-functionality-failure")
    );

    println!(
        "\n{}",
        tr("example-all-features-4-structured-field-conventions")
    );
    println!(
        "   {}",
        tr("example-all-features-use-snake-case-names-user-id")
    );
    println!(
        "   {}",
        tr("example-all-features-pass-numeric-values-directly")
    );
    println!(
        "   {}",
        tr("example-all-features-always-include-component-or")
    );
    println!(
        "   {}",
        tr("example-all-features-error-logs-must-include-error")
    );
}
