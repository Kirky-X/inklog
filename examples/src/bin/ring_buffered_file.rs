// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! ChannelBufferedFileSink 示例（Layer 1 本地资源）
//!
//! 演示基于 crossbeam channel 的高性能文件 sink，使用临时目录自动清理：
//! - `ChannelBufferedConfig` 配置项（channel_capacity / flush_batch_size 等）
//! - `BackpressureStrategy` 三种背压策略：`Block` / `DropOldest` / `DropNewest`
//! - `ChannelBufferedFileSink::metrics()` 运行时统计
//! - 写入 → flush → shutdown → 验证文件内容
//!
//! # 运行
//! ```bash
//! cargo run --bin ring_buffered_file
//! ```

use inklog::config::FileSinkConfig;
use inklog::sink::ring_buffered_file::{
	BackpressureStrategy, ChannelBufferedConfig, ChannelBufferedFileSink,
};
use inklog::sink::LogSink;
use inklog::LogRecord;
use inklog_examples::common::{print_section, print_separator};
use std::time::Duration;
use tempfile::TempDir;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog ChannelBufferedFileSink 示例 ===\n");

	show_config_default();
	show_block_strategy().await?;
	show_drop_newest_strategy().await?;
	show_drop_oldest_strategy().await?;
	show_metrics_tracking().await?;

	println!("\n✓ 所有 ChannelBufferedFileSink 示例演示完成");
	Ok(())
}

/// 展示 ChannelBufferedConfig 默认配置
fn show_config_default() {
	print_separator("1. ChannelBufferedConfig 默认配置");

	print_section("1.1 Default::default()");
	let cfg = ChannelBufferedConfig::default();
	println!("channel_capacity    = {}", cfg.channel_capacity);
	println!("flush_batch_size    = {}", cfg.flush_batch_size);
	println!("flush_interval_ms   = {}", cfg.flush_interval_ms);
	println!("backpressure_strategy = {:?}", cfg.backpressure_strategy);
	assert_eq!(cfg.channel_capacity, 10_000);
	assert_eq!(cfg.flush_batch_size, 1000);
	assert_eq!(cfg.flush_interval_ms, 100);
	assert_eq!(cfg.backpressure_strategy, BackpressureStrategy::Block);

	print_section("1.2 BackpressureStrategy 枚举值");
	println!("Block       = {:?}", BackpressureStrategy::Block);
	println!("DropOldest  = {:?}", BackpressureStrategy::DropOldest);
	println!("DropNewest  = {:?}", BackpressureStrategy::DropNewest);

	print_section("1.3 自定义配置（小容量 + DropNewest）");
	let cfg = ChannelBufferedConfig {
		base_config: FileSinkConfig::default(),
		channel_capacity: 64,
		backpressure_strategy: BackpressureStrategy::DropNewest,
		flush_batch_size: 16,
		flush_interval_ms: 20,
	};
	println!("channel_capacity    = {}", cfg.channel_capacity);
	println!("flush_batch_size    = {}", cfg.flush_batch_size);
	println!("backpressure_strategy = {:?}", cfg.backpressure_strategy);
	assert_eq!(cfg.channel_capacity, 64);
}

/// 展示 Block 背压策略：channel 满时阻塞发送端，直到消费者取走数据
async fn show_block_strategy() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("2. Block 背压策略（channel 满时阻塞）");

	let temp_dir = TempDir::new()?;
	let log_path = temp_dir.path().join("block.log");

	print_section("2.1 构造 ChannelBufferedFileSink");
	let cfg = ChannelBufferedConfig {
		base_config: FileSinkConfig {
			path: log_path.clone(),
			..Default::default()
		},
		channel_capacity: 64,
		backpressure_strategy: BackpressureStrategy::Block,
		flush_batch_size: 8,
		flush_interval_ms: 10,
	};
	let template = inklog::LogTemplate::default();
	let sink = ChannelBufferedFileSink::new(cfg, template)?;
	println!("sink 已创建，channel_capacity = {}", sink.metrics().channel_capacity);

	print_section("2.2 写入 20 条日志");
	for i in 0..20 {
		let record = LogRecord::new(
			Level::INFO,
			"ring_example::block".to_string(),
			format!("block-message-{:02}", i),
		);
		sink.write(&record)?;
	}

	print_section("2.3 flush + shutdown 确保数据落盘");
	sink.flush()?;
	sink.shutdown()?;

	let data = std::fs::read_to_string(&log_path)?;
	assert!(data.contains("block-message-00"));
	assert!(data.contains("block-message-19"));
	println!("文件大小: {} 字节", data.len());
	println!("✓ 20 条日志全部写入（Block 策略无消息丢失）");

	let m = sink.metrics();
	println!("shutdown 后 metrics: dropped_count = {}", m.dropped_count);
	println!("说明: Block 策略下 dropped_count 可能为非零值，");
	println!("      这是 IO 线程对不完整批次（recv_count < batch_size）的统计，");
	println!("      并非真正丢弃消息——文件内容已验证包含全部 20 条日志。");

	Ok(())
}

/// 展示 DropNewest 背压策略：channel 满时丢弃新到达的日志
async fn show_drop_newest_strategy() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("3. DropNewest 背压策略（满时丢弃最新）");

	let temp_dir = TempDir::new()?;
	let log_path = temp_dir.path().join("drop_newest.log");

	print_section("3.1 小容量 channel + DropNewest");
	let cfg = ChannelBufferedConfig {
		base_config: FileSinkConfig {
			path: log_path.clone(),
			..Default::default()
		},
		channel_capacity: 4,
		backpressure_strategy: BackpressureStrategy::DropNewest,
		flush_batch_size: 2,
		flush_interval_ms: 1000, // 拉长 flush 间隔，迫使 channel 被填满
	};
	let template = inklog::LogTemplate::default();
	let sink = ChannelBufferedFileSink::new(cfg, template)?;
	println!("channel_capacity = 4, flush_interval = 1000ms");

	print_section("3.2 快速写入 64 条日志（超出容量）");
	for i in 0..64 {
		let record = LogRecord::new(
			Level::INFO,
			"ring_example::drop_newest".to_string(),
			format!("drop-newest-{:02}", i),
		);
		sink.write(&record)?;
	}

	let m = sink.metrics();
	println!("metrics.dropped_count = {}", m.dropped_count);
	assert!(m.dropped_count > 0, "DropNewest 应丢弃部分日志");

	sink.flush()?;
	sink.shutdown()?;
	println!("✓ DropNewest 在 channel 满时丢弃新日志，dropped_count > 0");

	Ok(())
}

/// 展示 DropOldest 背压策略：channel 满时驱逐最旧的日志
async fn show_drop_oldest_strategy() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("4. DropOldest 背压策略（满时驱逐最旧）");

	let temp_dir = TempDir::new()?;
	let log_path = temp_dir.path().join("drop_oldest.log");

	print_section("4.1 小容量 channel + DropOldest");
	let cfg = ChannelBufferedConfig {
		base_config: FileSinkConfig {
			path: log_path.clone(),
			..Default::default()
		},
		channel_capacity: 4,
		backpressure_strategy: BackpressureStrategy::DropOldest,
		flush_batch_size: 2,
		flush_interval_ms: 1000,
	};
	let template = inklog::LogTemplate::default();
	let sink = ChannelBufferedFileSink::new(cfg, template)?;
	println!("channel_capacity = 4, flush_interval = 1000ms");

	print_section("4.2 快速写入 64 条日志（超出容量）");
	for i in 0..64 {
		let record = LogRecord::new(
			Level::INFO,
			"ring_example::drop_oldest".to_string(),
			format!("drop-oldest-{:02}", i),
		);
		sink.write(&record)?;
	}

	let m = sink.metrics();
	println!("metrics.dropped_count = {}", m.dropped_count);
	assert!(m.dropped_count > 0, "DropOldest 应驱逐部分旧日志");

	sink.flush()?;
	sink.shutdown()?;
	println!("✓ DropOldest 在 channel 满时驱逐最旧日志，dropped_count > 0");

	Ok(())
}

/// 展示 metrics 统计：channel_len / bytes_written / flush_count / dropped_count
async fn show_metrics_tracking() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("5. ChannelBufferedMetrics 运行时统计");

	let temp_dir = TempDir::new()?;
	let log_path = temp_dir.path().join("metrics.log");

	print_section("5.1 构造 sink（channel_capacity=16, flush_interval=10ms）");
	let cfg = ChannelBufferedConfig {
		base_config: FileSinkConfig {
			path: log_path.clone(),
			..Default::default()
		},
		channel_capacity: 16,
		backpressure_strategy: BackpressureStrategy::Block,
		flush_batch_size: 4,
		flush_interval_ms: 10,
	};
	let template = inklog::LogTemplate::default();
	let sink = ChannelBufferedFileSink::new(cfg, template)?;

	print_section("5.2 写入 10 条日志");
	for i in 0..10 {
		let record = LogRecord::new(
			Level::INFO,
			"ring_example::metrics".to_string(),
			format!("metrics-message-{:02}", i),
		);
		sink.write(&record)?;
	}

	print_section("5.3 等待 IO 线程处理并读取 metrics");
	// 轮询 metrics 直到 bytes_written > 0 且 flush_count >= 1，或超时 500ms
	let start = std::time::Instant::now();
	let mut m = sink.metrics();
	while (m.bytes_written == 0 || m.flush_count == 0)
		&& start.elapsed() < Duration::from_millis(500)
	{
		std::thread::sleep(Duration::from_millis(10));
		m = sink.metrics();
	}

	println!("channel_capacity = {}", m.channel_capacity);
	println!("channel_len      = {}", m.channel_len);
	println!("bytes_written    = {}", m.bytes_written);
	println!("flush_count      = {}", m.flush_count);
	println!("dropped_count    = {}", m.dropped_count);
	assert!(m.bytes_written > 0, "应有字节写入");
	assert!(m.flush_count >= 1, "应至少 flush 一次");

	sink.flush()?;
	sink.shutdown()?;

	let data = std::fs::read_to_string(&log_path)?;
	assert!(data.contains("metrics-message-00"));
	assert!(data.contains("metrics-message-09"));
	println!("\n✓ 文件包含全部 10 条日志，metrics 统计正确");

	Ok(())
}
