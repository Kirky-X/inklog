// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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

use inklog::tokio::fs;
use inklog::{InklogConfig, LoggerManager};
use inklog_examples::common::{format_duration, print_section, print_separator, temp_file_path};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog performance test example ===\n");

    console_sink_performance().await?;

    file_sink_performance().await?;

    latency_analysis().await?;

    backpressure_test().await?;

    concurrent_test().await?;

    println!("\n✓ All performance tests completed");
    Ok(())
}

/// Console Sink 性能测试
///
/// 测试同步 Console Sink 的吞吐量和延迟
async fn console_sink_performance() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("1. Console sink throughput test");

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

    print_section("1.1 Throughput test (5,000 logs)");
    let count = 5_000;
    let start = Instant::now();

    for i in 0..count {
        tracing::info!(iteration = i, "Console perf test");
    }

    let duration = start.elapsed();
    let throughput = count as f64 / duration.as_secs_f64();

    println!("  Total logs:   {}", count);
    println!("  Total time:   {}", format_duration(duration));
    println!("  Throughput:   {:.2} logs/s", throughput);
    println!("  Avg latency:  {}", format_duration(duration / count));

    // Console Sink 是同步写入，延迟极低
    assert!(
        throughput > 1000.0,
        "console sink throughput should exceed 1000 logs/s"
    );

    Ok(())
}

/// File Sink 性能测试
///
/// 测试异步 File Sink 的吞吐量和延迟
async fn file_sink_performance() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("2. File sink throughput test");

    let log_path = temp_file_path("performance");

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

    print_section("2.1 Throughput test (2,000 logs)");
    let count = 2_000;
    let start = Instant::now();

    for i in 0..count {
        tracing::info!(iteration = i, "File perf test");
    }

    manager.shutdown()?;

    let duration = start.elapsed();
    let throughput = count as f64 / duration.as_secs_f64();

    println!("  Total logs:   {}", count);
    println!("  Total time:   {}", format_duration(duration));
    println!("  Throughput:   {:.2} logs/s", throughput);
    println!("  Avg latency:  {}", format_duration(duration / count));

    // File Sink 是异步写入，吞吐量受 Channel 和 Worker 影响
    // 实际吞吐量取决于系统配置，479-1000+ 条/秒 都是正常的
    println!("  Note: file sink throughput depends on channel capacity and worker thread count");
    println!("      Current throughput reflects async write overhead");

    if let Err(e) = fs::remove_file(&log_path).await {
        eprintln!(
            "  Warning: failed to delete temp file: {} ({})",
            log_path, e
        );
    } else {
        println!("  ✓ Cleaned up temp file: {}", log_path);
    }

    Ok(())
}

/// 延迟分析
///
/// 测量单条日志的延迟分布（P50/P95/P99）
async fn latency_analysis() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("3. Latency analysis (P50/P95/P99)");

    let log_path = temp_file_path("latency");

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

    print_section("3.1 Single-log latency measurement (500 samples)");
    let count = 500;
    let mut produce_instants = Vec::with_capacity(count);

    // 记录每条日志的产生时刻
    for i in 0..count {
        let produce_at = Instant::now();
        tracing::info!(iteration = i, "Latency test");
        produce_instants.push(produce_at);
    }

    // 以 shutdown()/flush 全部完成作为持久化终点：
    // 端到端延迟 = 持久化完成时刻 - 该条日志产生时刻。
    // 异步 Sink 中单条日志的落盘时刻不可观测，
    // 用全量 flush 的终点作为每条日志"从产生到持久化"的度量（上界），
    // 与文件头声明的延迟定义保持一致
    manager.shutdown()?;
    let persist_done = Instant::now();

    // 计算每条日志的产生 → 持久化延迟
    let mut latencies: Vec<_> = produce_instants
        .iter()
        .map(|produce_at| persist_done.duration_since(*produce_at))
        .collect();

    latencies.sort();

    let p50 = latencies[count / 2];
    let p95 = latencies[(count as f64 * 0.95) as usize];
    let p99 = latencies[(count as f64 * 0.99) as usize];

    println!("  Total samples:  {}", count);
    println!("  P50 latency:  {}", format_duration(p50));
    println!("  P95 latency:  {}", format_duration(p95));
    println!("  P99 latency:  {}", format_duration(p99));

    // 延迟性能通常很好，即使在异步场景下 P99 也应保持在合理范围内
    println!(
        "  Note: latency is measured \"from creation to persistence\" (ends when shutdown/flush completes, including queue wait)"
    );
    println!("      Actual numbers depend on system load and channel state");

    if let Err(e) = fs::remove_file(&log_path).await {
        eprintln!(
            "  Warning: failed to delete temp file: {} ({})",
            log_path, e
        );
    } else {
        println!("  ✓ Cleaned up temp file: {}", log_path);
    }

    Ok(())
}

/// 背压测试
///
/// 测试 Channel 容量满时的阻塞行为
async fn backpressure_test() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("4. Backpressure test");

    let log_path = temp_file_path("backpressure");

    // 测试不同 Channel 容量（含不足/临界/充足三档，
    // 使下方的容量检查逻辑真实生效）
    let capacities = [100, 1000, 10_000];

    for (idx, capacity) in capacities.iter().enumerate() {
        print_section(&format!("4.{} Channel capacity: {}", idx + 1, capacity));

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

        let flush_start = Instant::now();
        manager.shutdown()?;
        let flush_duration = flush_start.elapsed();

        let total_duration = start.elapsed();
        let throughput = count as f64 / total_duration.as_secs_f64();

        println!("  Channel capacity: {}", capacity);
        println!("  Enqueue time:     {}", format_duration(enqueue_duration));
        println!("  Flush time:       {}", format_duration(flush_duration));
        println!("  Total time:       {}", format_duration(total_duration));
        println!("  Throughput:       {:.2} logs/s", throughput);
        println!("  Note: throughput includes total log processing and flush time");

        // 小容量 Channel 会导致背压，入队耗时增加
        if *capacity < 1000 {
            println!("  ⚠️  A small channel may cause backpressure");
        } else {
            println!("  ✓  Capacity is sufficient; no obvious backpressure");
        }

        let _ = fs::remove_file(&log_path).await;
    }

    Ok(())
}

/// 并发测试
///
/// 测试多任务并发写入性能
async fn concurrent_test() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("5. Concurrency test");

    let log_path = temp_file_path("concurrent");

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

    print_section("5.1 Multi-task concurrent writes (4 tasks × 100 logs)");
    let task_count = 4;
    let logs_per_task = 100;
    let total_logs = task_count * logs_per_task;

    let start = Instant::now();

    let mut handles = vec![];
    for task_id in 0..task_count {
        let handle = tokio::spawn(async move {
            for i in 0..logs_per_task {
                tracing::info!(task_id = task_id, iteration = i, "Concurrent log message");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Arc::try_unwrap(manager)
        .map_err(|_| "Manager still has references")?
        .shutdown()?;

    let duration = start.elapsed();
    let throughput = total_logs as f64 / duration.as_secs_f64();

    println!("  Concurrent tasks: {}", task_count);
    println!("  Logs per task:  {}", logs_per_task);
    println!("  Total logs:     {}", total_logs);
    println!("  Total time:     {}", format_duration(duration));
    println!("  Throughput:     {:.2} logs/s", throughput);
    println!("  Note: throughput includes total log processing and flush time");

    if let Err(e) = fs::remove_file(&log_path).await {
        eprintln!(
            "  Warning: failed to delete temp file: {} ({})",
            log_path, e
        );
    } else {
        println!("  ✓ Cleaned up temp file: {}", log_path);
    }

    Ok(())
}
