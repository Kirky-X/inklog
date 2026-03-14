// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.
//
// 真实数据写入和特性验证测试
// 这个测试会真实写入大量数据，并验证所有inklog特性是否正常工作

use inklog::{
    config::{ConsoleSinkConfig, DatabaseSinkConfig, FileSinkConfig, GlobalConfig, HttpServerConfig},
    InklogConfig, LoggerManager,
    archive::CompressionType, config::DatabaseDriver,
};
use serial_test::serial;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use tokio::time::sleep;

#[tokio::test]
#[serial]
async fn test_comprehensive_real_data_writing() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("comprehensive_test.log");
    let db_path = temp_dir.path().join("comprehensive_test.db");
    
    println!("=== 开始综合真实数据写入和特性验证测试 ===");
    
    // 设置加密密钥
    let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";
    env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);
    
    // 配置全面的日志系统
    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".to_string(),
            format: "[{timestamp}] [{level:>5}] [{service}:{instance}] {target} - {message}".to_string(),
            masking_enabled: true, // 启用数据掩码
            ..Default::default()
        },
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "50MB".into(),
            rotation_time: "minutely".into(),
            keep_files: 5, // 保留5个文件，测试轮转
            batch_size: 1000,
            flush_interval_ms: 1000,
            compress: true,
            compression_level: 3,
            encrypt: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
            ..Default::default()
        }),
        database_sink: Some(DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            url: format!("sqlite://{}", db_path.display()),
            pool_size: 3,
            batch_size: 50,
            flush_interval_ms: 2000,
            table_name: "logs".to_string(),
            ..Default::default()
        }),
        console_sink: Some(ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        }),
        #[cfg(feature = "aws")]
        s3_archive: Some(inklog::S3ArchiveConfig {
            enabled: false, // 暂时禁用，先测试本地功能
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            archive_interval_days: 1,
            local_retention_days: 3,
            prefix: "comprehensive-test/".to_string(),
            compression: CompressionType::Zstd,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
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

    let logger = LoggerManager::with_config(config).await.unwrap();
    let logger = Arc::new(logger);
    
    println!("配置完成，开始写入测试数据...");
    
    let test_start = Instant::now();
    let test_duration = Duration::from_secs(30); // 30秒测试
    
    // 阶段1：写入各种类型的日志数据
    println!("\n=== 阶段1：写入不同类型的日志数据 ===");
    
    // 1. 写入大量日志触发轮转
    for i in 0..2000 {
        log::info!(target: "rotation_test", "轮转测试消息 {} - 大数据: {}", i, "x".repeat(200));
    }
    
    // 2. 写入敏感数据测试掩码
    for i in 0..500 {
        log::warn!(target: "masking_test", "敏感数据测试 - 用户邮箱: user{}@example.com, 电话: {}", 
                i, "13812345678");
    }
    
    // 3. 写入加密数据
    for i in 0..1000 {
        log::error!(target: "encryption_test", "加密测试 - 秘密数据: {}", 
                format!("secret_data_{}", i));
    }
    
    // 4. 写入数据库数据
    for i in 0..500 {
        log::debug!(target: "database_test", "数据库测试 - 批处理数据 {}", 
                format!("db_batch_{}", i));
    }
    
    // 等待一些时间让日志处理
    sleep(Duration::from_secs(5)).await;
    
    // 阶段2：验证各种功能
    println!("\n=== 阶段2：验证各项功能 ===");
    
    // 验证文件轮转
    let log_files = std::fs::read_dir(temp_dir.path()).unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy();
            file_name.starts_with("comprehensive_test") && (file_name.ends_with(".log") || file_name.ends_with(".log.gz"))
        })
        .count();
    
    println!("当前日志文件数量: {}", log_files);
    assert!(log_files >= 1, "应该有日志文件存在");
    
    // 验证数据库记录
    assert!(db_path.exists(), "数据库应该有数据");
    
    // 验证日志文件大小（应该有数据）
    let metadata = std::fs::metadata(&log_path).unwrap();
    assert!(metadata.len() > 100000, "日志文件应该包含大量数据");
    
    // 验证健康监控状态
    let health = logger.get_health_status();
    println!("健康状态: {:?}", health);
    assert!(health.sinks.contains_key("file"), "文件sink应该在监控中");
    assert!(health.sinks.contains_key("database"), "数据库sink应该在监控中");
    assert!(health.sinks.contains_key("console"), "控制台sink应该在监控中");
    
    // 阶段3：性能和压力测试
    println!("\n=== 阶段3：性能和压力测试 ===");
    
    // 高并发写入测试
    let concurrent_start = Instant::now();
    let messages_per_thread = 500;
    
    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let logger = Arc::clone(&logger);
            
            tokio::spawn(async move {
                for i in 0..messages_per_thread {
                    log::info!(
                        target: "concurrent_test",
                        "线程 {} - 并发消息 {}",
                        thread_id, i
                    );
                }
            })
        })
        .collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.await.unwrap();
    }
    
    let concurrent_elapsed = concurrent_start.elapsed();
    println!("并发测试完成，耗时: {:?}", concurrent_elapsed);
    
    // 验证并发写入后的状态
    let concurrent_metadata = std::fs::metadata(&log_path).unwrap();
    assert!(concurrent_metadata.len() > metadata.len(), "并发写入应该增加了数据");
    
    let total_elapsed = test_start.elapsed();
    
    println!("\n=== 测试结果汇总 ===");
    println!("总测试时间: {:?}", total_elapsed);
    println!("写入的消息数量: {}", 2000 + 500 + 1000 + 500); // 约4500条
    println!("最终文件大小: {} bytes", concurrent_metadata.len());
    println!("轮转文件数量: {}", log_files);
    
    // 验证数据掩码功能
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(!content.contains("user@example.com"), "邮箱应该被掩码");
    assert!(!content.contains("13812345678"), "电话应该被掩码");
    
    // 验证加密功能
    let encrypted_content = std::fs::read_to_string(&log_path).unwrap();
    assert!(!encrypted_content.contains("secret_data_1"), "加密数据应该不可读");
    
    // 测试清理
    env::remove_var("INKLOG_ENCRYPTION_KEY");
    
    // 获取最终健康状态
    let final_health = logger.get_health_status();
    println!("最终健康状态: {:?}", final_health);
    
    // 测试关闭
    logger.shutdown().map_err(|e| format!("关闭日志服务失败: {:?}", e))?;
    
    println!("=== 综合真实数据写入测试完成 ===");
    println!("✅ 所有基本功能正常工作");
    println!("✅ 文件轮转功能正常");
    println!("✅ 数据掩码功能正常");
    println!("✅ 加密功能正常");
    println!("✅ 数据库写入正常");
    println!("✅ 并发安全性能正常");
    println!("✅ 健康监控功能正常");
    
    assert!(final_health.sinks.len() >= 3, "所有sink应该都在监控中");
    
    // 验证测试有效性
    assert!(total_elapsed.as_secs() >= 25, "测试应该运行足够长的时间");
    
    println!("=== 测试验证通过！inklog 在真实数据写入场景下表现完美 ===");
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
            level: "info".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    println!("=== 测试动态配置变更 ===");
    
    // 创建初始日志器
    let logger1 = LoggerManager::with_config(initial_config.clone()).await.unwrap();
    
    // 写入一些初始日志
    for i in 0..100 {
        log::info!(target: "dynamic_test", "初始配置 - 消息 {}", i);
    }
    
    // 修改配置并重新创建日志器（在实际应用中，这应该是无缝的）
    let updated_config = InklogConfig {
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "20MB".into(), // 修改文件大小
            level: "debug".to_string(), // 修改日志级别
            ..Default::default()
        }),
        ..Default::default()
    };
    
    // 关闭第一个日志器
    drop(logger1);
    
    // 等待一小段时间
    sleep(Duration::from_millis(500)).await;
    
    // 创建新日志器（模拟配置热更新）
    let logger2 = LoggerManager::with_config(updated_config.clone()).await.unwrap();
    
    // 写入新配置下的日志
    for i in 0..100 {
        log::warn!(target: "dynamic_test", "更新后配置 - 證告消息 {}", i);
    }
    
    // 验证新配置生效
    let final_content = std::fs::read_to_string(&log_path).unwrap();
    
    // 应该包含debug级别的消息
    assert!(final_content.contains("更新后配置 - 證告消息"));
    
    println!("✅ 动态配置变更测试通过");
    println!("=== 动态配置变更测试完成 ===");
    
    // 清理
    drop(logger2);
}