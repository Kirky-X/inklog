// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! per-target 分级限流（R7）配置链端到端测试：
//! TOML `[rate_limit.rules]` → 校验 → manager 接线 → 压力下按组裁决。

use inklog::{InklogConfig, InklogError, LoggerManager};
use serial_test::serial;
use std::time::Duration;
use tempfile::tempdir;
use tracing_subscriber::layer::SubscriberExt;

// 建真实 logger 的用例必须串行：logger 构建会向进程级 OPS_EVENT_HUB 注册
// 通道且不注销（与 multi_sink_fallback_test 的串行化理由一致）
#[tokio::test]
#[serial]
async fn test_invalid_rate_limit_rule_rejected_at_build() {
    let mut rate_limit = inklog::RateLimitConfig::default();
    // 手工注入零速率规则：绕过 TOML 链 validate 的直接注入必须仍被
    // manager 接线期的 TargetRateLimiter::from_rules 拒绝（第二道防线）
    rate_limit.rules.insert("app::noise".to_string(), 0);
    let config = InklogConfig {
        console_sink: Some(inklog::ConsoleSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        file_sink: Some(inklog::FileSinkConfig {
            enabled: false,
            ..Default::default()
        }),
        rate_limit,
        ..Default::default()
    };

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

    match result {
        Err(InklogError::ConfigError(msg)) => {
            assert!(
                msg.contains("app::noise") && msg.contains(">= 1"),
                "error must name the offending rule, got: {msg}"
            );
        }
        Ok(_) => panic!("zero-rate rule must be rejected at build time"),
        Err(other) => panic!("expected ConfigError, got: {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn test_rate_limit_rules_from_toml_govern_ungoverned_split() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("rate_limit_rules.log");

    // TOML 全链：无全局限流（performance.rate_limit 缺省），仅前缀组——
    // 受管辖组的裁决独立于全局路径，未管辖 target 完全不受限
    let toml_str = r#"
[global]
level = "info"

[console_sink]
enabled = false

[file_sink]
enabled = true

[rate_limit.rules]
"app::noise" = 1
"#;
    let mut config: InklogConfig = toml_str
        .parse()
        .expect("valid rate_limit TOML must pass the config chain");
    assert!(!config.rate_limit.is_empty());
    config.file_sink = Some(inklog::FileSinkConfig {
        enabled: true,
        path: log_path.clone(),
        batch_size: 1,
        flush_interval_ms: 10,
        fsync: false,
        audit_chain_enabled: false,
        ..Default::default()
    });

    let (manager, subscriber, filter) = LoggerManager::build_detached(
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

    let registry = tracing_subscriber::registry().with(subscriber).with(filter);
    tracing::subscriber::with_default(registry, || {
        // 组桶 1 令牌：第 1 条放行、第 2 条组配额丢弃；未管辖 target 不受限
        tracing::warn!(target: "app::noise::spam", message = "noise-kept");
        tracing::warn!(target: "app::noise::spam", message = "noise-dropped");
        tracing::warn!(target: "other::clean", message = "clean-kept");
    });

    // 等待 file sink worker 落盘（batch_size=1 + flush 10ms）
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let content = loop {
        let current = std::fs::read_to_string(&log_path).unwrap_or_default();
        if current.contains("noise-kept") || std::time::Instant::now() >= deadline {
            break current;
        }
        inklog::tokio::time::sleep(Duration::from_millis(20)).await;
    };

    assert!(
        content.contains("noise-kept") && !content.contains("noise-dropped"),
        "governed group must keep the first record and drop the quota-exhausted one: {content}"
    );
    assert!(
        content.contains("clean-kept"),
        "ungoverned target must be untouched by quota rules: {content}"
    );

    let status = manager.get_health_status();
    assert_eq!(
        status.metrics.logs_dropped, 1,
        "quota drop must surface as logs_dropped"
    );

    let _ = manager.shutdown();
}
