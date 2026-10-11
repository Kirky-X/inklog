// Copyright (c) 2026 Kirky.X🌠
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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

fn main() {
    print_separator(&tr("example-log-level-inklog-loglevel-type-example"));

    show_from_str_parsing();
    show_trait_from_str();
    show_ordering_comparison();
    show_display_trait();
    show_helper_methods();
    show_error_handling();

    println!(
        "\n{}",
        tr("example-log-level-all-loglevel-examples-completed")
    );
}

/// 演示 `LogLevel::from_str` 静态方法（大小写不敏感）
fn show_from_str_parsing() {
    print_section(&tr("example-log-level-example-1-the-loglevel-from-str"));

    println!("{}", tr("example-log-level-case-insensitive-parsing"));
    for input in [
        "trace", "DEBUG", "Info", "WARN", "warning", "ERROR", "fatal", "critical",
    ] {
        match LogLevel::from_str(input) {
            Some(level) => println!("  {:>10} → {:?}", input, level),
            None => println!(
                "  {}",
                tr_args(
                    "example-log-level-none",
                    &[("input", &format!("{:>10}", input))]
                )
            ),
        }
    }

    println!("\n{}", tr("example-log-level-invalid-input-returns-none"));
    for input in ["", "invalid", "trace_level", "123"] {
        let result = LogLevel::from_str(input);
        println!("  {:>12} → {:?}", input, result);
    }
}

/// 演示 `FromStr` trait 实现（返回 `Result`，支持 `?` 运算符）
fn show_trait_from_str() {
    print_section(&tr("example-log-level-example-2-the-fromstr-trait"));

    println!("{}", tr("example-log-level-using-parse-loglevel"));
    let valid_inputs = [
        "trace", "debug", "info", "warn", "warning", "error", "fatal", "critical",
    ];
    for input in valid_inputs {
        let level: LogLevel = input.parse().expect("valid level should parse");
        println!(
            "  {}",
            tr_args(
                "example-log-level-parse-ok",
                &[
                    ("input", &format!("{:>10}", input)),
                    ("level", &format!("{:?}", level))
                ]
            )
        );
    }

    println!("\n{}", tr("example-log-level-error-handling"));
    let invalid_inputs = ["", "invalid", "trace_level"];
    for input in invalid_inputs {
        let result: Result<LogLevel, _> = input.parse();
        match &result {
            Ok(level) => println!(
                "  {}",
                tr_args(
                    "example-log-level-parse-ok",
                    &[
                        ("input", &format!("{:>12}", input)),
                        ("level", &format!("{:?}", level))
                    ]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-log-level-parse-err",
                    &[("input", &format!("{:>12}", input)), ("e", &e)]
                )
            ),
        }
    }
}

/// 演示 `Ord` / `PartialOrd` 序比较（用于级别过滤）
fn show_ordering_comparison() {
    print_section(&tr("example-log-level-example-3-ordering-ord"));

    println!(
        "{}",
        tr("example-log-level-levels-low-to-high-higher-value")
    );
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

    println!("\n{}", tr("example-log-level-comparison-operators"));
    let l1 = LogLevel::Error;
    let l2 = LogLevel::Error;
    println!(
        "  {}",
        tr_args(
            "example-log-level-error-debug",
            &[("value", &(LogLevel::Error > LogLevel::Debug))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-log-level-info-warn",
            &[("value", &(LogLevel::Info <= LogLevel::Warn))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-log-level-trace-fatal",
            &[("value", &(LogLevel::Trace < LogLevel::Fatal))]
        )
    );
    println!(
        "  {}",
        tr_args("example-log-level-error-error", &[("value", &(l1 == l2))])
    );

    println!(
        "\n{}",
        tr("example-log-level-practical-use-level-filtering")
    );
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
            if enabled { "recorded" } else { "dropped" }
        );
    }
}

/// 演示 `Display` trait 实现
fn show_display_trait() {
    print_section(&tr("example-log-level-example-4-the-display-trait"));

    println!("format!(\"{{}}\", level):");
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

    println!(
        "\n{}",
        tr("example-log-level-consistency-check-against-as-str")
    );
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
            "  {}",
            tr_args(
                "example-log-level-display-as-str-match",
                &[
                    ("level", &format!("{:?}", level)),
                    ("display", &display),
                    ("as_str", &as_str),
                    ("consistent", &consistent)
                ]
            )
        );
    }
}

/// 演示辅助方法 `as_str` 和 `as_short_str`
fn show_helper_methods() {
    print_section(&tr("example-log-level-example-5-helper-methods-as-str"));

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

    println!(
        "\n{}",
        tr("example-log-level-short-format-use-case-compact")
    );
    println!(
        "  {}",
        tr("example-log-level-inf-2026-01-01-service-started")
    );
    println!(
        "  {}",
        tr("example-log-level-wrn-2026-01-01-missing-config")
    );
    println!("  {}", tr("example-log-level-err-2026-01-01-database"));
}

/// 演示错误处理和 `LogLevelParseError`
fn show_error_handling() {
    print_section(&tr("example-log-level-example-6-error-handling"));

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
                println!(
                    "  {}",
                    tr_args(
                        "example-log-level-input",
                        &[("value", &format!("{:>12}", format!("\"{}\"", input)))]
                    )
                );
                println!(
                    "    {}",
                    tr_args(
                        "example-log-level-error-type",
                        &[("e", &format!("{:?}", e))]
                    )
                );
                println!(
                    "    {}",
                    tr_args("example-log-level-error-message", &[("msg", &msg)])
                );
                println!(
                    "    {}",
                    tr_args(
                        "example-log-level-input-included",
                        &[("contains_input", &contains_input)]
                    )
                );
            }
        }
    }
}
