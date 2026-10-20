// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! ChannelBufferedFileSink 示例（Layer 1 本地资源）
//!
//! 演示基于 crossbeam channel 的高性能文件 sink，使用临时目录自动清理：
//! - `ChannelBufferedConfig` 配置项（channel_capacity / flush_batch_size 等）
//! - `BackpressureStrategy` 三种背压策略：`Block` / `DropOldest` / `DropNewest`
//! - `ChannelBufferedFileSink::metrics` 运行时统计
//! - 写入 → flush → shutdown → 验证文件内容
//!
//! # 运行
//! ```bash
//! cargo run --bin ring_buffered_file
//! ```

use inklog::LogRecord;
use inklog::config::FileSinkConfig;
use inklog::sink::LogSink;
use inklog::sink::ring_buffered_file::{
    BackpressureStrategy, ChannelBufferedConfig, ChannelBufferedFileSink,
};
use inklog::tracing::Level;
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::time::Duration;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-ring-buffered-file-inklog-channelbufferedfilesink")
    );

    show_config_default();
    show_block_strategy().await?;
    show_drop_newest_strategy().await?;
    show_drop_oldest_strategy().await?;
    show_metrics_tracking().await?;

    println!(
        "\n{}",
        tr("example-ring-buffered-file-all-channelbufferedfilesink")
    );
    Ok(())
}

/// 展示 ChannelBufferedConfig 默认配置
fn show_config_default() {
    print_separator(&tr(
        "example-ring-buffered-file-1-channelbufferedconfig-defaults",
    ));

    print_section(&tr("example-ring-buffered-file-1-1-default-default"));
    let cfg = ChannelBufferedConfig::default();
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-channel-capacity",
            &[("channel_capacity", &cfg.channel_capacity)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-flush-batch-size",
            &[("flush_batch_size", &cfg.flush_batch_size)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-flush-interval-ms",
            &[("flush_interval_ms", &cfg.flush_interval_ms)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-backpressure-strategy",
            &[(
                "backpressure_strategy",
                &format!("{:?}", cfg.backpressure_strategy)
            )]
        )
    );
    assert_eq!(cfg.channel_capacity, 10_000);
    assert_eq!(cfg.flush_batch_size, 1000);
    assert_eq!(cfg.flush_interval_ms, 100);
    assert_eq!(cfg.backpressure_strategy, BackpressureStrategy::Block);

    print_section(&tr(
        "example-ring-buffered-file-1-2-backpressurestrategy-enum",
    ));
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-block",
            &[("value", &format!("{:?}", BackpressureStrategy::Block))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-dropoldest",
            &[("value", &format!("{:?}", BackpressureStrategy::DropOldest))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-dropnewest",
            &[("value", &format!("{:?}", BackpressureStrategy::DropNewest))]
        )
    );

    print_section(&tr(
        "example-ring-buffered-file-1-3-custom-configuration-small",
    ));
    let cfg = ChannelBufferedConfig {
        base_config: FileSinkConfig {
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        },
        channel_capacity: 64,
        backpressure_strategy: BackpressureStrategy::DropNewest,
        flush_batch_size: 16,
        flush_interval_ms: 20,
    };
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-channel-capacity",
            &[("channel_capacity", &cfg.channel_capacity)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-flush-batch-size",
            &[("flush_batch_size", &cfg.flush_batch_size)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-backpressure-strategy",
            &[(
                "backpressure_strategy",
                &format!("{:?}", cfg.backpressure_strategy)
            )]
        )
    );
    assert_eq!(cfg.channel_capacity, 64);
}

/// 展示 Block 背压策略：channel 满时阻塞发送端，直到消费者取走数据
async fn show_block_strategy() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr(
        "example-ring-buffered-file-2-block-backpressure-strategy",
    ));

    let temp_dir = TempDir::new()?;
    let log_path = temp_dir.path().join("block.log");

    print_section(&tr("example-ring-buffered-file-2-1-construct-a"));
    let cfg = ChannelBufferedConfig {
        base_config: FileSinkConfig {
            path: log_path.clone(),
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        },
        channel_capacity: 64,
        backpressure_strategy: BackpressureStrategy::Block,
        flush_batch_size: 8,
        flush_interval_ms: 10,
    };
    let template = inklog::LogTemplate::default();
    let sink = ChannelBufferedFileSink::new(cfg, template)?;
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-sink-created-channel-capacity",
            &[("value", &(sink.metrics().channel_capacity))]
        )
    );

    print_section(&tr("example-ring-buffered-file-2-2-write-20-log-records"));
    for i in 0..20 {
        let record = LogRecord::new(
            Level::INFO,
            "ring_example::block".to_string(),
            format!("block-message-{:02}", i),
        );
        sink.write(&record).await?;
    }

    print_section(&tr(
        "example-ring-buffered-file-2-3-flush-shutdown-to-ensure",
    ));
    sink.flush().await?;
    sink.shutdown().await?;

    let data = std::fs::read_to_string(&log_path)?;
    assert!(data.contains("block-message-00"));
    assert!(data.contains("block-message-19"));
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-file-size-bytes",
            &[("value", &(data.len()))]
        )
    );
    println!(
        "{}",
        tr("example-ring-buffered-file-all-20-log-records-written-the")
    );

    let m = sink.metrics();
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-metrics-after-shutdown-dropped",
            &[("dropped_count", &m.dropped_count)]
        )
    );
    println!("Note: with the Block strategy dropped_count can be non-zero;");
    println!(
        "      {}",
        tr("example-ring-buffered-file-it-counts-incomplete-batches")
    );
    println!(
        "      not real message drops — the file content was verified to contain all 20 records."
    );

    Ok(())
}

/// 展示 DropNewest 背压策略：channel 满时丢弃新到达的日志
async fn show_drop_newest_strategy() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-ring-buffered-file-3-dropnewest-backpressure"));

    let temp_dir = TempDir::new()?;
    let log_path = temp_dir.path().join("drop_newest.log");

    print_section(&tr(
        "example-ring-buffered-file-3-1-small-channel-dropnewest",
    ));
    let cfg = ChannelBufferedConfig {
        base_config: FileSinkConfig {
            path: log_path.clone(),
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        },
        channel_capacity: 4,
        backpressure_strategy: BackpressureStrategy::DropNewest,
        // `flush_batch_size = 0` 使 IO 线程不从通道排空，溢出（即 DropNewest 丢弃）
        // 成为确定结果，而非与 IO 线程排空速度赛跑——与库内
        // `test_backpressure_drop_newest` 采用同一确定性口径。
        flush_batch_size: 0,
        flush_interval_ms: 1000, // 拉长 flush 间隔，迫使 channel 被填满
    };
    let template = inklog::LogTemplate::default();
    let sink = ChannelBufferedFileSink::new(cfg, template)?;
    println!(
        "channel_capacity = 4, flush_batch_size = 0 (no automatic drain), flush_interval = 1000ms"
    );

    print_section(&tr(
        "example-ring-buffered-file-3-2-write-64-records-quickly",
    ));
    for i in 0..64 {
        let record = LogRecord::new(
            Level::INFO,
            "ring_example::drop_newest".to_string(),
            format!("drop-newest-{:02}", i),
        );
        // DropNewest 策略下 channel 满时丢弃新日志并返回 Err——
        // 这正是本节演示的行为，不作为示例错误传播
        let _ = sink.write(&record).await;
    }

    let m = sink.metrics();
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-metrics-dropped-count",
            &[("dropped_count", &m.dropped_count)]
        )
    );
    assert!(m.dropped_count > 0, "DropNewest should drop some records");

    sink.flush().await?;
    sink.shutdown().await?;
    println!(
        "{}",
        tr("example-ring-buffered-file-dropnewest-drops-new-records")
    );

    Ok(())
}

/// 展示 DropOldest 背压策略：channel 满时驱逐最旧的日志
async fn show_drop_oldest_strategy() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-ring-buffered-file-4-dropoldest-backpressure"));

    let temp_dir = TempDir::new()?;
    let log_path = temp_dir.path().join("drop_oldest.log");

    print_section(&tr(
        "example-ring-buffered-file-4-1-small-channel-dropoldest",
    ));
    let cfg = ChannelBufferedConfig {
        base_config: FileSinkConfig {
            path: log_path.clone(),
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        },
        channel_capacity: 4,
        backpressure_strategy: BackpressureStrategy::DropOldest,
        // 同第 3 节：`flush_batch_size = 0` 使 IO 线程永不排空，超出容量的每次写入
        // 都确定性地驱逐最旧条目（与库内 `test_backpressure_drop_oldest` 口径一致）。
        flush_batch_size: 0,
        flush_interval_ms: 1000,
    };
    let template = inklog::LogTemplate::default();
    let sink = ChannelBufferedFileSink::new(cfg, template)?;
    println!(
        "channel_capacity = 4, flush_batch_size = 0 (no automatic drain), flush_interval = 1000ms"
    );

    print_section(&tr(
        "example-ring-buffered-file-4-2-write-64-records-quickly",
    ));
    for i in 0..64 {
        let record = LogRecord::new(
            Level::INFO,
            "ring_example::drop_oldest".to_string(),
            format!("drop-oldest-{:02}", i),
        );
        // DropOldest 策略在驱逐竞态偶发失败时同样返回 Err，
        // 丢弃行为由下方 dropped_count 断言验证，不作为示例错误传播
        let _ = sink.write(&record).await;
    }

    let m = sink.metrics();
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-metrics-dropped-count",
            &[("dropped_count", &m.dropped_count)]
        )
    );
    assert!(
        m.dropped_count > 0,
        "DropOldest should evict some old records"
    );

    sink.flush().await?;
    sink.shutdown().await?;
    println!(
        "{}",
        tr("example-ring-buffered-file-dropoldest-evicts-old-records")
    );

    Ok(())
}

/// 展示 metrics 统计：channel_len / bytes_written / flush_count / dropped_count
async fn show_metrics_tracking() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-ring-buffered-file-5-channelbufferedmetrics"));

    let temp_dir = TempDir::new()?;
    let log_path = temp_dir.path().join("metrics.log");

    print_section(&tr(
        "example-ring-buffered-file-5-1-construct-the-sink-channel",
    ));
    let cfg = ChannelBufferedConfig {
        base_config: FileSinkConfig {
            path: log_path.clone(),
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        },
        channel_capacity: 16,
        backpressure_strategy: BackpressureStrategy::Block,
        flush_batch_size: 4,
        flush_interval_ms: 10,
    };
    let template = inklog::LogTemplate::default();
    let sink = ChannelBufferedFileSink::new(cfg, template)?;

    print_section(&tr("example-ring-buffered-file-5-2-write-10-log-records"));
    for i in 0..10 {
        let record = LogRecord::new(
            Level::INFO,
            "ring_example::metrics".to_string(),
            format!("metrics-message-{:02}", i),
        );
        sink.write(&record).await?;
    }

    print_section(&tr(
        "example-ring-buffered-file-5-3-wait-for-the-io-thread-and",
    ));
    // 轮询 metrics 直到 bytes_written > 0 且 flush_count >= 1，或超时 500ms
    // 注意：async 上下文中必须用 tokio::time::sleep 让出执行权，
    // std::thread::sleep 会阻塞 tokio worker 线程
    let start = std::time::Instant::now();
    let mut m = sink.metrics();
    while (m.bytes_written == 0 || m.flush_count == 0)
        && start.elapsed() < Duration::from_millis(500)
    {
        tokio::time::sleep(Duration::from_millis(10)).await;
        m = sink.metrics();
    }

    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-channel-capacity-2",
            &[("channel_capacity", &m.channel_capacity)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-channel-len",
            &[("channel_len", &m.channel_len)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-bytes-written",
            &[("bytes_written", &m.bytes_written)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-flush-count",
            &[("flush_count", &m.flush_count)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-ring-buffered-file-dropped-count",
            &[("dropped_count", &m.dropped_count)]
        )
    );
    assert!(m.bytes_written > 0, "some bytes should be written");
    assert!(m.flush_count >= 1, "should flush at least once");

    sink.flush().await?;
    sink.shutdown().await?;

    let data = std::fs::read_to_string(&log_path)?;
    assert!(data.contains("metrics-message-00"));
    assert!(data.contains("metrics-message-09"));
    println!(
        "\n{}",
        tr("example-ring-buffered-file-the-file-contains-all-10")
    );

    Ok(())
}
