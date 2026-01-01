use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use inklog::masking::{self, DataMasker};
use inklog::sink::database::convert_logs_to_parquet;
use inklog::{
    config::{FileSinkConfig, PerformanceConfig},
    log_record::LogRecord,
    template::LogTemplate,
    InklogConfig, LoggerManager,
};
use rand::Rng;
use rayon::prelude::*;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tracing::Level;
use tracing_subscriber::prelude::*;

fn bench_log_creation(c: &mut Criterion) {
    c.bench_function("create_log_record", |b| {
        b.iter(|| {
            LogRecord::new(
                Level::INFO,
                "benchmark_target".to_string(),
                "This is a benchmark log message with some data".to_string(),
            )
        })
    });
}

fn bench_console_sink_latency(c: &mut Criterion) {
    let _rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("console_sink_latency");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("console_sync_latency", |b| {
        b.iter_custom(|iters| {
            let config = InklogConfig {
                file_sink: None,
                console_sink: Some(inklog::config::ConsoleSinkConfig {
                    enabled: true,
                    colored: true,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let rt = Runtime::new().unwrap();
            let (_manager, subscriber, filter) =
                rt.block_on(async { LoggerManager::build_detached(config).await.unwrap() });
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            for i in 0..iters {
                tracing::info!(iteration = i, "Console latency test message");
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_channel_enqueue_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("channel_enqueue_latency");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("async_channel_enqueue", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("channel_bench.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            for i in 0..iters {
                tracing::info!(iteration = i, "Channel enqueue test message");
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_throughput_sustained(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput_sustained");
    group.throughput(Throughput::Elements(100));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("sustained_5_logs_per_sec", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("throughput_sustained.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            let mut count = 0;
            let target_duration = Duration::from_millis(200); // 5 logs per second
            let mut next_log = Instant::now();

            while count < iters {
                if Instant::now() >= next_log {
                    tracing::info!(count = count, "Sustained throughput test");
                    count += 1;
                    next_log += target_duration;
                }
                tokio::task::yield_now().await;
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_throughput_burst(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput_burst");
    group.throughput(Throughput::Elements(500));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("burst_500_logs_per_sec", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("throughput_burst.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            for i in 0..iters {
                tracing::info!(iteration = i, "Burst throughput test message");
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_file_sink_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_sink");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("async_file_log", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("bench.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None, // Disable console for pure file bench
                ..Default::default()
            };

            // Use build_detached to avoid setting global subscriber repeatedly
            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);

            // Set subscriber for the current scope/thread
            let _guard = tracing::subscriber::set_default(registry);

            let start = std::time::Instant::now();
            for i in 0..iters {
                tracing::info!(iteration = i, "Benchmark log message");
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_noop_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("noop_sink");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("async_noop_log", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let config = InklogConfig {
                file_sink: None,
                console_sink: None,
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);

            let _guard = tracing::subscriber::set_default(registry);

            let start = std::time::Instant::now();
            for i in 0..iters {
                tracing::info!(iteration = i, "Benchmark log message");
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("steady_state_memory", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("memory_bench.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            // 模拟稳态运行
            let start = Instant::now();
            for i in 0..iters.min(1000) {
                tracing::info!(count = i, "Memory usage test message");
                if i % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }
            start.elapsed()
        })
    });
    group.finish();
}

// ============ Parquet 转换性能测试 ============

fn bench_parquet_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("parquet_conversion");
    group.measurement_time(Duration::from_secs(10));

    // 生成测试日志数据
    fn generate_test_logs(count: usize) -> Vec<inklog::sink::database::Model> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|i| inklog::sink::database::Model {
                id: i as i64,
                timestamp: chrono::Utc::now(),
                level: format!("{:?}", Level::INFO),
                target: format!("target_{}", i % 10),
                message: format!(
                    "Test log message {} with some additional data for testing performance",
                    i
                ),
                fields: Some(serde_json::json!({
                    "user_id": rng.gen::<u64>() % 10000,
                    "request_id": format!("req-{:x}", rng.gen::<u64>()),
                    "duration_ms": rng.gen::<u64>() % 500,
                })),
                file: Some(format!("src/file_{}.rs", i % 5)),
                line: Some((i % 1000) as i32),
                thread_id: format!("thread-{}", i % 4),
            })
            .collect()
    }

    // 小批量转换
    group.bench_function("convert_100_logs", |b| {
        b.iter(|| {
            let logs = generate_test_logs(100);
            let _ = convert_logs_to_parquet(&logs);
        })
    });

    // 中批量转换
    group.bench_function("convert_1000_logs", |b| {
        b.iter(|| {
            let logs = generate_test_logs(1000);
            let _ = convert_logs_to_parquet(&logs);
        })
    });

    // 大批量转换
    group.bench_function("convert_10000_logs", |b| {
        b.iter(|| {
            let logs = generate_test_logs(10000);
            let _ = convert_logs_to_parquet(&logs);
        })
    });

    // 吞吐量测试
    group.bench_function("convert_throughput_1k", |b| {
        b.iter(|| {
            let logs = generate_test_logs(1000);
            let start = Instant::now();
            let _ = convert_logs_to_parquet(&logs);
            1000.0 / start.elapsed().as_secs_f64()
        })
    });

    group.finish();
}

// ============ 模板渲染性能测试 ============

fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");
    group.measurement_time(Duration::from_secs(10));

    let template = LogTemplate::new("{timestamp} [{level}] {target} - {message} {fields}");
    let record = LogRecord::new(
        Level::INFO,
        "test_module".to_string(),
        "Test message for performance testing with some additional context data".to_string(),
    );

    group.bench_function("render_simple_template", |b| {
        let simple_template = LogTemplate::new("{timestamp} [{level}] {message}");
        b.iter(|| {
            let _ = simple_template.render(&record);
        })
    });

    group.bench_function("render_complex_template", |b| {
        b.iter(|| {
            let _ = template.render(&record);
        })
    });

    group.finish();
}

// ============ 敏感信息脱敏性能测试 ============

fn bench_masking(c: &mut Criterion) {
    let mut group = c.benchmark_group("masking");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("mask_email", |b| {
        b.iter(|| {
            let text = "Contact: user@example.com for support";
            let _ = masking::mask_email(text);
        })
    });

    group.bench_function("mask_phone", |b| {
        b.iter(|| {
            let text = "Call me at 13812345678 for more info";
            let _ = masking::mask_phone(text);
        })
    });

    group.bench_function("mask_data_masher", |b| {
        b.iter(|| {
            let masker = DataMasker::new();
            let text = "User john@example.com, phone 13912345678, ID 110101199001011234";
            let _ = masker.mask(text);
        })
    });

    group.finish();
}

// ============ 背压性能测试 ============

fn bench_backpressure(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("backpressure");
    group.measurement_time(Duration::from_secs(15));

    // 测试小容量 Channel 的背压效果
    group.bench_function("backpressure_100_capacity", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("backpressure.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                performance: PerformanceConfig {
                    channel_capacity: 100,
                    ..Default::default()
                },
                ..Default::default()
            };

            let (_manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            for i in 0..iters.min(500) {
                tracing::info!(
                    iteration = i,
                    "Backpressure test message with some additional content"
                );
            }
            start.elapsed()
        })
    });

    // 测试大批量写入时的延迟
    group.bench_function("backpressure_burst_10k", |b| {
        b.to_async(&rt).iter_custom(|_iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("backpressure_burst.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                performance: PerformanceConfig {
                    channel_capacity: 10000,
                    ..Default::default()
                },
                ..Default::default()
            };

            let (manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();
            for i in 0..10000 {
                tracing::info!(iteration = i, "Backpressure burst test message");
            }

            // 等待所有日志处理完成
            manager.shutdown().unwrap();

            start.elapsed()
        })
    });

    group.finish();
}

// ============ 并发性能测试 ============

fn bench_concurrency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrency");
    group.measurement_time(Duration::from_secs(15));
    group.throughput(Throughput::Elements(1));

    // 多线程并发写入
    group.bench_function("concurrent_4_threads", |b| {
        b.to_async(&rt).iter_custom(|_iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("concurrent.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();

            // 使用 rayon 进行并行写入
            let result = (0..1000u32)
                .into_par_iter()
                .map(|i| {
                    tracing::info!(
                        thread_id = rayon::current_thread_index(),
                        iteration = i,
                        "Concurrent log"
                    );
                    i
                })
                .count();

            // 等待完成
            manager.shutdown().unwrap();

            assert_eq!(result, 1000);
            start.elapsed()
        })
    });

    // 异步任务并发
    group.bench_function("concurrent_async_tasks", |b| {
        b.to_async(&rt).iter_custom(|_iters| async move {
            let temp_dir = TempDir::new().unwrap();
            let log_path = temp_dir.path().join("async_concurrent.log");

            let config = InklogConfig {
                file_sink: Some(FileSinkConfig {
                    enabled: true,
                    path: log_path.to_path_buf(),
                    ..Default::default()
                }),
                console_sink: None,
                ..Default::default()
            };

            let (manager, subscriber, filter) =
                LoggerManager::build_detached(config).await.unwrap();
            let registry = tracing_subscriber::registry().with(subscriber).with(filter);
            let _guard = tracing::subscriber::set_default(registry);

            let start = Instant::now();

            // 并发任务
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    tokio::spawn(async {
                        for i in 0..250 {
                            tracing::info!(
                                task_id = format!("{}", tokio::task::id()),
                                iteration = i,
                                "Async concurrent log"
                            );
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.await.unwrap();
            }

            manager.shutdown().unwrap();
            start.elapsed()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_log_creation,
    bench_console_sink_latency,
    bench_channel_enqueue_latency,
    bench_throughput_sustained,
    bench_throughput_burst,
    bench_memory_usage,
    bench_file_sink_throughput,
    bench_noop_throughput,
    bench_parquet_conversion,
    bench_template_rendering,
    bench_masking,
    bench_backpressure,
    bench_concurrency
);
criterion_main!(benches);
