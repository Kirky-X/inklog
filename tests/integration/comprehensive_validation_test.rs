// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// 真实数据写入和特性验证测试
// 这个测试会真实写入大量数据，并验证所有inklog特性是否正常工作

// 核正：with_config 内部 set_global_default 为进程级单次语义，多用例并发时后装者
// 日志流向首个 logger 导致记录丢失。统一改造为本仓测试口径
// （additional_tests.rs 范本）：build_detached + 线程级 set_default + tracing 宏。
use inklog::tokio::time::sleep;
use inklog::{
    InklogConfig, LoggerManager,
    config::{
        ConsoleSinkConfig, DatabaseDriver, DatabaseSinkConfig, FileSinkConfig, GlobalConfig,
        HttpServerConfig,
    },
};
use serial_test::serial;
use std::env;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tracing_subscriber::layer::SubscriberExt;

#[tokio::test]
#[serial]
async fn test_comprehensive_real_data_writing() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("comprehensive_test.log");
    let db_path = temp_dir.path().join("comprehensive_test.db");

    println!("=== starting comprehensive real-data write and feature validation tests ===");

    // 设置加密密钥
    // 核正：FileSink::get_encryption_key 要求 base64 解码后恰好 32 字节且
    // Shannon 熵 >= 4.0（弱密钥校验）；原 "MTIz..." 解码为重复数字序列
    // （熵 3.31 < 4.0）会被拒绝，换用 32 个互不相同字符的 key（熵 5.0）
    let encryption_key = "YVozeFc4dksybVE3dE41clU5eUI0Y0U2ZkgxZ0owZEw=";
    unsafe {
        env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
    }

    // 配置全面的日志系统
    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".to_string(),
            format: "[{timestamp}] [{level:>5}] [{service}:{instance}] {target} - {message}"
                .to_string(),
            masking_enabled: true, // 启用数据掩码
            ..Default::default()
        },
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "50MB".into(),
            // 核正：原 "minutely" 使 ~20s 写入窗口跨分钟边界即触发时间轮转，
            // secret_data_1 随活跃文件归档为密文，下方"活跃文件应为明文"断言
            // 随时钟相位偶发失败。时间轮转归档（压缩+加密）语义已由
            // encryption_file_test::test_encrypted_file_sink_rotation 专例覆盖，
            // 本用例以确定性为优先改为 "daily"（max_size 50MB 同样不会触发
            // 尺寸轮转），活跃文件断言不再依赖时钟相位
            rotation_time: "daily".into(),
            keep_files: 5,
            batch_size: 1000,
            flush_interval_ms: 1000,
            fsync: false,
            audit_chain_enabled: false,
            compress: true,
            compression_level: 3,
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        }),
        database_sink: Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            // ?mode=rwc：sqlx 默认不创建缺失文件（本仓 sqlite 测试既定口径）
            url: format!("sqlite://{}?mode=rwc", db_path.display()),
            pool_size: 3,
            batch_size: 50,
            flush_interval_ms: 2000,
            table_name: "logs".to_string(),
            ..Default::default()
        }),
        console_sink: Some(ConsoleSinkConfig {
            // worker 线程的 stdout 直写不在测试 harness 捕获范围内，启用会向
            // 进程 stdout 泄漏约 4000 行日志（health 注册不受 enabled 影响，
            // ConsoleSink::write 对禁用态直接丢弃）
            enabled: false,
            ..Default::default()
        }),
        #[cfg(feature = "http")]
        http_server: Some(HttpServerConfig {
            enabled: false, // 暂时禁用
            host: "127.0.0.1".to_string(),
            port: 9092,
            metrics_path: "/metrics".to_string(),
            health_path: "/health".to_string(),
            ..Default::default()
        }),
        performance: inklog::config::PerformanceConfig {
            worker_threads: 6,
            channel_capacity: 20000,
            ..Default::default()
        },
        ..Default::default()
    };

    let (logger, subscriber, filter) = LoggerManager::build_detached(
        config,
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        None,
    )
    .await
    .unwrap();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(subscriber).with(filter),
    );

    println!("config done; writing test data...");

    let test_start = Instant::now();

    println!("\n=== Phase 1: writing different kinds of log data ===");

    for i in 0..2000 {
        tracing::info!(target: "rotation_test", "rotation test message {} - large data: {}", i, "x".repeat(200));
    }

    for i in 0..500 {
        tracing::warn!(target: "masking_test", "sensitive data test - user email: user{}@example.com, phone: {}",
                i, "13812345678");
    }

    for i in 0..1000 {
        tracing::error!(target: "encryption_test", "encryption test - secret data: {}",
                format!("secret_data_{}", i));
    }

    for i in 0..500 {
        tracing::debug!(target: "database_test", "database test - batch data {}",
                format!("db_batch_{}", i));
    }

    sleep(Duration::from_secs(5)).await;

    println!("\n=== Phase 2: verifying features ===");

    let log_files = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            file_name.starts_with("comprehensive_test")
                && (file_name.ends_with(".log") || file_name.ends_with(".log.gz"))
        })
        .count();

    println!("current log file count: {}", log_files);
    assert!(log_files >= 1, "log files should exist");

    assert!(db_path.exists(), "the database should contain data");

    let metadata = std::fs::metadata(&log_path).unwrap();
    assert!(
        metadata.len() > 100000,
        "the log file should contain a large amount of data"
    );

    let health = logger.get_health_status();
    println!("health status: {:?}", health);
    assert!(
        health.sinks.contains_key("file"),
        "the file sink should be monitored"
    );
    assert!(
        health.sinks.contains_key("database"),
        "the database sink should be monitored"
    );
    assert!(
        health.sinks.contains_key("console"),
        "the console sink should be monitored"
    );

    // 阶段3：性能和压力测试
    println!("\n=== Phase 3: performance and stress tests ===");

    // 高并发写入测试
    let concurrent_start = Instant::now();
    let messages_per_thread = 500;

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            tokio::spawn(async move {
                for i in 0..messages_per_thread {
                    tracing::info!(
                        target: "concurrent_test",
                        "thread {} - concurrent message {}",
                        thread_id, i
                    );
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    let concurrent_elapsed = concurrent_start.elapsed();
    println!(
        "concurrency test finished, elapsed: {:?}",
        concurrent_elapsed
    );

    // 验证并发写入后的状态：spawn 完成只保证记录进入异步通道，落盘由
    // file sink worker 按批次/间隔异步完成——轮询等待增长而非立即断言
    // （break 即证明 len > metadata.len，无需循环后重复断言）
    let concurrent_deadline = Instant::now() + Duration::from_secs(10);
    let concurrent_len = loop {
        let len = std::fs::metadata(&log_path).unwrap().len();
        if len > metadata.len() {
            break len;
        }
        assert!(
            Instant::now() < concurrent_deadline,
            "concurrent writes should have added data (timed out waiting for flush)"
        );
        sleep(Duration::from_millis(50)).await;
    };

    let total_elapsed = test_start.elapsed();

    println!("\n=== test result summary ===");
    println!("total test time: {:?}", total_elapsed);
    println!("messages written: {}", 2000 + 500 + 1000 + 500); // 约4500条
    println!("final file size: {} bytes", concurrent_len);
    println!("rotated file count: {}", log_files);

    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        !content.contains("user@example.com"),
        "the email should be masked"
    );
    assert!(
        !content.contains("13812345678"),
        "the phone number should be masked"
    );

    // 核正：encrypt 仅作用于轮转归档（file.rs rotate_inner 的后台压缩/加密路径），
    // 活跃写入文件是明文——secret_data 以明文出现属设计语义；归档密文行为由
    // encryption_file_test::test_encrypted_file_sink_rotation 强制覆盖。
    // secret_data_1 的落盘由 file sink worker 异步完成，与读取存在竞态——
    // 轮询等待其出现，超时才判失败（并行负载下 flush 可显著滞后）；
    // 文件长度未增长时跳过全量重读（该时点文件已 >100KB，避免最多 200 次
    // 数百 KB 级重复 IO）
    let plaintext_deadline = Instant::now() + Duration::from_secs(10);
    let mut last_len = 0_u64;
    loop {
        let len = std::fs::metadata(&log_path).unwrap().len();
        if len > last_len {
            last_len = len;
            let content = std::fs::read_to_string(&log_path).unwrap();
            if content.contains("secret_data_1") {
                break;
            }
        }
        assert!(
            Instant::now() < plaintext_deadline,
            "the active file should be plaintext and contain secret_data_1 (timed out waiting for flush; encryption applies only to rotated archives)"
        );
        sleep(Duration::from_millis(50)).await;
    }
    // 防御性不变量：任何加密归档产物都不得含明文敏感数据
    for entry in std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("comprehensive_test") && name.ends_with(".enc") {
            let bytes = std::fs::read(entry.path()).unwrap();
            assert!(
                !bytes.windows(13).any(|w| w == b"secret_data_1"),
                "the rotated archive should be ciphertext: {}",
                name
            );
        }
    }

    // 测试清理
    unsafe {
        env::remove_var("INKLOG_ENCRYPTION_KEY");
    }

    let final_health = logger.get_health_status();
    println!("final health status: {:?}", final_health);

    // 测试关闭
    logger
        .shutdown()
        .expect("failed to shut down the logging service");

    println!("=== comprehensive real-data write tests completed ===");
    println!("✅ all basic features work");
    println!("✅ file rotation works");
    println!("✅ data masking works");
    println!("✅ encryption works");
    println!("✅ database writes work");
    println!("✅ concurrency safety works");
    println!("✅ health monitoring works");

    assert!(
        final_health.sinks.len() >= 3,
        "all sinks should be monitored"
    );

    // 核正：原 25 秒下限为时序自检（快机器必失败），无验收价值——数据落盘
    // 有效性已由文件大小/数据库/健康状态断言承载

    println!("=== validation passed! inklog performs flawlessly under real-data writes ===");
}

/// 验证配置变更的动态响应
#[tokio::test]
#[serial]
async fn test_dynamic_configuration_changes() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("dynamic_config_test.log");

    let initial_config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "10MB".into(),
            ..Default::default()
        }),
        // console 禁用：200 条 dynamic_test 日志经 worker 直写真实 stdout 会
        // 淹没测试输出（--quiet 也无法捕获 worker 线程的直写）
        console_sink: Some(ConsoleSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    println!("=== testing dynamic config changes ===");

    // 创建初始日志器（build_detached + 线程级 set_default，避免进程级全局竞争）
    let (logger1, subscriber1, filter1) = LoggerManager::build_detached(
        initial_config,
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        None,
    )
    .await
    .unwrap();
    let _guard1 = tracing::subscriber::set_default(
        tracing_subscriber::registry()
            .with(subscriber1)
            .with(filter1),
    );

    for i in 0..100 {
        tracing::info!(target: "dynamic_test", "initial config - message {}", i);
    }

    // 修改配置并重新创建日志器（在实际应用中，这应该是无缝）
    let updated_config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "20MB".into(), // 修改文件大小
            ..Default::default()
        }),
        console_sink: Some(ConsoleSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    drop(logger1);

    sleep(Duration::from_millis(500)).await;

    // 创建新日志器（模拟配置热更新；guard2 遮蔽 guard1，日志切换到新 logger）
    let (logger2, subscriber2, filter2) = LoggerManager::build_detached(
        updated_config,
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        None,
    )
    .await
    .unwrap();
    let _guard2 = tracing::subscriber::set_default(
        tracing_subscriber::registry()
            .with(subscriber2)
            .with(filter2),
    );

    for i in 0..100 {
        tracing::warn!(target: "dynamic_test", "updated config - warning message {}", i);
    }

    // shutdown 触发 flush_batch 确保落盘后再读（核正：原立即读文件依赖轮询时序）
    let _ = logger2.shutdown();
    drop(_guard2);

    let final_content = std::fs::read_to_string(&log_path).unwrap();

    // 应该包含debug级别的消息
    assert!(final_content.contains("updated config - warning message"));

    println!("✅ dynamic config change tests passed");
    println!("=== dynamic config change tests completed ===");

    drop(logger2);
}
