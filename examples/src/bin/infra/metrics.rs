// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 健康监控与指标收集示例（Layer 2 外部服务）
//!
//! 演示 Metrics、HealthStatus、SinkHealthMonitor、SinkStatus、GaugeF64 的使用，
//! 覆盖指标采集、Sink 健康状态转换、降级监控以及 Prometheus 格式导出。
//!
//! # 功能演示
//!
//! - `Metrics::new()` 创建指标收集器并采集各类计数器
//! - `Histogram` 延迟分布（P50/P95/P99）
//! - `GaugeF64` 浮点指标（pool_hit_rate）
//! - `SinkStatus` 状态转换：NotStarted → Healthy → Degraded → Unhealthy → Healthy
//! - `SinkHealthMonitor` 智能降级与自动恢复
//! - `HealthStatus` 整体健康快照
//! - `Metrics::export_prometheus()` Prometheus 格式导出
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin metrics
//! ```

use inklog::FallbackAction;
use inklog::{
    FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealth, SinkHealthMonitor,
    SinkStatus,
};
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}\n", tr("example-metrics-inklog-health-monitoring-and"));

    // 1. Metrics creation and basic counters
    show_metrics_creation();

    // 2. Latency histogram and percentiles
    show_latency_histogram();

    // 3. GaugeF64 floating-point metric
    show_gauge_f64();

    // 4. SinkStatus transitions
    show_sink_status_transitions();

    // 5. Sink 健康监控（Metrics.update_sink_health）
    show_sink_health_tracking();

    // 6. HealthStatus overall snapshot
    show_health_status_snapshot();

    // 7. SinkHealthMonitor smart fallback and recovery
    show_sink_health_monitor();

    // 8. Prometheus format export
    show_prometheus_export();

    println!("\n{}", tr("example-metrics-all-health-monitoring-and"));
    Ok(())
}

/// 展示 Metrics 创建和基础计数器操作
fn show_metrics_creation() {
    print_separator(&tr("example-metrics-1-metrics-creation-and-basic"));

    print_section(&tr("example-metrics-1-1-creating-an-instance-with"));
    let metrics = Metrics::new();
    println!(
        "{}",
        tr_args(
            "example-metrics-initial-logs-written",
            &[("value", &(metrics.logs_written()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-initial-logs-dropped",
            &[("value", &(metrics.logs_dropped()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-initial-sink-errors",
            &[("value", &(metrics.sink_errors()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-initial-channel-blocked",
            &[("value", &(metrics.channel_blocked()))]
        )
    );
    assert_eq!(metrics.logs_written(), 0);

    print_section(&tr("example-metrics-1-2-inc-logs-written"));
    for i in 0..5 {
        metrics.inc_logs_written();
        println!(
            "  {}",
            tr_args(
                "example-metrics-write-cumulative",
                &[("value", &(i + 1)), ("value2", &(metrics.logs_written()))]
            )
        );
    }
    assert_eq!(metrics.logs_written(), 5);

    print_section(&tr("example-metrics-1-3-inc-logs-dropped-inc-sink"));
    metrics.inc_logs_dropped();
    metrics.inc_logs_dropped();
    metrics.inc_sink_error();
    metrics.inc_channel_blocked();
    metrics.inc_channel_blocked();
    metrics.inc_channel_blocked();
    println!(
        "{}",
        tr_args(
            "example-metrics-logs-dropped",
            &[("value", &(metrics.logs_dropped()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-sink-errors",
            &[("value", &(metrics.sink_errors()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-channel-blocked",
            &[("value", &(metrics.channel_blocked()))]
        )
    );

    print_section(&tr("example-metrics-1-4-db-batch-metrics-set-db"));
    metrics.set_db_batch_size(64);
    metrics.add_db_batch_records_total(64);
    metrics.add_db_batch_records_total(32);
    println!(
        "{}",
        tr_args(
            "example-metrics-db-batch-size",
            &[("value", &(metrics.db_batch_size()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-db-batch-records-total",
            &[("value", &(metrics.db_batch_records_total()))]
        )
    );
    assert_eq!(metrics.db_batch_size(), 64);
    assert_eq!(metrics.db_batch_records_total(), 96);

    print_section(&tr("example-metrics-1-5-lock-contention-counter"));
    metrics.inc_lock_contention();
    metrics.inc_lock_contention();
    println!(
        "{}",
        tr_args(
            "example-metrics-active-workers-default-0-set-by",
            &[("value", &(metrics.active_workers()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-lock-contention",
            &[("value", &(metrics.lock_contention()))]
        )
    );

    print_section(&tr("example-metrics-1-6-uptime-runtime-duration"));
    let uptime = metrics.uptime();
    println!(
        "{}",
        tr_args(
            "example-metrics-uptime",
            &[("uptime", &format!("{:?}", uptime))]
        )
    );
    assert!(uptime.as_secs() < 60);
}

/// 展示延迟直方图与百分位统计
fn show_latency_histogram() {
    print_separator(&tr("example-metrics-2-latency-histogram-and"));

    print_section(&tr("example-metrics-2-1-record-latency-records"));
    let metrics = Metrics::new();
    // 模拟 10 条延迟：50μs ~ 5000μs
    let samples = [50u64, 120, 300, 800, 1500, 2200, 3500, 5000, 7500, 12000];
    for &us in &samples {
        metrics.record_latency(Duration::from_micros(us));
    }
    println!(
        "{}",
        tr_args(
            "example-metrics-recorded-latency-samples",
            &[("value", &(samples.len()))]
        )
    );

    print_section(&tr("example-metrics-2-2-latency-aggregates-avg-p50"));
    let status = metrics.get_status(0, 100);
    let snap = &status.metrics;
    println!(
        "{}",
        tr_args(
            "example-metrics-avg-latency-us",
            &[("avg_latency_us", &snap.avg_latency_us)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-p50-latency-us",
            &[("p50_latency_us", &snap.p50_latency_us)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-p95-latency-us",
            &[("p95_latency_us", &snap.p95_latency_us)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-p99-latency-us",
            &[("p99_latency_us", &snap.p99_latency_us)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-latency-distribution",
            &[(
                "latency_distribution",
                &format!("{:?}", snap.latency_distribution)
            )]
        )
    );
    assert!(snap.p95_latency_us >= snap.p50_latency_us);
}

/// 展示 GaugeF64 浮点指标
fn show_gauge_f64() {
    print_separator(&tr("example-metrics-3-gaugef64-floating-point-metric"));

    print_section(&tr("example-metrics-3-1-gaugef64-new-set-get"));
    let gauge = GaugeF64::new(0.0);
    println!(
        "{}",
        tr_args(
            "example-metrics-initial-value",
            &[("value", &(gauge.get()))]
        )
    );
    gauge.set(87.5);
    println!(
        "{}",
        tr_args(
            "example-metrics-after-set-87-5",
            &[("value", &(gauge.get()))]
        )
    );
    gauge.set(99.9);
    println!(
        "{}",
        tr_args(
            "example-metrics-after-set-99-9",
            &[("value", &(gauge.get()))]
        )
    );

    print_section(&tr("example-metrics-3-2-the-pool-hit-rate-metric"));
    let metrics = Metrics::new();
    metrics.set_pool_hit_rate(85.5);
    println!(
        "{}",
        tr_args(
            "example-metrics-pool-hit-rate",
            &[("value", &format!("{:.2}", metrics.pool_hit_rate()))]
        )
    );
    metrics.set_pool_hit_rate(0.0);
    println!(
        "{}",
        tr_args(
            "example-metrics-pool-hit-rate-after-reset",
            &[("value", &format!("{:.2}", metrics.pool_hit_rate()))]
        )
    );
    assert!((metrics.pool_hit_rate() - 0.0).abs() < f64::EPSILON);

    print_section(&tr("example-metrics-3-3-sinkhealth-healthy"));
    let healthy = SinkHealth::healthy();
    assert!(matches!(healthy.status, SinkStatus::Healthy));
    assert_eq!(healthy.consecutive_failures, 0);
    println!(
        "{}",
        tr_args(
            "example-metrics-healthy-status",
            &[("status", &format!("{:?}", healthy.status))]
        )
    );

    let unhealthy = SinkHealth::unhealthy("Connection refused".to_string());
    assert!(matches!(unhealthy.status, SinkStatus::Unhealthy { .. }));
    assert_eq!(unhealthy.consecutive_failures, 1);
    println!(
        "{}",
        tr_args(
            "example-metrics-unhealthy-status",
            &[("status", &format!("{:?}", unhealthy.status))]
        )
    );
}

/// 展示 SinkStatus 状态转换
fn show_sink_status_transitions() {
    print_separator(&tr("example-metrics-4-sinkstatus-transitions"));

    print_section(&tr("example-metrics-4-1-default-state-notstarted"));
    let mut status = SinkStatus::default();
    println!(
        "{}",
        tr_args(
            "example-metrics-status",
            &[("status", &format!("{:?}", status))]
        )
    );
    assert_eq!(status, SinkStatus::NotStarted);
    assert!(!status.is_operational());

    print_section(&tr("example-metrics-4-2-notstarted-healthy"));
    status = SinkStatus::Healthy;
    println!(
        "{}",
        tr_args(
            "example-metrics-status",
            &[("status", &format!("{:?}", status))]
        )
    );
    assert!(status.is_operational());

    print_section("4.3 Healthy → Degraded { reason }");
    status = SinkStatus::Degraded {
        reason: "slow disk IO".to_string(),
    };
    println!(
        "{}",
        tr_args(
            "example-metrics-status",
            &[("status", &format!("{:?}", status))]
        )
    );
    assert!(status.is_operational(), "Degraded still runs");

    print_section("4.4 Degraded → Unhealthy { error }");
    status = SinkStatus::Unhealthy {
        error: "disk space low".to_string(),
    };
    println!(
        "{}",
        tr_args(
            "example-metrics-status",
            &[("status", &format!("{:?}", status))]
        )
    );
    assert!(!status.is_operational());

    print_section(&tr("example-metrics-4-5-unhealthy-healthy-recovery"));
    status = SinkStatus::Healthy;
    println!(
        "{}",
        tr_args(
            "example-metrics-status",
            &[("status", &format!("{:?}", status))]
        )
    );
    assert!(status.is_operational());

    print_section(&tr("example-metrics-4-6-state-machine-summary"));
    println!(
        "{}",
        tr_args(
            "example-metrics-notstarted-is-operational",
            &[("value", &(SinkStatus::NotStarted.is_operational()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-healthy-is-operational",
            &[("value", &(SinkStatus::Healthy.is_operational()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-degraded-is-operational",
            &[(
                "value",
                &(SinkStatus::Degraded { reason: "r".into() }.is_operational())
            )]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-unhealthy-is-operational",
            &[(
                "value",
                &(SinkStatus::Unhealthy { error: "e".into() }.is_operational())
            )]
        )
    );
}

/// 展示 Metrics 的 Sink 健康跟踪
fn show_sink_health_tracking() {
    print_separator(&tr("example-metrics-5-metrics-sink-health-tracking"));

    print_section(&tr("example-metrics-5-1-sink-started-marks-a-sink"));
    let metrics = Metrics::new();
    metrics.sink_started("console");
    metrics.sink_started("file");
    metrics.sink_started("database");
    let health = metrics.sink_health();
    for name in ["console", "file", "database"] {
        let h = health.get(name).expect("sink should exist");
        println!(
            "  {}",
            tr_args(
                "example-metrics-failures",
                &[
                    ("name", &name),
                    ("status", &format!("{:?}", h.status)),
                    ("consecutive_failures", &h.consecutive_failures)
                ]
            )
        );
        assert!(matches!(h.status, SinkStatus::Healthy));
    }

    print_section(&tr("example-metrics-5-2-sink-degraded-marks"));
    metrics.sink_degraded("file", "disk IO latency increased".to_string());
    let health = metrics.sink_health();
    let h = health.get("file").expect("file should exist");
    println!(
        "{}",
        tr_args(
            "example-metrics-file-status",
            &[("status", &format!("{:?}", h.status))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-file-last-error",
            &[("last_error", &format!("{:?}", h.last_error))]
        )
    );
    assert!(matches!(h.status, SinkStatus::Degraded { .. }));

    print_section(&tr("example-metrics-5-3-update-sink-health"));
    // 多次失败，consecutive_failures 应累加
    metrics.update_sink_health("database", false, Some("Connection refused".into()));
    metrics.update_sink_health("database", false, Some("Timeout".into()));
    let health = metrics.sink_health();
    let h = health.get("database").expect("database should exist");
    println!(
        "{}",
        tr_args(
            "example-metrics-database-status",
            &[("status", &format!("{:?}", h.status))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-database-consecutive-failures",
            &[("consecutive_failures", &h.consecutive_failures)]
        )
    );
    assert_eq!(h.consecutive_failures, 2);

    print_section(&tr("example-metrics-5-4-update-sink-health-resets"));
    metrics.update_sink_health("database", true, None);
    let health = metrics.sink_health();
    let h = health.get("database").expect("database should exist");
    println!(
        "{}",
        tr_args(
            "example-metrics-database-status",
            &[("status", &format!("{:?}", h.status))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-database-consecutive-failures",
            &[("consecutive_failures", &h.consecutive_failures)]
        )
    );
    assert_eq!(h.consecutive_failures, 0);

    print_section(&tr("example-metrics-5-5-update-sink-health-uses-a"));
    metrics.update_sink_health("file", false, None);
    let health = metrics.sink_health();
    let h = health.get("file").expect("file should exist");
    if let SinkStatus::Unhealthy { error } = &h.status {
        println!(
            "{}",
            tr_args(
                "example-metrics-file-status-error",
                &[("error", &format!("{:?}", error))]
            )
        );
        assert_eq!(error, "Unknown error");
    } else {
        panic!("file should be Unhealthy");
    }
}

/// 展示 HealthStatus 整体快照
fn show_health_status_snapshot() {
    print_separator(&tr("example-metrics-6-healthstatus-overall-snapshot"));

    print_section(&tr("example-metrics-6-1-empty-sinks-notstarted"));
    let metrics = Metrics::new();
    let status: HealthStatus = metrics.get_status(0, 100);
    println!(
        "{}",
        tr_args(
            "example-metrics-overall-status",
            &[("overall_status", &format!("{:?}", status.overall_status))]
        )
    );
    assert!(matches!(status.overall_status, SinkStatus::NotStarted));

    print_section(&tr("example-metrics-6-2-all-healthy"));
    metrics.sink_started("console");
    metrics.sink_started("file");
    let status = metrics.get_status(50, 1000);
    println!(
        "{}",
        tr_args(
            "example-metrics-overall-status",
            &[("overall_status", &format!("{:?}", status.overall_status))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-channel-usage",
            &[("value", &format!("{:.2}", status.channel_usage * 100.0))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-uptime-seconds",
            &[("uptime_seconds", &status.uptime_seconds)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-encryption-key-valid",
            &[("encryption_key_valid", &status.encryption_key_valid)]
        )
    );
    assert!(matches!(status.overall_status, SinkStatus::Healthy));

    print_section(&tr("example-metrics-6-3-degraded-present-no"));
    metrics.sink_degraded("file", "slow IO".to_string());
    let status = metrics.get_status(0, 100);
    println!(
        "{}",
        tr_args(
            "example-metrics-overall-status",
            &[("overall_status", &format!("{:?}", status.overall_status))]
        )
    );
    assert!(matches!(status.overall_status, SinkStatus::Degraded { .. }));

    print_section(&tr("example-metrics-6-4-unhealthy-present"));
    metrics.update_sink_health("database", false, Some("Connection refused".into()));
    let status = metrics.get_status(0, 100);
    println!(
        "{}",
        tr_args(
            "example-metrics-overall-status",
            &[("overall_status", &format!("{:?}", status.overall_status))]
        )
    );
    match &status.overall_status {
        SinkStatus::Unhealthy { error } => {
            println!("{}", tr_args("example-metrics-error", &[("error", &error)]));
            assert!(error.contains("Connection refused"));
        }
        _ => panic!("should be Unhealthy"),
    }

    print_section(&tr("example-metrics-6-5-healthstatus-fields-at-a"));
    println!(
        "{}",
        tr_args(
            "example-metrics-sinks-count",
            &[("value", &(status.sinks.len()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-metrics-logs-written",
            &[("logs_written", &status.metrics.logs_written)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-metrics-sink-errors",
            &[("sink_errors", &status.metrics.sink_errors)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-metrics-active-workers",
            &[("active_workers", &status.metrics.active_workers)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-metrics-pool-hit-rate",
            &[("pool_hit_rate", &status.metrics.pool_hit_rate)]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-pool-stats",
            &[("pool_stats", &format!("{:?}", status.pool_stats))]
        )
    );
}

/// 展示 SinkHealthMonitor 智能降级与恢复
fn show_sink_health_monitor() {
    print_separator(&tr("example-metrics-7-sinkhealthmonitor-smart"));

    print_section(&tr("example-metrics-7-1-sinkhealthmonitor-default"));
    let monitor = SinkHealthMonitor::default();
    println!(
        "{}",
        tr_args(
            "example-metrics-database-initial-state",
            &[(
                "value",
                &format!("{:?}", monitor.get_fallback_state("database"))
            )]
        )
    );
    assert_eq!(
        monitor.get_fallback_state("database"),
        FallbackState::Active
    );
    assert!(!monitor.is_any_in_fallback());

    print_section(&tr("example-metrics-7-2-failures-below-threshold"));
    let action = monitor.check_and_fallback("database", false, Some("Connection refused"));
    println!(
        "{}",
        tr_args(
            "example-metrics-action",
            &[("action", &format!("{:?}", action))]
        )
    );
    assert!(matches!(action, FallbackAction::Retry { .. }));
    println!(
        "{}",
        tr_args(
            "example-metrics-requires-action",
            &[("value", &(action.requires_action()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-sink-name",
            &[("value", &format!("{:?}", action.sink_name()))]
        )
    );

    print_section(&tr("example-metrics-7-3-threshold-reached-fallback"));
    // 默认 failure_threshold = 3，前面已失败 1 次
    for _ in 0..3 {
        let _ = monitor.check_and_fallback("database", false, Some("Connection refused"));
    }
    let state = monitor.get_fallback_state("database");
    println!(
        "{}",
        tr_args(
            "example-metrics-database-state",
            &[("state", &format!("{:?}", state))]
        )
    );
    match &state {
        FallbackState::Fallback { target, reason } => {
            println!(
                "{}",
                tr_args("example-metrics-target", &[("target", &target)])
            );
            println!(
                "{}",
                tr_args("example-metrics-reason", &[("reason", &reason)])
            );
            assert_eq!(target, "file");
        }
        _ => panic!("should be Fallback"),
    }
    assert!(monitor.is_any_in_fallback());

    print_section(&tr("example-metrics-7-4-healthy-again"));
    let action = monitor.check_and_fallback("database", true, None);
    println!(
        "{}",
        tr_args(
            "example-metrics-action",
            &[("action", &format!("{:?}", action))]
        )
    );
    match &action {
        FallbackAction::AttemptRecovery { delay_ms, .. } => {
            println!(
                "{}",
                tr_args("example-metrics-delay-ms", &[("delay_ms", &delay_ms)])
            );
        }
        _ => panic!("should be AttemptRecovery"),
    }

    print_section(&tr("example-metrics-7-5-healthy-again-during"));
    let action = monitor.check_and_fallback("database", true, None);
    println!(
        "{}",
        tr_args(
            "example-metrics-action",
            &[("action", &format!("{:?}", action))]
        )
    );
    assert!(matches!(action, FallbackAction::Wait { .. }));

    print_section(&tr("example-metrics-7-6-confirm-recovery-confirms"));
    monitor.confirm_recovery("database");
    println!(
        "{}",
        tr_args(
            "example-metrics-database-state-2",
            &[(
                "value",
                &format!("{:?}", monitor.get_fallback_state("database"))
            )]
        )
    );
    assert_eq!(
        monitor.get_fallback_state("database"),
        FallbackState::Active
    );

    print_section(&tr("example-metrics-7-7-filesink-disk-full-fallback"));
    let monitor2 = SinkHealthMonitor::with_defaults();
    for _ in 0..3 {
        let _ = monitor2.check_and_fallback("file", false, Some("No space left on device"));
    }
    let state = monitor2.get_fallback_state("file");
    println!(
        "{}",
        tr_args(
            "example-metrics-file-state",
            &[("state", &format!("{:?}", state))]
        )
    );
    match &state {
        FallbackState::Fallback { target, .. } => assert_eq!(target, "console"),
        _ => panic!("should be Fallback"),
    }

    print_section(&tr("example-metrics-7-8-encryption-error-fallback"));
    let monitor3 = SinkHealthMonitor::with_defaults();
    let action = monitor3.handle_encryption_error("file", "Invalid key");
    println!(
        "{}",
        tr_args(
            "example-metrics-action",
            &[("action", &format!("{:?}", action))]
        )
    );
    match &action {
        FallbackAction::Fallback { target, .. } => assert_eq!(target, "plaintext"),
        _ => panic!("should be Fallback"),
    }

    print_section(&tr("example-metrics-7-9-custom-fallbackconfig"));
    let config = FallbackConfig {
        enabled: true,
        initial_delay_ms: 500,
        max_delay_ms: 30000,
        max_retries: 5,
        failure_threshold: 2,
    };
    let monitor4 = SinkHealthMonitor::new(config);
    // 触发 1 次失败（未达阈值 2）
    let _ = monitor4.check_and_fallback("database", false, Some("err"));
    let stats = monitor4.get_fallback_stats();
    println!(
        "{}",
        tr_args(
            "example-metrics-stats",
            &[("stats", &format!("{:?}", stats))]
        )
    );
    assert_eq!(stats.active_fallbacks, 0);

    print_section(&tr("example-metrics-7-10-reset-resets-the-monitor"));
    for _ in 0..3 {
        let _ = monitor4.check_and_fallback("database", false, Some("err"));
    }
    assert!(monitor4.is_any_in_fallback());
    monitor4.reset();
    assert!(!monitor4.is_any_in_fallback());
    println!(
        "{}",
        tr_args(
            "example-metrics-is-any-in-fallback-after-reset",
            &[("value", &(monitor4.is_any_in_fallback()))]
        )
    );

    print_section(&tr("example-metrics-7-11-get-fallback-events-event"));
    let monitor5 = SinkHealthMonitor::with_defaults();
    for _ in 0..3 {
        let _ = monitor5.check_and_fallback("database", false, Some("err"));
    }
    let events = monitor5.get_fallback_events(10);
    println!(
        "{}",
        tr_args("example-metrics-event-count", &[("value", &(events.len()))])
    );
    assert!(!events.is_empty());
    println!(
        "{}",
        tr_args(
            "example-metrics-first-event-sink",
            &[("value", &(events[0].sink_name))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-first-event-from",
            &[("value", &format!("{:?}", events[0].from_state))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-metrics-first-event-to",
            &[("value", &format!("{:?}", events[0].to_state))]
        )
    );
}

/// 展示 Prometheus 格式导出
fn show_prometheus_export() {
    print_separator(&tr("example-metrics-8-prometheus-format-export"));

    print_section(&tr("example-metrics-8-1-simulated-metric-collection"));
    let metrics = Metrics::new();
    // 模拟日志写入
    for _ in 0..1000 {
        metrics.inc_logs_written();
    }
    // 模拟少量丢弃与错误
    for _ in 0..5 {
        metrics.inc_logs_dropped();
    }
    for _ in 0..3 {
        metrics.inc_sink_error();
    }
    metrics.inc_channel_blocked();
    // 模拟延迟样本
    for us in [50u64, 120, 350, 800, 1500, 4500] {
        metrics.record_latency(Duration::from_micros(us));
    }
    // 设置 gauge 指标（active_workers 由内部 worker 维护，此处只设置可公开写入的）
    metrics.set_db_batch_size(128);
    metrics.add_db_batch_records_total(128);
    metrics.set_pool_hit_rate(92.5);
    metrics.sink_started("console");
    metrics.sink_started("file");
    metrics.sink_started("database");
    metrics.update_sink_health("database", false, Some("Connection refused".into()));

    print_section(&tr("example-metrics-8-2-export-prometheus-output"));
    let output = metrics.export_prometheus();
    // 验证关键 Prometheus 行存在
    assert!(output.contains("# TYPE inklog_logs_written_total counter"));
    assert!(output.contains("inklog_logs_written_total 1000"));
    assert!(output.contains("# TYPE inklog_logs_dropped_total counter"));
    assert!(output.contains("inklog_logs_dropped_total 5"));
    assert!(output.contains("# TYPE inklog_sink_errors_total counter"));
    assert!(output.contains("inklog_sink_errors_total 3"));
    assert!(output.contains("# TYPE inklog_channel_blocked_total counter"));
    assert!(output.contains("# TYPE inklog_db_batch_size gauge"));
    assert!(output.contains("inklog_db_batch_size{sink=\"database\"} 128"));
    assert!(output.contains("# TYPE inklog_active_workers gauge"));
    assert!(output.contains("inklog_active_workers 0"));
    assert!(output.contains("# TYPE inklog_pool_hit_rate gauge"));
    assert!(output.contains("inklog_pool_hit_rate 92.5"));
    assert!(output.contains("# TYPE inklog_sink_healthy gauge"));
    assert!(output.contains("inklog_sink_healthy{sink=\"console\"} 1"));
    assert!(output.contains("inklog_sink_healthy{sink=\"database\"} 0"));
    // 原生直方图形态：累积桶 + sum + count
    assert!(output.contains("# TYPE inklog_write_latency_us histogram"));
    assert!(output.contains("inklog_write_latency_us_bucket{le=\"+Inf\"}"));
    assert!(output.contains("inklog_write_latency_us_count"));

    // 打印部分输出（避免刷屏）
    for line in output.lines() {
        if line.starts_with("inklog_") && !line.starts_with("inklog_write_latency_us_bucket") {
            println!("  {}", line);
        }
    }

    print_section(&tr("example-metrics-8-3-healthstatus-serialization"));
    let status = metrics.get_status(128, 1024);
    let json = serde_json::to_string_pretty(&status.metrics).unwrap_or_default();
    // 仅打印前若干行避免刷屏
    for (i, line) in json.lines().enumerate() {
        if i >= 12 {
            println!("  {}", tr("example-metrics-truncated"));
            break;
        }
        println!("  {}", line);
    }

    println!(
        "\n{}",
        tr_args(
            "example-metrics-prometheus-export-has",
            &[
                ("value", &(output.len())),
                ("value2", &(output.lines().count()))
            ]
        )
    );
}
