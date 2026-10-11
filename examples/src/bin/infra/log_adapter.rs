// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `log` crate 适配器示例（Layer 0 零依赖）
//!
//! 演示 LogAdapter、LogLogger 的使用，展示 inklog 兼容标准 `log` crate。
//! LogAdapter 实现 `log::Log` trait，将 `log` 宏产生的日志转换为 inklog 的
//! LogRecord 并分发到 console / async 两个 channel。
//!
//! # 运行
//! ```bash
//! cargo run --bin log_adapter
//! ```

use crossbeam_channel::bounded;
use inklog::{LogAdapter, LogLogger, LogRecord, Metrics};
use inklog_examples::common::{print_section, print_separator};
use log::LevelFilter;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    println!("=== inklog log crate adapter example ===\n");

    show_install_and_log();
    show_level_filter_behavior();

    println!("\n✓ All log adapter examples completed");
}

/// 展示安装 LogLogger 并使用 log 宏输出日志
fn show_install_and_log() {
    print_separator("1. Install LogLogger and use log macros");

    // 创建 console 和 async channel（容量 100）
    let (console_tx, console_rx) = bounded::<Arc<LogRecord>>(100);
    let (async_tx, async_rx) = bounded::<Arc<LogRecord>>(100);

    let metrics = Arc::new(Metrics::new());

    // 构建 LogAdapter 并安装为全局 logger
    let adapter = LogAdapter::new(console_tx, async_tx, metrics);
    let logger = LogLogger::new(adapter, LevelFilter::Info);
    let installed = logger.install().is_ok();
    if installed {
        println!("✓ LogLogger installed (max_level = Info)");
    } else {
        // 全局 logger 只能安装一次；独立运行本 binary 不会触发此分支
        println!(
            "⚠ A global logger is already installed; log macros will route to the existing logger"
        );
    }

    print_section("1.1 Using log::info! / warn! / error! macros");
    log::info!("this is an INFO level log");
    log::warn!("this is a WARN level log");
    log::error!("this is an ERROR level log");
    log::debug!("this DEBUG log should be filtered (max_level = Info)");
    log::trace!("this TRACE log should be filtered (max_level = Info)");

    // 从 channel 接收日志记录并打印
    print_section("1.2 Records received from the console channel");
    let console_count = drain_channel(&console_rx, "console", installed);

    print_section("1.3 Records received from the async channel");
    let async_count = drain_channel(&async_rx, "async", installed);

    if installed {
        // info + warn + error = 3 条（debug/trace 被过滤）
        assert_eq!(console_count, 3, "console channel should receive 3 records");
        assert_eq!(async_count, 3, "async channel should receive 3 records");
        println!("\n✓ Verified: each channel received 3 records (info/warn/error)");
    }
}

/// 展示 LevelFilter 过滤行为说明
fn show_level_filter_behavior() {
    print_separator("2. LevelFilter filtering behavior");
    println!("LogLogger::new(adapter, LevelFilter::Info) sets max_level = Info");
    println!("  → log::info! / warn! / error! pass through");
    println!("  → log::debug! / trace! filtered out");
    println!();
    println!("LogAdapter::enabled() impl: metadata.level() <= log::max_level()");
    println!("Level enum order: Error < Warn < Info < Debug < Trace");
}

/// 排空 channel 并打印接收到的 LogRecord，返回接收到的记录数
fn drain_channel(
    rx: &crossbeam_channel::Receiver<Arc<LogRecord>>,
    name: &str,
    installed: bool,
) -> usize {
    let mut count = 0usize;
    while let Ok(record) = rx.recv_timeout(Duration::from_millis(100)) {
        count += 1;
        println!(
            "  [{}] level={}, target={}, message={}",
            name, record.level, record.target, record.message
        );
    }
    if count == 0 {
        if installed {
            println!("  ({} channel received no records)", name);
        } else {
            println!(
                "  ({} channel received no records; the global logger is not installed on this adapter)",
                name
            );
        }
    } else {
        println!("  Received {} {} records in total", count, name);
    }
    count
}
