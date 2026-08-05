// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! LogLevel 类型解析/比较/Display 示例
//!
//! 演示 `inklog::LogLevel` 枚举的核心功能：
//!
//! 1. 字符串解析（`from_str` / `FromStr` trait）
//! 2. 序比较（`Ord` / `PartialOrd`）
//! 3. `Display` trait 实现
//! 4. 辅助方法（`as_str` / `as_short_str`）
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin log_level
//! ```

use inklog::LogLevel;
use inklog_examples::common::{print_section, print_separator};

fn main() {
    print_separator("inklog LogLevel 类型示例");

    show_from_str_parsing();
    show_trait_from_str();
    show_ordering_comparison();
    show_display_trait();
    show_helper_methods();
    show_error_handling();

    println!("\n所有 LogLevel 示例展示完毕。");
}

/// 演示 `LogLevel::from_str` 静态方法（大小写不敏感）
fn show_from_str_parsing() {
    print_section("示例 1：LogLevel::from_str 静态方法");

    println!("大小写不敏感解析：");
    for input in [
        "trace", "DEBUG", "Info", "WARN", "warning", "ERROR", "fatal", "critical",
    ] {
        match LogLevel::from_str(input) {
            Some(level) => println!("  {:>10} → {:?}", input, level),
            None => println!("  {:>10} → None", input),
        }
    }

    println!("\n无效输入返回 None：");
    for input in ["", "invalid", "trace_level", "123"] {
        let result = LogLevel::from_str(input);
        println!("  {:>12} → {:?}", input, result);
    }
}

/// 演示 `FromStr` trait 实现（返回 `Result`，支持 `?` 运算符）
fn show_trait_from_str() {
    print_section("示例 2：FromStr trait（返回 Result）");

    println!("使用 .parse::<LogLevel>()：");
    let valid_inputs = [
        "trace", "debug", "info", "warn", "warning", "error", "fatal", "critical",
    ];
    for input in valid_inputs {
        let level: LogLevel = input.parse().expect("valid level should parse");
        println!("  {:>10}.parse() → Ok({:?})", input, level);
    }

    println!("\n错误处理：");
    let invalid_inputs = ["", "invalid", "trace_level"];
    for input in invalid_inputs {
        let result: Result<LogLevel, _> = input.parse();
        match &result {
            Ok(level) => println!("  {:>12}.parse() → Ok({:?})", input, level),
            Err(e) => println!("  {:>12}.parse() → Err(\"{}\")", input, e),
        }
    }
}

/// 演示 `Ord` / `PartialOrd` 序比较（用于级别过滤）
fn show_ordering_comparison() {
    print_section("示例 3：序比较（Ord / PartialOrd）");

    println!("级别从低到高（数值越大级别越高）：");
    let levels = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ];
    for level in levels {
        println!("  {:?} = {}", level, level as u8);
    }

    println!("\n比较运算符：");
    let l1 = LogLevel::Error;
    let l2 = LogLevel::Error;
    println!("  Error > Debug  → {}", LogLevel::Error > LogLevel::Debug);
    println!("  Info  <= Warn  → {}", LogLevel::Info <= LogLevel::Warn);
    println!("  Trace < Fatal  → {}", LogLevel::Trace < LogLevel::Fatal);
    println!("  Error == Error → {}", l1 == l2);

    println!("\n实际应用：级别过滤（只记录 >= WARN 的日志）");
    let min_level = LogLevel::Warn;
    let incoming = [
        LogLevel::Trace,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ];
    for level in incoming {
        let enabled = level >= min_level;
        println!(
            "  {:?} >= {:?} → {} → {}",
            level,
            min_level,
            enabled,
            if enabled { "记录" } else { "丢弃" }
        );
    }
}

/// 演示 `Display` trait 实现
fn show_display_trait() {
    print_section("示例 4：Display trait");

    println!("format!(\"{{}}\", level)：");
    for level in [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ] {
        println!("  format!(\"{{}}\", {:?}) → \"{}\"", level, level);
    }

    println!("\n与 as_str() 一致性验证：");
    for level in [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ] {
        let display = format!("{}", level);
        let as_str = level.as_str();
        let consistent = display == as_str;
        println!(
            "  {:?}: Display=\"{}\", as_str=\"{}\", 一致={}",
            level, display, as_str, consistent
        );
    }
}

/// 演示辅助方法 `as_str` 和 `as_short_str`
fn show_helper_methods() {
    print_section("示例 5：辅助方法 as_str / as_short_str");

    println!("{:<10} {:<8} {:<10}", "Variant", "as_str", "as_short_str");
    println!("{}", "-".repeat(30));
    for level in [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ] {
        println!(
            "{:<10} {:<8} {:<10}",
            format!("{:?}", level),
            level.as_str(),
            level.as_short_str()
        );
    }

    println!("\n短格式适用场景：紧凑控制台输出");
    println!("  [INF] 2026-01-01 服务启动");
    println!("  [WRN] 2026-01-01 配置项缺失，使用默认值");
    println!("  [ERR] 2026-01-01 数据库连接失败");
}

/// 演示错误处理和 `LogLevelParseError`
fn show_error_handling() {
    print_section("示例 6：错误处理（LogLevelParseError）");

    use inklog::LogLevelParseError;

    let invalid_inputs = ["", "invalid", "TRACE_LEVEL", "123"];
    for input in invalid_inputs {
        let result: Result<LogLevel, LogLevelParseError> = input.parse();
        match result {
            Ok(_) => unreachable!("invalid input should error"),
            Err(e) => {
                let msg = format!("{}", e);
                let contains_input =
                    msg.contains(input) || (input.is_empty() && msg.contains("Unknown"));
                println!("  输入: {:>12}", format!("\"{}\"", input));
                println!("    错误类型: {:?}", e);
                println!("    错误消息: {}", msg);
                println!("    包含原始输入: {}", contains_input);
            }
        }
    }
}
