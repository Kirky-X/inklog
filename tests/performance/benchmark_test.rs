// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 性能基准测试
// 测试高并发日志写入性能、内存使用和吞吐量
//
// 核正：统一改造为本仓测试口径（tests/integration/additional_tests.rs 范本）——
// with_config 内部 set_global_default 为进程级单次语义，多用例下后装者的日志
// 流向首个 logger（已 shutdown）导致测量失真（测得的是丢弃速率而非真实写盘
// 吞吐）；改用 build_detached + 线程级 set_default + tracing::info!。

#[cfg(test)]
mod performance_test {
    use inklog::tokio::sync::Barrier;
    use inklog::{InklogConfig, LoggerManager};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    // === 吞吐量测试 ===

    #[tokio::test]
    async fn test_log_throughput_single_thread() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("throughput.log");

        let config = InklogConfig {
            file_sink: Some(inklog::FileSinkConfig {
                enabled: true,
                path: log_path,
                max_size: "1GB".into(),
                batch_size: 1000,
                flush_interval_ms: 100,
                ..Default::default()
            }),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        let (_logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        let message_count = 10000;
        let start = Instant::now();

        for i in 0..message_count {
            tracing::info!(target: "throughput_test", "Throughput test message #{}", i);
        }

        let elapsed = start.elapsed();
        let throughput = message_count as f64 / elapsed.as_secs_f64();

        println!("Single-thread throughput: {:.0} msg/s", throughput);

        // 至少应该达到 1000 msg/s
        assert!(
            throughput > 1000.0,
            "Throughput too low: {:.0} msg/s",
            throughput
        );
    }

    #[tokio::test]
    async fn test_log_throughput_multi_thread() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("concurrent_throughput.log");

        let config = InklogConfig {
            file_sink: Some(inklog::FileSinkConfig {
                enabled: true,
                path: log_path,
                max_size: "1GB".into(),
                batch_size: 2000,
                flush_interval_ms: 50,
                ..Default::default()
            }),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            performance: inklog::PerformanceConfig {
                worker_threads: 8,
                channel_capacity: 50000,
                ..Default::default()
            },
            ..Default::default()
        };

        // #[tokio::test] 默认 current_thread flavor：spawn 任务全在测试线程 poll，
        // 线程级 set_default 对全部并发任务生效
        let (_logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        let thread_count = 10;
        let messages_per_thread = 5000;
        let barrier = Arc::new(Barrier::new(thread_count));
        let counter = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();

        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let barrier = barrier.clone();
                let counter = counter.clone();
                tokio::spawn(async move {
                    barrier.wait().await;
                    for i in 0..messages_per_thread {
                        tracing::info!(target: "concurrent_throughput", "Concurrent test #{}", i);
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        let elapsed = start.elapsed();
        let total_messages = thread_count * messages_per_thread;
        let throughput = total_messages as f64 / elapsed.as_secs_f64();

        println!("Multi-thread throughput: {:.0} msg/s", throughput);

        // 多线程情况下应该达到更高的吞吐量
        assert!(
            throughput > 3000.0,
            "Multi-thread throughput too low: {:.0} msg/s",
            throughput
        );
    }

    // === 延迟测试 ===

    #[tokio::test]
    async fn test_log_latency() {
        let config = InklogConfig {
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let (_logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        let iterations = 1000;
        let latencies: Vec<Duration> = (0..iterations)
            .map(|_| {
                let start = Instant::now();
                tracing::info!(target: "latency_test", "Latency test message");
                start.elapsed()
            })
            .collect();

        // 计算统计信息
        let avg_latency: Duration = latencies.iter().sum();
        let avg_latency = avg_latency / iterations as u32;

        let max_latency = latencies.iter().max().unwrap();
        let min_latency = latencies.iter().min().unwrap();

        println!("Latency stats:");
        println!("  Average: {:?}", avg_latency);
        println!("  Min: {:?}", min_latency);
        println!("  Max: {:?}", max_latency);

        // 平均延迟应该小于 1ms
        assert!(
            avg_latency.as_millis() < 1,
            "Average latency too high: {:?}",
            avg_latency
        );
    }

    // === 内存使用测试 ===

    #[tokio::test]
    async fn test_memory_usage_during_burst() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("memory_test.log");

        let config = InklogConfig {
            file_sink: Some(inklog::FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "1GB".into(),
                batch_size: 5000,
                flush_interval_ms: 200,
                ..Default::default()
            }),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            performance: inklog::PerformanceConfig {
                channel_capacity: 100000,
                ..Default::default()
            },
            ..Default::default()
        };

        let (_logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        // 突发写入大量日志
        let burst_size = 50000;
        for i in 0..burst_size {
            tracing::info!(target: "memory_test", "Burst test message #{}", i);
        }

        // 核正：原断言 `log_path.exists() || burst_size > 0` 恒真无意义；
        // shutdown drain 确保 50000 条全部落盘后按真实行为固化断言
        let _ = _logger.shutdown();
        assert!(log_path.exists(), "突发写入后日志文件应存在");
        let metadata = std::fs::metadata(&log_path).unwrap();
        assert!(metadata.len() > 0, "日志文件应非空");

        println!("Memory burst test completed successfully");
    }

    // === 批处理性能测试 ===

    #[tokio::test]
    async fn test_batch_write_performance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("batch_test.db");

        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        {
            let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

            let config = InklogConfig {
                database_sink: Some(inklog::DatabaseSinkConfig {
                    enabled: true,
                    driver: inklog::config::DatabaseDriver::SQLite,
                    url: db_url,
                    pool_size: 5,
                    batch_size: 500,
                    flush_interval_ms: 100,
                    table_name: "logs".to_string(),
                    ..Default::default()
                }),
                console_sink: Some(inklog::config::ConsoleSinkConfig {
                    enabled: false,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let (_logger, subscriber, filter) =
                LoggerManager::build_detached(config, None).await.unwrap();
            let _guard = tracing::subscriber::set_default(
                tracing_subscriber::registry().with(subscriber).with(filter),
            );

            let batch_sizes = vec![100, 500, 1000];

            for batch_size in batch_sizes {
                let start = Instant::now();

                for i in 0..batch_size {
                    tracing::info!(target: "batch_performance", "Batch test message #{}", i);
                }

                // 等待批次完成
                tokio::time::sleep(Duration::from_millis(200)).await;

                let elapsed = start.elapsed();
                let throughput = batch_size as f64 / elapsed.as_secs_f64();

                println!("Batch size {}: {:.0} msg/s", batch_size, throughput);
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        {
            println!("Skipping test: requires --features \"dbnexus\"");
        }
    }

    // === 并发连接池测试 ===

    #[tokio::test]
    async fn test_connection_pool_performance() {
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("pool_test.db");
            let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

            let pool_sizes = vec![2, 5, 10];

            for pool_size in pool_sizes {
                let config = InklogConfig {
                    database_sink: Some(inklog::DatabaseSinkConfig {
                        enabled: true,
                        driver: inklog::config::DatabaseDriver::SQLite,
                        url: db_url.clone(),
                        pool_size,
                        batch_size: 100,
                        flush_interval_ms: 50,
                        table_name: "logs".to_string(),
                        ..Default::default()
                    }),
                    console_sink: Some(inklog::config::ConsoleSinkConfig {
                        enabled: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let (_logger, subscriber, filter) =
                    LoggerManager::build_detached(config, None).await.unwrap();
                let _guard = tracing::subscriber::set_default(
                    tracing_subscriber::registry().with(subscriber).with(filter),
                );

                // 并发写入测试
                let thread_count = pool_size as usize * 2;
                let messages_per_thread = 500;
                let barrier = Arc::new(Barrier::new(thread_count));

                let start = Instant::now();

                let handles: Vec<_> = (0..thread_count)
                    .map(|_| {
                        let barrier = barrier.clone();
                        tokio::spawn(async move {
                            barrier.wait().await;
                            for i in 0..messages_per_thread {
                                tracing::info!(target: "pool_test", "Pool test message #{}", i);
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.await.unwrap();
                }

                let elapsed = start.elapsed();
                let total = thread_count * messages_per_thread;
                let throughput = total as f64 / elapsed.as_secs_f64();

                println!("Pool size {}: {:.0} msg/s", pool_size, throughput);
            }
        }

        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        {
            println!("Skipping test: requires --features \"dbnexus\"");
        }
    }

    // === 长期稳定性测试 ===

    #[tokio::test]
    async fn test_sustained_load() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("sustained_test.log");

        let config = InklogConfig {
            file_sink: Some(inklog::FileSinkConfig {
                enabled: true,
                path: log_path,
                max_size: "100MB".into(),
                batch_size: 1000,
                flush_interval_ms: 100,
                ..Default::default()
            }),
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            performance: inklog::PerformanceConfig {
                channel_capacity: 50000,
                worker_threads: 4,
                ..Default::default()
            },
            ..Default::default()
        };

        let (_logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        // 持续负载测试：每 100ms 写入 100 条消息，持续 5 秒
        let duration_secs = 5;
        let batch_interval_ms = 100;
        let batch_size = 100;

        let start = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_millis(batch_interval_ms));

        let mut total_messages = 0;

        while start.elapsed().as_secs() < duration_secs {
            interval.tick().await;

            for i in 0..batch_size {
                tracing::info!(target: "sustained_load", "Sustained load test #{}", i);
            }
            total_messages += batch_size;
        }

        let elapsed = start.elapsed();
        let throughput = total_messages as f64 / elapsed.as_secs_f64();

        println!(
            "Sustained load: {} messages in {:.2}s = {:.0} msg/s",
            total_messages,
            elapsed.as_secs_f64(),
            throughput
        );

        // 持续吞吐量应该稳定
        assert!(
            throughput > 500.0,
            "Sustained throughput too low: {:.0} msg/s",
            throughput
        );
    }
}
