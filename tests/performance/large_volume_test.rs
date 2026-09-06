// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// 大数据量性能测试
//
// 核正：with_config 内部 set_global_default 为进程级单次语义，多用例并发时后装者
// 日志流向首个 logger 导致记录丢失（metadata.len() 断言随机失败）。统一改造为
// 本仓测试口径（additional_tests.rs 范本）：build_detached + 线程级 set_default
// + tracing::info!；serial 不再需要（无进程级全局状态竞争）。

#[cfg(test)]
mod large_volume {
    use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
    use tempfile::TempDir;
    use tracing_subscriber::layer::SubscriberExt;

    #[tokio::test]
    async fn test_large_volume_writing() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("large_volume_test.log");

        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: log_path.clone(),
                max_size: "1GB".into(),
                batch_size: 1000,
                flush_interval_ms: 500,
                compress: true,
                ..Default::default()
            }),
            performance: inklog::config::PerformanceConfig {
                worker_threads: 4,
                channel_capacity: 50000,
                ..Default::default()
            },
            ..Default::default()
        };

        let (logger, subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .unwrap();
        let _guard = tracing::subscriber::set_default(
            tracing_subscriber::registry().with(subscriber).with(filter),
        );

        // 写入大量数据（5000 条 × ~240B ≈ 1.2MB）
        for i in 0..5000 {
            tracing::info!(target: "large_volume", "Large volume test message {} - Data: {}", i, "x".repeat(100));
        }

        // shutdown 触发 flush_batch 确保全部落盘后再断言（替代原 sleep(2s) 轮询等待）
        let _ = logger.shutdown();

        // 验证数据已写入
        assert!(log_path.exists());
        let metadata = std::fs::metadata(&log_path).unwrap();
        assert!(
            metadata.len() > 50000,
            "写入量应超过 50KB，实际 {} 字节",
            metadata.len()
        );
    }
}
