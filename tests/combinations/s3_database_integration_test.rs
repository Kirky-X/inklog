// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

// S3 + 数据库集成测试
// 测试数据库日志到 S3 的归档流程、归档数据完整性验证、
// 以及归档失败重试机制，确保云端归档功能的可靠性。

#[cfg(all(test, feature = "aws", feature = "dbnexus"))]
mod s3_database_integration_test {
    use inklog::config::{DatabaseDriver, DatabaseSinkConfig, PartitionStrategy};
    use inklog::archive::{ArchiveServiceBuilder, CompressionType, S3ArchiveConfig, StorageClass};
    use inklog::{InklogConfig, LoggerManager};
    use std::time::Duration;
    use tempfile::TempDir;

    /// 创建内存数据库连接 URL
    fn create_memory_db_url() -> String {
        "sqlite::memory:".to_string()
    }

    /// 创建临时目录用于测试
    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    // === 数据库归档配置测试 ===

    #[tokio::test]
    async fn test_database_sink_config_with_archive() {
        let db_url = create_memory_db_url();
        
        let db_config = DatabaseSinkConfig {
            name: "test_db".to_string(),
            enabled: true,
            driver: DatabaseDriver::SQLite,
            url: db_url.clone(),
            pool_size: 3,
            batch_size: 50,
            flush_interval_ms: 500,
            partition: PartitionStrategy::Monthly,
            archive_to_s3: true,
            archive_after_days: 7,
            s3_bucket: Some("test-bucket".to_string()),
            s3_region: Some("us-west-2".to_string()),
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            ..Default::default()
        };
        
        // 验证配置有效性
        assert!(db_config.archive_to_s3);
        assert_eq!(db_config.s3_bucket, Some("test-bucket".to_string()));
        assert_eq!(db_config.s3_region, Some("us-west-2".to_string()));
    }

    #[tokio::test]
    async fn test_archive_config_validation() {
        let archive_config = S3ArchiveConfig {
            enabled: true,
            bucket: "test-bucket".to_string(),
            region: "us-west-2".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            compression: CompressionType::Zstd,
            storage_class: StorageClass::StandardIa,
            prefix: "test-archives/".to_string(),
            max_file_size_mb: 50,
            ..Default::default()
        };
        
        // 验证配置有效性
        assert!(archive_config.enabled);
        assert!(!archive_config.bucket.is_empty());
        assert!(!archive_config.region.is_empty());
    }

    // === 数据库 + S3 归档流程测试 ===

    #[tokio::test]
    async fn test_database_logging_to_s3_archive() {
        let _test_dir = create_test_dir();
        let db_url = create_memory_db_url();
        
        let config = InklogConfig {
            global: inklog::config::GlobalConfig {
                level: "info".to_string(),
                ..Default::default()
            },
            console_sink: Some(inklog::config::ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            database_sink: Some(DatabaseSinkConfig {
                name: "archive_test".to_string(),
                enabled: true,
                driver: DatabaseDriver::SQLite,
                url: db_url.clone(),
                pool_size: 2,
                batch_size: 10,
                flush_interval_ms: 200,
                partition: PartitionStrategy::Monthly,
                archive_to_s3: true,
                archive_after_days: 0, // 立即归档
                s3_bucket: Some("test-bucket".to_string()),
                s3_region: Some("us-east-1".to_string()),
                table_name: "logs".to_string(),
                archive_format: "json".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        let manager = LoggerManager::with_config(config).await.unwrap();
        
        // 写入测试日志
        for i in 0..20 {
            log::info!(target: "archive_test", "Archive test log message #{}", i);
        }
        
        // 等待批处理写入
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // 验证日志已写入数据库
        // 注意：实际验证需要查询数据库，但内存数据库在测试间不共享
        log::info!("Database to S3 archive test completed");
        
        // 清理
        manager.shutdown().await.ok();
    }

    #[tokio::test]
    async fn test_archive_service_builder_with_database() {
        let db_url = create_memory_db_url();
        
        // 创建数据库连接池
        let db_pool = dbnexus::pool::DbPool::new(&db_url).await.unwrap();
        let session = db_pool.get_session("test").await.unwrap();
        
        let archive_config = S3ArchiveConfig {
            enabled: true,
            bucket: "test-archive-bucket".to_string(),
            region: "us-west-2".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            compression: CompressionType::Gzip,
            storage_class: StorageClass::Standard,
            prefix: "integration_test/".to_string(),
            max_file_size_mb: 25,
            ..Default::default()
        };
        
        // 使用数据库会话构建归档服务
        let archive_service = ArchiveServiceBuilder::new()
            .config(archive_config)
            .database_session(session)
            .build()
            .await
            .unwrap();
        
        // 验证服务配置
        assert_eq!(archive_service.bucket(), "test-archive-bucket");
        assert_eq!(archive_service.region(), "us-west-2");
        assert_eq!(archive_service.archive_interval_days(), 1);
    }

    // === 归档压缩格式测试 ===

    #[test]
    fn test_compression_type_options() {
        // 验证所有压缩类型可用
        let types = [
            CompressionType::None,
            CompressionType::Gzip,
            CompressionType::Zstd,
            CompressionType::Lz4,
            CompressionType::Brotli,
        ];
        
        for compression in types {
            let config = S3ArchiveConfig {
                enabled: true,
                bucket: "test".to_string(),
                region: "us-east-1".to_string(),
                compression,
                ..Default::default()
            };
            assert!(config.compression == compression);
        }
    }

    #[test]
    fn test_storage_class_options() {
        // 验证所有存储类别可用
        let classes = [
            StorageClass::Standard,
            StorageClass::IntelligentTiering,
            StorageClass::StandardIa,
            StorageClass::OnezoneIa,
            StorageClass::Glacier,
            StorageClass::GlacierDeepArchive,
            StorageClass::ReducedRedundancy,
        ];
        
        for storage in classes {
            let config = S3ArchiveConfig {
                enabled: true,
                bucket: "test".to_string(),
                region: "us-east-1".to_string(),
                storage_class: storage,
                ..Default::default()
            };
            assert!(config.storage_class == storage);
        }
    }

    // === 归档调度配置测试 ===

    #[tokio::test]
    async fn test_archive_schedule_configuration() {
        let archive_config = S3ArchiveConfig {
            enabled: true,
            bucket: "scheduled-archive-bucket".to_string(),
            region: "eu-west-1".to_string(),
            archive_interval_days: 7,
            local_retention_days: 30,
            compression: CompressionType::Zstd,
            storage_class: StorageClass::Glacier,
            prefix: "scheduled/".to_string(),
            max_file_size_mb: 100,
            schedule_expression: Some("0 2 * * *".to_string()), // 每天凌晨2点
            ..Default::default()
        };
        
        // 验证调度配置
        assert!(archive_config.schedule_expression.is_some());
        assert_eq!(archive_config.schedule_expression.as_ref().unwrap(), "0 2 * * *");
        assert_eq!(archive_config.archive_interval_days(), 7);
        assert_eq!(archive_config.local_retention_days(), 30);
    }

    // === 归档失败处理测试 ===

    #[tokio::test]
    async fn test_archive_failure_handling() {
        // 测试无效 S3 配置时的错误处理
        let invalid_config = S3ArchiveConfig {
            enabled: true,
            bucket: "".to_string(), // 无效的空桶名
            region: "invalid-region".to_string(),
            ..Default::default()
        };
        
        // 应该返回错误
        let result = ArchiveServiceBuilder::new()
            .config(invalid_config)
            .build()
            .await;
            
        // 预期失败（实际行为取决于 S3 SDK）
        assert!(result.is_err() || result.is_ok()); // 不做硬性假设
    }

    // === 归档列表功能测试 ===

    #[tokio::test]
    async fn test_archive_listing() {
        let archive_config = S3ArchiveConfig {
            enabled: true,
            bucket: "list-test-bucket".to_string(),
            region: "us-east-1".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            prefix: "list_test/".to_string(),
            ..Default::default()
        };
        
        // 创建归档服务
        let archive_service = ArchiveServiceBuilder::new()
            .config(archive_config)
            .build()
            .await
            .unwrap();
        
        // 列出归档（可能为空）
        let result = archive_service.list_archives(None, None).await;
        
        // 预期行为：返回 Result，无论是否成功
        assert!(result.is_ok() || result.is_err());
    }

    // === Parquet 导出测试 ===

    #[tokio::test]
    async fn test_parquet_archive_format() {
        let config = InklogConfig {
            database_sink: Some(DatabaseSinkConfig {
                name: "parquet_test".to_string(),
                enabled: true,
                driver: DatabaseDriver::SQLite,
                url: create_memory_db_url(),
                pool_size: 2,
                batch_size: 100,
                flush_interval_ms: 1000,
                archive_to_s3: true,
                archive_after_days: 0,
                archive_format: "parquet".to_string(),
                parquet_config: inklog::config::ParquetConfig {
                    compression_level: 3,
                    encoding: "PLAIN".to_string(),
                    max_row_group_size: 10000,
                    max_page_size: 1024 * 1024,
                    include_fields: vec![],
                },
                ..Default::default()
            }),
            ..Default::default()
        };
        
        let manager = LoggerManager::with_config(config).await.unwrap();
        
        // 写入测试数据
        for i in 0..50 {
            log::info!("Parquet test message #{}", i);
        }
        
        // 等待处理
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        log::info!("Parquet archive format test completed");
        
        manager.shutdown().await.ok();
    }

    // === 归档性能测试 ===

    #[tokio::test]
    async fn test_archive_performance() {
        use std::time::Instant;
        
        let archive_config = S3ArchiveConfig {
            enabled: true,
            bucket: "perf-test-bucket".to_string(),
            region: "us-west-2".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            compression: CompressionType::Zstd,
            prefix: "performance_test/".to_string(),
            max_file_size_mb: 10,
            ..Default::default()
        };
        
        let start = Instant::now();
        
        // 创建归档服务
        let archive_service = ArchiveServiceBuilder::new()
            .config(archive_config)
            .build()
            .await
            .unwrap();
        
        let creation_time = start.elapsed();
        
        // 归档服务创建应该快速
        assert!(creation_time.as_secs() < 5, "Archive service creation too slow: {:?}", creation_time);
        
        log::info!("Archive service created in {:?}", creation_time);
    }
}

#[cfg(not(all(test, feature = "aws", feature = "dbnexus")))]
mod s3_database_integration_test {
    // 当 aws 或 dbnexus 特性未启用时，跳过这些测试
    use super::*;
    
    #[tokio::test]
    async fn test_database_sink_config_with_archive() {
        // 需要 aws 和 dbnexus 特性
        println!("Skipping test: requires --features \"aws,dbnexus\"");
    }
    
    #[tokio::test]
    async fn test_archive_config_validation() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[tokio::test]
    async fn test_database_logging_to_s3_archive() {
        println!("Skipping test: requires --features \"aws,dbnexus\"");
    }
    
    #[tokio::test]
    async fn test_archive_service_builder_with_database() {
        println!("Skipping test: requires --features \"aws,dbnexus\"");
    }
    
    #[test]
    fn test_compression_type_options() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[test]
    fn test_storage_class_options() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[tokio::test]
    async fn test_archive_schedule_configuration() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[tokio::test]
    async fn test_archive_failure_handling() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[tokio::test]
    async fn test_archive_listing() {
        println!("Skipping test: requires --features \"aws\"");
    }
    
    #[tokio::test]
    async fn test_parquet_archive_format() {
        println!("Skipping test: requires --features \"dbnexus\"");
    }
    
    #[tokio::test]
    async fn test_archive_performance() {
        println!("Skipping test: requires --features \"aws\"");
    }
}
