// SPDX-License-Identifier: MIT
//! 完整功能演示
//!
//! 综合演示 inklog 的所有主要功能模块及其组合使用。
//!
//! # 内容
//!
//! 1. 数据脱敏（DataMasker）：内置规则 + 自定义规则 + 验证
//! 2. 日志模板（LogTemplate）：多种格式渲染 + 验证
//! 3. 日志级别控制：不同级别的过滤行为
//! 4. 结构化日志：带字段的日志记录
//! 5. 路径验证（PathValidator）：安全路径检查
//! 6. 对象池（ObjectPool）：高性能对象复用
//! 7. 功能组合：多模块协同工作流
//! 8. 最佳实践与使用建议
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin all_features
//! ```

use inklog::tracing::Level;
use inklog::LogRecord;
use inklog::{
    DataMasker, LogTemplate, LoggerManager, ObjectPool, PathValidator, PathValidatorConfig,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog 完整功能演示 ===\n");

    show_data_masking();
    show_log_template();
    show_log_levels_with_logger().await?;
    show_structured_logging();
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

    // 1.1 内置规则
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

    // 1.2 自定义规则
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
    // 注意: 默认 replace_count=1，仅替换第一个匹配
    println!("  ✓ 自定义订单号规则生效（默认替换首个匹配）\n");

    // 1.3 无敏感信息
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

    // 2.1 默认模板
    let default_tpl = LogTemplate::default();
    let rendered = default_tpl.render(&record);
    println!("默认模板渲染：");
    println!("  模板: {{timestamp}} [{{level}}] {{target}} - {{message}}");
    println!("  输出: {}", rendered);
    assert!(rendered.contains("INFO"), "应包含日志级别");
    assert!(rendered.contains("用户登录成功"), "应包含消息内容");
    println!("  ✓ 渲染验证通过\n");

    // 2.2 简洁模板
    let simple_tpl = LogTemplate::new("[{level}] {message}");
    let simple_rendered = simple_tpl.render(&record);
    println!("简洁模板渲染：");
    println!("  模板: [{{level}}] {{message}}");
    println!("  输出: {}", simple_rendered);
    assert_eq!(simple_rendered, "[INFO] 用户登录成功");
    println!("  ✓ 精确匹配验证通过\n");

    // 2.3 多格式对比
    let formats = vec![
        (
            "JSON 风格",
            r#"{"level":"{level}","msg":"{message}","target":"{target}"}"#,
        ),
        ("管道分隔", "{timestamp} | {level} | {target} | {message}"),
        ("带边框", "│ {level:<5} │ {target} │ {message}"),
    ];
    println!("多格式对比：");
    for (name, fmt) in formats {
        let tpl = LogTemplate::new(fmt);
        println!("  {}: {}", name, tpl.render(&record));
    }
    println!();
}

/// 功能 3：日志级别控制
///
/// 演示不同日志级别配置下的过滤行为。
async fn show_log_levels_with_logger() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 功能 3：日志级别控制 ---\n");

    // 3.1 debug 级别
    let logger = LoggerManager::builder()
        .level("debug")
        .console(true)
        .build()
        .await?;

    println!("日志级别 = debug：");
    tracing::trace!("TRACE - 不会输出（低于 debug）");
    tracing::debug!("DEBUG - 会输出");
    tracing::info!("INFO - 会输出");
    println!("  ✓ trace 被过滤，debug 及以上可见\n");

    logger.shutdown()?;

    // 3.2 warn 级别
    let logger = LoggerManager::builder()
        .level("warn")
        .console(true)
        .build()
        .await?;

    println!("日志级别 = warn：");
    tracing::info!("INFO - 不会输出（低于 warn）");
    tracing::warn!("WARN - 会输出");
    tracing::error!("ERROR - 会输出");
    println!("  ✓ info/debug/trace 被过滤，warn 及以上可见\n");

    logger.shutdown()?;
    Ok(())
}

/// 功能 4：结构化日志
///
/// 演示带结构化字段的日志记录。
fn show_structured_logging() {
    println!("--- 功能 4：结构化日志 ---\n");

    // 用户操作
    tracing::info!(
        user_id = 12345,
        action = "purchase",
        amount = 99.99,
        currency = "USD",
        "用户购买商品"
    );
    println!("✓ 用户操作日志（user_id, action, amount, currency）");

    // 请求处理
    tracing::info!(
        request_id = "req-abc-123",
        method = "POST",
        path = "/api/orders",
        status = 201,
        latency_ms = 42,
        "API 请求完成"
    );
    println!("✓ 请求日志（request_id, method, path, status, latency_ms）");

    // 错误处理
    tracing::error!(
        error_code = "PAYMENT_FAILED",
        component = "payment",
        retry_count = 3,
        max_retries = 5,
        "支付处理失败"
    );
    println!("✓ 错误日志（error_code, component, retry_count）\n");
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

    println!("{:<25} {:<10} {:<10} {}", "路径", "期望", "实际", "说明");
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

    // 步骤 1：创建脱敏器
    let masker = DataMasker::new();
    println!("1. 创建 DataMasker");

    // 步骤 2：创建模板
    let template = LogTemplate::new("{timestamp} [{level}] {message}");
    println!("2. 创建 LogTemplate");

    // 步骤 3：模拟含敏感信息的日志
    let raw_message = "用户 test@example.com 登录成功，手机号 13812345678";
    println!("3. 原始日志: {}", raw_message);

    // 步骤 4：脱敏
    let safe_message = masker.mask(raw_message);
    println!("4. 脱敏后:   {}", safe_message);
    assert!(!safe_message.contains("test@example.com"));

    // 步骤 5：用模板渲染
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
