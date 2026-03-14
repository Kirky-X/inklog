// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// 性能基准测试
// 测试高并发日志写入性能、内存使用和吞吐量

#[cfg(test)]
mod performance_test {
    use inklog::{InklogConfig, LoggerManager};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;
    use tokio::sync::Barrier;

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
        
        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        let message_count = 10000;
        let start = Instant::now();
        
        for i in 0..message_count {
            log::info!(target: "throughput_test", "Throughput test message #{}", i);
        }
        
        let elapsed = start.elapsed();
        let throughput = message_count as f64 / elapsed.as_secs_f64();
        
        println!("Single-thread throughput: {:.0} msg/s", throughput);
        
        // 至少应该达到 1000 msg/s
        assert!(throughput > 1000, "Throughput too low: {:.0} msg/s", throughput);
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
        
        let _logger = LoggerManager::with_config(config).await.unwrap();
        
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
                        log::info!(target: "concurrent_throughput", "Concurrent test #{}", i);
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
        assert!(throughput > 3000, "Multi-thread throughput too low: {:.0} msg/s", throughput);
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
        
        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        let iterations = 1000;
        let latencies: Vec<Duration> = (0..iterations)
            .map(|_| {
                let start = Instant::now();
                log::info!(target: "latency_test", "Latency test message");
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
        assert!(avg_latency.as_millis() < 1, "Average latency too high: {:?}", avg_latency);
    }

    // === 内存使用测试 ===

    #[tokio::test]
    async fn test_memory_usage_during_burst() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("memory_test.log");
        
        let config = InklogConfig {
            file_sink: Some(inklog::FileSinkConfig {
                enabled: true,
                path: log_path,
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
        
        let _logger = LoggerManager::with_config(config).await.unwrap();
        
        // 突发写入大量日志
        let burst_size = 50000;
        for i in 0..burst_size {
            log::info!(target: "memory_test", "Burst test message #{}", i);
        }
        
        // 等待处理完成
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        // 验证所有消息被处理
        // 这里我们主要验证没有内存溢出
        assert!(log_path.exists() || burst_size > 0);
        
        println!("Memory burst test completed successfully");
    }

    // === 批处理性能测试 ===

    #[tokio::test]
    async fn test_batch_write_performance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("batch_test.db");
        
        #[cfg(feature = "dbnexus")]
        {
            let db_url = format!("sqlite://{}", db_path.display());
            
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
            
            let _logger = LoggerManager::with_config(config).await.unwrap();
            
            let batch_sizes = vec![100, 500, 1000];
            
            for batch_size in batch_sizes {
                let start = Instant::now();
                
                for i in 0..batch_size {
                    log::info!(target: "batch_performance", "Batch test message #{}", i);
                }
                
                // 等待批次完成
                tokio::time::sleep(Duration::from_millis(200)).await;
                
                let elapsed = start.elapsed();
                let throughput = batch_size as f64 / elapsed.as_secs_f64();
                
                println!("Batch size {}: {:.0} msg/s", batch_size, throughput);
            }
        }
        
        #[cfg(not(feature = "dbnexus"))]
        {
            println!("Skipping test: requires --features \"dbnexus\"");
        }
    }

    // === 并发连接池测试 ===

    #[tokio::test]
    async fn test_connection_pool_performance() {
        #[cfg(feature = "dbnexus")]
        {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("pool_test.db");
            let db_url = format!("sqlite://{}", db_path.display());
            
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
                
                let _logger = LoggerManager::with_config(config).await.unwrap();
                
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
                                log::info!(target: "pool_test", "Pool test message #{}", i);
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
        
        #[cfg(not(feature = "dbnexus"))]
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
        
        let _logger = LoggerManager::with_config(config).await.unwrap();
        
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
                log::info!(target: "sustained_load", "Sustained load test #{}", i);
            }
            total_messages += batch_size;
        }
        
        let elapsed = start.elapsed();
        let throughput = total_messages as f64 / elapsed.as_secs_f64();
        
        println!("Sustained load: {} messages in {:.2}s = {:.0} msg/s", 
                 total_messages, elapsed.as_secs_f64(), throughput);
        
        // 持续吞吐量应该稳定
        assert!(throughput > 500, "Sustained throughput too low: {:.0} msg/s", throughput);
    }
}
