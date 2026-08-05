// SPDX-License-Identifier: MIT
//! 错误处理示例（Layer 0 零依赖）
//!
//! 演示 `inklog::InklogError` 和 `inklog::InklogResult` 的使用：
//!
//! 1. 所有错误变体的构造与 Display
//! 2. `safe_message()` 敏感数据自动脱敏
//! 3. `localized_message()` 国际化错误消息
//! 4. 便捷构造方法（`database_error`、`encryption_error`）
//! 5. From trait 自动转换（io::Error、serde_json::Error、toml::Error）
//! 6. 实际错误处理模式
//!
//! # 运行
//! ```bash
//! cargo run --bin error_handling
//! ```

use inklog::{InklogError, InklogResult};
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog 错误处理示例");

    show_error_variants();
    show_display_trait();
    show_safe_message();
    show_localized_message();
    show_convenience_constructors();
    show_from_conversions();
    show_result_type_alias();
    show_error_handling_patterns();

    println!("\n✓ 所有错误处理示例演示完成");
}

/// 展示所有错误变体的构造
fn show_error_variants() {
    print_section("1. InklogError 所有变体");

    let errors: Vec<(&str, InklogError)> = vec![
        (
            "ConfigError",
            InklogError::ConfigError("invalid log level".into()),
        ),
        (
            "IoError",
            InklogError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file missing",
            )),
        ),
        (
            "DatabaseError",
            InklogError::database_error("connection refused"),
        ),
        (
            "CacheError",
            InklogError::CacheError("connection lost".into()),
        ),
        (
            "EncryptionError",
            InklogError::encryption_error("invalid key length"),
        ),
        (
            "Shutdown",
            InklogError::Shutdown("timeout waiting for workers".into()),
        ),
        (
            "ChannelError",
            InklogError::ChannelError("channel closed".into()),
        ),
        (
            "CompressionError",
            InklogError::CompressionError("zstd decompress failed".into()),
        ),
        (
            "RuntimeError",
            InklogError::RuntimeError("worker thread panicked".into()),
        ),
        (
            "HttpServerError",
            InklogError::HttpServerError("bind address in use".into()),
        ),
        ("Unknown", InklogError::Unknown("unexpected state".into())),
    ];

    println!("{:<20} Display 输出", "变体");
    println!("{}", "-".repeat(70));
    for (name, err) in &errors {
        println!("{:<20} {}", name, err);
    }

    println!("\n  ✓ 共 {} 种错误变体", errors.len());
}

/// 展示 Display trait
fn show_display_trait() {
    print_section("2. Display trait 格式");

    let err = InklogError::ConfigError("invalid level 'xyz'".into());
    println!("错误: {}", err);
    assert_eq!(err.to_string(), "Configuration error: invalid level 'xyz'");

    let err = InklogError::ChannelError("disconnected".into());
    println!("错误: {}", err);
    assert_eq!(err.to_string(), "Channel error: disconnected");

    println!("\n  ✓ Display 格式: \"{{前缀}}: {{详情}}\"");
}

/// 展示 safe_message() 敏感数据脱敏
fn show_safe_message() {
    print_section("3. safe_message() 敏感数据脱敏");

    println!("safe_message() 自动脱敏错误消息中的敏感信息：\n");

    // AWS 密钥
    let err = InklogError::ConfigError("Failed to load AKIAIOSFODNN7EXAMPLE from config".into());
    println!("AWS 密钥脱敏：");
    println!("  原始: {}", err);
    println!("  安全: {}", err.safe_message());
    assert!(!err.safe_message().contains("AKIAIOSFODNN7EXAMPLE"));

    // 数据库连接串
    let err = InklogError::DatabaseError {
        message: "Connection failed: postgres://admin:secret@localhost/db".into(),
        source: None,
    };
    println!("\n数据库连接串脱敏：");
    println!("  原始: {}", err);
    println!("  安全: {}", err.safe_message());
    assert!(!err.safe_message().contains("secret"));

    // 用户路径
    let err = InklogError::ConfigError("Config not found at /home/user/.config/inklog.toml".into());
    println!("\n用户路径脱敏：");
    println!("  原始: {}", err);
    println!("  安全: {}", err.safe_message());
    assert!(!err.safe_message().contains("/home/user/"));

    // 邮箱
    let err = InklogError::ConfigError("Contact admin@example.com for help".into());
    println!("\n邮箱脱敏：");
    println!("  原始: {}", err);
    println!("  安全: {}", err.safe_message());
    assert!(!err.safe_message().contains("admin@example.com"));

    // 非敏感信息保持不变
    let err = InklogError::ConfigError("Invalid log level value".into());
    println!("\n非敏感信息保持不变：");
    println!("  原始: {}", err);
    println!("  安全: {}", err.safe_message());
    assert!(err.safe_message().contains("Invalid log level value"));

    println!("\n  ✓ safe_message() 正确脱敏敏感数据");
}

/// 展示 localized_message() 国际化
fn show_localized_message() {
    print_section("4. localized_message() 国际化消息");

    println!("localized_message() 根据当前 locale 返回翻译后的错误前缀：\n");

    let errors = vec![
        InklogError::ConfigError("bad value".into()),
        InklogError::database_error("timeout"),
        InklogError::CacheError("evicted".into()),
        InklogError::encryption_error("key expired"),
        InklogError::Shutdown("worker hang".into()),
        InklogError::ChannelError("full".into()),
        InklogError::CompressionError("corrupt data".into()),
        InklogError::RuntimeError("panic".into()),
        InklogError::HttpServerError("port in use".into()),
        InklogError::Unknown("something".into()),
    ];

    println!("{:<25} localized_message()", "错误类型");
    println!("{}", "-".repeat(70));
    for err in &errors {
        println!(
            "{:<25} {}",
            format!("{:?}", err).split('(').next().unwrap_or(""),
            err.localized_message()
        );
    }

    println!("\n  ✓ 所有错误变体均支持国际化消息");
}

/// 展示便捷构造方法
fn show_convenience_constructors() {
    print_section("5. 便捷构造方法");

    // database_error
    let err = InklogError::database_error("connection pool exhausted");
    println!("InklogError::database_error(\"connection pool exhausted\")：");
    println!("  Display: {}", err);
    match &err {
        InklogError::DatabaseError { message, source } => {
            println!("  message: {}", message);
            println!("  source:  {:?}", source.is_some());
        }
        _ => panic!("expected DatabaseError"),
    }

    // database_error_with_source
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let err = InklogError::database_error_with_source("table creation failed", io_err);
    println!("\nInklogError::database_error_with_source(...)：");
    println!("  Display: {}", err);
    match &err {
        InklogError::DatabaseError { source, .. } => {
            println!("  source:  {} (有错误链)", source.as_ref().unwrap());
        }
        _ => panic!("expected DatabaseError"),
    }

    // encryption_error
    let err = InklogError::encryption_error("invalid key length: 16");
    println!("\nInklogError::encryption_error(\"invalid key length: 16\")：");
    println!("  Display: {}", err);

    // encryption_error_with_source
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "key file missing");
    let err = InklogError::encryption_error_with_source("key load failed", io_err);
    println!("\nInklogError::encryption_error_with_source(...)：");
    println!("  Display: {}", err);
}

/// 展示 From trait 自动转换
fn show_from_conversions() {
    print_section("6. From trait 自动转换");

    // io::Error → InklogError
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err: InklogError = io_err.into();
    println!("io::Error → InklogError：");
    println!("  {}", err);
    assert!(matches!(err, InklogError::IoError(_)));

    // serde_json::Error → InklogError
    let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
    let err: InklogError = json_err.into();
    println!("\nserde_json::Error → InklogError：");
    println!("  {}", err);
    assert!(matches!(err, InklogError::SerializationError(_)));

    // toml::de::Error → InklogError (via From<toml::de::Error>)
    let toml_err = toml::from_str::<toml::Value>("invalid = = toml").unwrap_err();
    let err: InklogError = toml_err.into();
    println!("\ntoml::de::Error → InklogError：");
    println!("  {}", err);
    assert!(matches!(err, InklogError::ConfigError(_)));

    println!("\n  ✓ 标准错误类型可自动转换为 InklogError");
}

/// 展示 InklogResult 类型别名
fn show_result_type_alias() {
    print_section("7. InklogResult<T> 类型别名");

    println!("type InklogResult<T> = Result<T, InklogError>;\n");

    // 成功场景
    let ok: InklogResult<i32> = Ok(42);
    println!("Ok(42) → {:?}", ok.ok());

    // 失败场景
    let err: InklogResult<i32> = Err(InklogError::ConfigError("bad config".into()));
    println!("Err(...) → is_err = {}", err.is_err());

    // 使用 ? 运算符
    fn read_value() -> InklogResult<String> {
        let content = std::fs::read_to_string("/nonexistent/file")?;
        Ok(content)
    }
    match read_value() {
        Ok(_) => println!("  意外成功"),
        Err(e) => {
            println!("\n使用 ? 运算符传播 io::Error：");
            println!("  {}", e);
            assert!(matches!(e, InklogError::IoError(_)));
        }
    }

    println!("\n  ✓ InklogResult 与 ? 运算符兼容");
}

/// 展示实际错误处理模式
fn show_error_handling_patterns() {
    print_section("8. 实际错误处理模式");

    println!("模式 1：match 分支处理\n");

    let result: InklogResult<()> = Err(InklogError::ConfigError("invalid level".into()));
    match result {
        Ok(()) => println!("  操作成功"),
        Err(InklogError::ConfigError(msg)) => {
            println!("  配置错误（可恢复）: {}", msg);
            println!("  → 使用默认配置重试");
        }
        Err(InklogError::IoError(e)) => {
            println!("  IO 错误: {}", e);
            println!("  → 检查文件权限和磁盘空间");
        }
        Err(e) => {
            println!("  其他错误: {}", e);
            println!("  → 上报并退出");
        }
    }

    println!("\n模式 2：安全日志输出\n");
    let err = InklogError::ConfigError("Failed: postgres://user:pass@localhost/db".into());
    // 在日志中使用 safe_message 避免泄露敏感信息
    println!("  日志输出: {}", err.safe_message());
    // 在终端使用 localized_message 提供本地化提示
    println!("  用户提示: {}", err.localized_message());

    println!("\n模式 3：错误链追踪\n");
    let source = std::io::Error::new(std::io::ErrorKind::NotFound, "config.toml");
    let err = InklogError::database_error_with_source("failed to load db config", source);
    println!("  顶层错误: {}", err);
    if let InklogError::DatabaseError {
        source: Some(src), ..
    } = &err
    {
        println!("  根因: {}", src);
    }
}
