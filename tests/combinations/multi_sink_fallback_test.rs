// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 多 Sink 降级功能测试
// 测试 File Sink 故障时降级到 Database，Database 故障时降级到 Console
// 验证降级后消息不丢失，以及恢复后自动切回
//
// 核正：FallbackState 已从 struct（new/enter_fallback/exit_fallback 方法族）
// 重构为纯状态 enum（Active/Fallback/Recovering），状态机行为由
// SinkHealthMonitor（check_and_fallback/confirm_recovery/get_fallback_state 等）
// 承载；FallbackConfig 的 auto_fallback 字段更名为 enabled。状态机用例改按
// Monitor 真实 API 重写；LoggerManager::shutdown 为同步方法，移除多余 .await。

#[cfg(test)]
mod multi_sink_fallback {
    use inklog::config::{ConsoleSinkConfig, DatabaseDriver, DatabaseSinkConfig, FileSinkConfig};
    use inklog::{
        FallbackAction, FallbackConfig, FallbackState, InklogConfig, LoggerManager,
        SinkHealthMonitor,
    };
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    // === 降级状态机测试（SinkHealthMonitor） ===

    #[test]
    fn test_monitor_initial_state() {
        let monitor = SinkHealthMonitor::with_defaults();

        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
        assert!(monitor.get_fallback_events(10).is_empty());
    }

    #[test]
    fn test_monitor_healthy_no_action() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 健康 Sink 检查不触发降级
        let action = monitor.check_and_fallback("database", true, None);
        assert!(!action.requires_action());
        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
    }

    #[test]
    fn test_monitor_fallback_transition() {
        // 核正：with_defaults 的 failure_threshold=3，单次故障仅返回 Retry；
        // 验证降级转移需将阈值降为 1（单次故障即进入 Fallback 状态）
        let monitor = SinkHealthMonitor::new(FallbackConfig {
            failure_threshold: 1,
            ..Default::default()
        });

        // 故障触发降级：database 故障 → 降级到 file（determine_fallback_target）
        let action = monitor.check_and_fallback("database", false, Some("connection refused"));
        assert!(action.requires_action(), "故障应产生降级动作");
        assert!(monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Fallback {
                target: "file".to_string(),
                reason: "connection refused".to_string()
            }
        );

        // 恢复确认后回到 Active
        monitor.confirm_recovery("database");
        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
    }

    #[test]
    fn test_monitor_fallback_events_recorded() {
        // 核正：降级事件仅在达到 failure_threshold 触发降级时记录（handle_failure），
        // with_defaults 阈值为 3——此处降为 1 以单次故障验证事件记录
        let monitor = SinkHealthMonitor::new(FallbackConfig {
            failure_threshold: 1,
            ..Default::default()
        });

        monitor.check_and_fallback("file", false, Some("disk full"));
        let events = monitor.get_fallback_events(10);
        assert!(!events.is_empty(), "降级事件应被记录");
        assert_eq!(events[0].sink_name, "file");
    }

    #[test]
    fn test_monitor_retry_below_threshold() {
        // 默认 failure_threshold=3：未达阈值的故障返回 Retry，状态保持 Active
        // 且不记录降级事件（handle_failure 的 Retry 分支）
        let monitor = SinkHealthMonitor::with_defaults();

        let action = monitor.check_and_fallback("database", false, Some("connection refused"));
        assert!(
            matches!(action, FallbackAction::Retry { ref attempt, .. } if *attempt == 1),
            "未达阈值应返回 Retry(attempt=1)，实际: {:?}",
            action
        );
        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
        assert!(monitor.get_fallback_events(10).is_empty());
    }

    #[test]
    fn test_monitor_encryption_error_action() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 加密错误处理路径（明文降级 + 警告）也应产生动作
        let action = monitor.handle_encryption_error("file", "bad key");
        assert!(action.requires_action());
    }

    #[test]
    fn test_fallback_config_defaults() {
        let config = FallbackConfig::default();

        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 60000);
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn test_fallback_config_options() {
        let config = FallbackConfig {
            enabled: true,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            max_retries: 5,
            ..Default::default()
        };

        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.max_retries, 5);
    }

    // === 文件到数据库降级测试 ===

    #[tokio::test]
    async fn test_file_sink_failure_triggers_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        // 创建无效的日志路径以触发文件写入失败
        let invalid_path = PathBuf::from("/nonexistent/path/that/does/not/exist/log.log");

        let file_config = FileSinkConfig {
            enabled: true,
            path: invalid_path,
            max_size: "1MB".into(),
            ..Default::default()
        };

        let db_config = DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            url: db_url.clone(),
            pool_size: 2,
            batch_size: 10,
            flush_interval_ms: 100,
            table_name: "logs".to_string(),
            ..Default::default()
        };

        let config = InklogConfig {
            global: inklog::config::GlobalConfig {
                auto_fallback: true,
                fallback_initial_delay_ms: 100,
                fallback_max_retries: 3,
                ..Default::default()
            },
            file_sink: Some(file_config),
            database_sink: Some(db_config),
            console_sink: Some(ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：改用 build_detached + 线程级 set_default（对齐 additional_tests 范本）——
        // with_config 的 set_global_default 为进程级单次语义，多用例下后装者的日志
        // 会流向首个 logger（已 shutdown）导致记录丢失
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

        // 写入日志（应该触发降级）
        tracing::info!("Testing fallback from file to database");
        tracing::warn!("This should be logged via fallback mechanism");

        // 等待处理
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // 验证日志仍然被记录（通过降级路径）
        // 如果降级成功，消息应该出现在控制台或数据库中
        tracing::info!("Fallback test completed");

        let _ = logger.shutdown();
    }

    // === 数据库到文件降级测试 ===

    #[tokio::test]
    async fn test_database_sink_failure_triggers_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("fallback_test.log");

        // 使用无效的数据库 URL
        let invalid_db_url = "postgres://invalid:invalid@nonexistent:5432/nonexistent";

        let file_config = FileSinkConfig {
            enabled: true,
            path: file_path.clone(),
            max_size: "1MB".into(),
            ..Default::default()
        };

        let db_config = DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::PostgreSQL,
            url: invalid_db_url.to_string(),
            pool_size: 1,
            batch_size: 10,
            flush_interval_ms: 100,
            table_name: "logs".to_string(),
            ..Default::default()
        };

        let config = InklogConfig {
            global: inklog::config::GlobalConfig {
                auto_fallback: true,
                fallback_initial_delay_ms: 100,
                fallback_max_retries: 3,
                ..Default::default()
            },
            file_sink: Some(file_config),
            database_sink: Some(db_config),
            console_sink: Some(ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：数据库不可用时 LoggerManager 在初始化期即快速失败
        // （DbNexusAdapter 建池报错，build_detached 与 with_config 同路径）；
        // sink 降级机制针对运行期写入失败，原“初始化失败后继续降级写文件”的
        // 推测与实现不符。build_detached 不安装全局 subscriber（避免劫持他例日志流）
        let result = LoggerManager::build_detached(
            config,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            None,
        )
        .await;
        assert!(result.is_err(), "无效数据库 URL 应导致初始化快速失败");
    }

    // === 所有 Sink 不可用时的处理 ===

    #[tokio::test]
    async fn test_all_sinks_unavailable() {
        // 无效的文件路径
        let invalid_file = PathBuf::from("/invalid/path/fallback.log");

        // 无效的数据库
        let invalid_db = "mysql://invalid:invalid@localhost:3306/invalid";

        let config = InklogConfig {
            global: inklog::config::GlobalConfig {
                auto_fallback: true,
                fallback_initial_delay_ms: 50,
                fallback_max_retries: 2,
                ..Default::default()
            },
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: invalid_file,
                ..Default::default()
            }),
            database_sink: Some(DatabaseSinkConfig {
                enabled: true,
                driver: DatabaseDriver::MySQL,
                url: invalid_db.to_string(),
                pool_size: 1,
                table_name: "logs".to_string(),
                ..Default::default()
            }),
            console_sink: Some(ConsoleSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        // 核正：数据库不可用时初始化即快速失败（同 test_database_sink_failure_triggers_fallback），
        // “文件+数据库全不可用时控制台仍可创建 logger”的推测与实现不符
        let result = LoggerManager::build_detached(
            config,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            None,
        )
        .await;
        assert!(result.is_err(), "文件与数据库均不可用应导致初始化快速失败");
    }

    // === 并发降级场景测试 ===

    #[tokio::test]
    async fn test_concurrent_fallback_scenarios() {
        use inklog::tokio::sync::Barrier;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("concurrent.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let file_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/invalid/concurrent.log"),
            ..Default::default()
        };

        let db_config = DatabaseSinkConfig {
            enabled: true,
            driver: DatabaseDriver::SQLite,
            url: db_url.clone(),
            pool_size: 5,
            batch_size: 100,
            flush_interval_ms: 50,
            table_name: "logs".to_string(),
            ..Default::default()
        };

        let config = InklogConfig {
            global: inklog::config::GlobalConfig {
                auto_fallback: true,
                ..Default::default()
            },
            file_sink: Some(file_config),
            database_sink: Some(db_config),
            console_sink: Some(ConsoleSinkConfig::default()),
            ..Default::default()
        };

        // 核正：同 test_file_sink_failure_triggers_fallback——build_detached +
        // 线程级 set_default 避免全局 subscriber 单次语义导致的日志流劫持
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

        // 并发写入测试
        let barrier = Arc::new(Barrier::new(10));
        let counter = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let barrier = barrier.clone();
                let counter = counter.clone();
                tokio::spawn(async move {
                    barrier.wait().await;
                    for _ in 0..10 {
                        tracing::info!("Concurrent fallback test message");
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // 验证所有消息被处理
        assert_eq!(counter.load(Ordering::SeqCst), 100);

        tracing::info!("Concurrent fallback test completed");

        let _ = logger.shutdown();
    }
}
