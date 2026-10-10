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
//! 3. 日志级别控制：级别过滤 + `set_level()` 运行时热调
//! 4. 结构化日志：带字段的日志记录（字段经 `{fields}` 占位符落屏）
//! 5. 路径验证（PathValidator）：安全路径检查
//! 6. 对象池（ObjectPool）：高性能对象复用
//! 7. 功能组合：多模块协同工作流
//! 8. 最佳实践与使用建议
//!
//! 全局 tracing subscriber 在首个 `LoggerManager` 构建时安装，进程内只安装一次；
//! 因此本示例全程只保留一个 LoggerManager，级别切换用 `set_level()` 热调，
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
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog 完整功能演示 ===\n");

    show_data_masking();
    show_log_template();

    // 全局 tracing subscriber 只安装一次：这里构建唯一的 LoggerManager，并让它
    // 存活到功能 3、4 演示结束。重复构建第二个 logger 不会接管全局订阅者，
    // 其记录会静默丢失——切换级别请用 set_level() 热调。
    let logger = LoggerManager::builder()
        .level("debug")
        .console(true)
        // {fields} 占位符把结构化字段渲染为 key=value（无字段的记录渲染为空）
        .format("{timestamp} [{level}] {target} - {message} {fields}")
        .build()
        .await?;

    show_log_levels(&logger).await?;
    show_structured_logging().await;

    // shutdown() 会等待 channel 中的日志全部写出
    logger.shutdown()?;

    show_path_validation();
    show_object_pool().await?;
    show_combined_workflow();
    show_best_practices();

    println!("\n所有功能演示完毕。");
    Ok(())
}

/// 功能 1：数据脱敏
///
/// 演示 DataMasker 的内置规则和自定义规则，包含验证逻辑。
fn show_data_masking() {
    println!("--- 功能 1：数据脱敏 (DataMasker) ---\n");

    let masker = DataMasker::new();
    let input = "用户登录: email=test@example.com, phone=13812345678";
    let masked = masker.mask(input);
    println!("内置规则脱敏：");
    println!("  原始: {}", input);
    println!("  脱敏: {}", masked);
    assert_ne!(masked, input, "脱敏后内容应与原始不同");
    assert!(!masked.contains("test@example.com"), "邮箱应被脱敏");
    assert!(!masked.contains("13812345678"), "手机号应被脱敏");
    println!("  ✓ 邮箱和手机号已脱敏\n");

    let custom_rule = inklog::MaskRule::builder("order_id")
        .pattern(r"\bORD-\d{8}\b")
        .replacement("[ORDER_ID]")
        .build()
        .expect("Invalid regex pattern");
    let custom_masker = DataMasker::builder().add_rule(custom_rule).build();
    let order_input = "订单 ORD-20260803 已创建，关联订单 ORD-20260101";
    let order_masked = custom_masker.mask(order_input);
    println!("自定义规则脱敏：");
    println!("  原始: {}", order_input);
    println!("  脱敏: {}", order_masked);
    assert!(
        !order_masked.contains("ORD-20260803"),
        "第一个订单号应被脱敏"
    );
    assert!(order_masked.contains("[ORDER_ID]"), "应包含替换标记");
    // 默认 apply_fn 为 regex.replace_all：替换全部匹配项
    assert!(!order_masked.contains("ORD-"), "所有订单号都应被脱敏");
    println!("  ✓ 自定义订单号规则生效（默认替换全部匹配）\n");

    let clean = "这是一条普通日志，没有敏感信息";
    let clean_masked = masker.mask(clean);
    assert_eq!(clean, clean_masked, "无敏感信息时内容不变");
    println!("无敏感信息测试：");
    println!("  ✓ 内容保持不变（符合预期）\n");
}

/// 功能 2：日志模板
///
/// 演示 LogTemplate 的多种格式和渲染验证。
fn show_log_template() {
    println!("--- 功能 2：日志模板 (LogTemplate) ---\n");

    let record = LogRecord::new(
        Level::INFO,
        "my_app::auth".to_string(),
        "用户登录成功".to_string(),
    );

    let default_tpl = LogTemplate::default();
    let rendered = default_tpl.render(&record);
    println!("默认模板渲染：");
    println!("  模板: {{timestamp}} [{{level}}] {{target}} - {{message}}");
    println!("  输出: {}", rendered);
    assert!(rendered.contains("INFO"), "应包含日志级别");
    assert!(rendered.contains("用户登录成功"), "应包含消息内容");
    println!("  ✓ 渲染验证通过\n");

    let simple_tpl = LogTemplate::new("[{level}] {message}");
    let simple_rendered = simple_tpl.render(&record);
    println!("简洁模板渲染：");
    println!("  模板: [{{level}}] {{message}}");
    println!("  输出: {}", simple_rendered);
    assert_eq!(simple_rendered, "[INFO] 用户登录成功");
    println!("  ✓ 精确匹配验证通过\n");

    // 模板只支持 {name} 占位符：不支持宽度/对齐语法（{level:<5} 会原样输出），
    // 字面花括号必须转义为 {{ }}，否则会被当作占位符起始
    let formats = [
        (
            "JSON 风格",
            r#"{{"level":"{level}","msg":"{message}","target":"{target}"}}"#,
        ),
        ("管道分隔", "{timestamp} | {level} | {target} | {message}"),
        ("带边框", "│ {level} │ {target} │ {message}"),
    ];
    println!("多格式对比：");
    for (name, fmt) in formats {
        let tpl = LogTemplate::new(fmt);
        let line = tpl.render(&record);
        println!("  {}: {}", name, line);
        if name == "JSON 风格" {
            // 转义正确时输出为合法 JSON，可被下游日志管道直接解析
            serde_json::from_str::<serde_json::Value>(&line)
                .expect("JSON 风格模板渲染结果应为合法 JSON");
        }
    }
    println!("  ✓ JSON 风格输出可被 serde_json 解析\n");
}

/// 功能 3：日志级别控制
///
/// 演示级别过滤与 `set_level()` 运行时热调。全局 tracing subscriber 在首个
/// `LoggerManager` 构建时安装且进程内只安装一次，切换级别必须用 `set_level()`
/// 热调同一实例；重建 LoggerManager 不会接管全局订阅者，其记录会静默丢失。
async fn show_log_levels(logger: &LoggerManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 功能 3：日志级别控制 ---\n");

    println!("日志级别 = debug：");
    tracing::trace!("TRACE - 不会输出（低于 debug）");
    tracing::debug!("DEBUG - 会输出");
    tracing::info!("INFO - 会输出");
    // 控制台 sink 经 worker 异步写出，短暂等待让记录落屏；
    // 生产代码无需 sleep，shutdown() 会等待队列排空
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("  ✓ trace 被过滤，debug 及以上可见\n");

    // 运行时热调：无需重启进程或重建 logger
    logger.set_level(None, "warn")?;
    println!("set_level(None, \"warn\") 热调后：");
    tracing::info!("INFO - 不会输出（低于 warn）");
    tracing::warn!("WARN - 会输出");
    tracing::error!("ERROR - 会输出");
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("  ✓ info/debug/trace 被过滤，warn 及以上可见\n");

    // 恢复 info：功能 4 的 info 级结构化记录需要可见；热调本身会留下一条
    // info 级审计记录（"log level changed at runtime"）
    logger.set_level(None, "info")?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("  ✓ 级别已恢复为 info（set_level 热调，logger 全程存活）\n");

    Ok(())
}

/// 功能 4：结构化日志
///
/// 演示带结构化字段的日志记录。字段随记录进入 sink，由 main() 中配置的
/// `{fields}` 占位符渲染为 `key=value`。
async fn show_structured_logging() {
    println!("--- 功能 4：结构化日志 ---\n");

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
        "用户购买商品"
    );

    // 请求处理：非敏感字段直接记录（数值类型直接传，无需转字符串）
    tracing::info!(
        request_id = "req-abc-123",
        method = "POST",
        path = "/api/orders",
        status = 201,
        latency_ms = 42,
        "API 请求完成"
    );

    // 错误处理：运行指标 + error_code
    tracing::error!(
        error_code = "PAYMENT_FAILED",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "支付处理失败"
    );

    // 等待异步 worker 把记录写到控制台（生产代码由 shutdown() 排空队列）
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("✓ 用户操作日志（user_email, phone 先经 DataMasker 脱敏, action, amount, currency）");
    println!("✓ 请求日志（request_id, method, path, status, latency_ms）");
    println!("✓ 错误日志（error_code, component, retry_count）");
    println!("  字段以 key=value 追加在消息后（模板 {{fields}}）；需要整条 JSON");
    println!("  记录时可用 LoggerManager::builder().console_json(true)\n");
}

/// 功能 5：路径验证
///
/// 演示 PathValidator 的安全路径检查。
fn show_path_validation() {
    println!("--- 功能 5：路径验证 (PathValidator) ---\n");

    let validator = PathValidator::with_config(PathValidatorConfig::default());

    let test_cases: Vec<(&str, bool, &str)> = vec![
        ("logs/app.log", true, "正常相对路径"),
        ("/var/log/app.log", true, "绝对路径"),
        ("../etc/passwd", false, "目录遍历攻击"),
        ("/etc/shadow", false, "系统敏感文件"),
    ];

    println!("{:<25} {:<10} {:<10} 说明", "路径", "期望", "实际");
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
            if expected_valid { "允许" } else { "拒绝" },
            if is_valid { "允许" } else { "拒绝" },
            status,
            desc
        );
        assert_eq!(
            is_valid, expected_valid,
            "路径 '{}' 验证结果不符合预期",
            path
        );
    }
    println!("  ✓ 所有路径验证结果符合预期\n");
}

/// 功能 6：对象池
///
/// 演示 ObjectPool 的高性能对象复用。
async fn show_object_pool() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 功能 6：对象池 (ObjectPool) ---\n");

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
    println!("✓ 对象池存取验证通过");
    println!("  key1 = {:?}, key2 = {:?}, missing = {:?}", v1, v2, v3);
    println!();

    Ok(())
}

/// 功能 7：多模块组合工作流
///
/// 演示脱敏 + 模板 + 日志的组合使用。
fn show_combined_workflow() {
    println!("--- 功能 7：多模块组合工作流 ---\n");

    let masker = DataMasker::new();
    println!("1. 创建 DataMasker");

    let template = LogTemplate::new("{timestamp} [{level}] {message}");
    println!("2. 创建 LogTemplate");

    let raw_message = "用户 test@example.com 登录成功，手机号 13812345678";
    println!("3. 原始日志: {}", raw_message);

    let safe_message = masker.mask(raw_message);
    println!("4. 脱敏后:   {}", safe_message);
    assert!(!safe_message.contains("test@example.com"));

    let record = LogRecord::new(Level::INFO, "app".to_string(), safe_message);
    let rendered = template.render(&record);
    println!("5. 模板渲染: {}", rendered);
    assert!(rendered.contains("INFO"));
    println!("\n  ✓ 脱敏 → 模板 → 日志 完整工作流验证通过\n");
}

/// 最佳实践建议
fn show_best_practices() {
    println!("--- 最佳实践 ---\n");

    println!("1. 功能组合原则：");
    println!("   - DataMasker 应在日志写入前调用（脱敏 → 记录）");
    println!("   - LogTemplate 用于统一日志格式，建议全局使用同一模板");
    println!("   - PathValidator 用于验证用户输入的文件路径");

    println!("\n2. 脱敏规则设计：");
    println!("   - 优先使用内置规则（email, phone, password 等）");
    println!("   - 自定义规则使用有意义的名称便于维护");
    println!("   - 定期审查脱敏规则，确保覆盖新增敏感字段");

    println!("\n3. 日志级别策略：");
    println!("   - trace: 仅开发环境，生产环境禁用");
    println!("   - debug: 开发/测试环境，生产环境按需临时开启");
    println!("   - info: 生产默认，记录关键业务事件");
    println!("   - warn: 可恢复异常，需关注但不需立即处理");
    println!("   - error: 功能失败，需要人工介入");

    println!("\n4. 结构化字段规范：");
    println!("   - 使用 snake_case 命名（user_id, request_id）");
    println!("   - 数值类型直接传（status = 200，不用字符串）");
    println!("   - 必须包含: component 或 target 标识来源");
    println!("   - 错误日志必须包含: error_code");
}
