// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 可配采样策略端到端测试：TOML 配置链 → LoggerManager 接线 → 压力路径行为。
//!
//! 三条覆盖线：
//! - 构建防线：绕过 `InklogConfig::validate` 的直接注入（模拟 `from_file` 类
//!   不经 validate 的加载路径）由接线处 `SamplingPolicy::from_config` 拒绝；
//! - TOML 全链：`[sampling]` 经 `FromStr` → `validate` 硬校验后接线生效；
//! - 未配置 = 现状：TOML 无 `[sampling]` 时不接线，压力下非关键级别全丢、
//!   ERROR 保留 1-in-100，与既有兜底语义一致。

use inklog::{
    ConsoleSinkConfig, FileSinkConfig, InklogConfig, InklogError, LoggerManager, TargetSamplingRule,
};
use serial_test::serial;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tracing_subscriber::layer::SubscriberExt;

/// 轮询读取文件直至内容连续多次无变化（batch_size=1 + flush 10ms 的落盘节奏），
/// 返回最终内容。轮询在 `with_default` 之外执行，不再产生新日志。
fn await_settled_content(path: &std::path::Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = String::new();
    let mut stable_polls = 0_usize;
    while Instant::now() < deadline {
        let current = std::fs::read_to_string(path).unwrap_or_default();
        if current == last && !current.is_empty() {
            stable_polls += 1;
            if stable_polls >= 10 {
                return current;
            }
        } else {
            stable_polls = 0;
            last = current;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    last
}

fn file_sink(path: std::path::PathBuf) -> FileSinkConfig {
    FileSinkConfig {
        enabled: true,
        path,
        batch_size: 1,
        flush_interval_ms: 10,
        fsync: false,
        audit_chain_enabled: false,
        ..Default::default()
    }
}

// 建真实 logger 的用例必须串行：logger 构建会向进程级 OPS_EVENT_HUB 注册
// 通道且不注销，并行测试的 ops 广播会写入本测试的 file sink（与
// multi_sink_fallback_test 的串行化理由一致）

#[tokio::test]
#[serial]
async fn test_invalid_sampling_rule_rejected_at_build() {
    let temp_dir = tempdir().unwrap();
    let mut config = InklogConfig::default();
    config.global.level = "info".to_string();
    config.console_sink = Some(ConsoleSinkConfig {
        enabled: false,
        ..Default::default()
    });
    config.file_sink = Some(FileSinkConfig {
        enabled: false,
        ..Default::default()
    });
    // 手工注入非法 keep_level：跳过 TOML 链 validate 的加载路径必须仍被
    // manager 接线期的策略编译拒绝（第二道防线）
    config.sampling.per_target_prefix.insert(
        "app::audit".to_string(),
        TargetSamplingRule {
            keep_level: Some("verbose".to_string()),
            sample_every_n: 1,
        },
    );

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
                msg.contains("keep_level"),
                "error must name the offending field, got: {msg}"
            );
        }
        Ok(_) => panic!("invalid keep_level must be rejected at build time"),
        Err(other) => panic!("expected ConfigError, got: {other:?}"),
    }
    let _ = temp_dir;
}

#[tokio::test]
#[serial]
async fn test_sampling_policy_from_toml_shapes_rate_limited_output() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("sampling_policy.log");

    let toml_str = r#"
[global]
level = "info"

[console_sink]
enabled = false

[file_sink]
enabled = true

[performance]
rate_limit = 1

[sampling.per_level]
warn = 2
"#;
    let mut config: InklogConfig = toml_str
        .parse()
        .expect("valid sampling TOML must pass the config chain");
    assert!(
        !config.sampling.is_empty(),
        "TOML section must be carried into the config"
    );
    config.file_sink = Some(file_sink(log_path.clone()));

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
        // 令牌桶启动 1 令牌：首条 WARN 直接放行，其余 5 条进入压力路径，
        // 由 per_level 1-in-2 决定保留（计数器 0/2/4 放行 → 3 条）
        for i in 0..6 {
            tracing::warn!(target: "inklog::sampling::e2e", message = format!("warn-{i}"));
        }
        for i in 0..6 {
            tracing::info!(target: "inklog::sampling::e2e", message = format!("info-{i}"));
        }
    });

    let content = await_settled_content(&log_path);
    let warn_kept = (0..6)
        .filter(|i| content.contains(&format!("warn-{i}")))
        .count();
    let info_kept = (0..6)
        .filter(|i| content.contains(&format!("info-{i}")))
        .count();

    assert_eq!(
        info_kept, 0,
        "INFO records without a matching rule must be dropped by the baseline stress path: {content}"
    );
    // 期望 4（1 令牌 + 3 条采样保留）；±1 容差仅吸收令牌桶真实时钟 refill 的
    // 亚秒累积。下界 3 同时区分未配置基线（WARN 作为非关键级别全丢 = 0）
    assert!(
        (3..=5).contains(&warn_kept),
        "per_level 1-in-2 must keep about half of the rate-limited WARN records, got {warn_kept}: {content}"
    );

    let _ = manager.shutdown();
}

#[tokio::test]
#[serial]
async fn test_absent_sampling_keeps_baseline_behavior() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("baseline_sampling.log");

    // TOML 无 [sampling]：空策略不接线，压力路径保持既有兜底语义
    let toml_str = r#"
[global]
level = "info"

[console_sink]
enabled = false

[file_sink]
enabled = true

[performance]
rate_limit = 1
"#;
    let mut config: InklogConfig = toml_str
        .parse()
        .expect("baseline TOML must pass the config chain");
    assert!(
        config.sampling.is_empty(),
        "absent section must default to an empty policy"
    );
    config.file_sink = Some(file_sink(log_path.clone()));

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
        for i in 0..6 {
            tracing::warn!(target: "inklog::sampling::e2e", message = format!("warn-{i}"));
        }
        for i in 0..6 {
            tracing::error!(target: "inklog::sampling::e2e", message = format!("error-{i}"));
        }
    });

    let content = await_settled_content(&log_path);
    let warn_kept = (0..6)
        .filter(|i| content.contains(&format!("warn-{i}")))
        .count();
    let error_kept = (0..6)
        .filter(|i| content.contains(&format!("error-{i}")))
        .count();

    assert_eq!(
        warn_kept, 0,
        "baseline must drop non-critical WARN records entirely under stress: {content}"
    );
    assert_eq!(
        error_kept, 1,
        "baseline must keep exactly the first sampled ERROR (counter 0 of 1-in-100): {content}"
    );

    let _ = manager.shutdown();
}
