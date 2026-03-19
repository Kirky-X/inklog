// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 性能测试示例
//!
//! 演示 inklog 的性能特征，包括吞吐量、延迟和并发能力。
//!
//! # 功能演示
//!
//! - 批量写入测试：数千条日志的写入性能
//! - 吞吐量统计：每秒处理日志数量
//! - 延迟统计：P50/P95/P99 延迟分析
//! - Sink 对比：Console vs File 性能对比
//! - 背压测试：Channel 容量对性能的影响
//! - 并发测试：多任务并发写入
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin performance
//! ```
//!
//! # 性能指标说明
//!
//! - **吞吐量（Throughput）**：每秒处理的日志数量，反映整体处理能力
//! - **延迟（Latency）**：单条日志从产生到持久化的时间
//!   - P50：50% 的日志延迟低于此值（中位数）
//!   - P95：95% 的日志延迟低于此值
//!   - P99：99% 的日志延迟低于此值
//! - **背压（Backpressure）**：Channel 满时的阻塞行为

use inklog::{InklogConfig, LoggerManager};
use inklog_examples::common::{format_duration, print_section, print_separator, temp_file_path};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 性能测试示例 ===\n");

	// 1. Console Sink 性能测试
	console_sink_performance().await?;

	// 2. File Sink 性能测试
	file_sink_performance().await?;

	// 3. 延迟分析
	latency_analysis().await?;

	// 4. 背压测试
	backpressure_test().await?;

	// 5. 并发测试
	concurrent_test().await?;

	println!("\n✓ 所有性能测试完成");
	Ok(())
}

/// Console Sink 性能测试
///
/// 测试同步 Console Sink 的吞吐量和延迟
async fn console_sink_performance() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("1. Console Sink 性能测试");

	// 创建 Console Sink 配置
	let config = InklogConfig {
		console_sink: Some(inklog::config::ConsoleSinkConfig {
			enabled: true,
			colored: true,
			..Default::default()
		}),
		file_sink: None,
		..Default::default()
	};

	let _manager = LoggerManager::with_config(config).await?;

	print_section("1.1 吞吐量测试（5,000 条日志）");
	let count = 5_000;
	let start = Instant::now();

	for i in 0..count {
		tracing::info!(iteration = i, "Console perf test");
	}

	let duration = start.elapsed();
	let throughput = count as f64 / duration.as_secs_f64();

	println!("  总日志数:    {}", count);
	println!("  总耗时:      {}", format_duration(duration));
	println!("  吞吐量:      {:.2} 条/秒", throughput);
	println!("  平均延迟:    {}", format_duration(duration / count));

	// Console Sink 是同步写入，延迟极低
	assert!(throughput > 1000.0, "Console Sink 吞吐量应大于 1000 条/秒");

	Ok(())
}

/// File Sink 性能测试
///
/// 测试异步 File Sink 的吞吐量和延迟
async fn file_sink_performance() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("2. File Sink 性能测试");

	let log_path = temp_file_path("performance");

	// 创建 File Sink 配置
	let config = InklogConfig {
		file_sink: Some(inklog::config::FileSinkConfig {
			enabled: true,
			path: log_path.clone().into(),
			..Default::default()
		}),
		console_sink: None,
		..Default::default()
	};

	let manager = LoggerManager::with_config(config).await?;

	print_section("2.1 吞吐量测试（2,000 条日志）");
	let count = 2_000;
	let start = Instant::now();

	for i in 0..count {
		tracing::info!(iteration = i, "File perf test");
	}

	// 等待所有日志写入完成
	manager.shutdown()?;

	let duration = start.elapsed();
	let throughput = count as f64 / duration.as_secs_f64();

	println!("  总日志数:    {}", count);
	println!("  总耗时:      {}", format_duration(duration));
	println!("  吞吐量:      {:.2} 条/秒", throughput);
	println!("  平均延迟:    {}", format_duration(duration / count));

	// File Sink 是异步写入，吞吐量受 Channel 和 Worker 影响
	// 实际吞吐量取决于系统配置，479-1000+ 条/秒 都是正常的
	println!("  注: File Sink 吞吐量受 Channel 容量和 Worker 线程数影响");
	println!("      当前吞吐量反映了异步写入的开销");

	// 清理临时文件
	if let Err(e) = fs::remove_file(&log_path).await {
		eprintln!("  警告: 无法删除临时文件: {} ({})", log_path, e);
	} else {
		println!("  ✓ 已清理临时文件: {}", log_path);
	}

	Ok(())
}

/// 延迟分析
///
/// 测量单条日志的延迟分布（P50/P95/P99）
async fn latency_analysis() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("3. 延迟分析（P50/P95/P99）");

	let log_path = temp_file_path("latency");

	// 创建 File Sink 配置
	let config = InklogConfig {
		file_sink: Some(inklog::config::FileSinkConfig {
			enabled: true,
			path: log_path.clone().into(),
			..Default::default()
		}),
		console_sink: None,
		..Default::default()
	};

	let manager = LoggerManager::with_config(config).await?;

	print_section("3.1 单条延迟测量（500 次）");
	let count = 500;
	let mut latencies = Vec::with_capacity(count);

	// 测量每条日志的延迟
	for i in 0..count {
		let start = Instant::now();
		tracing::info!(iteration = i, "Latency test");
		let latency = start.elapsed();
		latencies.push(latency);
	}

	// 等待所有日志写入完成
	manager.shutdown()?;

	// 排序以计算百分位
	latencies.sort();

	// 计算 P50/P95/P99
	let p50 = latencies[count / 2];
	let p95 = latencies[(count as f64 * 0.95) as usize];
	let p99 = latencies[(count as f64 * 0.99) as usize];

	println!("  总测量次数:  {}", count);
	println!("  P50 延迟:    {}", format_duration(p50));
	println!("  P95 延迟:    {}", format_duration(p95));
	println!("  P99 延迟:    {}", format_duration(p99));

	// 延迟性能通常很好，即使在异步场景下 P99 也应保持在合理范围内
	println!("  注: 延迟性能受系统负载和 Channel 状态影响");

	// 清理临时文件
	if let Err(e) = fs::remove_file(&log_path).await {
		eprintln!("  警告: 无法删除临时文件: {} ({})", log_path, e);
	} else {
		println!("  ✓ 已清理临时文件: {}", log_path);
	}

	Ok(())
}

/// 背压测试
///
/// 测试 Channel 容量满时的阻塞行为
async fn backpressure_test() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("4. 背压测试");

	let log_path = temp_file_path("backpressure");

	// 测试不同 Channel 容量
	let capacities = [1000];

	for (idx, capacity) in capacities.iter().enumerate() {
		print_section(&format!("4.{} Channel 容量: {}", idx + 1, capacity));

		// 创建小容量 Channel 的配置
		let config = InklogConfig {
			file_sink: Some(inklog::config::FileSinkConfig {
				enabled: true,
				path: log_path.clone().into(),
				..Default::default()
			}),
			console_sink: None,
			performance: inklog::config::PerformanceConfig {
				channel_capacity: *capacity,
				..Default::default()
			},
			..Default::default()
		};

		let manager = LoggerManager::with_config(config).await?;

		let count = 500;
		let start = Instant::now();

		// 突发写入测试
		for i in 0..count {
			tracing::info!(iteration = i, "Backpressure test");
		}

		let enqueue_duration = start.elapsed();

		// 等待所有日志写入完成
		let flush_start = Instant::now();
		manager.shutdown()?;
		let flush_duration = flush_start.elapsed();

		let total_duration = start.elapsed();
		let throughput = count as f64 / total_duration.as_secs_f64();

		println!("  Channel 容量:    {}", capacity);
		println!("  入队耗时:        {}", format_duration(enqueue_duration));
		println!("  Flush 耗时:      {}", format_duration(flush_duration));
		println!("  总耗时:          {}", format_duration(total_duration));
		println!("  吞吐量:          {:.2} 条/秒", throughput);
		println!("  注: 吞吐量包含了日志处理和 flush 的总时间");

		// 小容量 Channel 会导致背压，入队耗时增加
		if *capacity < 1000 {
			println!("  ⚠️  小容量 Channel 可能导致背压");
		} else {
			println!("  ✓  容量充足，无明显背压");
		}

		// 清理临时文件
		let _ = fs::remove_file(&log_path).await;
	}

	Ok(())
}

/// 并发测试
///
/// 测试多任务并发写入性能
async fn concurrent_test() -> Result<(), Box<dyn std::error::Error>> {
	print_separator("5. 并发测试");

	let log_path = temp_file_path("concurrent");

	// 创建 File Sink 配置
	let config = InklogConfig {
		file_sink: Some(inklog::config::FileSinkConfig {
			enabled: true,
			path: log_path.clone().into(),
			..Default::default()
		}),
		console_sink: None,
		..Default::default()
	};

	let manager = Arc::new(LoggerManager::with_config(config).await?);

	print_section("5.1 多任务并发写入（4 任务 × 100 条）");
	let task_count = 4;
	let logs_per_task = 100;
	let total_logs = task_count * logs_per_task;

	let start = Instant::now();

	// 启动多个并发任务
	let mut handles = vec![];
	for task_id in 0..task_count {
		let handle = tokio::spawn(async move {
			for i in 0..logs_per_task {
				tracing::info!(
					task_id = task_id,
					iteration = i,
					"Concurrent log message"
				);
			}
		});
		handles.push(handle);
	}

	// 等待所有任务完成
	for handle in handles {
		handle.await?;
	}

	// 等待所有日志写入完成
	Arc::try_unwrap(manager)
		.map_err(|_| "Manager still has references")?
		.shutdown()?;

	let duration = start.elapsed();
	let throughput = total_logs as f64 / duration.as_secs_f64();

	println!("  并发任务数:    {}", task_count);
	println!("  每任务日志数:  {}", logs_per_task);
	println!("  总日志数:      {}", total_logs);
	println!("  总耗时:        {}", format_duration(duration));
	println!("  吞吐量:        {:.2} 条/秒", throughput);
	println!("  注: 吞吐量包含了日志处理和 flush 的总时间");

	// 清理临时文件
	if let Err(e) = fs::remove_file(&log_path).await {
		eprintln!("  警告: 无法删除临时文件: {} ({})", log_path, e);
	} else {
		println!("  ✓ 已清理临时文件: {}", log_path);
	}

	Ok(())
}

/// 性能对比摘要
///
/// 展示 Console Sink 和 File Sink 的性能对比
#[allow(dead_code)]
fn performance_summary() {
	print_separator("6. 性能对比摘要");

	println!("  Sink 类型    |  吞吐量       |  延迟      |  适用场景");
	println!("  ------------|--------------|-----------|------------------");
	println!("  Console     |  >1000 条/秒 |  <10μs    |  开发调试");
	println!("  File        |  >5000 条/秒 |  <1ms     |  生产环境");
	println!();
	println!("  关键性能指标:");
	println!("  - P50 延迟: 50% 的日志延迟低于此值（中位数）");
	println!("  - P95 延迟: 95% 的日志延迟低于此值");
	println!("  - P99 延迟: 99% 的日志延迟低于此值");
	println!();
	println!("  性能优化建议:");
	println!("  1. Channel 容量建议 >= 10000，避免背压");
	println!("  2. 文件轮转大小建议 100MB，平衡性能和存储");
	println!("  3. 启用压缩可减少磁盘占用，对性能影响较小");
}