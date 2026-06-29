// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
	println!("=== inklog log crate 适配器示例 ===\n");

	show_install_and_log();
	show_level_filter_behavior();

	println!("\n✓ 所有 log 适配器示例演示完成");
}

/// 展示安装 LogLogger 并使用 log 宏输出日志
fn show_install_and_log() {
	print_separator("1. 安装 LogLogger 并使用 log 宏");

	// 创建 console 和 async channel（容量 100）
	let (console_tx, console_rx) = bounded::<Arc<LogRecord>>(100);
	let (async_tx, async_rx) = bounded::<Arc<LogRecord>>(100);

	// 创建 Metrics 指标收集器
	let metrics = Arc::new(Metrics::new());

	// 构建 LogAdapter 并安装为全局 logger
	let adapter = LogAdapter::new(console_tx, async_tx, metrics);
	let logger = LogLogger::new(adapter, LevelFilter::Info);
	let installed = logger.install().is_ok();
	if installed {
		println!("✓ LogLogger 安装成功（max_level = Info）");
	} else {
		// 全局 logger 只能安装一次；独立运行本 binary 不会触发此分支
		println!("⚠ 全局 logger 已被安装，log 宏将路由到既有 logger");
	}

	print_section("1.1 使用 log::info! / warn! / error! 宏");
	log::info!("这是一条 INFO 级别日志");
	log::warn!("这是一条 WARN 级别日志");
	log::error!("这是一条 ERROR 级别日志");
	log::debug!("这条 DEBUG 日志应被过滤（max_level = Info）");
	log::trace!("这条 TRACE 日志应被过滤（max_level = Info）");

	// 从 channel 接收日志记录并打印
	print_section("1.2 从 console channel 接收的记录");
	let console_count = drain_channel(&console_rx, "console", installed);

	print_section("1.3 从 async channel 接收的记录");
	let async_count = drain_channel(&async_rx, "async", installed);

	if installed {
		// info + warn + error = 3 条（debug/trace 被过滤）
		assert_eq!(console_count, 3, "console channel 应收到 3 条记录");
		assert_eq!(async_count, 3, "async channel 应收到 3 条记录");
		println!("\n✓ 校验通过：每个 channel 各收到 3 条记录（info/warn/error）");
	}
}

/// 展示 LevelFilter 过滤行为说明
fn show_level_filter_behavior() {
	print_separator("2. LevelFilter 过滤行为");
	println!("LogLogger::new(adapter, LevelFilter::Info) 设置 max_level = Info");
	println!("  → log::info! / warn! / error! 通过");
	println!("  → log::debug! / trace! 被过滤");
	println!();
	println!("LogAdapter::enabled() 实现：metadata.level() <= log::max_level()");
	println!("Level 枚举序：Error < Warn < Info < Debug < Trace");
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
			println!("  （{} channel 未收到记录）", name);
		} else {
			println!("  （{} channel 未收到记录，全局 logger 未安装到本适配器）", name);
		}
	} else {
		println!("  共接收 {} 条 {} 记录", count, name);
	}
	count
}
